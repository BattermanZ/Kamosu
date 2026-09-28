//! Sheets: a recipe set as a printable PDF, for a Person or for a stranger
//! holding a Share Link.

use super::*;

/// Where a stranger's Sheet has got to, as the Share Link page reads it.
pub enum SheetState {
    /// Set and kept: the PDF, and the name a browser saves it under.
    Ready { name: String, bytes: Vec<u8> },
    /// Still waiting in its lane, or being set.
    Setting,
    /// It could not be made, and why.
    Failed(String),
}

impl Core {
    // ── Sheets (#75, ADR 0023) ───────────────────────────────────────────────

    /// **Set a Sheet of a Branch as this Person sees it** — the recipe as it
    /// stands on their screen, scaling included, on the paper their Reading
    /// Measures print on. Runs as a Job; the PDF waits under the Job's id to be
    /// fetched at `GET /api/sheets/<job_id>` under the same Credential.
    pub fn make_sheet(
        &self,
        person_id: &str,
        branch_id: &str,
        wanted: Option<&Value>,
        job: Option<&JobProgress>,
    ) -> Result<Value, OpError> {
        let named = wanted.map(parse_named_yield).transpose()?;
        let gathered = self.db().with_conn(|conn| {
            ensure_sees_branch(conn, branch_id, person_id)?;
            let reader = Reader::of(conn, person_id)?;
            let paper = sheet::Paper::for_measures(&reading_measures_of(conn, person_id)?);
            gather_sheet(
                conn,
                branch_id,
                &SheetReader::Person {
                    person_id,
                    reader: &reader,
                    wanted: named.as_ref(),
                },
                paper,
                None,
            )
        })?;
        self.set_sheet(gathered, job)
    }

    /// **Set a Sheet for a stranger holding a Share Link** (ADR 0023): the
    /// Branch the link shows, or one of its Translations where `language`
    /// names one, as written — a stranger is cooking nothing, so nothing is
    /// scaled but a Component, which is printed at the amount its line asks
    /// for. `locale` is the reader's browser locale and decides only the paper.
    pub fn make_shared_sheet(
        &self,
        token: &str,
        language: Option<&str>,
        locale: Option<&str>,
        job: Option<&JobProgress>,
    ) -> Result<Value, OpError> {
        let shared = self.read_shared_recipe(token)?;
        if shared["ended"].as_bool().unwrap_or(false) {
            return Err(OpError::not_found(
                "this Share Link was ended, so it no longer prints a Sheet",
            ));
        }
        // The same choice the page makes: a Translation where one is in the
        // Language asked for, and the Branch shared otherwise.
        let recipe = language
            .and_then(|language| {
                shared["translations"]
                    .as_array()?
                    .iter()
                    .find(|t| t["language"].as_str() == Some(language))
            })
            .unwrap_or(&shared["recipe"]);
        let branch_id = recipe["branch_id"]
            .as_str()
            .ok_or_else(|| OpError::internal("a shared recipe carries no Branch"))?
            .to_string();
        let share_url = shared["public_address"]
            .as_str()
            .map(|address| format!("{address}/s/{token}"));
        let gathered = self.db().with_conn(|conn| {
            let cookbook_id = branch_cookbook(conn, &branch_id)?;
            gather_sheet(
                conn,
                &branch_id,
                &SheetReader::Stranger {
                    cookbook_id: &cookbook_id,
                },
                sheet::Paper::for_locale(locale),
                share_url,
            )
        })?;
        self.set_sheet(gathered, job)
    }

    /// Word, set and keep one gathered Sheet.
    ///
    /// **A Sheet is remembered** (ADR 0032): the same page — the same Version,
    /// paper, scaling, Components and date printed — sets to the same bytes, so
    /// it is set once and kept under a key made from exactly that, and asking
    /// again is a file read. Kept Sheets are derived and swept after a day
    /// (`sheet::prune`), which is also when the date printed moves on.
    ///
    /// The database lock is not held here: setting is layout, and a reader has
    /// no reason to wait on it.
    fn set_sheet(
        &self,
        gathered: sheet::Gathered,
        job: Option<&JobProgress>,
    ) -> Result<Value, OpError> {
        let job_id = job
            .map(|job| job.job_id().to_string())
            .ok_or_else(|| OpError::internal("a Sheet is only ever set as a Job"))?;
        let main_photo = gathered.content["main_photo"].as_str().map(str::to_string);
        let mut document = sheet::compose(&gathered);

        let data_dir = self.data_dir();
        sheet::prune(&data_dir);
        let key = sheet::key_of(&document, main_photo.as_deref());
        let kept = match sheet::kept(&data_dir, &key) {
            Some(kept) => kept,
            None => {
                // A Main Photo whose picture cannot be read prints no strip
                // rather than failing the page: the recipe is what it is for.
                let photo = main_photo.as_deref().and_then(|hash| {
                    self.display_copy_bytes(hash, photographs::DisplaySize::Print)
                        .and_then(|copy| sheet::print_photo(&copy))
                        .map_err(|err| {
                            tracing::warn!("a Sheet prints without its photograph: {err}")
                        })
                        .ok()
                });
                if photo.is_none() {
                    document["recipe"]["photo"] = json!(false);
                }
                let set = sheet::typeset(&document, photo)?;
                sheet::keep(&data_dir, &key, &set)?
            }
        };

        let title = gathered.content["title"].as_str().unwrap_or("");
        Ok(json!({
            "file_name": sheet::file_name(title),
            "fetch_at": format!("/api/sheets/{job_id}"),
            "paper": gathered.paper.as_str(),
            "pages": kept.pages,
            "kept_as": kept.name,
        }))
    }

    /// **A finished Sheet's bytes**, for whoever may read its Job: the Person
    /// who asked, or anyone at all when a stranger did — a Sheet made through a
    /// Share Link carries nothing the link did not already show (`jobs::
    /// ensure_reader`), and one whose link has since ended is not handed over.
    pub fn read_sheet(
        &self,
        secret: Option<&str>,
        job_id: &str,
    ) -> Result<(String, Vec<u8>), OpError> {
        let caller = match secret.filter(|s| !s.is_empty()) {
            Some(secret) => Some(self.resolve_credential(secret)?),
            None => None,
        };
        let record = self.sheet_job(job_id)?;
        // This route resolved a **Sheet** id, so a Sheet that is not the
        // caller's answers word for word as an id naming no Sheet does — the
        // refusal `sheet_job` itself raises (ADR 0040).
        jobs::ensure_reader(&record, &caller, no_such_sheet)?;
        if record.operation == "make_shared_sheet" {
            self.ensure_link_live(record.input["token"].as_str().unwrap_or_default())?;
        }
        if record.status != jobs::JobStatus::Completed {
            return Err(OpError::not_found("that Sheet is not ready"));
        }
        self.sheet_bytes(&record)
    }

    /// **Where a stranger's Sheet has got to**, for the Share Link page, which
    /// has no script and so asks this again until it is ready. Only a Sheet
    /// made through this very link answers, and only while the link is live.
    pub fn shared_sheet(&self, token: &str, job_id: &str) -> Result<SheetState, OpError> {
        self.ensure_link_live(token)?;
        let record = self
            .sheet_job(job_id)
            .ok()
            .filter(|record| {
                record.operation == "make_shared_sheet"
                    && record.input["token"].as_str() == Some(token)
            })
            .ok_or_else(no_such_sheet)?;
        Ok(match record.status {
            jobs::JobStatus::Completed => {
                let (name, bytes) = self.sheet_bytes(&record)?;
                SheetState::Ready { name, bytes }
            }
            jobs::JobStatus::Failed | jobs::JobStatus::Cancelled => {
                SheetState::Failed(record.error.unwrap_or_default())
            }
            jobs::JobStatus::Queued | jobs::JobStatus::Running => SheetState::Setting,
        })
    }

    fn sheet_job(&self, job_id: &str) -> Result<JobRecord, OpError> {
        self.job(job_id)?
            .filter(|record| {
                matches!(
                    record.operation.as_str(),
                    "make_sheet" | "make_shared_sheet"
                )
            })
            .ok_or_else(no_such_sheet)
    }

    fn sheet_bytes(&self, record: &JobRecord) -> Result<(String, Vec<u8>), OpError> {
        let result = record.result.as_ref();
        let kept_as = result
            .and_then(|result| result["kept_as"].as_str())
            .unwrap_or_default();
        let bytes = sheet::read_kept(&self.data_dir(), kept_as).ok_or_else(|| {
            OpError::not_found("that Sheet is no longer kept — ask for a new one")
        })?;
        let name = result
            .and_then(|result| result["file_name"].as_str())
            .unwrap_or("Recipe.pdf")
            .to_string();
        Ok((name, bytes))
    }

    /// Refuse a Share Link that was ended: it stops new people arriving, and
    /// fetching a Sheet through it is arriving (ADR 0018).
    fn ensure_link_live(&self, token: &str) -> Result<(), OpError> {
        let shared = self.read_shared_recipe(token)?;
        if shared["ended"].as_bool().unwrap_or(false) {
            return Err(OpError::not_found(
                "this Share Link was ended, so it no longer prints a Sheet",
            ));
        }
        Ok(())
    }
}

/// **Who a Sheet is being set for** (ADR 0023), which decides the two things
/// that differ: which Branch of a Component is in view, and what "as it stands
/// on screen" means for scaling.
enum SheetReader<'a> {
    /// A Person, whose screen is scaled to the Yield they are cooking and
    /// whose Components resolve against every Kitchen they cook in.
    Person {
        person_id: &'a str,
        reader: &'a Reader,
        /// The Yield the recipe page is scaled to, where it named one (#109).
        wanted: Option<&'a Value>,
    },
    /// A stranger holding a Share Link: cooking nothing, and reading the
    /// Components the sharing Cookbook holds.
    Stranger { cookbook_id: &'a str },
}

/// **Everything one Sheet carries**, read from the Branch's head Version.
///
/// A Sheet carries the recipe, not the library (ADR 0023), and this is where
/// that is true by construction: it reads the content, the Components, and the
/// few facts the provenance block names — and nothing about Tags, Attempts or
/// the Thread, because none of those is asked for.
fn gather_sheet(
    conn: &Connection,
    branch_id: &str,
    who: &SheetReader<'_>,
    paper: sheet::Paper,
    share_url: Option<String>,
) -> Result<sheet::Gathered, OpError> {
    let (lineage_id, language, head_version_id): (String, String, String) = conn
        .query_row(
            "SELECT lineage_id, language, head_version_id FROM branches WHERE id = ?1",
            params![branch_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?
        .ok_or_else(no_such_branch)?;
    let content: String = conn
        .query_row(
            "SELECT content FROM versions WHERE id = ?1",
            params![head_version_id],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot read a Version: {e}")))?;
    let content: Value = serde_json::from_str(&content)
        .map(content_as_declared)
        .map_err(|e| OpError::internal(format!("a Version's content is unreadable: {e}")))?;
    let (version_name, written_at, hand_id): (Option<String>, String, String) = conn
        .query_row(
            "SELECT name, created_at, hand_id FROM branch_versions \
              WHERE branch_id = ?1 AND version_id = ?2 ORDER BY sequence DESC LIMIT 1",
            params![branch_id, head_version_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| OpError::internal(format!("cannot read the Version's record: {e}")))?;
    let today: String = conn
        .query_row("SELECT date('now')", [], |row| row.get(0))
        .map_err(|e| OpError::internal(format!("cannot read today's date: {e}")))?;

    // A stranger reads in the recipe's own measures: Kamosu converts to a
    // kitchen, and a stranger has none (ADR 0016). Only a Component's scaled
    // amount can reach their page, so that is all this is ever used for.
    let stranger_reader = Reader {
        language: language.clone(),
        measures: units::Measures::AsWritten,
    };
    let (scale, unfolds) = match who {
        SheetReader::Person {
            person_id,
            reader,
            wanted,
        } => (
            yield_scale(
                &wanted_yield(conn, &lineage_id, person_id, *wanted)?,
                &content["yield"],
            ),
            Unfolds::ForReader { person_id, reader },
        ),
        SheetReader::Stranger { cookbook_id } => (
            1.0,
            Unfolds::PrintedPassenger {
                cookbook_id,
                reader: &stranger_reader,
            },
        ),
    };

    // Printed as it stands on screen, scaling included — and a scaled Sheet is
    // the only one with anything beneath a line (ADR 0023). A recipe read at
    // the Yield as written prints its lines and nothing else, even for a
    // reader whose screen converts them: paper is the worst place to rewrite
    // what a cook wrote.
    let scaled = sheet::scales(scale);
    let (about, scaled_yields) = match who {
        SheetReader::Person {
            person_id,
            reader,
            wanted,
        } if scaled => {
            let measured = measured_for_version(conn, &content, &head_version_id, reader, scale)?;
            let about = measured["ingredients"]
                .as_array()
                .map_or(Vec::new(), |slots| {
                    slots
                        .iter()
                        .map(|slot| slot.as_str().map(str::to_string))
                        .collect()
                });
            let wanted = wanted_yield(conn, &lineage_id, person_id, *wanted)?;
            (about, Some((wanted, content["yield"].clone())))
        }
        _ => (Vec::new(), None),
    };

    let mut walk = Unfolding {
        open: vec![lineage_id.clone()],
        ..Unfolding::default()
    };
    unfold_components(conn, &unfolds, &head_version_id, scale, &mut walk)?;
    // Which Language each Component is held in, so the page can mark one that
    // is not its own (ADR 0023).
    for component in &mut walk.found {
        if let Some(branch_id) = component["branch_id"].as_str() {
            let language: Option<String> = conn
                .query_row(
                    "SELECT language FROM branches WHERE id = ?1",
                    params![branch_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| {
                    OpError::internal(format!("cannot read a Component's Language: {e}"))
                })?;
            component["language"] = json!(language);
        }
    }

    Ok(sheet::Gathered {
        language,
        content,
        about,
        scaled: scaled_yields,
        components: walk.found,
        version_name,
        written_at,
        hand: person_name(conn, &hand_id)?,
        fingerprint: head_version_id,
        share_url,
        paper,
        today,
    })
}

/// A Sheet id that names nothing here — and, by ADR 0040, a Sheet belonging to
/// somebody else. `/api/sheets/<id>` resolved a Sheet id, so this is the answer
/// it owes in both cases; saying *no Job with that id* instead would tell the
/// caller the id names a Job that is not theirs.
fn no_such_sheet() -> OpError {
    OpError::not_found("no Sheet with that id")
}
