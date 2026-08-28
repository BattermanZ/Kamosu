//! The Core: where Operations are carried out and where permission is checked.
//! It is reached only through a Door (or the terminal) and knows nothing about
//! how a request arrived.
//!
//! Authorisation lives here, beneath both Doors, keyed on a Credential. A
//! permission check written inside a Door is a bug.

use std::sync::Arc;

use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use rand::{TryRng, rngs::SysRng};
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::params;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::catalogue::{self, Kind, Permission};
use crate::db::Db;
use crate::jobs::{self, JobProgress, JobRecord};
use crate::photographs;

/// What went wrong with an Operation, in words a caller can act on. The Doors map
/// these to their own transports; they never decide them.
#[derive(Debug, Clone)]
pub struct OpError {
    pub kind: ErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// The Credential was presented but does not resolve: unknown, revoked or spent.
    Unauthorized,
    /// No such Operation exists in the Catalogue.
    UnknownOperation,
    /// The input envelope does not fit the Operation's declaration.
    BadRequest,
    /// No such thing — a Job id that names nothing, read by its owner or not at all.
    NotFound,
    /// The work is refused rather than queued: the lane asked for is full.
    /// A stranger may cause work, never work that scales with them (ADR 0032) —
    /// a full line answers *busy* so collapse becomes latency instead.
    Busy,
    /// Kamosu itself failed. Never the caller's fault.
    Internal,
}

impl OpError {
    pub fn unauthorized(message: impl Into<String>) -> Self {
        OpError {
            kind: ErrorKind::Unauthorized,
            message: message.into(),
        }
    }
    pub fn unknown_operation(name: &str) -> Self {
        OpError {
            kind: ErrorKind::UnknownOperation,
            message: format!("no Operation named '{name}' exists in the Catalogue"),
        }
    }
    pub fn bad_request(message: impl Into<String>) -> Self {
        OpError {
            kind: ErrorKind::BadRequest,
            message: message.into(),
        }
    }
    pub fn not_found(message: impl Into<String>) -> Self {
        OpError {
            kind: ErrorKind::NotFound,
            message: message.into(),
        }
    }
    pub fn busy() -> Self {
        OpError {
            kind: ErrorKind::Busy,
            message: "Kamosu is busy right now — try again in a moment".to_string(),
        }
    }
    pub fn internal(message: impl Into<String>) -> Self {
        OpError {
            kind: ErrorKind::Internal,
            message: message.into(),
        }
    }

    /// A courteous sentence for the caller; safe to show through either Door.
    pub fn to_sentence(&self) -> String {
        self.message.clone()
    }
}

impl std::fmt::Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

/// Who is acting: a Person resolved from a Credential. An agent is never a Person
/// — it acts as one, under that Person's Hand.
#[derive(Debug, Clone)]
pub struct Caller {
    pub person_id: String,
    /// True when the Credential came from an Access Key minted read-only.
    /// `Core::execute` refuses every Operation the Catalogue declares `write`
    /// to a caller carrying this (ADR 0031).
    pub read_only: bool,
    /// True when the Credential came from an Access Key rather than a login
    /// Session. `Core::execute` refuses every Operation the Catalogue declares
    /// `session_only` to a caller carrying this, however unrestricted the Key.
    pub via_access_key: bool,
    /// Whether this Person administers the instance.
    pub is_operator: bool,
    /// The Access Key that resolved this Credential, if any — never set for a
    /// login Session. Local only: a Version records it (CONTEXT.md, "Version")
    /// so it never travels in a Bundle. No Operation surfaces it back yet —
    /// that is for whichever future ticket reads a Version's full detail.
    pub access_key_id: Option<String>,
}

/// One asking of an Operation: who is acting, and (when the Operation runs as a
/// Job) the handle through which the running work reports its progress.
#[derive(Clone)]
struct Session {
    id: String,
    secret: String,
}

/// The answer to minting an Access Key: the raw Secret, shown once and never
/// stored, alongside what the Key is known by afterwards.
pub struct AccessKey {
    pub id: String,
    pub secret: String,
    pub name: String,
    pub read_only: bool,
}

/// One row `lookup_credential` can find: `(person_id, read_only, is_session,
/// is_operator, access_key_id)`.
type CredentialLookup = (String, bool, bool, bool, Option<String>);

pub struct Invocation {
    pub caller: Option<Caller>,
    /// Set only while an Operation declared `Kind::Job` is being carried out.
    pub job: Option<JobProgress>,
}

/// The Core. Holds the database — the truth (ADR 0003) — and the Job lanes that
/// carry slow work (ADR 0032): one for members, one depth-one lane for strangers.
pub struct Core {
    db: Arc<Db>,
    lanes: jobs::Lanes,
}

impl Core {
    pub fn open(db: Arc<Db>) -> Core {
        // Terminal commands open a Core without Job workers; they never ask for
        // slow work. A Job asked for here would be refused as busy.
        let (lanes, _) = jobs::lanes();
        Core { db, lanes }
    }

    /// Open the database *and* start carrying Jobs: both Doors share this entry.
    /// The lanes exist from the moment the instance serves — and anything a
    /// previous process left behind is recovered before the first answer goes out.
    pub fn start(db: Arc<Db>) -> Arc<Core> {
        let (lanes, receivers) = jobs::lanes();
        jobs::recover_at_startup(&db, &lanes);
        let core = Core { db, lanes };
        let core = Arc::new(core);
        jobs::spawn_workers(core.clone(), receivers);
        core
    }

    /// The database, shared. An Arc clone keeps Job progress reporting able to
    /// write while the Core itself is borrowed elsewhere.
    pub fn db(&self) -> Arc<Db> {
        self.db.clone()
    }

    /// Where Kamosu's truth lives: one SQLite file under the one data directory.
    pub fn database_path(&self) -> std::path::PathBuf {
        self.db.database_path()
    }

    /// Carry out one Operation on behalf of whatever presented itself.
    ///
    /// This is the whole authorisation surface of Kamosu: resolve the Credential,
    /// check the permission the Catalogue declares, then perform. Both Doors call
    /// exactly this and add no checks of their own — that is Parity working.
    pub fn execute(
        &self,
        credential_secret: Option<&str>,
        operation_name: &str,
        input: Value,
    ) -> Result<Value, OpError> {
        let op = catalogue::find(operation_name)
            .ok_or_else(|| OpError::unknown_operation(operation_name))?;

        // Resolving a Credential is not an Operation (it precedes every one), but
        // presenting an unrecognised Secret must fail loudly rather than silently
        // become a stranger.
        let caller = match credential_secret.filter(|secret| !secret.is_empty()) {
            None => None,
            Some(secret) => Some(self.resolve_credential(secret)?),
        };

        match op.permission {
            Permission::Public => {}
            Permission::Person => {
                if caller.is_none() {
                    return Err(OpError::unauthorized(format!(
                        "Operation '{}' requires a Credential naming a Person",
                        op.name
                    )));
                }
            }
            Permission::Operator => {
                if !caller.as_ref().is_some_and(|caller| caller.is_operator) {
                    return Err(OpError::unauthorized(format!(
                        "Operation '{}' requires a Credential naming an Operator",
                        op.name
                    )));
                }
            }
        }

        // The read-only filter and the session-only rule both live here, beneath
        // both Doors, alongside the permission check above — a Door adding either
        // check itself would be the same bug as a Door checking permission.
        if let Some(caller) = &caller {
            if op.write && caller.read_only {
                return Err(OpError::unauthorized(format!(
                    "Operation '{}' writes, and this Credential is a read-only Access Key",
                    op.name
                )));
            }
            if op.session_only && caller.via_access_key {
                return Err(OpError::unauthorized(format!(
                    "Operation '{}' may be performed only by a Person logged in directly, \
                     never by an Access Key",
                    op.name
                )));
            }
        }

        let invocation = Invocation { caller, job: None };

        match op.kind {
            Kind::Immediate => (op.handler)(self, &invocation, input),
            Kind::Job => self.ask_job(op.name, &invocation, input),
        }
    }

    /// Accept slow work and answer at once with an id. The work itself runs in a
    /// lane: members never wait behind strangers (ADR 0032). A full line refuses
    /// rather than grows — the caller is told busy, try again in a moment.
    fn ask_job(
        &self,
        operation_name: &str,
        invocation: &Invocation,
        input: Value,
    ) -> Result<Value, OpError> {
        let job_id = jobs::record(self, operation_name, invocation, input)?;
        let lane = if invocation.caller.is_some() {
            &self.lanes.member
        } else {
            &self.lanes.stranger
        };
        if lane.try_send(job_id.clone()).is_err() {
            // Refused outright: the row must not linger as work nobody carries.
            let _ = jobs::forget(self, &job_id);
            return Err(OpError::busy());
        }
        Ok(json!({ "job_id": job_id }))
    }

    /// Read one Job back: its state, progress, and result or failure reason.
    pub fn job(&self, job_id: &str) -> Result<Option<JobRecord>, OpError> {
        jobs::read(self, job_id)
    }

    /// Every Job one Person has asked for, newest first.
    pub fn jobs_of(&self, person_id: &str) -> Result<Vec<JobRecord>, OpError> {
        jobs::read_of_person(self, person_id)
    }

    /// Cancel a Job cooperatively: honoured while it still waits in line, acked
    /// either way. Running work decides for itself whether to notice.
    pub fn cancel_job_if_queued(&self, job_id: &str) -> Result<bool, OpError> {
        jobs::cancel_if_queued(self, job_id)
    }

    /// Look up what a Secret's hash names, without marking it used: `(person_id,
    /// read_only, is_session)`, or nothing for an unknown or revoked Secret.
    /// `resolve_credential` builds on this and additionally records the use;
    /// `is_read_only_credential` reads only, so a Door merely listing what a
    /// Key may do never counts as the Key being used.
    fn lookup_credential(&self, hash: &str) -> Result<Option<CredentialLookup>, OpError> {
        self.db().with_conn(|conn| {
            conn.query_row(
                "SELECT access_keys.person_id, access_keys.read_only, 0, people.is_operator, access_keys.id
                   FROM access_keys JOIN people ON people.id = access_keys.person_id
                   WHERE access_keys.secret_hash = ?1 AND access_keys.revoked = 0 AND people.disabled = 0 AND people.deleted = 0
                 UNION ALL
                 SELECT sessions.person_id, 0, 1, people.is_operator, NULL
                   FROM sessions JOIN people ON people.id = sessions.person_id
                   WHERE sessions.secret_hash = ?1 AND sessions.revoked = 0 AND people.disabled = 0 AND people.deleted = 0
                 LIMIT 1",
                params![hash],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .optional()
            .map_err(|e| OpError::internal(format!("cannot resolve Credential: {e}")))
        })
    }

    /// Resolve a raw Secret to the Person it acts as: an Access Key today; a login
    /// Session joins when accounts do. Stored hashed, never in the clear.
    fn resolve_credential(&self, secret: &str) -> Result<Caller, OpError> {
        let hash = hash_secret(secret);
        match self.lookup_credential(&hash)? {
            Some((person_id, read_only, is_session, is_operator, access_key_id)) => {
                let _ = self.db().with_conn(|conn| {
                    let table = if is_session { "sessions" } else { "access_keys" };
                    conn.execute(
                        &format!("UPDATE {table} SET last_used_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE secret_hash = ?1"),
                        params![hash],
                    )
                    .map_err(|e| OpError::internal(format!("cannot record Credential use: {e}")))?;
                    Ok(())
                });
                Ok(Caller {
                    person_id,
                    read_only,
                    via_access_key: !is_session,
                    is_operator,
                    access_key_id,
                })
            }
            // Unknown or already-revoked: the same answer either way, saying nothing
            // about which.
            None => Err(OpError::unauthorized(
                "this Credential does not name anyone",
            )),
        }
    }

    /// Whether a Secret resolves to a read-only Access Key — for the MCP door's
    /// tool listing alone, which is not itself an Operation and so does not go
    /// through `execute` and must not mark the Key used just for being listed
    /// against. An unresolvable or absent Secret is simply not read-only: the
    /// listing itself requires no Credential.
    pub fn is_read_only_credential(&self, secret: &str) -> bool {
        let hash = hash_secret(secret);
        self.lookup_credential(&hash)
            .ok()
            .flatten()
            .map(|(_, read_only, _, _, _)| read_only)
            .unwrap_or(false)
    }

    /// Mint an Access Key for a Person. Plumbing beneath both Doors and the
    /// terminal — obtaining a Credential is not an Operation. The raw Secret is
    /// returned once and stored only as its hash.
    pub fn mint_access_key(
        &self,
        person_id: &str,
        key_name: &str,
        read_only: bool,
    ) -> Result<AccessKey, OpError> {
        let key_name = required_text(key_name, "name")?.to_string();
        let secret = generate_secret();
        let hash = hash_secret(&secret);
        let id = format!("ak_{}", hex::encode(random_bytes(8)));
        self.db().with_conn(|conn| {
            let exists: bool = conn
                .query_row("SELECT COUNT(*) FROM people WHERE id = ?1", params![person_id], |r| r.get(0))
                .map(|n: i64| n > 0)
                .map_err(|e| OpError::internal(e.to_string()))?;
            if !exists {
                return Err(OpError::bad_request(format!("no Person '{person_id}'")));
            }
            conn.execute(
                "INSERT INTO access_keys (id, secret_hash, person_id, name, read_only) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, hash, person_id, key_name, read_only as i64],
            )
            .map_err(|e| OpError::internal(format!("cannot mint access key: {e}")))?;
            Ok(())
        })?;
        Ok(AccessKey {
            id,
            secret,
            name: key_name,
            read_only,
        })
    }

    /// Access Keys remain knowable by their name and last use until revoked;
    /// their Secret never appears in this record — only its one-time minting
    /// answer carries it.
    pub fn access_keys_of(&self, person_id: &str) -> Result<Vec<Value>, OpError> {
        self.db().with_conn(|conn| {
            let mut statement = conn.prepare(
                "SELECT id, name, read_only, created_at, last_used_at, revoked FROM access_keys WHERE person_id = ?1 ORDER BY created_at DESC",
            ).map_err(|e| OpError::internal(format!("cannot list Access Keys: {e}")))?;
            statement.query_map(params![person_id], |row| Ok(json!({
                "id": row.get::<_, String>(0)?, "name": row.get::<_, String>(1)?,
                "read_only": row.get::<_, i64>(2)? != 0,
                "created_at": row.get::<_, String>(3)?, "last_used_at": row.get::<_, Option<String>>(4)?,
                "revoked": row.get::<_, i64>(5)? != 0,
            }))).map_err(|e| OpError::internal(format!("cannot read Access Keys: {e}")))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| OpError::internal(format!("cannot read Access Keys: {e}")))
        })
    }

    pub fn revoke_access_key(&self, person_id: &str, key_id: &str) -> Result<(), OpError> {
        let changed = self.db().with_conn(|conn| {
            conn.execute(
                "UPDATE access_keys SET revoked = 1 WHERE id = ?1 AND person_id = ?2 AND revoked = 0",
                params![key_id, person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot revoke Access Key: {e}")))
        })?;
        if changed == 0 {
            return Err(OpError::not_found(
                "no live Access Key with that id belongs to this Person",
            ));
        }
        Ok(())
    }

    /// The fresh-instance door: one Person wins it, becomes the Operator, and
    /// receives the Kitchen and Hand that make a Person a complete account.
    pub fn create_first_person(
        &self,
        name: &str,
        password: &str,
        session_name: &str,
    ) -> Result<Value, OpError> {
        let name = required_text(name, "name")?;
        let password_hash = hash_password(password)?;
        let session_name = required_text(session_name, "session_name")?;
        let person_id = format!("p_{}", hex::encode(random_bytes(8)));
        let kitchen_id = format!("k_{}", hex::encode(random_bytes(8)));
        let session = Session {
            id: format!("s_{}", hex::encode(random_bytes(8))),
            secret: generate_secret(),
        };

        self.db().with_conn(|conn| {
            conn.execute_batch("BEGIN IMMEDIATE")
                .map_err(|e| OpError::internal(format!("cannot begin first-person setup: {e}")))?;
            let created = (|| -> Result<(), OpError> {
                if conn.execute(
                    "INSERT OR IGNORE INTO instance_setup(singleton) VALUES (1)",
                    [],
                ).map_err(|e| OpError::internal(format!("cannot reserve first-person setup: {e}")))? == 0 {
                    return Err(OpError::unauthorized("this instance already has its first Person"));
                }
                conn.execute(
                    "INSERT INTO people (id, name, password_hash, home_kitchen_id, is_operator) VALUES (?1, ?2, ?3, ?4, 1)",
                    params![person_id, name, password_hash, kitchen_id],
                ).map_err(|e| OpError::internal(format!("cannot create first Person: {e}")))?;
                conn.execute(
                    "UPDATE instance_setup SET operator_person_id = ?1 WHERE singleton = 1",
                    params![person_id],
                ).map_err(|e| OpError::internal(format!("cannot name first Operator: {e}")))?;
                conn.execute(
                    "INSERT INTO kitchens (id, name, hand_id) VALUES (?1, ?2, ?1)",
                    params![kitchen_id, format!("{}'s Home Kitchen", name)],
                ).map_err(|e| OpError::internal(format!("cannot create Home Kitchen: {e}")))?;
                conn.execute(
                    "INSERT INTO kitchen_members (kitchen_id, person_id) VALUES (?1, ?2)",
                    params![kitchen_id, person_id],
                ).map_err(|e| OpError::internal(format!("cannot add first Person to Home Kitchen: {e}")))?;
                conn.execute(
                    "INSERT INTO sessions (id, secret_hash, person_id, name) VALUES (?1, ?2, ?3, ?4)",
                    params![session.id, hash_secret(&session.secret), person_id, session_name],
                ).map_err(|e| OpError::internal(format!("cannot mint first Session: {e}")))?;
                Ok(())
            })();
            match created {
                Ok(()) => conn.execute_batch("COMMIT").map_err(|e| OpError::internal(format!("cannot finish first-person setup: {e}"))),
                Err(err) => {
                    let _ = conn.execute_batch("ROLLBACK");
                    Err(err)
                }
            }
        })?;

        Ok(json!({
            "person": {
                "id": person_id,
                "name": name,
                "hand_id": person_id,
                "home_kitchen_id": kitchen_id,
                "reading_language": "en",
                "reading_measures": "us",
                "is_operator": true,
            },
            "_session_secret": session.secret,
            "session_id": session.id,
        }))
    }

    /// Check a human-chosen password and mint the browser Session it unlocks.
    pub fn log_in(&self, name: &str, password: &str, session_name: &str) -> Result<Value, OpError> {
        let name = required_text(name, "name")?;
        let session_name = required_text(session_name, "session_name")?;
        self.throttle_login(name)?;
        let password_hash: Option<String> = self.db().with_conn(|conn| {
            conn.query_row(
                "SELECT password_hash FROM people WHERE name = ?1 AND disabled = 0 AND deleted = 0",
                params![name],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| OpError::internal(format!("cannot read Person for login: {e}")))
        })?;
        let verified = password_hash.as_deref().is_some_and(|encoded| {
            PasswordHash::new(encoded)
                .ok()
                .and_then(|parsed| {
                    Argon2::default()
                        .verify_password(password.as_bytes(), &parsed)
                        .ok()
                })
                .is_some()
        });
        if !verified {
            self.record_login_failure(name)?;
            return Err(OpError::unauthorized("that name and password do not match"));
        }
        self.db().with_conn(|conn| {
            conn.execute("DELETE FROM login_failures WHERE name = ?1", params![name])
                .map_err(|e| OpError::internal(format!("cannot clear login throttle: {e}")))?;
            Ok(())
        })?;
        let person_id: String = self.db().with_conn(|conn| {
            conn.query_row(
                "SELECT id FROM people WHERE name = ?1 AND disabled = 0 AND deleted = 0",
                params![name],
                |r| r.get(0),
            )
            .map_err(|e| OpError::internal(format!("cannot read logged-in Person: {e}")))
        })?;
        let session = self.mint_session(&person_id, session_name)?;
        Ok(json!({ "session_id": session.id, "_session_secret": session.secret }))
    }

    pub fn set_reading_preferences(
        &self,
        person_id: &str,
        language: &str,
        measures: &str,
    ) -> Result<(), OpError> {
        if !matches!(language, "en" | "fr" | "es")
            || !matches!(measures, "us" | "metric" | "as_written")
        {
            return Err(OpError::bad_request(
                "reading_language and reading_measures are not supported",
            ));
        }
        self.db().with_conn(|conn| {
            conn.execute(
                "UPDATE people SET reading_language = ?1, reading_measures = ?2 WHERE id = ?3",
                params![language, measures, person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot set reading preferences: {e}")))?;
            Ok(())
        })
    }

    /// A Person's name is a reminder, not their Hand's identity: records keep
    /// the permanent Person id and render this current value when read.
    pub fn rename_person(&self, person_id: &str, name: &str) -> Result<(), OpError> {
        let name = required_text(name, "name")?;
        self.db().with_conn(|conn| {
            conn.execute(
                "UPDATE people SET name = ?1 WHERE id = ?2",
                params![name, person_id],
            )
            .map_err(|e| OpError::bad_request(format!("cannot use that name: {e}")))?;
            Ok(())
        })
    }

    /// Sessions remain knowable by their device name and last use until their
    /// owner revokes them; their Secret never appears in this record.
    pub fn sessions_of(&self, person_id: &str) -> Result<Vec<Value>, OpError> {
        self.db().with_conn(|conn| {
            let mut statement = conn.prepare(
                "SELECT id, name, created_at, last_used_at, revoked FROM sessions WHERE person_id = ?1 ORDER BY created_at DESC",
            ).map_err(|e| OpError::internal(format!("cannot list Sessions: {e}")))?;
            statement.query_map(params![person_id], |row| Ok(json!({
                "id": row.get::<_, String>(0)?, "name": row.get::<_, String>(1)?,
                "created_at": row.get::<_, String>(2)?, "last_used_at": row.get::<_, Option<String>>(3)?,
                "revoked": row.get::<_, i64>(4)? != 0,
            }))).map_err(|e| OpError::internal(format!("cannot read Sessions: {e}")))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| OpError::internal(format!("cannot read Sessions: {e}")))
        })
    }

    pub fn revoke_session(&self, person_id: &str, session_id: &str) -> Result<(), OpError> {
        let changed = self.db().with_conn(|conn| {
            conn.execute(
                "UPDATE sessions SET revoked = 1 WHERE id = ?1 AND person_id = ?2 AND revoked = 0",
                params![session_id, person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot revoke Session: {e}")))
        })?;
        if changed == 0 {
            return Err(OpError::not_found(
                "no live Session by that name belongs to this Person",
            ));
        }
        Ok(())
    }

    fn mint_session(&self, person_id: &str, name: &str) -> Result<Session, OpError> {
        let secret = generate_secret();
        let id = format!("s_{}", hex::encode(random_bytes(8)));
        self.db().with_conn(|conn| {
            conn.execute(
                "INSERT INTO sessions (id, secret_hash, person_id, name) VALUES (?1, ?2, ?3, ?4)",
                params![id, hash_secret(&secret), person_id, name],
            )
            .map_err(|e| OpError::internal(format!("cannot mint Session: {e}")))?;
            Ok(())
        })?;
        Ok(Session { id, secret })
    }

    /// A wrong password slows only the next attempt for this name: two free
    /// misses, then 1, 2, 4… seconds capped at 30. A correct password deletes
    /// the counter, so a stranger cannot lock an account out (ADR 0031).
    fn throttle_login(&self, name: &str) -> Result<(), OpError> {
        let failures: i64 = self.db().with_conn(|conn| {
            conn.query_row(
                "SELECT consecutive_failures FROM login_failures WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )
            .optional()
            .map(|count| count.unwrap_or(0))
            .map_err(|e| OpError::internal(format!("cannot read login throttle: {e}")))
        })?;
        let seconds = if failures < 2 {
            0
        } else {
            1u64 << (failures - 2).min(5)
        };
        if seconds > 0 {
            std::thread::sleep(std::time::Duration::from_secs(seconds));
        }
        Ok(())
    }

    fn record_login_failure(&self, name: &str) -> Result<(), OpError> {
        self.db().with_conn(|conn| {
            conn.execute("INSERT INTO login_failures(name, consecutive_failures) VALUES (?1, 1) ON CONFLICT(name) DO UPDATE SET consecutive_failures = consecutive_failures + 1", params![name])
                .map_err(|e| OpError::internal(format!("cannot record login failure: {e}")))?;
            Ok(())
        })
    }

    pub fn mint_invite(&self, is_operator: bool) -> Result<String, OpError> {
        let secret = generate_secret();
        self.db().with_conn(|conn| {
            conn.execute("INSERT INTO account_links (secret_hash, kind, is_operator) VALUES (?1, 'invite', ?2)", params![hash_secret(&secret), is_operator as i64])
                .map_err(|e| OpError::internal(format!("cannot mint Invite: {e}")))?;
            Ok(())
        })?;
        Ok(format!("/invite/{secret}"))
    }

    pub fn redeem_invite(
        &self,
        link: &str,
        name: &str,
        password: &str,
        session_name: &str,
    ) -> Result<Value, OpError> {
        let secret = link
            .strip_prefix("/invite/")
            .filter(|s| !s.is_empty())
            .ok_or_else(|| OpError::bad_request("Invite link is invalid"))?;
        let name = required_text(name, "name")?;
        let password_hash = hash_password(password)?;
        let session_name = required_text(session_name, "session_name")?;
        let person_id = format!("p_{}", hex::encode(random_bytes(8)));
        let kitchen_id = format!("k_{}", hex::encode(random_bytes(8)));
        let session = Session {
            id: format!("s_{}", hex::encode(random_bytes(8))),
            secret: generate_secret(),
        };
        let operator = self.db().with_conn(|conn| {
            conn.execute_batch("BEGIN IMMEDIATE").map_err(|e| OpError::internal(e.to_string()))?;
            let result = (|| -> Result<bool, OpError> {
                let role: Option<i64> = conn.query_row("SELECT is_operator FROM account_links WHERE secret_hash=?1 AND kind='invite' AND spent=0 AND revoked=0", params![hash_secret(secret)], |r| r.get(0)).optional().map_err(|e| OpError::internal(e.to_string()))?;
                let role = role.ok_or_else(|| OpError::unauthorized("this Invite has already been spent or revoked"))?;
                conn.execute("UPDATE account_links SET spent=1 WHERE secret_hash=?1", params![hash_secret(secret)]).map_err(|e| OpError::internal(e.to_string()))?;
                conn.execute("INSERT INTO people (id,name,password_hash,home_kitchen_id,is_operator) VALUES (?1,?2,?3,?4,?5)", params![person_id,name,password_hash,kitchen_id,role]).map_err(|e| OpError::bad_request(e.to_string()))?;
                insert_kitchen_with_member(conn, &kitchen_id, &format!("{name}'s Home Kitchen"), &person_id)?;
                conn.execute("INSERT INTO sessions (id,secret_hash,person_id,name) VALUES (?1,?2,?3,?4)", params![session.id,hash_secret(&session.secret),person_id,session_name]).map_err(|e| OpError::internal(e.to_string()))?;
                Ok(role != 0)
            })();
            match result { Ok(v) => { conn.execute_batch("COMMIT").map_err(|e| OpError::internal(e.to_string()))?; Ok(v) }, Err(e) => { let _=conn.execute_batch("ROLLBACK"); Err(e) } }
        })?;
        Ok(
            json!({"person":{"id":person_id,"name":name,"hand_id":person_id,"home_kitchen_id":kitchen_id,"reading_language":"en","reading_measures":"us","is_operator":operator},"session_id":session.id,"_session_secret":session.secret}),
        )
    }

    pub fn mint_recovery_link(&self, name: &str) -> Result<String, OpError> {
        let secret = generate_secret();
        self.db().with_conn(|conn| {
            let person_id: Option<String> = conn.query_row("SELECT id FROM people WHERE name = ?1 AND disabled = 0 AND deleted = 0", params![required_text(name, "name")?], |r| r.get(0)).optional()
                .map_err(|e| OpError::internal(format!("cannot read Person for recovery: {e}")))?;
            let person_id = person_id.ok_or_else(|| OpError::not_found(format!("no active Person '{name}'")))?;
            conn.execute("INSERT INTO account_links (secret_hash, kind, person_id) VALUES (?1, 'recovery', ?2)", params![hash_secret(&secret), person_id])
                .map_err(|e| OpError::internal(format!("cannot mint recovery link: {e}")))?;
            Ok(())
        })?;
        Ok(format!("/recover/{secret}"))
    }

    pub fn redeem_recovery(
        &self,
        link: &str,
        password: &str,
        session_name: &str,
    ) -> Result<Value, OpError> {
        let secret = link
            .strip_prefix("/recover/")
            .filter(|s| !s.is_empty())
            .ok_or_else(|| OpError::bad_request("recovery link is invalid"))?;
        let password_hash = hash_password(password)?;
        let session_name = required_text(session_name, "session_name")?;
        let session = Session {
            id: format!("s_{}", hex::encode(random_bytes(8))),
            secret: generate_secret(),
        };
        self.db().with_conn(|conn| {
            conn.execute_batch("BEGIN IMMEDIATE").map_err(|e| OpError::internal(e.to_string()))?;
            let result = (|| -> Result<(), OpError> {
                let person_id: Option<String> = conn.query_row("SELECT people.id FROM account_links JOIN people ON people.id=account_links.person_id WHERE secret_hash=?1 AND kind='recovery' AND spent=0 AND revoked=0 AND disabled=0 AND deleted=0", params![hash_secret(secret)], |r| r.get(0)).optional().map_err(|e| OpError::internal(e.to_string()))?;
                let person_id=person_id.ok_or_else(|| OpError::unauthorized("this recovery link has already been spent or revoked"))?;
                conn.execute("UPDATE account_links SET spent=1 WHERE secret_hash=?1 AND spent=0",params![hash_secret(secret)]).map_err(|e| OpError::internal(e.to_string()))?;
                conn.execute("UPDATE people SET password_hash=?1 WHERE id=?2",params![password_hash,person_id]).map_err(|e| OpError::internal(e.to_string()))?;
                conn.execute("INSERT INTO sessions (id,secret_hash,person_id,name) VALUES (?1,?2,?3,?4)",params![session.id,hash_secret(&session.secret),person_id,session_name]).map_err(|e| OpError::internal(e.to_string()))?;
                Ok(())
            })();
            match result { Ok(()) => conn.execute_batch("COMMIT").map_err(|e| OpError::internal(e.to_string())), Err(e) => { let _=conn.execute_batch("ROLLBACK"); Err(e) } }
        })?;
        Ok(json!({"session_id":session.id,"_session_secret":session.secret}))
    }

    pub fn end_account(&self, name: &str, deleted: bool) -> Result<(), OpError> {
        let name = required_text(name, "name")?;
        self.db().with_conn(|conn| {
            let changed = conn.execute("UPDATE people SET disabled = 1, deleted = CASE WHEN ?1 THEN 1 ELSE deleted END WHERE name = ?2 AND deleted = 0", params![deleted as i64, name])
                .map_err(|e| OpError::internal(format!("cannot end account: {e}")))?;
            if changed == 0 { return Err(OpError::not_found(format!("no active Person '{name}'"))); }
            conn.execute("UPDATE sessions SET revoked = 1 WHERE person_id = (SELECT id FROM people WHERE name = ?1)", params![name]).map_err(|e| OpError::internal(e.to_string()))?;
            conn.execute("UPDATE access_keys SET revoked = 1 WHERE person_id = (SELECT id FROM people WHERE name = ?1)", params![name]).map_err(|e| OpError::internal(e.to_string()))?;
            if deleted { conn.execute("DELETE FROM kitchen_members WHERE person_id = (SELECT id FROM people WHERE name = ?1)", params![name]).map_err(|e| OpError::internal(e.to_string()))?; }
            Ok(())
        })
    }

    /// Create a Person with their Home Kitchen. Test plumbing uses this until
    /// Invites arrive; real account creation enters through `create_first_person`
    /// or an Invite. A Person on their own is a Kitchen of one (ADR 0007), so
    /// this never leaves a Person without one.
    pub fn create_person(&self, name: &str) -> Result<String, OpError> {
        let id = format!("p_{}", hex::encode(random_bytes(8)));
        let kitchen_id = format!("k_{}", hex::encode(random_bytes(8)));
        self.db().with_conn(|conn| {
            conn.execute(
                "INSERT INTO people (id, name, home_kitchen_id) VALUES (?1, ?2, ?3)",
                params![id, name, kitchen_id],
            )
            .map_err(|e| OpError::internal(format!("cannot create Person: {e}")))?;
            insert_kitchen_with_member(conn, &kitchen_id, &format!("{name}'s Home Kitchen"), &id)
        })?;
        Ok(id)
    }

    /// Whether setup has happened on this instance. Nothing sets it yet — the
    /// first-visitor-wins flow is a later ticket — so a fresh install says false.
    pub fn setup_complete(&self) -> Result<bool, OpError> {
        self.db().with_conn(|conn| {
            let setup: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM instance_setup WHERE singleton = 1)",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot read instance setup: {e}")))?;
            Ok(setup)
        })
    }

    pub fn mark_setup_complete(&self) -> Result<(), OpError> {
        self.db().with_conn(|conn| {
            conn.execute(
                "INSERT OR IGNORE INTO instance_setup(singleton) VALUES (1)",
                [],
            )
            .map_err(|e| OpError::internal(format!("cannot record setup flag: {e}")))?;
            Ok(())
        })
    }

    /// Create a Kitchen: anyone may, and its creator is its first (and, until
    /// someone is invited in, only) member (ADR 0007).
    pub fn create_kitchen(&self, person_id: &str, name: &str) -> Result<Value, OpError> {
        let name = required_text(name, "name")?.to_string();
        let kitchen_id = format!("k_{}", hex::encode(random_bytes(8)));
        self.db().with_conn(|conn| {
            insert_kitchen_with_member(conn, &kitchen_id, &name, person_id)?;
            kitchen_summary(conn, &kitchen_id, person_id)
        })
    }

    /// Every Kitchen a Person cooks in.
    pub fn list_kitchens(&self, person_id: &str) -> Result<Vec<Value>, OpError> {
        self.db().with_conn(|conn| {
            let ids: Vec<String> = {
                let mut statement = conn
                    .prepare("SELECT kitchen_id FROM kitchen_members WHERE person_id = ?1")
                    .map_err(|e| OpError::internal(format!("cannot list Kitchens: {e}")))?;
                statement
                    .query_map(params![person_id], |row| row.get(0))
                    .map_err(|e| OpError::internal(format!("cannot list Kitchens: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot list Kitchens: {e}")))?
            };
            ids.iter()
                .map(|id| kitchen_summary(conn, id, person_id))
                .collect()
        })
    }

    /// Any member may change the Kitchen's shared Name.
    pub fn rename_kitchen(
        &self,
        person_id: &str,
        kitchen_id: &str,
        name: &str,
    ) -> Result<(), OpError> {
        let name = required_text(name, "name")?;
        self.db().with_conn(|conn| {
            ensure_member(conn, kitchen_id, person_id)?;
            conn.execute(
                "UPDATE kitchens SET name = ?1 WHERE id = ?2",
                params![name, kitchen_id],
            )
            .map_err(|e| OpError::internal(format!("cannot rename Kitchen: {e}")))?;
            Ok(())
        })
    }

    /// A member's own private relabelling of a Kitchen — theirs alone, never
    /// seen by anyone else and never travelling with it (ADR 0007). An empty or
    /// absent Nickname clears it back to the shared Name.
    pub fn set_kitchen_nickname(
        &self,
        person_id: &str,
        kitchen_id: &str,
        nickname: Option<&str>,
    ) -> Result<(), OpError> {
        let nickname = nickname.map(str::trim).filter(|n| !n.is_empty());
        self.db().with_conn(|conn| {
            ensure_member(conn, kitchen_id, person_id)?;
            conn.execute(
                "UPDATE kitchen_members SET nickname = ?1 WHERE kitchen_id = ?2 AND person_id = ?3",
                params![nickname, kitchen_id, person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot set Nickname: {e}")))?;
            Ok(())
        })
    }

    /// Mint a one-use link a member can pass to another Person to join this
    /// Kitchen. The raw Secret is returned once and stored only as its hash.
    pub fn invite_to_kitchen(
        &self,
        person_id: &str,
        kitchen_id: &str,
    ) -> Result<(String, String), OpError> {
        let secret = generate_secret();
        let hash = hash_secret(&secret);
        let id = format!("ki_{}", hex::encode(random_bytes(8)));
        self.db().with_conn(|conn| {
            ensure_member(conn, kitchen_id, person_id)?;
            conn.execute(
                "INSERT INTO kitchen_invites (id, secret_hash, kitchen_id, created_by) VALUES (?1, ?2, ?3, ?4)",
                params![id, hash, kitchen_id, person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot mint Kitchen Invite: {e}")))?;
            Ok(())
        })?;
        Ok((id, secret))
    }

    /// Open a Kitchen Invite: the Person presenting it joins the Kitchen, and
    /// the Invite is spent. Already being a member is not an error — the
    /// Invite is still spent, and the Kitchen is simply handed back.
    pub fn accept_kitchen_invite(&self, person_id: &str, secret: &str) -> Result<Value, OpError> {
        let hash = hash_secret(secret);
        self.db().with_conn(|conn| {
            let (invite_id, kitchen_id): (String, String) = conn
                .query_row(
                    "SELECT id, kitchen_id FROM kitchen_invites WHERE secret_hash = ?1 AND used_at IS NULL",
                    params![hash],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot resolve Kitchen Invite: {e}")))?
                .ok_or_else(|| {
                    OpError::unauthorized("this Invite does not name a live Kitchen invitation")
                })?;

            conn.execute(
                "UPDATE kitchen_invites SET used_at = strftime('%Y-%m-%dT%H:%M:%fZ','now'), used_by = ?1 WHERE id = ?2",
                params![person_id, invite_id],
            )
            .map_err(|e| OpError::internal(format!("cannot spend Kitchen Invite: {e}")))?;
            conn.execute(
                "INSERT OR IGNORE INTO kitchen_members (kitchen_id, person_id) VALUES (?1, ?2)",
                params![kitchen_id, person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot join Kitchen: {e}")))?;
            kitchen_summary(conn, &kitchen_id, person_id)
        })
    }

    /// Any member may remove another member, or leave by removing themselves.
    /// Two invariants stand in the way: a Kitchen never drops to zero members,
    /// and a Person never drops to zero Kitchens (ADR 0007).
    pub fn remove_kitchen_member(
        &self,
        caller_person_id: &str,
        kitchen_id: &str,
        target_person_id: &str,
    ) -> Result<(), OpError> {
        self.db().with_conn(|conn| {
            ensure_member(conn, kitchen_id, caller_person_id)?;
            if !is_member(conn, kitchen_id, target_person_id)? {
                return Err(OpError::not_found(
                    "that Person does not cook in this Kitchen",
                ));
            }
            let kitchen_members: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM kitchen_members WHERE kitchen_id = ?1",
                    params![kitchen_id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot count Kitchen members: {e}")))?;
            if kitchen_members <= 1 {
                return Err(OpError::bad_request(
                    "the last member of a Kitchen cannot be removed — invite someone else, or delete it",
                ));
            }
            let person_kitchens: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM kitchen_members WHERE person_id = ?1",
                    params![target_person_id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot count that Person's Kitchens: {e}")))?;
            if person_kitchens <= 1 {
                return Err(OpError::bad_request(
                    "a Person cooks in one or more Kitchens and cannot be removed from their last one",
                ));
            }
            conn.execute(
                "DELETE FROM kitchen_members WHERE kitchen_id = ?1 AND person_id = ?2",
                params![kitchen_id, target_person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot remove Kitchen member: {e}")))?;
            // Leaving your own Home Kitchen hands the default landing spot to
            // the oldest Kitchen you still cook in — the one you have been in
            // longest, rather than an arbitrary remaining one.
            conn.execute(
                "UPDATE people SET home_kitchen_id = (
                    SELECT kitchen_members.kitchen_id FROM kitchen_members
                    JOIN kitchens ON kitchens.id = kitchen_members.kitchen_id
                    WHERE kitchen_members.person_id = ?1
                    ORDER BY kitchens.created_at ASC LIMIT 1
                 ) WHERE id = ?1 AND home_kitchen_id = ?2",
                params![target_person_id, kitchen_id],
            )
            .map_err(|e| OpError::internal(format!("cannot reassign Home Kitchen: {e}")))?;
            Ok(())
        })
    }

    /// The Operator's one power over a Kitchen: deleting one nobody is left in.
    pub fn delete_kitchen(&self, caller_person_id: &str, kitchen_id: &str) -> Result<(), OpError> {
        self.db().with_conn(|conn| {
            let is_operator: bool = conn
                .query_row(
                    "SELECT is_operator FROM people WHERE id = ?1",
                    params![caller_person_id],
                    |row| row.get::<_, i64>(0),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot check Operator status: {e}")))?
                .map(|v| v != 0)
                .unwrap_or(false);
            if !is_operator {
                return Err(OpError::unauthorized(
                    "only an Operator may delete a Kitchen",
                ));
            }
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM kitchens WHERE id = ?1",
                    params![kitchen_id],
                    |row| row.get::<_, i64>(0),
                )
                .map(|n| n > 0)
                .map_err(|e| OpError::internal(format!("cannot look up Kitchen: {e}")))?;
            if !exists {
                return Err(OpError::not_found("no such Kitchen"));
            }
            let members: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM kitchen_members WHERE kitchen_id = ?1",
                    params![kitchen_id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot count Kitchen members: {e}")))?;
            if members > 0 {
                return Err(OpError::bad_request(
                    "a Kitchen may be deleted only once nobody is left in it",
                ));
            }
            conn.execute("DELETE FROM kitchens WHERE id = ?1", params![kitchen_id])
                .map_err(|e| OpError::internal(format!("cannot delete Kitchen: {e}")))?;
            Ok(())
        })
    }

    /// Create a Tag in a Kitchen, named in one Language. Any member may.
    ///
    /// A word already used in that Kitchen and Language does not make a second
    /// Tag: the one already there is returned, which is what keeps *dessert*
    /// and *dessert* one Tag (#51). Two people reaching for the same word have
    /// agreed, not collided.
    pub fn create_tag(
        &self,
        person_id: &str,
        kitchen_id: &str,
        language: &str,
        name: &str,
    ) -> Result<Value, OpError> {
        let language = supported_language(language)?;
        let name = required_text(name, "name")?.to_string();
        let tag_id = format!("t_{}", hex::encode(random_bytes(8)));
        self.db().with_conn(|conn| {
            ensure_member(conn, kitchen_id, person_id)?;
            if let Some(existing) = tag_id_for_word(conn, kitchen_id, language, &name)? {
                return tag_summary(conn, &existing, person_id);
            }
            conn.execute(
                "INSERT INTO tags (id, kitchen_id) VALUES (?1, ?2)",
                params![tag_id, kitchen_id],
            )
            .map_err(|e| OpError::internal(format!("cannot create Tag: {e}")))?;
            conn.execute(
                "INSERT INTO tag_names (tag_id, kitchen_id, language, name, name_folded) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![tag_id, kitchen_id, language, name, folded_word(&name)],
            )
            .map_err(|e| OpError::internal(format!("cannot name Tag: {e}")))?;
            tag_summary(conn, &tag_id, person_id)
        })
    }

    /// Every Tag a Kitchen files by, each shown in the reader's Reading
    /// Language and falling back to whatever name it does have (#51).
    pub fn list_tags(&self, person_id: &str, kitchen_id: &str) -> Result<Vec<Value>, OpError> {
        self.db().with_conn(|conn| {
            ensure_member(conn, kitchen_id, person_id)?;
            let ids: Vec<String> = {
                let mut statement = conn
                    .prepare("SELECT id FROM tags WHERE kitchen_id = ?1 ORDER BY created_at, id")
                    .map_err(|e| OpError::internal(format!("cannot list Tags: {e}")))?;
                statement
                    .query_map(params![kitchen_id], |row| row.get(0))
                    .map_err(|e| OpError::internal(format!("cannot list Tags: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot list Tags: {e}")))?
            };
            ids.iter()
                .map(|id| tag_summary(conn, id, person_id))
                .collect()
        })
    }

    /// Give a Tag its name in one Language — the first name it has there, or a
    /// different one in place of the name it had.
    ///
    /// Because the Tag is kept once and pointed at, this reaches every recipe
    /// carrying it at once, with nothing to walk and nothing to re-save: no
    /// Version is minted and no fingerprint moves (ADR 0035).
    pub fn rename_tag(
        &self,
        person_id: &str,
        tag_id: &str,
        language: &str,
        name: &str,
    ) -> Result<Value, OpError> {
        let language = supported_language(language)?;
        let name = required_text(name, "name")?.to_string();
        self.db().with_conn(|conn| {
            let kitchen_id = kitchen_of_tag(conn, tag_id)?;
            ensure_member(conn, &kitchen_id, person_id)?;
            // The word may already be this Tag's own — renaming *dessert* to
            // *Dessert* is a change of spelling, not a collision with itself.
            match tag_id_for_word(conn, &kitchen_id, language, &name)? {
                Some(owner) if owner != tag_id => {
                    return Err(OpError::bad_request(
                        "this Kitchen already files under that word in that Language",
                    ));
                }
                _ => {}
            }
            conn.execute(
                "INSERT INTO tag_names (tag_id, kitchen_id, language, name, name_folded) \
                 VALUES (?1, ?2, ?3, ?4, ?5) \
                 ON CONFLICT(tag_id, language) \
                 DO UPDATE SET name = excluded.name, name_folded = excluded.name_folded",
                params![tag_id, kitchen_id, language, name, folded_word(&name)],
            )
            .map_err(|e| OpError::internal(format!("cannot rename Tag: {e}")))?;
            tag_summary(conn, tag_id, person_id)
        })
    }

    /// Merge two of a Kitchen's Tags into one: every recipe filed under the
    /// merged Tag is filed under the kept one instead, and the merged Tag is
    /// gone. The other half of what CONTEXT.md says a Tag is — "renaming or
    /// merging one reaches all of them at once".
    ///
    /// The kept Tag keeps its own names. Where it has no name in a Language and
    /// the merged Tag did, that name is adopted rather than thrown away: the
    /// merge is how two words are found to have meant one thing, so the word
    /// the other Language had is worth keeping.
    ///
    /// Like every other act of filing, this mints no Version (ADR 0035).
    pub fn merge_tags(
        &self,
        person_id: &str,
        keep_tag_id: &str,
        merge_tag_id: &str,
    ) -> Result<Value, OpError> {
        if keep_tag_id == merge_tag_id {
            return Err(OpError::bad_request("a Tag cannot be merged into itself"));
        }
        self.db().with_conn(|conn| {
            let keep_kitchen = kitchen_of_tag(conn, keep_tag_id)?;
            let merge_kitchen = kitchen_of_tag(conn, merge_tag_id)?;
            ensure_member(conn, &keep_kitchen, person_id)?;
            // Two Kitchens' filing systems are separate things, and neither is
            // the other's to fold into (ADR 0007).
            if keep_kitchen != merge_kitchen {
                return Err(OpError::bad_request(
                    "two Tags of different Kitchens cannot be merged",
                ));
            }

            let carried: Vec<(String, String, String)> = {
                let mut statement = conn
                    .prepare("SELECT language, name, name_folded FROM tag_names WHERE tag_id = ?1")
                    .map_err(|e| OpError::internal(format!("cannot read Tag names: {e}")))?;
                statement
                    .query_map(params![merge_tag_id], |row| {
                        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                    })
                    .map_err(|e| OpError::internal(format!("cannot read Tag names: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot read Tag names: {e}")))?
            };

            // Every recipe under the merged Tag is now under the kept one.
            // OR IGNORE for the recipes already carrying both.
            conn.execute(
                "INSERT OR IGNORE INTO branch_tags (branch_id, tag_id) \
                 SELECT branch_id, ?1 FROM branch_tags WHERE tag_id = ?2",
                params![keep_tag_id, merge_tag_id],
            )
            .map_err(|e| OpError::internal(format!("cannot carry recipes across: {e}")))?;

            // The merged Tag's own rows go before the kept Tag adopts any of
            // its words: one word is unique per Kitchen and Language, so the
            // two Tags may not hold the same word even for an instant.
            for statement in [
                "DELETE FROM branch_tags WHERE tag_id = ?1",
                "DELETE FROM tag_names WHERE tag_id = ?1",
                "DELETE FROM tags WHERE id = ?1",
            ] {
                conn.execute(statement, params![merge_tag_id])
                    .map_err(|e| OpError::internal(format!("cannot merge Tag: {e}")))?;
            }

            for (language, name, folded) in carried {
                conn.execute(
                    "INSERT OR IGNORE INTO tag_names \
                     (tag_id, kitchen_id, language, name, name_folded) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![keep_tag_id, keep_kitchen, language, name, folded],
                )
                .map_err(|e| OpError::internal(format!("cannot adopt Tag name: {e}")))?;
            }

            tag_summary(conn, keep_tag_id, person_id)
        })
    }

    /// Take a Tag out of a Kitchen's list, and off every recipe carrying it.
    /// No recipe changes: a Version records what was written, never how it was
    /// filed (ADR 0035).
    pub fn delete_tag(&self, person_id: &str, tag_id: &str) -> Result<(), OpError> {
        self.db().with_conn(|conn| {
            let kitchen_id = kitchen_of_tag(conn, tag_id)?;
            ensure_member(conn, &kitchen_id, person_id)?;
            for statement in [
                "DELETE FROM branch_tags WHERE tag_id = ?1",
                "DELETE FROM tag_names WHERE tag_id = ?1",
                "DELETE FROM tags WHERE id = ?1",
            ] {
                conn.execute(statement, params![tag_id])
                    .map_err(|e| OpError::internal(format!("cannot delete Tag: {e}")))?;
            }
            Ok(())
        })
    }

    /// File a recipe under a Tag, or take it back out — `carried` says which.
    /// Both are ordinary filing: neither mints a Version, neither appears in
    /// the Thread, and the recipe's fingerprint is untouched (ADR 0035).
    pub fn set_recipe_tag(
        &self,
        person_id: &str,
        branch_id: &str,
        tag_id: &str,
        carried: bool,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let branch_kitchen: String = conn
                .query_row(
                    "SELECT kitchen_id FROM branches WHERE id = ?1",
                    params![branch_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?
                .ok_or_else(|| OpError::not_found("no such Branch"))?;
            ensure_member(conn, &branch_kitchen, person_id)?;

            // A Tag belongs to one Kitchen, so a recipe can only be filed
            // under its own Kitchen's words (ADR 0007). Reaching across is a
            // request for a Tag this Kitchen does not have.
            let tag_kitchen = kitchen_of_tag(conn, tag_id)?;
            if tag_kitchen != branch_kitchen {
                return Err(OpError::not_found(
                    "no such Tag in the Kitchen holding this recipe",
                ));
            }

            if carried {
                conn.execute(
                    "INSERT OR IGNORE INTO branch_tags (branch_id, tag_id) VALUES (?1, ?2)",
                    params![branch_id, tag_id],
                )
                .map_err(|e| OpError::internal(format!("cannot file recipe under Tag: {e}")))?;
            } else {
                conn.execute(
                    "DELETE FROM branch_tags WHERE branch_id = ?1 AND tag_id = ?2",
                    params![branch_id, tag_id],
                )
                .map_err(|e| OpError::internal(format!("cannot take recipe off Tag: {e}")))?;
            }
            Ok(json!({ "tags": tags_of_branch(conn, branch_id, person_id)? }))
        })
    }

    /// Relate two Recipes held on this Kitchen's shelf, or take that one
    /// shelf-local relation back off. The stored pair is canonically ordered,
    /// so asking from either end changes the same untyped, two-way link (#52).
    pub fn set_related_recipe(
        &self,
        person_id: &str,
        branch_id: &str,
        related_branch_id: &str,
        related: bool,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let (kitchen_id, lineage_id, title) = recipe_shelf_identity(conn, branch_id)?
                .ok_or_else(|| OpError::not_found("no such Recipe on this shelf"))?;
            ensure_member(conn, &kitchen_id, person_id)?;

            let (related_kitchen_id, related_lineage_id, related_title) =
                recipe_shelf_identity(conn, related_branch_id)?
                    .ok_or_else(|| OpError::not_found("no such related Recipe on this shelf"))?;
            if related_kitchen_id != kitchen_id {
                return Err(OpError::not_found(
                    "no such related Recipe on the Kitchen's shelf",
                ));
            }
            if lineage_id == related_lineage_id {
                return Err(OpError::bad_request("a Recipe cannot be Related to itself"));
            }

            let (lineage_a_id, lineage_a_name, lineage_b_id, lineage_b_name) =
                if lineage_id < related_lineage_id {
                    (lineage_id.as_str(), title.as_str(), related_lineage_id.as_str(), related_title.as_str())
                } else {
                    (related_lineage_id.as_str(), related_title.as_str(), lineage_id.as_str(), title.as_str())
                };
            if related {
                conn.execute(
                    "INSERT OR IGNORE INTO related_recipes \
                     (kitchen_id, lineage_a_id, lineage_b_id, lineage_a_name, lineage_b_name) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![kitchen_id, lineage_a_id, lineage_b_id, lineage_a_name, lineage_b_name],
                )
                .map_err(|e| OpError::internal(format!("cannot relate Recipes: {e}")))?;
            } else {
                conn.execute(
                    "DELETE FROM related_recipes WHERE kitchen_id = ?1 AND lineage_a_id = ?2 AND lineage_b_id = ?3",
                    params![kitchen_id, lineage_a_id, lineage_b_id],
                )
                .map_err(|e| OpError::internal(format!("cannot remove Related Recipe: {e}")))?;
            }
            Ok(json!({
                "related_recipes": related_recipes_of_lineage(conn, &kitchen_id, &lineage_id)?
            }))
        })
    }

    /// Create a Recipe: a Lineage, a Branch of it in the creating Kitchen, and
    /// a first Version fingerprinted from its content (ADR 0004). A recipe
    /// needs only a title — every other field of `input` (Yield, Prep/Cook
    /// Time, Note, Source, Ingredients, Steps) is optional (#43).
    pub fn create_recipe(
        &self,
        caller: &Caller,
        kitchen_id: &str,
        input: &Value,
        language: Option<&str>,
    ) -> Result<Value, OpError> {
        let content = parse_recipe_content(input)?;
        let version_id = fingerprint_content(&content);
        let content_text = canonical_json(&content);
        let lineage_id = format!("l_{}", hex::encode(random_bytes(8)));
        let branch_id = format!("b_{}", hex::encode(random_bytes(8)));

        self.db().with_conn(|conn| {
            ensure_member(conn, kitchen_id, &caller.person_id)?;
            let kitchen_hand_id: String = conn
                .query_row(
                    "SELECT hand_id FROM kitchens WHERE id = ?1",
                    params![kitchen_id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot read Kitchen's Hand: {e}")))?;
            let language = match language {
                Some(language) => required_text(language, "language")?.to_string(),
                None => conn
                    .query_row(
                        "SELECT reading_language FROM people WHERE id = ?1",
                        params![caller.person_id],
                        |row| row.get(0),
                    )
                    .map_err(|e| OpError::internal(format!("cannot read reading Language: {e}")))?,
            };

            insert_new_lineage_and_branch(
                conn,
                &lineage_id,
                &branch_id,
                kitchen_id,
                &kitchen_hand_id,
                &language,
                &version_id,
                &content_text,
                &caller.person_id,
                caller.access_key_id.as_deref(),
            )
        })?;

        self.get_recipe(&caller.person_id, &branch_id)
    }

    /// Import: land a batch of candidates already read from an outside
    /// source into the caller's Home Kitchen, matched through that
    /// Kitchen's ledger for this source kind rather than doubled on every
    /// re-run (ADR 0025). Reading the source itself — a `.crumb`, a web
    /// page, a Bundle — is each importer's own job; this is the shared
    /// machinery every importer lands its candidates through.
    ///
    /// Every candidate gets a fate: newly made or found unchanged (both
    /// `arrived`), found changed and `offered` for review rather than
    /// written over, or `unreadable` and named with why. One candidate's
    /// failure never stops the rest — the Report is a ledger, not an error
    /// log that drops what it could not place.
    pub fn import(
        &self,
        caller: &Caller,
        source_kind: &str,
        candidates: &[Value],
        progress: Option<&JobProgress>,
    ) -> Result<Value, OpError> {
        let source_kind = required_text(source_kind, "source_kind")?.to_string();
        let (kitchen_id, kitchen_hand_id) = self.db().with_conn(|conn| {
            let kitchen_id: String = conn
                .query_row(
                    "SELECT home_kitchen_id FROM people WHERE id = ?1",
                    params![caller.person_id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot read Home Kitchen: {e}")))?;
            let hand_id: String = conn
                .query_row(
                    "SELECT hand_id FROM kitchens WHERE id = ?1",
                    params![kitchen_id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot read Kitchen's Hand: {e}")))?;
            Ok((kitchen_id, hand_id))
        })?;

        let import_id = self
            .db()
            .with_conn(|conn| find_or_create_import(conn, &kitchen_id, &source_kind))?;

        let total = candidates.len() as u64;
        let mut arrived = Vec::new();
        let mut offered = Vec::new();
        let mut unreadable = Vec::new();

        for (done, candidate) in candidates.iter().enumerate() {
            if let Some(progress) = progress {
                progress.report(done as u64, Some(total), format!("{done} of {total}"));
            }

            let foreign_id = match candidate.get("foreign_id").and_then(Value::as_str) {
                Some(id) if !id.trim().is_empty() => id.trim().to_string(),
                _ => {
                    unreadable.push(json!({
                        "foreign_id": candidate.get("foreign_id").cloned().unwrap_or(Value::Null),
                        "reason": "a foreign id is required to keep the ledger",
                    }));
                    continue;
                }
            };

            match self.import_one(
                caller,
                &kitchen_id,
                &kitchen_hand_id,
                &import_id,
                &foreign_id,
                candidate,
            ) {
                Ok(ImportOutcome::Landed {
                    lineage_id,
                    branch_id,
                    title,
                    status,
                }) => arrived.push(json!({
                    "foreign_id": foreign_id, "status": status,
                    "lineage_id": lineage_id, "branch_id": branch_id, "title": title,
                })),
                Ok(ImportOutcome::Offered {
                    lineage_id,
                    branch_id,
                    title,
                    candidate_version_id,
                }) => offered.push(json!({
                    "foreign_id": foreign_id,
                    "lineage_id": lineage_id, "branch_id": branch_id, "title": title,
                    "candidate_version_id": candidate_version_id,
                })),
                Err(err) => unreadable.push(json!({
                    "foreign_id": foreign_id,
                    "reason": err.to_sentence(),
                })),
            }
        }

        if let Some(progress) = progress {
            progress.report(total, Some(total), "finished".to_string());
        }

        Ok(json!({
            "import_id": import_id,
            "kitchen_id": kitchen_id,
            "source_kind": source_kind,
            "arrived": arrived,
            "offered": offered,
            "unreadable": unreadable,
        }))
    }

    /// One candidate against the ledger: unseen becomes a new Lineage,
    /// Branch and first Version, the importing Person's Hand on it
    /// (CONTEXT.md, "Hand"; ADR 0025). Seen before and now identical is
    /// `Unchanged`; seen before and now different records the candidate's
    /// content as a Version — content-addressed, so this never collides with
    /// or moves anything already on the Branch — and answers `Offered`
    /// without touching `head_version_id`: the offer is never written over
    /// the Branch on its own.
    fn import_one(
        &self,
        caller: &Caller,
        kitchen_id: &str,
        kitchen_hand_id: &str,
        import_id: &str,
        foreign_id: &str,
        candidate: &Value,
    ) -> Result<ImportOutcome, OpError> {
        let content = parse_recipe_content(candidate)?;
        let title = content["title"].as_str().unwrap_or_default().to_string();
        let version_id = fingerprint_content(&content);
        let content_text = canonical_json(&content);

        self.db().with_conn(|conn| {
            let existing: Option<(String, String, String)> = conn
                .query_row(
                    "SELECT import_ledger.lineage_id, import_ledger.branch_id, branches.head_version_id \
                       FROM import_ledger JOIN branches ON branches.id = import_ledger.branch_id \
                      WHERE import_ledger.import_id = ?1 AND import_ledger.foreign_id = ?2",
                    params![import_id, foreign_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read Import ledger: {e}")))?;

            match existing {
                None => {
                    let lineage_id = format!("l_{}", hex::encode(random_bytes(8)));
                    let branch_id = format!("b_{}", hex::encode(random_bytes(8)));
                    let language = match candidate.get("language").and_then(Value::as_str) {
                        Some(language) => required_text(language, "language")?.to_string(),
                        None => conn
                            .query_row(
                                "SELECT reading_language FROM people WHERE id = ?1",
                                params![caller.person_id],
                                |row| row.get(0),
                            )
                            .map_err(|e| {
                                OpError::internal(format!("cannot read reading Language: {e}"))
                            })?,
                    };

                    insert_new_lineage_and_branch(
                        conn,
                        &lineage_id,
                        &branch_id,
                        kitchen_id,
                        kitchen_hand_id,
                        &language,
                        &version_id,
                        &content_text,
                        &caller.person_id,
                        caller.access_key_id.as_deref(),
                    )?;
                    conn.execute(
                        "INSERT INTO import_ledger (import_id, foreign_id, lineage_id, branch_id) \
                         VALUES (?1, ?2, ?3, ?4)",
                        params![import_id, foreign_id, lineage_id, branch_id],
                    )
                    .map_err(|e| {
                        OpError::internal(format!("cannot record Import ledger entry: {e}"))
                    })?;
                    Ok(ImportOutcome::Landed {
                        lineage_id,
                        branch_id,
                        title,
                        status: "created",
                    })
                }
                Some((lineage_id, branch_id, head_version_id)) => {
                    if head_version_id == version_id {
                        Ok(ImportOutcome::Landed {
                            lineage_id,
                            branch_id,
                            title,
                            status: "unchanged",
                        })
                    } else {
                        conn.execute(
                            "INSERT OR IGNORE INTO versions (id, content) VALUES (?1, ?2)",
                            params![version_id, content_text],
                        )
                        .map_err(|e| {
                            OpError::internal(format!("cannot record candidate Version: {e}"))
                        })?;
                        Ok(ImportOutcome::Offered {
                            lineage_id,
                            branch_id,
                            title,
                            candidate_version_id: version_id,
                        })
                    }
                }
            }
        })
    }

    /// Save a new state of a Recipe onto a Branch: an ordinary edit becomes an
    /// append-only Version. A save by the same Hand within the collapse
    /// window collapses into the Version already being shaped, rather than
    /// starting a new one (ADR 0004). Saving content identical to what is
    /// already there mints nothing.
    ///
    /// Changing a recipe your Kitchen did not write is a **Copy**
    /// (CONTEXT.md, "Copy"): it happens here, at the moment of the change,
    /// never at the moment of merely reading `branch_id`. `kitchen_id` names
    /// which of the caller's own Kitchens this save is on behalf of — their
    /// Home Kitchen unless they say otherwise — and a Copy is made the moment
    /// that Kitchen turns out not to be the one that already holds this
    /// Branch: a brand new Branch of the same Lineage, held by that Kitchen,
    /// carrying the whole chain behind it, starting at the Version being
    /// changed. The Branch being edited is never touched by a Copy.
    #[allow(clippy::too_many_arguments)]
    pub fn save_recipe_version(
        &self,
        caller: &Caller,
        branch_id: &str,
        input: &Value,
        name: Option<&str>,
        change_note: Option<&str>,
        kitchen_id: Option<&str>,
    ) -> Result<Value, OpError> {
        let content = parse_recipe_content(input)?;
        let version_id = fingerprint_content(&content);
        let content_text = canonical_json(&content);

        self.db().with_conn(|conn| {
            let (owning_kitchen_id, lineage_id, language): (String, String, String) = conn
                .query_row(
                    "SELECT kitchen_id, lineage_id, language FROM branches WHERE id = ?1",
                    params![branch_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?
                .ok_or_else(|| OpError::not_found("no such Branch"))?;

            // Which of the caller's own Kitchens this save is on behalf of.
            // Named explicitly, or — when the caller already cooks in the
            // Kitchen that holds this Branch — that same Kitchen, so an
            // ordinary edit by a co-editor never needs to say so. Only a
            // caller whose Kitchens hold none of them falls back to their
            // Home Kitchen (CONTEXT.md, "Home Kitchen").
            let target_kitchen_id = match kitchen_id {
                Some(id) => id.to_string(),
                None if is_member(conn, &owning_kitchen_id, &caller.person_id)? => {
                    owning_kitchen_id.clone()
                }
                None => conn
                    .query_row(
                        "SELECT home_kitchen_id FROM people WHERE id = ?1",
                        params![caller.person_id],
                        |row| row.get::<_, Option<String>>(0),
                    )
                    .optional()
                    .map_err(|e| OpError::internal(format!("cannot read Home Kitchen: {e}")))?
                    .flatten()
                    .ok_or_else(|| OpError::internal("this Person has no Home Kitchen"))?,
            };
            ensure_member(conn, &target_kitchen_id, &caller.person_id)?;

            let (head_sequence, head_version_id, head_hand_id, head_parent_id, within_window, head_content): (
                i64,
                String,
                String,
                Option<String>,
                bool,
                String,
            ) = conn
                .query_row(
                    "SELECT branch_versions.sequence, branch_versions.version_id, \
                            branch_versions.hand_id, branch_versions.parent_version_id, \
                            (julianday('now') - julianday(branch_versions.created_at)) * 86400.0 <= ?2, \
                            versions.content \
                       FROM branch_versions JOIN versions ON versions.id = branch_versions.version_id \
                      WHERE branch_versions.branch_id = ?1 ORDER BY branch_versions.sequence DESC LIMIT 1",
                    params![branch_id, COLLAPSE_WINDOW_SECONDS as f64],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                        ))
                    },
                )
                .map_err(|e| OpError::internal(format!("cannot read Branch head: {e}")))?;

            if version_id == head_version_id {
                // Identical content: the fingerprint already names this state,
                // so there is nothing new to save — and merely reading a
                // recipe you cannot change must never start a Copy.
                return Ok(json!({
                    "branch_id": branch_id,
                    "version_id": version_id,
                    "parent_version_id": head_parent_id,
                    "sequence": head_sequence,
                    "collapsed": false,
                    "copied": false,
                }));
            }

            conn.execute(
                "INSERT OR IGNORE INTO versions (id, content) VALUES (?1, ?2)",
                params![version_id, content_text],
            )
            .map_err(|e| OpError::internal(format!("cannot record Version: {e}")))?;

            // A Reading travels with the Version it belongs to and is never
            // recomputed (ADR 0021) — so wherever this save left a line
            // reading exactly as it did before, its Reading carries forward
            // onto the new Version rather than being silently lost. A line
            // that actually changed loses its Reading, which is ADR 0002's
            // "re-reading the edited line refreshes the Reading" (the
            // refresh itself is deferred: nothing here re-parses it). This
            // holds identically for a Copy's first save: it is starting
            // exactly at this head.
            let head_content: Value = serde_json::from_str(&head_content)
                .map_err(|e| OpError::internal(format!("cannot read Version content: {e}")))?;
            carry_forward_readings(conn, &head_version_id, &head_content, &version_id, &content)?;

            if target_kitchen_id != owning_kitchen_id {
                // Copy: your Kitchen did not write this Branch, so the change
                // starts a new one of its own — the source Branch is left
                // exactly as it was.
                let new_branch_id = format!("b_{}", hex::encode(random_bytes(8)));
                let kitchen_hand_id: String = conn
                    .query_row(
                        "SELECT hand_id FROM kitchens WHERE id = ?1",
                        params![target_kitchen_id],
                        |row| row.get(0),
                    )
                    .map_err(|e| OpError::internal(format!("cannot read Kitchen's Hand: {e}")))?;

                conn.execute(
                    "INSERT INTO branches (id, lineage_id, kitchen_id, hand_id, language, head_version_id) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        new_branch_id,
                        lineage_id,
                        target_kitchen_id,
                        kitchen_hand_id,
                        language,
                        version_id
                    ],
                )
                .map_err(|e| OpError::internal(format!("cannot start Branch: {e}")))?;

                // The whole chain behind the Version being changed, carried
                // across verbatim — same Versions, same Hands, same names and
                // change notes, nothing truncated.
                conn.execute(
                    "INSERT INTO branch_versions \
                     (branch_id, sequence, version_id, parent_version_id, hand_id, name, change_note, access_key_id, created_at) \
                     SELECT ?1, sequence, version_id, parent_version_id, hand_id, name, change_note, access_key_id, created_at \
                       FROM branch_versions WHERE branch_id = ?2",
                    params![new_branch_id, branch_id],
                )
                .map_err(|e| OpError::internal(format!("cannot carry the chain onto the new Branch: {e}")))?;

                conn.execute(
                    "INSERT INTO branch_versions \
                     (branch_id, sequence, version_id, parent_version_id, hand_id, name, change_note, access_key_id) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        new_branch_id,
                        head_sequence + 1,
                        version_id,
                        head_version_id,
                        caller.person_id,
                        name,
                        change_note,
                        caller.access_key_id
                    ],
                )
                .map_err(|e| OpError::internal(format!("cannot append Version: {e}")))?;

                return Ok(json!({
                    "branch_id": new_branch_id,
                    "version_id": version_id,
                    "parent_version_id": head_version_id,
                    "sequence": head_sequence + 1,
                    "collapsed": false,
                    "copied": true,
                }));
            }

            let collapse = within_window && head_hand_id == caller.person_id;
            if collapse {
                conn.execute(
                    "UPDATE branch_versions SET version_id = ?1, hand_id = ?2, name = ?3, \
                            change_note = ?4, access_key_id = ?5, \
                            created_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
                     WHERE branch_id = ?6 AND sequence = ?7",
                    params![
                        version_id,
                        caller.person_id,
                        name,
                        change_note,
                        caller.access_key_id,
                        branch_id,
                        head_sequence
                    ],
                )
                .map_err(|e| OpError::internal(format!("cannot collapse Version: {e}")))?;
            } else {
                conn.execute(
                    "INSERT INTO branch_versions \
                     (branch_id, sequence, version_id, parent_version_id, hand_id, name, change_note, access_key_id) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        branch_id,
                        head_sequence + 1,
                        version_id,
                        head_version_id,
                        caller.person_id,
                        name,
                        change_note,
                        caller.access_key_id
                    ],
                )
                .map_err(|e| OpError::internal(format!("cannot append Version: {e}")))?;
            }
            conn.execute(
                "UPDATE branches SET head_version_id = ?1 WHERE id = ?2",
                params![version_id, branch_id],
            )
            .map_err(|e| OpError::internal(format!("cannot move Branch head: {e}")))?;

            Ok(json!({
                "branch_id": branch_id,
                "version_id": version_id,
                "parent_version_id": if collapse { head_parent_id } else { Some(head_version_id) },
                "sequence": if collapse { head_sequence } else { head_sequence + 1 },
                "collapsed": collapse,
                "copied": false,
            }))
        })
    }

    /// Rename a Version — the one thing about it that can change later
    /// (CONTEXT.md, "Version"). An absent or empty name clears it. Targeted
    /// by `sequence` rather than `version_id`: the same content can recur
    /// more than once on one Branch (a save reverting to exact earlier
    /// text), and each occurrence carries its own independent name — the
    /// content hash alone cannot tell them apart, but `sequence` always can.
    /// Returns the name as it was actually stored, trimmed and cleared.
    pub fn rename_version(
        &self,
        person_id: &str,
        branch_id: &str,
        sequence: i64,
        name: Option<&str>,
    ) -> Result<Option<String>, OpError> {
        let name = name.map(str::trim).filter(|n| !n.is_empty());
        self.db().with_conn(|conn| {
            let kitchen_id: String = conn
                .query_row(
                    "SELECT kitchen_id FROM branches WHERE id = ?1",
                    params![branch_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?
                .ok_or_else(|| OpError::not_found("no such Branch"))?;
            ensure_member(conn, &kitchen_id, person_id)?;
            let changed = conn
                .execute(
                    "UPDATE branch_versions SET name = ?1 WHERE branch_id = ?2 AND sequence = ?3",
                    params![name, branch_id, sequence],
                )
                .map_err(|e| OpError::internal(format!("cannot rename Version: {e}")))?;
            if changed == 0 {
                return Err(OpError::not_found(
                    "that sequence does not occur on this Branch",
                ));
            }
            Ok(name.map(str::to_string))
        })
    }

    /// The one data directory everything durable lives under (ADR 0028) — the
    /// database beside it, Photographs and Display Copies below it.
    pub fn data_dir(&self) -> std::path::PathBuf {
        self.db().data_dir().to_path_buf()
    }

    /// Resolve a Credential for a route that carries the same ability as a
    /// `Permission::Person` Operation, without requiring it also write
    /// (ADR 0031's read-only Access Keys may still use it). Exists for the
    /// out-of-band Photograph routes (ADR 0001), which never pass through
    /// [`Core::execute`] and so must repeat its permission check themselves
    /// rather than skip it — a Door deciding this on its own would be the
    /// same bug `execute` exists to prevent.
    pub fn authenticate(&self, secret: Option<&str>) -> Result<Caller, OpError> {
        let secret = secret.filter(|s| !s.is_empty()).ok_or_else(|| {
            OpError::unauthorized("this route requires a Credential naming a Person")
        })?;
        self.resolve_credential(secret)
    }

    /// The same resolution, additionally refusing a read-only Access Key — the
    /// out-of-band equivalent of a Catalogue Operation declared `write: true`.
    pub fn authenticate_for_write(&self, secret: Option<&str>) -> Result<Caller, OpError> {
        let caller = self.authenticate(secret)?;
        if caller.read_only {
            return Err(OpError::unauthorized(
                "this route writes, and this Credential is a read-only Access Key",
            ));
        }
        Ok(caller)
    }

    /// Remake an uploaded picture and give it its identity (ADR 0017). Two
    /// uploads of the same picture — including this one, again — answer the
    /// same id: the write is `INSERT OR IGNORE`, never a duplicate.
    pub fn store_photograph(&self, bytes: &[u8]) -> Result<Value, OpError> {
        let remade = photographs::remake(bytes)?;
        let hash = photographs::hash_bytes(&remade);
        self.record_photograph(&hash, &remade)?;
        Ok(json!({ "photograph_id": hash }))
    }

    /// Store a Photograph exactly as it arrived, with no remaking (ADR 0017):
    /// one already made and travelling in a Bundle is stored byte-for-byte, so
    /// that two instances receiving the same Bundle cannot re-encode their way
    /// into disagreeing about what its bytes are. No Bundle-receiving
    /// Operation exists yet to call this — it is the primitive that one will.
    pub fn store_photograph_verbatim(&self, bytes: &[u8]) -> Result<Value, OpError> {
        let hash = photographs::hash_bytes(bytes);
        self.record_photograph(&hash, bytes)?;
        Ok(json!({ "photograph_id": hash }))
    }

    /// Write a Photograph's bytes under its hash if not already present, and
    /// record the hash — the part `store_photograph` and
    /// `store_photograph_verbatim` share once each has decided what the bytes
    /// to store actually are.
    fn record_photograph(&self, hash: &str, bytes: &[u8]) -> Result<(), OpError> {
        let path = photographs::photograph_path(&self.data_dir(), hash);
        if !path.exists() {
            std::fs::create_dir_all(photographs::photographs_dir(&self.data_dir())).map_err(
                |e| OpError::internal(format!("cannot create Photographs directory: {e}")),
            )?;
            std::fs::write(&path, bytes)
                .map_err(|e| OpError::internal(format!("cannot store Photograph: {e}")))?;
        }
        self.db().with_conn(|conn| {
            conn.execute(
                "INSERT OR IGNORE INTO photographs (hash) VALUES (?1)",
                params![hash],
            )
            .map_err(|e| OpError::internal(format!("cannot record Photograph: {e}")))?;
            Ok(())
        })
    }

    /// Read a Photograph's own bytes back — the out-of-band Web Door route
    /// asks for these directly; no Operation wraps binary output in JSON.
    pub fn read_photograph(&self, hash: &str) -> Result<Vec<u8>, OpError> {
        std::fs::read(photographs::photograph_path(&self.data_dir(), hash))
            .map_err(|_| OpError::not_found("no such Photograph"))
    }

    /// Read a Display Copy, generating and caching it on first ask. Display
    /// Copies are worked out from the Photograph and kept only for
    /// convenience (ADR 0017), so a missing one is made rather than an error.
    pub fn read_display_copy(
        &self,
        hash: &str,
        size: photographs::DisplaySize,
    ) -> Result<Vec<u8>, OpError> {
        let path = photographs::display_path(&self.data_dir(), hash, size);
        if let Ok(cached) = std::fs::read(&path) {
            return Ok(cached);
        }
        let source = self.read_photograph(hash)?;
        let copy = photographs::display_copy(&source, size.long_edge())?;
        std::fs::create_dir_all(photographs::display_dir(&self.data_dir())).map_err(|e| {
            OpError::internal(format!("cannot create Display Copies directory: {e}"))
        })?;
        std::fs::write(&path, &copy)
            .map_err(|e| OpError::internal(format!("cannot cache Display Copy: {e}")))?;
        Ok(copy)
    }

    /// Read a Recipe: the Branch as it stands and its whole chain of Versions,
    /// oldest first — the Thread's raw material.
    pub fn get_recipe(&self, person_id: &str, branch_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let (lineage_id, kitchen_id, hand_id, language, origin_address, head_version_id): (
                String,
                String,
                String,
                String,
                Option<String>,
                String,
            ) = conn
                .query_row(
                    "SELECT lineage_id, kitchen_id, hand_id, language, origin_address, head_version_id \
                       FROM branches WHERE id = ?1",
                    params![branch_id],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                        ))
                    },
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?
                .ok_or_else(|| OpError::not_found("no such Branch"))?;
            ensure_member(conn, &kitchen_id, person_id)?;

            let mut statement = conn
                .prepare(
                    "SELECT branch_versions.sequence, branch_versions.version_id, \
                            branch_versions.parent_version_id, branch_versions.hand_id, \
                            branch_versions.name, branch_versions.change_note, \
                            branch_versions.created_at, versions.content \
                       FROM branch_versions JOIN versions ON versions.id = branch_versions.version_id \
                      WHERE branch_versions.branch_id = ?1 ORDER BY branch_versions.sequence ASC",
                )
                .map_err(|e| OpError::internal(format!("cannot read Thread: {e}")))?;
            let mut versions: Vec<Value> = statement
                .query_map(params![branch_id], |row| {
                    let content: String = row.get(7)?;
                    Ok(json!({
                        "sequence": row.get::<_, i64>(0)?,
                        "version_id": row.get::<_, String>(1)?,
                        "parent_version_id": row.get::<_, Option<String>>(2)?,
                        "hand_id": row.get::<_, String>(3)?,
                        "name": row.get::<_, Option<String>>(4)?,
                        "change_note": row.get::<_, Option<String>>(5)?,
                        "created_at": row.get::<_, String>(6)?,
                        "content": serde_json::from_str::<Value>(&content).unwrap_or(Value::Null),
                    }))
                })
                .map_err(|e| OpError::internal(format!("cannot read Thread: {e}")))?
                .collect::<Result<_, _>>()
                .map_err(|e| OpError::internal(format!("cannot read Thread: {e}")))?;

            // A Reading never sits inside `content` (ADR 0021), so it is
            // fetched separately here and laid alongside it: one slot per
            // Ingredient Line, null wherever no Reading has been recorded.
            for version in &mut versions {
                let version_id = version["version_id"]
                    .as_str()
                    .expect("version_id is always a string")
                    .to_string();
                let line_count = version["content"]["ingredients"]
                    .as_array()
                    .map(Vec::len)
                    .unwrap_or(0);
                version["readings"] = json!(readings_for_version(conn, &version_id, line_count)?);
            }

            Ok(json!({
                "branch_id": branch_id,
                "lineage_id": lineage_id,
                "kitchen_id": kitchen_id,
                "hand_id": hand_id,
                "language": language,
                "origin_address": origin_address,
                "head_version_id": head_version_id,
                "versions": versions,
                // Beside the Versions rather than inside any of them: a Tag is
                // how this Kitchen files the recipe, not part of what the
                // recipe is, so it belongs to the Branch as it stands now and
                // to no Version's content (ADR 0035).
                "tags": tags_of_branch(conn, branch_id, person_id)?,
                // Related Recipes are shelf notes between Lineages. They sit
                // beside the Thread just as Tags do, never inside a Version.
                "related_recipes": related_recipes_of_lineage(conn, &kitchen_id, &lineage_id)?,
            }))
        })
    }

    /// Correct a Reading on the Branch's current head Version: Kamosu's
    /// interpretation of one Ingredient Line, addressed by its position in
    /// that line's list. This never mints a Version and appears in no
    /// Thread (ADR 0021) — the row beside the head Version is simply
    /// replaced or removed. `amount`, `unit` and `target` describe the
    /// whole new Reading together, the same whole-state convention
    /// `save_recipe_version` uses for the whole recipe — this is not a
    /// per-field patch, so correcting one field means sending all three
    /// wanted. All three absent or blank clears the Reading entirely,
    /// taking the line back to fully unread.
    ///
    /// A non-blank `target` is matched against the instance's Foods in the
    /// Branch's own Language, creating one where nothing answers to the word
    /// yet (#47, ADR 0022). Which Food it resolved to is internal
    /// bookkeeping alone — the Reading's own shape stays the bare word,
    /// exactly as `set_reading` has always answered.
    pub fn set_reading(
        &self,
        person_id: &str,
        branch_id: &str,
        line_index: i64,
        amount: Option<&str>,
        unit: Option<&str>,
        target: Option<&str>,
    ) -> Result<Value, OpError> {
        if line_index < 0 {
            return Err(OpError::bad_request("line_index must be zero or more"));
        }
        self.db().with_conn(|conn| {
            let (kitchen_id, language, head_version_id, content): (String, String, String, String) = conn
                .query_row(
                    "SELECT branches.kitchen_id, branches.language, branches.head_version_id, versions.content \
                       FROM branches JOIN versions ON versions.id = branches.head_version_id \
                      WHERE branches.id = ?1",
                    params![branch_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?
                .ok_or_else(|| OpError::not_found("no such Branch"))?;
            ensure_member(conn, &kitchen_id, person_id)?;

            let content: Value = serde_json::from_str(&content)
                .map_err(|e| OpError::internal(format!("cannot read Version content: {e}")))?;
            let line = content["ingredients"]
                .get(line_index as usize)
                .ok_or_else(|| OpError::bad_request("no Ingredient Line at that index"))?;
            if line["kind"] != "ingredient" {
                return Err(OpError::bad_request(
                    "a Reading belongs to an Ingredient Line, not a section",
                ));
            }

            let amount = amount.map(str::trim).filter(|v| !v.is_empty());
            let unit = unit.map(str::trim).filter(|v| !v.is_empty());
            let target = target.map(str::trim).filter(|v| !v.is_empty());

            if amount.is_none() && unit.is_none() && target.is_none() {
                conn.execute(
                    "DELETE FROM readings WHERE version_id = ?1 AND line_index = ?2",
                    params![head_version_id, line_index],
                )
                .map_err(|e| OpError::internal(format!("cannot clear Reading: {e}")))?;
                return Ok(json!({ "line_index": line_index, "reading": Value::Null }));
            }

            let food_id = target
                .map(|word| {
                    resolve_food_for_word(
                        conn,
                        &language,
                        word,
                        Some((head_version_id.as_str(), line_index)),
                    )
                })
                .transpose()?;

            conn.execute(
                "INSERT INTO readings (version_id, line_index, amount, unit, target, food_id) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
                 ON CONFLICT (version_id, line_index) DO UPDATE SET \
                    amount = excluded.amount, unit = excluded.unit, target = excluded.target, \
                    food_id = excluded.food_id, \
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')",
                params![head_version_id, line_index, amount, unit, target, food_id],
            )
            .map_err(|e| OpError::internal(format!("cannot save Reading: {e}")))?;

            Ok(json!({
                "line_index": line_index,
                "reading": { "amount": amount, "unit": unit, "target": target },
            }))
        })
    }

    // --- Cooking it: Attempts (#57, ADR 0005, ADR 0010) ---------------------

    /// Start cooking a Recipe — creates the Attempt, or hands back the one
    /// already In Progress for this Lineage: the cooking screen *is* that
    /// Attempt while it lives, so starting twice is the same Attempt seen
    /// twice, never a second one. Pinned by fingerprint to the Branch's head
    /// Version at this moment (ADR 0005) — a later edit to the recipe never
    /// turns this Attempt into a lie. "May I see this recipe" is the whole
    /// permission this needs, the same membership `get_recipe` checks.
    pub fn start_attempt(&self, person_id: &str, branch_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let (lineage_id, kitchen_id, head_version_id): (String, String, String) = conn
                .query_row(
                    "SELECT lineage_id, kitchen_id, head_version_id FROM branches WHERE id = ?1",
                    params![branch_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?
                .ok_or_else(|| OpError::not_found("no such Branch"))?;
            ensure_member(conn, &kitchen_id, person_id)?;

            if let Some(existing) = in_progress_attempt(conn, &lineage_id, person_id)? {
                let id = existing["id"].as_str().expect("id is always a string");
                touch_attempt(conn, id)?;
                return attempt_by_id(conn, id);
            }

            let id = format!("at_{}", hex::encode(random_bytes(8)));
            conn.execute(
                "INSERT INTO attempts (id, lineage_id, person_id, version_id) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![id, lineage_id, person_id, head_version_id],
            )
            .map_err(|e| OpError::internal(format!("cannot start Attempt: {e}")))?;
            attempt_by_id(conn, &id)
        })
    }

    /// Move an In Progress Attempt forward: which Step, which Ingredients
    /// are ticked, and the Yield being cooked to — a fact about this
    /// cooking, held on the Attempt and never written as a deviation
    /// (ADR 0010). Each of the three is sent whole, the same convention
    /// `save_recipe_version` and `set_reading` use — never a per-field
    /// patch — and any absent one is simply left as it stood.
    pub fn advance_attempt(
        &self,
        person_id: &str,
        attempt_id: &str,
        current_step_index: Option<i64>,
        ticked_ingredients: Option<&[i64]>,
        cooking_yield: Option<&Value>,
    ) -> Result<Value, OpError> {
        if current_step_index.is_none() && ticked_ingredients.is_none() && cooking_yield.is_none() {
            return Err(OpError::bad_request(
                "advance_attempt takes at least one of current_step_index, \
                 ticked_ingredients, cooking_yield",
            ));
        }
        if current_step_index.is_some_and(|index| index < 0) {
            return Err(OpError::bad_request(
                "current_step_index must be zero or more",
            ));
        }

        self.db().with_conn(|conn| {
            let state = attempt_state_owned_by(conn, attempt_id, person_id)?;
            if state.finished_at.is_some() {
                return Err(OpError::bad_request("this Attempt has already finished"));
            }
            let version_id = state.version_id;

            if let Some(index) = current_step_index {
                let steps_len = version_field_len(conn, &version_id, "steps")?;
                if index >= steps_len {
                    return Err(OpError::bad_request(
                        "current_step_index is out of range for this recipe",
                    ));
                }
                conn.execute(
                    "UPDATE attempts SET current_step_index = ?2 WHERE id = ?1",
                    params![attempt_id, index],
                )
                .map_err(|e| OpError::internal(format!("cannot advance Attempt: {e}")))?;
            }
            if let Some(indices) = ticked_ingredients {
                let ingredients_len = version_field_len(conn, &version_id, "ingredients")?;
                if indices.iter().any(|&i| i < 0 || i >= ingredients_len) {
                    return Err(OpError::bad_request(
                        "a ticked Ingredient index is out of range for this recipe",
                    ));
                }
                let stored = serde_json::to_string(indices).expect("serialisable indices");
                conn.execute(
                    "UPDATE attempts SET ticked_ingredients = ?2 WHERE id = ?1",
                    params![attempt_id, stored],
                )
                .map_err(|e| OpError::internal(format!("cannot tick Ingredients: {e}")))?;
            }
            if let Some(value) = cooking_yield {
                let stored = match value {
                    Value::Null => None,
                    other => Some(
                        serde_json::to_string(&parse_yield(other)?).expect("serialisable Yield"),
                    ),
                };
                conn.execute(
                    "UPDATE attempts SET cooking_yield = ?2 WHERE id = ?1",
                    params![attempt_id, stored],
                )
                .map_err(|e| OpError::internal(format!("cannot set the cooking Yield: {e}")))?;
            }

            touch_attempt(conn, attempt_id)?;
            attempt_by_id(conn, attempt_id)
        })
    }

    /// End an In Progress Attempt. Ending is not what makes the cooking
    /// real — starting already did (ADR 0010) — only what stops it being
    /// In Progress.
    pub fn finish_attempt(&self, person_id: &str, attempt_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let state = attempt_state_owned_by(conn, attempt_id, person_id)?;
            if state.finished_at.is_some() {
                return Err(OpError::bad_request("this Attempt has already finished"));
            }
            conn.execute(
                "UPDATE attempts SET finished_at = strftime('%Y-%m-%dT%H:%M:%fZ','now'), \
                                      last_action_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
                 WHERE id = ?1",
                params![attempt_id],
            )
            .map_err(|e| OpError::internal(format!("cannot finish Attempt: {e}")))?;
            attempt_by_id(conn, attempt_id)
        })
    }

    /// Change an Attempt's free text or its five-star rating, whether it is
    /// still In Progress or long finished — an Attempt is freely editable
    /// by its cook (CONTEXT.md, "Attempt"), unlike the recipe it was cooked
    /// from. `None` leaves a field as it stood; `Some(&Value::Null)` clears
    /// it; any other value sets it, validated.
    pub fn edit_attempt(
        &self,
        person_id: &str,
        attempt_id: &str,
        note: Option<&Value>,
        rating: Option<&Value>,
    ) -> Result<Value, OpError> {
        if note.is_none() && rating.is_none() {
            return Err(OpError::bad_request(
                "edit_attempt takes at least one of note, rating",
            ));
        }
        self.db().with_conn(|conn| {
            let state = attempt_state_owned_by(conn, attempt_id, person_id)?;

            if let Some(value) = note {
                let stored = match value {
                    Value::Null => None,
                    Value::String(text) => Some(required_text(text, "note")?.to_string()),
                    _ => return Err(OpError::bad_request("note must be a string or null")),
                };
                conn.execute(
                    "UPDATE attempts SET note = ?2 WHERE id = ?1",
                    params![attempt_id, stored],
                )
                .map_err(|e| OpError::internal(format!("cannot save note: {e}")))?;
            }
            if let Some(value) = rating {
                let stored = match value {
                    Value::Null => None,
                    Value::Number(number) => Some(
                        number
                            .as_i64()
                            .filter(|n| (1..=5).contains(n))
                            .ok_or_else(|| {
                                OpError::bad_request("rating must be a whole number from 1 to 5")
                            })?,
                    ),
                    _ => {
                        return Err(OpError::bad_request(
                            "rating must be a whole number from 1 to 5, or null",
                        ));
                    }
                };
                conn.execute(
                    "UPDATE attempts SET rating = ?2 WHERE id = ?1",
                    params![attempt_id, stored],
                )
                .map_err(|e| OpError::internal(format!("cannot save rating: {e}")))?;
            }

            // Correcting a note or a rating mid-cook is itself an action —
            // the resume window counts from it exactly as advancing a Step
            // does. Once finished, resuming is never offered regardless, so
            // there is nothing to refresh.
            if state.finished_at.is_none() {
                touch_attempt(conn, attempt_id)?;
            }
            attempt_by_id(conn, attempt_id)
        })
    }

    /// Delete an Attempt outright — the explicit way a false start is
    /// undone, or any cooking record put away (ADR 0010). Never
    /// soft-deleted: this is the whole of how an Attempt leaves.
    pub fn delete_attempt(&self, person_id: &str, attempt_id: &str) -> Result<(), OpError> {
        self.db().with_conn(|conn| {
            attempt_state_owned_by(conn, attempt_id, person_id)?;
            conn.execute("DELETE FROM attempts WHERE id = ?1", params![attempt_id])
                .map_err(|e| OpError::internal(format!("cannot delete Attempt: {e}")))?;
            Ok(())
        })
    }

    /// Read the caller's own In Progress Attempt for a Lineage, if any —
    /// how two devices cooking the same dish stay in step (the last one to
    /// call `advance_attempt` is where the cook is), and whether resuming
    /// should still be offered. Scoped to the caller's own Attempts alone,
    /// so this needs no membership check of its own: whoever holds one
    /// already passed it when they started.
    pub fn get_current_attempt(&self, person_id: &str, lineage_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            Ok(json!({ "attempt": in_progress_attempt(conn, lineage_id, person_id)? }))
        })
    }

    /// Every Food this instance knows, each shown in the reader's Reading
    /// Language and falling back to whatever name it does have (#47).
    pub fn list_foods(&self, person_id: &str) -> Result<Vec<Value>, OpError> {
        self.db().with_conn(|conn| {
            let ids: Vec<String> = {
                let mut statement = conn
                    .prepare("SELECT id FROM foods ORDER BY created_at, id")
                    .map_err(|e| OpError::internal(format!("cannot list Foods: {e}")))?;
                statement
                    .query_map([], |row| row.get(0))
                    .map_err(|e| OpError::internal(format!("cannot list Foods: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot list Foods: {e}")))?
            };
            ids.iter()
                .map(|id| food_summary(conn, id, person_id))
                .collect()
        })
    }

    /// Read one Food back: its names, its Cup Weight and how many Readings
    /// currently point at it.
    pub fn get_food(&self, person_id: &str, food_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            ensure_food_exists(conn, food_id)?;
            food_summary(conn, food_id, person_id)
        })
    }

    /// Give a Food its name in one Language, or correct the one it has
    /// there. Any Person may (CONTEXT.md) — a Food is instance-wide, not a
    /// Kitchen's to guard. Where that word already answers for a different
    /// Food, this still attaches it here: typing a name onto a Food another
    /// already answers to is the one deliberate way to make a duplicate name
    /// (ADR 0022) rather than an error.
    pub fn set_food_name(
        &self,
        person_id: &str,
        food_id: &str,
        language: &str,
        name: &str,
    ) -> Result<Value, OpError> {
        let language = supported_language(language)?;
        let name = required_text(name, "name")?.to_string();
        self.db().with_conn(|conn| {
            ensure_food_exists(conn, food_id)?;
            conn.execute(
                "INSERT INTO food_names (food_id, language, name, name_folded) \
                 VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT(food_id, language) \
                 DO UPDATE SET name = excluded.name, name_folded = excluded.name_folded",
                params![food_id, language, name, folded_word(&name)],
            )
            .map_err(|e| OpError::internal(format!("cannot name Food: {e}")))?;
            food_summary(conn, food_id, person_id)
        })
    }

    /// Take a Food's name in one Language back off. A Food is known by its
    /// words alone, so its last remaining name may not be removed this way —
    /// deleting the Food nothing points at is the Operator's own power (#48).
    pub fn remove_food_name(
        &self,
        person_id: &str,
        food_id: &str,
        language: &str,
    ) -> Result<Value, OpError> {
        let language = supported_language(language)?;
        self.db().with_conn(|conn| {
            ensure_food_exists(conn, food_id)?;
            let name_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM food_names WHERE food_id = ?1",
                    params![food_id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot count Food names: {e}")))?;
            if name_count <= 1 {
                return Err(OpError::bad_request("a Food must keep at least one name"));
            }
            conn.execute(
                "DELETE FROM food_names WHERE food_id = ?1 AND language = ?2",
                params![food_id, language],
            )
            .map_err(|e| OpError::internal(format!("cannot remove Food name: {e}")))?;
            food_summary(conn, food_id, person_id)
        })
    }

    /// Set or clear a Food's Cup Weight — the one figure that turns a volume
    /// of it into a weight. Kamosu ships none by default; anyone may correct
    /// or add one (CONTEXT.md). `None` clears it back to "offers millilitres
    /// instead of grams".
    pub fn set_food_cup_weight(
        &self,
        person_id: &str,
        food_id: &str,
        cup_weight_grams: Option<f64>,
    ) -> Result<Value, OpError> {
        if cup_weight_grams.is_some_and(|grams| grams <= 0.0) {
            return Err(OpError::bad_request(
                "cup_weight_grams must be greater than zero",
            ));
        }
        self.db().with_conn(|conn| {
            ensure_food_exists(conn, food_id)?;
            conn.execute(
                "UPDATE foods SET cup_weight_grams = ?2 WHERE id = ?1",
                params![food_id, cup_weight_grams],
            )
            .map_err(|e| OpError::internal(format!("cannot set Cup Weight: {e}")))?;
            food_summary(conn, food_id, person_id)
        })
    }
}

/// Carry a Reading forward onto a freshly saved Version wherever the
/// Ingredient Line it belongs to reads identically (same kind, same text)
/// at the same position in both — the cheapest honest approximation of
/// Pairing (ADR 0019) available before that exists. A Reading whose line
/// moved or changed is left behind rather than guessed at.
fn carry_forward_readings(
    conn: &Connection,
    old_version_id: &str,
    old_content: &Value,
    new_version_id: &str,
    new_content: &Value,
) -> Result<(), OpError> {
    let no_lines = Vec::new();
    let old_lines = old_content["ingredients"].as_array().unwrap_or(&no_lines);
    let new_lines = new_content["ingredients"].as_array().unwrap_or(&no_lines);
    for (index, (old_line, new_line)) in old_lines.iter().zip(new_lines.iter()).enumerate() {
        if old_line != new_line {
            continue;
        }
        conn.execute(
            "INSERT OR IGNORE INTO readings (version_id, line_index, amount, unit, target, food_id) \
             SELECT ?1, line_index, amount, unit, target, food_id FROM readings \
              WHERE version_id = ?2 AND line_index = ?3",
            params![new_version_id, old_version_id, index as i64],
        )
        .map_err(|e| OpError::internal(format!("cannot carry Reading forward: {e}")))?;
    }
    Ok(())
}

/// Every Reading recorded against one Version, laid out as one slot per
/// Ingredient Line — `null` wherever no Reading has been recorded, which is
/// an entirely ordinary and permanent state for a line (ADR 0002).
fn readings_for_version(
    conn: &Connection,
    version_id: &str,
    line_count: usize,
) -> Result<Vec<Value>, OpError> {
    let mut slots = vec![Value::Null; line_count];
    let mut statement = conn
        .prepare("SELECT line_index, amount, unit, target FROM readings WHERE version_id = ?1")
        .map_err(|e| OpError::internal(format!("cannot read Readings: {e}")))?;
    let rows = statement
        .query_map(params![version_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(|e| OpError::internal(format!("cannot read Readings: {e}")))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| OpError::internal(format!("cannot read Readings: {e}")))?;
    for (line_index, amount, unit, target) in rows {
        if let Some(slot) = usize::try_from(line_index)
            .ok()
            .and_then(|index| slots.get_mut(index))
        {
            *slot = json!({ "amount": amount, "unit": unit, "target": target });
        }
    }
    Ok(slots)
}

/// How long a gap between saves on the same Branch, by the same Hand, still
/// collapses into the Version already being shaped rather than starting a
/// new one. Chosen at an hour: a save is a deliberate act, never periodic
/// autosave, so genuinely separate editing sessions land far apart in
/// practice — there is no realistic pattern this window would wrongly merge,
/// while it comfortably absorbs one meandering sitting, pauses included
/// (decided with Aurélien on issue #42).
const COLLAPSE_WINDOW_SECONDS: i64 = 3600;

/// The canonical serialisation a Version's fingerprint is taken over: object
/// keys sorted recursively, independent of `serde_json`'s own default Map
/// ordering, so a field added later stays deterministic.
fn canonical_json(value: &Value) -> String {
    fn canonicalise(value: &Value) -> Value {
        match value {
            Value::Object(map) => {
                let sorted: std::collections::BTreeMap<String, Value> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), canonicalise(v)))
                    .collect();
                json!(sorted)
            }
            Value::Array(items) => Value::Array(items.iter().map(canonicalise).collect()),
            other => other.clone(),
        }
    }
    canonicalise(value).to_string()
}

/// A Version's id: the fingerprint of its content alone (ADR 0004, ADR 0021).
fn fingerprint_content(content: &Value) -> String {
    format!(
        "v_{}",
        hex::encode(Sha256::digest(canonical_json(content).as_bytes()))
    )
}

/// Build and validate the stored shape of a Recipe's content out of raw
/// request input (#43): the title, the optional Yield, Prep/Cook Time, Note
/// and Source, and the Ingredient Line and Step lists — each a flat, ordered
/// sequence in which a Section is a real entry rather than a faked line
/// (CONTEXT.md, "Section"). Every field but the title is optional and
/// normalises to `null` or `[]` when absent, so `{ "title": "..." }` alone is
/// a complete, valid Recipe. This is the whole state, never a delta: calling
/// it again with fields left out replaces them, exactly as a fresh save of
/// the title alone already did before this ticket.
fn parse_recipe_content(input: &Value) -> Result<Value, OpError> {
    let title = input
        .get("title")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("title is required"))?;
    let title = required_text(title, "title")?.to_string();

    let recipe_yield = match input.get("yield") {
        None | Some(Value::Null) => Value::Null,
        Some(value) => parse_yield(value)?,
    };
    let prep_time_minutes =
        parse_optional_minutes(input.get("prep_time_minutes"), "prep_time_minutes")?;
    let cook_time_minutes =
        parse_optional_minutes(input.get("cook_time_minutes"), "cook_time_minutes")?;
    let note = match input.get("note") {
        None | Some(Value::Null) => Value::Null,
        Some(Value::String(text)) => json!(text),
        Some(_) => return Err(OpError::bad_request("note must be a string or null")),
    };
    // The Main Photo is carried only as a reference to a Photograph already
    // uploaded (#45, ADR 0017) — the same loose, unvalidated pointer a Step's
    // photo already is, and part of the fingerprint below for the same reason.
    let main_photo = match input.get("main_photo") {
        None | Some(Value::Null) => None,
        Some(Value::String(photo)) if !photo.is_empty() => Some(photo.as_str()),
        Some(Value::String(_)) => None,
        Some(_) => return Err(OpError::bad_request("main_photo must be a string or null")),
    };
    let source = match input.get("source") {
        None | Some(Value::Null) => Value::Null,
        Some(value) => parse_source(value)?,
    };
    let ingredients = parse_line_list(input.get("ingredients"), "ingredients", "ingredient")?;
    let steps = parse_step_list(input.get("steps"))?;

    Ok(json!({
        "title": title,
        "yield": recipe_yield,
        "prep_time_minutes": prep_time_minutes,
        "cook_time_minutes": cook_time_minutes,
        "note": note,
        "main_photo": main_photo,
        "source": source,
        "ingredients": ingredients,
        "steps": steps,
    }))
}

/// A Yield: one amount and what it is an amount of — "4 servings", "24
/// cookies", "1.5 litres" are all the same field (CONTEXT.md, "Yield"), kept
/// as written rather than parsed into a number and a Unit. The noun is a
/// distinct concept from Unit (grams, cups, spoons — a closed, convertible
/// list), so it is never called `unit` here.
fn parse_yield(value: &Value) -> Result<Value, OpError> {
    let object = value
        .as_object()
        .ok_or_else(|| OpError::bad_request("yield must be an object with amount and noun"))?;
    let amount = object
        .get("amount")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("yield.amount is required"))?;
    let amount = required_text(amount, "yield.amount")?.to_string();
    let noun = object
        .get("noun")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("yield.noun is required"))?;
    let noun = required_text(noun, "yield.noun")?.to_string();
    Ok(json!({ "amount": amount, "noun": noun }))
}

/// Prep Time and Cook Time are whole minutes; Cook Time includes resting,
/// proving, marinating and chilling — one field for however the dish spends
/// unattended time, documented at the Catalogue level for any Door reading
/// it (CONTEXT.md, "Cook Time").
fn parse_optional_minutes(value: Option<&Value>, field: &str) -> Result<Option<i64>, OpError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(number)) => number
            .as_i64()
            .filter(|minutes| *minutes >= 0)
            .map(Some)
            .ok_or_else(|| {
                OpError::bad_request(format!(
                    "{field} must be a whole number of minutes, zero or more"
                ))
            }),
        Some(_) => Err(OpError::bad_request(format!(
            "{field} must be a whole number of minutes, zero or more"
        ))),
    }
}

/// A Source: free text with an optional link — "Mum's ring binder, p.40" is
/// as real an attribution as a URL (CONTEXT.md, "Source").
fn parse_source(value: &Value) -> Result<Value, OpError> {
    let object = value.as_object().ok_or_else(|| {
        OpError::bad_request("source must be an object with text and an optional link")
    })?;
    let text = object
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("source.text is required"))?;
    let text = required_text(text, "source.text")?.to_string();
    let link = match object.get("link") {
        None | Some(Value::Null) => None,
        Some(Value::String(link)) => {
            let link = link.trim();
            if link.is_empty() { None } else { Some(link) }
        }
        Some(_) => return Err(OpError::bad_request("source.link must be a string or null")),
    };
    Ok(json!({ "text": text, "link": link }))
}

/// The Ingredient list: a flat, ordered sequence of Sections and Ingredient
/// Lines, each kept exactly as typed (ADR 0002) — never trimmed, never
/// rewritten. Absent entirely, this is an empty list rather than an error,
/// since a bare-name recipe with no ingredients yet is a real Recipe.
fn parse_line_list(value: Option<&Value>, field: &str, line_kind: &str) -> Result<Value, OpError> {
    let items = match value {
        None => return Ok(json!([])),
        Some(Value::Array(items)) => items,
        Some(_) => return Err(OpError::bad_request(format!("{field} must be an array"))),
    };
    let mut parsed = Vec::with_capacity(items.len());
    for item in items {
        let object = item
            .as_object()
            .ok_or_else(|| OpError::bad_request(format!("each {field} entry must be an object")))?;
        let kind = object
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| OpError::bad_request(format!("each {field} entry needs a kind")))?;
        if kind != "section" && kind != line_kind {
            return Err(OpError::bad_request(format!(
                "{field} entries must be 'section' or '{line_kind}'"
            )));
        }
        // The written line itself: preserved verbatim, so only its raw string
        // reaches storage — `required_text`'s trim is used to reject a
        // whitespace-only line, never to alter what is kept.
        let text = object
            .get("text")
            .and_then(Value::as_str)
            .ok_or_else(|| OpError::bad_request(format!("each {field} entry needs text")))?;
        required_text(text, "text")?;
        parsed.push(json!({ "kind": kind, "text": text }));
    }
    Ok(Value::Array(parsed))
}

/// The Step list: the same flat Section-and-entry shape as the Ingredient
/// list, where a Step carries text and an optional photo — no timer or
/// temperature field to fill in (CONTEXT.md, "Step"). The photo is carried
/// only as a reference to a Photograph already uploaded through
/// `upload_photograph`, unvalidated here the way any other pointer in this
/// codebase is (ADR 0017).
fn parse_step_list(value: Option<&Value>) -> Result<Value, OpError> {
    let items = match value {
        None => return Ok(json!([])),
        Some(Value::Array(items)) => items,
        Some(_) => return Err(OpError::bad_request("steps must be an array")),
    };
    let mut parsed = Vec::with_capacity(items.len());
    for item in items {
        let object = item
            .as_object()
            .ok_or_else(|| OpError::bad_request("each step entry must be an object"))?;
        let kind = object
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| OpError::bad_request("each step entry needs a kind"))?;
        if kind != "section" && kind != "step" {
            return Err(OpError::bad_request(
                "step entries must be 'section' or 'step'",
            ));
        }
        let text = object
            .get("text")
            .and_then(Value::as_str)
            .ok_or_else(|| OpError::bad_request("each step entry needs text"))?;
        required_text(text, "text")?;
        let photo = match object.get("photo") {
            None | Some(Value::Null) => None,
            Some(Value::String(photo)) if !photo.is_empty() => Some(photo.as_str()),
            Some(Value::String(_)) => None,
            Some(_) => return Err(OpError::bad_request("step photo must be a string or null")),
        };
        parsed.push(json!({ "kind": kind, "text": text, "photo": photo }));
    }
    Ok(Value::Array(parsed))
}

/// The Attempt state every write Operation on it needs before doing
/// anything else: what it was pinned to, and whether it has already
/// finished. Not the full read shape `attempt_by_id` answers — just enough
/// to decide whether the caller may act at all.
struct AttemptState {
    version_id: String,
    finished_at: Option<String>,
}

/// Look up an Attempt's state and prove the caller owns it — the one check
/// `advance_attempt`, `finish_attempt`, `edit_attempt` and `delete_attempt`
/// all open with, since an Attempt is a private diary until its cook says
/// otherwise (ADR 0005): no Kitchen membership substitutes for it.
fn attempt_state_owned_by(
    conn: &Connection,
    attempt_id: &str,
    person_id: &str,
) -> Result<AttemptState, OpError> {
    let (owner_id, version_id, finished_at): (String, String, Option<String>) = conn
        .query_row(
            "SELECT person_id, version_id, finished_at FROM attempts WHERE id = ?1",
            params![attempt_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read Attempt: {e}")))?
        .ok_or_else(|| OpError::not_found("no such Attempt"))?;
    if owner_id != person_id {
        return Err(OpError::unauthorized(
            "this Attempt belongs to someone else",
        ));
    }
    Ok(AttemptState {
        version_id,
        finished_at,
    })
}

/// Read one Attempt back in the shape `attempt_schema` declares.
/// `resumable` is computed in SQL rather than in Rust: still In Progress
/// and within three days of `last_action_at` (ADR 0010) — the same clock
/// every other timestamp in Kamosu is stamped from, `strftime('now')`.
fn attempt_by_id(conn: &Connection, id: &str) -> Result<Value, OpError> {
    conn.query_row(
        "SELECT id, lineage_id, person_id, version_id, current_step_index, \
                ticked_ingredients, cooking_yield, note, rating, finished_at, \
                created_at, last_action_at, \
                CASE WHEN finished_at IS NULL \
                          AND julianday('now') - julianday(last_action_at) <= 3.0 \
                     THEN 1 ELSE 0 END AS resumable \
           FROM attempts WHERE id = ?1",
        params![id],
        |row| {
            let ticked_ingredients: String = row.get(5)?;
            let cooking_yield: Option<String> = row.get(6)?;
            let resumable: i64 = row.get(12)?;
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "lineage_id": row.get::<_, String>(1)?,
                "person_id": row.get::<_, String>(2)?,
                "version_id": row.get::<_, String>(3)?,
                "current_step_index": row.get::<_, i64>(4)?,
                "ticked_ingredients": serde_json::from_str::<Value>(&ticked_ingredients)
                    .unwrap_or(json!([])),
                "cooking_yield": cooking_yield
                    .and_then(|text| serde_json::from_str::<Value>(&text).ok())
                    .unwrap_or(Value::Null),
                "note": row.get::<_, Option<String>>(7)?,
                "rating": row.get::<_, Option<i64>>(8)?,
                "finished_at": row.get::<_, Option<String>>(9)?,
                "created_at": row.get::<_, String>(10)?,
                "last_action_at": row.get::<_, String>(11)?,
                "resumable": resumable != 0,
            }))
        },
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read Attempt: {e}")))?
    .ok_or_else(|| OpError::not_found("no such Attempt"))
}

/// The one Attempt a Person may have In Progress on a Lineage at a time
/// (ADR 0010), if any.
fn in_progress_attempt(
    conn: &Connection,
    lineage_id: &str,
    person_id: &str,
) -> Result<Option<Value>, OpError> {
    let id: Option<String> = conn
        .query_row(
            "SELECT id FROM attempts WHERE lineage_id = ?1 AND person_id = ?2 \
             AND finished_at IS NULL",
            params![lineage_id, person_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read Attempt: {e}")))?;
    id.map(|id| attempt_by_id(conn, &id)).transpose()
}

/// Record that the cook just did something — starting, resuming or
/// advancing — so the three-day resume window (ADR 0010) counts from now.
fn touch_attempt(conn: &Connection, id: &str) -> Result<(), OpError> {
    conn.execute(
        "UPDATE attempts SET last_action_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?1",
        params![id],
    )
    .map_err(|e| OpError::internal(format!("cannot update Attempt: {e}")))?;
    Ok(())
}

/// How many entries a Version's `"steps"` or `"ingredients"` list holds —
/// the bound `advance_attempt` checks a Step index or a ticked Ingredient
/// index against, read from the pinned Version rather than the recipe's
/// current head, since an Attempt never moves off the Version it started on.
fn version_field_len(conn: &Connection, version_id: &str, field: &str) -> Result<i64, OpError> {
    let content: String = conn
        .query_row(
            "SELECT content FROM versions WHERE id = ?1",
            params![version_id],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot read Version: {e}")))?;
    let content: Value = serde_json::from_str(&content)
        .map_err(|e| OpError::internal(format!("cannot read Version content: {e}")))?;
    Ok(content[field].as_array().map(Vec::len).unwrap_or(0) as i64)
}

/// Create a Kitchen and seat its first member in one place — the shape a
/// Person's Home Kitchen and any Kitchen they later create both share.
/// What became of one Import candidate against the ledger.
enum ImportOutcome {
    /// Newly made (`status: "created"`) or matched and found unchanged
    /// (`status: "unchanged"`) — both `arrived`, told apart only by that tag.
    Landed {
        lineage_id: String,
        branch_id: String,
        title: String,
        status: &'static str,
    },
    Offered {
        lineage_id: String,
        branch_id: String,
        title: String,
        candidate_version_id: String,
    },
}

/// Mint a brand-new Lineage, a Branch of it in `kitchen_id`, and its first
/// Version — the one sequence `create_recipe` and a freshly-seen Import
/// candidate both start from (ADR 0004): the Kitchen's own Hand on the
/// Branch, the writing Person's Hand on this first Version.
#[allow(clippy::too_many_arguments)]
fn insert_new_lineage_and_branch(
    conn: &Connection,
    lineage_id: &str,
    branch_id: &str,
    kitchen_id: &str,
    kitchen_hand_id: &str,
    language: &str,
    version_id: &str,
    content_text: &str,
    writer_person_id: &str,
    access_key_id: Option<&str>,
) -> Result<(), OpError> {
    conn.execute(
        "INSERT OR IGNORE INTO versions (id, content) VALUES (?1, ?2)",
        params![version_id, content_text],
    )
    .map_err(|e| OpError::internal(format!("cannot record Version: {e}")))?;
    conn.execute("INSERT INTO lineages (id) VALUES (?1)", params![lineage_id])
        .map_err(|e| OpError::internal(format!("cannot mint Lineage: {e}")))?;
    conn.execute(
        "INSERT INTO branches (id, lineage_id, kitchen_id, hand_id, language, head_version_id) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            branch_id,
            lineage_id,
            kitchen_id,
            kitchen_hand_id,
            language,
            version_id
        ],
    )
    .map_err(|e| OpError::internal(format!("cannot start Branch: {e}")))?;
    conn.execute(
        "INSERT INTO branch_versions \
         (branch_id, sequence, version_id, parent_version_id, hand_id, access_key_id) \
         VALUES (?1, 1, ?2, NULL, ?3, ?4)",
        params![branch_id, version_id, writer_person_id, access_key_id],
    )
    .map_err(|e| OpError::internal(format!("cannot record first Version: {e}")))?;
    Ok(())
}

/// The Import a Kitchen runs candidates of one source kind through — the one
/// already open for a re-run, or a fresh one on the first run (ADR 0025).
fn find_or_create_import(
    conn: &Connection,
    kitchen_id: &str,
    source_kind: &str,
) -> Result<String, OpError> {
    if let Some(id) = conn
        .query_row(
            "SELECT id FROM imports WHERE kitchen_id = ?1 AND source_kind = ?2",
            params![kitchen_id, source_kind],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read Import: {e}")))?
    {
        return Ok(id);
    }
    let id = format!("imp_{}", hex::encode(random_bytes(8)));
    conn.execute(
        "INSERT INTO imports (id, kitchen_id, source_kind) VALUES (?1, ?2, ?3)",
        params![id, kitchen_id, source_kind],
    )
    .map_err(|e| OpError::internal(format!("cannot start Import: {e}")))?;
    Ok(id)
}

fn insert_kitchen_with_member(
    conn: &rusqlite::Connection,
    kitchen_id: &str,
    name: &str,
    person_id: &str,
) -> Result<(), OpError> {
    conn.execute(
        "INSERT INTO kitchens (id, name, hand_id) VALUES (?1, ?2, ?1)",
        params![kitchen_id, name],
    )
    .map_err(|e| OpError::internal(format!("cannot create Kitchen: {e}")))?;
    conn.execute(
        "INSERT INTO kitchen_members (kitchen_id, person_id) VALUES (?1, ?2)",
        params![kitchen_id, person_id],
    )
    .map_err(|e| OpError::internal(format!("cannot add Person to Kitchen: {e}")))?;
    Ok(())
}

/// Whether a Person cooks in a Kitchen.
fn is_member(
    conn: &rusqlite::Connection,
    kitchen_id: &str,
    person_id: &str,
) -> Result<bool, OpError> {
    conn.query_row(
        "SELECT COUNT(*) FROM kitchen_members WHERE kitchen_id = ?1 AND person_id = ?2",
        params![kitchen_id, person_id],
        |row| row.get::<_, i64>(0),
    )
    .map(|n| n > 0)
    .map_err(|e| OpError::internal(format!("cannot check Kitchen membership: {e}")))
}

/// Only a member may change a Kitchen — the only circle that may (ADR 0007).
fn ensure_member(
    conn: &rusqlite::Connection,
    kitchen_id: &str,
    person_id: &str,
) -> Result<(), OpError> {
    if is_member(conn, kitchen_id, person_id)? {
        Ok(())
    } else {
        Err(OpError::unauthorized(
            "this Person does not cook in this Kitchen",
        ))
    }
}

/// One Kitchen, as the Person asking sees it: their own Nickname (never
/// anyone else's), whether it is their Home Kitchen, and who else is in it.
fn kitchen_summary(
    conn: &rusqlite::Connection,
    kitchen_id: &str,
    viewer_person_id: &str,
) -> Result<Value, OpError> {
    let (name, hand_id): (String, String) = conn
        .query_row(
            "SELECT name, hand_id FROM kitchens WHERE id = ?1",
            params![kitchen_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| OpError::internal(format!("cannot read Kitchen: {e}")))?;
    let nickname: Option<String> = conn
        .query_row(
            "SELECT nickname FROM kitchen_members WHERE kitchen_id = ?1 AND person_id = ?2",
            params![kitchen_id, viewer_person_id],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot read Nickname: {e}")))?;
    let home_kitchen_id: Option<String> = conn
        .query_row(
            "SELECT home_kitchen_id FROM people WHERE id = ?1",
            params![viewer_person_id],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot read Home Kitchen: {e}")))?;
    let mut statement = conn
        .prepare(
            "SELECT people.id, people.name FROM kitchen_members \
             JOIN people ON people.id = kitchen_members.person_id \
             WHERE kitchen_members.kitchen_id = ?1 ORDER BY people.name",
        )
        .map_err(|e| OpError::internal(format!("cannot list Kitchen members: {e}")))?;
    let members: Vec<Value> = statement
        .query_map(params![kitchen_id], |row| {
            Ok(json!({
                "person_id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
            }))
        })
        .map_err(|e| OpError::internal(format!("cannot list Kitchen members: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot list Kitchen members: {e}")))?;

    Ok(json!({
        "id": kitchen_id,
        "name": name,
        "hand_id": hand_id,
        "is_home": home_kitchen_id.as_deref() == Some(kitchen_id),
        "nickname": nickname,
        "members": members,
    }))
}

/// The Languages Kamosu is written in. A Tag may be named in any of them and
/// need be named in only one.
const LANGUAGES: [&str; 3] = ["en", "fr", "es"];

fn supported_language(language: &str) -> Result<&str, OpError> {
    if LANGUAGES.contains(&language) {
        Ok(language)
    } else {
        Err(OpError::bad_request(format!(
            "language must be one of {}",
            LANGUAGES.join(", ")
        )))
    }
}

/// A Person's own Reading Language — the first thing consulted whenever
/// something named per Language (a Tag, a Food) is shown back to them.
fn reading_language_of(conn: &rusqlite::Connection, person_id: &str) -> Result<String, OpError> {
    conn.query_row(
        "SELECT reading_language FROM people WHERE id = ?1",
        params![person_id],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot read reading Language: {e}")))
}

/// Of a set of `(language, name)` pairs, the one to show a reader — their own
/// Reading Language where it has a name there, and otherwise the fixed order
/// of [`LANGUAGES`] rather than whatever order SQLite happened to return
/// rows in. Shared by `tag_summary` (#51) and `food_summary` (#47), which
/// show a per-Language name the identical way.
fn shown_name<'a>(
    named: &'a [(String, String)],
    reading_language: &str,
) -> Option<&'a (String, String)> {
    named
        .iter()
        .find(|(language, _)| language == reading_language)
        .or_else(|| {
            LANGUAGES
                .iter()
                .find_map(|wanted| named.iter().find(|(language, _)| language == wanted))
        })
}

/// The same `(language, name)` pairs laid out as `list_tags`/`list_foods`
/// serve them: one entry per Language that has one, in [`LANGUAGES`]' fixed
/// order.
fn names_json(named: &[(String, String)]) -> Vec<Value> {
    LANGUAGES
        .iter()
        .filter_map(|wanted| {
            named
                .iter()
                .find(|(language, _)| language == wanted)
                .map(|(language, name)| json!({ "language": language, "name": name }))
        })
        .collect()
}

/// Which Kitchen a Tag belongs to — and, by failing, that it exists at all.
fn kitchen_of_tag(conn: &rusqlite::Connection, tag_id: &str) -> Result<String, OpError> {
    conn.query_row(
        "SELECT kitchen_id FROM tags WHERE id = ?1",
        params![tag_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read Tag: {e}")))?
    .ok_or_else(|| OpError::not_found("no such Tag"))
}

/// One word reduced to the form every spelling of it shares, so a Kitchen can
/// hold one word once. This is Unicode canonical caseless matching — decompose,
/// case-fold, decompose again — written out as a value rather than a comparison
/// so the fold can be stored and indexed.
///
/// It settles both ways one word arrives looking like two. Case: *Été* and
/// *été* are one word, which SQLite's own NOCASE cannot say, folding ASCII
/// alone. And shape: an *é* typed as one character and an *e* followed by a
/// combining accent look identical on screen and are different bytes — a French
/// cookbook meets both, depending on the keyboard.
///
/// What it does not fold is what the Unicode default fold leaves alone: *straße*
/// and *strasse* stay two words. That is the standard's own line, not one drawn
/// here.
fn folded_word(name: &str) -> String {
    use caseless::Caseless;
    use unicode_normalization::UnicodeNormalization;
    name.chars().nfd().default_case_fold().nfd().collect()
}

/// The Tag a Kitchen already files under this word in this Language, if any.
/// Compared on the fold, which is what the schema holds unique.
fn tag_id_for_word(
    conn: &rusqlite::Connection,
    kitchen_id: &str,
    language: &str,
    name: &str,
) -> Result<Option<String>, OpError> {
    conn.query_row(
        "SELECT tag_id FROM tag_names \
           WHERE kitchen_id = ?1 AND language = ?2 AND name_folded = ?3",
        params![kitchen_id, language, folded_word(name)],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot look up Tag: {e}")))
}

/// One Tag as a reader sees it: every name it has, and the one to show them —
/// their Reading Language where the Tag has a name there, and otherwise
/// whatever name it does have (#51). `language` says which of the two happened,
/// so a screen can mark a word standing in from another Language rather than
/// having to guess.
fn tag_summary(
    conn: &rusqlite::Connection,
    tag_id: &str,
    viewer_person_id: &str,
) -> Result<Value, OpError> {
    let kitchen_id = kitchen_of_tag(conn, tag_id)?;
    let mut statement = conn
        .prepare("SELECT language, name FROM tag_names WHERE tag_id = ?1")
        .map_err(|e| OpError::internal(format!("cannot read Tag names: {e}")))?;
    let named: Vec<(String, String)> = statement
        .query_map(params![tag_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| OpError::internal(format!("cannot read Tag names: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read Tag names: {e}")))?;

    let reading_language = reading_language_of(conn, viewer_person_id)?;
    let shown = shown_name(&named, &reading_language);
    let names = names_json(&named);

    Ok(json!({
        "id": tag_id,
        "kitchen_id": kitchen_id,
        "name": shown.map(|(_, name)| name.as_str()),
        "language": shown.map(|(language, _)| language.as_str()),
        "names": names,
    }))
}

/// The Tags one recipe carries, as its reader sees them.
fn tags_of_branch(
    conn: &rusqlite::Connection,
    branch_id: &str,
    viewer_person_id: &str,
) -> Result<Vec<Value>, OpError> {
    let ids: Vec<String> = {
        let mut statement = conn
            .prepare(
                "SELECT branch_tags.tag_id FROM branch_tags \
                 JOIN tags ON tags.id = branch_tags.tag_id \
                 WHERE branch_tags.branch_id = ?1 ORDER BY tags.created_at, tags.id",
            )
            .map_err(|e| OpError::internal(format!("cannot read a recipe's Tags: {e}")))?;
        statement
            .query_map(params![branch_id], |row| row.get(0))
            .map_err(|e| OpError::internal(format!("cannot read a recipe's Tags: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot read a recipe's Tags: {e}")))?
    };
    ids.iter()
        .map(|id| tag_summary(conn, id, viewer_person_id))
        .collect()
}

/// Kamosu's whole automatic Food Match (#47, ADR 0022): given the name or
/// names one arriving Food is known by — `(language, word)` pairs — resolve
/// which Food this is, creating one if nothing on the instance answers.
///
/// `exclude`, when given, names the one Reading (`version_id`, `line_index`)
/// this resolution is *for* — so that when `set_reading` corrects an
/// already-read line into fresh ambiguity, the busiest-Food tie-break
/// (ADR 0022) counts existing Readings only, not the very row about to be
/// overwritten still carrying its old Food.
///
/// A Reading or Kamosu's own parser always arrives knowing exactly one name
/// ("one rule at every door" — ADR 0022), which is what `resolve_food_for_word`
/// reduces to below. A Bundle importing a Food from another instance arrives
/// with one entry per Language it has a name in, and is the only situation in
/// which more than one name arrives together — no Operation in this
/// Catalogue does that yet, so the multi-name branches below are exercised
/// directly by `resolve_food_for_names_makes_a_third_food_on_collision`
/// rather than through an Operation, and will carry Bundle import's own
/// behaviour tests once that ticket lands.
fn resolve_food_for_names(
    conn: &Connection,
    names: &[(&str, &str)],
    exclude: Option<(&str, i64)>,
) -> Result<String, OpError> {
    assert!(
        !names.is_empty(),
        "a Food always arrives known by at least one name"
    );

    // Every existing Food that any arriving name already answers to, in the
    // same Language, deduplicated — two arriving names may hit the same Food.
    let mut hits: Vec<String> = Vec::new();
    for (language, word) in names {
        let folded = folded_word(word);
        let mut statement = conn
            .prepare("SELECT food_id FROM food_names WHERE language = ?1 AND name_folded = ?2")
            .map_err(|e| OpError::internal(format!("cannot look up Food: {e}")))?;
        let found: Vec<String> = statement
            .query_map(params![language, folded], |row| row.get(0))
            .map_err(|e| OpError::internal(format!("cannot look up Food: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot look up Food: {e}")))?;
        for food_id in found {
            if !hits.contains(&food_id) {
                hits.push(food_id);
            }
        }
    }

    if names.len() == 1 && hits.len() >= 2 {
        // A lone word asserts nothing about which Food is meant — welding
        // two Foods together on that alone is the very weld ADR 0022
        // refuses. Go with whichever the most Readings already point at.
        return busiest_food(conn, &hits, exclude);
    }

    match hits.len() {
        0 => create_food(conn, names),
        1 => {
            // The surviving Food learns every arriving name that matches
            // nothing at all on the instance (ADR 0022) — but never
            // overwrites a Language it already has a name in.
            let food_id = hits.into_iter().next().unwrap();
            let held: Vec<String> = {
                let mut statement = conn
                    .prepare("SELECT language FROM food_names WHERE food_id = ?1")
                    .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?;
                statement
                    .query_map(params![food_id], |row| row.get(0))
                    .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?
            };
            for (language, word) in names {
                if held.iter().any(|held_language| held_language == language) {
                    continue;
                }
                conn.execute(
                    "INSERT INTO food_names (food_id, language, name, name_folded) \
                     VALUES (?1, ?2, ?3, ?4)",
                    params![food_id, language, word, folded_word(word)],
                )
                .map_err(|e| OpError::internal(format!("cannot name Food: {e}")))?;
            }
            Ok(food_id)
        }
        _ => {
            // Doubt makes a new Food, never a merge (ADR 0022): a third Food
            // carries every name every hit Food already has, plus every
            // arriving name for a Language none of them covers. The hit
            // Foods themselves are untouched — nothing merged, nothing
            // destroyed.
            let mut carried: Vec<(String, String)> = Vec::new();
            for food_id in &hits {
                let mut statement = conn
                    .prepare("SELECT language, name FROM food_names WHERE food_id = ?1")
                    .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?;
                let rows: Vec<(String, String)> = statement
                    .query_map(params![food_id], |row| Ok((row.get(0)?, row.get(1)?)))
                    .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?;
                for (language, name) in rows {
                    if !carried
                        .iter()
                        .any(|(carried_language, _)| *carried_language == language)
                    {
                        carried.push((language, name));
                    }
                }
            }
            for (language, word) in names {
                if !carried
                    .iter()
                    .any(|(carried_language, _)| carried_language == language)
                {
                    carried.push(((*language).to_string(), (*word).to_string()));
                }
            }
            let carried_refs: Vec<(&str, &str)> = carried
                .iter()
                .map(|(language, name)| (language.as_str(), name.as_str()))
                .collect();
            create_food(conn, &carried_refs)
        }
    }
}

/// The lone-word reduction of [`resolve_food_for_names`] — what a Reading's
/// `target` or Kamosu's own parser always supplies.
fn resolve_food_for_word(
    conn: &Connection,
    language: &str,
    word: &str,
    exclude: Option<(&str, i64)>,
) -> Result<String, OpError> {
    resolve_food_for_names(conn, &[(language, word)], exclude)
}

/// Among Foods a lone ambiguous word hit, the one the most Readings already
/// point at (ADR 0022) — `exclude` left out of that count, see
/// `resolve_food_for_names`. Ties — commonly all-zero, since nothing has
/// read either yet — resolve to whichever Food was minted first, so the
/// same input always resolves the same way rather than to whatever order
/// SQLite happened to return rows in.
fn busiest_food(
    conn: &Connection,
    candidates: &[String],
    exclude: Option<(&str, i64)>,
) -> Result<String, OpError> {
    let placeholders = candidates
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");
    let join_condition = if exclude.is_some() {
        "readings.food_id = foods.id \
         AND NOT (readings.version_id = ? AND readings.line_index = ?)"
    } else {
        "readings.food_id = foods.id"
    };
    let sql = format!(
        "SELECT foods.id FROM foods \
         LEFT JOIN readings ON {join_condition} \
         WHERE foods.id IN ({placeholders}) \
         GROUP BY foods.id \
         ORDER BY COUNT(readings.food_id) DESC, foods.created_at ASC, foods.id ASC \
         LIMIT 1"
    );
    let mut bound: Vec<&dyn rusqlite::ToSql> = Vec::new();
    if let Some((version_id, line_index)) = &exclude {
        bound.push(version_id);
        bound.push(line_index);
    }
    for candidate in candidates {
        bound.push(candidate);
    }
    conn.query_row(&sql, bound.as_slice(), |row| row.get(0))
        .map_err(|e| OpError::internal(format!("cannot resolve ambiguous Food: {e}")))
}

/// Mint a brand-new Food carrying exactly the given names — the "created
/// automatically from whatever word a Reading found" half of #47.
fn create_food(conn: &Connection, names: &[(&str, &str)]) -> Result<String, OpError> {
    let food_id = format!("f_{}", hex::encode(random_bytes(8)));
    conn.execute("INSERT INTO foods (id) VALUES (?1)", params![food_id])
        .map_err(|e| OpError::internal(format!("cannot create Food: {e}")))?;
    for (language, word) in names {
        conn.execute(
            "INSERT INTO food_names (food_id, language, name, name_folded) VALUES (?1, ?2, ?3, ?4)",
            params![food_id, language, word, folded_word(word)],
        )
        .map_err(|e| OpError::internal(format!("cannot name Food: {e}")))?;
    }
    Ok(food_id)
}

/// That a Food exists at all — and, by failing, that it doesn't.
fn ensure_food_exists(conn: &rusqlite::Connection, food_id: &str) -> Result<(), OpError> {
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM foods WHERE id = ?1",
            params![food_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .map_err(|e| OpError::internal(format!("cannot look up Food: {e}")))?;
    if !exists {
        return Err(OpError::not_found("no such Food"));
    }
    Ok(())
}

/// One Food as a reader sees it: every name it has, the one to show them —
/// their Reading Language where the Food has a name there, and otherwise
/// whatever name it does have, the same fallback `tag_summary` uses (#51) —
/// its Cup Weight, its (always-null in v1) nutrition slot, and how many
/// Readings currently point at it. The reading count is what lets the
/// ADR 0022 busiest tie-break be verified through Operations rather than
/// taken on faith, and is exactly the number an unreferenced-Food sweep
/// would need to be zero before ever touching one (#48).
fn food_summary(
    conn: &rusqlite::Connection,
    food_id: &str,
    viewer_person_id: &str,
) -> Result<Value, OpError> {
    let cup_weight_grams: Option<f64> = conn
        .query_row(
            "SELECT cup_weight_grams FROM foods WHERE id = ?1",
            params![food_id],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot read Food: {e}")))?;

    let reading_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM readings WHERE food_id = ?1",
            params![food_id],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot count a Food's Readings: {e}")))?;

    let mut statement = conn
        .prepare("SELECT language, name FROM food_names WHERE food_id = ?1")
        .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?;
    let named: Vec<(String, String)> = statement
        .query_map(params![food_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?;

    let reading_language = reading_language_of(conn, viewer_person_id)?;
    let shown = shown_name(&named, &reading_language);
    let names = names_json(&named);

    Ok(json!({
        "id": food_id,
        "name": shown.map(|(_, name)| name.as_str()),
        "language": shown.map(|(language, _)| language.as_str()),
        "names": names,
        "cup_weight_grams": cup_weight_grams,
        "nutrition": Value::Null,
        "reading_count": reading_count,
    }))
}

/// The Kitchen, Lineage, and current title of one Branch. These are the three
/// facts a shelf link needs from either end, so the two lookups in
/// `set_related_recipe` cannot drift apart.
fn recipe_shelf_identity(
    conn: &rusqlite::Connection,
    branch_id: &str,
) -> Result<Option<(String, String, String)>, OpError> {
    conn.query_row(
        "SELECT branches.kitchen_id, branches.lineage_id, json_extract(versions.content, '$.title') \
         FROM branches JOIN versions ON versions.id = branches.head_version_id WHERE branches.id = ?1",
        params![branch_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read Recipe: {e}")))
}

/// The Related Recipes one Lineage shows on one Kitchen's shelf. A Lineage
/// that is no longer held there is deliberately returned with its remembered
/// title and no Branch id: text is better than a broken pointer (#52).
fn related_recipes_of_lineage(
    conn: &rusqlite::Connection,
    kitchen_id: &str,
    lineage_id: &str,
) -> Result<Vec<Value>, OpError> {
    let links: Vec<(String, String)> = {
        let mut statement = conn
            .prepare(
                "SELECT CASE WHEN lineage_a_id = ?2 THEN lineage_b_id ELSE lineage_a_id END, \
                        CASE WHEN lineage_a_id = ?2 THEN lineage_b_name ELSE lineage_a_name END \
                 FROM related_recipes \
                 WHERE kitchen_id = ?1 AND (lineage_a_id = ?2 OR lineage_b_id = ?2) \
                 ORDER BY created_at, lineage_a_id, lineage_b_id",
            )
            .map_err(|e| OpError::internal(format!("cannot read Related Recipes: {e}")))?;
        statement
            .query_map(params![kitchen_id, lineage_id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|e| OpError::internal(format!("cannot read Related Recipes: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot read Related Recipes: {e}")))?
    };

    links
        .into_iter()
        .map(|(related_lineage_id, remembered_title)| {
            let present: Option<(String, String)> = conn
                .query_row(
                    "SELECT branches.id, json_extract(versions.content, '$.title') \
                     FROM branches JOIN versions ON versions.id = branches.head_version_id \
                     WHERE branches.kitchen_id = ?1 AND branches.lineage_id = ?2 \
                     ORDER BY branches.created_at, branches.id LIMIT 1",
                    params![kitchen_id, related_lineage_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read related Recipe: {e}")))?;
            let (branch_id, title) = match present {
                Some((branch_id, title)) => (Some(branch_id), title),
                None => (None, remembered_title),
            };
            Ok(json!({
                "lineage_id": related_lineage_id,
                "branch_id": branch_id,
                "title": title,
            }))
        })
        .collect()
}

/// Hash a Secret for storage/lookup. The Secret itself is never stored.
pub fn hash_secret(secret: &str) -> String {
    hex::encode(Sha256::digest(secret.as_bytes()))
}

/// 256 bits of randomness — every Secret in Kamosu is this size (ADR 0031).
pub fn generate_secret() -> String {
    hex::encode(random_bytes(32))
}

fn random_bytes(n: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; n];
    SysRng
        .try_fill_bytes(&mut bytes)
        .expect("the operating system random source is unavailable");
    bytes
}

/// Passwords are short human-chosen secrets, not Secrets: Argon2id salts and
/// hashes them separately at the cost ADR 0031 names (20 MiB, roughly a tenth s).
fn hash_password(password: &str) -> Result<String, OpError> {
    let password = required_text(password, "password")?;
    let params = Params::new(20 * 1024, 2, 1, None)
        .map_err(|e| OpError::internal(format!("cannot configure password hashing: {e}")))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let salt = SaltString::encode_b64(&random_bytes(16))
        .map_err(|e| OpError::internal(format!("cannot generate password salt: {e}")))?;
    argon
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| OpError::internal(format!("cannot hash password: {e}")))
}

fn required_text<'a>(value: &'a str, field: &str) -> Result<&'a str, OpError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(OpError::bad_request(format!("{field} is required")));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::folded_word;

    /// The fold is what holds one word to one Tag, so it is checked directly
    /// rather than only through the Operations that lean on it.
    #[test]
    fn one_word_folds_to_one_form_however_it_was_typed() {
        // Case, ASCII and beyond it.
        assert_eq!(folded_word("Dessert"), folded_word("dessert"));
        assert_eq!(folded_word("ÉTÉ"), folded_word("été"));
        assert_eq!(folded_word("RÁPIDO"), folded_word("rápido"));

        // Shape: one character, or a letter and a combining accent.
        assert_eq!(folded_word("crème"), folded_word("cre\u{0300}me"));
        assert_eq!(folded_word("Crème"), folded_word("CRE\u{0300}ME"));

        // And words that really are different stay different.
        assert_ne!(folded_word("dessert"), folded_word("desert"));
        assert_ne!(folded_word("été"), folded_word("ete"));
    }

    /// The "two hits make a third Food" half of ADR 0022 only ever fires for
    /// an arriving Food known by more than one name — today, exclusively a
    /// Bundle import from another instance, a ticket not yet built. No
    /// Operation in this Catalogue can drive it, so it is checked directly
    /// against the matcher rather than through `set_reading` or a future
    /// Bundle-import Operation.
    #[test]
    fn resolve_food_for_names_makes_a_third_food_on_collision() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::Db::open(dir.path()).unwrap();
        db.with_conn(|conn| {
            let food_a = super::create_food(conn, &[("fr", "farine")]).unwrap();
            let food_b = super::create_food(conn, &[("en", "flour")]).unwrap();

            // An arriving Food naming both farine (fr) and flour (en) hits
            // both existing Foods by two different words — doubt makes a
            // third carrying both rather than welding food_a and food_b
            // together.
            let food_c =
                super::resolve_food_for_names(conn, &[("fr", "farine"), ("en", "flour")], None)
                    .unwrap();
            assert_ne!(food_c, food_a, "nothing is merged");
            assert_ne!(food_c, food_b, "nothing is merged");

            let names_of = |food_id: &str| -> Vec<(String, String)> {
                conn.prepare("SELECT language, name FROM food_names WHERE food_id = ?1")
                    .unwrap()
                    .query_map(rusqlite::params![food_id], |row| {
                        Ok((row.get(0)?, row.get(1)?))
                    })
                    .unwrap()
                    .collect::<Result<_, _>>()
                    .unwrap()
            };

            assert_eq!(
                names_of(&food_a),
                vec![("fr".to_string(), "farine".to_string())],
                "the original Food is untouched — nothing destroyed"
            );
            assert_eq!(
                names_of(&food_b),
                vec![("en".to_string(), "flour".to_string())],
                "the original Food is untouched — nothing destroyed"
            );
            let mut carried = names_of(&food_c);
            carried.sort();
            assert_eq!(
                carried,
                vec![
                    ("en".to_string(), "flour".to_string()),
                    ("fr".to_string(), "farine".to_string()),
                ],
                "the new Food carries every arriving name"
            );

            // A lone word matching the same two Foods, by contrast, never
            // makes a fourth Food — it goes to the busiest one instead.
            let resolved = super::resolve_food_for_word(conn, "fr", "farine", None).unwrap();
            assert!(
                resolved == food_a || resolved == food_c,
                "a lone ambiguous word resolves to an existing Food, never a new one"
            );

            Ok(())
        })
        .unwrap();
    }
}
