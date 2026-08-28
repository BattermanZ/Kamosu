//! Slow work: the Job machinery beneath every Operation declared `Kind::Job`.
//!
//! Asking for a Job records it in the database — the truth about the work, its
//! progress and its result or failure reason (ADR 0003) — and hands it to a
//! **lane**. There are exactly two lanes, and ordinarily which one carries a
//! job depends on who asked, never on what was asked (ADR 0032): members have
//! their own lane, and everything a stranger can cause runs in one depth-one
//! lane behind a short bounded line. When the line is full the ask is refused
//! with *busy*, so the worst a stranger can inflict is latency on other
//! strangers — never collapse.
//!
//! One kind of Job breaks that rule on purpose: the Catalogue may declare a Job
//! `JobLane::AlwaysSingle` when its risk lives in the outbound fetch itself
//! rather than in who asked for it (the web-link importer, #70 — a page's own
//! text can tell an agent to fetch another URL, ADR 0033). Such a Job always
//! takes the single depth-one lane, member or not.
//!
//! A streaming progress mechanism, if one ever exists, may decorate this but is
//! never instead of it: the row in this table is where watching slow work reads.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use rusqlite::OptionalExtension;
use rusqlite::params;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::catalogue;
use crate::core::{Caller, ErrorKind, Invocation, OpError};
use crate::db::Db;

/// How often a client should look back at a running Job.
pub const POLL_INTERVAL_MS: i64 = 5000;

/// The handle through which running work reports its progress. Given to a
/// handler only while it runs as a Job; reporting is best-effort — a failed
/// progress write never fails the work itself, whose truth is its outcome.
#[derive(Clone)]
pub struct JobProgress {
    job_id: String,
    db: Arc<Db>,
}

impl JobProgress {
    pub fn new(job_id: &str, db: Arc<Db>) -> Self {
        JobProgress {
            job_id: job_id.to_string(),
            db,
        }
    }

    /// Say how far the work has got: how much done, of what total if it knows,
    /// and one sentence a reader can act on. Read back through `get_job`.
    pub fn report(&self, done: u64, total: Option<u64>, message: impl Into<String>) {
        let _ = self.db.with_conn(|conn| {
            conn.execute(
                "UPDATE jobs SET progress_done = ?2, progress_total = ?3,
                 progress_message = ?4,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
                 WHERE id = ?1",
                params![
                    self.job_id,
                    done as i64,
                    total.map(|t| t as i64),
                    message.into()
                ],
            )
            .map_err(|e| OpError::internal(format!("cannot record progress: {e}")))?;
            Ok(())
        });
    }
}

/// The member lane: two pieces of work at once, a bounded line behind them.
const MEMBER_WORKERS: usize = 2;
const MEMBER_LINE: usize = 16;
/// The stranger lane: one piece of work at once and nowhere to queue beyond a
/// handful. A stranger may cause work, never work that scales with them.
const STRANGER_WORKERS: usize = 1;
const STRANGER_LINE: usize = 4;

/// The five states a Job can be in. Kept an enum rather than bare strings so
/// every comparison and mapping spells the states in one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            JobStatus::Queued => "queued",
            JobStatus::Running => "running",
            JobStatus::Completed => "completed",
            JobStatus::Failed => "failed",
            JobStatus::Cancelled => "cancelled",
        }
    }

    fn from_column(value: String) -> Self {
        match value.as_str() {
            "running" => JobStatus::Running,
            "completed" => JobStatus::Completed,
            "failed" => JobStatus::Failed,
            "cancelled" => JobStatus::Cancelled,
            _ => JobStatus::Queued,
        }
    }
}

/// One Job as the database holds it: what was asked, by whom, how far it has
/// got, and how it ended. The row is the truth; this is that truth in memory.
#[derive(Debug)]
pub struct JobRecord {
    pub id: String,
    /// The Person who asked, when one did; None for work a stranger caused.
    pub person_id: Option<String>,
    pub read_only: bool,
    pub via_access_key: bool,
    pub operation: String,
    pub input: Value,
    pub status: JobStatus,
    pub progress_done: Option<i64>,
    pub progress_total: Option<i64>,
    pub progress_message: Option<String>,
    /// The eventual output, present once completed.
    pub result: Option<Value>,
    /// Why it failed: one sentence and the JSON-RPC code that carries it.
    pub error: Option<String>,
    pub error_code: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

/// The sending ends of both lanes, held by the Core.
pub struct Lanes {
    pub member: mpsc::Sender<String>,
    pub stranger: mpsc::Sender<String>,
}

/// The receiving ends, consumed once when the workers are spawned. Each lane's
/// receiver is shared by its workers, so they compete for the next Job.
pub struct LaneReceivers {
    member: Arc<tokio::sync::Mutex<mpsc::Receiver<String>>>,
    stranger: Arc<tokio::sync::Mutex<mpsc::Receiver<String>>>,
}

/// Build both lanes. Called once per Core.
pub fn lanes() -> (Lanes, LaneReceivers) {
    let (member_tx, member_rx) = mpsc::channel(MEMBER_LINE);
    let (stranger_tx, stranger_rx) = mpsc::channel(STRANGER_LINE);
    (
        Lanes {
            member: member_tx,
            stranger: stranger_tx,
        },
        LaneReceivers {
            member: Arc::new(tokio::sync::Mutex::new(member_rx)),
            stranger: Arc::new(tokio::sync::Mutex::new(stranger_rx)),
        },
    )
}

/// Start the workers that carry Jobs from the lanes. Each worker takes work off
/// its own lane only — members never wait behind strangers.
pub fn spawn_workers(core: Arc<crate::core::Core>, receivers: LaneReceivers) {
    for _ in 0..MEMBER_WORKERS {
        let core = core.clone();
        let rx = receivers.member.clone();
        tokio::spawn(async move {
            loop {
                let job_id = rx.lock().await.recv().await;
                match job_id {
                    Some(job_id) => carry(core.clone(), job_id).await,
                    None => return, // the lane is gone; nothing more to carry
                }
            }
        });
    }
    for _ in 0..STRANGER_WORKERS {
        let core = core.clone();
        let rx = receivers.stranger.clone();
        tokio::spawn(async move {
            while let Some(job_id) = rx.lock().await.recv().await {
                carry(core.clone(), job_id).await;
            }
        });
    }
}

async fn carry(core: Arc<crate::core::Core>, job_id: String) {
    // The database is behind a mutex and handlers block on it; keep them off the
    // async worker threads by carrying the work on the blocking pool.
    let _ = tokio::task::spawn_blocking(move || core.run_job(&job_id)).await;
}

/// Called once at startup, before any Door answers: work a previous process
/// accepted must not strand. Running work can't be resumed honestly — the
/// handler died with the process — so it reports why it ended; queued work
/// re-enters its lane and is carried as if just asked for.
pub fn recover_at_startup(db: &Arc<Db>, lanes: &Lanes) {
    let _ = db.with_conn(|conn| {
        let interrupted = conn
            .execute(
                "UPDATE jobs SET status = 'failed',
             error = 'this Job was interrupted when Kamosu restarted',
             error_code = -32603,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE status = 'running'",
                [],
            )
            .map_err(|e| OpError::internal(format!("cannot recover interrupted Jobs: {e}")))?;
        if interrupted > 0 {
            tracing::warn!(
                "{interrupted} Job(s) were mid-flight at shutdown; marked failed with the reason"
            );
        }
        Ok(())
    });

    let waiting: Vec<(String, bool)> = match db.with_conn(|conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, person_id IS NULL FROM jobs
                 WHERE status = 'queued' ORDER BY created_at",
            )
            .map_err(|e| OpError::internal(format!("cannot list waiting Jobs: {e}")))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? != 0))
            })
            .map_err(|e| OpError::internal(format!("cannot list waiting Jobs: {e}")))?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }) {
        Ok(rows) => rows,
        Err(err) => {
            tracing::warn!("cannot recover waiting Jobs: {err}");
            return;
        }
    };

    for (job_id, is_stranger_work) in waiting {
        let lane = if is_stranger_work {
            &lanes.stranger
        } else {
            &lanes.member
        };
        if lane.try_send(job_id.clone()).is_err() {
            // The line was already full at boot; refuse honestly rather than
            // leave the row pretending to wait forever.
            let _ = db.with_conn(|conn| {
                conn.execute(
                    "UPDATE jobs SET status = 'failed',
                     error = 'refused when Kamosu restarted: the lane for this Job was full',
                     error_code = -32603,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
                     WHERE id = ?1",
                    params![job_id],
                )
                .map_err(|e| OpError::internal(format!("cannot fail recovered Job: {e}")))?;
                Ok(())
            });
        }
    }
}

impl crate::core::Core {
    /// Carry one Job from its row to its end state. Runs on the blocking pool.
    pub(crate) fn run_job(&self, job_id: &str) {
        let record = match self.job(job_id) {
            // A row nobody knows has nothing to carry.
            Ok(Some(record)) => record,
            Ok(None) => return,
            Err(_) => return,
        };
        // Only a queued row is ours to carry: a cancelled one is skipped, and
        // anything else has already ended some other way.
        if record.status != JobStatus::Queued {
            return;
        }

        set_running(self, job_id);

        let outcome = catch_unwind(AssertUnwindSafe(|| {
            match catalogue::find(&record.operation) {
                Some(op) => {
                    let caller = record.person_id.clone().map(|person_id| Caller {
                        person_id,
                        read_only: record.read_only,
                        via_access_key: record.via_access_key,
                        is_operator: false,
                        // A JobRecord does not carry which Access Key asked, only
                        // that one did (`via_access_key`) — nothing built on a Job
                        // needs the specific Key today.
                        access_key_id: None,
                    });
                    let invocation = Invocation {
                        caller,
                        job: Some(JobProgress::new(job_id, self.db().clone())),
                    };
                    (op.handler)(self, &invocation, record.input.clone())
                }
                None => Err(OpError::internal(format!(
                    "the Catalogue no longer knows '{}'",
                    record.operation
                ))),
            }
        }));

        match outcome {
            Ok(Ok(value)) => finish(
                self,
                job_id,
                "completed",
                Some(value.to_string()),
                None,
                None,
            ),
            Ok(Err(err)) => finish(
                self,
                job_id,
                "failed",
                None,
                Some(err.to_sentence()),
                Some(rpc_code(err.kind)),
            ),
            // A panic is a failure like any other: reported through the same
            // Operations rather than vanishing with the request.
            Err(_) => finish(
                self,
                job_id,
                "failed",
                None,
                Some("this Operation failed unexpectedly".to_string()),
                Some(-32603),
            ),
        }
    }
}

/// Map an Operation failure onto the JSON-RPC code the MCP door reports when the
/// failed Job is read back through the long-running-task extension.
fn rpc_code(kind: ErrorKind) -> i64 {
    match kind {
        ErrorKind::BadRequest | ErrorKind::UnknownOperation | ErrorKind::NotFound => -32602,
        ErrorKind::Unauthorized => -32001,
        ErrorKind::Busy | ErrorKind::Internal => -32603,
    }
}

/// Record that slow work was asked for, before any answer goes out: by the time
/// the caller sees the job id, `get_job` must be able to see the Job too.
pub fn record(
    core: &crate::core::Core,
    operation: &str,
    invocation: &Invocation,
    input: Value,
) -> Result<String, OpError> {
    let job_id = new_id();
    let (person_id, read_only, via_access_key) = match &invocation.caller {
        Some(caller) => (
            Some(caller.person_id.clone()),
            caller.read_only,
            caller.via_access_key,
        ),
        None => (None, false, false),
    };
    core.db().with_conn(|conn| {
        conn.execute(
            "INSERT INTO jobs (id, person_id, read_only, via_access_key, operation, input)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                job_id,
                person_id,
                read_only as i64,
                via_access_key as i64,
                operation,
                input.to_string()
            ],
        )
        .map_err(|e| OpError::internal(format!("cannot record Job: {e}")))?;
        Ok(())
    })?;
    Ok(job_id)
}

/// Remove the row of work that was refused before anyone saw its id.
pub fn forget(core: &crate::core::Core, job_id: &str) -> Result<(), OpError> {
    core.db().with_conn(|conn| {
        conn.execute("DELETE FROM jobs WHERE id = ?1", params![job_id])
            .map_err(|e| OpError::internal(format!("cannot forget refused Job: {e}")))?;
        Ok(())
    })
}

/// Read one Job back from the database.
pub fn read(core: &crate::core::Core, job_id: &str) -> Result<Option<JobRecord>, OpError> {
    core.db().with_conn(|conn| {
        conn.prepare(
            "SELECT id, person_id, read_only, via_access_key, operation, input, status,
                        progress_done, progress_total, progress_message,
                        result, error, error_code, created_at, updated_at
                 FROM jobs WHERE id = ?1",
        )
        .map_err(|e| OpError::internal(format!("cannot read Job: {e}")))?
        .query_row(params![job_id], map_record)
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read Job: {e}")))
    })
}

/// Every Job one Person has asked for, newest first, most recent hundred.
pub fn read_of_person(
    core: &crate::core::Core,
    person_id: &str,
) -> Result<Vec<JobRecord>, OpError> {
    core.db().with_conn(|conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, person_id, read_only, via_access_key, operation, input, status,
                        progress_done, progress_total, progress_message,
                        result, error, error_code, created_at, updated_at
                 FROM jobs WHERE person_id = ?1
                 ORDER BY created_at DESC LIMIT 100",
            )
            .map_err(|e| OpError::internal(format!("cannot list Jobs: {e}")))?;
        let rows = stmt
            .query_map(params![person_id], map_record)
            .map_err(|e| OpError::internal(format!("cannot list Jobs: {e}")))?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    })
}

fn map_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<JobRecord> {
    Ok(JobRecord {
        id: row.get(0)?,
        person_id: row.get(1)?,
        read_only: row.get::<_, i64>(2)? != 0,
        via_access_key: row.get::<_, i64>(3)? != 0,
        operation: row.get(4)?,
        input: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or(Value::Null),
        status: JobStatus::from_column(row.get(6)?),
        progress_done: row.get(7)?,
        progress_total: row.get(8)?,
        progress_message: row.get(9)?,
        result: row
            .get::<_, Option<String>>(10)?
            .map(|t| serde_json::from_str(&t).unwrap_or(Value::Null)),
        error: row.get(11)?,
        error_code: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

/// Cancel a Job cooperatively: honoured while it still waits in line, acked
/// either way. Running work decides for itself whether to notice.
pub fn cancel_if_queued(core: &crate::core::Core, job_id: &str) -> Result<bool, OpError> {
    core.db().with_conn(|conn| {
        let changed = conn
            .execute(
                "UPDATE jobs SET status = 'cancelled',
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
                 WHERE id = ?1 AND status = 'queued'",
                params![job_id],
            )
            .map_err(|e| OpError::internal(format!("cannot cancel Job: {e}")))?;
        Ok(changed > 0)
    })
}

/// Who may read a Job: the Person who asked for it — or anyone, when no Person
/// did. Work caused anonymously (a Sheet rendered for a Share Link, say) carries
/// nothing that was not already public, so hiding its state would hide nothing.
pub fn ensure_reader(record: &JobRecord, caller: &Option<Caller>) -> Result<(), OpError> {
    match (&record.person_id, caller) {
        (None, _) => Ok(()),
        (Some(owner), Some(c)) if owner.as_str() == c.person_id => Ok(()),
        _ => Err(OpError::unauthorized("no Job with that id")),
    }
}

fn set_running(core: &crate::core::Core, job_id: &str) {
    let _ = core.db().with_conn(|conn| {
        conn.execute(
            "UPDATE jobs SET status = 'running',
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?1",
            params![job_id],
        )
        .map_err(|e| OpError::internal(format!("cannot start Job: {e}")))?;
        Ok(())
    });
}

#[allow(clippy::too_many_arguments)]
fn finish(
    core: &crate::core::Core,
    job_id: &str,
    status: &str,
    result: Option<String>,
    error: Option<String>,
    error_code: Option<i64>,
) {
    let _ = core.db().with_conn(|conn| {
        conn.execute(
            "UPDATE jobs SET status = ?2, result = ?3, error = ?4, error_code = ?5,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?1",
            params![job_id, status, result, error, error_code],
        )
        .map_err(|e| OpError::internal(format!("cannot finish Job: {e}")))?;
        Ok(())
    });
}

/// Render one Job as the ordinary Operation output both Doors serve.
pub fn to_value(record: &JobRecord) -> Value {
    json!({
        "id": record.id,
        "operation": record.operation,
        "status": record.status.as_str(),
        "progress": {
            "done": record.progress_done,
            "total": record.progress_total,
            "message": record.progress_message,
        },
        "result": record.result,
        "error": record.error,
        "errorCode": record.error_code,
        "created_at": record.created_at,
        "updated_at": record.updated_at,
    })
}

/// Render one Job as a summary for `list_jobs`.
pub fn to_summary(record: &JobRecord) -> Value {
    json!({
        "id": record.id,
        "operation": record.operation,
        "status": record.status.as_str(),
        "created_at": record.created_at,
    })
}

fn new_id() -> String {
    // A Job id names work; it grants nothing, so plain randomness suffices and
    // the leading letter keeps it legibly not a Secret.
    let mut bytes = [0u8; 12];
    rand::fill(&mut bytes);
    format!("j_{}", hex::encode(bytes))
}
