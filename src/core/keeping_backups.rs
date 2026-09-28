//! Backups: taking one, listing them, and the hourly tick that keeps three at
//! three distances (ADR 0039).

use super::*;

impl Core {
    /// How often Kamosu looks at whether a Backup is owed.
    ///
    /// Hourly, which is not the same as backing up hourly: the three slots are
    /// due on their own cadences (ADR 0039) and the wake-up finds nothing owed
    /// twenty-three times out of twenty-four. It is a short tick rather than a
    /// daily one so that a server restarted every evening still gets a Backup
    /// — a daily timer that resets at every boot is a daily timer that never
    /// fires.
    const BACKUP_TICK: std::time::Duration = std::time::Duration::from_secs(60 * 60);

    // --- Backups (#78, ADR 0039) --------------------------------------------

    /// Take a Backup now: one archive holding a consistent database snapshot
    /// and every Photograph, written under `/data` beside the database.
    ///
    /// The same work the hourly tick does, asked for by name — and the daily
    /// archive is taken whether or not it was due, because somebody asking for
    /// a Backup is usually about to do something to the server.
    ///
    /// A **Job**, because a Backup is the size of the library: on an instance
    /// with years of cooking in it this copies gigabytes, which is what
    /// `Kind::Job` means (`catalogue::Kind`). It also puts the work on the
    /// blocking pool rather than the runtime the Doors answer on.
    pub fn take_backup(&self, job: Option<&JobProgress>) -> Result<Value, OpError> {
        if let Some(job) = job {
            job.report(0, Some(1), "copying the database and the Photographs");
        }
        backups::run(&self.data_dir(), backups::Ask::Now)
    }

    /// Every Backup this instance holds, newest first. Recomputed from the
    /// directory each time rather than from a table: the files are the truth
    /// about what can be restored, and a tally could disagree with them.
    pub fn list_backups(&self) -> Result<Value, OpError> {
        let held = backups::list(&self.data_dir())?;
        Ok(json!({ "backups": backups::as_json(&held) }))
    }

    /// Where one named Backup sits, for the out-of-band route to send. An
    /// archive is far too large to wrap in a JSON envelope, so it travels as
    /// its own bytes exactly as a Photograph does (ADR 0001).
    pub fn backup_path(&self, name: &str) -> Result<std::path::PathBuf, OpError> {
        let path = backups::archive_path(&self.data_dir(), name)?;
        if !path.is_file() {
            return Err(OpError::not_found("no such Backup"));
        }
        Ok(path)
    }
}

/// Keep the three Backups current for as long as the instance serves (#78).
///
/// Deliberately not a Job, for the same reason the orphan sweep is not one:
/// nobody asked for this and nobody is watching it. A failed run is logged and
/// forgotten, and the next wake-up recomputes what is owed from the files
/// themselves, so there is no state a failure can corrupt.
///
/// The first wake-up is one tick away rather than immediate. Backing up at
/// startup would put a copy of the whole library between an Operator restarting
/// the instance and it answering, and would take a fresh archive after every
/// restart — which is how three slots become one moment repeated three times.
pub(super) fn spawn_backups(core: Arc<Core>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval_at(
            tokio::time::Instant::now() + Core::BACKUP_TICK,
            Core::BACKUP_TICK,
        );
        loop {
            ticker.tick().await;
            let core = core.clone();
            // Blocking SQLite and a great many file bytes: never on the
            // runtime the Doors answer on.
            let taken = tokio::task::spawn_blocking(move || {
                crate::backups::run(&core.data_dir(), crate::backups::Ask::WhenDue)
            })
            .await;
            match taken {
                Ok(Ok(report)) => {
                    let taken = report["taken"].as_array().map(Vec::len).unwrap_or(0);
                    if taken > 0 {
                        tracing::info!(target: "kamosu::backups", report = %report, "Backup taken");
                    }
                }
                Ok(Err(error)) => {
                    tracing::warn!(target: "kamosu::backups", %error, "Backup failed");
                }
                Err(error) => {
                    tracing::warn!(target: "kamosu::backups", %error, "Backup panicked");
                }
            }
        }
    });
}
