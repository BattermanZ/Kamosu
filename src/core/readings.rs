//! Ingredient Lines, Readings and Foods: reading a line, correcting a Reading,
//! measuring one, and merging Foods (ADR 0021, ADR 0022).

use super::*;

/// **A whole Reading, as somebody sends one in.** The four fields travel
/// together because they describe one understanding of one line — the same
/// whole-state convention `save_recipe_version` uses for a whole recipe — so
/// correcting the unit means sending the amount and the target with it, and
/// all four absent clears the Reading entirely (ADR 0021).
///
/// `target` and `lineage_id` are the one slot in two spellings: a Reading's
/// target is either a Food's written word or the Lineage of a Recipe, never
/// both (ADR 0008).
pub struct Reading<'a> {
    pub amount: Option<&'a str>,
    pub unit: Option<&'a str>,
    pub target: Option<&'a str>,
    pub lineage_id: Option<&'a str>,
}

impl Core {
    /// **Read the Ingredient Lines of every recipe on this instance** that
    /// nothing has read yet, as a Job (#71).
    ///
    /// A save reads the lines it wrote, so an instance that has only ever been
    /// written to is already read and this finds nothing. What it is for is the
    /// library that existed before Kamosu could read a line at all, and the
    /// day this module gets better at reading them: a Reading is derived from
    /// the written line and can always be worked out again (ADR 0003), which
    /// is what makes running this safe to repeat.
    ///
    /// It reads the head of every Branch and no earlier Version. A Reading
    /// belongs to the Version it was laid over, and re-reading a history
    /// nobody is looking at would be work with no reader.
    ///
    /// **A line that already carries a Reading is left exactly as it is**,
    /// whether Kamosu wrote it or a person corrected it (ADR 0003).
    ///
    /// **What this does not defend, said plainly** (ADR 0034's habit): a
    /// Reading somebody *cleared* is indistinguishable from a line nothing has
    /// read, because clearing one deletes its row and leaves no headstone. So
    /// this fills it back in. The automatic path never does — a save reads only
    /// the lines it wrote — and this is the Operator asking, in as many words,
    /// for the unread lines to be read. Recording a deliberate emptiness would
    /// mean a third state for every line in the library, to serve the cook who
    /// cleared a Reading and then asked for a re-read; the honest trade is to
    /// say so here rather than to build it.
    pub fn read_ingredient_lines(
        &self,
        progress: Option<&crate::jobs::JobProgress>,
    ) -> Result<Value, OpError> {
        // **The database lock is never held across the whole walk** — the
        // same rule Meaning Search's build follows, and for a sharper reason
        // here: reporting progress takes the very lock this holds, so holding
        // it across the loop would not merely block the instance, it would
        // deadlock against itself on the first report.
        let heads = self.db().with_conn(|conn| {
            let mut statement = conn
                .prepare(
                    "SELECT branches.head_version_id, branches.language, versions.content \
                       FROM branches JOIN versions ON versions.id = branches.head_version_id",
                )
                .map_err(|e| OpError::internal(format!("cannot read the library: {e}")))?;
            let heads = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(|e| OpError::internal(format!("cannot read the library: {e}")))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| OpError::internal(format!("cannot read the library: {e}")))?;
            Ok(heads)
        })?;

        let total = heads.len();
        let mut read = 0u64;
        for (done, (version_id, language, content)) in heads.into_iter().enumerate() {
            if let Some(progress) = progress {
                progress.report(
                    done as u64,
                    Some(total as u64),
                    format!("reading the lines of {done} of {total} recipes"),
                );
            }
            // One recipe whose content will not parse is one recipe left
            // unread, never a failed Job (#71).
            let Ok(content) = serde_json::from_str::<Value>(&content) else {
                continue;
            };
            read += self.db().with_conn(|conn| {
                Ok(read_unread_lines(
                    conn,
                    &version_id,
                    &language,
                    &content,
                    None,
                ))
            })?;
        }
        Ok(json!({ "read": read }))
    }

    /// Correct a Reading on the Branch's current head Version: Kamosu's
    /// interpretation of one Ingredient Line, addressed by its position in
    /// that line's list. This never mints a Version and appears in no
    /// Thread (ADR 0021) — the row beside the head Version is simply
    /// replaced or removed. `amount`, `unit` and `target` describe the
    /// whole new Reading together, the same whole-state convention
    /// `save_recipe_version` uses for the whole recipe — this is not a
    /// per-field patch, so correcting one field means sending all three
    /// wanted. All three absent or blank clears the Reading entirely,
    /// taking the line back to fully unread.
    ///
    /// A non-blank `target` is matched against the instance's Foods in the
    /// Branch's own Language, creating one where nothing answers to the word
    /// yet (#47, ADR 0022). Which Food it resolved to is internal
    /// bookkeeping alone — the Reading's own shape stays the bare word,
    /// exactly as `set_reading` has always answered.
    ///
    /// **`lineage_id` is the other kind of target**: the Lineage of a
    /// Recipe, which makes this Ingredient a Component (ADR 0008). It is
    /// exclusive with `target` — a Reading points at a Food or at a Recipe,
    /// never at both — and it names a Lineage rather than a Version, so it goes
    /// on resolving to whatever Branch of the dough its reader holds.
    pub fn set_reading(
        &self,
        person_id: &str,
        branch_id: &str,
        line_index: i64,
        sent: Reading<'_>,
    ) -> Result<Value, OpError> {
        let Reading {
            amount,
            unit,
            target,
            lineage_id,
        } = sent;
        if line_index < 0 {
            return Err(OpError::bad_request("line_index must be zero or more"));
        }
        self.db().with_conn(|conn| {
            // The Branch's OWN Lineage, which is what the cooking scale is read
            // against — not the one this Reading may point at.
            let (cookbook_id, language, head_version_id, content, of_this_branch): (String, String, String, String, String) = conn
                .query_row(
                    "SELECT branches.cookbook_id, branches.language, branches.head_version_id, versions.content, branches.lineage_id \
                       FROM branches JOIN versions ON versions.id = branches.head_version_id \
                      WHERE branches.id = ?1",
                    params![branch_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?
                .ok_or_else(no_such_branch)?;
            ensure_sees_or_absent(conn, &cookbook_id, person_id, no_such_branch)?;

            let content: Value = serde_json::from_str(&content)
                .map_err(|e| OpError::internal(format!("cannot read Version content: {e}")))?;
            let line = content["ingredients"]
                .get(line_index as usize)
                .ok_or_else(|| OpError::bad_request("no Ingredient Line at that index"))?;
            if line["kind"] != "ingredient" {
                return Err(OpError::bad_request(
                    "a Reading belongs to an Ingredient Line, not a section",
                ));
            }

            let amount = amount.map(str::trim).filter(|v| !v.is_empty());
            let unit = unit.map(str::trim).filter(|v| !v.is_empty());
            let target = target.map(str::trim).filter(|v| !v.is_empty());
            let lineage_id = lineage_id.map(str::trim).filter(|v| !v.is_empty());

            // **A Reading's target is either a Food or a Lineage** (ADR 0008).
            // Refused rather than silently preferred one: a line claiming to be
            // both a flour and a dough is a caller's mistake, and quietly
            // dropping half of what they sent would hide it.
            if target.is_some() && lineage_id.is_some() {
                return Err(OpError::bad_request(
                    "a Reading points at a Food or at a Recipe, never both: send `target` or `lineage_id`",
                ));
            }

            if amount.is_none() && unit.is_none() && target.is_none() && lineage_id.is_none() {
                conn.execute(
                    "DELETE FROM readings WHERE version_id = ?1 AND line_index = ?2",
                    params![head_version_id, line_index],
                )
                .map_err(|e| OpError::internal(format!("cannot clear Reading: {e}")))?;
                // A cleared Reading has nothing left to measure, so the line
                // beneath goes with it.
                return Ok(json!({
                    "line_index": line_index,
                    "reading": Value::Null,
                    "measured": Value::Null,
                }));
            }

            let food_id = target
                .map(|word| {
                    resolve_food_for_word(
                        conn,
                        &language,
                        word,
                        Some((head_version_id.as_str(), line_index)),
                    )
                })
                .transpose()?;

            // **A Lineage this instance does not hold is accepted.** ADR 0008
            // is explicit that a Component must survive its recipe being
            // deleted, arriving without its passenger, or never being received
            // — so there is nothing to check here, and checking would refuse
            // the very pointer the decision exists to keep.
            conn.execute(
                "INSERT INTO readings (version_id, line_index, amount, unit, target, food_id, lineage_id) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
                 ON CONFLICT (version_id, line_index) DO UPDATE SET \
                    amount = excluded.amount, unit = excluded.unit, target = excluded.target, \
                    food_id = excluded.food_id, lineage_id = excluded.lineage_id, \
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')",
                params![head_version_id, line_index, amount, unit, target, food_id, lineage_id],
            )
            .map_err(|e| OpError::internal(format!("cannot save Reading: {e}")))?;

            // The corrected Reading's own subordinate line, worked out here
            // rather than left for the next fetch — the moment somebody tells
            // Kamosu it misread a line is the moment they want to see what the
            // corrected line now says (#49).
            let measured = measured_for_line(
                conn,
                &head_version_id,
                line_index,
                &Reader::of(conn, person_id)?,
                cooking_scale(conn, &of_this_branch, person_id, &content)?,
            )?;

            Ok(json!({
                "line_index": line_index,
                "reading": {
                    "amount": amount,
                    "unit": unit,
                    "target": target,
                    "lineage_id": lineage_id,
                },
                "measured": measured,
            }))
        })
    }

    /// Every Food this instance knows, each shown in the reader's Reading
    /// Language and falling back to whatever name it does have (#47).
    pub fn list_foods(&self, person_id: &str) -> Result<Vec<Value>, OpError> {
        self.db().with_conn(|conn| {
            let ids: Vec<String> = {
                let mut statement = conn
                    .prepare("SELECT id FROM foods ORDER BY created_at, id")
                    .map_err(|e| OpError::internal(format!("cannot list Foods: {e}")))?;
                statement
                    .query_map([], |row| row.get(0))
                    .map_err(|e| OpError::internal(format!("cannot list Foods: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot list Foods: {e}")))?
            };
            ids.iter()
                .map(|id| food_summary(conn, id, person_id))
                .collect()
        })
    }

    /// Read one Food back: its names, its Cup Weight and how many Readings
    /// currently point at it.
    pub fn get_food(&self, person_id: &str, food_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            ensure_food_exists(conn, food_id)?;
            food_summary(conn, food_id, person_id)
        })
    }

    /// Give a Food its name in one Language, or correct the one it has
    /// there. Any Person may (CONTEXT.md) — a Food is instance-wide, not a
    /// Kitchen's to guard. Where that word already answers for a different
    /// Food, this still attaches it here: typing a name onto a Food another
    /// already answers to is the one deliberate way to make a duplicate name
    /// (ADR 0022) rather than an error.
    pub fn set_food_name(
        &self,
        person_id: &str,
        food_id: &str,
        language: &str,
        name: &str,
    ) -> Result<Value, OpError> {
        let language = supported_language(language)?;
        let name = required_text(name, "name")?.to_string();
        self.db().with_conn(|conn| {
            ensure_food_exists(conn, food_id)?;
            conn.execute(
                "INSERT INTO food_names (food_id, language, name, name_folded) \
                 VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT(food_id, language) \
                 DO UPDATE SET name = excluded.name, name_folded = excluded.name_folded",
                params![food_id, language, name, folded_word(&name)],
            )
            .map_err(|e| OpError::internal(format!("cannot name Food: {e}")))?;

            // Typing a name onto a Food another already answers to is the one
            // remaining way to make a duplicate name, and it leaves the same
            // trail as an arriving collision does (ADR 0022). The word is not
            // refused: a Food is anybody's to name, and the duplicate is
            // evidence for the Operator rather than an error for the typist.
            for other in &foods_already_answering_to(conn, language, &name, food_id)? {
                record_merge_suggestion(
                    conn,
                    food_id,
                    other,
                    "name_typed_onto_another",
                    &[(language, name.as_str())],
                )?;
            }

            food_summary(conn, food_id, person_id)
        })
    }

    /// Take a Food's name in one Language back off. A Food is known by its
    /// words alone, so its last remaining name may not be removed this way —
    /// deleting the Food nothing points at is the Operator's own power (#48).
    pub fn remove_food_name(
        &self,
        person_id: &str,
        food_id: &str,
        language: &str,
    ) -> Result<Value, OpError> {
        let language = supported_language(language)?;
        self.db().with_conn(|conn| {
            ensure_food_exists(conn, food_id)?;
            let name_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM food_names WHERE food_id = ?1",
                    params![food_id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot count Food names: {e}")))?;
            if name_count <= 1 {
                return Err(OpError::bad_request("a Food must keep at least one name"));
            }
            conn.execute(
                "DELETE FROM food_names WHERE food_id = ?1 AND language = ?2",
                params![food_id, language],
            )
            .map_err(|e| OpError::internal(format!("cannot remove Food name: {e}")))?;
            food_summary(conn, food_id, person_id)
        })
    }

    /// Set or clear a Food's Cup Weight — the one figure that turns a volume
    /// of it into a weight. Kamosu ships none by default; anyone may correct
    /// or add one (CONTEXT.md). `None` clears it back to "offers millilitres
    /// instead of grams".
    pub fn set_food_cup_weight(
        &self,
        person_id: &str,
        food_id: &str,
        cup_weight_grams: Option<f64>,
    ) -> Result<Value, OpError> {
        if cup_weight_grams.is_some_and(|grams| grams <= 0.0) {
            return Err(OpError::bad_request(
                "cup_weight_grams must be greater than zero",
            ));
        }
        self.db().with_conn(|conn| {
            ensure_food_exists(conn, food_id)?;
            conn.execute(
                "UPDATE foods SET cup_weight_grams = ?2 WHERE id = ?1",
                params![food_id, cup_weight_grams],
            )
            .map_err(|e| OpError::internal(format!("cannot set Cup Weight: {e}")))?;
            food_summary(conn, food_id, person_id)
        })
    }

    /// Every Merge Suggestion this instance holds, newest first: two Foods
    /// something said were probably one thing, and what said it.
    ///
    /// A worklist rather than a hunt (ADR 0022). Kamosu never scans the Food
    /// list looking for words that resemble each other — every row here was
    /// put there by a specific event that is recorded alongside it, and
    /// nothing acts on one automatically.
    pub fn list_merge_suggestions(&self, person_id: &str) -> Result<Vec<Value>, OpError> {
        self.db().with_conn(|conn| {
            let rows: Vec<(String, String, String, String, String)> = {
                let mut statement = conn
                    .prepare(
                        "SELECT food_a_id, food_b_id, reason, words, created_at \
                         FROM merge_suggestions ORDER BY created_at DESC, food_a_id, food_b_id",
                    )
                    .map_err(|e| {
                        OpError::internal(format!("cannot list Merge Suggestions: {e}"))
                    })?;
                statement
                    .query_map([], |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                        ))
                    })
                    .map_err(|e| OpError::internal(format!("cannot list Merge Suggestions: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot list Merge Suggestions: {e}")))?
            };
            rows.into_iter()
                .map(|(food_a_id, food_b_id, reason, words, created_at)| {
                    Ok(json!({
                        "foods": [
                            food_summary(conn, &food_a_id, person_id)?,
                            food_summary(conn, &food_b_id, person_id)?,
                        ],
                        "reason": reason,
                        "words": serde_json::from_str::<Value>(&words).map_err(|e| {
                            OpError::internal(format!("cannot read a Suggestion's words: {e}"))
                        })?,
                        "created_at": created_at,
                    }))
                })
                .collect()
        })
    }

    /// What a Merge is about to do, in plain numbers, without doing any of it.
    ///
    /// This is the whole safety net: a Merge cannot be undone in v1 (ADR 0022),
    /// so the blast radius is stated first, by an Operation that writes
    /// nothing — and `merge_food` then *refuses* to act until the Operator
    /// says the `ingredient_lines` figure back. The saying is therefore
    /// binding rather than advisory: a Merge cannot be reached without having
    /// been told what it moves, and a figure that has gone stale between the
    /// looking and the doing stops the Merge instead of surprising it.
    pub fn preview_food_merge(
        &self,
        person_id: &str,
        survivor_food_id: &str,
        absorbed_food_id: &str,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let (survivor, absorbed) =
                ensure_mergeable(conn, survivor_food_id, absorbed_food_id, person_id)?;
            let moving = merge_blast_radius(conn, absorbed_food_id)?;
            Ok(json!({
                "survivor": survivor,
                "absorbed": absorbed,
                "ingredient_lines": moving.ingredient_lines,
                "readings": moving.readings,
                "cup_weight_conflict": cup_weight_conflict(conn, survivor_food_id, absorbed_food_id)?
                    .is_some(),
            }))
        })
    }

    /// Join two Foods into one — the Operator's, and no one else's (ADR 0022).
    ///
    /// The survivor takes every name the absorbed Food had in a Language the
    /// survivor has none for, every Reading that pointed at the absorbed Food
    /// points at the survivor instead, and every Merge Suggestion naming
    /// either is cleared: the question they asked has now been answered.
    ///
    /// `ingredient_lines` is the figure `preview_food_merge` announced, said
    /// back. It must match what the Merge is about to move or the Merge is
    /// refused — which is what makes the saying the safety net CONTEXT.md
    /// calls it rather than a number nobody had to read.
    ///
    /// Where the two disagree about Cup Weight, `cup_weight_grams` says which
    /// **of the two** figures survives; omitting it on a disagreement is
    /// refused rather than guessed at, and a third figure is refused too — a
    /// Merge settles a disagreement, it does not set a Cup Weight. Where only
    /// one of them knows one, that figure is the answer and nothing needs
    /// saying.
    ///
    /// **There is no un-merge in v1** and nothing here pretends otherwise.
    pub fn merge_food(
        &self,
        person_id: &str,
        survivor_food_id: &str,
        absorbed_food_id: &str,
        ingredient_lines: i64,
        cup_weight_grams: Option<f64>,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            ensure_mergeable(conn, survivor_food_id, absorbed_food_id, person_id)?;

            // Counted before anything moves, so what is reported is what the
            // preview would have said rather than what is left afterwards.
            let moving = merge_blast_radius(conn, absorbed_food_id)?;
            if ingredient_lines != moving.ingredient_lines {
                return Err(OpError::bad_request(format!(
                    "this Merge moves {} Ingredient Lines, not {ingredient_lines}: \
                     read preview_food_merge again before merging",
                    moving.ingredient_lines
                )));
            }

            let surviving_cup_weight = match (
                cup_weight_grams,
                cup_weight_conflict(conn, survivor_food_id, absorbed_food_id)?,
            ) {
                (Some(chosen), Some((survivor, absorbed))) => {
                    if chosen != survivor && chosen != absorbed {
                        return Err(OpError::bad_request(format!(
                            "cup_weight_grams must be one of the two figures in \
                             dispute, {survivor} or {absorbed}"
                        )));
                    }
                    Some(chosen)
                }
                (Some(_), None) => {
                    return Err(OpError::bad_request(
                        "these two Foods do not disagree about Cup Weight: \
                         a Merge settles a disagreement, it does not set one",
                    ));
                }
                (None, Some(_)) => {
                    return Err(OpError::bad_request(
                        "these two Foods disagree about Cup Weight: say which figure survives",
                    ));
                }
                (None, None) => {
                    let survivor: Option<f64> = cup_weight_of(conn, survivor_food_id)?;
                    survivor.or(cup_weight_of(conn, absorbed_food_id)?)
                }
            };

            let carried: Vec<(String, String)> = {
                let mut statement = conn
                    .prepare("SELECT language, name FROM food_names WHERE food_id = ?1")
                    .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?;
                statement
                    .query_map(params![absorbed_food_id], |row| {
                        Ok((row.get(0)?, row.get(1)?))
                    })
                    .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?
            };

            conn.execute(
                "UPDATE readings SET food_id = ?1 WHERE food_id = ?2",
                params![survivor_food_id, absorbed_food_id],
            )
            .map_err(|e| OpError::internal(format!("cannot move Readings: {e}")))?;

            conn.execute(
                "UPDATE foods SET cup_weight_grams = ?2 WHERE id = ?1",
                params![survivor_food_id, surviving_cup_weight],
            )
            .map_err(|e| OpError::internal(format!("cannot set the surviving Cup Weight: {e}")))?;

            // The absorbed Food's own rows go first: one name per Food per
            // Language is a primary key, so the survivor may only adopt a
            // word once its old owner has let go of it. Erasing it clears the
            // Suggestions that named it in the same breath.
            erase_food(conn, absorbed_food_id)?;
            // The survivor's own suggestions are answered too: whatever they
            // asked, the Operator has now said what these Foods are.
            conn.execute(
                "DELETE FROM merge_suggestions WHERE food_a_id = ?1 OR food_b_id = ?1",
                params![survivor_food_id],
            )
            .map_err(|e| OpError::internal(format!("cannot clear Merge Suggestions: {e}")))?;

            for (language, name) in carried {
                conn.execute(
                    "INSERT OR IGNORE INTO food_names \
                     (food_id, language, name, name_folded) VALUES (?1, ?2, ?3, ?4)",
                    params![survivor_food_id, language, name, folded_word(&name)],
                )
                .map_err(|e| OpError::internal(format!("cannot adopt a Food's name: {e}")))?;

                // An adopted word may be one some *third* Food already answers
                // to, and a duplicate name leaves the same trail however it was
                // made (ADR 0022). Recorded after the survivor's own
                // suggestions are cleared, so answering this merge's question
                // does not swallow the one this adoption just raised.
                for other in foods_already_answering_to(conn, &language, &name, survivor_food_id)? {
                    record_merge_suggestion(
                        conn,
                        survivor_food_id,
                        &other,
                        "name_typed_onto_another",
                        &[(language.as_str(), name.as_str())],
                    )?;
                }
            }

            Ok(json!({
                "food": food_summary(conn, survivor_food_id, person_id)?,
                "ingredient_lines": moving.ingredient_lines,
                "readings": moving.readings,
            }))
        })
    }

    /// Delete a Food nothing points at — the Operator's, from the same screen
    /// merging happens on (ADR 0022).
    ///
    /// A Food something still points at is refused rather than cascaded:
    /// deleting a recipe must never quietly discard the fact that a cup of
    /// this flour is 125 g. A Food nothing points at is *kept* by Kamosu on
    /// its own — this Operation is the deliberate act, never a sweep.
    ///
    /// "Points at" means a Reading some Branch still holds
    /// ([`REACHABLE_READING`]). Readings a deleted recipe or a collapsed save
    /// left behind are deleted here with the Food, since nobody can see them
    /// and the foreign key would refuse the Food's row while they stood.
    pub fn delete_food(&self, food_id: &str) -> Result<(), OpError> {
        self.db().with_conn(|conn| {
            ensure_food_exists(conn, food_id)?;
            let reading_count = reachable_reading_count(conn, food_id)?;
            if reading_count > 0 {
                let readings = if reading_count == 1 {
                    "1 Reading still points".to_string()
                } else {
                    format!("{reading_count} Readings still point")
                };
                return Err(OpError::bad_request(format!(
                    "{readings} at this Food: only one nothing points at may be deleted"
                )));
            }
            // One transaction, so a Food is never left standing with the
            // Readings that named it already gone.
            let transaction = conn
                .unchecked_transaction()
                .map_err(|e| OpError::internal(format!("cannot begin: {e}")))?;
            conn.execute("DELETE FROM readings WHERE food_id = ?1", params![food_id])
                .map_err(|e| {
                    OpError::internal(format!("cannot clear a Food's unreachable Readings: {e}"))
                })?;
            erase_food(conn, food_id)?;
            transaction
                .commit()
                .map_err(|e| OpError::internal(format!("cannot delete Food: {e}")))?;
            Ok(())
        })
    }
}

/// How much a Merge is about to move, in the two numbers that differ.
///
/// They differ because a Reading is carried forward onto each new Version of
/// its recipe (`carry_forward_readings`), so one flour line in a recipe edited
/// five times is **one** Ingredient Line and **five** Reading rows. Reporting
/// only the row count would tell an Operator a merge touches five lines when
/// one line moves on screen — an inflated safety net is a broken one.
struct MergeBlastRadius {
    /// The Ingredient Lines a cook can actually see move: Readings lying on a
    /// Branch's head Version, which is the recipe as it stands today.
    ingredient_lines: i64,
    /// Every Reading row that changes hands, the head Versions' and the past
    /// Versions' alike. Rows no Branch holds move too but are not counted:
    /// they are on no recipe anybody can open (#162).
    readings: i64,
}

/// Count what a Merge would move, touching nothing.
fn merge_blast_radius(
    conn: &Connection,
    absorbed_food_id: &str,
) -> Result<MergeBlastRadius, OpError> {
    let ingredient_lines: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM readings \
               JOIN branches ON branches.head_version_id = readings.version_id \
              WHERE readings.food_id = ?1",
            params![absorbed_food_id],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot count the lines a Merge moves: {e}")))?;
    Ok(MergeBlastRadius {
        ingredient_lines,
        readings: reachable_reading_count(conn, absorbed_food_id)?,
    })
}

/// A Reading on a Version some Branch still holds, as its head or in its
/// history — a condition on a query whose `readings` table is unaliased.
///
/// A Version outlives every Branch that held it (ADR 0004), and so do its
/// Readings: a deleted recipe leaves them, and so does a save the collapse
/// window replaced. No Door reaches those, so they must not keep a Food alive
/// (#162, Aurélien's choice of 27 September 2026). Every count of a Food's
/// Readings asks this same question, or the Foods screen would offer to
/// delete a Food `delete_food` refuses, or the reverse.
const REACHABLE_READING: &str = "EXISTS (SELECT 1 FROM branch_versions \
                                   WHERE branch_versions.version_id = readings.version_id)";

/// How many Readings some Branch still holds point at a Food.
fn reachable_reading_count(conn: &Connection, food_id: &str) -> Result<i64, OpError> {
    conn.query_row(
        &format!("SELECT COUNT(*) FROM readings WHERE food_id = ?1 AND {REACHABLE_READING}"),
        params![food_id],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot count a Food's Readings: {e}")))
}

/// Delete a Food and everything held about it — its names, and the Merge
/// Suggestions that named it, which without it are dangling rows rather than
/// evidence. Shared by `merge_food` and `delete_food`, the only two ways a
/// Food ever goes.
fn erase_food(conn: &Connection, food_id: &str) -> Result<(), OpError> {
    for statement in [
        "DELETE FROM merge_suggestions WHERE food_a_id = ?1 OR food_b_id = ?1",
        "DELETE FROM food_names WHERE food_id = ?1",
        "DELETE FROM foods WHERE id = ?1",
    ] {
        conn.execute(statement, params![food_id])
            .map_err(|e| OpError::internal(format!("cannot delete Food: {e}")))?;
    }
    Ok(())
}

/// Every other Food already answering to this word in this Language — the
/// duplicate-name check ADR 0022 wants run wherever a name lands on a Food.
fn foods_already_answering_to(
    conn: &Connection,
    language: &str,
    name: &str,
    other_than: &str,
) -> Result<Vec<String>, OpError> {
    let mut statement = conn
        .prepare(
            "SELECT food_id FROM food_names \
             WHERE language = ?1 AND name_folded = ?2 AND food_id <> ?3",
        )
        .map_err(|e| OpError::internal(format!("cannot look up Food: {e}")))?;
    let found = statement
        .query_map(params![language, folded_word(name), other_than], |row| {
            row.get(0)
        })
        .map_err(|e| OpError::internal(format!("cannot look up Food: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot look up Food: {e}")))?;
    Ok(found)
}

/// A Food's Cup Weight, or `None` when it has never been told one.
fn cup_weight_of(conn: &Connection, food_id: &str) -> Result<Option<f64>, OpError> {
    conn.query_row(
        "SELECT cup_weight_grams FROM foods WHERE id = ?1",
        params![food_id],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot read a Food's Cup Weight: {e}")))
}

/// The two figures, when both Foods know one and the two disagree — the one
/// case where a Merge has a question only the Operator can answer.
fn cup_weight_conflict(
    conn: &Connection,
    survivor_food_id: &str,
    absorbed_food_id: &str,
) -> Result<Option<(f64, f64)>, OpError> {
    Ok(
        match (
            cup_weight_of(conn, survivor_food_id)?,
            cup_weight_of(conn, absorbed_food_id)?,
        ) {
            (Some(survivor), Some(absorbed)) if survivor != absorbed => Some((survivor, absorbed)),
            _ => None,
        },
    )
}

/// Both Foods exist and are two rather than one — the checks a Merge and its
/// preview share. Answers them as a reader sees them, which is what both
/// report back.
fn ensure_mergeable(
    conn: &Connection,
    survivor_food_id: &str,
    absorbed_food_id: &str,
    viewer_person_id: &str,
) -> Result<(Value, Value), OpError> {
    if survivor_food_id == absorbed_food_id {
        return Err(OpError::bad_request("a Food cannot be merged into itself"));
    }
    ensure_food_exists(conn, survivor_food_id)?;
    ensure_food_exists(conn, absorbed_food_id)?;
    Ok((
        food_summary(conn, survivor_food_id, viewer_person_id)?,
        food_summary(conn, absorbed_food_id, viewer_person_id)?,
    ))
}

/// Record that two Foods are probably one thing, with the words that said so.
///
/// Evidence, never an instruction (ADR 0022): writing one merges nothing. The
/// pair is stored smaller-id-first, so recording it twice in either order
/// makes one row rather than two.
///
/// Where a pair already has a note, the **stronger** testimony wins.
/// `arrived_as_one` is somebody on another server saying plainly that these
/// words are one Food — ADR 0022 calls a collision "the only trustworthy
/// signal in the whole design" — while a duplicate typed on this instance is
/// merely a coincidence of spelling. So a collision overwrites a typed
/// duplicate, and nothing overwrites a collision.
fn record_merge_suggestion(
    conn: &Connection,
    one_food_id: &str,
    other_food_id: &str,
    reason: &str,
    words: &[(&str, &str)],
) -> Result<(), OpError> {
    if one_food_id == other_food_id {
        return Ok(());
    }
    let (food_a_id, food_b_id) = if one_food_id < other_food_id {
        (one_food_id, other_food_id)
    } else {
        (other_food_id, one_food_id)
    };
    let words = Value::Array(
        words
            .iter()
            .map(|(language, name)| json!({ "language": language, "name": name }))
            .collect(),
    );
    conn.execute(
        "INSERT INTO merge_suggestions (food_a_id, food_b_id, reason, words) \
         VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT(food_a_id, food_b_id) DO UPDATE SET \
            reason = excluded.reason, words = excluded.words, \
            created_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
          WHERE merge_suggestions.reason <> 'arrived_as_one' \
            AND excluded.reason = 'arrived_as_one'",
        params![food_a_id, food_b_id, reason, words.to_string()],
    )
    .map_err(|e| OpError::internal(format!("cannot record a Merge Suggestion: {e}")))?;
    Ok(())
}

/// **Read the Ingredient Lines of a Version that nothing has read yet** (#71),
/// laying a Reading over each line Kamosu can make sense of and leaving the
/// rest alone. Answers how many Readings it wrote.
///
/// `lines` names the ones it may touch, or every line when it is `None`.
/// A save passes the lines that actually changed, because a line nobody
/// edited must keep whatever it has — including nothing, where somebody
/// cleared its Reading on purpose. Regenerating that would silently discard a
/// correction, which ADR 0003 refuses in as many words.
///
/// A line that already carries a Reading is never overwritten here, whoever
/// wrote it, and its word is never resolved either: resolving can create a
/// Food, and a read line needs none (#178). The check comes first; the
/// `INSERT OR IGNORE` behind it is what a Reading would meet if it did not.
///
/// **This returns no error, and that is the point** (#71): reading a line may
/// never fail a save, an import or a recipe. A line Kamosu cannot read is not
/// an error, and neither is a whole recipe of them — so a Food that will not
/// resolve, or a row the database refuses, costs that one line its Reading and
/// nothing else. A signature that could fail would leave the promise resting
/// on every caller remembering to ignore it.
///
/// `start_translation` calls it too (#172). Its input is the translated
/// recipe, never the source's words, so its lines are read in the
/// Translation's own Language like any other first Version's.
pub(super) fn read_unread_lines(
    conn: &Connection,
    version_id: &str,
    language: &str,
    content: &Value,
    lines: Option<&HashSet<usize>>,
) -> u64 {
    let no_lines = Vec::new();
    let ingredients = content["ingredients"].as_array().unwrap_or(&no_lines);
    let mut written = 0;
    for (index, line) in ingredients.iter().enumerate() {
        if lines.is_some_and(|allowed| !allowed.contains(&index)) {
            continue;
        }
        if line["kind"] != "ingredient" {
            continue;
        }
        let Some(text) = line["text"].as_str() else {
            continue;
        };
        // Before the word is resolved, not after: resolving can create a Food
        // (#178). A failed check leaves the line unread rather than risk one.
        let already_read = conn
            .prepare_cached("SELECT 1 FROM readings WHERE version_id = ?1 AND line_index = ?2")
            .and_then(|mut statement| statement.exists(params![version_id, index as i64]))
            .inspect_err(|error| {
                tracing::warn!(
                    target: "kamosu::reading",
                    %error, version_id, index,
                    "a line was left unread"
                )
            });
        if already_read.unwrap_or(true) {
            continue;
        }
        let Some(reading) = crate::reading::read_line(text) else {
            continue;
        };
        let food_id = match reading
            .target
            .as_deref()
            .map(|word| resolve_food_for_word(conn, language, word, None))
            .transpose()
        {
            Ok(food_id) => food_id,
            // One unresolvable Food is one unread line, never a failed save.
            Err(_) => continue,
        };
        match conn.execute(
            "INSERT OR IGNORE INTO readings (version_id, line_index, amount, unit, target, food_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                version_id,
                index as i64,
                reading.amount,
                reading.unit,
                reading.target,
                food_id
            ],
        ) {
            Ok(inserted) => written += inserted as u64,
            Err(error) => tracing::warn!(
                target: "kamosu::reading",
                %error, version_id, index,
                "a line was left unread"
            ),
        }
    }
    written
}

/// Every Reading recorded against one Version, laid out as one slot per
/// Ingredient Line — `null` wherever no Reading has been recorded, which is
/// an entirely ordinary and permanent state for a line (ADR 0002).
pub(super) fn readings_for_version(
    conn: &Connection,
    version_id: &str,
    line_count: usize,
) -> Result<Vec<Value>, OpError> {
    let mut slots = vec![Value::Null; line_count];
    let mut statement = conn
        .prepare(
            "SELECT line_index, amount, unit, target, lineage_id FROM readings WHERE version_id = ?1",
        )
        .map_err(|e| OpError::internal(format!("cannot read Readings: {e}")))?;
    let rows = statement
        .query_map(params![version_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|e| OpError::internal(format!("cannot read Readings: {e}")))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| OpError::internal(format!("cannot read Readings: {e}")))?;
    for (line_index, amount, unit, target, lineage_id) in rows {
        if let Some(slot) = usize::try_from(line_index)
            .ok()
            .and_then(|index| slots.get_mut(index))
        {
            // A Reading's target is either a Food or a Lineage (ADR 0008), and
            // the two sit in the one slot rather than beside each other: a
            // Component is not a line carrying extra, it is a line pointing
            // somewhere else.
            *slot = json!({
                "amount": amount,
                "unit": unit,
                "target": target,
                "lineage_id": lineage_id,
            });
        }
    }
    Ok(slots)
}

/// **The one subordinate line under each Ingredient Line, and what each Step
/// carries beside its text**, worked out in the Core so both Doors get it and
/// an agent asked *how much flour in grams* answers correctly for free
/// (ADR 0016, ADR 0001).
///
/// One slot per line, in the same order, `null` wherever there is nothing to
/// say — which is the common case. A Step's slot is every conversion it
/// offers — its oven and each amount it writes — as `[{ written, measured }]`
/// in the order written (#150).
///
/// It is nothing where Kamosu read no quantity, where the quantity could not
/// be read, and where the line is already in this reader's measures at the
/// Yield they are reading, so a line would only repeat what is already above
/// it.
///
/// `scale` is how far the Yield being cooked is from the Yield as written; a
/// recipe being read rather than cooked is 1.0. Scaling and conversion are one
/// act and share this one slot, so the reader never has to work out which of
/// the two happened.
///
/// Nothing here is stored. Reading Measures is a preference: changing it makes
/// no Version and writes nothing (ADR 0016).
pub(super) fn measured_for_version(
    conn: &Connection,
    content: &Value,
    version_id: &str,
    reader: &Reader,
    scale: f64,
) -> Result<Value, OpError> {
    let Reader { language, measures } = reader;
    let measures = *measures;

    let no_lines = Vec::new();
    let ingredient_lines = content["ingredients"].as_array().unwrap_or(&no_lines);
    let step_lines = content["steps"].as_array().unwrap_or(&no_lines);

    let readings = readings_to_measure(conn, version_id)?;
    // What decides a bare `180°` in a Step, which could be either dial (#150).
    let written_in = units::written_in(readings.iter().filter_map(|r| r.unit.as_deref()));

    // A Step's truth is its text, so everything here is an addition beside it
    // and never written into it (CONTEXT.md, ADR 0016): the oven in the other
    // system, and each amount the Step writes, converted and scaled exactly as
    // an Ingredient Line is (#150). Each is placed straight after what it
    // converts, so they come in the order the text has them. A step already in
    // this reader's measures at this Yield gets nothing.
    let steps: Vec<Value> = step_lines
        .iter()
        .map(|line| {
            if line["kind"] != "step" {
                return Value::Null;
            }
            let Some(text) = line["text"].as_str() else {
                return Value::Null;
            };
            let oven = units::step_temperature(text, measures, language, written_in)
                .map(|oven| (oven.start, oven.written(text), oven.measured));
            let amounts = crate::reading::amounts_in_step(text)
                .into_iter()
                .filter_map(|amount| {
                    let cup_weight = amount
                        .food
                        .as_deref()
                        .and_then(|food| line_named_first(&readings, food))
                        .and_then(|reading| reading.cup_weight);
                    let measured = units::measured_line(
                        Some(&amount.amount),
                        Some(&amount.unit),
                        scale,
                        measures,
                        language,
                        cup_weight,
                    )?;
                    Some((amount.start, amount.written(text), measured))
                });
            let mut conversions: Vec<_> = oven.into_iter().chain(amounts).collect();
            if conversions.is_empty() {
                return Value::Null;
            }
            conversions.sort_by_key(|(start, _, _)| *start);
            conversions
                .into_iter()
                .map(|(_, written, measured)| json!({ "written": written, "measured": measured }))
                .collect()
        })
        .collect();

    let mut ingredients = vec![Value::Null; ingredient_lines.len()];
    for reading in &readings {
        if let Some(slot) = usize::try_from(reading.line_index)
            .ok()
            .and_then(|index| ingredients.get_mut(index))
        {
            *slot = reading.worded(reader, scale);
        }
    }

    Ok(json!({ "ingredients": ingredients, "steps": steps }))
}

/// The Reading a Step's amount is an amount of: the one named soonest in the
/// words after its Unit, so `1 cup panko, a pinch of salt` is panko and
/// `3 oz. freshly grated Parmesan` is Parmesan. Where two names start at the
/// same word the longer wins, so `all-purpose flour` beats `flour`.
///
/// It is the join a Step's `uses` makes ([`named_at`] beneath `names_in`),
/// asked where rather than whether.
fn line_named_first<'a>(readings: &'a [Measurable], food: &str) -> Option<&'a Measurable> {
    let food = folded_for_search(food);
    readings
        .iter()
        .filter_map(|reading| {
            let target = folded_for_search(reading.target.as_deref()?.trim());
            named_at(&food, &target).map(|at| ((at, std::cmp::Reverse(target.len())), reading))
        })
        .min_by_key(|(rank, _)| *rank)
        .map(|(_, reading)| reading)
}

/// Every Reading on a Version that could carry a measurement, with the/// Every Reading on a Version that could carry a measurement, with the
/// effective Cup Weight of the Food it points at.
///
/// **Effective** is the whole point: the figure somebody set on the Food wins,
/// and the shipped staples table answers where nobody has (ADR 0016 — "an
/// override always beats the shipped figure"). The Food's own names are what
/// the shipped table is searched by, with the Reading's bare target word as a
/// fallback for a Reading that resolved to no Food at all.
pub(super) fn readings_to_measure(
    conn: &Connection,
    version_id: &str,
) -> Result<Vec<Measurable>, OpError> {
    // char(31), the ASCII unit separator, joins a Food's names: it is a control
    // character, so no name a cook could type contains one.
    let mut statement = conn
        .prepare(
            "SELECT readings.line_index, readings.amount, readings.unit, readings.target, \
                    foods.cup_weight_grams, \
                    (SELECT group_concat(food_names.name, char(31)) FROM food_names \
                      WHERE food_names.food_id = readings.food_id), \
                    readings.food_id \
               FROM readings LEFT JOIN foods ON foods.id = readings.food_id \
              WHERE readings.version_id = ?1",
        )
        .map_err(|e| OpError::internal(format!("cannot read Readings to measure: {e}")))?;
    let rows = statement
        .query_map(params![version_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<f64>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
            ))
        })
        .map_err(|e| OpError::internal(format!("cannot read Readings to measure: {e}")))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| OpError::internal(format!("cannot read Readings to measure: {e}")))?;

    Ok(rows
        .into_iter()
        .map(
            |(line_index, amount, unit, target, override_weight, names, food_id)| {
                let cup_weight = override_weight.or_else(|| {
                    let names = names.unwrap_or_default();
                    units::shipped_cup_weight(
                        names
                            .split('\u{1f}')
                            .chain(target.as_deref())
                            .filter(|name| !name.is_empty()),
                    )
                });
                Measurable {
                    line_index,
                    amount,
                    unit,
                    target,
                    cup_weight,
                    food_id,
                }
            },
        )
        .collect())
}

/// How one Person reads: the Language they read in and the measures they
/// measure in, fetched once and carried. Both are wanted together everywhere a
/// subordinate line is worded, and a recipe has as many Versions as it has been
/// edited — reading the account row inside that loop would be two queries per
/// Version to answer a question about the reader, who does not change.
pub(super) struct Reader {
    pub(super) language: String,
    pub(super) measures: units::Measures,
}

impl Reader {
    pub(super) fn of(conn: &Connection, person_id: &str) -> Result<Self, OpError> {
        Ok(Self {
            language: reading_language_of(conn, person_id)?,
            measures: units::Measures::from_stored(&reading_measures_of(conn, person_id)?),
        })
    }
}

/// One Reading that might carry a measurement, with the effective Cup Weight of
/// the Food it points at.
pub(super) struct Measurable {
    pub(super) line_index: i64,
    pub(super) amount: Option<String>,
    pub(super) unit: Option<String>,
    /// What the Reading names. A Step's amount is joined to its Ingredient
    /// Line by it, and borrows that line's Cup Weight (#150).
    pub(super) target: Option<String>,
    pub(super) cup_weight: Option<f64>,
    /// The Food this Reading points at, where it found one. Unused when a
    /// recipe is merely being read; a Shopping Row is built on it (#73), and
    /// it rides here rather than in a reader of its own so that a cup of flour
    /// cannot weigh one thing on a recipe page and another in a shop.
    pub(super) food_id: Option<String>,
}

impl Measurable {
    /// The one subordinate line this Reading produces for one reader, or null.
    fn worded(&self, reader: &Reader, scale: f64) -> Value {
        units::measured_line(
            self.amount.as_deref(),
            self.unit.as_deref(),
            scale,
            reader.measures,
            &reader.language,
            self.cup_weight,
        )
        .map_or(Value::Null, Value::from)
    }
}

/// **How far the Yield being cooked is from the Yield as written**, or 1.0.
///
/// ADR 0016 says the subordinate line is "scaled to the Yield being cooked",
/// and the Yield being cooked is a fact held on this cook's own In Progress
/// Attempt — where the cooking screen (#61) will later set it, and where two
/// devices cooking one dish already read it from. Nothing is asked of the
/// reader and nothing is stored on the recipe: a cook who has told Kamosu she
/// is making eight instead of four simply finds the amounts doubled while that
/// cooking is open, and finds them as written again once it ends.
///
/// It is 1.0 for every recipe merely being read, and 1.0 whenever the two
/// Yields cannot honestly be compared: a Yield nobody wrote, an amount that is
/// not a number (`a dozen`), or two different nouns — four *servings* against
/// two *loaves* is not a ratio, and inventing one would put a wrong number on a
/// worktop.
pub(super) fn cooking_scale(
    conn: &Connection,
    lineage_id: &str,
    person_id: &str,
    content: &Value,
) -> Result<f64, OpError> {
    Ok(yield_scale(
        &wanted_yield(conn, lineage_id, person_id, None)?,
        &content["yield"],
    ))
}

/// **How much of this recipe the reader means**, or null for the recipe as
/// written: the Yield they named, where they named one, and otherwise the one
/// their own In Progress Attempt is cooking to.
///
/// Named is the recipe page's scaler (#109) — a view, asked for on a read and
/// stored nowhere, which is why it wins over the cooking: somebody working out
/// what to buy for six is not cooking for six yet. Named as null is the recipe
/// as written, on purpose, and is still a naming.
pub(super) fn wanted_yield(
    conn: &Connection,
    lineage_id: &str,
    person_id: &str,
    named: Option<&Value>,
) -> Result<Value, OpError> {
    if let Some(named) = named {
        return Ok(named.clone());
    }
    Ok(in_progress_attempt(conn, lineage_id, person_id)?
        .map_or(Value::Null, |attempt| attempt["cooking_yield"].clone()))
}

/// **How far a Yield somebody means is from the Yield as written**, or 1.0.
///
/// One rule, two callers: the Yield being *cooked* on an In Progress Attempt
/// (#61) and the Yield being *shopped for* on a Shopping List (#73). They are
/// the same question — how much of this recipe do you mean — and two spellings
/// of it would be two chances to round a worktop's amounts differently from a
/// shop's.
///
/// It is 1.0 whenever the two Yields cannot honestly be compared: a Yield
/// nobody wrote, an amount that is not a number (`a dozen`), or two different
/// nouns — four *servings* against two *loaves* is not a ratio, and inventing
/// one would put a wrong number on a worktop.
///
/// **A wanted Yield with an empty noun is a multiplier** (#109): `{"amount":
/// "2", "noun": ""}` is twice the recipe, whatever it makes — the one way to
/// scale the third of the library that never says what it makes. No written
/// Yield can have an empty noun (`parse_yield` refuses one), so the two never
/// meet.
pub(crate) fn yield_scale(wanted: &Value, written: &Value) -> f64 {
    let wanted_amount = wanted["amount"].as_str().and_then(units::parse_amount);
    if is_multiplier(wanted) {
        return wanted_amount.filter(|times| *times > 0.0).unwrap_or(1.0);
    }
    let same_noun = wanted["noun"].as_str() == written["noun"].as_str();
    let written_amount = written["amount"].as_str().and_then(units::parse_amount);
    match (same_noun, wanted_amount, written_amount) {
        (true, Some(wanted), Some(written)) if written > 0.0 && wanted > 0.0 => wanted / written,
        _ => 1.0,
    }
}

/// Whether a Yield somebody means is a multiplier — `{"amount": "2", "noun":
/// ""}`, twice the recipe — rather than an amount of what the recipe makes
/// (#109). The one place the empty noun is read as that.
pub(crate) fn is_multiplier(wanted: &Value) -> bool {
    wanted["noun"].as_str() == Some("")
}

/// The subordinate line for ONE Ingredient Line — what `set_reading` answers
/// with, so a corrected Reading redraws its own row without Kamosu working out
/// every other line of the recipe to throw them away.
fn measured_for_line(
    conn: &Connection,
    version_id: &str,
    line_index: i64,
    reader: &Reader,
    scale: f64,
) -> Result<Value, OpError> {
    Ok(readings_to_measure(conn, version_id)?
        .iter()
        .find(|reading| reading.line_index == line_index)
        .map_or(Value::Null, |reading| reading.worded(reader, scale)))
}

/// How this Person measures, as stored on their account. The default is
/// American, a stated convention rather than a guess about anybody (ADR 0016).
pub(super) fn reading_measures_of(conn: &Connection, person_id: &str) -> Result<String, OpError> {
    conn.query_row(
        "SELECT reading_measures FROM people WHERE id = ?1",
        params![person_id],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot read Reading Measures: {e}")))
}

/// Of a set of `(language, name)` pairs, the one to show a reader — their own
/// Reading Language where it has a name there, and otherwise the fixed order
/// of [`LANGUAGES`] rather than whatever order SQLite happened to return
/// rows in. Shared by `tag_summary` (#51) and `food_summary` (#47), which
/// show a per-Language name the identical way.
pub(super) fn shown_name<'a>(
    named: &'a [(String, String)],
    reading_language: &str,
) -> Option<&'a (String, String)> {
    named
        .iter()
        .find(|(language, _)| language == reading_language)
        .or_else(|| {
            LANGUAGES
                .iter()
                .find_map(|wanted| named.iter().find(|(language, _)| language == wanted))
        })
}

/// The same `(language, name)` pairs laid out as `list_tags`/`list_foods`
/// serve them: one entry per Language that has one, in [`LANGUAGES`]' fixed
/// order.
pub(super) fn names_json(named: &[(String, String)]) -> Vec<Value> {
    LANGUAGES
        .iter()
        .filter_map(|wanted| {
            named
                .iter()
                .find(|(language, _)| language == wanted)
                .map(|(language, name)| json!({ "language": language, "name": name }))
        })
        .collect()
}

/// Kamosu's whole automatic Food Match (#47, ADR 0022): given the name or
/// names one arriving Food is known by — `(language, word)` pairs — resolve
/// which Food this is, creating one if nothing on the instance answers.
///
/// `exclude`, when given, names the one Reading (`version_id`, `line_index`)
/// this resolution is *for* — so that when `set_reading` corrects an
/// already-read line into fresh ambiguity, the busiest-Food tie-break
/// (ADR 0022) counts existing Readings only, not the very row about to be
/// overwritten still carrying its old Food.
///
/// A Reading or Kamosu's own parser always arrives knowing exactly one name
/// ("one rule at every door" — ADR 0022), which is what `resolve_food_for_word`
/// reduces to below. A Bundle importing a Food from another instance arrives
/// with one entry per Language it has a name in, and is the only situation in
/// which more than one name arrives together. `import_bundle` is therefore
/// the one Operation that reaches the multi-name branches below, and
/// `a_food_whose_names_hit_two_foods_here_arrives_as_a_third_and_a_suggestion`
/// drives them through it.
pub(super) fn resolve_food_for_names(
    conn: &Connection,
    names: &[(&str, &str)],
    exclude: Option<(&str, i64)>,
) -> Result<String, OpError> {
    assert!(
        !names.is_empty(),
        "a Food always arrives known by at least one name"
    );

    // Every existing Food that any arriving name already answers to, in the
    // same Language, deduplicated — two arriving names may hit the same Food.
    let mut hits: Vec<String> = Vec::new();
    for (language, word) in names {
        let folded = folded_word(word);
        let mut statement = conn
            .prepare("SELECT food_id FROM food_names WHERE language = ?1 AND name_folded = ?2")
            .map_err(|e| OpError::internal(format!("cannot look up Food: {e}")))?;
        let found: Vec<String> = statement
            .query_map(params![language, folded], |row| row.get(0))
            .map_err(|e| OpError::internal(format!("cannot look up Food: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot look up Food: {e}")))?;
        for food_id in found {
            if !hits.contains(&food_id) {
                hits.push(food_id);
            }
        }
    }

    if names.len() == 1 && hits.len() >= 2 {
        // A lone word asserts nothing about which Food is meant — welding
        // two Foods together on that alone is the very weld ADR 0022
        // refuses. Go with whichever the most Readings already point at.
        return busiest_food(conn, &hits, exclude);
    }

    match hits.len() {
        0 => create_food(conn, names),
        1 => {
            // The surviving Food learns every arriving name that matches
            // nothing at all on the instance (ADR 0022) — but never
            // overwrites a Language it already has a name in.
            let food_id = hits.into_iter().next().unwrap();
            let held: Vec<String> = {
                let mut statement = conn
                    .prepare("SELECT language FROM food_names WHERE food_id = ?1")
                    .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?;
                statement
                    .query_map(params![food_id], |row| row.get(0))
                    .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?
            };
            for (language, word) in names {
                if held.iter().any(|held_language| held_language == language) {
                    continue;
                }
                conn.execute(
                    "INSERT INTO food_names (food_id, language, name, name_folded) \
                     VALUES (?1, ?2, ?3, ?4)",
                    params![food_id, language, word, folded_word(word)],
                )
                .map_err(|e| OpError::internal(format!("cannot name Food: {e}")))?;
            }
            Ok(food_id)
        }
        _ => {
            // Doubt makes a new Food, never a merge (ADR 0022): a third Food
            // carries every name every hit Food already has, plus every
            // arriving name for a Language none of them covers. The hit
            // Foods themselves are untouched — nothing merged, nothing
            // destroyed.
            let mut carried: Vec<(String, String)> = Vec::new();
            for food_id in &hits {
                let mut statement = conn
                    .prepare("SELECT language, name FROM food_names WHERE food_id = ?1")
                    .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?;
                let rows: Vec<(String, String)> = statement
                    .query_map(params![food_id], |row| Ok((row.get(0)?, row.get(1)?)))
                    .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?;
                for (language, name) in rows {
                    if !carried
                        .iter()
                        .any(|(carried_language, _)| *carried_language == language)
                    {
                        carried.push((language, name));
                    }
                }
            }
            for (language, word) in names {
                if !carried
                    .iter()
                    .any(|(carried_language, _)| carried_language == language)
                {
                    carried.push(((*language).to_string(), (*word).to_string()));
                }
            }
            let carried_refs: Vec<(&str, &str)> = carried
                .iter()
                .map(|(language, name)| (language.as_str(), name.as_str()))
                .collect();
            let third = create_food(conn, &carried_refs)?;

            // The collision is testimony, not a resemblance: somebody put
            // these words in one Food, and that is the only trustworthy
            // signal in the whole design (ADR 0022). It is recorded across
            // every pair in the cluster — the Foods that were hit and the
            // third just minted — so the Operator's worklist shows one
            // group to consider rather than a single arbitrary pair.
            let cluster: Vec<&str> = hits
                .iter()
                .map(String::as_str)
                .chain(std::iter::once(third.as_str()))
                .collect();
            for (position, one) in cluster.iter().enumerate() {
                for other in &cluster[position + 1..] {
                    record_merge_suggestion(conn, one, other, "arrived_as_one", names)?;
                }
            }
            Ok(third)
        }
    }
}

/// The lone-word reduction of [`resolve_food_for_names`] — what a Reading's
/// `target` or Kamosu's own parser always supplies.
fn resolve_food_for_word(
    conn: &Connection,
    language: &str,
    word: &str,
    exclude: Option<(&str, i64)>,
) -> Result<String, OpError> {
    resolve_food_for_names(conn, &[(language, word)], exclude)
}

/// Among Foods a lone ambiguous word hit, the one the most Readings already
/// point at (ADR 0022) — `exclude` left out of that count, see
/// `resolve_food_for_names`. Ties — commonly all-zero, since nothing has
/// read either yet — resolve to whichever Food was minted first, so the
/// same input always resolves the same way rather than to whatever order
/// SQLite happened to return rows in.
fn busiest_food(
    conn: &Connection,
    candidates: &[String],
    exclude: Option<(&str, i64)>,
) -> Result<String, OpError> {
    let placeholders = candidates
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");
    // Only Readings some Branch holds, so this agrees with the count a Food
    // reports (#162).
    let mut join_condition = format!("readings.food_id = foods.id AND {REACHABLE_READING}");
    if exclude.is_some() {
        join_condition.push_str(" AND NOT (readings.version_id = ? AND readings.line_index = ?)");
    }
    let sql = format!(
        "SELECT foods.id FROM foods \
         LEFT JOIN readings ON {join_condition} \
         WHERE foods.id IN ({placeholders}) \
         GROUP BY foods.id \
         ORDER BY COUNT(readings.food_id) DESC, foods.created_at ASC, foods.id ASC \
         LIMIT 1"
    );
    let mut bound: Vec<&dyn rusqlite::ToSql> = Vec::new();
    if let Some((version_id, line_index)) = &exclude {
        bound.push(version_id);
        bound.push(line_index);
    }
    for candidate in candidates {
        bound.push(candidate);
    }
    conn.query_row(&sql, bound.as_slice(), |row| row.get(0))
        .map_err(|e| OpError::internal(format!("cannot resolve ambiguous Food: {e}")))
}

/// Mint a brand-new Food carrying exactly the given names — the "created
/// automatically from whatever word a Reading found" half of #47.
fn create_food(conn: &Connection, names: &[(&str, &str)]) -> Result<String, OpError> {
    let food_id = format!("f_{}", hex::encode(random_bytes(8)));
    conn.execute("INSERT INTO foods (id) VALUES (?1)", params![food_id])
        .map_err(|e| OpError::internal(format!("cannot create Food: {e}")))?;
    for (language, word) in names {
        conn.execute(
            "INSERT INTO food_names (food_id, language, name, name_folded) VALUES (?1, ?2, ?3, ?4)",
            params![food_id, language, word, folded_word(word)],
        )
        .map_err(|e| OpError::internal(format!("cannot name Food: {e}")))?;
    }
    Ok(food_id)
}

/// That a Food exists at all — and, by failing, that it doesn't.
fn ensure_food_exists(conn: &rusqlite::Connection, food_id: &str) -> Result<(), OpError> {
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM foods WHERE id = ?1",
            params![food_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .map_err(|e| OpError::internal(format!("cannot look up Food: {e}")))?;
    if !exists {
        return Err(OpError::not_found("no such Food"));
    }
    Ok(())
}

/// One Food as a reader sees it: every name it has, the one to show them —
/// their Reading Language where the Food has a name there, and otherwise
/// whatever name it does have, the same fallback `tag_summary` uses (#51) —
/// its Cup Weight, its (always-null in v1) nutrition slot, and how many
/// Readings currently point at it. The reading count is what lets the
/// ADR 0022 busiest tie-break be verified through Operations rather than
/// taken on faith, and is exactly the number an unreferenced-Food sweep
/// would need to be zero before ever touching one (#48).
fn food_summary(
    conn: &rusqlite::Connection,
    food_id: &str,
    viewer_person_id: &str,
) -> Result<Value, OpError> {
    let cup_weight_grams: Option<f64> = conn
        .query_row(
            "SELECT cup_weight_grams FROM foods WHERE id = ?1",
            params![food_id],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot read Food: {e}")))?;

    let reading_count = reachable_reading_count(conn, food_id)?;

    let mut statement = conn
        .prepare("SELECT language, name FROM food_names WHERE food_id = ?1")
        .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?;
    let named: Vec<(String, String)> = statement
        .query_map(params![food_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?;

    let reading_language = reading_language_of(conn, viewer_person_id)?;
    let shown = shown_name(&named, &reading_language);
    let names = names_json(&named);

    Ok(json!({
        "id": food_id,
        "name": shown.map(|(_, name)| name.as_str()),
        "language": shown.map(|(language, _)| language.as_str()),
        "names": names,
        "cup_weight_grams": cup_weight_grams,
        "nutrition": Value::Null,
        "reading_count": reading_count,
    }))
}

#[cfg(test)]
mod tests {
    /// The "two hits make a third Food" half of ADR 0022 only ever fires for
    /// an arriving Food known by more than one name — exclusively a Bundle
    /// from another instance. The behaviour test drives it through
    /// `import_bundle`; this one pins the matcher itself, including a weaker
    /// suggestion the collision must overwrite.
    #[test]
    fn resolve_food_for_names_makes_a_third_food_on_collision() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::Db::open(dir.path()).unwrap();
        db.with_conn(|conn| {
            let food_a = super::create_food(conn, &[("fr", "farine")]).unwrap();
            let food_b = super::create_food(conn, &[("en", "flour")]).unwrap();

            // A weaker note already stands between them: somebody typed one
            // Food's word onto the other on this instance. The collision
            // below is far stronger testimony — ADR 0022 calls it "the only
            // trustworthy signal in the whole design" — so it must overwrite
            // this one rather than being dropped as a duplicate row.
            super::record_merge_suggestion(
                conn,
                &food_a,
                &food_b,
                "name_typed_onto_another",
                &[("fr", "farine")],
            )
            .unwrap();

            // An arriving Food naming both farine (fr) and flour (en) hits
            // both existing Foods by two different words — doubt makes a
            // third carrying both rather than welding food_a and food_b
            // together.
            let food_c =
                super::resolve_food_for_names(conn, &[("fr", "farine"), ("en", "flour")], None)
                    .unwrap();
            assert_ne!(food_c, food_a, "nothing is merged");
            assert_ne!(food_c, food_b, "nothing is merged");

            let names_of = |food_id: &str| -> Vec<(String, String)> {
                conn.prepare("SELECT language, name FROM food_names WHERE food_id = ?1")
                    .unwrap()
                    .query_map(rusqlite::params![food_id], |row| {
                        Ok((row.get(0)?, row.get(1)?))
                    })
                    .unwrap()
                    .collect::<Result<_, _>>()
                    .unwrap()
            };

            assert_eq!(
                names_of(&food_a),
                vec![("fr".to_string(), "farine".to_string())],
                "the original Food is untouched — nothing destroyed"
            );
            assert_eq!(
                names_of(&food_b),
                vec![("en".to_string(), "flour".to_string())],
                "the original Food is untouched — nothing destroyed"
            );
            let mut carried = names_of(&food_c);
            carried.sort();
            assert_eq!(
                carried,
                vec![
                    ("en".to_string(), "flour".to_string()),
                    ("fr".to_string(), "farine".to_string()),
                ],
                "the new Food carries every arriving name"
            );

            // The collision is testimony, and it is kept as such (#48): every
            // pair in the cluster of three is recorded, with the words that
            // said so. Nothing was merged by recording it.
            let suggestions: Vec<(String, String, String, String)> = conn
                .prepare(
                    "SELECT food_a_id, food_b_id, reason, words \
                     FROM merge_suggestions ORDER BY food_a_id, food_b_id",
                )
                .unwrap()
                .query_map([], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
                })
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap();
            assert_eq!(
                suggestions.len(),
                3,
                "the three Foods are pairwise suggested: {suggestions:#?}"
            );
            for (food_a_id, food_b_id, reason, words) in &suggestions {
                assert!(food_a_id < food_b_id, "the pair is held smaller id first");
                assert_eq!(reason, "arrived_as_one");
                assert_eq!(
                    serde_json::from_str::<serde_json::Value>(words).unwrap(),
                    serde_json::json!([
                        { "language": "fr", "name": "farine" },
                        { "language": "en", "name": "flour" },
                    ]),
                    "the arriving words are the evidence"
                );
            }

            // A lone word matching the same two Foods, by contrast, never
            // makes a fourth Food — it goes to the busiest one instead, and
            // records nothing: it is already inside a flagged cluster.
            let resolved = super::resolve_food_for_word(conn, "fr", "farine", None).unwrap();
            assert!(
                resolved == food_a || resolved == food_c,
                "a lone ambiguous word resolves to an existing Food, never a new one"
            );
            let still: i64 = conn
                .query_row("SELECT COUNT(*) FROM merge_suggestions", [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(still, 3, "a lone ambiguous word adds no new evidence");

            Ok(())
        })
        .unwrap();
    }
}
