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
use rusqlite::OptionalExtension;
use rusqlite::params;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::catalogue::{self, Kind, Permission};
use crate::db::Db;
use crate::jobs::{self, JobProgress, JobRecord};

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
    fn lookup_credential(&self, hash: &str) -> Result<Option<(String, bool, bool, bool)>, OpError> {
        self.db().with_conn(|conn| {
            conn.query_row(
                "SELECT access_keys.person_id, access_keys.read_only, 0, people.is_operator
                   FROM access_keys JOIN people ON people.id = access_keys.person_id
                   WHERE access_keys.secret_hash = ?1 AND access_keys.revoked = 0 AND people.disabled = 0 AND people.deleted = 0
                 UNION ALL
                 SELECT sessions.person_id, 0, 1, people.is_operator
                   FROM sessions JOIN people ON people.id = sessions.person_id
                   WHERE sessions.secret_hash = ?1 AND sessions.revoked = 0 AND people.disabled = 0 AND people.deleted = 0
                 LIMIT 1",
                params![hash],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
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
            Some((person_id, read_only, is_session, is_operator)) => {
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
            .map(|(_, read_only, _, _)| read_only)
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
}

/// Create a Kitchen and seat its first member in one place — the shape a
/// Person's Home Kitchen and any Kitchen they later create both share.
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
