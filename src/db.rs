//! The SQLite file under `/data` — the truth (ADR 0003). WAL mode is a
//! requirement, not a detail: a second process (the recovery command) opens the
//! same file while the server runs.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::OpError;

pub const DATABASE_FILE: &str = "kamosu.db";

/// The database handle shared by the Core.
pub struct Db {
    conn: Mutex<Connection>,
    data_dir: std::path::PathBuf,
}

impl Db {
    /// Open (creating if needed) the database inside `data_dir`, turn on WAL and
    /// the pragmas Kamosu depends on, and bring the schema forward. The path is
    /// forward-only (ADR 0030): version 1 is the floor, never rewritten.
    pub fn open(data_dir: &Path) -> Result<Db, OpError> {
        std::fs::create_dir_all(data_dir).map_err(|e| {
            OpError::internal(format!(
                "cannot create data directory {}: {e}",
                data_dir.display()
            ))
        })?;
        let conn = Connection::open(data_dir.join(DATABASE_FILE))
            .map_err(|e| OpError::internal(format!("cannot open database: {e}")))?;
        Self::initialise(conn, data_dir)
    }

    pub fn initialise(conn: Connection, data_dir: &Path) -> Result<Db, OpError> {
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
        db.with_conn(migrate)?;
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

fn migrate(conn: &Connection) -> Result<(), OpError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS meta (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

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
    )
    .map_err(|e| OpError::internal(format!("migration failed: {e}")))?;

    conn.execute(
        "INSERT OR IGNORE INTO meta(key, value) VALUES ('schema_version', '1')",
        [],
    )
    .map_err(|e| OpError::internal(format!("cannot record schema version: {e}")))?;

    Ok(())
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
