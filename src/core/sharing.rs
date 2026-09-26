//! Sharing: Share Links, the Sheet, and Bundles in both directions.

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
    // ── Share Links (#65, ADR 0026, ADR 0018) ─────────────────────────────
    //
    // There are exactly two levels of visibility: a recipe is seen by its
    // Kitchen, or by anyone holding its Share Link. So there is no visibility
    // setting to read or write anywhere below — a live row in `share_links`
    // *is* the second level.

    /// Turn a recipe's Share Link on, and answer the link.
    ///
    /// Idempotent: a recipe already shared answers the link it already has,
    /// because "share this" asked twice is one intention, not two. Minting a
    /// second live link would be indistinguishable from the first and would
    /// leave the earlier one working, which is precisely what ADR 0018 forbids
    /// — a link that is ended must stay dead.
    ///
    /// `address` is the instance's public address, offered here because the
    /// first Share Link is the first moment Kamosu has any reason to know it
    /// (#65). It is stored once and then ignored on every later call.
    pub fn share_recipe(
        &self,
        person_id: &str,
        branch_id: &str,
        address: Option<&str>,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            ensure_sees_branch(conn, branch_id, person_id)?;

            if let Some(offered) = address {
                let offered = normalise_public_address(offered)?;
                if stored_public_address(conn)?.is_none() {
                    store_public_address(conn, &offered)?;
                }
            }

            if let Some(live) = live_share_link(conn, branch_id)? {
                return share_link_summary(conn, Some(live));
            }

            // The address is required at the first Share Link and never
            // after, because a link is stored as a token: this instance goes
            // on serving every link it ever minted whatever address reaches
            // it. That is not the same as a link already *sent* following a
            // change of address — it cannot, and `set_public_address` says why
            // (#103).
            if stored_public_address(conn)?.is_none() {
                return Err(OpError::bad_request(
                    "this instance has no public address yet, and a Share Link needs one to be \
                     an address somebody can open. Send it with this Operation as \
                     `public_address` — it is stored once, and every link minted \
                     afterwards is built against it.",
                ));
            }

            let secret = generate_secret();
            let id = format!("sl_{}", hex::encode(random_bytes(8)));
            // The Secret is kept beside its hash, so the share screen can show
            // the address for as long as the link lives (#171, ADR 0031 as
            // amended). A visitor is still looked up by the hash alone, which
            // is what keeps a link minted before the Secret was kept opening.
            conn.execute(
                "INSERT INTO share_links (id, branch_id, secret_hash, secret, shared_by) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, branch_id, hash_secret(&secret), secret, person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot mint a Share Link: {e}")))?;

            let live = live_share_link(conn, branch_id)?;
            share_link_summary(conn, live)
        })
    }

    /// End a recipe's Share Link. Permanent: the row stays, dead, and turning
    /// sharing back on mints a new one (ADR 0018).
    pub fn end_share_link(&self, person_id: &str, branch_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            ensure_sees_branch(conn, branch_id, person_id)?;
            conn.execute(
                "UPDATE share_links SET ended_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
                  WHERE branch_id = ?1 AND ended_at IS NULL",
                params![branch_id],
            )
            .map_err(|e| OpError::internal(format!("cannot end the Share Link: {e}")))?;
            share_link_summary(conn, None)
        })
    }

    /// What the share screen reads: whether this recipe is shared, and where.
    pub fn get_share_link(&self, person_id: &str, branch_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            ensure_sees_branch(conn, branch_id, person_id)?;
            let live = live_share_link(conn, branch_id)?;
            share_link_summary(conn, live)
        })
    }

    /// A Photograph on a shared recipe, for a reader holding the token and no
    /// Credential at all.
    ///
    /// The check is here rather than in the route, like every other check in
    /// Kamosu: a live token is a key to **this recipe's** pictures and not to
    /// the instance's. What counts as this recipe's is its Main Photo and the
    /// Photograph on any of its Steps, on the Version being shown — which is
    /// exactly the set the page can put an `<img>` around.
    pub fn read_shared_photograph(
        &self,
        token: &str,
        hash: &str,
        size: photographs::DisplaySize,
    ) -> Result<Vec<u8>, OpError> {
        let shared = self.read_shared_recipe(token)?;
        if shared["ended"].as_bool().unwrap_or(false) {
            return Err(OpError::not_found("no such Share Link"));
        }
        let carried = std::iter::once(&shared["recipe"])
            .chain(shared["translations"].as_array().into_iter().flatten())
            .any(|version| version_shows_photograph(version, hash));
        if !carried {
            return Err(OpError::not_found(
                "this Photograph is not on this shared Recipe",
            ));
        }
        self.display_copy_bytes(hash, size)
    }

    /// The picture a messaging app shows for a Share Link (#65).
    ///
    /// **Rendered once and kept** (ADR 0032). This is the one piece of real
    /// work a stranger can cause on this instance: sending a link into a busy
    /// group chat asks a dozen clients for the same picture at once, and
    /// rasterising a 1200×628 card each time is work that scales with the
    /// strangers rather than with the library.
    ///
    /// The cache is keyed by **Version id**, which is exactly right and needs
    /// no sweeping: a Version is named by a fingerprint of its own words and
    /// Photographs (ADR 0004), so the card for one can never need redrawing,
    /// and a recipe edited after being shared simply asks for a different key.
    /// Losing the directory costs one redraw.
    ///
    /// It draws from the already-resized Display Copy rather than a camera
    /// original, so even the first miss is bounded work.
    pub fn shared_card(&self, token: &str) -> Result<Vec<u8>, OpError> {
        let shared = self.read_shared_recipe(token)?;
        if shared["ended"].as_bool().unwrap_or(false) {
            return Err(OpError::not_found("no such Share Link"));
        }
        let recipe = &shared["recipe"];
        let version_id = recipe["version_id"].as_str().unwrap_or_default();
        let cached = self
            .data_dir()
            .join("cards")
            .join(format!("{version_id}.png"));
        if let Ok(bytes) = std::fs::read(&cached) {
            return Ok(bytes);
        }

        let title = recipe["content"]["title"].as_str().unwrap_or("");
        let lineage_id = recipe["lineage_id"].as_str().unwrap_or("");
        let photograph = match recipe["content"]["main_photo"].as_str() {
            Some(hash) => Some(self.display_copy_bytes(hash, photographs::DisplaySize::Print)?),
            None => None,
        };
        let drawn = crate::share_card::draw(
            title,
            photograph.as_deref(),
            &crate::cover::cover_for(lineage_id),
        )?;

        // A card that cannot be cached is still a card: the drawing succeeded,
        // and refusing to serve it because a directory is unwritable would
        // turn a disk problem into a broken link in somebody's chat.
        if let Some(directory) = cached.parent()
            && std::fs::create_dir_all(directory).is_ok()
        {
            let _ = std::fs::write(&cached, &drawn);
        }
        Ok(drawn)
    }

    /// What is stored, or `null` where nothing is (#103). Null is an ordinary
    /// answer rather than an error: an instance that has never minted a Share
    /// Link has never been asked, and that is the state most instances are in.
    pub fn get_public_address(&self) -> Result<Value, OpError> {
        self.db()
            .with_conn(|conn| Ok(json!({ "public_address": stored_public_address(conn)? })))
    }

    /// The instance's public address, set by the Operator.
    ///
    /// Separate from `share_recipe`'s one-time offer because moving an instance
    /// to a new address is a deliberate act taken long after the first link.
    ///
    /// **It fixes the future, not the past** (#103). This said the opposite
    /// until then — that every link already minted "must follow it, which is
    /// exactly what storing a token rather than a URL buys" — and that is
    /// false. A Share Link is `<address>/s/<secret>`, and a visitor is looked
    /// up by the secret's hash alone (`read_shared_recipe`), the address never
    /// entering the lookup. So a link already sent is a string in somebody
    /// else's phone that nothing here can reach, and if the old address stops
    /// resolving it is dead. What storing a token rather than a URL buys is
    /// that the *instance* keeps serving every link it ever minted, at whatever
    /// address reaches it — not that a link already handed out changes its
    /// own text.
    ///
    /// What does change is the address the share screen shows. Since #171 a
    /// link keeps its secret, and `share_link_summary` builds the URL against
    /// the address stored now, so after a move the screen offers the address
    /// that opens, ready to send again. That is intended (ADR 0031).
    pub fn set_public_address(&self, address: &str) -> Result<Value, OpError> {
        let address = normalise_public_address(address)?;
        self.db().with_conn(|conn| {
            store_public_address(conn, &address)?;
            Ok(json!({ "public_address": address }))
        })
    }

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

    /// Everything the public Share Link page shows, for a stranger holding a
    /// token and no Credential at all.
    ///
    /// **Never any Attempt** (ADR 0005, ADR 0026): not a rating, not a note,
    /// not an Attempt photograph. That is not a filter applied here so much as
    /// a shape — nothing below reads the `attempts` table.
    ///
    /// The chain is complete back to the first Version, names and *what
    /// changed* lines included, because a share cannot be made to start
    /// part-way along (ADR 0018). There is no parameter here that could
    /// shorten it.
    pub fn read_shared_recipe(&self, token: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let found: Option<(String, String, String, Option<String>)> = conn
                .query_row(
                    "SELECT id, branch_id, shared_by, ended_at FROM share_links \
                      WHERE secret_hash = ?1",
                    params![hash_secret(token)],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read the Share Link: {e}")))?;

            // A token nobody ever minted is not found. A token whose link was
            // *ended* is a different answer and is deliberately not an error:
            // somebody holding a real link that has since been withdrawn is
            // owed "this was ended", because "no such page" reads as a mistake
            // they might keep retrying. Enumeration is answered by the size of
            // the Secret (ADR 0031, ADR 0032), never by being coy about a
            // token the asker demonstrably already holds.
            let Some((share_id, branch_id, shared_by, ended_at)) = found else {
                return Err(OpError::not_found("no such Share Link"));
            };
            if ended_at.is_some() {
                return Ok(json!({
                    "ended": true,
                    "share_id": share_id,
                    "shared_by": Value::Null,
                    "public_address": stored_public_address(conn)?,
                    "recipe": Value::Null,
                    "translations": [],
                    "thread": [],
                }));
            }

            let shared = shared_branch(conn, &branch_id)?;
            Ok(json!({
                "ended": false,
                "share_id": share_id,
                // The page says who shared it, and a Person is who that is —
                // never the Kitchen. The name is looked up live, so renaming
                // yourself reaches every link you have ever minted at once
                // (CONTEXT.md, "Hand").
                "shared_by": person_name(conn, &shared_by)?,
                "public_address": stored_public_address(conn)?,
                "recipe": shared["recipe"].clone(),
                "translations": shared["translations"].clone(),
                "thread": shared["thread"].clone(),
            }))
        })
    }

    // ── Bundles (#66, ADR 0020) ──────────────────────────────────────────────

    /// **Write one Bundle** of the Branch a Person names: that Branch, its
    /// Translations, every Component it needs as a Passenger, and every
    /// Photograph any of their Versions shows.
    ///
    /// Nothing is written anywhere, and nothing about who can see the
    /// Passengers changes: they travel because the recipe needs them (ADR
    /// 0008), and exporting is a read of the library, not a change to it.
    pub fn bundle(&self, person_id: &str, branch_id: &str) -> Result<bundles::Written, OpError> {
        let (mut contents, hashes) = self.gather_bundle(person_id, branch_id)?;
        self.fill_photographs(&mut contents, hashes);
        bundles::write(&contents)
    }

    /// Put every Photograph a gathered Bundle shows into it.
    ///
    /// Read off disk outside the database lock: a Photograph is a file, and
    /// there is no reason to hold every other request up while it loads. One
    /// that has gone missing does not cost the recipe — the note still reads —
    /// but the Bundle says so rather than being quietly short.
    fn fill_photographs(&self, contents: &mut bundles::Contents, hashes: Vec<String>) {
        for hash in hashes {
            match self.photograph_bytes(&hash) {
                Ok(bytes) => contents.photographs.push((hash, bytes)),
                Err(_) => contents.missing_photographs.push(hash),
            }
        }
    }

    /// Which of a Bundle's Photographs this instance still holds, without
    /// reading one: the answer `export_bundle` describes a Bundle by, and the
    /// answer a kept Bundle is keyed on.
    fn split_photographs(&self, hashes: Vec<String>) -> (Vec<String>, Vec<String>) {
        let data_dir = self.data_dir();
        hashes
            .into_iter()
            .partition(|hash| photographs::photograph_path(&data_dir, hash).is_file())
    }

    /// What `export_bundle` answers: the Bundle described, and where to fetch
    /// its bytes. Described from what would be gathered rather than by
    /// building it, so asking costs a read of the database and not a zip of
    /// every Photograph; the bytes are built once, when they are fetched.
    pub fn export_bundle(&self, person_id: &str, branch_id: &str) -> Result<Value, OpError> {
        let (contents, hashes) = self.gather_bundle(person_id, branch_id)?;
        let (present, missing) = self.split_photographs(hashes);
        let described = |subject: bool| -> Vec<Value> {
            let mut seen = HashSet::new();
            contents
                .branches
                .iter()
                .map(|carried| &carried.record)
                .filter(|record| {
                    let about = record["lineage_id"]
                        .as_str()
                        .is_some_and(|lineage| contents.subjects.iter().any(|s| s == lineage));
                    about == subject
                })
                .filter(|record| {
                    seen.insert(record["lineage_id"].as_str().unwrap_or("").to_string())
                })
                .map(|record| {
                    json!({
                        "lineage_id": record["lineage_id"],
                        "title": bundles::title_of(record),
                    })
                })
                .collect()
        };
        Ok(json!({
            "file_name": bundles::file_name(&contents),
            "fetch_at": format!("/api/bundles/{branch_id}"),
            "subjects": described(true),
            "passengers": described(false),
            "notes": bundles::note_names(&contents.branches),
            "photographs": present.len(),
            "missing_photographs": missing,
        }))
    }

    /// **The Bundle a Share Link hands over** (#65, #66, ADR 0020, ADR 0032).
    ///
    /// The same zip `export_bundle` describes, for a stranger who holds the
    /// token and no Credential at all. Holding the link is the whole of the
    /// permission, so the Branch is gathered against the **sharing Kitchen**,
    /// which is the rule the page itself already reads by.
    ///
    /// A link that was ended hands over nothing. It is the same refusal a Sheet
    /// gets: withdrawing a link stops anyone new arriving, and taking the
    /// recipe away is arriving (ADR 0018).
    ///
    /// One zip per recipe rather than one per Translation. A Bundle carries
    /// every Translation and every Passenger whichever page you asked from, so
    /// reading a share in French and taking the file is the same file.
    pub fn shared_bundle(&self, token: &str) -> Result<bundles::Written, OpError> {
        let shared = self.read_shared_recipe(token)?;
        if shared["ended"].as_bool().unwrap_or(false) {
            return Err(OpError::not_found(
                "this Share Link was ended, so it no longer hands over a recipe file",
            ));
        }
        let branch_id = shared["recipe"]["branch_id"]
            .as_str()
            .ok_or_else(|| OpError::internal("a shared recipe carries no Branch"))?
            .to_string();
        let (mut contents, hashes) = self.db().with_conn(|conn| {
            let cookbook_id = branch_cookbook(conn, &branch_id)?;
            bundle_contents(conn, &branch_id, &cookbook_id)
        })?;

        let data_dir = self.data_dir();
        let (present, missing) = self.split_photographs(hashes);
        bundles::prune(&data_dir);
        let key = bundles::key_of(&contents, &present, &missing);
        let file_name = bundles::file_name(&contents);
        if let Some(bytes) = bundles::kept(&data_dir, &key) {
            return Ok(bundles::Written { file_name, bytes });
        }

        contents.missing_photographs = missing.clone();
        self.fill_photographs(&mut contents, present);
        let written = bundles::write(&contents)?;
        // Keep only what the key actually describes. The key is made from a
        // Photograph's *presence* on disk, which is cheap; the zip is made from
        // reading it, which can still fail. Keeping a short Bundle under a key
        // that promised a whole one would serve the shortfall for a day.
        if contents.missing_photographs == missing {
            bundles::keep(&data_dir, &key, &written.bytes);
        }
        Ok(written)
    }

    /// Everything one Bundle carries but the Photographs' bytes, for a Person
    /// who may see the Branch — the same circle that may read it, and the one
    /// `share_recipe` lets mint a link to it.
    fn gather_bundle(
        &self,
        person_id: &str,
        branch_id: &str,
    ) -> Result<(bundles::Contents, Vec<String>), OpError> {
        self.db().with_conn(|conn| {
            let cookbook_id = ensure_sees_branch(conn, branch_id, person_id)?;
            bundle_contents(conn, branch_id, &cookbook_id)
        })
    }

    /// **Receive a Bundle** into the caller's own Cookbook (#67, ADR 0020,
    /// ADR 0041).
    ///
    /// Every Branch the Bundle carries is placed under the sender's Lineage id
    /// and Hands — held here, written by them — travelling on under the
    /// sender's Branch id, and every Version, Reading and Photograph arrives as
    /// it was sent, never recomputed. The row itself takes a local id of this
    /// instance's own (#90), which is what the Report names it by and what
    /// every Operation and URL here asks for. A Branch this Kitchen already
    /// holds is extended by what the Bundle carries past it, which is how the
    /// same friend's second Bundle continues their recipe rather than lining up
    /// a third. Another Kitchen here holding the sender's Branch is no part of
    /// the question: each household receives its own copy. Receiving makes
    /// nothing of your own: changing what arrived is what starts your Branch
    /// (`cookbook_writes_branch`).
    ///
    /// A Branch whose history is damaged keeps the dinner and loses where it
    /// came from: its words arrive as a new recipe of the caller's own, with
    /// no history and no Lineage id, and the Report says all three. One
    /// damaged Branch costs only itself. A file whose sidecar is missing or
    /// unreadable is the same answer reached by a different road (ADR 0020):
    /// each of its notes is read back into a recipe of the caller's own.
    ///
    /// The answer is the Import Report every importer answers (ADR 0025), each
    /// carried Branch named in it by the Branch id it carried. A damaged one
    /// is one row, in `unreadable`, pointing at the recipe its words became.
    pub fn import_bundle(
        &self,
        caller: &Caller,
        bytes: &[u8],
        progress: Option<&JobProgress>,
    ) -> Result<Value, OpError> {
        let (cookbook_id, import_id) = self.db().with_conn(|conn| {
            let cookbook_id = cookbook_of_person(conn, &caller.person_id)?;
            let import_id = find_or_create_import(conn, &cookbook_id, "bundle")?;
            Ok((cookbook_id, import_id))
        })?;
        let mut arrived: Vec<Value> = Vec::new();
        let mut unreadable: Vec<Value> = Vec::new();
        let report = |arrived: Vec<Value>, unreadable: Vec<Value>| {
            json!({
                "import_id": import_id,
                "cookbook_id": cookbook_id,
                "source_kind": "bundle",
                "arrived": arrived,
                "offered": [],
                "unreadable": unreadable,
                "left_out": [],
                "related_candidates": [],
            })
        };

        let opened = match bundles::open(bytes) {
            Ok(opened) => opened,
            Err(unopened) if unopened.notes.is_empty() => {
                unreadable.push(json!({ "foreign_id": Value::Null, "reason": unopened.reason }));
                return Ok(report(arrived, unreadable));
            }
            Err(unopened) => {
                for (note, content) in &unopened.notes {
                    let why = &unopened.reason;
                    let fate = self.db().with_conn(|conn| {
                        keep_words_as_new_recipe(
                            conn,
                            caller,
                            &cookbook_id,
                            content,
                            None,
                            |named| {
                                format!(
                                    "{named} was read from its note alone, because this Bundle's \
                                 machine half could not be used ({why}). Its words were kept as \
                                 a new recipe of your own, with no history, no photographs and \
                                 no link to the recipe it came from."
                                )
                            },
                        )
                    })?;
                    unreadable.push(fate.row(&json!(note), false));
                }
                return Ok(report(arrived, unreadable));
            }
        };

        // The Photographs first, and off the database lock: each is a file.
        // Stored exactly as they arrived, since a Photograph from another
        // instance is already made (ADR 0017).
        for photograph in opened.photographs {
            match photograph {
                bundles::CarriedPhotograph::Sound { hash, bytes } => {
                    // Two different failures, kept apart on purpose. A picture
                    // that is not one, or that claims a size Kamosu will not
                    // open, is left out and named: losing a picture costs a
                    // picture, while losing a recipe is discovered weeks later
                    // looking for a dish you were sure you had. A disk that
                    // will not take the bytes is not that, and must not be
                    // reported to an Operator as a bad picture.
                    match photographs::check(&bytes) {
                        Err(why) => unreadable.push(json!({
                            "foreign_id": hash,
                            "reason": format!(
                                "{}, so it was left out; the recipe shows no picture there",
                                why.message
                            ),
                        })),
                        Ok(()) => {
                            self.store_photograph_verbatim(&bytes)?;
                        }
                    }
                }
                bundles::CarriedPhotograph::Unusable { name, reason } => {
                    unreadable.push(json!({ "foreign_id": name, "reason": reason }));
                }
            }
        }

        let subjects = subjects_of(&opened.sidecar);
        let records = opened.sidecar["branches"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let total = records.len() as u64;

        // Which Versions this instance held before anything arrived: a
        // Version already held keeps its own Readings (ADR 0021).
        let held_before = self.db().with_conn(|conn| {
            let mut held_before: HashSet<String> = HashSet::new();
            // Every sound Version's words go in before any Branch is placed, so
            // a Translation can point at the Versions of what it translates
            // wherever in the Bundle that sits. A Version is its content and
            // nothing else, so storing one places nothing and says nothing
            // about who holds it.
            for record in records.iter().filter(|r| bundles::damage(r).is_none()) {
                for version in record["versions"].as_array().into_iter().flatten() {
                    let (id, text) = stored_version(&version["content"]);
                    let held: bool = conn
                        .query_row(
                            "SELECT COUNT(*) > 0 FROM versions WHERE id = ?1",
                            params![id],
                            |row| row.get(0),
                        )
                        .map_err(|e| OpError::internal(format!("cannot read Version: {e}")))?;
                    if held {
                        held_before.insert(id);
                    } else {
                        conn.execute(
                            "INSERT INTO versions (id, content) VALUES (?1, ?2)",
                            params![id, text],
                        )
                        .map_err(|e| OpError::internal(format!("cannot record Version: {e}")))?;
                    }
                }
            }
            Ok(held_before)
        })?;

        for (done, record) in records.iter().enumerate() {
            // Reported between Branches and never under the lock: a Job's
            // progress is itself written to the database.
            if let Some(progress) = progress {
                progress.report(done as u64, Some(total), format!("{done} of {total}"));
            }
            let foreign_id = record["branch_id"].clone();
            let subject = is_subject(record, &subjects);

            // Each Branch in a transaction of its own: one that cannot be
            // placed leaves nothing half-written, and costs nothing else.
            let fate = self.db().with_conn(|conn| {
                let transaction = conn
                    .unchecked_transaction()
                    .map_err(|e| OpError::internal(format!("cannot begin: {e}")))?;
                let fate = match bundles::damage(record) {
                    None => place_carried_branch(conn, &cookbook_id, caller, record, &held_before)?,
                    Some(damage) => {
                        let language = record["language"].as_str();
                        let words = bundles::head(record).map(|version| &version["content"]);
                        match words {
                            Some(content) => keep_words_as_new_recipe(
                                conn,
                                caller,
                                &cookbook_id,
                                content,
                                language,
                                |named| {
                                    format!(
                                        "The history of {named} was refused, because {damage}. \
                                         Its words were kept as a new recipe of your own, with no \
                                         history and no link to the recipe it came from."
                                    )
                                },
                            )?,
                            None => Fate::Refused(format!(
                                "a recipe in this Bundle arrived damaged — {damage} — and \
                                 carried no words to keep, so nothing of it was kept"
                            )),
                        }
                    }
                };
                if !matches!(fate, Fate::Refused(_)) {
                    transaction
                        .commit()
                        .map_err(|e| OpError::internal(format!("cannot commit: {e}")))?;
                }
                Ok(fate)
            })?;
            match fate {
                Fate::Placed { .. } => arrived.push(fate.row(&foreign_id, subject)),
                _ => unreadable.push(fate.row(&foreign_id, subject)),
            }
        }

        if let Some(progress) = progress {
            progress.report(total, Some(total), "finished".to_string());
        }
        Ok(report(arrived, unreadable))
    }

    /// **What importing a shared recipe would do**, said before anything is
    /// written (#170, Aurélien's choices 1 and 3 of 26 September 2026).
    ///
    /// Reads the Bundle a Share Link served and answers the recipe it is about
    /// — its title, its Source, who wrote it, how many Versions it carries, a
    /// small picture — and whether the caller's own Cookbook already holds it,
    /// with how many newer Versions the file carries past what is held. That
    /// last is [`place_carried_branch`]'s own test run without writing: held is
    /// the travelling id in this Cookbook, and newer is the file's chain
    /// running on past the held one.
    ///
    /// The file is staged as the caller's upload, so importing is
    /// `import_bundle` with the id this answers: the recipe is fetched once,
    /// and what is imported is exactly what was previewed. An upload nobody
    /// imports is swept after a day like any other.
    ///
    /// The picture travels as a `data:` address, since the file's Photographs
    /// are stored nowhere until it is imported and a stranger's instance is
    /// no address the app may load an image from (#139).
    pub fn preview_shared_bundle(
        &self,
        caller: &Caller,
        bytes: &[u8],
        shared_by: Option<&str>,
    ) -> Result<Value, OpError> {
        let opened = bundles::open(bytes).map_err(|unopened| {
            OpError::bad_request(format!(
                "this link did not hand over a recipe Kamosu can read: {}",
                unopened.reason
            ))
        })?;
        let subjects = subjects_of(&opened.sidecar);
        let records = opened.sidecar["branches"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default();
        let record = records
            .iter()
            .find(|record| is_subject(record, &subjects))
            .or_else(|| records.first())
            .ok_or_else(|| {
                OpError::bad_request("this link did not hand over a recipe: its file carries none")
            })?;

        let content = bundles::head(record)
            .map(|version| &version["content"])
            .unwrap_or(&Value::Null);
        let versions = record["versions"].as_array().map_or(0, Vec::len);
        let source = match (
            text_at(&content["source"], "text"),
            text_at(&content["source"], "link"),
        ) {
            (None, None) => Value::Null,
            (text, link) => json!({ "text": text, "link": link }),
        };
        let photo = content["main_photo"].as_str().and_then(|wanted| {
            opened
                .photographs
                .iter()
                .find_map(|photograph| match photograph {
                    bundles::CarriedPhotograph::Sound { hash, bytes } if hash == wanted => {
                        photographs::check(bytes).ok()?;
                        let small = photographs::display_copy(bytes, PREVIEW_PICTURE_EDGE).ok()?;
                        use base64::Engine;
                        Some(format!(
                            "data:image/webp;base64,{}",
                            base64::engine::general_purpose::STANDARD.encode(small)
                        ))
                    }
                    _ => None,
                })
        });

        let held = match self.db().with_conn(|conn| {
            let cookbook_id = cookbook_of_person(conn, &caller.person_id)?;
            receiving(conn, &cookbook_id, record)
        })? {
            Receiving::New => Value::Null,
            Receiving::Held {
                branch_id,
                arrived,
                since,
                verdict,
            } => json!({
                "branch_id": branch_id,
                "arrived": arrived,
                "since": since,
                "newer": match verdict {
                    Verdict::Extends { held } => versions - held,
                    _ => 0,
                },
                "diverged": matches!(verdict, Verdict::Refused(_)),
            }),
        };

        // One preview staged per Person at a time. Opening `/import?link=…` is
        // enough to ask for one, so a link crafted to be opened over and over
        // must not fill the disk a recipe file at a time: each preview replaces
        // the last, and only the one on screen can be imported.
        let (upload_id, path) = self.begin_upload(&caller.person_id)?;
        let marker = path.with_file_name(STAGED_PREVIEW);
        if let Ok(previous) = std::fs::read_to_string(&marker)
            && let Ok(staged) = self.staged_upload(&caller.person_id, previous.trim())
        {
            let _ = std::fs::remove_file(staged);
        }
        std::fs::write(&path, bytes)
            .map_err(|e| OpError::internal(format!("cannot stage the recipe file: {e}")))?;
        std::fs::write(&marker, &upload_id)
            .map_err(|e| OpError::internal(format!("cannot stage the recipe file: {e}")))?;

        Ok(json!({
            "upload_id": upload_id,
            "title": bundles::title_of(record),
            "shared_by": shared_by,
            "written_by": text_at(&record["hand"], "name"),
            "source": source,
            "versions": versions,
            "photo": photo,
            "held": held,
        }))
    }
}

/// The Lineages a Bundle says it is about (ADR 0020).
fn subjects_of(sidecar: &Value) -> HashSet<String> {
    sidecar["subjects"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|lineage| lineage.as_str().map(str::to_string))
        .collect()
}

/// Whether a carried Branch is one the Bundle is about rather than a
/// Passenger: as the record says, or, from a writer that did not say, as its
/// Lineage is one of the Bundle's subjects.
fn is_subject(record: &Value, subjects: &HashSet<String>) -> bool {
    record["subject"].as_bool().unwrap_or_else(|| {
        record["lineage_id"]
            .as_str()
            .is_some_and(|lineage| subjects.contains(lineage))
    })
}

/// The file beside a Person's staged uploads naming the one their last
/// preview staged. Not an upload id's shape, so no Operation can name it.
const STAGED_PREVIEW: &str = "preview";

/// How large the picture a preview carries is, on its long edge: sharp at the
/// 72 px (`--photo-thumb`) the confirm screen draws it at on a phone's
/// triple-density screen, and a few kilobytes inside the answer.
const PREVIEW_PICTURE_EDGE: u32 = 216;

/// A text field of a JSON object, when it holds any text.
fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value[key]
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

/// What receiving one carried Branch into a Cookbook does, decided before
/// anything is written: the one answer both [`place_carried_branch`] acts on
/// and `preview_shared_bundle` reports (#170), so a preview cannot promise
/// what an import would then refuse.
enum Receiving {
    /// The Cookbook holds no Branch travelling under this id.
    New,
    /// It holds one already, which the file would leave or carry forward.
    Held {
        /// The **Local id** it is held under here (#90).
        branch_id: String,
        /// Whether it arrived from elsewhere, rather than being written here.
        arrived: bool,
        /// When the Cookbook first held it.
        since: String,
        verdict: Verdict,
    },
}

/// What a file does to a Branch the Cookbook already holds.
enum Verdict {
    /// It carries nothing the Cookbook does not hold: a Bundle that left here
    /// and came back, or the same one received twice.
    Unchanged,
    /// It carries the held chain and more; `held` is how many Versions of it
    /// are here already, so what arrives is everything past that.
    Extends { held: usize },
    /// It cannot be placed without undoing or overwriting what is here, and
    /// this is the sentence that says why. Nothing is changed.
    Refused(String),
}

/// Decide what receiving `record` into `cookbook_id` would do.
///
/// Scoped to the Cookbook receiving it. A Cookbook next door holding the same
/// sender's Branch is another household's business, and neither an import nor
/// a preview reads it or says it is there.
fn receiving(conn: &Connection, cookbook_id: &str, record: &Value) -> Result<Receiving, OpError> {
    let travelling_id = record["branch_id"].as_str().unwrap_or_default();
    let title = bundles::title_of(record);
    let held: Option<(String, String, String, bool, String)> = conn
        .query_row(
            "SELECT id, lineage_id, hand_id, arrived, created_at FROM branches \
              WHERE COALESCE(travelling_id, id) = ?1 AND cookbook_id = ?2",
            params![travelling_id, cookbook_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?;
    let Some((branch_id, held_lineage, held_hand, arrived, since)) = held else {
        return Ok(Receiving::New);
    };
    let held = |verdict| {
        Ok(Receiving::Held {
            branch_id: branch_id.clone(),
            arrived,
            since: since.clone(),
            verdict,
        })
    };

    if held_lineage != record["lineage_id"].as_str().unwrap_or_default() {
        return held(Verdict::Refused(format!(
            "«{title}» names a Branch your Cookbook already holds as a different recipe, \
             so it was left out and nothing here was changed"
        )));
    }
    if held_hand != record["hand"]["id"].as_str().unwrap_or_default() {
        // A Branch has one Cookbook writing it (ADR 0020), so a Bundle naming
        // this Branch under another Hand is not its next chapter.
        return held(Verdict::Refused(format!(
            "«{title}» names a Branch held here under another Hand, \
             so it was left out and nothing here was changed"
        )));
    }

    let mut statement = conn
        .prepare("SELECT version_id FROM branch_versions WHERE branch_id = ?1 ORDER BY sequence")
        .map_err(|e| OpError::internal(format!("cannot read Branch chain: {e}")))?;
    let chain: Vec<String> = statement
        .query_map(params![&branch_id], |row| row.get(0))
        .map_err(|e| OpError::internal(format!("cannot read Branch chain: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read Branch chain: {e}")))?;
    let versions = record["versions"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    if let Some(at) = chain
        .iter()
        .zip(versions)
        .position(|(held, arriving)| arriving["version_id"].as_str() != Some(held.as_str()))
    {
        return held(Verdict::Refused(format!(
            "«{title}» disagrees with the one held here from Version {} on, \
             so the one here was kept as it is",
            at + 1
        )));
    }
    if versions.len() <= chain.len() {
        return held(Verdict::Unchanged);
    }
    if !arrived {
        // Only this Cookbook writes this Branch, so no Bundle can hold more of
        // it than this instance does. One that claims to is not believed.
        return held(Verdict::Refused(format!(
            "«{title}» claims Versions of a recipe written here that were never written \
             here, so they were left out and nothing here was changed"
        )));
    }
    held(Verdict::Extends { held: chain.len() })
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

/// Write the instance's public address.
///
/// An upsert rather than an `UPDATE`, because `instance_setup` holds at most
/// one row and there is no guarantee it exists yet: the row is written when a
/// first Person claims the instance, and an address can be set before that —
/// an `UPDATE` would then quietly change nothing and the first Share Link would
/// have no address to render against.
fn store_public_address(conn: &Connection, address: &str) -> Result<(), OpError> {
    conn.execute(
        "INSERT INTO instance_setup (singleton, public_address) VALUES (1, ?1)          ON CONFLICT(singleton) DO UPDATE SET public_address = excluded.public_address",
        params![address],
    )
    .map_err(|e| OpError::internal(format!("cannot store the public address: {e}")))?;
    Ok(())
}

/// The instance's public address, or nothing if no Share Link has needed one.
fn stored_public_address(conn: &Connection) -> Result<Option<String>, OpError> {
    conn.query_row(
        "SELECT public_address FROM instance_setup WHERE singleton = 1",
        [],
        |row| row.get(0),
    )
    .optional()
    .map(Option::flatten)
    .map_err(|e| OpError::internal(format!("cannot read the public address: {e}")))
}

/// One spelling of the instance's address, so a link built today and a link
/// built next year are the same string.
///
/// Deliberately strict about the two things that would silently break a link —
/// a missing scheme and a trailing slash — and deliberately incurious about
/// everything else. Kamosu cannot tell from inside whether an address is
/// reachable, and refusing one that turns out to work is worse than storing
/// one that does not (ADR 0033: assume the proxy did nothing but carry bytes).
fn normalise_public_address(address: &str) -> Result<String, OpError> {
    let trimmed = address.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(OpError::bad_request("a public address cannot be empty"));
    }
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(OpError::bad_request(
            "a public address must start with http:// or https:// — it is what goes in front of \
             a Share Link, so it has to be the whole of how somebody reaches this instance",
        ));
    }
    Ok(trimmed.to_string())
}

/// The live Share Link on a Branch.
struct LiveShare {
    id: String,
    shared_by: String,
    created_at: String,
    /// Kept since #171, and never on a link minted before it.
    secret: Option<String>,
}

fn live_share_link(conn: &Connection, branch_id: &str) -> Result<Option<LiveShare>, OpError> {
    conn.query_row(
        "SELECT id, shared_by, created_at, secret FROM share_links \
          WHERE branch_id = ?1 AND ended_at IS NULL",
        params![branch_id],
        |row| {
            Ok(LiveShare {
                id: row.get(0)?,
                shared_by: row.get(1)?,
                created_at: row.get(2)?,
                secret: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read the Share Link: {e}")))
}

/// What the share screen and `share_recipe` both answer.
///
/// `url` is answered for as long as a live link's Secret is kept, which is
/// every link minted since #171 (ADR 0031 as amended). It is built against the
/// public address as it stands now, so moving the instance moves the address
/// the screen shows: the new one is the one that opens. A link minted before
/// #171 kept only its hash and answers no `url`, and the screen says why.
fn share_link_summary(conn: &Connection, live: Option<LiveShare>) -> Result<Value, OpError> {
    let address = stored_public_address(conn)?;
    let Some(LiveShare {
        id,
        shared_by,
        created_at,
        secret,
    }) = live
    else {
        return Ok(json!({
            "shared": false,
            "share_id": Value::Null,
            "url": Value::Null,
            "shared_by": Value::Null,
            "created_at": Value::Null,
            "public_address": address,
        }));
    };
    Ok(json!({
        "shared": true,
        "share_id": id,
        "url": match (secret.as_deref(), address.as_deref()) {
            (Some(secret), Some(address)) => json!(format!("{address}/s/{secret}")),
            _ => Value::Null,
        },
        "shared_by": person_name(conn, &shared_by)?,
        "created_at": created_at,
        "public_address": address,
    }))
}

/// A Person's name, looked up live (CONTEXT.md, "Hand").
fn person_name(conn: &Connection, person_id: &str) -> Result<String, OpError> {
    // A Person here is named live. A Hand that arrived in a Bundle is named by
    // what arrived with it, and nothing else ever will name it (#67).
    conn.query_row(
        "SELECT name FROM people WHERE id = ?1 \
         UNION ALL SELECT name FROM arrived_hands WHERE hand_id = ?1 LIMIT 1",
        params![person_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read a Person's name: {e}")))?
    // An account deleted after minting a link leaves the link working — ending
    // one is its own deliberate act (ADR 0018) and closing an account is not
    // it. What is lost is the name, and saying so beats printing an id.
    .ok_or(())
    .or_else(|()| Ok("somebody who has since left".to_string()))
}

/// One row of a shared Thread as it comes off the database: sequence, name,
/// *what changed* line, the Hand that wrote it, and when.
type ThreadRow = (i64, Option<String>, Option<String>, String, String);

/// Whether one shared Version actually shows this Photograph — its Main Photo,
/// or the picture on one of its Steps.
fn version_shows_photograph(version: &Value, hash: &str) -> bool {
    content_shows_photograph(&version["content"], hash)
}

/// Everything the public page draws: the shared Branch, its Translations and
/// its Thread. No Attempt is read anywhere in here, by construction.
fn shared_branch(conn: &Connection, branch_id: &str) -> Result<Value, OpError> {
    let (lineage_id, language, head_version_id): (String, String, String) = conn
        .query_row(
            "SELECT lineage_id, language, head_version_id FROM branches WHERE id = ?1",
            params![branch_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?
        .ok_or_else(no_such_branch)?;

    let recipe = shared_version(conn, branch_id, &head_version_id, &language)?;

    // Its Translations (ADR 0006, ADR 0018): the other Branches of this
    // Lineage in this Cookbook written in another Language. They are carried
    // whole rather than as links, because each is a Branch of its own and a
    // second token per Translation would be a second link to end.
    let mut statement = conn
        .prepare(
            "SELECT branches.id, branches.language, branches.head_version_id \
               FROM branches \
              WHERE branches.lineage_id = ?1 AND branches.id <> ?2 \
                AND branches.cookbook_id = (SELECT cookbook_id FROM branches WHERE id = ?2) \
              ORDER BY branches.created_at ASC",
        )
        .map_err(|e| OpError::internal(format!("cannot read Translations: {e}")))?;
    let siblings: Vec<(String, String, String)> = statement
        .query_map(params![lineage_id, branch_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| OpError::internal(format!("cannot read Translations: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read Translations: {e}")))?;

    let mut translations = Vec::new();
    for (sibling_id, sibling_language, sibling_head) in siblings {
        // Only a Translation travels — a Copy or a Divergence is somebody
        // else's recipe standing beside this one, and sharing this Branch says
        // nothing about that one.
        let is_translation = translation_of_branch(conn, &lineage_id, &sibling_id)?
            .get("source_branch_id")
            .and_then(Value::as_str)
            == Some(branch_id);
        if !is_translation {
            continue;
        }
        translations.push(shared_version(
            conn,
            &sibling_id,
            &sibling_head,
            &sibling_language,
        )?);
    }

    // The Thread of the Branch shared: every Version in order with its name,
    // its *what changed* line and its author. Complete back to the first
    // Version, because a share cannot be made to start part-way along, and
    // there is no argument here that could shorten it (ADR 0018).
    let mut statement = conn
        .prepare(
            "SELECT sequence, name, change_note, hand_id, created_at \
               FROM branch_versions WHERE branch_id = ?1 ORDER BY sequence ASC",
        )
        .map_err(|e| OpError::internal(format!("cannot read the Thread: {e}")))?;
    let rows: Vec<ThreadRow> = statement
        .query_map(params![branch_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .map_err(|e| OpError::internal(format!("cannot read the Thread: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read the Thread: {e}")))?;
    let mut thread = Vec::with_capacity(rows.len());
    for (sequence, name, change_note, hand_id, created_at) in rows {
        thread.push(json!({
            "sequence": sequence,
            "name": name,
            "change_note": change_note,
            // A Version's Hand is a Person's (ADR 0015), so this is the name a
            // share carries — which is what makes credit travel with a recipe.
            "hand": person_name(conn, &hand_id)?,
            "created_at": created_at,
        }));
    }

    Ok(json!({
        "recipe": recipe,
        "translations": translations,
        "thread": thread,
    }))
}

/// **What one Bundle carries** (ADR 0020): the Lineages it is about and every
/// Branch that travels, with the hash of every Photograph those show beside it.
///
/// The Branch shared and its Translations are the subjects — the same
/// Translations a Share Link carries, by the same test. Every Component their
/// head Versions unfold to, at any depth, travels as a Passenger, resolved
/// against the Cookbook holding the shared Branch exactly as the Share Link
/// page resolves one.
fn bundle_contents(
    conn: &Connection,
    branch_id: &str,
    cookbook_id: &str,
) -> Result<(bundles::Contents, Vec<String>), OpError> {
    let (lineage_id, language): (String, String) = conn
        .query_row(
            "SELECT lineage_id, language FROM branches WHERE id = ?1",
            params![branch_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?;

    let mut order: Vec<String> = vec![branch_id.to_string()];
    let mut statement = conn
        .prepare(
            "SELECT id FROM branches WHERE lineage_id = ?1 AND id <> ?2 AND cookbook_id = ?3 \
              ORDER BY created_at ASC, id ASC",
        )
        .map_err(|e| OpError::internal(format!("cannot read Translations: {e}")))?;
    let siblings: Vec<String> = statement
        .query_map(params![lineage_id, branch_id, cookbook_id], |row| {
            row.get(0)
        })
        .map_err(|e| OpError::internal(format!("cannot read Translations: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read Translations: {e}")))?;
    for sibling in siblings {
        // Only a Translation travels, as on the Share Link page: a Copy or a
        // Divergence beside this Branch is somebody else's recipe.
        let translates = translation_of_branch(conn, &lineage_id, &sibling)?
            .get("source_branch_id")
            .and_then(Value::as_str)
            == Some(branch_id);
        if translates {
            order.push(sibling);
        }
    }

    // Walk every carried Branch's head for Components, Passengers included,
    // so a dough inside a dough travels too. `order` grows as the walk finds
    // them, and a Branch already carried is never added twice.
    let mut carried: Vec<bundles::Carried> = Vec::new();
    let mut index = 0;
    while index < order.len() {
        let this = order[index].clone();
        let record = bundle_branch(conn, &this)?;
        let head = bundles::head(&record)
            .and_then(|version| version["version_id"].as_str())
            .unwrap_or("")
            .to_string();
        let this_lineage = record["lineage_id"].as_str().unwrap_or("").to_string();
        let mut walk = Unfolding {
            open: vec![this_lineage],
            ..Unfolding::default()
        };
        unfold_components(
            conn,
            &Unfolds::AsPassenger {
                cookbook_id,
                language: record["language"].as_str().unwrap_or(&language),
            },
            &head,
            1.0,
            &mut walk,
        )?;
        for component in &walk.found {
            if let Some(passenger) = component["branch_id"].as_str()
                && !order.iter().any(|held| held == passenger)
            {
                order.push(passenger.to_string());
            }
        }
        carried.push(bundles::Carried {
            local_id: this,
            record,
            components: walk.found,
        });
        index += 1;
    }

    let mut hashes: Vec<String> = Vec::new();
    for branch in &carried {
        for version in branch.record["versions"].as_array().into_iter().flatten() {
            let content = &version["content"];
            let shown = std::iter::once(&content["main_photo"]).chain(
                content["steps"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|step| &step["photo"]),
            );
            for hash in shown.filter_map(Value::as_str) {
                if !hashes.iter().any(|held| held == hash) {
                    hashes.push(hash.to_string());
                }
            }
        }
    }
    // The Photographs themselves are read by the caller, off the lock.
    let contents = bundles::Contents {
        subjects: vec![lineage_id],
        branches: carried,
        photographs: Vec::new(),
        missing_photographs: Vec::new(),
    };
    Ok((contents, hashes))
}

/// One Branch as a Bundle's sidecar records it: its ids, its Language, the
/// Cookbook's Hand, its name where it is a variation (#131, question 10), its
/// origin address, its Tags as names, and its complete chain of Versions.
///
/// The origin address is carried exactly as stored and never filled in from
/// this instance's own address (ADR 0020: nothing in v1 writes one). A Branch
/// that arrived carrying one keeps it, which is what stops a reshare laundering
/// where a recipe came from.
///
/// The `branch_id` written here is the **Travelling id**, never this instance's
/// **Local id** (#90). On a Branch minted here the two are the same; on one
/// that arrived, the Travelling id is the sender's, so a reshare continues the
/// sender's Branch at the next instance rather than starting a third.
fn bundle_branch(conn: &Connection, branch_id: &str) -> Result<Value, OpError> {
    let (travelling_id, lineage_id, language, hand_id, origin_address, branch_name): (
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT COALESCE(travelling_id, id), lineage_id, language, hand_id, origin_address, name \
               FROM branches WHERE id = ?1",
            params![branch_id],
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
        .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?
        .ok_or_else(no_such_branch)?;
    // A received Branch's Hand is the sender's, named by what arrived with it
    // (#67): resharing it names them, not this Cookbook. One written here is
    // named after the Cookbook answering to it now.
    let hand_name: Option<String> = conn
        .query_row(
            &format!("SELECT {}", hand_name_sql("?1")),
            params![hand_id],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot read a Hand's name: {e}")))?;

    // Tags travel as names, in every Language each has one in (ADR 0020).
    let mut statement = conn
        .prepare(
            "SELECT tag_names.tag_id, tag_names.language, tag_names.name \
               FROM branch_tags JOIN tag_names ON tag_names.tag_id = branch_tags.tag_id \
              WHERE branch_tags.branch_id = ?1 \
              ORDER BY tag_names.tag_id, tag_names.language",
        )
        .map_err(|e| OpError::internal(format!("cannot read Tags: {e}")))?;
    let rows: Vec<(String, String, String)> = statement
        .query_map(params![branch_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| OpError::internal(format!("cannot read Tags: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read Tags: {e}")))?;
    let mut tags: Vec<(String, serde_json::Map<String, Value>)> = Vec::new();
    for (tag_id, tag_language, name) in rows {
        if tags.last().is_none_or(|(held, _)| *held != tag_id) {
            tags.push((tag_id, serde_json::Map::new()));
        }
        if let Some((_, names)) = tags.last_mut() {
            names.insert(tag_language, json!(name));
        }
    }
    let tags: Vec<Value> = tags
        .into_iter()
        .map(|(_, names)| json!({ "names": names }))
        .collect();

    // The chain, complete back to the first Version (ADR 0018). The Access Key
    // that wrote each one is deliberately not selected: it is for its author
    // to read on this instance and never travels (ADR 0015).
    let mut statement = conn
        .prepare(
            "SELECT branch_versions.sequence, branch_versions.version_id, \
                    branch_versions.parent_version_id, branch_versions.translates_version_id, \
                    branch_versions.language, branch_versions.name, branch_versions.change_note, \
                    branch_versions.created_at, branch_versions.hand_id, versions.content \
               FROM branch_versions JOIN versions ON versions.id = branch_versions.version_id \
              WHERE branch_versions.branch_id = ?1 ORDER BY branch_versions.sequence ASC",
        )
        .map_err(|e| OpError::internal(format!("cannot read the Thread: {e}")))?;
    type Row = (
        i64,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
        String,
        String,
    );
    let rows: Vec<Row> = statement
        .query_map(params![branch_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
                row.get(9)?,
            ))
        })
        .map_err(|e| OpError::internal(format!("cannot read the Thread: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read the Thread: {e}")))?;
    let mut versions = Vec::with_capacity(rows.len());
    for (
        sequence,
        version_id,
        parent,
        translates,
        at_language,
        name,
        change_note,
        created_at,
        hand,
        content,
    ) in rows
    {
        // The content exactly as stored, never normalised on the way out: it
        // is what the receiver fingerprints to check this Version is what its
        // id says it is (ADR 0004, ADR 0020).
        let content: Value = serde_json::from_str(&content)
            .map_err(|e| OpError::internal(format!("a Version's content is unreadable: {e}")))?;
        let line_count = content["ingredients"].as_array().map(Vec::len).unwrap_or(0);
        versions.push(json!({
            "sequence": sequence,
            "version_id": version_id,
            "parent_version_id": parent,
            "translates_version_id": translates,
            "language": at_language,
            "name": name,
            "change_note": change_note,
            "created_at": created_at,
            "hand": { "id": hand, "name": person_name(conn, &hand)? },
            // The slot a signature would sit in, empty: v1 signs nothing, and
            // keeping the slot is what keeps that decision reversible.
            "signature": Value::Null,
            "readings": bundle_readings(conn, &version_id, line_count)?,
            "content": content,
        }));
    }

    Ok(json!({
        "branch_id": travelling_id,
        "lineage_id": lineage_id,
        "language": language,
        "hand": { "id": hand_id, "name": hand_name },
        // A variation's name travels, outside the fingerprint like a
        // Version's, so a receiver can tell "Classic" from "Vegetarian"
        // (#131, question 10). Absent on every other Branch.
        "name": branch_name,
        "origin_address": origin_address,
        "tags": tags,
        "versions": versions,
    }))
}

/// A Version's Readings as they travel: one slot per Ingredient Line, null
/// where there is none, each carried as stored and never recomputed (ADR 0021).
///
/// Where a Reading points at a **Food**, the Food travels as its names in
/// every Language it has one in, and nothing else: no id, since two instances
/// mint their *farine* separately, and no Cup Weight or nutrition, since what
/// this instance learned stays this instance's (ADR 0016, ADR 0021).
fn bundle_readings(
    conn: &Connection,
    version_id: &str,
    line_count: usize,
) -> Result<Vec<Value>, OpError> {
    let mut slots = vec![Value::Null; line_count];
    let mut statement = conn
        .prepare(
            "SELECT line_index, amount, unit, target, lineage_id, food_id \
               FROM readings WHERE version_id = ?1",
        )
        .map_err(|e| OpError::internal(format!("cannot read Readings: {e}")))?;
    type Row = (
        i64,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    );
    let rows: Vec<Row> = statement
        .query_map(params![version_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })
        .map_err(|e| OpError::internal(format!("cannot read Readings: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read Readings: {e}")))?;
    let mut names_of = conn
        .prepare("SELECT language, name FROM food_names WHERE food_id = ?1 ORDER BY language")
        .map_err(|e| OpError::internal(format!("cannot read a Food's names: {e}")))?;
    for (line_index, amount, unit, target, lineage_id, food_id) in rows {
        let food = match food_id {
            Some(food_id) => {
                let names: serde_json::Map<String, Value> = names_of
                    .query_map(params![food_id], |row| {
                        Ok((row.get::<_, String>(0)?, json!(row.get::<_, String>(1)?)))
                    })
                    .map_err(|e| OpError::internal(format!("cannot read a Food's names: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot read a Food's names: {e}")))?;
                json!({ "names": names })
            }
            None => Value::Null,
        };
        if let Some(slot) = usize::try_from(line_index)
            .ok()
            .and_then(|index| slots.get_mut(index))
        {
            *slot = json!({
                "amount": amount,
                "unit": unit,
                "target": target,
                "lineage_id": lineage_id,
                "food": food,
            });
        }
    }
    Ok(slots)
}

/// What became of one recipe a Bundle carried.
enum Fate {
    /// Held here under the Lineage id it carried, travelling on under the
    /// Branch id it carried: `created`, `extended` or `unchanged`.
    Placed {
        status: &'static str,
        lineage_id: String,
        /// The **Local id**, never the Travelling id (#90): this is what the
        /// Import Report names the recipe by, and what opens it here.
        branch_id: String,
        title: String,
    },
    /// Its history refused and its words kept as a new recipe of the
    /// caller's own (ADR 0020), with the sentence that says so.
    Kept {
        reason: String,
        lineage_id: String,
        branch_id: String,
        title: String,
    },
    /// Nothing of it kept, and why.
    Refused(String),
}

impl Fate {
    /// This fate as one row of the Import Report: `arrived` for a placed
    /// Branch, `unreadable` for everything else — a damaged one pointing at
    /// the recipe its words became (#67).
    fn row(&self, foreign_id: &Value, subject: bool) -> Value {
        match self {
            Fate::Placed {
                status,
                lineage_id,
                branch_id,
                title,
            } => json!({
                "foreign_id": foreign_id,
                "status": status,
                "lineage_id": lineage_id,
                "branch_id": branch_id,
                "title": title,
                "subject": subject,
            }),
            Fate::Kept {
                reason,
                lineage_id,
                branch_id,
                title,
            } => json!({
                "foreign_id": foreign_id,
                "reason": reason,
                "kept_as": { "lineage_id": lineage_id, "branch_id": branch_id, "title": title },
            }),
            Fate::Refused(reason) => json!({ "foreign_id": foreign_id, "reason": reason }),
        }
    }
}

/// Place one sound carried Branch (#67, ADR 0020): new to this Cookbook, it is
/// held by `cookbook_id` under the sender's Lineage id and Hands, travelling
/// under the sender's Branch id; already here, it is extended by whatever the
/// Bundle carries past the Version held.
///
/// **The question is always about this Cookbook's copy** (#90, ADR 0041). A
/// Branch's **Travelling id** is unique per Cookbook rather than per instance,
/// so the lookup is scoped to `cookbook_id` and what another household here
/// holds is neither consulted nor mentioned. The row itself gets a freshly
/// minted **Local id**, and that is what the Import Report names it by, since
/// that is the id every Operation and every URL here takes.
///
/// A Branch placed here is the sender's writing, and **arrives**: the first
/// change to it starts a Branch of the receiver's own. The one exception is a
/// Branch carrying a Hand this Cookbook answers to — its own recipe coming
/// home — which it goes on writing. A variation's name, where the Bundle
/// carries one, comes with it (#131, question 10).
///
/// `Fate::Refused` is for a Branch that cannot be placed without undoing or
/// overwriting something already here — which a well-formed Bundle never asks
/// for, since only the Cookbook writing a Branch ever adds to it.
fn place_carried_branch(
    conn: &Connection,
    cookbook_id: &str,
    caller: &Caller,
    record: &Value,
    held_before: &HashSet<String>,
) -> Result<Fate, OpError> {
    let travelling_id = record["branch_id"].as_str().unwrap_or_default();
    let lineage_id = record["lineage_id"].as_str().unwrap_or_default();
    let hand_id = record["hand"]["id"].as_str().unwrap_or_default();
    let title = bundles::title_of(record);
    let versions = record["versions"].as_array().cloned().unwrap_or_default();
    let head_version_id = versions
        .last()
        .and_then(|version| version["version_id"].as_str())
        .unwrap_or_default();
    // A Language this instance does not know is stood in for by Unknown, which
    // shows the recipe to every reader rather than to none (ADR 0006).
    let language = record["language"]
        .as_str()
        .filter(|language| supported_branch_language(language).is_ok())
        .unwrap_or(crate::language::UNKNOWN);
    let origin_address = record["origin_address"].as_str().filter(|a| !a.is_empty());
    let placed = |status, branch_id: &str| Fate::Placed {
        status,
        lineage_id: lineage_id.to_string(),
        branch_id: branch_id.to_string(),
        title: title.to_string(),
    };

    let (branch_id, verdict) = match receiving(conn, cookbook_id, record)? {
        Receiving::Held {
            branch_id, verdict, ..
        } => (branch_id, verdict),
        Receiving::New => {
            let branch_id = format!("b_{}", hex::encode(random_bytes(8)));
            conn.execute(
                "INSERT OR IGNORE INTO lineages (id) VALUES (?1)",
                params![lineage_id],
            )
            .map_err(|e| OpError::internal(format!("cannot record Lineage: {e}")))?;
            let comes_home: bool = conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM cookbook_hands WHERE hand_id = ?1 AND cookbook_id = ?2)",
                params![hand_id, cookbook_id],
                |row| row.get(0),
            )
            .map_err(|e| OpError::internal(format!("cannot read Hands: {e}")))?;
            let name = record["name"]
                .as_str()
                .map(str::trim)
                .filter(|name| !name.is_empty());
            // Its own recipe coming home keeps the one-unnamed rule the Cookbook
            // keeps for everything it writes.
            let name = match name {
                Some(name) => Some(name.to_string()),
                None if comes_home => name_on_arrival(
                    conn,
                    cookbook_id,
                    lineage_id,
                    language,
                    record["hand"]["name"].as_str().unwrap_or_default(),
                )?,
                None => None,
            };
            // The origin address is a hint and stays one: stored as it came, never
            // fetched, and carried on unchanged by every reshare (ADR 0020).
            conn.execute(
                "INSERT INTO branches \
             (id, travelling_id, lineage_id, cookbook_id, hand_id, language, origin_address, \
              head_version_id, name, started_by, arrived) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    branch_id,
                    travelling_id,
                    lineage_id,
                    cookbook_id,
                    hand_id,
                    language,
                    origin_address,
                    head_version_id,
                    name,
                    caller.person_id,
                    !comes_home as i64
                ],
            )
            .map_err(|e| OpError::internal(format!("cannot place Branch: {e}")))?;
            write_carried_versions(conn, &branch_id, &versions, held_before)?;
            file_carried_tags(conn, cookbook_id, &branch_id, record)?;
            remember_arrived_hands(conn, record)?;
            return Ok(placed("created", &branch_id));
        }
    };
    let held = match verdict {
        Verdict::Refused(why) => return Ok(Fate::Refused(why)),
        Verdict::Unchanged => return Ok(placed("unchanged", &branch_id)),
        Verdict::Extends { held } => held,
    };

    write_carried_versions(conn, &branch_id, &versions[held..], held_before)?;
    conn.execute(
        "UPDATE branches SET head_version_id = ?1, language = ?2, \
                origin_address = COALESCE(?3, origin_address) \
          WHERE id = ?4",
        params![head_version_id, language, origin_address, branch_id],
    )
    .map_err(|e| OpError::internal(format!("cannot move Branch head: {e}")))?;
    remember_arrived_hands(conn, record)?;
    Ok(placed("extended", &branch_id))
}

/// Write carried Versions onto a Branch's chain, each at the sequence it
/// carried, with its Hand, name, *what changed* line and date as they came —
/// and, for a Version this instance did not already hold, its Readings as they
/// were sent (ADR 0021). The Access Key that wrote a Version never travels, so
/// none is recorded.
///
/// A Translation's pointer is kept only where the Version it names is here: a
/// Bundle of a Translation on its own does not carry what it translates, and a
/// pointer at nothing is not one the database will hold. Losing it costs only
/// the "how far behind" count, never the recipe.
fn write_carried_versions(
    conn: &Connection,
    branch_id: &str,
    versions: &[Value],
    held_before: &HashSet<String>,
) -> Result<(), OpError> {
    for version in versions {
        let version_id = version["version_id"].as_str().unwrap_or_default();
        let translates = match version["translates_version_id"].as_str() {
            Some(id) => conn
                .query_row(
                    "SELECT id FROM versions WHERE id = ?1",
                    params![id],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read Version: {e}")))?,
            None => None,
        };
        let language = version["language"]
            .as_str()
            .filter(|language| supported_branch_language(language).is_ok());
        conn.execute(
            "INSERT INTO branch_versions \
             (branch_id, sequence, version_id, parent_version_id, hand_id, name, change_note, \
              created_at, translates_version_id, language) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, \
                     COALESCE(?8, strftime('%Y-%m-%dT%H:%M:%fZ','now')), ?9, ?10)",
            params![
                branch_id,
                version["sequence"].as_i64(),
                version_id,
                version["parent_version_id"].as_str(),
                version["hand"]["id"].as_str(),
                version["name"].as_str().filter(|n| !n.is_empty()),
                version["change_note"].as_str().filter(|n| !n.is_empty()),
                version["created_at"].as_str(),
                translates,
                language,
            ],
        )
        .map_err(|e| OpError::internal(format!("cannot place a carried Version: {e}")))?;
        if !held_before.contains(version_id) {
            write_carried_readings(conn, version_id, version)?;
        }
    }
    Ok(())
}

/// A carried Version's Readings, written as they were sent and never
/// recomputed (ADR 0021) — a line the sender had no Reading for stays unread
/// here too. A Food arrives as its names and goes through the one Food Match
/// every door uses (ADR 0022), with every name it carried at once: that is the
/// one place two names can hit two Foods, and doubt then makes a third.
fn write_carried_readings(
    conn: &Connection,
    version_id: &str,
    version: &Value,
) -> Result<(), OpError> {
    let lines = version["content"]["ingredients"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let slots = version["readings"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    for (index, (line, slot)) in lines.iter().zip(slots).enumerate() {
        if !slot.is_object() || line["kind"].as_str() == Some("section") {
            continue;
        }
        let text = |field: &str| slot[field].as_str().filter(|value| !value.is_empty());
        let (amount, unit) = (text("amount"), text("unit"));
        // A Reading's target is a Food or a Lineage, never both (ADR 0008).
        let (target, lineage_id) = match text("lineage_id") {
            Some(lineage_id) => (None, Some(lineage_id)),
            None => (text("target"), None),
        };
        let names: Vec<(&str, &str)> = slot["food"]["names"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(language, name)| {
                Some((
                    language.as_str(),
                    name.as_str().filter(|n| !n.trim().is_empty())?,
                ))
            })
            .collect();
        let food_id = match (lineage_id, names.is_empty()) {
            (None, false) => Some(resolve_food_for_names(conn, &names, None)?),
            _ => None,
        };
        if amount.is_none() && unit.is_none() && target.is_none() && lineage_id.is_none() {
            continue;
        }
        conn.execute(
            "INSERT OR IGNORE INTO readings \
             (version_id, line_index, amount, unit, target, lineage_id, food_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                version_id,
                index as i64,
                amount,
                unit,
                target,
                lineage_id,
                food_id
            ],
        )
        .map_err(|e| OpError::internal(format!("cannot record a carried Reading: {e}")))?;
    }
    Ok(())
}

/// Keep the name each Hand a Bundle carried arrived under (CONTEXT.md,
/// "Hand"): the Kitchen's on the Branch, and the Person's on each Version.
/// A Hand minted here is named live and is never renamed by what arrives.
/// Neither is a deleted Kitchen's, kept here once its row is gone (#129).
fn remember_arrived_hands(conn: &Connection, record: &Value) -> Result<(), OpError> {
    let hands = std::iter::once(&record["hand"]).chain(
        record["versions"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|version| &version["hand"]),
    );
    for hand in hands {
        let (Some(id), Some(name)) = (
            hand["id"].as_str().filter(|id| !id.is_empty()),
            hand["name"].as_str().filter(|name| !name.trim().is_empty()),
        ) else {
            continue;
        };
        conn.execute(
            "INSERT INTO arrived_hands (hand_id, name) \
             SELECT ?1, ?2 \
              WHERE NOT EXISTS (SELECT 1 FROM people WHERE id = ?1) \
                AND NOT EXISTS (SELECT 1 FROM kitchens WHERE hand_id = ?1) \
                AND NOT EXISTS (SELECT 1 FROM cookbook_hands WHERE hand_id = ?1) \
             ON CONFLICT (hand_id) DO UPDATE SET name = excluded.name \
              WHERE arrived_hands.minted_here = 0",
            params![id, name],
        )
        .map_err(|e| OpError::internal(format!("cannot keep a Hand's name: {e}")))?;
    }
    Ok(())
}

/// File a newly arrived Branch under the Tags it carried, in the receiving
/// Cookbook's own list ([`arriving_tag`]).
fn file_carried_tags(
    conn: &Connection,
    cookbook_id: &str,
    branch_id: &str,
    record: &Value,
) -> Result<(), OpError> {
    for tag in record["tags"].as_array().into_iter().flatten() {
        let names: Vec<(&str, &str)> = tag["names"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(language, name)| {
                let name = name.as_str()?.trim();
                (supported_language(language).is_ok() && !name.is_empty())
                    .then_some((language.as_str(), name))
            })
            .collect();
        if let Some(tag_id) = arriving_tag(conn, cookbook_id, &names)? {
            file_branch_under(conn, branch_id, &tag_id)?;
        }
    }
    Ok(())
}

/// **A damaged Bundle keeps the dinner and loses where it came from** (ADR
/// 0020): its words — a damaged Branch's as they read at its newest Version, or
/// a note read back when there was no sidecar to trust — arrive as a new recipe
/// of the caller's own: a Lineage minted here, the caller's Hand on it, and
/// nothing carried over that claims to know where it came from. It is read
/// here as any recipe written here is, since the Readings it carried belong
/// to the history that was refused. `said` writes the Report's sentence from
/// the recipe's name.
fn keep_words_as_new_recipe(
    conn: &Connection,
    caller: &Caller,
    cookbook_id: &str,
    words: &Value,
    language: Option<&str>,
    said: impl FnOnce(&str) -> String,
) -> Result<Fate, OpError> {
    let title = words["title"]
        .as_str()
        .unwrap_or_default()
        .trim()
        .to_string();
    let named = if title.is_empty() {
        "A recipe".to_string()
    } else {
        format!("«{title}»")
    };
    let Ok(content) = parse_recipe_content(words) else {
        return Ok(Fate::Refused(format!(
            "{named} arrived damaged, and its words could not be read either, \
             so nothing of it was kept"
        )));
    };
    let (version_id, content_text) = stored_version(&content);
    let lineage_id = format!("l_{}", hex::encode(random_bytes(8)));
    let branch_id = format!("b_{}", hex::encode(random_bytes(8)));
    let stated = language.filter(|language| supported_branch_language(language).is_ok());
    let language = language_for_new_branch(conn, &caller.person_id, stated, &content)?;
    insert_new_lineage_and_branch(
        conn,
        &lineage_id,
        &branch_id,
        cookbook_id,
        &cookbook_hand(conn, cookbook_id)?,
        &language,
        &version_id,
        &content_text,
        &caller.person_id,
        caller.access_key_id.as_deref(),
    )?;
    Ok(Fate::Kept {
        reason: said(&named),
        lineage_id,
        branch_id,
        title,
    })
}

/// One Branch's current state, as a stranger sees it.
///
/// **There is no `measured` here**, and its absence is the point. Kamosu
/// converts to the kitchen (ADR 0016) and a stranger holding a link has no
/// kitchen, so every conversion would be into a system nobody asked for. The
/// Reading itself still travels — it is Kamosu's structured reading of the
/// line, useful to an agent reading this share at the MCP door — but the words
/// a stranger is shown are the written ones, which is what ADR 0002 says they
/// are: the truth.
fn shared_version(
    conn: &Connection,
    branch_id: &str,
    version_id: &str,
    language: &str,
) -> Result<Value, OpError> {
    // A Cover is derived from the Lineage id and nothing else (#46), so a page
    // drawing one for a recipe with no photograph needs it here.
    let lineage_id: String = conn
        .query_row(
            "SELECT lineage_id FROM branches WHERE id = ?1",
            params![branch_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?
        .ok_or_else(no_such_branch)?;
    let content: String = conn
        .query_row(
            "SELECT content FROM versions WHERE id = ?1",
            params![version_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read a Version: {e}")))?
        .ok_or_else(|| OpError::not_found("no such Version"))?;
    let content: Value = serde_json::from_str(&content)
        .map(content_as_declared)
        .map_err(|e| OpError::internal(format!("a Version's content is unreadable: {e}")))?;

    let line_count = content["ingredients"].as_array().map(Vec::len).unwrap_or(0);
    let readings = readings_for_version(conn, version_id, line_count)?;

    // The Passengers (ADR 0008): every Component this recipe composes, carried
    // through the link because a recipe that cannot tell you how to make its
    // own dough is incomplete. Resolved against the Cookbook holding what is
    // being shared, and changing nothing about who may see the dough.
    let cookbook_id = branch_cookbook(conn, branch_id)?;
    let mut walk = Unfolding {
        open: vec![lineage_id.clone()],
        ..Unfolding::default()
    };
    unfold_components(
        conn,
        &Unfolds::AsPassenger {
            cookbook_id: &cookbook_id,
            language,
        },
        version_id,
        // A stranger holding a link is cooking nothing, so there is no Yield
        // being cooked to carry into the factor.
        1.0,
        &mut walk,
    )?;
    let components = walk.found;

    Ok(json!({
        "branch_id": branch_id,
        "lineage_id": lineage_id,
        "version_id": version_id,
        "language": language,
        "content": content,
        "readings": readings,
        "components": components,
    }))
}
