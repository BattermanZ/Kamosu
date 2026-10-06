//! Imports: a Crouton export, staged uploads, and the ledger that finds every
//! Import again (#108).

use super::*;

impl Core {
    /// Bring in a Crouton library (#69): every `.crumb` in the export read one
    /// at a time, its first picture remade into a Photograph at the door (#45),
    /// and landed through the ledger every importer shares, so a second run
    /// matches what the first made (ADR 0025). What the reader left out on
    /// purpose — the site's favicon, pictures past the first — is named in the
    /// Report rather than stored.
    pub fn import_crouton(
        &self,
        caller: &Caller,
        mut export: crouton::Export,
        progress: Option<&JobProgress>,
    ) -> Result<Value, OpError> {
        let total = export.len();
        let incoming = (0..total).map(move |index| {
            let (name, read) = export.read(index);
            let crumb = match read {
                Ok(crumb) => crumb,
                Err(reason) => {
                    return Incoming {
                        candidate: Err(Unread {
                            foreign_id: None,
                            name,
                            reason,
                        }),
                        left_out: Vec::new(),
                        tags: Vec::new(),
                    };
                }
            };
            let mut left_out = Vec::new();
            if let Some(icon) = crumb.site_icon {
                left_out.push(LeftOut::SiteIcon(icon));
            }
            if crumb.photos.len() > 1 {
                left_out.push(LeftOut::ExtraPhotos(crumb.photos.len() - 1));
            }
            // A picture that cannot be remade never fails its recipe (#45):
            // the recipe lands without it, and the Report says so.
            let mut unreadable_photos = crumb.unreadable_photos;
            let main_photo = crumb.photos.first().and_then(|bytes| {
                let stored = self
                    .store_photograph(bytes)
                    .ok()
                    .and_then(|stored| stored["photograph_id"].as_str().map(str::to_string));
                if stored.is_none() {
                    unreadable_photos += 1;
                }
                stored
            });
            if unreadable_photos > 0 {
                left_out.push(LeftOut::UnreadablePhotos(unreadable_photos));
            }
            let mut candidate = crumb.candidate;
            candidate["foreign_id"] = json!(crumb.foreign_id);
            candidate["main_photo"] = json!(main_photo);
            Incoming {
                candidate: Ok(candidate),
                left_out,
                tags: crumb.tags,
            }
        });
        self.import_each(caller, "crouton", total, incoming, progress)
    }

    /// What this Person has brought in from outside, and what happened each
    /// time (#108).
    ///
    /// The answer is grouped by **source kind** rather than by Import row, and
    /// the distinction is the whole design. An Import is one durable channel
    /// per Kitchen per source, reused by every run after the first (ADR 0025),
    /// so three Imports can sit behind forty-eight arrivals. `forget_import`
    /// deletes the channel; the Jobs it ran never vanish, and each still holds
    /// the Report that `get_job` serves. Keying the answer on the Import row
    /// would therefore hide every past Report the moment a ledger was
    /// forgotten — which is the exact unreachability this Operation exists to
    /// end. So a source is listed while it has *either* a ledger or an
    /// arrival, and `import_id` is null once the ledger is gone.
    ///
    /// **Scoped to the caller's own Cookbook, and that is what an Import is.**
    /// Every importer lands its recipes in the Cookbook of whoever asked
    /// (`import_each`, and the Bundle path alike; ADR 0041), which GLOSSARY.md
    /// states as the definition rather than as an implementation detail.
    /// `imports` is unique per `(cookbook_id, source_kind)`, so one Cookbook
    /// has one channel of each kind, however many Co-authors run it.
    ///
    /// Arrivals are the caller's own Jobs, matching `list_jobs` and the reader
    /// check `get_job` makes: listing a Co-author's Job would offer a link that
    /// then refuses. `remembered` is the Cookbook's, because a ledger belongs
    /// to the Cookbook and not to whoever happened to run the importer.
    pub fn list_imports(&self, person_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            // The ledgers this Person's Cookbook holds, newest channel first.
            let ledgers: Vec<(String, String, String, i64)> = {
                let mut statement = conn
                    .prepare(
                        "SELECT imports.id, imports.source_kind, imports.created_at,
                                (SELECT COUNT(*) FROM import_ledger
                                  WHERE import_ledger.import_id = imports.id)
                           FROM imports
                           JOIN cookbook_authors ON cookbook_authors.cookbook_id = imports.cookbook_id
                          WHERE cookbook_authors.person_id = ?1
                          ORDER BY imports.created_at DESC",
                    )
                    .map_err(|e| OpError::internal(format!("cannot list Imports: {e}")))?;
                statement
                    .query_map(params![person_id], |row| {
                        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
                    })
                    .map_err(|e| OpError::internal(format!("cannot list Imports: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot list Imports: {e}")))?
            };

            // Every arrival this Person asked for, newest first. Queried on the
            // importer names rather than through `jobs_of`, whose hundred-row
            // cap is about watching recent work and would quietly drop the old
            // arrivals this screen exists to reach.
            //
            // `input` is read with `json_extract` and never whole. A Job's input
            // is stored verbatim (`jobs::record`), and `import_crouton` accepts
            // the export itself as base64 `data` for a Door that can send only
            // JSON — so a row's input can be a 114 MB library rendered as 152 MB
            // of text. All this needs from it is one short string, and only for
            // `import`, the one importer that does not name its source in its
            // own definition.
            let arrivals: Vec<ArrivalRow> = {
                let mut statement = conn
                    .prepare(
                        "SELECT id, operation, json_extract(input, '$.source_kind'),
                                status, result, created_at
                           FROM jobs
                          WHERE person_id = ?1
                            AND operation IN ('import', 'import_crouton',
                                              'import_bundle', 'import_web_link')
                          ORDER BY created_at DESC",
                    )
                    .map_err(|e| OpError::internal(format!("cannot list arrivals: {e}")))?;
                statement
                    .query_map(params![person_id], |row| {
                        let result: Option<String> = row.get(4)?;
                        Ok(ArrivalRow {
                            job_id: row.get(0)?,
                            operation: row.get(1)?,
                            declared: row.get(2)?,
                            status: row.get(3)?,
                            result: result.and_then(|text| serde_json::from_str(&text).ok()),
                            created_at: row.get(5)?,
                        })
                    })
                    .map_err(|e| OpError::internal(format!("cannot list arrivals: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot list arrivals: {e}")))?
            };

            let mut grouped: Vec<(String, Vec<Value>)> = Vec::new();
            for row in arrivals {
                let Some(source_kind) = arrival_source_kind(
                    &row.operation,
                    row.declared.as_deref(),
                    row.result.as_ref(),
                ) else {
                    continue;
                };
                let summary = arrival_summary(
                    &row.job_id,
                    &row.status,
                    &row.created_at,
                    row.result.as_ref(),
                );
                match grouped.iter_mut().find(|(kind, _)| *kind == source_kind) {
                    Some((_, rows)) => rows.push(summary),
                    None => grouped.push((source_kind, vec![summary])),
                }
            }

            // A source the ledger knows leads; one that only arrivals remember
            // — a forgotten channel — follows, rather than disappearing.
            let mut imports = Vec::new();
            for (import_id, source_kind, created_at, remembered) in ledgers {
                let position = grouped.iter().position(|(kind, _)| *kind == source_kind);
                let rows = position.map(|at| grouped.remove(at).1).unwrap_or_default();
                imports.push(json!({
                    "import_id": import_id,
                    "source_kind": source_kind,
                    "created_at": created_at,
                    "remembered": remembered,
                    "arrivals": rows,
                }));
            }
            for (source_kind, rows) in grouped {
                let created_at = rows
                    .last()
                    .and_then(|row| row["created_at"].as_str())
                    .unwrap_or_default()
                    .to_string();
                imports.push(json!({
                    "import_id": Value::Null,
                    "source_kind": source_kind,
                    "created_at": created_at,
                    "remembered": 0,
                    "arrivals": rows,
                }));
            }

            Ok(json!({ "imports": imports }))
        })
    }

    /// Throw an Import's ledger away whole (ADR 0025): the record of which
    /// foreign id became which recipe, and the Import itself. Every recipe it
    /// made stays exactly as it is — nothing about where a recipe came from was
    /// ever on it. The cost is the ledger's whole point run backwards: an
    /// import of the same file afterwards matches nothing, and brings every
    /// recipe in again.
    pub fn forget_import(&self, person_id: &str, import_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let cookbook_id: String = conn
                .query_row(
                    "SELECT cookbook_id FROM imports WHERE id = ?1",
                    params![import_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read Import: {e}")))?
                .ok_or_else(no_such_import)?;
            ensure_writes_or_absent(conn, &cookbook_id, person_id, no_such_import)?;
            let transaction = conn
                .unchecked_transaction()
                .map_err(|e| OpError::internal(format!("cannot begin: {e}")))?;
            let forgotten = transaction
                .execute(
                    "DELETE FROM import_ledger WHERE import_id = ?1",
                    params![import_id],
                )
                .map_err(|e| OpError::internal(format!("cannot forget the ledger: {e}")))?;
            transaction
                .execute("DELETE FROM imports WHERE id = ?1", params![import_id])
                .map_err(|e| OpError::internal(format!("cannot forget the Import: {e}")))?;
            transaction
                .commit()
                .map_err(|e| OpError::internal(format!("cannot forget the Import: {e}")))?;
            Ok(json!({ "import_id": import_id, "forgotten": forgotten }))
        })
    }

    /// A file too large for an Operation's JSON envelope, staged on disk under
    /// the Person who sent it until an Operation names it by its id (ADR 0001:
    /// non-JSON payloads travel out of band under the same Credential). A
    /// Crouton library is 114 MB; the envelope, and a Job's stored input,
    /// should carry an id, not the library.
    ///
    /// Staged uploads live beside the database and the Photographs but are
    /// neither: no Backup carries them, and [`Core::sweep_uploads`] clears any
    /// nobody used.
    pub fn begin_upload(&self, person_id: &str) -> Result<(String, std::path::PathBuf), OpError> {
        let dir = self.uploads_dir(person_id)?;
        std::fs::create_dir_all(&dir)
            .map_err(|e| OpError::internal(format!("cannot create the uploads directory: {e}")))?;
        let upload_id = format!("u_{}", hex::encode(random_bytes(16)));
        let path = dir.join(&upload_id);
        Ok((upload_id, path))
    }

    /// Where the Person's own staged upload sits. Another Person's upload is
    /// not found rather than refused: its id is theirs, and says nothing here.
    pub fn staged_upload(
        &self,
        person_id: &str,
        upload_id: &str,
    ) -> Result<std::path::PathBuf, OpError> {
        let well_formed = upload_id.len() == 34
            && upload_id.starts_with("u_")
            && upload_id[2..].bytes().all(|b| b.is_ascii_hexdigit());
        let path = self.uploads_dir(person_id)?.join(upload_id);
        if !well_formed || !path.is_file() {
            return Err(OpError::not_found(
                "no such upload: send the file again, then ask with the id it answers",
            ));
        }
        Ok(path)
    }

    fn uploads_dir(&self, person_id: &str) -> Result<std::path::PathBuf, OpError> {
        if person_id.is_empty()
            || !person_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            return Err(OpError::internal(
                "a Person id is not a safe directory name",
            ));
        }
        Ok(self.data_dir().join("uploads").join(person_id))
    }

    /// How long a staged upload nobody used is kept. A day covers a person
    /// who sent a file and wandered off before the import started.
    const UPLOAD_GRACE: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);

    /// Delete staged uploads older than a day. An upload is consumed by the
    /// Operation that names it, so anything this finds is a file somebody
    /// sent and never used — or one whose import was interrupted.
    pub fn sweep_uploads(&self) -> Result<usize, OpError> {
        let root = self.data_dir().join("uploads");
        let Ok(people) = std::fs::read_dir(&root) else {
            return Ok(0);
        };
        let mut swept = 0;
        for person in people.flatten() {
            let Ok(files) = std::fs::read_dir(person.path()) else {
                continue;
            };
            for file in files.flatten() {
                let stale = file
                    .metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|modified| modified.elapsed().ok())
                    .is_some_and(|age| age > Self::UPLOAD_GRACE);
                if stale && std::fs::remove_file(file.path()).is_ok() {
                    swept += 1;
                }
            }
        }
        Ok(swept)
    }

    /// Import: land a batch of candidates already read from an outside
    /// source into the caller's own Cookbook, matched through that
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
        self.import_each(
            caller,
            source_kind,
            candidates.len(),
            candidates.iter().cloned().map(Incoming::candidate),
            progress,
        )
    }

    /// The same landing, for an importer that reads its source one recipe at a
    /// time rather than holding the whole of it: a Crouton library is 86 files
    /// and 110 MB of photographs, and none of it needs to be in memory at once
    /// (#69). `incoming` is pulled once per recipe, after the progress for it
    /// is reported, so the slow part of reading — remaking a photograph — is
    /// what the progress counts.
    ///
    /// Beyond what every Import Report says, this one also says what an
    /// importer read and deliberately did not bring in (`left_out`), and which
    /// of the recipes it landed share a name or a web page (`related_candidates`)
    /// — offered as **Related Recipes** to tick, never joined, since importing
    /// never guesses at a Lineage (ADR 0025).
    pub fn import_each(
        &self,
        caller: &Caller,
        source_kind: &str,
        total: usize,
        mut incoming: impl Iterator<Item = Incoming>,
        progress: Option<&JobProgress>,
    ) -> Result<Value, OpError> {
        let source_kind = required_text(source_kind, "source_kind")?.to_string();
        // Always into the caller's own Cookbook (ADR 0041): there is no
        // question of where an imported recipe goes.
        let (cookbook_id, cookbook_hand_id) = self.db().with_conn(|conn| {
            let cookbook_id = cookbook_of_person(conn, &caller.person_id)?;
            let hand_id = cookbook_hand(conn, &cookbook_id)?;
            Ok((cookbook_id, hand_id))
        })?;

        let import_id = self
            .db()
            .with_conn(|conn| find_or_create_import(conn, &cookbook_id, &source_kind))?;

        let total = total as u64;
        let mut arrived = Vec::new();
        let mut offered = Vec::new();
        let mut unreadable = Vec::new();
        let mut left_out = Vec::new();
        let mut landed = Vec::new();

        let mut done = 0u64;
        loop {
            if let Some(progress) = progress {
                progress.report(done, Some(total), format!("{done} of {total}"));
            }
            let Some(item) = incoming.next() else { break };
            done += 1;

            let candidate = match item.candidate {
                Ok(candidate) => candidate,
                Err(unread) => {
                    let mut row =
                        json!({ "foreign_id": unread.foreign_id, "reason": unread.reason });
                    if let Some(name) = unread.name {
                        row["name"] = json!(name);
                    }
                    unreadable.push(row);
                    continue;
                }
            };

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
            let shape = LandedShape::of(&candidate);

            let (lineage_id, branch_id, title) = match self.import_one(
                caller,
                &cookbook_id,
                &cookbook_hand_id,
                &import_id,
                &foreign_id,
                &candidate,
                &item.tags,
            ) {
                Ok(ImportOutcome::Landed {
                    lineage_id,
                    branch_id,
                    title,
                    status,
                }) => {
                    arrived.push(json!({
                        "foreign_id": foreign_id, "status": status,
                        "lineage_id": lineage_id, "branch_id": branch_id, "title": title,
                        "main_photo": shape.main_photo, "bare": shape.bare,
                    }));
                    (lineage_id, branch_id, title)
                }
                Ok(ImportOutcome::Offered {
                    lineage_id,
                    branch_id,
                    title,
                    candidate_version_id,
                }) => {
                    offered.push(json!({
                        "foreign_id": foreign_id,
                        "lineage_id": lineage_id, "branch_id": branch_id, "title": title,
                        "candidate_version_id": candidate_version_id,
                    }));
                    (lineage_id, branch_id, title)
                }
                Err(err) => {
                    unreadable.push(json!({
                        "foreign_id": foreign_id,
                        "reason": err.to_sentence(),
                    }));
                    continue;
                }
            };

            for left in &item.left_out {
                left_out.push(left.row(&foreign_id, &branch_id, &title));
            }
            landed.push(Relatable {
                lineage_id,
                branch_id,
                title,
                shape,
            });
        }

        if let Some(progress) = progress {
            progress.report(total, Some(total), "finished".to_string());
        }

        let related_candidates = self
            .db()
            .with_conn(|conn| related_candidates(conn, &cookbook_id, &landed))?;

        Ok(json!({
            "import_id": import_id,
            "cookbook_id": cookbook_id,
            "source_kind": source_kind,
            "arrived": arrived,
            "offered": offered,
            "unreadable": unreadable,
            "left_out": left_out,
            "related_candidates": related_candidates,
        }))
    }

    /// One candidate against the ledger: unseen becomes a new Lineage,
    /// Branch and first Version, the importing Person's Hand on it
    /// (GLOSSARY.md, "Hand"; ADR 0025). Seen before and now identical is
    /// `Unchanged`; seen before and now different records the candidate's
    /// content as a Version — content-addressed, so this never collides with
    /// or moves anything already on the Branch — and answers `Offered`
    /// without touching `head_version_id`: the offer is never written over
    /// the Branch on its own.
    ///
    /// Whatever the fate, the Branch is then filed under `tags`, English words
    /// in the Cookbook's own list (#128). Filing only adds, so a recipe matched
    /// again gains the tags it lacked and keeps every one given it here; and
    /// since a Tag is no part of a Version (ADR 0035), filing moves no id and
    /// leaves `unchanged` unchanged.
    #[allow(clippy::too_many_arguments)]
    fn import_one(
        &self,
        caller: &Caller,
        cookbook_id: &str,
        cookbook_hand_id: &str,
        import_id: &str,
        foreign_id: &str,
        candidate: &Value,
        tags: &[String],
    ) -> Result<ImportOutcome, OpError> {
        let content = parse_recipe_content(candidate)?;
        let title = content["title"].as_str().unwrap_or_default().to_string();
        let (version_id, content_text) = stored_version(&content);

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

            let outcome = match existing {
                None => {
                    let lineage_id = format!("l_{}", hex::encode(random_bytes(8)));
                    let branch_id = format!("b_{}", hex::encode(random_bytes(8)));
                    // An imported recipe has no Language to disagree with, so
                    // detection simply sets one (ADR 0025) — the same rule a
                    // recipe typed in by hand follows, through the same code.
                    let language = language_for_new_branch(
                        conn,
                        &caller.person_id,
                        candidate.get("language").and_then(Value::as_str),
                        &content,
                    )?;

                    insert_new_lineage_and_branch(
                        conn,
                        &lineage_id,
                        &branch_id,
                        cookbook_id,
                        cookbook_hand_id,
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
            }?;

            let (ImportOutcome::Landed { branch_id, .. }
            | ImportOutcome::Offered { branch_id, .. }) = &outcome;
            for name in tags {
                if let Some(tag_id) = arriving_tag(conn, cookbook_id, &[("en", name)])? {
                    file_branch_under(conn, branch_id, &tag_id)?;
                }
            }
            Ok(outcome)
        })
    }
}

/// One recipe as an importer hands it to [`Core::import_each`]: the candidate,
/// or why the importer could not read one, and what it read but deliberately
/// did not bring in, named in the Report's `left_out`.
pub struct Incoming {
    pub candidate: Result<Value, Unread>,
    pub left_out: Vec<LeftOut>,
    /// English words to file the recipe under in the Kitchen's own list: a
    /// Crouton library's tags (#128). Beside the candidate, not in it,
    /// because a Tag is filing, not content (ADR 0035).
    pub tags: Vec<String>,
}

/// Something an importer read and chose not to bring in (#69).
pub enum LeftOut {
    /// The source site's favicon — not a photograph of the dish (ADR 0017).
    /// Its bytes ride into the Report, and only there, so the Report can show
    /// what was left out; it is never stored as a Photograph.
    SiteIcon(Vec<u8>),
    /// Pictures past the first: a recipe keeps one Main Photo, and Kamosu does
    /// not guess which Step another one shows.
    ExtraPhotos(usize),
    /// Pictures that could not be decoded or remade.
    UnreadablePhotos(usize),
}

impl LeftOut {
    /// The Report's row for it, naming the recipe it was left out of.
    fn row(&self, foreign_id: &str, branch_id: &str, title: &str) -> Value {
        let (what, count) = match self {
            LeftOut::SiteIcon(_) => ("site_icon", 1),
            LeftOut::ExtraPhotos(count) => ("extra_photos", *count),
            LeftOut::UnreadablePhotos(count) => ("unreadable_photos", *count),
        };
        let mut row = json!({
            "foreign_id": foreign_id, "branch_id": branch_id, "title": title,
            "what": what, "count": count,
        });
        if let LeftOut::SiteIcon(bytes) = self
            && let Some(icon) = icon_data_uri(bytes)
        {
            row["icon"] = json!(icon);
        }
        row
    }
}

/// A favicon as a `data:` URI the Report can draw, or nothing when the bytes
/// are not a small picture in a format a browser shows. A site's icon is a
/// couple of kilobytes; anything past 64 KB is not one, and is not carried.
fn icon_data_uri(bytes: &[u8]) -> Option<String> {
    use base64::Engine;
    const MAX_ICON_BYTES: usize = 64 * 1024;
    let mime = match bytes {
        [0xff, 0xd8, 0xff, ..] => "image/jpeg",
        [0x89, b'P', b'N', b'G', ..] => "image/png",
        [b'G', b'I', b'F', b'8', ..] => "image/gif",
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'E',
            b'B',
            b'P',
            ..,
        ] => "image/webp",
        _ => return None,
    };
    (bytes.len() <= MAX_ICON_BYTES).then(|| {
        format!(
            "data:{mime};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        )
    })
}

/// A recipe an importer could not read at all. `name` is what the source
/// called it where its own id was unreadable too — a Crouton file's name.
pub struct Unread {
    pub foreign_id: Option<String>,
    pub name: Option<String>,
    pub reason: String,
}

impl Incoming {
    /// A candidate already read, with nothing left out of it.
    pub fn candidate(candidate: Value) -> Self {
        Incoming {
            candidate: Ok(candidate),
            left_out: Vec::new(),
            tags: Vec::new(),
        }
    }
}

/// What the Report says about a landed recipe beyond its name, read off the
/// candidate rather than the stored Version, which is the same content.
struct LandedShape {
    main_photo: Option<String>,
    /// Neither an Ingredient Line nor a Step: a name and, usually, a link —
    /// a real recipe and a real habit, reported as arriving, never as failing
    /// (ADR 0025).
    bare: bool,
    ingredients: usize,
    link: Option<String>,
}

impl LandedShape {
    fn of(candidate: &Value) -> Self {
        let count = |field: &str, kind: &str| {
            candidate[field].as_array().map_or(0, |items| {
                items.iter().filter(|item| item["kind"] == kind).count()
            })
        };
        let ingredients = count("ingredients", "ingredient");
        LandedShape {
            main_photo: candidate["main_photo"]
                .as_str()
                .filter(|photo| !photo.is_empty())
                .map(str::to_string),
            bare: ingredients == 0 && count("steps", "step") == 0,
            ingredients,
            link: candidate["source"]["link"]
                .as_str()
                .map(str::trim)
                .filter(|link| !link.is_empty())
                .map(str::to_string),
        }
    }
}

struct Relatable {
    lineage_id: String,
    branch_id: String,
    title: String,
    shape: LandedShape,
}

/// The name two recipes are compared by when offered as related: case and
/// spacing set aside, and a trailing `(Version 2)` — how a person marks a
/// second go at a dish in an app with nowhere else to put it.
fn relating_name(title: &str) -> String {
    let folded = title
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    if let Some(open) = folded.rfind(" (version ")
        && folded.ends_with(')')
    {
        let number = &folded[open + " (version ".len()..folded.len() - 1];
        if !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()) {
            return folded[..open].to_string();
        }
    }
    folded
}

/// Pairs among the recipes one Import landed that share a name or a web page,
/// offered as Related Recipe candidates (GLOSSARY.md, "Related Recipe"). A
/// site's name alone is not enough: nine Instagram recipes share one and have
/// nothing to do with each other. A pair already related is not offered again.
fn related_candidates(
    conn: &Connection,
    cookbook_id: &str,
    landed: &[Relatable],
) -> Result<Vec<Value>, OpError> {
    let mut pairs: Vec<((usize, usize), Vec<&'static str>)> = Vec::new();
    let mut offer = |a: usize, b: usize, shared: &'static str| match pairs
        .iter_mut()
        .find(|(pair, _)| *pair == (a, b))
    {
        Some((_, why)) => {
            if !why.contains(&shared) {
                why.push(shared);
            }
        }
        None => pairs.push(((a, b), vec![shared])),
    };
    let names: Vec<String> = landed.iter().map(|r| relating_name(&r.title)).collect();
    for a in 0..landed.len() {
        for b in a + 1..landed.len() {
            if landed[a].lineage_id == landed[b].lineage_id {
                continue;
            }
            if names[a] == names[b] {
                offer(a, b, "name");
            }
            if landed[a].shape.link.is_some() && landed[a].shape.link == landed[b].shape.link {
                offer(a, b, "page");
            }
        }
    }

    let mut offered = Vec::new();
    for ((a, b), shared) in pairs {
        let (first, second) = (&landed[a], &landed[b]);
        let (low, high) = if first.lineage_id < second.lineage_id {
            (&first.lineage_id, &second.lineage_id)
        } else {
            (&second.lineage_id, &first.lineage_id)
        };
        let already: bool = conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM related_recipes \
                  WHERE cookbook_id = ?1 AND lineage_a_id = ?2 AND lineage_b_id = ?3)",
                params![cookbook_id, low, high],
                |row| row.get(0),
            )
            .map_err(|e| OpError::internal(format!("cannot read Related Recipes: {e}")))?;
        if already {
            continue;
        }
        let side = |recipe: &Relatable| {
            json!({
                "lineage_id": recipe.lineage_id, "branch_id": recipe.branch_id,
                "title": recipe.title, "main_photo": recipe.shape.main_photo,
                "ingredients": recipe.shape.ingredients,
            })
        };
        offered.push(json!({ "recipes": [side(first), side(second)], "shared": shared }));
    }
    Ok(offered)
}

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

/// One arrival as the database hands it over, before it is placed under a
/// source (#108). Named rather than a six-wide tuple, because positional
/// fields of which four are `String` are how the wrong two get swapped.
struct ArrivalRow {
    job_id: String,
    operation: String,
    /// The `source_kind` this Job's own input declared, which only the general
    /// `import` carries — read with `json_extract` so a base64 export never
    /// leaves the database.
    declared: Option<String>,
    status: String,
    /// The Import Report, once there is one.
    result: Option<Value>,
    created_at: String,
}

/// Which source an arrival belongs to, answerable the moment the Job row
/// exists rather than only once it has finished (#108).
///
/// The Report states `source_kind` outright, and it is the authority, because
/// it is the very string the ledger row was opened under. But it exists only
/// once the work has finished, and an import still in flight is exactly the one
/// a screen most wants to place. So the Report is read where there is one and
/// the Job's own declaration otherwise: three importers name their source in
/// their own definition, and `import`, the general one, carries it in its input
/// where its schema requires it.
///
/// `Report.svelte` faces the same question and answers it the other way round,
/// preferring the Job's Operation over the Report — which is not a disagreement
/// but the same reasoning applied to a different need. It is deciding what to
/// call a page while the bar is still filling, so the earliest answer wins
/// outright. Here the row being placed is usually long finished, and where it
/// is not, this falls back to exactly what that screen reads.
fn arrival_source_kind(
    operation: &str,
    declared: Option<&str>,
    result: Option<&Value>,
) -> Option<String> {
    if let Some(kind) = result
        .and_then(|report| report.get("source_kind"))
        .and_then(Value::as_str)
    {
        return Some(kind.to_string());
    }
    match operation {
        "import_crouton" => Some("crouton".to_string()),
        "import_bundle" => Some("bundle".to_string()),
        "import_web_link" => Some("web".to_string()),
        "import" => declared.map(str::to_string),
        _ => None,
    }
}

/// One arrival as its row: when it happened, how it ended, and the three
/// counts that let a screen say what happened in a line without reading the
/// whole Report back. The Report itself is untouched and still reached by
/// `get_job` with `job_id` (ADR 0025) — these are a summary of it, never a
/// second copy of it.
fn arrival_summary(job_id: &str, status: &str, created_at: &str, result: Option<&Value>) -> Value {
    let arrived = result
        .and_then(|report| report.get("arrived"))
        .and_then(Value::as_array);
    let count = |key: &str| {
        result
            .and_then(|report| report.get(key))
            .and_then(Value::as_array)
            .map_or(0, Vec::len)
    };
    json!({
        "job_id": job_id,
        "status": status,
        "created_at": created_at,
        "arrived": arrived.map_or(0, Vec::len),
        "created": arrived.map_or(0, |rows| {
            rows.iter()
                .filter(|row| row["status"] == json!("created"))
                .count()
        }),
        "offered": count("offered"),
        "unreadable": count("unreadable"),
    })
}

/// An Import id that names nothing here — and, by ADR 0040, an Import of a
/// Cookbook the caller may not see.
fn no_such_import() -> OpError {
    OpError::not_found("no such Import")
}
