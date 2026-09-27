//! Translation, and a Branch's Language (ADR 0006).

use super::*;

impl Core {
    /// Start a Translation: a new Branch of the same Lineage in a different
    /// Language, whose first Version records which Version of the source it
    /// renders (ADR 0006).
    ///
    /// **There is no Translation object.** What this makes is an ordinary
    /// Branch — the same row, the same chain, the same Operations from here
    /// on. It is a Translation only in the sense that its Language differs
    /// from the Branch it grew out of and its Versions point at what they
    /// translate, both of which are facts read off ordinary columns rather
    /// than a flag anyone maintains.
    ///
    /// The chain starts fresh at sequence 1 with no parent, which is what
    /// separates this from a Copy: a Copy is the same words continuing, so it
    /// carries the whole chain behind it, whereas a Translation is different
    /// words rendering the same dish and has a history of its own from its
    /// first line.
    ///
    /// An agent asked to translate calls exactly this, under the Person's own
    /// Credential: the Hand on the Branch is that Person's Cookbook's, the Hand
    /// on the Version is the Person's, and the Access Key is recorded locally
    /// and travels nowhere (ADR 0015). A scribe, not an author.
    ///
    /// The Translation is always the caller's own, in their own Cookbook
    /// (ADR 0041): of a recipe they write, it sits beside it there; of anybody
    /// else's they may see, it is a Branch of their own, as a change to it
    /// would be.
    #[allow(clippy::too_many_arguments)]
    pub fn start_translation(
        &self,
        caller: &Caller,
        source_branch_id: &str,
        language: &str,
        input: &Value,
        name: Option<&str>,
        change_note: Option<&str>,
        translates_version_id: Option<&str>,
    ) -> Result<Value, OpError> {
        let language = supported_branch_language(required_text(language, "language")?)?.to_string();
        let content = parse_recipe_content(input)?;
        let (version_id, content_text) = stored_version(&content);
        let branch_id = format!("b_{}", hex::encode(random_bytes(8)));

        self.db().with_conn(|conn| {
            ensure_sees_branch(conn, source_branch_id, &caller.person_id)?;
            let (lineage_id, source_language, source_head): (String, String, String) = conn
                .query_row(
                    "SELECT lineage_id, language, head_version_id FROM branches WHERE id = ?1",
                    params![source_branch_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?;

            // Unknown can neither be a Translation nor have one (ADR 0006). A
            // recipe that is honestly two Languages has no single source text
            // to render, and nothing to render it into.
            if crate::language::is_stated_unknown(&source_language) {
                return Err(OpError::bad_request(
                    "a recipe whose Language is unknown cannot have a Translation: \
                     it is honestly more than one Language already",
                ));
            }
            if crate::language::is_stated_unknown(&language) {
                return Err(OpError::bad_request(
                    "a Translation is written in a Language, not in unknown",
                ));
            }
            if language == source_language {
                return Err(OpError::bad_request(format!(
                    "a Translation is in a different Language from the recipe it \
                     translates, and this one is already in {source_language}"
                )));
            }

            // Always the caller's own Cookbook, the same rule a save follows.
            let own_cookbook_id = cookbook_of_person(conn, &caller.person_id)?;
            let branch_name = name_on_arrival(
                conn,
                &own_cookbook_id,
                &lineage_id,
                &language,
                &whose_branch(conn, source_branch_id)?,
            )?;

            // `branch_id` names a Branch that does not exist yet, so the
            // "on some Branch other than this one" half of the check is
            // trivially satisfied here — what it is really asking is that the
            // Version belongs to this Lineage at all. The same call from a
            // save, where the Branch does exist, is where the exclusion earns
            // its keep.
            let translates = match translates_version_id {
                Some(named) => translated_source_version(
                    conn,
                    &lineage_id,
                    &branch_id,
                    required_text(named, "translates_version_id")?,
                )?,
                None => source_head,
            };

            let hand_id = cookbook_hand(conn, &own_cookbook_id)?;

            conn.execute(
                "INSERT OR IGNORE INTO versions (id, content) VALUES (?1, ?2)",
                params![version_id, content_text],
            )
            .map_err(|e| OpError::internal(format!("cannot record Version: {e}")))?;
            conn.execute(
                "INSERT INTO branches (id, lineage_id, cookbook_id, hand_id, language, head_version_id, name, started_by) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    branch_id,
                    lineage_id,
                    own_cookbook_id,
                    hand_id,
                    language,
                    version_id,
                    branch_name,
                    caller.person_id
                ],
            )
            .map_err(|e| OpError::internal(format!("cannot start Branch: {e}")))?;
            conn.execute(
                "INSERT INTO branch_versions \
                 (branch_id, sequence, version_id, parent_version_id, hand_id, name, change_note, access_key_id, translates_version_id, language) \
                 VALUES (?1, 1, ?2, NULL, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    branch_id,
                    version_id,
                    caller.person_id,
                    name,
                    change_note,
                    caller.access_key_id,
                    translates,
                    language
                ],
            )
            .map_err(|e| OpError::internal(format!("cannot record first Version: {e}")))?;
            // The input is the recipe as it now reads in the new Language, so
            // its lines are read here, in that Language, as a new recipe's are
            // (#172). No later save would: it carries an unchanged line's
            // Reading forward, and here there is none to carry.
            read_unread_lines(conn, &version_id, &language, &content, None);
            Ok(())
        })?;

        self.get_recipe(&caller.person_id, &branch_id, None)
    }

    /// Say what Language a recipe is written in — the *never changed without
    /// the cook saying so* half of ADR 0006's rule, and the only thing that
    /// acts on a save's `language_offer`.
    ///
    /// **Changing a Language makes a Version.** Once recipes travel between
    /// instances, a label alterable without a trace would be a hole in an
    /// otherwise append-only history, so this appends an occurrence of the
    /// current head content carrying the new Language rather than editing the
    /// Branch quietly. The content is unchanged, so the fingerprint is the
    /// same one — the same Version occurring twice on one Branch, which the
    /// chain has always allowed and which `sequence` tells apart.
    ///
    /// Setting **Unknown** is how a cook says a recipe is honestly more than
    /// one Language. From then on it is offered nothing, marked nothing, and
    /// shown to every reader whatever they read in.
    pub fn set_recipe_language(
        &self,
        caller: &Caller,
        branch_id: &str,
        language: &str,
    ) -> Result<Value, OpError> {
        let language = supported_branch_language(required_text(language, "language")?)?.to_string();
        self.db().with_conn(|conn| {
            ensure_sees_branch(conn, branch_id, &caller.person_id)?;
            let (lineage_id, current, head_version_id): (String, String, String) = conn
                .query_row(
                    "SELECT lineage_id, language, head_version_id FROM branches WHERE id = ?1",
                    params![branch_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?;

            if language == current {
                // Already what it says: nothing changed, so no Version. Saying
                // a recipe is in the Language it is already in is not an edit.
                return Ok(json!({
                    "branch_id": branch_id,
                    "language": current,
                    "sequence": Value::Null,
                }));
            }

            // A Translation renders one source text, and a recipe that is
            // honestly two Languages renders none — so calling one Unknown
            // would leave a pointer with nothing on the end of it (ADR 0006).
            if crate::language::is_stated_unknown(&language)
                && translation_source(conn, branch_id)?.is_some()
            {
                return Err(OpError::bad_request(
                    "this recipe is a Translation, so it is written in the Language \
                     it was translated into, not in unknown",
                ));
            }
            if crate::language::is_stated_unknown(&language)
                && has_translations(conn, &lineage_id, branch_id)?
            {
                return Err(OpError::bad_request(
                    "this recipe has a Translation, so it is the single Language \
                     that Translation renders, not unknown",
                ));
            }

            // Saying what Language a recipe is in is a change to it, so on a
            // Branch another Cookbook writes — a Kitchen-mate's, or one that
            // arrived in a Bundle — it is a Copy like any other change (ADR
            // 0020, ADR 0041), and the new Language lands on the Copy.
            let own_cookbook_id = cookbook_of_person(conn, &caller.person_id)?;
            let branch_id = if cookbook_writes_branch(conn, &own_cookbook_id, branch_id)? {
                branch_id.to_string()
            } else {
                start_copy(
                    conn,
                    branch_id,
                    &lineage_id,
                    &own_cookbook_id,
                    &caller.person_id,
                    &current,
                    &head_version_id,
                )?
            };
            let branch_id = branch_id.as_str();

            let (head_sequence, head_translates): (i64, Option<String>) = conn
                .query_row(
                    "SELECT sequence, translates_version_id FROM branch_versions \
                      WHERE branch_id = ?1 ORDER BY sequence DESC LIMIT 1",
                    params![branch_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(|e| OpError::internal(format!("cannot read Branch head: {e}")))?;

            // The new occurrence carries the head's own content unchanged — no
            // word of the recipe changed, only what it is said to be written
            // in — so its `version_id` and its `parent_version_id` are both
            // the head Version. That is not a loop: `parent_version_id` names
            // the Version this occurrence *follows on this Branch*, and what
            // it follows is text identical to its own. It is exactly the
            // invariant `ordered_chain` checks — each row's parent is the
            // previous row's Version — and the schema has always allowed one
            // Version id to occur more than once on one Branch, told apart by
            // `sequence`.
            conn.execute(
                "INSERT INTO branch_versions \
                 (branch_id, sequence, version_id, parent_version_id, hand_id, access_key_id, translates_version_id, language) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    branch_id,
                    head_sequence + 1,
                    head_version_id,
                    head_version_id,
                    caller.person_id,
                    caller.access_key_id,
                    head_translates,
                    language
                ],
            )
            .map_err(|e| OpError::internal(format!("cannot record the Language change: {e}")))?;
            conn.execute(
                "UPDATE branches SET language = ?1 WHERE id = ?2",
                params![language, branch_id],
            )
            .map_err(|e| OpError::internal(format!("cannot set the Language: {e}")))?;

            Ok(json!({
                "branch_id": branch_id,
                "language": language,
                "sequence": head_sequence + 1,
            }))
        })
    }
}

/// The Language a Branch that has none yet starts in: the one stated outright,
/// or — with none stated — what the recipe's own text reads as, or, where there
/// is too little text to tell, the writer's own Reading Language (ADR 0006).
///
/// This is the *set when blank* half of the rule and the only place detection
/// ever writes. `create_recipe` and a freshly-seen Import candidate share it
/// because they are the same moment: a recipe arriving with no Language yet,
/// and so nothing for detection to disagree with (ADR 0025).
///
/// Falling back to the Reading Language rather than to Unknown is deliberate:
/// Unknown means "honestly more than one Language", which is a statement about
/// the recipe, and a recipe that is nothing but a title has made no such
/// statement. The cook's own Language is the better guess, and the next save
/// that has real text in it will offer to correct it.
pub(super) fn language_for_new_branch(
    conn: &Connection,
    person_id: &str,
    stated: Option<&str>,
    content: &Value,
) -> Result<String, OpError> {
    if let Some(stated) = stated {
        return Ok(supported_branch_language(required_text(stated, "language")?)?.to_string());
    }
    if let Some(detected) = crate::language::detect_content(content) {
        return Ok(detected.to_string());
    }
    reading_language_of(conn, person_id)
}

/// Check that a Version being claimed as the source of a Translation really is
/// one: a Version of this Lineage, on some Branch other than the translating
/// one. Answers it back so the caller stores what was checked rather than what
/// was asked for.
///
/// A Translation of its own text is the one shape worth refusing outright —
/// it would make staleness self-referential, and "how far behind itself has it
/// fallen" is not a question.
pub(super) fn translated_source_version(
    conn: &Connection,
    lineage_id: &str,
    translating_branch_id: &str,
    version_id: &str,
) -> Result<String, OpError> {
    let occurs: bool = conn
        .query_row(
            "SELECT EXISTS ( \
                SELECT 1 FROM branch_versions \
                  JOIN branches ON branches.id = branch_versions.branch_id \
                 WHERE branch_versions.version_id = ?1 \
                   AND branches.lineage_id = ?2 \
                   AND branch_versions.branch_id <> ?3 )",
            params![version_id, lineage_id, translating_branch_id],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot read the source Version: {e}")))?;
    if !occurs {
        return Err(OpError::bad_request(
            "translates_version_id must name a Version of another Branch of this recipe",
        ));
    }
    Ok(version_id.to_string())
}

/// What this Branch's newest Version translates, if anything — the whole of
/// how "is this a Translation" is answered (ADR 0006). The original is the
/// Branch that translates nothing; nobody declares which one that is.
fn translation_source(conn: &Connection, branch_id: &str) -> Result<Option<String>, OpError> {
    conn.query_row(
        "SELECT translates_version_id FROM branch_versions \
          WHERE branch_id = ?1 ORDER BY sequence DESC LIMIT 1",
        params![branch_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read what this Version translates: {e}")))
    .map(Option::flatten)
}

/// Whether any other Branch of this Lineage renders a Version of this one.
fn has_translations(conn: &Connection, lineage_id: &str, branch_id: &str) -> Result<bool, OpError> {
    conn.query_row(
        "SELECT EXISTS ( \
            SELECT 1 FROM branch_versions AS translation \
              JOIN branches ON branches.id = translation.branch_id \
              JOIN branch_versions AS source \
                ON source.version_id = translation.translates_version_id \
             WHERE branches.lineage_id = ?1 \
               AND translation.branch_id <> ?2 \
               AND source.branch_id = ?2 )",
        params![lineage_id, branch_id],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot look for Translations: {e}")))
}

/// How this Branch stands as a Translation: what its newest Version renders,
/// which Branch that Version is on, and how many Versions that Branch has
/// moved on since — the exact staleness ADR 0006 promised, computed from
/// current facts rather than marked by anyone.
///
/// Null for the great majority of recipes, which translate nothing.
pub(super) fn translation_of_branch(
    conn: &Connection,
    lineage_id: &str,
    branch_id: &str,
) -> Result<Value, OpError> {
    let Some(translates_version_id) = translation_source(conn, branch_id)? else {
        return Ok(Value::Null);
    };

    // The source Branch, and where in its chain the translated Version sits.
    // The **oldest Branch** holding it, so a Version that later converged onto
    // a second Branch is still measured against the one it was translated from
    // — but that Branch's **newest** occurrence of it.
    //
    // Newest matters, and it is the whole of what makes staleness honest. One
    // Version id may occur more than once on one Branch: saying what Language
    // a recipe is written in appends its current content again, unchanged
    // (ADR 0006), and so does a save that reverts to exact earlier text.
    // Counting from the oldest occurrence would make every such recipe one
    // Version behind for ever, with nothing a cook could do about it — the
    // pointer already names the newest text, so re-pointing would resolve to
    // the old occurrence again. Counting from the newest asks the only
    // question worth asking: since this text was last what the recipe said,
    // how much has been written?
    let found: Option<(String, i64)> = conn
        .query_row(
            "SELECT branch_versions.branch_id, branch_versions.sequence \
               FROM branch_versions \
               JOIN branches ON branches.id = branch_versions.branch_id \
              WHERE branch_versions.version_id = ?1 \
                AND branches.lineage_id = ?2 \
                AND branch_versions.branch_id <> ?3 \
              ORDER BY branches.created_at ASC, branch_versions.sequence DESC LIMIT 1",
            params![translates_version_id, lineage_id, branch_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read the source Branch: {e}")))?;

    // The source Branch can be absent on an instance that received the
    // Translation alone. The pointer is still the truth about what was
    // translated; how far behind it has fallen is simply unanswerable here,
    // and saying so beats inventing a zero.
    let Some((source_branch_id, source_sequence)) = found else {
        return Ok(json!({
            "translates_version_id": translates_version_id,
            "source_branch_id": Value::Null,
            "versions_behind": Value::Null,
        }));
    };

    let versions_behind: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM branch_versions WHERE branch_id = ?1 AND sequence > ?2",
            params![source_branch_id, source_sequence],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot measure the Translation: {e}")))?;

    Ok(json!({
        "translates_version_id": translates_version_id,
        "source_branch_id": source_branch_id,
        "versions_behind": versions_behind,
    }))
}

/// One of the Languages Kamosu is written in — the guard on anything named per
/// Language. A Tag or a Food may be named in any of them and need be named in
/// only one.
pub(super) fn supported_language(language: &str) -> Result<&str, OpError> {
    if LANGUAGES.contains(&language) {
        Ok(language)
    } else {
        Err(OpError::bad_request(format!(
            "language must be one of {}",
            LANGUAGES.join(", ")
        )))
    }
}

/// The Languages a **Branch** may carry: the three Kamosu is written in, plus
/// **Unknown** for a recipe that is honestly more than one (ADR 0006). Wider
/// than [`supported_language`], which guards a Tag's or a Food's name — a word
/// in no language at all is a mistake, whereas a recipe in no single one is a
/// real and ordinary recipe.
pub(super) fn supported_branch_language(language: &str) -> Result<&str, OpError> {
    if crate::language::BRANCH_LANGUAGES.contains(&language) {
        Ok(language)
    } else {
        Err(OpError::bad_request(format!(
            "language must be one of {}",
            crate::language::BRANCH_LANGUAGES.join(", ")
        )))
    }
}
