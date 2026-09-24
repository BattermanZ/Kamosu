//! The Core: where Operations are carried out and where permission is checked.
//! It is reached only through a Door (or the terminal) and knows nothing about
//! how a request arrived.
//!
//! Authorisation lives here, beneath both Doors, keyed on a Credential. A
//! permission check written inside a Door is a bug.
//!
//! This file holds what every caller passes through before any Operation runs:
//! the `Core` itself, startup, `execute` with its permission check, and
//! resolving a Credential. The work behind each Operation lives beside it, one
//! file per area, each adding its methods to `Core` in an `impl` of its own.
//!
//! Three areas are named for what they do rather than what they hold —
//! `keeping_backups`, `keeping_photographs`, `shopping_list` — because
//! `crate::backups`, `crate::photographs` and `crate::shopping` already exist
//! and the code in those files names them by path.
//!
//! Every area's items are gathered back into this module, so the areas reach
//! one another's helpers through `use super::*` and every `crate::core::X`
//! path still works. How widely each area is re-exported follows what it
//! holds: `pub use` where it has something public, `pub(crate) use` where the
//! widest is crate-wide, and a plain `use` where everything in it is for the
//! other areas only. An area nobody outside it calls into has no line at all.

use std::collections::{HashMap, HashSet};
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

use crate::backups;
use crate::bundles;
use crate::catalogue::{self, JobLane, Kind, Permission};
use crate::crouton;
use crate::db::Db;
use crate::fingerprint::{canonical_json, stored_version};
use crate::jobs::{self, JobProgress, JobRecord};
use crate::language::LANGUAGES;
use crate::photographs;
use crate::sheet;
use crate::shopping;
use crate::units;

mod accounts;
mod cookbooks;
mod cooking;
mod imports;
mod keeping_backups;
mod keeping_photographs;
mod kitchens;
mod readings;
mod recipes;
mod related;
mod search;
mod shared;
mod sharing;
mod shelves;
mod shopping_list;
mod tags;
mod translation;

pub use accounts::*;
use cookbooks::*;
pub use cooking::*;
pub use imports::*;
use keeping_backups::*;
use keeping_photographs::*;
use kitchens::*;
pub use readings::*;
use recipes::*;
pub use related::*;
pub use search::*;
pub(crate) use shared::*;
pub use sharing::*;
use shelves::*;
use tags::*;
use translation::*;

/// What went wrong with an Operation, in words a caller can act on. The Doors map
/// these to their own transports; they never decide them.
#[derive(Debug, Clone)]
pub struct OpError {
    pub kind: ErrorKind,
    pub message: String,
    /// True for exactly one refusal: the Secret presented resolves to nobody,
    /// because it is unknown, revoked or spent. Every other `Unauthorized` is
    /// about what a Person Kamosu *did* recognise may do, and so says nothing
    /// about the Credential itself.
    ///
    /// A Door may act on that distinction — the web door takes back a Session
    /// cookie it set (#91) — but the distinction is drawn here, once, as every
    /// authorisation decision is.
    pub credential_names_nobody: bool,
    /// How many seconds to wait before asking again, on the one refusal that
    /// knows: a sign-in tried while its name is still waiting (#138). The web
    /// door carries it as `retry_after_seconds` and a `Retry-After` header, and
    /// the sign-in screen counts it down.
    pub retry_after_seconds: Option<u64>,
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
    /// Every refusal but one: it says what went wrong, and nothing about the
    /// Credential that was presented.
    fn of(kind: ErrorKind, message: impl Into<String>) -> Self {
        OpError {
            kind,
            message: message.into(),
            credential_names_nobody: false,
            retry_after_seconds: None,
        }
    }
    pub fn unauthorized(message: impl Into<String>) -> Self {
        OpError::of(ErrorKind::Unauthorized, message)
    }
    /// The one refusal that is about the Secret itself rather than about what
    /// its Person may do: it resolves to nobody. Unknown, revoked and spent all
    /// answer alike, saying nothing about which.
    pub fn credential_names_nobody() -> Self {
        OpError {
            credential_names_nobody: true,
            ..OpError::unauthorized("this Credential does not name anyone")
        }
    }
    pub fn unknown_operation(name: &str) -> Self {
        OpError::of(
            ErrorKind::UnknownOperation,
            format!("no Operation named '{name}' exists in the Catalogue"),
        )
    }
    pub fn bad_request(message: impl Into<String>) -> Self {
        OpError::of(ErrorKind::BadRequest, message)
    }
    pub fn not_found(message: impl Into<String>) -> Self {
        OpError::of(ErrorKind::NotFound, message)
    }
    pub fn busy() -> Self {
        OpError::of(
            ErrorKind::Busy,
            "Kamosu is busy right now — try again in a moment",
        )
    }
    /// Busy for a known while: this name's sign-in is waiting out a wrong
    /// password, or another try at it is being checked (#138).
    pub fn wait(seconds: u64, message: impl Into<String>) -> Self {
        OpError {
            retry_after_seconds: Some(seconds),
            ..OpError::of(ErrorKind::Busy, message)
        }
    }
    pub fn internal(message: impl Into<String>) -> Self {
        OpError::of(ErrorKind::Internal, message)
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
    /// The login Session that resolved this Credential, if any — never set
    /// for an Access Key. What lets the Sessions list say which one is in
    /// your hand (#114).
    pub session_id: Option<String>,
}

/// One asking of an Operation: who is acting, and (when the Operation runs as a
/// Job) the handle through which the running work reports its progress.
#[derive(Clone)]
struct Session {
    id: String,
    secret: String,
}

/// One row `lookup_credential` can find: `(person_id, read_only, is_session,
/// is_operator, access_key_id, session_id)`.
type CredentialLookup = (String, bool, bool, bool, Option<String>, Option<String>);

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
    /// Meaning Search, which on most instances is an empty slot and stays one
    /// (ADR 0029). Nothing branches on that outside the two places that must:
    /// the search itself, and the Operations that turn it on.
    meaning: Arc<crate::meaning::MeaningSearch>,
    /// Whose login is waiting, and the few threads that may hash or check a
    /// password at once (#138).
    passwords: accounts::Passwords,
}

impl Core {
    pub fn open(db: Arc<Db>) -> Core {
        // Terminal commands open a Core without Job workers; they never ask for
        // slow work. A Job asked for here would be refused as busy.
        let (lanes, _) = jobs::lanes();
        let meaning = Arc::new(crate::meaning::MeaningSearch::new(
            db.data_dir().to_path_buf(),
        ));
        Core {
            db,
            lanes,
            meaning,
            passwords: accounts::Passwords::default(),
        }
    }

    /// Open the database *and* start carrying Jobs: both Doors share this entry.
    /// The lanes exist from the moment the instance serves — and anything a
    /// previous process left behind is recovered before the first answer goes out.
    pub fn start(db: Arc<Db>) -> Arc<Core> {
        let (lanes, receivers) = jobs::lanes();
        jobs::recover_at_startup(&db, &lanes);
        let meaning = Arc::new(crate::meaning::MeaningSearch::new(
            db.data_dir().to_path_buf(),
        ));
        let core = Core {
            db,
            lanes,
            meaning,
            passwords: accounts::Passwords::default(),
        };
        core.passwords.prepare();
        let core = Arc::new(core);
        jobs::spawn_workers(core.clone(), receivers);
        spawn_orphan_sweep(core.clone());
        spawn_meaning_search(core.clone());
        spawn_backups(core.clone());
        core
    }

    /// Meaning Search as this instance holds it.
    pub fn meaning(&self) -> Arc<crate::meaning::MeaningSearch> {
        self.meaning.clone()
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

        // Shape is checked here, once, against the Operation's own declaration
        // (#85): both Doors inherit it, and a new Operation cannot forget it any
        // more than it can forget to appear at both. It runs after the
        // authorisation checks above so a caller who may not perform an
        // Operation learns nothing about its input, and before any handler, so
        // no handler is ever reached with an envelope the Catalogue does not
        // describe. What it does *not* check is meaning — whether a Branch
        // exists, whether an amount parses — which stays in the handlers.
        crate::schema::validate_input(op.name, &input)?;

        let invocation = Invocation { caller, job: None };

        match op.kind {
            Kind::Immediate => (op.handler)(self, &invocation, input),
            Kind::Job => self.ask_job(op.name, op.job_lane, &invocation, input),
        }
    }

    /// Accept slow work and answer at once with an id. The work itself runs in a
    /// lane: members never wait behind strangers (ADR 0032), except a Job whose
    /// own Catalogue entry declares `JobLane::AlwaysSingle` — the risk there is
    /// the outbound fetch itself, not who asked for it, so it never scales past
    /// one in flight regardless of who is signed in. A full line refuses rather
    /// than grows — the caller is told busy, try again in a moment.
    fn ask_job(
        &self,
        operation_name: &str,
        job_lane: JobLane,
        invocation: &Invocation,
        input: Value,
    ) -> Result<Value, OpError> {
        let job_id = jobs::record(self, operation_name, invocation, input)?;
        let lane = match job_lane {
            // The single depth-one lane, reused rather than duplicated — it
            // is a stranger's lane only in the sense that it was built for
            // "no more than one at a time"; an `AlwaysSingle` Job lands here
            // even with a signed-in Person asking (ADR 0032's #70 exception).
            JobLane::AlwaysSingle => &self.lanes.stranger,
            JobLane::ByCaller if invocation.caller.is_some() => &self.lanes.member,
            JobLane::ByCaller => &self.lanes.stranger,
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
                "SELECT access_keys.person_id, access_keys.read_only, 0, people.is_operator, access_keys.id, NULL
                   FROM access_keys JOIN people ON people.id = access_keys.person_id
                   WHERE access_keys.secret_hash = ?1 AND access_keys.revoked = 0 AND people.disabled = 0 AND people.deleted = 0
                 UNION ALL
                 SELECT sessions.person_id, 0, 1, people.is_operator, NULL, sessions.id
                   FROM sessions JOIN people ON people.id = sessions.person_id
                   WHERE sessions.secret_hash = ?1 AND sessions.revoked = 0 AND people.disabled = 0 AND people.deleted = 0
                 LIMIT 1",
                params![hash],
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
            .map_err(|e| OpError::internal(format!("cannot resolve Credential: {e}")))
        })
    }

    /// Resolve a raw Secret to the Person it acts as: an Access Key today; a login
    /// Session joins when accounts do. Stored hashed, never in the clear.
    fn resolve_credential(&self, secret: &str) -> Result<Caller, OpError> {
        let hash = hash_secret(secret);
        match self.lookup_credential(&hash)? {
            Some((person_id, read_only, is_session, is_operator, access_key_id, session_id)) => {
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
                    session_id,
                })
            }
            // Unknown or already-revoked: the same answer either way, saying nothing
            // about which. Marked, so the web door can take back a Session
            // cookie it set that now names nobody (#91).
            None => Err(OpError::credential_names_nobody()),
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
            .map(|(_, read_only, _, _, _, _)| read_only)
            .unwrap_or(false)
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

    /// The same resolution, additionally requiring an Operator — the
    /// out-of-band equivalent of a Catalogue Operation declared
    /// `Permission::Operator`. Fetching a Backup is the one route that needs
    /// it: an archive holds every Person's recipes, Attempts and shopping
    /// lists, so carrying one off the machine is on the Operator's exact list
    /// (CONTEXT.md, "Operator") and nobody else's.
    pub fn authenticate_for_operator(&self, secret: Option<&str>) -> Result<Caller, OpError> {
        let caller = self.authenticate(secret)?;
        if !caller.is_operator {
            return Err(OpError::unauthorized(
                "this route requires a Credential naming an Operator",
            ));
        }
        Ok(caller)
    }
}
