//! Cooking: Attempts, from starting one to its judgement, and promoting what
//! was cooked back into the recipe.

use super::*;

impl Core {
    // --- Cooking it: Attempts (#57, ADR 0005, ADR 0010) ---------------------

    /// Start cooking a Recipe — creates the Attempt, or hands back the one
    /// already In Progress for this Lineage: the cooking screen *is* that
    /// Attempt while it lives, so starting twice is the same Attempt seen
    /// twice, never a second one. Pinned by fingerprint to the Branch's head
    /// Version at this moment (ADR 0005) — a later edit to the recipe never
    /// turns this Attempt into a lie. "May I see this recipe" is the whole
    /// permission this needs, the same membership `get_recipe` checks.
    ///
    /// `version_id`, given, cooks an older Version read back from the Thread
    /// instead of the head — it must actually be one of this Branch's own
    /// Versions, so an Attempt can never be pinned to content the caller
    /// never had in front of them.
    ///
    /// `attempt_id` and `started_at` are how a cooking started with no
    /// network arrives (#77, ADR 0013). The phone names the Attempt itself,
    /// so everything it did afterwards can say which cooking it belongs to
    /// before the server has ever heard of it; sending the same start twice
    /// hands back the same Attempt. Where the Lineage already has one In
    /// Progress, begun on another device while this one was out, that one
    /// is handed back as always and the phone follows it. Two devices are
    /// one cooking staying in step, and nobody is asked which they meant.
    pub fn start_attempt(
        &self,
        person_id: &str,
        branch_id: &str,
        version_id: Option<&str>,
        attempt_id: Option<&str>,
        started_at: Option<&str>,
    ) -> Result<Value, OpError> {
        if let Some(id) = attempt_id
            && !is_minted_attempt_id(id)
        {
            return Err(OpError::bad_request(
                "attempt_id must be at_ followed by sixteen lower-case hex digits",
            ));
        }
        self.db().with_conn(|conn| {
            let (lineage_id, cookbook_id, head_version_id): (String, String, String) = conn
                .query_row(
                    "SELECT lineage_id, cookbook_id, head_version_id FROM branches WHERE id = ?1",
                    params![branch_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?
                .ok_or_else(no_such_branch)?;
            ensure_sees_or_absent(conn, &cookbook_id, person_id, no_such_branch)?;

            let pinned_version_id = match version_id {
                None => head_version_id,
                Some(requested) => {
                    let on_this_branch: i64 = conn
                        .query_row(
                            "SELECT COUNT(*) FROM branch_versions \
                              WHERE branch_id = ?1 AND version_id = ?2",
                            params![branch_id, requested],
                            |row| row.get(0),
                        )
                        .map_err(|e| OpError::internal(format!("cannot read Branch chain: {e}")))?;
                    if on_this_branch == 0 {
                        return Err(OpError::bad_request(
                            "version_id is not a Version of this Branch",
                        ));
                    }
                    requested.to_string()
                }
            };

            let moment = written_moment(conn, started_at)?;

            // The same start sent twice: a phone that sent it and never heard
            // the answer sends it again, and must get the same cooking back.
            if let Some(id) = attempt_id {
                let owner: Option<String> = conn
                    .query_row(
                        "SELECT person_id FROM attempts WHERE id = ?1",
                        params![id],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(|e| OpError::internal(format!("cannot read Attempt: {e}")))?;
                match owner {
                    Some(owner) if owner == person_id => return attempt_by_id(conn, id),
                    Some(_) => return Err(OpError::bad_request("that attempt_id is taken")),
                    None => {}
                }
            }

            if let Some(existing) = in_progress_attempt(conn, &lineage_id, person_id)? {
                let id = existing["id"].as_str().expect("id is always a string");
                touch_attempt_at(conn, id, &moment)?;
                return attempt_by_id(conn, id);
            }

            let id = attempt_id
                .map(str::to_string)
                .unwrap_or_else(|| format!("at_{}", hex::encode(random_bytes(8))));
            conn.execute(
                "INSERT INTO attempts (id, lineage_id, person_id, version_id, created_at, last_action_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![id, lineage_id, person_id, pinned_version_id, moment],
            )
            .map_err(|e| OpError::internal(format!("cannot start Attempt: {e}")))?;
            attempt_by_id(conn, &id)
        })
    }

    /// Move an In Progress Attempt forward: which Step, which Ingredients
    /// are ticked, and the Yield being cooked to — a fact about this
    /// cooking, held on the Attempt and never written as a deviation
    /// (ADR 0010). Each of the three is sent whole, the same convention
    /// `save_recipe_version` and `set_reading` use — never a per-field
    /// patch — and any absent one is simply left as it stood.
    ///
    /// **The last one moved on is where the cook is** (ADR 0010), and since
    /// #77 that is decided by when a move was made rather than when it
    /// arrived. A move written with no signal and sent hours later, after the
    /// iPad has moved the same cooking on, is older than where the cook now
    /// stands, so it changes nothing and the Attempt comes back as it is.
    pub fn advance_attempt(
        &self,
        person_id: &str,
        attempt_id: &str,
        current_step_index: Option<i64>,
        ticked_ingredients: Option<&[i64]>,
        cooking_yield: Option<&Value>,
        written_at: Option<&str>,
    ) -> Result<Value, OpError> {
        if current_step_index.is_none() && ticked_ingredients.is_none() && cooking_yield.is_none() {
            return Err(OpError::bad_request(
                "advance_attempt takes at least one of current_step_index, \
                 ticked_ingredients, cooking_yield",
            ));
        }
        if current_step_index.is_some_and(|index| index < 0) {
            return Err(OpError::bad_request(
                "current_step_index must be zero or more",
            ));
        }

        self.db().with_conn(|conn| {
            let state = attempt_state_owned_by(conn, attempt_id, person_id)?;
            if state.finished_at.is_some() {
                // A move a phone held with no network, arriving after the
                // cooking was finished on another device: a finished cooking
                // is final, so it changes nothing, like any move older than
                // where the cook stands (#77). Asked live, it is a mistake.
                if written_at.is_some() {
                    return attempt_by_id(conn, attempt_id);
                }
                return Err(OpError::bad_request("this Attempt has already finished"));
            }
            let moment = written_moment(conn, written_at)?;
            if state
                .moved_at
                .as_deref()
                .is_some_and(|moved| moment.as_str() < moved)
            {
                return attempt_by_id(conn, attempt_id);
            }
            let version_id = state.version_id;

            if let Some(index) = current_step_index {
                // Against the recipe THIS COOKING is walking through, which is
                // the As Cooked once there is one (#58). A cook who inserted a
                // step has more places to stand than the Version has, and
                // validating against the Version would refuse the last of them
                // — silently, because the screen puts a refused move back.
                let steps_len = version_field_len(
                    conn,
                    cooking_version_id(conn, attempt_id)?
                        .as_deref()
                        .unwrap_or(&version_id),
                    "steps",
                )?;
                if index >= steps_len {
                    return Err(OpError::bad_request(
                        "current_step_index is out of range for this recipe",
                    ));
                }
                conn.execute(
                    "UPDATE attempts SET current_step_index = ?2 WHERE id = ?1",
                    params![attempt_id, index],
                )
                .map_err(|e| OpError::internal(format!("cannot advance Attempt: {e}")))?;
            }
            if let Some(indices) = ticked_ingredients {
                let ingredients_len = version_field_len(conn, &version_id, "ingredients")?;
                if indices.iter().any(|&i| i < 0 || i >= ingredients_len) {
                    return Err(OpError::bad_request(
                        "a ticked Ingredient index is out of range for this recipe",
                    ));
                }
                let stored = serde_json::to_string(indices).expect("serialisable indices");
                conn.execute(
                    "UPDATE attempts SET ticked_ingredients = ?2 WHERE id = ?1",
                    params![attempt_id, stored],
                )
                .map_err(|e| OpError::internal(format!("cannot tick Ingredients: {e}")))?;
            }
            if let Some(value) = cooking_yield {
                let stored = match value {
                    Value::Null => None,
                    other => Some(
                        serde_json::to_string(&parse_wanted_yield(other)?)
                            .expect("serialisable Yield"),
                    ),
                };
                conn.execute(
                    "UPDATE attempts SET cooking_yield = ?2 WHERE id = ?1",
                    params![attempt_id, stored],
                )
                .map_err(|e| OpError::internal(format!("cannot set the cooking Yield: {e}")))?;
            }

            conn.execute(
                "UPDATE attempts SET moved_at = ?2 WHERE id = ?1",
                params![attempt_id, moment],
            )
            .map_err(|e| OpError::internal(format!("cannot advance Attempt: {e}")))?;
            touch_attempt_at(conn, attempt_id, &moment)?;
            attempt_by_id(conn, attempt_id)
        })
    }

    /// End an In Progress Attempt, and take the judgement that lands with it
    /// (#59): a **rating**, a **note** and **Photographs**, each optional and
    /// each sent whole. Ending is not what makes the cooking real — starting
    /// already did (ADR 0010) — only what stops it being In Progress, which is
    /// why every one of the three may be left out and the cooking still
    /// counts.
    ///
    /// Finishing writes the same three fields `edit_attempt` does, through
    /// the same code, so that filling them in at the stove and correcting them
    /// a week later cannot validate differently.
    ///
    /// `written_at` is when the cook finished, for a finish that waited on a
    /// phone with no signal (#77): the cooking ended on Saturday, whenever
    /// Monday's connection delivered it.
    pub fn finish_attempt(
        &self,
        person_id: &str,
        attempt_id: &str,
        judgement: Judgement<'_>,
        written_at: Option<&str>,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let state = attempt_state_owned_by(conn, attempt_id, person_id)?;
            if state.finished_at.is_some() {
                // Finished twice: once on each device, or a finish sent again
                // by a phone that never heard the answer to the first (#77).
                // Either way it is one cooking finished once, so the second
                // finish keeps the first one's time. What the cook said with
                // it — a rating, a note, pictures — is still theirs to say,
                // exactly as a late `edit_attempt` would land it, and a phone's
                // sending does not fail over it.
                if written_at.is_some() {
                    write_attempt_judgement(conn, attempt_id, &judgement)?;
                    return attempt_by_id(conn, attempt_id);
                }
                return Err(OpError::bad_request("this Attempt has already finished"));
            }
            let moment = written_moment(conn, written_at)?;
            write_attempt_judgement(conn, attempt_id, &judgement)?;
            conn.execute(
                "UPDATE attempts SET finished_at = ?2, \
                                      last_action_at = max(last_action_at, ?2) \
                 WHERE id = ?1",
                params![attempt_id, moment],
            )
            .map_err(|e| OpError::internal(format!("cannot finish Attempt: {e}")))?;
            attempt_by_id(conn, attempt_id)
        })
    }

    /// Change an Attempt's free text, its rating or its Photographs, whether
    /// it is still In Progress or long finished — an Attempt is freely
    /// editable by its cook (GLOSSARY.md, "Attempt"), unlike the recipe it was
    /// cooked from. `None` leaves a field as it stood; `Some(&Value::Null)`
    /// clears it; any other value sets it, validated.
    pub fn edit_attempt(
        &self,
        person_id: &str,
        attempt_id: &str,
        judgement: Judgement<'_>,
        written_at: Option<&str>,
    ) -> Result<Value, OpError> {
        if judgement.is_empty() {
            return Err(OpError::bad_request(
                "edit_attempt takes at least one of note, rating, photographs, add_photographs",
            ));
        }
        self.db().with_conn(|conn| {
            let state = attempt_state_owned_by(conn, attempt_id, person_id)?;
            let moment = written_moment(conn, written_at)?;
            write_attempt_judgement(conn, attempt_id, &judgement)?;

            // Correcting a note or a rating mid-cook is itself an action —
            // the resume window counts from it exactly as advancing a Step
            // does. Once finished, resuming is never offered regardless, so
            // there is nothing to refresh.
            if state.finished_at.is_none() {
                touch_attempt_at(conn, attempt_id, &moment)?;
            }
            attempt_by_id(conn, attempt_id)
        })
    }

    /// Delete an Attempt outright — the explicit way a false start is
    /// undone, or any cooking record put away (ADR 0010). Never
    /// soft-deleted: this is the whole of how an Attempt leaves.
    pub fn delete_attempt(&self, person_id: &str, attempt_id: &str) -> Result<(), OpError> {
        self.db().with_conn(|conn| {
            attempt_state_owned_by(conn, attempt_id, person_id)?;
            conn.execute("DELETE FROM attempts WHERE id = ?1", params![attempt_id])
                .map_err(|e| OpError::internal(format!("cannot delete Attempt: {e}")))?;
            Ok(())
        })
    }

    /// Promote a Photograph taken while cooking to the recipe's **Main Photo**
    /// or to a **Step's photo**, so the picture you actually took becomes the
    /// recipe's picture (#59).
    ///
    /// This is the one deliberate way a picture crosses from a private cooking
    /// record into the recipe everybody holds, and it is **an ordinary edit
    /// making a Version** — not a special move. It goes through
    /// `save_recipe_version` for exactly that reason: the Main Photo and a
    /// Step's photo are part of the fingerprint (#45), so promoting one is the
    /// same act as rewording a step, and inherits the whole of it — the
    /// collapse window, carrying Readings forward, and taking a **Copy** where
    /// the Branch was written under another Hand (ADR 0007). A Branch held by
    /// a Kitchen the caller does not cook in is refused as absent (#100).
    ///
    /// The Attempt itself is untouched. The picture stays on the cooking
    /// record as well, because a promotion is not a move: the same Photograph
    /// is one stored picture however many things point at it (ADR 0017).
    #[allow(clippy::too_many_arguments)]
    pub fn promote_attempt_photograph(
        &self,
        caller: &Caller,
        attempt_id: &str,
        photograph_id: &str,
        branch_id: &str,
        step_index: Option<i64>,
        change_note: Option<&str>,
    ) -> Result<Value, OpError> {
        let content = self.db().with_conn(|conn| {
            // Yours to promote from: an Attempt is a private record, and
            // reaching into somebody else's for a picture is not a promotion.
            attempt_state_owned_by(conn, attempt_id, &caller.person_id)?;

            // The picture has to be one this cooking actually holds. Promoting
            // an arbitrary Photograph id would make this a second, quieter way
            // to set the Main Photo with none of `save_recipe_version`'s
            // shape checks in front of it.
            if !attempt_photographs(conn, attempt_id)?
                .iter()
                .any(|held| held == photograph_id)
            {
                return Err(OpError::bad_request(
                    "that Photograph is not one of this Attempt's",
                ));
            }

            // The recipe being promoted into must be the dish that was
            // cooked. An Attempt belongs to a Lineage rather than a Branch
            // (ADR 0005), so any Branch of that Lineage one of the caller's
            // Kitchens holds is a legitimate target — including a Translation,
            // and including an arrived one, which `save_recipe_version` will
            // turn into a Copy. One held elsewhere answers as absent before
            // its Lineage is compared, or the comparison would say it exists
            // (#100, ADR 0040).
            let (lineage_id, head_version_id, cookbook_id): (String, String, String) = conn
                .query_row(
                    "SELECT lineage_id, head_version_id, cookbook_id FROM branches WHERE id = ?1",
                    params![branch_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?
                .ok_or_else(no_such_branch)?;
            ensure_sees_or_absent(conn, &cookbook_id, &caller.person_id, no_such_branch)?;
            let attempt_lineage: String = conn
                .query_row(
                    "SELECT lineage_id FROM attempts WHERE id = ?1",
                    params![attempt_id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot read Attempt: {e}")))?;
            if lineage_id != attempt_lineage {
                return Err(OpError::bad_request(
                    "that Branch is not a Branch of the recipe this Attempt cooked",
                ));
            }

            let stored: String = conn
                .query_row(
                    "SELECT content FROM versions WHERE id = ?1",
                    params![head_version_id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot read Version: {e}")))?;
            let content = serde_json::from_str::<Value>(&stored)
                .map_err(|e| OpError::internal(format!("cannot read Version content: {e}")))?;

            // The name the head Version carries, if any. Carried through the
            // save below because a promotion inside the collapse window folds
            // into that very Version, and a save naming nothing writes its name
            // away — so promoting a picture into a Version somebody had named
            // would quietly un-name it.
            let head_name: Option<String> = conn
                .query_row(
                    "SELECT name FROM branch_versions \
                      WHERE branch_id = ?1 ORDER BY sequence DESC LIMIT 1",
                    params![branch_id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot read Version name: {e}")))?;

            Ok((content, head_name))
        })?;
        let (content, head_name) = content;

        // The edit itself: the head Version as it stands with one photo
        // changed. `save_recipe_version` re-parses this whole shape, so
        // nothing here has to be trusted.
        let mut edited = content.clone();
        match step_index {
            None => {
                // `edited["main_photo"] = …` would panic where a Version's
                // content is not an object at all — a damaged row is a failure
                // to report, never a crash.
                edited
                    .as_object_mut()
                    .ok_or_else(|| OpError::internal("a Version's content is not readable"))?
                    .insert("main_photo".into(), json!(photograph_id));
            }
            Some(index) => {
                let steps = edited["steps"]
                    .as_array_mut()
                    .ok_or_else(|| OpError::internal("a Version has no steps"))?;
                let step = usize::try_from(index)
                    .ok()
                    .and_then(|index| steps.get_mut(index))
                    .ok_or_else(|| {
                        OpError::bad_request("step_index is out of range for this recipe")
                    })?;
                if step["kind"] != json!("step") {
                    return Err(OpError::bad_request(
                        "step_index names a section heading rather than a Step",
                    ));
                }
                step["photo"] = json!(photograph_id);
            }
        }

        self.save_recipe_version(
            caller,
            branch_id,
            &edited,
            head_name.as_deref(),
            change_note,
            None,
        )
    }

    /// Write this cooking's **As Cooked**: the complete recipe state the cook
    /// actually cooked, held only where it differed from the Version they
    /// started from (ADR 0005).
    ///
    /// It is not a record of what changed. What is stored is a whole recipe —
    /// ordinary Ingredient Lines and ordinary Step text, in the same shape a
    /// Version takes, going through the same `parse_recipe_content` — so a
    /// cook who added a line, removed one, or grew a step has said so in the
    /// only vocabulary Kamosu has (ADR 0002). `None` clears it.
    ///
    /// **Cooked as written stores nothing, and that is enforced here rather
    /// than trusted to the screen.** A Version is named by a fingerprint of
    /// its content, so content identical to the Version this Attempt pinned to
    /// fingerprints to that same Version — and the column is set to NULL
    /// instead. A client that helpfully posts the whole recipe back unchanged
    /// on every keystroke therefore stores no As Cooked at all, which is the
    /// overwhelming majority of cookings and the reason the common case is
    /// free.
    ///
    /// **No Reading is computed here**, unlike `save_recipe_version`. Reading
    /// a line is work, this runs while somebody is typing at a stove, and
    /// nothing consumes an As Cooked's Readings: the cooking screen draws
    /// which Ingredients a Step uses off the pinned Version, because rewriting
    /// *4 tbsp* as *2 tbsp* does not change which step uses the soy sauce. The
    /// Readings arrive at Promotion, from `save_recipe_version`, on the one
    /// path where they are actually read.
    pub fn set_as_cooked(
        &self,
        person_id: &str,
        attempt_id: &str,
        content: Option<&Value>,
        written_at: Option<&str>,
    ) -> Result<Value, OpError> {
        // Parsed outside the connection: a malformed recipe is a bad request
        // that touches nothing, and shape checking has no business holding the
        // database lock.
        let written = match content {
            None | Some(Value::Null) => None,
            Some(value) => {
                let parsed = parse_recipe_content(value)?;
                Some(stored_version(&parsed))
            }
        };

        self.db().with_conn(|conn| {
            let state = attempt_state_owned_by(conn, attempt_id, person_id)?;

            // The Version cooked, as it is actually stored, compared WORD FOR
            // WORD. Both sides go through `parse_recipe_content` first, so the
            // comparison is between two recipes rather than between two
            // encodings of one — whatever normalising the parser does, it does
            // to the input and to what is on disk alike.
            //
            // #58 wrote it this way because comparing ids was, at the time,
            // wrong: `nutrition` had been added to the recipe (#72) and a
            // Version saved before that day fingerprinted without it, so an
            // identical cooking compared unequal and stored a whole recipe for
            // itself — the one thing ADR 0005 promises never happens. ADR 0038
            // has since removed that whole class of mismatch, and the ids
            // would now answer identically. This stays as it is because it
            // costs one parse and does not lean on the invariant holding.
            let cooked_from = conn
                .query_row(
                    "SELECT content FROM versions WHERE id = ?1",
                    params![state.version_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read the Version cooked: {e}")))?
                .and_then(|stored| serde_json::from_str::<Value>(&stored).ok())
                .and_then(|content| parse_recipe_content(&content).ok())
                .map(|content| canonical_json(&content));

            let as_cooked_version_id = match &written {
                None => None,
                // Cooked as written after all, so nothing is stored — however
                // faithfully the screen posted the recipe back.
                Some((_, content_text)) if Some(content_text) == cooked_from.as_ref() => None,
                Some((version_id, content_text)) => {
                    conn.execute(
                        "INSERT OR IGNORE INTO versions (id, content) VALUES (?1, ?2)",
                        params![version_id, content_text],
                    )
                    .map_err(|e| OpError::internal(format!("cannot record As Cooked: {e}")))?;
                    Some(version_id.clone())
                }
            };

            conn.execute(
                "UPDATE attempts SET as_cooked_version_id = ?2 WHERE id = ?1",
                params![attempt_id, as_cooked_version_id],
            )
            .map_err(|e| OpError::internal(format!("cannot record As Cooked: {e}")))?;

            // Writing down what you did is an action like any other, so the
            // three-day resume window counts from it. Once finished there is
            // nothing to resume, and correcting the record a week later must
            // not pretend otherwise.
            let moment = written_moment(conn, written_at)?;
            if state.finished_at.is_none() {
                touch_attempt_at(conn, attempt_id, &moment)?;
            }
            attempt_by_id(conn, attempt_id)
        })
    }

    /// Say that the words this cooking used belong in the diary and **not** in
    /// the recipe — or take that back (#58).
    ///
    /// It answers the offer, and nothing else: the As Cooked stays exactly
    /// where it is, because declining is a decision about the recipe and the
    /// cooking record is untouched by it. Kept because a question already
    /// answered, asked twice, is a nag — the same reason declining Meaning
    /// Search is remembered rather than re-offered.
    pub fn decline_promotion(
        &self,
        person_id: &str,
        attempt_id: &str,
        declined: bool,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            attempt_state_owned_by(conn, attempt_id, person_id)?;
            conn.execute(
                if declined {
                    "UPDATE attempts SET promotion_declined_at = \
                        strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?1"
                } else {
                    "UPDATE attempts SET promotion_declined_at = NULL WHERE id = ?1"
                },
                params![attempt_id],
            )
            .map_err(|e| OpError::internal(format!("cannot record the answer: {e}")))?;
            attempt_by_id(conn, attempt_id)
        })
    }

    /// **Promotion**: an As Cooked becoming an ordinary Version on a Branch
    /// (ADR 0005). The point at which an Attempt's freedoms end and the
    /// recipe's rules begin.
    ///
    /// It is mechanical, and it is mechanical because there is nothing to do.
    /// The As Cooked is already a complete recipe state, already stored in
    /// `versions`, already named by its own fingerprint — so promoting appends
    /// a `branch_versions` row naming a Version that has existed since the
    /// cook wrote it at the stove. Nothing is retyped and no identity is
    /// minted, exactly as ADR 0005 predicted when it chose to reuse the
    /// Version's shape rather than model a deviation twice.
    ///
    /// It goes through `save_recipe_version` for the same reason
    /// `promote_attempt_photograph` does: this is **an ordinary edit of the
    /// recipe**, so it inherits the whole of one — the collapse window,
    /// Readings carried forward and unread lines read, the language offer, and
    /// taking a **Copy** where the Branch was written under another Hand. A
    /// Branch held by a Kitchen the caller does not cook in is refused as
    /// absent (#100).
    ///
    /// **Promoting from an Attempt against an older Version is not a merge.**
    /// `save_recipe_version` appends onto wherever the Branch stands now, so
    /// cooking Tuesday's text and promoting on Friday writes Friday's recipe
    /// with your words in it — one Version, appended, the way any edit would
    /// be. Nothing is reconciled, because ADR 0004 refuses to combine two
    /// states and this is not an exception to that.
    ///
    /// **The Attempt is untouched.** It keeps its As Cooked and keeps pinning
    /// to the Version it cooked, so the diary goes on saying truthfully what
    /// happened that afternoon. Whether an As Cooked has already been promoted
    /// needs no flag: its `version_id` is either in the Branch's chain or it
    /// is not.
    pub fn promote_as_cooked(
        &self,
        caller: &Caller,
        attempt_id: &str,
        branch_id: &str,
        name: Option<&str>,
        change_note: Option<&str>,
    ) -> Result<Value, OpError> {
        let content = self.db().with_conn(|conn| {
            // Yours to promote from. An Attempt is a private record, and
            // reaching into somebody else's for the words they cooked is not a
            // promotion.
            attempt_state_owned_by(conn, attempt_id, &caller.person_id)?;

            let (lineage_id, as_cooked_version_id): (String, Option<String>) = conn
                .query_row(
                    "SELECT lineage_id, as_cooked_version_id FROM attempts WHERE id = ?1",
                    params![attempt_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(|e| OpError::internal(format!("cannot read Attempt: {e}")))?;

            let as_cooked_version_id = as_cooked_version_id.ok_or_else(|| {
                OpError::bad_request("this cooking has no As Cooked — it was cooked as written")
            })?;

            // The recipe promoted into must be the dish that was cooked, on a
            // Branch one of the caller's Kitchens holds, checked in that order
            // for the reason `promote_attempt_photograph` records next door.
            let (branch_lineage, cookbook_id): (String, String) = conn
                .query_row(
                    "SELECT lineage_id, cookbook_id FROM branches WHERE id = ?1",
                    params![branch_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?
                .ok_or_else(no_such_branch)?;
            ensure_sees_or_absent(conn, &cookbook_id, &caller.person_id, no_such_branch)?;
            if branch_lineage != lineage_id {
                return Err(OpError::bad_request(
                    "that Branch is not a Branch of the recipe this Attempt cooked",
                ));
            }

            let stored: String = conn
                .query_row(
                    "SELECT content FROM versions WHERE id = ?1",
                    params![as_cooked_version_id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot read As Cooked: {e}")))?;
            let content = serde_json::from_str::<Value>(&stored)
                .map_err(|e| OpError::internal(format!("cannot read As Cooked content: {e}")))?;

            // The name the head Version carries, if any. Carried through the
            // save below for the reason `promote_attempt_photograph` records
            // next door: a promotion inside the collapse window folds into that
            // very Version, and a save naming nothing writes its name away — so
            // promoting into a Version somebody had named would quietly un-name
            // it. A name given here overrides it.
            let head_name: Option<String> = conn
                .query_row(
                    "SELECT name FROM branch_versions \
                      WHERE branch_id = ?1 ORDER BY sequence DESC LIMIT 1",
                    params![branch_id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot read Version name: {e}")))?;

            Ok((content, head_name))
        })?;
        let (content, head_name) = content;

        // An ordinary save of an ordinary recipe. Deliberately not inside the
        // connection above: `save_recipe_version` takes the database itself.
        self.save_recipe_version(
            caller,
            branch_id,
            &content,
            name.or(head_name.as_deref()),
            change_note,
            None,
        )
    }

    /// Read the caller's own In Progress Attempt for a Lineage, if any —
    /// how two devices cooking the same dish stay in step (the last one to
    /// call `advance_attempt` is where the cook is), and whether resuming
    /// should still be offered. Scoped to the caller's own Attempts alone,
    /// so this needs no membership check of its own: whoever holds one
    /// already passed it when they started.
    pub fn get_current_attempt(&self, person_id: &str, lineage_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            Ok(json!({ "attempt": in_progress_attempt(conn, lineage_id, person_id)? }))
        })
    }

    /// **The cooking diary** (#60): every Attempt this Person has made, newest
    /// first, whatever recipe it was against.
    ///
    /// Sorted by date rather than by recipe, which is the whole of why this
    /// exists as a screen: *what did I cook that week* is a question the shelf
    /// cannot answer however it is filtered. Cookings nobody ever finished are
    /// in it beside the finished ones, because starting is what makes a
    /// cooking real (ADR 0010).
    ///
    /// Scoped to the caller's own Attempts and so needing no membership check
    /// of its own — whoever holds one already passed it when they started, the
    /// same reasoning `get_current_attempt` runs on. The recipe named beside
    /// each entry is a second question, and that one is asked once per
    /// *Lineage* rather than once per Attempt: cooking the katsu curry twenty
    /// times is twenty lines of one diary and one recipe to look up.
    ///
    /// Unbounded on purpose. A cap here would silently stop answering the
    /// question the screen exists for — "and before that?" — and this is a
    /// personal library: the real one is 86 recipes, and a lifetime of cooking
    /// them is a few thousand rows.
    pub fn list_attempts(&self, person_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let reading_language = reading_language_of(conn, person_id)?;
            let mut statement = conn
                .prepare(&format!(
                    "SELECT {ATTEMPT_COLUMNS} FROM attempts WHERE person_id = ?1 \
                     ORDER BY created_at DESC, id DESC"
                ))
                .map_err(|e| OpError::internal(format!("cannot read the diary: {e}")))?;
            let mut entries: Vec<Value> = statement
                .query_map(params![person_id], attempt_row)
                .map_err(|e| OpError::internal(format!("cannot read the diary: {e}")))?
                .collect::<Result<_, _>>()
                .map_err(|e| OpError::internal(format!("cannot read the diary: {e}")))?;

            // Keyed by Lineage *and* the Version cooked, because those are the
            // two things the answer turns on: a Lineage that has left the shelf
            // is titled from the Version, and two Attempts on one Lineage can
            // be pinned to different Versions.
            let mut looked_up: HashMap<(String, String), Value> = HashMap::new();
            for entry in &mut entries {
                let lineage_id = entry["lineage_id"]
                    .as_str()
                    .expect("lineage_id is always a string")
                    .to_string();
                let version_id = entry["version_id"]
                    .as_str()
                    .expect("version_id is always a string")
                    .to_string();
                let recipe = match looked_up.get(&(lineage_id.clone(), version_id.clone())) {
                    Some(known) => known.clone(),
                    None => {
                        let found = diary_recipe(
                            conn,
                            person_id,
                            &reading_language,
                            &lineage_id,
                            &version_id,
                        )?;
                        looked_up.insert((lineage_id, version_id), found.clone());
                        found
                    }
                };
                entry["recipe"] = recipe;
            }
            Ok(json!({ "attempts": entries }))
        })
    }
}

/// The Attempt state every write Operation on it needs before doing
/// anything else: what it was pinned to, and whether it has already
/// finished. Not the full read shape `attempt_by_id` answers — just enough
/// to decide whether the caller may act at all.
struct AttemptState {
    version_id: String,
    finished_at: Option<String>,
    /// When the cook last moved: the Step, the ticks or the Yield (#77).
    moved_at: Option<String>,
}

/// Look up an Attempt's state and prove the caller owns it — the one check
/// `advance_attempt`, `finish_attempt`, `edit_attempt` and `delete_attempt`
/// all open with, since an Attempt is a private diary until its cook says
/// otherwise (ADR 0005): no Kitchen membership substitutes for it.
fn attempt_state_owned_by(
    conn: &Connection,
    attempt_id: &str,
    person_id: &str,
) -> Result<AttemptState, OpError> {
    let (owner_id, version_id, finished_at, moved_at): (
        String,
        String,
        Option<String>,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT person_id, version_id, finished_at, moved_at FROM attempts WHERE id = ?1",
            params![attempt_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read Attempt: {e}")))?
        .ok_or_else(no_such_attempt)?;
    // An Attempt Kamosu found from an id the caller named, so somebody else's
    // answers exactly as an id naming nothing does (ADR 0040). An Attempt is a
    // private record, and whose it is is no part of the answer.
    if owner_id != person_id {
        return Err(no_such_attempt());
    }
    Ok(AttemptState {
        version_id,
        finished_at,
        moved_at,
    })
}

/// Read one Attempt back in the shape `attempt_schema` declares.
/// `resumable` is computed in SQL rather than in Rust: still In Progress
/// and within three days of `last_action_at` (ADR 0010) — the same clock
/// every other timestamp in Kamosu is stamped from, `strftime('now')`.
/// The columns `attempt_row` expects, in exactly this order — shared by every
/// query that reads an Attempt back in `attempt_schema`'s shape, so the one
/// query reading a single Attempt and the one listing a Lineage's worth of
/// them can never drift apart on what "resumable" means or how a Yield or
/// ticked Ingredients are stored.
///
/// The As Cooked arrives as a scalar subquery rather than a join, so that
/// every existing query reading Attempts — one by id, one Lineage's worth, the
/// whole diary — grows it without any of them changing their FROM clause.
pub(super) const ATTEMPT_COLUMNS: &str = "id, lineage_id, person_id, version_id, current_step_index, \
     ticked_ingredients, cooking_yield, note, rating, finished_at, \
     created_at, last_action_at, \
     CASE WHEN finished_at IS NULL \
               AND julianday('now') - julianday(last_action_at) <= 3.0 \
          THEN 1 ELSE 0 END AS resumable, \
     photographs, \
     promotion_declined_at, \
     as_cooked_version_id, \
     (SELECT content FROM versions WHERE versions.id = attempts.as_cooked_version_id), \
     (SELECT content FROM versions WHERE versions.id = attempts.version_id \
        AND attempts.as_cooked_version_id IS NOT NULL)";

/// One Attempt row, in `ATTEMPT_COLUMNS`' order, read into `attempt_schema`'s
/// shape. `resumable` is computed in SQL rather than in Rust: still In
/// Progress and within three days of `last_action_at` (ADR 0010).
pub(super) fn attempt_row(row: &rusqlite::Row) -> rusqlite::Result<Value> {
    let ticked_ingredients: String = row.get(5)?;
    let cooking_yield: Option<String> = row.get(6)?;
    let resumable: i64 = row.get(12)?;
    let photographs: String = row.get(13)?;
    // The As Cooked, served whole (#58, ADR 0005): a screen showing what this
    // cook actually did needs the words, and an As Cooked exists only on the
    // cookings that deviated, so nothing is paid for the common case. `null`
    // where the recipe was cooked as written, which is most of them.
    // The cook said these words belong in the diary and not in the recipe.
    // Served as a plain yes-or-no: when it was decided is Kamosu's business.
    let promotion_declined: Option<String> = row.get(14)?;
    let as_cooked_version_id: Option<String> = row.get(15)?;
    let as_cooked_content: Option<String> = row.get(16)?;
    // The Version this cooking was pinned to — fetched only where there is an
    // As Cooked to read against it, so a cooking that followed the recipe (most
    // of them) parses nothing extra.
    let cooked_from: Option<String> = row.get(17)?;
    let as_cooked = match (as_cooked_version_id, as_cooked_content) {
        (Some(version_id), Some(content)) => {
            let content = serde_json::from_str::<Value>(&content).unwrap_or(Value::Null);
            let against = cooked_from
                .and_then(|text| serde_json::from_str::<Value>(&text).ok())
                .map(|from| as_cooked_against(&from, &content))
                .unwrap_or(Value::Null);
            json!({
                "version_id": version_id,
                "content": content,
                "against": against,
                "promotion_declined": promotion_declined.is_some(),
            })
        }
        _ => Value::Null,
    };
    Ok(json!({
        "as_cooked": as_cooked,
        "id": row.get::<_, String>(0)?,
        "lineage_id": row.get::<_, String>(1)?,
        "person_id": row.get::<_, String>(2)?,
        "version_id": row.get::<_, String>(3)?,
        "current_step_index": row.get::<_, i64>(4)?,
        "ticked_ingredients": serde_json::from_str::<Value>(&ticked_ingredients)
            .unwrap_or(json!([])),
        "cooking_yield": cooking_yield
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .unwrap_or(Value::Null),
        "note": row.get::<_, Option<String>>(7)?,
        "rating": row.get::<_, Option<String>>(8)?,
        "finished_at": row.get::<_, Option<String>>(9)?,
        "created_at": row.get::<_, String>(10)?,
        "last_action_at": row.get::<_, String>(11)?,
        "resumable": resumable != 0,
        "photographs": serde_json::from_str::<Value>(&photographs).unwrap_or(json!([])),
    }))
}

/// An As Cooked laid over the Version it was cooked from, by the **same
/// Pairing** two Branches are laid over each other with (ADR 0019, which
/// predicted exactly this: "an As Cooked compares against its starting Version
/// by the same Pairing, so an Attempt's deviations need nothing of their own").
///
/// It is read here, in the Core, and never in a screen. Pairing lines is the
/// one piece of judgement in Kamosu that must give the same answer at both
/// Doors and to every client, and an index-based comparison in a frontend would
/// be wrong the moment a cook adds or drops a line — which they now can.
///
/// The Version cooked is both the base and `mine`, because there is only one
/// history here: the recipe as it stood, and what this cook did to it. So a row
/// reads `same` where the cook left the line alone, `changed` where they
/// rewrote it, `only-mine` where they dropped it, and `only-theirs` where they
/// added one.
fn as_cooked_against(cooked_from: &Value, as_cooked: &Value) -> Value {
    let rows = |field: &str| -> Vec<Value> {
        let base = crate::pairing::Line::list_from(cooked_from, field);
        let theirs = crate::pairing::Line::list_from(as_cooked, field);
        crate::pairing::read(&base, &base, &theirs)
            .iter()
            .map(crate::pairing::Row::to_json)
            .collect()
    };
    json!({ "ingredients": rows("ingredients"), "steps": rows("steps") })
}

/// The three things a rating can say (#59): the cook's decision about next
/// time, not a score. A score invites an average, and ADR 0015 refuses to
/// compute one — three words make that refusal obvious rather than a rule.
const RATINGS: [&str; 3] = ["again", "tweak", "no"];

/// Read a rating off an Operation's input. `Value::Null` clears it; any other
/// value must be one of [`RATINGS`].
fn parse_rating(value: &Value) -> Result<Option<String>, OpError> {
    match value {
        Value::Null => Ok(None),
        Value::String(text) if RATINGS.contains(&text.as_str()) => Ok(Some(text.clone())),
        _ => Err(OpError::bad_request(
            "rating must be one of \"again\", \"tweak\", \"no\", or null",
        )),
    }
}

/// Read the Photographs of one Attempt off an Operation's input: a list of
/// Photograph ids already uploaded, sent whole like the ticked Ingredients
/// beside it rather than added one at a time.
///
/// Each id must name a Photograph this instance actually holds. This is
/// stricter than the loose pointer a Version's `main_photo` is (#45) on
/// purpose: a Version's photo reference may arrive in a Bundle ahead of its
/// bytes, whereas an Attempt is only ever written by somebody who just
/// uploaded the picture through this same Door, so a name that resolves to
/// nothing here is a mistake rather than a picture still in the post.
fn parse_attempt_photographs(conn: &Connection, value: &Value) -> Result<Vec<String>, OpError> {
    let listed = value
        .as_array()
        .ok_or_else(|| OpError::bad_request("photographs must be an array of Photograph ids"))?;
    let mut photographs: Vec<String> = Vec::with_capacity(listed.len());
    for entry in listed {
        let id = entry
            .as_str()
            .ok_or_else(|| OpError::bad_request("each photograph must be a Photograph id"))?;
        let known: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM photographs WHERE hash = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|e| OpError::internal(format!("cannot read Photograph: {e}")))?;
        if known == 0 {
            return Err(OpError::bad_request("no such Photograph"));
        }
        // The same picture twice is one picture (ADR 0017), so attaching it
        // twice to one cooking is one attachment.
        if !photographs.iter().any(|held| held == id) {
            photographs.push(id.to_string());
        }
    }
    Ok(photographs)
}

/// What finishing or correcting a cooking may say: each field left out stays
/// as it stood, `Some(Null)` clears it, and anything else sets it.
#[derive(Default, Clone, Copy)]
pub struct Judgement<'a> {
    pub note: Option<&'a Value>,
    pub rating: Option<&'a Value>,
    /// The whole list, replacing what was there.
    pub photographs: Option<&'a Value>,
    /// Pictures to put beside what is already there (#77). A cooking
    /// photographed on the phone and on the iPad, one of them offline, keeps
    /// every picture: sending the whole list from each would leave only
    /// whichever list arrived last.
    pub add_photographs: Option<&'a Value>,
}

impl Judgement<'_> {
    fn is_empty(&self) -> bool {
        self.note.is_none()
            && self.rating.is_none()
            && self.photographs.is_none()
            && self.add_photographs.is_none()
    }
}

/// Write what a cook's judgement is made of — a note, a rating and
/// Photographs, whole or added to — onto one Attempt. `None` leaves a field
/// as it stood.
///
/// Shared by `finish_attempt` and `edit_attempt` rather than written twice:
/// filling these in at the stove and correcting them a week later are the
/// same act on the same fields, and two copies of this would eventually
/// disagree about what a rating may be.
fn write_attempt_judgement(
    conn: &Connection,
    attempt_id: &str,
    judgement: &Judgement<'_>,
) -> Result<(), OpError> {
    let Judgement {
        note,
        rating,
        photographs,
        add_photographs,
    } = *judgement;
    if let Some(value) = note {
        let stored = match value {
            Value::Null => None,
            Value::String(text) => Some(required_text(text, "note")?.to_string()),
            _ => return Err(OpError::bad_request("note must be a string or null")),
        };
        conn.execute(
            "UPDATE attempts SET note = ?2 WHERE id = ?1",
            params![attempt_id, stored],
        )
        .map_err(|e| OpError::internal(format!("cannot save note: {e}")))?;
    }
    if let Some(value) = rating {
        let stored = parse_rating(value)?;
        conn.execute(
            "UPDATE attempts SET rating = ?2 WHERE id = ?1",
            params![attempt_id, stored],
        )
        .map_err(|e| OpError::internal(format!("cannot save rating: {e}")))?;
    }
    if let Some(value) = photographs {
        let stored = match value {
            Value::Null => Vec::new(),
            other => parse_attempt_photographs(conn, other)?,
        };
        let stored = serde_json::to_string(&stored).expect("serialisable Photograph ids");
        conn.execute(
            "UPDATE attempts SET photographs = ?2 WHERE id = ?1",
            params![attempt_id, stored],
        )
        .map_err(|e| OpError::internal(format!("cannot save Photographs: {e}")))?;
    }
    if let Some(value) = add_photographs {
        let adding = match value {
            Value::Null => Vec::new(),
            other => parse_attempt_photographs(conn, other)?,
        };
        let mut held = attempt_photographs(conn, attempt_id)?;
        for id in adding {
            if !held.contains(&id) {
                held.push(id);
            }
        }
        let stored = serde_json::to_string(&held).expect("serialisable Photograph ids");
        conn.execute(
            "UPDATE attempts SET photographs = ?2 WHERE id = ?1",
            params![attempt_id, stored],
        )
        .map_err(|e| OpError::internal(format!("cannot save Photographs: {e}")))?;
    }
    Ok(())
}

/// The Photograph ids one Attempt carries.
fn attempt_photographs(conn: &Connection, attempt_id: &str) -> Result<Vec<String>, OpError> {
    let stored: String = conn
        .query_row(
            "SELECT photographs FROM attempts WHERE id = ?1",
            params![attempt_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read Attempt: {e}")))?
        .ok_or_else(no_such_attempt)?;
    Ok(serde_json::from_str(&stored).unwrap_or_default())
}

fn attempt_by_id(conn: &Connection, id: &str) -> Result<Value, OpError> {
    conn.query_row(
        &format!("SELECT {ATTEMPT_COLUMNS} FROM attempts WHERE id = ?1"),
        params![id],
        attempt_row,
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read Attempt: {e}")))?
    .ok_or_else(no_such_attempt)
}

/// The recipe one diary entry was cooked from: which Branch it opens, and
/// what to call it (#60).
///
/// Two answers, and which one is given turns on a single question — is this
/// Lineage still on the caller's shelf?
///
/// - **It is.** The entry opens the Branch the shelf's own card would open —
///   the one written in the reader's Language, else the oldest — and is
///   titled as that Branch is titled *now*. A recipe you renamed on Tuesday
///   is the recipe you renamed on Tuesday, on every screen that shows it.
/// - **It is not** — the caller left the Kitchen holding it, and leaving is
///   not a deletion (#32's story 20). Then there is no Branch to open, and the
///   title comes off the Version actually cooked: the name it was known by.
///   This is the rule #52 already settled for a Related Recipe whose other end
///   has left the shelf — text is better than a pointer that opens nothing.
fn diary_recipe(
    conn: &Connection,
    person_id: &str,
    reading_language: &str,
    lineage_id: &str,
    version_id: &str,
) -> Result<Value, OpError> {
    let on_the_shelf: Option<(String, Option<String>)> = conn
        .query_row(
            &format!(
                "SELECT branches.id, json_extract(versions.content, '$.title') \
                   FROM branches \
                   JOIN versions ON versions.id = branches.head_version_id \
                  WHERE {} AND branches.lineage_id = ?2 \
                  ORDER BY (branches.language = ?3) DESC, {} \
                  LIMIT 1",
                visible_to("branches", "?1"),
                own_first("branches", "?1"),
            ),
            params![person_id, lineage_id, reading_language],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read the cooked Recipe: {e}")))?;

    // What the Version COOKED says it makes, off that Version rather than the
    // shelf's head, so a recipe edited since still says what it said then
    // beside how much was cooked (#109).
    let written_yield: Option<String> = conn
        .query_row(
            "SELECT json_extract(content, '$.yield') FROM versions WHERE id = ?1",
            params![version_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read the Version cooked: {e}")))?
        .flatten();
    let written_yield = written_yield
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .unwrap_or(Value::Null);

    // `title` is an `Option` only because `json_extract` is: every Version
    // carries a non-empty title by construction (`parse_recipe_content`
    // refuses one without), so the empty string below is the shape of a
    // hand-edited database rather than anything a Door can produce.
    let (branch_id, title) = match on_the_shelf {
        Some((branch_id, title)) => (Some(branch_id), title),
        None => {
            let remembered: Option<String> = conn
                .query_row(
                    "SELECT json_extract(content, '$.title') FROM versions WHERE id = ?1",
                    params![version_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read the Version cooked: {e}")))?
                .flatten();
            (None, remembered)
        }
    };
    Ok(json!({
        "branch_id": branch_id,
        "title": title.unwrap_or_default(),
        "written_yield": written_yield,
    }))
}

/// The one Attempt a Person may have In Progress on a Lineage at a time
/// (ADR 0010), if any.
pub(super) fn in_progress_attempt(
    conn: &Connection,
    lineage_id: &str,
    person_id: &str,
) -> Result<Option<Value>, OpError> {
    let id: Option<String> = conn
        .query_row(
            "SELECT id FROM attempts WHERE lineage_id = ?1 AND person_id = ?2 \
             AND finished_at IS NULL",
            params![lineage_id, person_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read Attempt: {e}")))?;
    id.map(|id| attempt_by_id(conn, &id)).transpose()
}

/// Every Attempt on a Lineage whose pinned Version is one the Thread is
/// actually showing — the same shape `attempt_by_id` reads a single one in.
/// An Attempt pinned to a Version on a Branch this caller cannot see (a
/// Kitchen they do not belong to) is left out rather than leaked just
/// because it shares a Lineage id.
/// The Version whose steps this cooking is walking through: the As Cooked
/// where the cook has written one, and otherwise the Version they started
/// from. What `current_step_index` indexes (#58).
fn cooking_version_id(conn: &Connection, attempt_id: &str) -> Result<Option<String>, OpError> {
    conn.query_row(
        "SELECT as_cooked_version_id FROM attempts WHERE id = ?1",
        params![attempt_id],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot read Attempt: {e}")))
}

/// Whether an Attempt id is one a phone may mint: the very shape the server
/// mints its own in, so the two are indistinguishable once stored.
fn is_minted_attempt_id(id: &str) -> bool {
    id.strip_prefix("at_").is_some_and(is_sixteen_hex)
}

/// Record that the cook did something — starting, resuming, advancing,
/// writing — at `moment`, so the three-day resume window (ADR 0010) counts
/// from it. Since #77 that moment may be hours before the action arrives, and
/// an action arriving late never winds the clock back past one that arrived
/// before it.
fn touch_attempt_at(conn: &Connection, id: &str, moment: &str) -> Result<(), OpError> {
    conn.execute(
        "UPDATE attempts SET last_action_at = max(last_action_at, ?2) WHERE id = ?1",
        params![id, moment],
    )
    .map_err(|e| OpError::internal(format!("cannot update Attempt: {e}")))?;
    Ok(())
}

/// How many entries a Version's `"steps"` or `"ingredients"` list holds —
/// the bound `advance_attempt` checks a Step index or a ticked Ingredient
/// index against, read from the pinned Version rather than the recipe's
/// current head, since an Attempt never moves off the Version it started on.
fn version_field_len(conn: &Connection, version_id: &str, field: &str) -> Result<i64, OpError> {
    let content: String = conn
        .query_row(
            "SELECT content FROM versions WHERE id = ?1",
            params![version_id],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot read Version: {e}")))?;
    let content: Value = serde_json::from_str(&content)
        .map_err(|e| OpError::internal(format!("cannot read Version content: {e}")))?;
    Ok(content[field].as_array().map(Vec::len).unwrap_or(0) as i64)
}

/// An Attempt id that names nothing here — and, by ADR 0040, an Attempt that
/// belongs to somebody else.
fn no_such_attempt() -> OpError {
    OpError::not_found("no such Attempt")
}
