//! Photographs: storing and reading them, who may see one, display copies,
//! and the daily sweep of pictures nothing refers to any more.

use super::*;

impl Core {
    /// How often the orphan sweep runs by itself.
    ///
    /// Daily, and never on a schedule anyone has to configure: a picture worth
    /// keeping for a week is not worth an environment variable.
    const SWEEP_EVERY: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);

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
    /// into disagreeing about what its bytes are. `import_bundle` calls this
    /// for every Photograph whose bytes hash to the name it arrived under.
    pub fn store_photograph_verbatim(&self, bytes: &[u8]) -> Result<Value, OpError> {
        // Kept as it arrived, but not taken on trust: a Bundle is a file a
        // stranger wrote, and the picture inside it is checked by its header
        // exactly as an uploaded one is (ADR 0017, ADR 0034). What is refused
        // here is refused before any decode, which is the whole point of
        // checking at all.
        photographs::check(bytes)?;
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

    /// A picture a Person uploaded, remembered as theirs so they can read it
    /// back before anything names it (#99). What the upload answers is exactly
    /// what [`Core::store_photograph`] answers, whether or not the picture was
    /// already here, so uploading still tells the caller nothing.
    pub fn upload_photograph(&self, caller: &Caller, bytes: &[u8]) -> Result<Value, OpError> {
        let stored = self.store_photograph(bytes)?;
        let hash = stored["photograph_id"]
            .as_str()
            .ok_or_else(|| OpError::internal("a stored Photograph came back with no id"))?;
        self.db().with_conn(|conn| {
            conn.execute(
                "INSERT OR IGNORE INTO photograph_uploads (hash, person_id) VALUES (?1, ?2)",
                params![hash, caller.person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot record the upload: {e}")))?;
            Ok(())
        })?;
        Ok(stored)
    }

    /// Read a Photograph's own bytes back, for a caller who may see it — the
    /// out-of-band Web Door route asks for these directly; no Operation wraps
    /// binary output in JSON.
    ///
    /// **Who may see it is asked here and not in the Door** (ADR 0001): the
    /// route carries raw bytes, so it never passes through the Catalogue's
    /// dispatch, and a check written into the Door would be the bug that rule
    /// names. A caller who may not see it is told exactly what a hash this
    /// instance never held is told (ADR 0040) — a Photograph's id is a hash of
    /// its bytes, so anybody holding the file can name it (ADR 0017).
    pub fn read_photograph(&self, caller: &Caller, hash: &str) -> Result<Vec<u8>, OpError> {
        self.may_see_photograph(caller, hash)?;
        self.photograph_bytes(hash)
    }

    /// Read a Photograph's bytes with no question asked of who wants them —
    /// for the paths that have already decided: a Share Link that carries the
    /// picture, a Sheet or Bundle of a recipe the caller may read.
    pub(super) fn photograph_bytes(&self, hash: &str) -> Result<Vec<u8>, OpError> {
        std::fs::read(photographs::photograph_path(&self.data_dir(), hash))
            .map_err(|_| no_such_photograph())
    }

    /// Refuse, as if the picture were not here at all, unless this caller can
    /// already see it somewhere (#99). Three places count, and ADR 0026's
    /// boundary — the Kitchens you cook in — is the whole of the first two:
    ///
    /// - **a Version** naming it, carried by a Branch of a Kitchen the caller
    ///   cooks in. Every Version the Branch carries, not only its head,
    ///   because the Thread opens any of them (ADR 0005); a Translation is a
    ///   Branch like any other, so it is covered by the same join (ADR 0006).
    /// - **an Attempt** holding it, of the caller's own or of anybody in the
    ///   household: the people sharing with them a Kitchen that holds the
    ///   Lineage, the scope [`cooking_record`] already uses. A household
    ///   Attempt counts for its own pictures only; the Version it pins is the
    ///   cook's own business, while their own Attempts count whole, since the
    ///   Diary shows them after the recipe has gone.
    /// - **an upload** of it by the caller. This is what lets the writing
    ///   screen show a picture before the save names it.
    ///
    /// The uploader keeps it for good, not only while it is attached to
    /// nothing. Limiting it that way makes an oracle of its own: upload a picture, and whether you can
    /// read it back would say whether some recipe on the instance shows it.
    /// An uploader holds the bytes already, so reading them back leaks
    /// nothing. The household, meanwhile, loses a detached picture at once
    /// rather than when the sweep takes it a week later.
    ///
    /// **Asked fresh every time, not kept in a table beside the sweep.** The
    /// Recipes screen asks for dozens of Display Copies at once, so this runs
    /// in bursts — but each one is a few indexed joins filtered by `instr` on
    /// the stored JSON, so only the rows that could name this hash are parsed.
    /// A kept table would need the sweep's reasoning about tallies (see
    /// [`Core::referenced_photographs`]) and would only move the cost to the
    /// writes, where a stale row wrongly *grants* a picture.
    ///
    /// Runs before any file is read, so a refused ask never draws a Display
    /// Copy: that is work a stranger could cause at will (ADR 0032).
    fn may_see_photograph(&self, caller: &Caller, hash: &str) -> Result<(), OpError> {
        let seen = self
            .db()
            .with_conn(|conn| photograph_seen_by(conn, &caller.person_id, hash))?;
        if seen {
            Ok(())
        } else {
            Err(no_such_photograph())
        }
    }

    /// How long a Photograph nothing points at is kept before the sweep takes
    /// it. A week, so that detaching a picture and putting it back — or
    /// editing a recipe in two sittings — never costs you the picture.
    const ORPHAN_GRACE_DAYS: i64 = 7;

    /// Every Photograph anything still points at, worked out **from scratch**.
    ///
    /// Two things reach a Version, and both count:
    ///
    /// - a Branch, at every sequence and not merely its head — a Version is
    ///   never rewritten and the Thread lets a person open and cook from any of
    ///   them (ADR 0005), so a picture named by an old Version is still in use;
    /// - an Attempt, which pins the Version it is cooking from (ADR 0010) —
    ///   that Version's pictures are on a screen in a kitchen right now.
    ///
    /// An Attempt also holds Photographs of its **own** — the pictures taken
    /// during that cooking (#59) — which belong to no Version at all. They are
    /// counted here for the same reason the rest is: a cooking record is a
    /// diary somebody keeps, and the sweep taking its pictures away a week
    /// later because no recipe happens to name them is the one failure this
    /// whole design exists to prevent.
    ///
    /// What does *not* count is a row left in `versions` that nothing reaches.
    /// A collapsing save repoints `branch_versions` at a fresh Version and
    /// leaves the one it replaced behind (ADR 0005's append-only rule applies
    /// to what a Branch carries, not to the row store). Reading `versions`
    /// whole would count those as live and the sweep would never take
    /// anything — the bug this join exists to prevent.
    ///
    /// Recomputing this each run is what makes the sweep safe. The alternative
    /// — a count kept as pictures are attached and detached — has one failure
    /// mode that deletes a picture still on screen, silently and permanently.
    ///
    /// Takes the connection rather than reaching for one, so the sweep can do
    /// its whole read-and-write inside a single `with_conn` — see
    /// [`Core::sweep_photographs`] for why that matters.
    fn referenced_photographs(
        conn: &rusqlite::Connection,
    ) -> Result<std::collections::HashSet<String>, OpError> {
        let mut statement = conn
            .prepare(
                "SELECT versions.content FROM versions
                 JOIN branch_versions ON branch_versions.version_id = versions.id
                 UNION
                 SELECT versions.content FROM versions
                 JOIN attempts ON attempts.version_id = versions.id",
            )
            .map_err(|e| OpError::internal(format!("cannot read Versions: {e}")))?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| OpError::internal(format!("cannot read Versions: {e}")))?;

        let mut referenced = std::collections::HashSet::new();
        for row in rows {
            let content =
                row.map_err(|e| OpError::internal(format!("cannot read a Version: {e}")))?;
            // A Version whose content will not parse is a damaged row, not a
            // licence to delete pictures. Skipping it here would treat its
            // photographs as unreferenced, so it is a hard failure.
            let value: Value = serde_json::from_str(&content).map_err(|e| {
                OpError::internal(format!("a Version's content is not readable: {e}"))
            })?;
            if let Some(hash) = value.get("main_photo").and_then(Value::as_str) {
                referenced.insert(hash.to_string());
            }
            if let Some(steps) = value.get("steps").and_then(Value::as_array) {
                for step in steps {
                    if let Some(hash) = step.get("photo").and_then(Value::as_str) {
                        referenced.insert(hash.to_string());
                    }
                }
            }
        }

        // The Photographs an Attempt holds in its own right (#59). Read from
        // the Attempts themselves, since no Version names them.
        let mut statement = conn
            .prepare("SELECT photographs FROM attempts")
            .map_err(|e| OpError::internal(format!("cannot read Attempts: {e}")))?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| OpError::internal(format!("cannot read Attempts: {e}")))?;
        for row in rows {
            let stored =
                row.map_err(|e| OpError::internal(format!("cannot read an Attempt: {e}")))?;
            // As above: an unreadable row is a damaged row, never a licence to
            // delete somebody's pictures.
            let listed: Vec<String> = serde_json::from_str(&stored).map_err(|e| {
                OpError::internal(format!("an Attempt's Photographs are not readable: {e}"))
            })?;
            referenced.extend(listed);
        }

        Ok(referenced)
    }

    /// The orphan sweep: take away Photographs nothing has pointed at for a
    /// week, and their Display Copies with them (#46, ADR 0017).
    ///
    /// Each run recomputes what is referenced rather than trusting a tally,
    /// then writes `unreferenced_since` to match. So the mark is a *derived*
    /// fact that every run re-derives: a picture detached and re-attached
    /// inside the week has its mark cleared and survives, and a mark that is
    /// somehow wrong is corrected by the next run rather than compounding.
    ///
    /// Display Copies go with the Photograph because they are worked out from
    /// it and are worth nothing without it.
    ///
    /// **The whole decision happens inside one `with_conn`**, which holds the
    /// one database connection's lock for its duration and so cannot interleave
    /// with a `save_recipe_version`. Working out what is referenced, aging the
    /// marks and deleting the rows in separate locks would leave a window in
    /// which a picture re-attached after the check is deleted anyway — the
    /// exact failure this design exists to prevent, and the one failure here
    /// that no later run can undo.
    ///
    /// Files are removed only once that transaction has committed, so the
    /// bytes never go while a row that names them could still be rolled back.
    /// A crash between the two leaves unreferenced files with no rows, which
    /// costs disk and nothing else.
    pub fn sweep_photographs(&self) -> Result<Value, OpError> {
        let (referenced_count, newly_marked, cleared, swept) = self.db().with_conn(|conn| {
            let referenced = Self::referenced_photographs(conn)?;

            let known: Vec<(String, Option<String>)> = {
                let mut statement = conn
                    .prepare("SELECT hash, unreferenced_since FROM photographs")
                    .map_err(|e| OpError::internal(format!("cannot list Photographs: {e}")))?;
                let rows = statement
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                    .map_err(|e| OpError::internal(format!("cannot list Photographs: {e}")))?;
                rows.collect::<Result<Vec<_>, _>>()
                    .map_err(|e| OpError::internal(format!("cannot list a Photograph: {e}")))?
            };

            let mut newly_marked = 0_i64;
            let mut cleared = 0_i64;
            let mut swept: Vec<String> = Vec::new();

            for (hash, unreferenced_since) in known {
                if referenced.contains(&hash) {
                    // In use. Clear any mark: the spell has ended.
                    if unreferenced_since.is_some() {
                        conn.execute(
                            "UPDATE photographs SET unreferenced_since = NULL WHERE hash = ?1",
                            params![hash],
                        )
                        .map_err(|e| OpError::internal(format!("cannot clear the mark: {e}")))?;
                        cleared += 1;
                    }
                    continue;
                }

                match unreferenced_since {
                    // First sweep to find it unreferenced: start the clock,
                    // take nothing. Nothing is ever deleted on the run that
                    // first notices it.
                    None => {
                        conn.execute(
                            "UPDATE photographs
                             SET unreferenced_since = strftime('%Y-%m-%dT%H:%M:%fZ','now')
                             WHERE hash = ?1",
                            params![hash],
                        )
                        .map_err(|e| OpError::internal(format!("cannot mark: {e}")))?;
                        newly_marked += 1;
                    }
                    Some(_) => {
                        // Old enough? Ask SQLite, so the comparison uses the
                        // same clock and format every other timestamp does.
                        let due: bool = conn
                            .query_row(
                                "SELECT unreferenced_since <= \
                                 strftime('%Y-%m-%dT%H:%M:%fZ','now', ?2) \
                                 FROM photographs WHERE hash = ?1",
                                params![hash, format!("-{} days", Self::ORPHAN_GRACE_DAYS)],
                                |row| row.get(0),
                            )
                            .map_err(|e| {
                                OpError::internal(format!("cannot age a Photograph: {e}"))
                            })?;
                        if !due {
                            continue;
                        }
                        conn.execute("DELETE FROM photographs WHERE hash = ?1", params![hash])
                            .map_err(|e| {
                                OpError::internal(format!("cannot forget a Photograph: {e}"))
                            })?;
                        swept.push(hash);
                    }
                }
            }

            Ok((referenced.len(), newly_marked, cleared, swept))
        })?;

        // The rows are gone and the lock is released; now the bytes. A file
        // that will not delete is left for the next sweep to meet again.
        for hash in &swept {
            for size in [
                photographs::DisplaySize::Card,
                photographs::DisplaySize::Page,
                photographs::DisplaySize::Print,
            ] {
                let _ =
                    std::fs::remove_file(photographs::display_path(&self.data_dir(), hash, size));
            }
            let _ = std::fs::remove_file(photographs::photograph_path(&self.data_dir(), hash));
        }

        Ok(json!({
            "referenced": referenced_count,
            "newly_unreferenced": newly_marked,
            "back_in_use": cleared,
            "swept": swept.len(),
            "swept_photograph_ids": swept,
        }))
    }

    /// Read a Display Copy, generating and caching it on first ask. Display
    /// Copies are worked out from the Photograph and kept only for
    /// convenience (ADR 0017), so a missing one is made rather than an error.
    ///
    /// Asks [`Core::may_see_photograph`] first, before the cache as much as
    /// before the drawing: a cached copy is still somebody's picture.
    pub fn read_display_copy(
        &self,
        caller: &Caller,
        hash: &str,
        size: photographs::DisplaySize,
    ) -> Result<Vec<u8>, OpError> {
        self.may_see_photograph(caller, hash)?;
        self.display_copy_bytes(hash, size)
    }

    /// [`Core::read_display_copy`] with no question asked of who wants it,
    /// for the paths that have already decided, as [`Core::photograph_bytes`].
    pub(super) fn display_copy_bytes(
        &self,
        hash: &str,
        size: photographs::DisplaySize,
    ) -> Result<Vec<u8>, OpError> {
        let path = photographs::display_path(&self.data_dir(), hash, size);
        if let Ok(cached) = std::fs::read(&path) {
            return Ok(cached);
        }
        let source = self.photograph_bytes(hash)?;
        let copy = photographs::display_copy(&source, size.long_edge())?;
        std::fs::create_dir_all(photographs::display_dir(&self.data_dir())).map_err(|e| {
            OpError::internal(format!("cannot create Display Copies directory: {e}"))
        })?;
        std::fs::write(&path, &copy)
            .map_err(|e| OpError::internal(format!("cannot cache Display Copy: {e}")))?;
        Ok(copy)
    }
}

/// Run the orphan sweep on a timer for as long as the instance serves (#46).
///
/// Deliberately not a Job: a Job is work someone asked for and can watch, and
/// nobody asks for this. It answers to nothing, reports to nothing, and a
/// failed run is logged and forgotten — the next run recomputes everything
/// from scratch anyway, so there is no state for a failure to corrupt.
///
/// The same daily pass clears staged uploads nobody used (#69), the other
/// kind of file Kamosu holds only for as long as something needs it.
///
/// The work itself is blocking SQLite and filesystem calls, so it goes to a
/// blocking thread rather than stalling the runtime the Doors answer on.
pub(super) fn spawn_orphan_sweep(core: Arc<Core>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Core::SWEEP_EVERY);
        loop {
            ticker.tick().await;
            let core = core.clone();
            let swept = tokio::task::spawn_blocking(move || {
                let uploads = core.sweep_uploads()?;
                let mut report = core.sweep_photographs()?;
                report["uploads_swept"] = json!(uploads);
                Ok::<_, OpError>(report)
            })
            .await;
            match swept {
                Ok(Ok(report)) => {
                    tracing::info!(target: "kamosu::sweep", report = %report, "orphan sweep");
                }
                Ok(Err(error)) => {
                    tracing::warn!(target: "kamosu::sweep", %error, "orphan sweep failed");
                }
                Err(error) => {
                    tracing::warn!(target: "kamosu::sweep", %error, "orphan sweep panicked");
                }
            }
        }
    });
}

/// What a Photograph route answers about a hash this instance never held, and
/// so, word for word, about one the caller may not see (#99, ADR 0040).
fn no_such_photograph() -> OpError {
    OpError::not_found("no such Photograph")
}

/// Whether this Person can already see this Photograph somewhere — the rule
/// [`Core::may_see_photograph`] explains.
///
/// Every query narrows with `instr` on the stored JSON before anything is
/// parsed, so a burst of Display Copies costs a string search per candidate
/// row rather than a parse of the whole library. What `instr` finds is then
/// confirmed field by field: a hash typed into a recipe's words is not a
/// picture the recipe shows.
fn photograph_seen_by(conn: &Connection, person_id: &str, hash: &str) -> Result<bool, OpError> {
    let failed = |e: rusqlite::Error| OpError::internal(format!("cannot read who sees it: {e}"));

    let uploaded: bool = conn
        .query_row(
            "SELECT EXISTS (SELECT 1 FROM photograph_uploads WHERE hash = ?1 AND person_id = ?2)",
            params![hash, person_id],
            |row| row.get(0),
        )
        .map_err(failed)?;
    if uploaded {
        return Ok(true);
    }

    // A Version carried by a Branch of a Cookbook this Person may see.
    let mut statement = conn
        .prepare_cached(
            "SELECT versions.content FROM versions
               JOIN branch_versions ON branch_versions.version_id = versions.id
               JOIN branches ON branches.id = branch_versions.branch_id
              WHERE branches.cookbook_id IN
                        (SELECT cookbook_id FROM visible_cookbooks WHERE person_id = ?2)
                AND instr(versions.content, ?1) > 0",
        )
        .map_err(failed)?;
    let contents = statement
        .query_map(params![hash, person_id], |row| row.get::<_, String>(0))
        .map_err(failed)?;
    for content in contents {
        if stored_content_shows(&content.map_err(failed)?, hash) {
            return Ok(true);
        }
    }

    // An Attempt: this Person's own, whole — its pictures and the Versions it
    // pins — or one of their company's on a recipe they may see, for the
    // pictures it holds itself (#131, question 1).
    let mut statement = conn
        .prepare_cached(
            &"SELECT attempts.person_id = ?2, attempts.photographs,
                    pinned.content, as_cooked.content
               FROM attempts
               JOIN versions AS pinned ON pinned.id = attempts.version_id
               LEFT JOIN versions AS as_cooked ON as_cooked.id = attempts.as_cooked_version_id
              WHERE (instr(attempts.photographs, ?1) > 0
                     OR instr(pinned.content, ?1) > 0
                     OR instr(as_cooked.content, ?1) > 0)
                AND (attempts.person_id = ?2 OR (
                      attempts.person_id IN (COMPANY)
                      AND EXISTS (
                        SELECT 1 FROM branches
                         WHERE branches.lineage_id = attempts.lineage_id
                           AND branches.cookbook_id IN
                               (SELECT cookbook_id FROM visible_cookbooks WHERE person_id = ?2))))"
                .replace("COMPANY", &company_of("?2")),
        )
        .map_err(failed)?;
    let attempts = statement
        .query_map(params![hash, person_id], |row| {
            Ok((
                row.get::<_, bool>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(failed)?;
    for attempt in attempts {
        let (own, held, pinned, as_cooked) = attempt.map_err(failed)?;
        let held: Vec<String> = serde_json::from_str(&held).unwrap_or_default();
        if held.iter().any(|photograph| photograph == hash) {
            return Ok(true);
        }
        if !own {
            continue;
        }
        if std::iter::once(pinned)
            .chain(as_cooked)
            .any(|content| stored_content_shows(&content, hash))
        {
            return Ok(true);
        }
    }

    Ok(false)
}

/// [`content_shows_photograph`] over a Version's content as it is stored.
///
/// A row that will not parse shows nothing, so its pictures are refused: the
/// safe way to fail here, where the sweep's hard failure would be the unsafe
/// one. It is still said, since a damaged Version is worth knowing about.
fn stored_content_shows(stored: &str, hash: &str) -> bool {
    match serde_json::from_str::<Value>(stored) {
        Ok(content) => content_shows_photograph(&content, hash),
        Err(error) => {
            tracing::warn!("a Version's content is not readable: {error}");
            false
        }
    }
}

/// Whether a Version's content shows this Photograph, read off the content
/// alone.
pub(super) fn content_shows_photograph(content: &Value, hash: &str) -> bool {
    if content["main_photo"].as_str() == Some(hash) {
        return true;
    }
    content["steps"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|step| step["photo"].as_str() == Some(hash))
}
