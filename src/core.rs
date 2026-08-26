//! The Core: where Operations are carried out and where permission is checked.
//! It is reached only through a Door (or the terminal) and knows nothing about
//! how a request arrived.
//!
//! Authorisation lives here, beneath both Doors, keyed on a Credential. A
//! permission check written inside a Door is a bug.

use std::sync::Arc;

use rusqlite::OptionalExtension;
use rusqlite::params;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::catalogue::{self, Permission};
use crate::db::Db;

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
    /// Nothing refuses it yet — there are no write Operations to refuse — but
    /// the first one must consult this or the flag is decoration.
    pub read_only: bool,
}

/// The Core. Holds the database — the truth (ADR 0003).
pub struct Core {
    db: Arc<Db>,
}

impl Core {
    pub fn open(db: Arc<Db>) -> Core {
        Core { db }
    }

    pub fn db(&self) -> &Db {
        &self.db
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
        }

        (op.handler)(self, &caller, input)
    }

    /// Resolve a raw Secret to the Person it acts as: an Access Key today; a login
    /// Session joins when accounts do. Stored hashed, never in the clear.
    fn resolve_credential(&self, secret: &str) -> Result<Caller, OpError> {
        let hash = hash_secret(secret);
        let key: Option<(String, bool)> = self.db().with_conn(|conn| {
            conn.query_row(
                "SELECT person_id, read_only FROM access_keys WHERE secret_hash = ?1 AND revoked = 0",
                params![hash],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| OpError::internal(format!("cannot read access keys: {e}")))
        })?;

        match key {
            Some((person_id, read_only)) => {
                let _ = self.db().with_conn(|conn| {
                    conn.execute(
                        "UPDATE access_keys SET last_used_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
                         WHERE secret_hash = ?1",
                        params![hash],
                    )
                    .map_err(|e| OpError::internal(format!("cannot record key use: {e}")))?;
                    Ok(())
                });
                Ok(Caller {
                    person_id,
                    read_only,
                })
            }
            // Unknown or already-revoked: the same answer either way, saying nothing
            // about which.
            None => Err(OpError::unauthorized(
                "this Credential does not name anyone",
            )),
        }
    }

    /// Mint an Access Key for a Person. Plumbing beneath both Doors and the
    /// terminal — obtaining a Credential is not an Operation. The raw Secret is
    /// returned once and stored only as its hash.
    pub fn mint_access_key(
        &self,
        person_id: &str,
        key_name: &str,
        read_only: bool,
    ) -> Result<String, OpError> {
        let secret = generate_secret();
        let hash = hash_secret(&secret);
        self.db().with_conn(|conn| {
            let exists: bool = conn
                .query_row("SELECT COUNT(*) FROM people WHERE id = ?1", params![person_id], |r| r.get(0))
                .map(|n: i64| n > 0)
                .map_err(|e| OpError::internal(e.to_string()))?;
            if !exists {
                return Err(OpError::bad_request(format!("no Person '{person_id}'")));
            }
            conn.execute(
                "INSERT INTO access_keys (secret_hash, person_id, name, read_only) VALUES (?1, ?2, ?3, ?4)",
                params![hash, person_id, key_name, read_only as i64],
            )
            .map_err(|e| OpError::internal(format!("cannot mint access key: {e}")))?;
            Ok(())
        })?;
        Ok(secret)
    }

    /// Create a Person. Onboarding proper arrives with a later ticket; this exists
    /// so the skeleton has real People behind its Credentials rather than fixtures.
    pub fn create_person(&self, name: &str) -> Result<String, OpError> {
        let id = format!("p_{}", hex::encode(random_bytes(8)));
        self.db().with_conn(|conn| {
            conn.execute(
                "INSERT INTO people (id, name) VALUES (?1, ?2)",
                params![id, name],
            )
            .map_err(|e| OpError::internal(format!("cannot create Person: {e}")))?;
            Ok(())
        })?;
        Ok(id)
    }

    /// Whether setup has happened on this instance. Nothing sets it yet — the
    /// first-visitor-wins flow is a later ticket — so a fresh install says false.
    pub fn setup_complete(&self) -> Result<bool, OpError> {
        self.db().with_conn(|conn| {
            let value: Option<String> = conn
                .query_row(
                    "SELECT value FROM meta WHERE key = 'setup_complete'",
                    [],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read meta: {e}")))?;
            Ok(value.as_deref() == Some("true"))
        })
    }

    pub fn mark_setup_complete(&self) -> Result<(), OpError> {
        self.db().with_conn(|conn| {
            conn.execute(
                "INSERT INTO meta(key, value) VALUES ('setup_complete', 'true')
                 ON CONFLICT(key) DO UPDATE SET value = 'true'",
                [],
            )
            .map_err(|e| OpError::internal(format!("cannot record setup flag: {e}")))?;
            Ok(())
        })
    }
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
    rand::fill(&mut bytes);
    bytes
}
