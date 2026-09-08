//! A **Backup**: one archive an instance can be restored from — a consistent
//! database snapshot together with the Photographs (#78, ADR 0039).
//!
//! Three archives are kept, on three cadences: one taken every day, one every
//! week, one every month. Each place holds exactly one file, and taking a new
//! one is what prunes the old one, so the directory settles at three.
//!
//! Restoring one is stopping Kamosu, deleting `kamosu.db` and its `-wal` and
//! `-shm` neighbours, and unzipping the archive over `/data`. Deleting the
//! neighbours is the part that matters: the database in an archive carries no
//! write-ahead log, and one left from the old install would replay over it.
//! [`crate::db`] says the same about putting a Snapshot back (ADR 0030).
//!
//! **Nothing here sends a Backup anywhere.** Carrying one off the machine is
//! the Operator's act, through the out-of-band fetch route; a test at the
//! bottom of this file fails the build if this module ever learns to speak to
//! the network.
//!
//! Two things are deliberately *not* in an archive:
//!
//! - **Display Copies and Covers**, which are rebuildable from the Photographs
//!   and would roughly double what an Operator has to store (ADR 0017).
//! - **The downloaded model**, which is 220 MB of somebody else's weights that
//!   `download_meaning_model` fetches again on demand (ADR 0029).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::Connection;
use serde_json::{Value, json};

use crate::core::OpError;
use crate::db::DATABASE_FILE;
use crate::photographs;

/// **Exactly one Backup is taken at a time**, the same rule Meaning Search's
/// index build holds itself to and for the same two reasons. Two runs in one
/// second would write each other's working file; two runs a minute apart would
/// each have read the directory before the other placed anything, so neither
/// could prune what the other left, and the ceiling of three would break.
static TAKING_ONE: Mutex<()> = Mutex::new(());

/// Whether a run takes a Backup only where one is owed, or takes the daily one
/// regardless. Written as a choice rather than a `bool` so the call site says
/// which it means.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    /// The hourly tick: take only what the calendar owes.
    WhenDue,
    /// An Operator asking by name: take the daily one whether it is owed or
    /// not, because somebody asking for a Backup is usually about to do
    /// something to the server.
    Now,
}

/// The working file a run builds into before it is placed. Never a Backup —
/// `parse_name` refuses the shape, so a half-written archive can never be
/// listed or fetched.
const WORKING_PREFIX: &str = ".taking-";

/// SQLite takes a time value and then modifiers; there is no modifier meaning
/// "leave it alone", so the present moment is written as a zero-sized step.
const THIS_MOMENT: &str = "+0 days";

/// Where Backups live under `/data` — a directory of their own, as every other
/// kind of file Kamosu keeps has, so listing them never means sieving `/data`.
pub fn backups_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("backups")
}

/// Which of the three places an archive holds, and therefore how long it holds
/// it before a fresh one takes it (ADR 0039).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// Yesterday: replaced every day.
    Daily,
    /// The past week: replaced every seventh day.
    Weekly,
    /// The past month: replaced every thirtieth day.
    Monthly,
}

impl Slot {
    /// Newest cadence first, which is also the order slots are filled in — the
    /// daily archive is written first and the others are copied from it.
    pub const ALL: [Slot; 3] = [Slot::Daily, Slot::Weekly, Slot::Monthly];

    pub fn as_str(self) -> &'static str {
        match self {
            Slot::Daily => "daily",
            Slot::Weekly => "weekly",
            Slot::Monthly => "monthly",
        }
    }

    fn parse(name: &str) -> Option<Self> {
        match name {
            "daily" => Some(Slot::Daily),
            "weekly" => Some(Slot::Weekly),
            "monthly" => Some(Slot::Monthly),
            _ => None,
        }
    }

    /// How old this place's archive is allowed to get, written as SQLite reads
    /// a date modifier — so the ages are measured on the same clock and in the
    /// same calendar as every other timestamp in Kamosu.
    fn age_allowed(self) -> &'static str {
        match self {
            Slot::Daily => "-1 day",
            Slot::Weekly => "-7 days",
            Slot::Monthly => "-30 days",
        }
    }
}

/// One archive on disk.
#[derive(Clone, Debug)]
pub struct Archive {
    /// The file's own name, and the only thing a caller needs to fetch it.
    pub name: String,
    pub slot: Slot,
    /// When it was taken, `YYYYMMDDTHHMMSSZ` — fixed width, so comparing two
    /// as plain strings compares them as moments.
    pub stamp: String,
    pub size_bytes: u64,
}

impl Archive {
    /// The same moment with its separators put back, which is how every other
    /// timestamp crosses a Door.
    pub fn taken_at(&self) -> String {
        let s = &self.stamp;
        format!(
            "{}-{}-{}T{}:{}:{}Z",
            &s[0..4],
            &s[4..6],
            &s[6..8],
            &s[9..11],
            &s[11..13],
            &s[13..15]
        )
    }

    pub fn as_json(&self) -> Value {
        json!({
            "name": self.name,
            "slot": self.slot.as_str(),
            "taken_at": self.taken_at(),
            "size_bytes": self.size_bytes,
        })
    }
}

/// Read one archive's name back into what it says about itself. Strict on
/// purpose: this is also what stands between a fetch and a path somebody
/// wrote by hand, so anything not of exactly this shape is not a Backup.
fn parse_name(name: &str) -> Option<(Slot, String)> {
    let rest = name.strip_prefix("kamosu-backup-")?.strip_suffix(".zip")?;
    let (slot, stamp) = rest.split_once('-')?;
    let slot = Slot::parse(slot)?;
    if !is_stamp(stamp) {
        return None;
    }
    Some((slot, stamp.to_string()))
}

/// `YYYYMMDDTHHMMSSZ` and nothing else.
fn is_stamp(stamp: &str) -> bool {
    stamp.len() == 16
        && stamp.chars().enumerate().all(|(at, ch)| match at {
            8 => ch == 'T',
            15 => ch == 'Z',
            _ => ch.is_ascii_digit(),
        })
}

fn archive_name(slot: Slot, stamp: &str) -> String {
    format!("kamosu-backup-{}-{stamp}.zip", slot.as_str())
}

/// Every Backup this instance holds, newest first.
pub fn list(data_dir: &Path) -> Result<Vec<Archive>, OpError> {
    let dir = backups_dir(data_dir);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        // No directory yet is no Backups yet, not a failure: an instance that
        // has never reached its first tick is in exactly this state.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(OpError::internal(format!(
                "cannot read the Backups directory {}: {e}",
                dir.display()
            )));
        }
    };

    let mut archives: Vec<Archive> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let Some((slot, stamp)) = parse_name(&name) else {
            continue;
        };
        let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
        archives.push(Archive {
            name,
            slot,
            stamp,
            size_bytes,
        });
    }
    archives.sort_by(|a, b| b.stamp.cmp(&a.stamp).then(a.name.cmp(&b.name)));
    Ok(archives)
}

/// Where one named Backup lives, refusing anything that is not a name this
/// module itself would have written. A caller never sends a path.
pub fn archive_path(data_dir: &Path, name: &str) -> Result<PathBuf, OpError> {
    if parse_name(name).is_none() {
        return Err(OpError::not_found("no such Backup"));
    }
    Ok(backups_dir(data_dir).join(name))
}

/// Take a Backup if any of the three places is due for one, and answer what
/// the instance holds afterwards.
///
/// An empty place counts as due, so the first run of a new instance fills all
/// three at once. That is deliberate: an instance with no past has nothing
/// further back to reach for, and the three pull apart from there as each
/// comes due.
pub fn run(data_dir: &Path, ask: Ask) -> Result<Value, OpError> {
    // Nothing here is re-entrant, and the lock guards `()`, so a run that
    // panicked left nothing for the next one to be confused by.
    let _taking = TAKING_ONE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let dir = backups_dir(data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| {
        OpError::internal(format!(
            "cannot create the Backups directory {}: {e}",
            dir.display()
        ))
    })?;

    // A run killed part-way through building leaves a working file behind, and
    // nothing else would ever take it away: it is not a Backup, so `list` does
    // not see it. Under the lock above, anything of this shape belongs to a run
    // that is no longer happening.
    sweep_working_files(&dir);

    let held = list(data_dir)?;
    // A second connection to the same file, which WAL is what makes safe
    // (ADR 0028). The Core's own connection stays free the whole time, so a
    // Backup never stops the instance answering.
    let conn = Connection::open(data_dir.join(DATABASE_FILE))
        .map_err(|e| OpError::internal(format!("cannot open the database to back it up: {e}")))?;

    let stamp = clock(&conn, THIS_MOMENT)?;
    let mut due: Vec<Slot> = Vec::new();
    for slot in Slot::ALL {
        let standing = held.iter().find(|archive| archive.slot == slot);
        let is_due = match standing {
            None => true,
            Some(archive) => archive.stamp <= clock(&conn, slot.age_allowed())?,
        };
        if is_due || (ask == Ask::Now && slot == Slot::Daily) {
            due.push(slot);
        }
    }

    if due.is_empty() {
        return Ok(json!({ "taken": [], "backups": as_json(&held) }));
    }

    // One archive, however many places are due: the three are copies of the
    // same moment, so taking it once and placing it is both cheaper and more
    // honest than backing the database up three times in a row.
    let working = dir.join(format!("{WORKING_PREFIX}{stamp}.zip"));
    let built = build(&conn, data_dir, &working).and_then(|()| place(&dir, &working, &due, &stamp));
    // Whatever happened, nothing of the working shape is left lying about.
    let _ = std::fs::remove_file(&working);
    let taken = built?;

    for slot in &due {
        let name = archive_name(*slot, &stamp);
        for old in held.iter().filter(|a| a.slot == *slot && a.name != name) {
            let _ = std::fs::remove_file(dir.join(&old.name));
        }
    }

    Ok(json!({ "taken": taken, "backups": as_json(&list(data_dir)?) }))
}

/// Put the finished archive into every place that is owed one, and answer what
/// they are called. The first takes the working file itself; the rest are
/// copies of it, because a second archive of the same moment is the same bytes.
///
/// Nothing is pruned here. Placing before pruning is the order that matters: a
/// failure part-way leaves every old archive exactly where it was.
fn place(dir: &Path, working: &Path, due: &[Slot], stamp: &str) -> Result<Vec<String>, OpError> {
    let mut taken: Vec<String> = Vec::new();
    let mut placed: Option<PathBuf> = None;
    for slot in due {
        let name = archive_name(*slot, stamp);
        let destination = dir.join(&name);
        let put = match &placed {
            None => std::fs::rename(working, &destination),
            Some(source) => std::fs::copy(source, &destination).map(|_| ()),
        };
        put.map_err(|e| {
            OpError::internal(format!(
                "cannot place the Backup at {}: {e}",
                destination.display()
            ))
        })?;
        placed.get_or_insert(destination);
        taken.push(name);
    }
    Ok(taken)
}

/// Take away anything of the working shape. Called under the lock, so nothing
/// it finds belongs to a run still in progress.
fn sweep_working_files(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(WORKING_PREFIX)
        {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// A list of archives as both Operations answer one.
pub fn as_json(archives: &[Archive]) -> Vec<Value> {
    archives.iter().map(Archive::as_json).collect()
}

/// A moment on SQLite's clock, in the fixed-width form archive names carry.
/// `offset` is a SQLite date modifier — `"+0 days"` for this moment, `"-7 days"`
/// for the oldest a weekly archive is allowed to be.
fn clock(conn: &Connection, offset: &str) -> Result<String, OpError> {
    conn.query_row(
        "SELECT strftime('%Y%m%dT%H%M%SZ','now',?1)",
        [offset],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot read the clock: {e}")))
}

/// Write one archive: the database as a consistent snapshot, then every
/// Photograph.
fn build(conn: &Connection, data_dir: &Path, destination: &Path) -> Result<(), OpError> {
    let snapshot = destination.with_extension("db");
    // `VACUUM INTO` refuses a file that already exists, and a previous run
    // interrupted mid-archive could have left one.
    let _ = std::fs::remove_file(&snapshot);

    // The consistency guarantee, and the whole reason this is not `fs::copy`:
    // `VACUUM INTO` reads the database inside one transaction, so what lands
    // in the file is the database as it stood at one instant — never a page
    // from before a save beside a page from after it. WAL is what lets that
    // read happen without stopping anybody writing (ADR 0028).
    conn.execute("VACUUM INTO ?1", [snapshot.to_string_lossy().as_ref()])
        .map_err(|e| {
            OpError::internal(format!("cannot take a consistent database snapshot: {e}"))
        })?;

    let result = write_zip(&snapshot, data_dir, destination);
    let _ = std::fs::remove_file(&snapshot);
    result
}

fn write_zip(snapshot: &Path, data_dir: &Path, destination: &Path) -> Result<(), OpError> {
    let file = std::fs::File::create(destination).map_err(|e| {
        OpError::internal(format!(
            "cannot write the Backup to {}: {e}",
            destination.display()
        ))
    })?;
    let mut archive = zip::ZipWriter::new(std::io::BufWriter::new(file));
    let deflated = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    // A Photograph is already a compressed picture. Deflating it again spends
    // the whole library's worth of CPU to save nothing.
    let stored =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // Named as it sits under `/data`, so restoring is unzipping over `/data`.
    archive
        .start_file(DATABASE_FILE, deflated)
        .map_err(|e| OpError::internal(format!("cannot start the database entry: {e}")))?;
    let snapshot_bytes = std::fs::File::open(snapshot)
        .map_err(|e| OpError::internal(format!("cannot read the database snapshot: {e}")))?;
    let mut snapshot_bytes = std::io::BufReader::new(snapshot_bytes);
    std::io::copy(&mut snapshot_bytes, &mut archive).map_err(|e| {
        OpError::internal(format!("cannot write the database into the Backup: {e}"))
    })?;

    // Named from the directory `photographs.rs` owns rather than typed here,
    // so the archive keeps matching `/data`'s layout if that directory moves —
    // which is the whole of what makes restoring an unzip (ADR 0039).
    let photographs = photographs::photographs_dir(data_dir);
    let inside = photographs
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "photographs".to_string());
    if let Ok(entries) = std::fs::read_dir(&photographs) {
        for entry in entries.flatten() {
            if !entry.file_type().is_ok_and(|kind| kind.is_file()) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            archive
                .start_file(format!("{inside}/{name}"), stored)
                .map_err(|e| {
                    OpError::internal(format!("cannot start the entry for {name}: {e}"))
                })?;
            let picture = std::fs::File::open(entry.path()).map_err(|e| {
                OpError::internal(format!("cannot read the Photograph {name}: {e}"))
            })?;
            let mut picture = std::io::BufReader::new(picture);
            std::io::copy(&mut picture, &mut archive).map_err(|e| {
                OpError::internal(format!(
                    "cannot write the Photograph {name} into the Backup: {e}"
                ))
            })?;
        }
    }

    let mut finished = archive
        .finish()
        .map_err(|e| OpError::internal(format!("cannot finish the Backup archive: {e}")))?;
    finished
        .flush()
        .map_err(|e| OpError::internal(format!("cannot flush the Backup archive: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Kamosu never sends a Backup anywhere** (#78, CONTEXT.md). That is a
    /// promise about code that does not exist, which is exactly the kind
    /// nothing else can check: no behaviour test can watch for a request that
    /// was never made. So this reads the module's own source and fails the
    /// build if it ever grows a way to reach the network.
    ///
    /// Only the source above `#[cfg(test)]` is scanned, so the names below can
    /// be written here without the check tripping over itself.
    #[test]
    fn nothing_here_can_send_a_backup_anywhere() {
        let whole = include_str!("backups.rs");
        let shipped = whole
            .split_once("#[cfg(test)]")
            .expect("this test module")
            .0;
        // Names, not URLs. A module that cannot reach an HTTP client, a socket
        // or a shell has no way out whatever addresses it mentions, and a list
        // of names does not fire on the day somebody cites an ADR by its link.
        for way_out in [
            "reqwest",
            "http_min",
            "TcpStream",
            "UdpSocket",
            "hf_hub",
            "ureq",
            "Command",
        ] {
            assert!(
                !shipped.contains(way_out),
                "src/backups.rs names `{way_out}`. A Backup is written beside the \
                 database and fetched by whoever asks for it — Kamosu never carries \
                 one anywhere itself (CONTEXT.md, \"Backup\")"
            );
        }
    }

    #[test]
    fn a_name_says_which_slot_it_holds_and_when_it_was_taken() {
        let (slot, stamp) =
            parse_name("kamosu-backup-weekly-20260908T141500Z.zip").expect("parses");
        assert_eq!(slot, Slot::Weekly);
        assert_eq!(stamp, "20260908T141500Z");
    }

    #[test]
    fn a_name_that_is_not_one_of_ours_is_not_a_backup() {
        for wrong in [
            "kamosu-backup-hourly-20260908T141500Z.zip",
            "kamosu-backup-daily-2026-09-08.zip",
            "kamosu-backup-daily-20260908T141500Z.tar",
            "../../etc/passwd",
            "kamosu-backup-daily-../../../etc/passwd.zip",
            "kamosu.db",
            "",
        ] {
            assert!(parse_name(wrong).is_none(), "`{wrong}` must not parse");
        }
    }

    #[test]
    fn a_stamp_reads_back_as_an_ordinary_timestamp() {
        let archive = Archive {
            name: archive_name(Slot::Daily, "20260908T141500Z"),
            slot: Slot::Daily,
            stamp: "20260908T141500Z".to_string(),
            size_bytes: 0,
        };
        assert_eq!(archive.name, "kamosu-backup-daily-20260908T141500Z.zip");
        assert_eq!(archive.taken_at(), "2026-09-08T14:15:00Z");
    }

    /// Fixed-width stamps are why the age comparisons can be string
    /// comparisons: later must sort after earlier, across every boundary.
    #[test]
    fn later_stamps_sort_after_earlier_ones() {
        let mut moments = [
            "20260101T000000Z",
            "20251231T235959Z",
            "20260908T141500Z",
            "20260908T141459Z",
        ];
        moments.sort();
        assert_eq!(
            moments,
            [
                "20251231T235959Z",
                "20260101T000000Z",
                "20260908T141459Z",
                "20260908T141500Z",
            ]
        );
    }
}
