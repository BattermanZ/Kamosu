//! The SQLite file under `/data` — the truth (ADR 0003). WAL mode is a
//! requirement, not a detail: a second process (the recovery command) opens the
//! same file while the server runs.
//!
//! The schema moves forward only (ADR 0030): each change is one numbered
//! [`Migration`], applied in order at startup behind a **Snapshot** of the
//! database taken just before. There is no down-migration and no tool for one —
//! going back is putting the Snapshot file back and running the old image.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{Connection, OptionalExtension};

use crate::OpError;

pub const DATABASE_FILE: &str = "kamosu.db";

/// One numbered step on the only path the schema walks: forward (ADR 0030).
/// Written carefully once — there is no inverse to get wrong.
#[derive(Debug, Clone, Copy)]
pub struct Migration {
    /// Where this step leaves the schema. Steps apply strictly in order.
    pub version: i64,
    /// What the step does, named for the log and the failure message.
    pub description: &'static str,
    /// The step itself. Runs inside one transaction, together with the
    /// `schema_version` stamp, so a failure leaves nothing half-done.
    pub sql: &'static str,
}

/// Every migration, oldest first. Appending a new tail entry *is* the upgrade;
/// nothing earlier is ever rewritten.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        description: "the base schema: People and their Access Keys",
        sql: r#"
        CREATE TABLE IF NOT EXISTS people (
            id         TEXT PRIMARY KEY,
            name       TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );

        -- An Access Key: the Secret a Person mints for an agent to act with.
        -- 256 bits of randomness, stored hashed, shown once at minting, revocable,
        -- ending only when spent or revoked — never on a clock (ADR 0031).
        CREATE TABLE IF NOT EXISTS access_keys (
            secret_hash TEXT PRIMARY KEY,
            person_id   TEXT NOT NULL REFERENCES people(id),
            name        TEXT NOT NULL,
            read_only   INTEGER NOT NULL DEFAULT 0,
            revoked     INTEGER NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            last_used_at TEXT
        );
        "#,
    },
    Migration {
        version: 2,
        description: "the Job shape: slow work answers an id at once",
        sql: r#"
        -- A Job: slow work asked for through an Operation. The row is the truth
        -- about the work — its state, its progress, and its result or the reason
        -- it failed — so it survives the request that started it and is read back
        -- through ordinary Operations at both Doors. A Job ends only in a terminal
        -- status; it never vanishes.
        CREATE TABLE IF NOT EXISTS jobs (
            id                TEXT PRIMARY KEY,
            person_id         TEXT REFERENCES people(id),
            read_only         INTEGER NOT NULL DEFAULT 0,
            operation         TEXT NOT NULL,
            input             TEXT NOT NULL,
            status            TEXT NOT NULL DEFAULT 'queued',
            progress_done     INTEGER,
            progress_total    INTEGER,
            progress_message  TEXT,
            result            TEXT,
            error             TEXT,
            error_code        INTEGER,
            created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            updated_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );
        "#,
    },
    Migration {
        version: 3,
        description: "the first Person, their Home Kitchen, and account preferences",
        sql: r#"
        ALTER TABLE people ADD COLUMN password_hash TEXT;
        ALTER TABLE people ADD COLUMN home_kitchen_id TEXT;
        ALTER TABLE people ADD COLUMN reading_language TEXT NOT NULL DEFAULT 'en';
        ALTER TABLE people ADD COLUMN reading_measures TEXT NOT NULL DEFAULT 'us';
        ALTER TABLE people ADD COLUMN is_operator INTEGER NOT NULL DEFAULT 0;
        CREATE TABLE IF NOT EXISTS kitchens (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            hand_id     TEXT NOT NULL UNIQUE,
            created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );
        CREATE TABLE IF NOT EXISTS kitchen_members (
            kitchen_id TEXT NOT NULL REFERENCES kitchens(id),
            person_id  TEXT NOT NULL REFERENCES people(id),
            PRIMARY KEY (kitchen_id, person_id)
        );
        CREATE TABLE IF NOT EXISTS instance_setup (
            singleton          INTEGER PRIMARY KEY CHECK (singleton = 1),
            operator_person_id TEXT REFERENCES people(id)
        );
        CREATE TABLE IF NOT EXISTS sessions (
            id           TEXT PRIMARY KEY,
            secret_hash  TEXT NOT NULL UNIQUE,
            person_id    TEXT NOT NULL REFERENCES people(id),
            name         TEXT NOT NULL,
            revoked      INTEGER NOT NULL DEFAULT 0,
            created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            last_used_at TEXT
        );
        CREATE TABLE IF NOT EXISTS login_failures (
            name             TEXT PRIMARY KEY,
            consecutive_failures INTEGER NOT NULL DEFAULT 0
        );
        "#,
    },
    Migration {
        version: 4,
        description: "a login name names exactly one password-bearing Person",
        sql: r#"
        CREATE UNIQUE INDEX people_login_names_unique
            ON people(name) WHERE password_hash IS NOT NULL;
        "#,
    },
    Migration {
        version: 5,
        description: "an Access Key is known by an id, not its secret hash",
        sql: r#"
        ALTER TABLE access_keys ADD COLUMN id TEXT;
        UPDATE access_keys SET id = 'ak_' || lower(hex(randomblob(8))) WHERE id IS NULL;
        CREATE UNIQUE INDEX access_keys_id_unique ON access_keys(id);
        "#,
    },
    Migration {
        version: 6,
        description: "a Job remembers whether an Access Key asked for it",
        sql: r#"
        ALTER TABLE jobs ADD COLUMN via_access_key INTEGER NOT NULL DEFAULT 0;
        "#,
    },
    Migration {
        version: 7,
        description: "Kitchen nicknames and Kitchen invites",
        sql: r#"
        -- A Nickname: one member's own private relabelling of a Kitchen, seen
        -- by nobody else and never travelling (ADR 0007).
        ALTER TABLE kitchen_members ADD COLUMN nickname TEXT;

        -- A Kitchen Invite: a one-use link a member mints, spent the moment
        -- another Person opens it and joins (CONTEXT.md, "Invite").
        CREATE TABLE IF NOT EXISTS kitchen_invites (
            id          TEXT PRIMARY KEY,
            secret_hash TEXT NOT NULL UNIQUE,
            kitchen_id  TEXT NOT NULL REFERENCES kitchens(id),
            created_by  TEXT NOT NULL REFERENCES people(id),
            created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            used_at     TEXT,
            used_by     TEXT REFERENCES people(id)
        );
        "#,
    },
    Migration {
        version: 8,
        description: "account links and account states",
        sql: r#"
        ALTER TABLE people ADD COLUMN disabled INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE people ADD COLUMN deleted INTEGER NOT NULL DEFAULT 0;
        CREATE TABLE account_links (
            secret_hash TEXT PRIMARY KEY,
            kind TEXT NOT NULL CHECK (kind IN ('invite', 'recovery')),
            person_id TEXT REFERENCES people(id),
            is_operator INTEGER NOT NULL DEFAULT 0,
            spent INTEGER NOT NULL DEFAULT 0,
            revoked INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );
        "#,
    },
];

/// The newest step [`MIGRATIONS`] carries: what this binary understands.
pub const LATEST_SCHEMA_VERSION: i64 = MIGRATIONS[MIGRATIONS.len() - 1].version;

/// The database handle shared by the Core.
pub struct Db {
    conn: Mutex<Connection>,
    data_dir: std::path::PathBuf,
}

impl Db {
    /// Open (creating if needed) the database inside `data_dir`, turn on WAL and
    /// the pragmas Kamosu depends on, and bring the schema forward along the
    /// shipped [`MIGRATIONS`].
    pub fn open(data_dir: &Path) -> Result<Db, OpError> {
        Self::open_with_migrations(data_dir, MIGRATIONS)
    }

    /// The machinery of [`Self::open`] with the migration steps chosen by the
    /// caller: tests build databases at an *old* schema by truncating the list,
    /// and force failures by appending a broken step. Startup always uses the
    /// full shipped list.
    pub fn open_with_migrations(data_dir: &Path, migrations: &[Migration]) -> Result<Db, OpError> {
        std::fs::create_dir_all(data_dir).map_err(|e| {
            OpError::internal(format!(
                "cannot create data directory {}: {e}",
                data_dir.display()
            ))
        })?;
        let conn = Connection::open(data_dir.join(DATABASE_FILE))
            .map_err(|e| OpError::internal(format!("cannot open database: {e}")))?;
        Self::initialise(conn, data_dir, migrations)
    }

    pub fn initialise(
        conn: Connection,
        data_dir: &Path,
        migrations: &[Migration],
    ) -> Result<Db, OpError> {
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| OpError::internal(format!("WAL mode refused: {e}")))?;
        // The documented pairing for WAL: safe and fast.
        conn.pragma_update(None, "synchronous", "NORMAL")
            .map_err(|e| OpError::internal(format!("synchronous refused: {e}")))?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| OpError::internal(format!("foreign_keys refused: {e}")))?;
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| OpError::internal(format!("busy timeout refused: {e}")))?;
        let db = Db {
            conn: Mutex::new(conn),
            data_dir: data_dir.to_path_buf(),
        };
        db.with_conn(|conn| migrate(conn, data_dir, migrations))?;
        Ok(db)
    }

    /// Run one closure with the connection. The database is behind a mutex, so
    /// every access serialises; that is plenty for v1 and honest about it.
    pub fn with_conn<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, OpError>,
    ) -> Result<T, OpError> {
        let conn = self.conn.lock().expect("database lock poisoned");
        f(&conn)
    }

    /// Where Kamosu's truth lives: one SQLite file under the one data directory.
    pub fn database_path(&self) -> std::path::PathBuf {
        self.data_dir.join(DATABASE_FILE)
    }
}

fn latest_step(migrations: &[Migration]) -> i64 {
    migrations.last().map(|m| m.version).unwrap_or(0)
}

fn stored_schema_version(conn: &Connection) -> Result<i64, OpError> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read schema version: {e}")))?;
    raw.map(|v| v.parse().map_err(|_| ()))
        .unwrap_or(Ok(0))
        .map_err(|_| {
            OpError::internal(
                "the schema_version recorded in meta is not a number; \
                 this file may not be a Kamosu database",
            )
        })
}

fn migrate(conn: &Connection, data_dir: &Path, migrations: &[Migration]) -> Result<(), OpError> {
    // The ledger itself: infrastructure beneath every step, never migrated.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    )
    .map_err(|e| OpError::internal(format!("cannot prepare the schema ledger: {e}")))?;

    let current = stored_schema_version(conn)?;
    let latest = latest_step(migrations);

    // An older binary against a newer database stops here, loudly. Running old
    // code over a new cookbook is how recipes get quietly mangled (ADR 0030).
    if current > latest {
        return Err(OpError::internal(format!(
            "this database stands at schema version {current}, which is NEWER \
             than what this Kamosu understands (up to version {latest}). \
             An upgrade went past this binary. Start the newer Kamosu again — \
             do not run this older image against it"
        )));
    }

    let pending: Vec<&Migration> = migrations.iter().filter(|m| m.version > current).collect();
    if pending.is_empty() {
        return Ok(());
    }

    // Before any migration runs, copy the database aside. It is the database
    // alone because a Photograph cannot change once it exists (ADR 0017) — the
    // file alone is a complete way back.
    let snapshot = write_snapshot(conn, data_dir, current, latest)?;

    for step in pending {
        // Schema change and version stamp commit together or not at all: SQLite
        // rolls them back as one, so a failed step leaves exactly what was there.
        let batch = format!(
            "BEGIN IMMEDIATE;\n{}\nINSERT INTO meta(key, value) VALUES ('schema_version', '{}') \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value;\nCOMMIT;",
            step.sql, step.version
        );
        if let Err(e) = conn.execute_batch(&batch) {
            return Err(OpError::internal(format!(
                "migration {} ({}) failed: {}. Kamosu refuses to serve \
                 half-migrated; nothing was changed. To go back, stop Kamosu and \
                 put the Snapshot back over {} (and remove any -wal/-shm \
                 neighbours of it first) — copy the Snapshot from {}",
                step.version,
                step.description,
                e,
                data_dir.join(DATABASE_FILE).display(),
                snapshot.display()
            )));
        }
    }
    tracing::info!(
        from = current,
        to = latest,
        snapshot = %snapshot.display(),
        "schema migrated forward; Snapshot kept beside the database"
    );
    Ok(())
}

/// Copy the database file aside under `/data`, naming the span of versions it
/// undoes. WAL is checkpointed first so the one file is complete on its own.
fn write_snapshot(
    conn: &Connection,
    data_dir: &Path,
    from: i64,
    to: i64,
) -> Result<PathBuf, OpError> {
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|e| OpError::internal(format!("cannot checkpoint before snapshot: {e}")))?;
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let dest = data_dir.join(format!("kamosu-snapshot-v{from}-to-v{to}-{seconds}.db"));
    std::fs::copy(data_dir.join(DATABASE_FILE), &dest).map_err(|e| {
        OpError::internal(format!(
            "cannot write the pre-migration Snapshot to {}: {e}",
            dest.display()
        ))
    })?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wal_mode_sticks() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(dir.path()).unwrap();
        let mode: String = db
            .with_conn(|c| {
                Ok(c.query_row("PRAGMA journal_mode", [], |r| r.get(0))
                    .unwrap())
            })
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal");
    }
}
