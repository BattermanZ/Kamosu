//! Finding a recipe: word search, and Meaning Search when an instance has
//! turned it on (ADR 0029).

use super::*;

impl Core {
    /// How often the index is brought level with the library.
    ///
    /// A recipe edited a minute ago must still be findable by meaning, and
    /// re-embedding it inside the save would put two seconds of model inference
    /// inside an Immediate Operation. So the index catches up on its own, on a
    /// short tick whose cost when nothing changed is one query that finds
    /// nothing. `build_meaning_index` is the same work asked for by name, for
    /// somebody who does not want to wait even this long.
    const REINDEX_EVERY: std::time::Duration = std::time::Duration::from_secs(60);

    // ── Meaning Search ───────────────────────────────────────────────────────
    //
    // Optional, off by default, and off on most instances forever (ADR 0029).
    // Every Operation below exists on every instance whether or not the model
    // does — the Catalogue is the sole source of what Kamosu can do, and one
    // that varied per server would make an agent's first question unanswerable.

    /// What Meaning Search is doing here, and whether this reader should be
    /// offered it.
    ///
    /// Readable by any Person, because the offer is carried inside *nothing
    /// found* and every Person can arrive there. Whether the offer is *made* is
    /// answered here rather than on the screen: only an Operator can turn it on,
    /// and an offer somebody cannot act on is worse than none.
    pub fn meaning_search_status(&self, caller: &Caller) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let held = crate::meaning::state(conn)?;
            let behind = if held.is_on() {
                crate::meaning::behind(conn)?
            } else {
                0
            };
            Ok(json!({
                "state": held.state,
                "on": held.is_on() && self.meaning.ready(),
                // True only where the offer can be acted on: an Operator, on an
                // instance nobody has answered for yet. Never again once
                // declined — a question answered and asked again is a nag.
                "offer": held.still_worth_offering() && caller.is_operator,
                // Whether this caller could turn Meaning Search on or off at
                // all. Answered here for the same reason `offer` is: a screen
                // that works it out for itself would be a permission check
                // written outside the Core.
                "may_change": caller.is_operator,
                "model": crate::meaning::MODEL_NAME,
                "terms_url": crate::meaning::TERMS_URL,
                "prohibited_use_policy_url": crate::meaning::PROHIBITED_USE_POLICY_URL,
                // What this build asks about — and, where somebody has already
                // answered, what they actually agreed to. Saying today's version
                // back to them would be putting words in their mouth.
                "terms_version": held
                    .terms_version
                    .clone()
                    .unwrap_or_else(|| crate::meaning::TERMS_VERSION.to_string()),
                "accepted_by": held.accepted_by,
                "accepted_via_access_key": held.accepted_via_access_key,
                "accepted_at": held.accepted_at,
                "declined_at": held.declined_at,
                // Whether the weights are actually on disk, which is a different
                // question from whether anybody agreed to them.
                "model_present": self.meaning.weights_are_present(),
                "indexed_at": held.indexed_at,
                // How many recipes the index has yet to catch up with. It
                // catches up by itself within the minute; this says so honestly
                // rather than pretending an edit is already searchable.
                "recipes_not_yet_indexed": behind,
            }))
        })
    }

    /// Accept the model's terms. The acceptance keeps the **Hand** that made it
    /// and whether it arrived by login or by Access Key (ADR 0029) — recorded,
    /// never verified, which is the same thing Kamosu says out loud about every
    /// Hand it records (ADR 0015).
    pub fn accept_meaning_search_terms(&self, caller: &Caller) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            crate::meaning::accept(conn, &caller.person_id, caller.via_access_key)?;
            Ok(json!({ "state": crate::meaning::state(conn)?.state }))
        })
    }

    /// Decline. The offer is never made again on this instance.
    pub fn decline_meaning_search(&self, caller: &Caller) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            crate::meaning::decline(conn, &caller.person_id)?;
            Ok(json!({ "state": crate::meaning::state(conn)?.state }))
        })
    }

    /// Download the model — the one thing Kamosu will not do without being
    /// asked, because it is 200 MB of somebody else's weights under somebody
    /// else's licence.
    ///
    /// A Job: it is far too slow for a request, and watching it is what
    /// `get_job` is for.
    pub fn download_meaning_model(
        &self,
        progress: Option<&crate::jobs::JobProgress>,
    ) -> Result<Value, OpError> {
        if !self
            .db()
            .with_conn(|conn| Ok(crate::meaning::state(conn)?.is_accepted()))?
        {
            return Err(OpError::bad_request(
                "the model's terms have not been accepted on this Kamosu",
            ));
        }

        // Which file, and how far into it. One blended percentage would be a
        // lie about a download whose six artefacts run from 662 bytes to
        // 197 MB — it would sit still for minutes and then leap.
        let report = |file: usize, of: usize, done: u64, total: u64| {
            if let Some(progress) = progress {
                let percent = done.saturating_mul(100).checked_div(total).unwrap_or(0);
                progress.report(
                    percent,
                    Some(100),
                    format!(
                        "downloading {} — file {file} of {of}, {percent}%",
                        crate::meaning::MODEL_NAME
                    ),
                );
            }
        };
        // A download may clear a half-written cache, and a build in flight is
        // reading the model out of that directory. One at a time.
        let _building = self.meaning.building();
        self.meaning.download(&report).map_err(OpError::internal)?;

        if let Some(progress) = progress {
            progress.report(100, Some(100), "loading the model");
        }
        self.meaning.load().map_err(OpError::internal)?;

        Ok(json!({
            "model": crate::meaning::MODEL_NAME,
            "repository": crate::meaning::MODEL_REPOSITORY,
            "revision": crate::meaning::MODEL_REVISION,
        }))
    }

    /// Build — or bring level — the index Meaning Search reads.
    ///
    /// A Job, and an incremental one: what is embedded is what the index does
    /// not already hold. The first run is the whole library; every later run is
    /// whatever changed, which is usually nothing.
    pub fn build_meaning_index(
        &self,
        progress: Option<&crate::jobs::JobProgress>,
    ) -> Result<Value, OpError> {
        let embedder = match self.meaning.embedder() {
            Some(embedder) => embedder,
            // Not loaded yet, but downloaded: load it here rather than refuse,
            // so accepting, downloading and indexing is three Operations in a
            // row and not three Operations and a restart.
            None if self.meaning.weights_are_present() => {
                self.meaning.load().map_err(OpError::internal)?
            }
            None => {
                return Err(OpError::bad_request(
                    "the Meaning Search model has not been downloaded on this Kamosu",
                ));
            }
        };

        let report = |done: usize, total: usize| {
            if let Some(progress) = progress {
                progress.report(
                    done as u64,
                    Some(total as u64),
                    format!("reading the lines of {done} of {total} recipes"),
                );
            }
        };
        // Waits for the tick if the tick got there first, rather than racing it
        // and putting every recipe in the index twice.
        let _building = self.meaning.building();
        let indexed = crate::meaning::build(&self.db(), &embedder, &report)?;
        self.db().with_conn(crate::meaning::turn_on)?;
        Ok(json!({ "indexed": indexed }))
    }

    /// Turn Meaning Search off. Everything it discards is derived from the
    /// recipes, so nothing is lost that cannot be rebuilt (ADR 0003, ADR 0009)
    /// — which is exactly why this needs no warning and no confirmation.
    pub fn turn_off_meaning_search(&self) -> Result<Value, OpError> {
        // Waits for a build in flight rather than pulling the index out from
        // under it — a build that finished after this ran would turn Meaning
        // Search straight back on.
        let _building = self.meaning.building();
        self.db().with_conn(|conn| {
            crate::meaning::turn_off(conn)?;
            self.meaning.unload();
            Ok(json!({ "state": crate::meaning::state(conn)?.state }))
        })
    }

    /// Bring Meaning Search up at startup, if this instance has it. Answers
    /// whether it is now answering searches.
    pub(crate) fn wake_meaning_search(&self) -> Result<bool, OpError> {
        let held = self.db().with_conn(crate::meaning::state)?;
        if !held.is_on() {
            return Ok(false);
        }
        // An instance can lose its model and be fine (ADR 0029): deleting
        // `/data/model/` returns it to word search and nothing else notices.
        // The index is left alone — the same weights coming back make it good
        // again, and a different revision is pruned on the next build anyway.
        if !self.meaning.weights_are_present() {
            self.db().with_conn(crate::meaning::fell_back)?;
            return Ok(false);
        }
        match self.meaning.load() {
            Ok(_) => Ok(true),
            Err(error) => {
                self.db().with_conn(crate::meaning::fell_back)?;
                Err(OpError::internal(error))
            }
        }
    }

    /// The tick that keeps the index level with the library. Answers how many
    /// recipes it had to read, which is nearly always none.
    pub(crate) fn catch_the_index_up(&self) -> Result<usize, OpError> {
        let Some(embedder) = self.meaning.embedder() else {
            return Ok(0);
        };
        if !self.db().with_conn(crate::meaning::state)?.is_on() {
            return Ok(0);
        }
        // Somebody asked for an index build by name and it is still running.
        // The work is being done; doing it again beside them would only put
        // every recipe in the index twice.
        let Some(_building) = self.meaning.building_now() else {
            return Ok(0);
        };
        crate::meaning::build(&self.db(), &embedder, &|_, _| {})
    }

    /// The shelf, and searching it — one Operation either way (ADR 0027).
    ///
    /// With no `query` this answers the whole shelf: everything the Kitchens
    /// this Person cooks in hold, merged, alphabetical, **one entry per
    /// Lineage**. With a `query` it answers the same shelf narrowed to what
    /// matched, an exact title first, every entry carrying the line that
    /// matched so a surprising result can explain itself.
    ///
    /// One Operation rather than two is not tidiness. The Catalogue cannot
    /// change shape per instance (ADR 0029), so **Meaning Search arrives inside
    /// this Operation** — a second way of finding entries here, never a second
    /// Operation an agent would have to know to look for. An instance with no
    /// model answers exactly the same shape; what changes is how a result can
    /// match, which a result already says.
    ///
    /// The two are **blended into one ranking**, not concatenated: an exact
    /// title wins outright, and after that a confident meaning match outranks a
    /// tag, an ingredient, a step, a Note or an Attempt while a barely-passing
    /// one sits below all of them. Only a title beats meaning. Concatenating
    /// would make *combined* a label rather than a behaviour.
    ///
    /// **No Kitchen name is returned**, by construction rather than by
    /// convention: a card cannot print a fence that is not there (ADR 0027),
    /// and the surest way to keep it off the card is to never send it.
    ///
    /// Both filters are the caller's to pass on each request and are held
    /// nowhere: a filter that persists is a mode, and a mode you forgot you
    /// set is the Kitchen switcher wearing a hat. `tag_id` is the third and
    /// behaves the same way (#104) — it is what makes a Tag something you can
    /// browse by rather than only a word that happens to match. It narrows the
    /// shelf *before* the query runs, so a Tag and a search compose: the eight
    /// recipes tagged *spicy*, and *chicken* among those eight.
    ///
    /// A Lineage passes the Tag filter when **any Branch the shelf just decided
    /// this reader may see** carries it. Tagging is per Branch, and a
    /// Translation is a Branch (ADR 0006), so a recipe tagged in French and
    /// read in English is still the recipe you tagged.
    ///
    /// This reads every visible recipe and matches in Rust rather than asking
    /// SQLite. That is honest for a library of this size — the real one is 86
    /// recipes — and it is what keeps the fold below the same fold the rest of
    /// Kamosu uses. The meaning half scans its whole index for the same reason:
    /// a couple of megabytes of vectors cost less to walk than the one model
    /// inference that produced the query.
    pub fn search_recipes(
        &self,
        person_id: &str,
        query: Option<&str>,
        kitchen_id: Option<&str>,
        mine: bool,
        tag_id: Option<&str>,
    ) -> Result<Value, OpError> {
        let query = query.map(str::trim).filter(|q| !q.is_empty());
        let needle = query.map(folded_for_search);

        // The query is embedded here, outside the database lock, because model
        // inference is tens of milliseconds and the lock is the whole
        // instance's. A model that is not loaded is not an error — it is the
        // ordinary state of an instance that never turned Meaning Search on.
        let asked: Option<Vec<f32>> = match (query, self.meaning.embedder()) {
            (Some(query), Some(embedder)) => embedder.embed_query(query).ok(),
            _ => None,
        };

        self.db().with_conn(|conn| {
            if let Some(kitchen_id) = kitchen_id {
                ensure_member(conn, kitchen_id, person_id)?;
            }
            // A Tag of a Cookbook the caller may not see is not here at all,
            // rather than here and refused (ADR 0040) — the same answer
            // `set_recipe_tag` gives for the same id.
            let tagged = match tag_id {
                Some(tag_id) => {
                    let of_cookbook = cookbook_of_tag(conn, tag_id)?;
                    ensure_sees_or_absent(conn, &of_cookbook, person_id, no_such_tag)?;
                    Some(branches_with_tag(conn, tag_id, person_id)?)
                }
                None => None,
            };
            let reading_language = reading_language_of(conn, person_id)?;
            let (lineages, branches) = shelf_of(conn, person_id, kitchen_id, &reading_language)?;

            // Every Lineage's best block, against this query — over exactly the
            // Branches the shelf has just decided this reader may see, filters
            // included. Handing that set down rather than asking the permission
            // question again is what keeps one Lineage's other Branch, in a
            // Cookbook you may not see, out of your ranking and off your card
            // (ADR 0026, ADR 0027).
            let visible: HashSet<&str> = branches
                .values()
                .flatten()
                .map(|branch| branch.branch_id.as_str())
                .collect();
            let meaning: HashMap<String, crate::meaning::Hit> = match &asked {
                Some(asked) if crate::meaning::state(conn)?.is_on() => {
                    crate::meaning::nearest(conn, asked, person_id, &visible)?
                }
                _ => HashMap::new(),
            };

            // What a match is worth, in one number, so words and meaning are
            // ordered against each other rather than one after the other. An
            // exact title scores 1.0, above anything meaning can reach, which
            // is how "exact title first" survives having a model in the room.
            let mut entries: Vec<(f32, String, Value)> = Vec::new();
            // Every Lineage the model had *something* to say about, however
            // faint. Read only when nothing matched at all — meaning-matching
            // always has a nearest neighbour, so *nothing found* means *nothing
            // close enough*, and Kamosu says exactly that and shows it anyway
            // (ADR 0027).
            let mut nearby: Vec<(f32, String, Value)> = Vec::new();
            for lineage_id in lineages {
                // *My recipes* is a history, not an ownership: created,
                // branched or cooked. The first two are one fact — this
                // Person's Hand on a Version of this Lineage — and the third
                // is an Attempt of their own (ADR 0027).
                if mine && !written_or_cooked_by(conn, &lineage_id, person_id)? {
                    continue;
                }

                let of_lineage = &branches[&lineage_id];
                // The Tag filter, asked of every Branch this reader may see
                // rather than only the one the card opens: a recipe tagged on
                // its French Branch and read in English is the same recipe.
                if let Some(tagged) = &tagged
                    && !of_lineage
                        .iter()
                        .any(|branch| tagged.contains(&branch.branch_id))
                {
                    continue;
                }
                let shown = &of_lineage[0];
                let content = version_content(conn, &shown.head_version_id)?;
                let title = content["title"].as_str().unwrap_or_default().to_string();

                let hit = meaning.get(&lineage_id);
                let matched = match &needle {
                    None => None,
                    Some(needle) => {
                        // Searched across **every** Branch of this Lineage, not
                        // only the one the card opens. A Translation is a Branch
                        // of the same Lineage (ADR 0006), and it is that
                        // multilingual model that makes the match: type
                        // *chocolat* and the recipe whose English rendering you
                        // are shown must still be found. The best rung any
                        // Branch reaches is the one the entry carries.
                        let mut best: Option<(u8, Value)> = None;
                        for branch in of_lineage {
                            let read = if branch.branch_id == shown.branch_id {
                                content.clone()
                            } else {
                                version_content(conn, &branch.head_version_id)?
                            };
                            let branch_title = read["title"].as_str().unwrap_or_default();
                            let found =
                                match_recipe(conn, branch, person_id, &read, branch_title, needle)?;
                            if let Some(found) = found
                                && best.as_ref().is_none_or(|(rank, _)| found.0 < *rank)
                            {
                                best = Some(found);
                            }
                        }
                        best
                    }
                };

                // The unsearched shelf is every recipe, in one flat rank: this
                // screen opens on it before anybody has typed anything.
                if needle.is_none() {
                    entries.push((
                        0.0,
                        folded_for_search(&title),
                        shelf_entry(
                            &lineage_id,
                            shown,
                            &title,
                            &reading_language,
                            &content,
                            None,
                        ),
                    ));
                    continue;
                }

                // **The one ranking.** Both halves are scored onto the same
                // scale and the better of the two both places the entry and
                // chooses the line it quotes — so a result is always standing
                // where the line it shows put it.
                let words = matched
                    .as_ref()
                    .map(|(rank, _)| crate::meaning::word_score(*rank));
                let sense = hit.and_then(|hit| crate::meaning::score(hit.similarity));
                let by_words = match (words, sense) {
                    (None, None) => {
                        // Neither half found it. It is not on this answer — but
                        // it may still be the closest thing there is, and
                        // meaning-matching always has a nearest neighbour, so
                        // *nothing found* means *nothing close enough* and
                        // Kamosu shows the closest anyway under that label
                        // (ADR 0027).
                        if let Some(hit) = hit {
                            nearby.push((
                                hit.similarity,
                                folded_for_search(&title),
                                shelf_entry(
                                    &lineage_id,
                                    shown,
                                    &title,
                                    &reading_language,
                                    &content,
                                    Some(matched_by(&hit.matched, "meaning")),
                                ),
                            ));
                        }
                        continue;
                    }
                    (Some(words), sense) => sense.is_none_or(|sense| words >= sense),
                    (None, Some(_)) => false,
                };

                let (score, quoted) = if by_words {
                    let (rank, line) = matched.expect("a word match, by the arm above");
                    (crate::meaning::word_score(rank), matched_by(&line, "words"))
                } else {
                    let hit = hit.expect("a meaning match, by the arm above");
                    (
                        sense.expect("a meaning match, by the arm above"),
                        matched_by(&hit.matched, "meaning"),
                    )
                };

                entries.push((
                    score,
                    folded_for_search(&title),
                    shelf_entry(
                        &lineage_id,
                        shown,
                        &title,
                        &reading_language,
                        &content,
                        Some(quoted),
                    ),
                ));
            }

            // Best first, and alphabetical inside one score — which for the
            // unsearched shelf, where every entry scores the same, is the whole
            // order. A list that reorders itself between visits cannot be
            // learned by the thumb.
            entries.sort_by(|a, b| {
                b.0.partial_cmp(&a.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.1.cmp(&b.1))
            });

            // Nothing found is not an empty screen. Where Meaning Search is on
            // there is always a nearest neighbour, so the closest few are shown
            // under exactly that label rather than silently passed off as
            // matches — the same refusal that keeps a weak match from wearing a
            // strong one's clothes (ADR 0027).
            let closest = entries.is_empty() && !nearby.is_empty();
            if closest {
                nearby.sort_by(|a, b| {
                    b.0.partial_cmp(&a.0)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.1.cmp(&b.1))
                });
                entries = nearby.into_iter().take(CLOSEST_SHOWN).collect();
            }

            Ok(json!({
                "query": query,
                // True where nothing was close enough and these are the nearest
                // anyway. The screen says so; it never shows them as matches.
                "closest": closest,
                "recipes": entries.into_iter().map(|(_, _, entry)| entry).collect::<Vec<_>>(),
            }))
        })
    }
}

/// Bring Meaning Search up, if this instance has it, and then keep its index
/// level with the library.
///
/// Loading the model takes seconds, so it happens beside the first answers
/// rather than in front of them: an instance serves immediately and searches by
/// words until the model is ready, which is exactly what it does on every
/// instance that never turns Meaning Search on at all.
pub(super) fn spawn_meaning_search(core: Arc<Core>) {
    tokio::spawn(async move {
        {
            let core = core.clone();
            let loaded = tokio::task::spawn_blocking(move || core.wake_meaning_search()).await;
            match loaded {
                Ok(Ok(true)) => tracing::info!(target: "kamosu::meaning", "Meaning Search is on"),
                Ok(Ok(false)) => {}
                Ok(Err(error)) => {
                    tracing::warn!(target: "kamosu::meaning", %error, "Meaning Search stayed off")
                }
                Err(error) => {
                    tracing::warn!(target: "kamosu::meaning", %error, "Meaning Search panicked")
                }
            }
        }

        let mut ticker = tokio::time::interval(Core::REINDEX_EVERY);
        loop {
            ticker.tick().await;
            let core = core.clone();
            let built = tokio::task::spawn_blocking(move || core.catch_the_index_up()).await;
            match built {
                Ok(Ok(0)) => {}
                Ok(Ok(done)) => {
                    tracing::info!(target: "kamosu::meaning", done, "index caught up")
                }
                Ok(Err(error)) => {
                    tracing::warn!(target: "kamosu::meaning", %error, "index did not catch up")
                }
                Err(error) => {
                    tracing::warn!(target: "kamosu::meaning", %error, "index catch-up panicked")
                }
            }
        }
    });
}

/// The Branches carrying one Tag, for the shelf's Tag filter (#104).
///
/// Read once before the shelf's loop rather than asked per Lineage: the filter
/// is a set membership test, and forty recipes would otherwise be forty
/// queries.
///
/// **By its word, across every Cookbook the reader may see** (#131, question
/// 3). A shelf mixing several Cookbooks shows "Dessert" once however many of
/// them file by it, so filtering by it finds the recipes each of them filed
/// there — any Tag sharing one of this one's words, in the same Language.
fn branches_with_tag(
    conn: &rusqlite::Connection,
    tag_id: &str,
    person_id: &str,
) -> Result<HashSet<String>, OpError> {
    let mut statement = conn
        .prepare(
            "SELECT branch_tags.branch_id FROM branch_tags \
              WHERE branch_tags.tag_id IN ( \
                    SELECT alike.tag_id FROM tag_names AS alike \
                      JOIN tag_names AS this \
                        ON this.language = alike.language AND this.name_folded = alike.name_folded \
                     WHERE this.tag_id = ?1 \
                       AND alike.cookbook_id IN \
                           (SELECT cookbook_id FROM visible_cookbooks WHERE person_id = ?2))",
        )
        .map_err(|e| OpError::internal(format!("cannot read a Tag's recipes: {e}")))?;
    statement
        .query_map(params![tag_id, person_id], |row| row.get(0))
        .map_err(|e| OpError::internal(format!("cannot read a Tag's recipes: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read a Tag's recipes: {e}")))
}

/// The fold two spellings of one word share **for the purpose of finding it**
/// — [`folded_word`]'s canonical caseless fold, and then combining marks
/// dropped, so *gateau* typed in a hurry finds *Gâteau*.
///
/// It is deliberately a second, looser rule, and the two are not
/// interchangeable. [`folded_word`] answers *is this the same word* — the
/// question a Tag and a Food are held unique by — and there *é* and *e* are
/// genuinely different words. This one answers *did the cook mean this*, where
/// refusing a match over an accent the phone keyboard buried three presses deep
/// is a search that looks broken. Nothing is stored in this form; it exists
/// only for the length of one comparison.
///
/// Public so the corpus test can assert the real shelf's order against *this*
/// rule rather than against a copy of it that would go stale the moment the
/// rule changed.
pub fn folded_for_search(text: &str) -> String {
    use unicode_normalization::char::is_combining_mark;
    folded_word(text)
        .chars()
        .filter(|c| !is_combining_mark(*c))
        .collect()
}

/// Whether this Person created, branched or cooked this Lineage — the three
/// things *My recipes* means, and all three already stored (ADR 0027).
/// Created and branched are one fact: their Hand is on a Version of it.
fn written_or_cooked_by(
    conn: &rusqlite::Connection,
    lineage_id: &str,
    person_id: &str,
) -> Result<bool, OpError> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM branch_versions \
                          JOIN branches ON branches.id = branch_versions.branch_id \
                         WHERE branches.lineage_id = ?1 AND branch_versions.hand_id = ?2) \
             OR EXISTS (SELECT 1 FROM attempts \
                         WHERE attempts.lineage_id = ?1 AND attempts.person_id = ?2)",
        params![lineage_id, person_id],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot read what this Person has cooked: {e}")))
}

/// How many recipes *nothing found* shows anyway, when Meaning Search is on and
/// nothing was close enough. Enough that the screen is not empty, few enough
/// that it never reads as a list of results.
const CLOSEST_SHOWN: usize = 3;

/// One quoted line, saying which half of searching found it.
///
/// A reader cannot tell a meaning match from a word match by looking at the
/// line, and a surprising result is exactly where trust is won or lost — so the
/// answer says which it was rather than leaving the screen to guess.
fn matched_by(line: &Value, by: &str) -> Value {
    let mut matched = line.clone();
    matched["by"] = json!(by);
    matched
}

/// What in one recipe carries the words searched for, and the line to quote for
/// it — a result that cannot explain itself is noise (ADR 0027).
///
/// The order below is the ranking, and its first rung is the one the whole
/// design rests on: **an exact title wins**, because most searching is
/// navigation — you know the recipe's name and you are typing it to get there.
/// After that, the more of the recipe a match had to reach into, the further
/// down it sits.
///
/// The last rung reaches this Person's own Attempts and nobody else's. A
/// Kitchen-mate's *burnt it again, honestly* turning up when you search *burnt*
/// is a different act from her showing you.
fn match_recipe(
    conn: &rusqlite::Connection,
    branch: &ShelfBranch,
    person_id: &str,
    content: &Value,
    title: &str,
    needle: &str,
) -> Result<Option<(u8, Value)>, OpError> {
    let carries = |text: &str| folded_for_search(text).contains(needle);
    let quote = |rank: u8, where_: &str, line: &str, step_number: Option<usize>| {
        Some((
            rank,
            json!({ "where": where_, "line": line, "step_number": step_number }),
        ))
    };

    if folded_for_search(title) == needle {
        return Ok(quote(0, "title", title, None));
    }
    if carries(title) {
        return Ok(quote(1, "title", title, None));
    }

    for tag in tags_of_branch(conn, &branch.branch_id, person_id)? {
        if let Some(name) = tag["name"].as_str()
            && carries(name)
        {
            return Ok(quote(2, "tag", name, None));
        }
    }

    // A Section header — "For the sauce" — is a line of the recipe like any
    // other and is searched with the rest, but it is not an ingredient and not
    // a step, so it is answered as what it is rather than mislabelled as one.
    //
    // A Step's number is counted the way the recipe page counts it: over the
    // Steps alone, Sections taking none. Handing back the position in the array
    // instead would have a Section header silently shift every "Step 6" that
    // follows it by one, and a result that points at the wrong step is worse
    // than one that points nowhere.
    for (kind, rank, where_) in [("ingredients", 3, "ingredient"), ("steps", 4, "step")] {
        if let Some(lines) = content[kind].as_array() {
            let mut number = 0;
            for line in lines {
                let section = line["kind"].as_str() == Some("section");
                if !section {
                    number += 1;
                }
                if let Some(text) = line["text"].as_str()
                    && carries(text)
                {
                    return Ok(if section {
                        quote(rank, "section", text, None)
                    } else {
                        quote(rank, where_, text, (kind == "steps").then_some(number))
                    });
                }
            }
        }
    }

    if let Some(note) = content["note"].as_str()
        && carries(note)
    {
        return Ok(quote(5, "note", note, None));
    }

    let attempts =
        |e: rusqlite::Error| OpError::internal(format!("cannot read this Person's Attempts: {e}"));
    let mut statement = conn
        .prepare(
            "SELECT note FROM attempts \
              WHERE lineage_id = ?1 AND person_id = ?2 AND note IS NOT NULL \
              ORDER BY created_at DESC",
        )
        .map_err(attempts)?;
    let notes: Vec<String> = statement
        .query_map(params![branch.lineage_id, person_id], |row| row.get(0))
        .map_err(attempts)?
        .collect::<Result<_, _>>()
        .map_err(attempts)?;
    for note in &notes {
        if carries(note) {
            return Ok(quote(6, "attempt", note, None));
        }
    }

    Ok(None)
}
