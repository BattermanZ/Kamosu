//! Tags: making, renaming, merging and deleting them, and putting one on a
//! recipe.

use super::*;

impl Core {
    /// Create a Tag in the caller's own Cookbook, named in one Language. Tags
    /// belong to the Cookbook, with its recipes (#131, question 3), so any of
    /// its Co-authors may and nobody else does.
    ///
    /// A word already used in that Cookbook and Language does not make a second
    /// Tag: the one already there is returned, which is what keeps *dessert*
    /// and *dessert* one Tag (#51). Two people reaching for the same word have
    /// agreed, not collided.
    pub fn create_tag(
        &self,
        person_id: &str,
        language: &str,
        name: &str,
    ) -> Result<Value, OpError> {
        let language = supported_language(language)?;
        let name = required_text(name, "name")?.to_string();
        let tag_id = format!("t_{}", hex::encode(random_bytes(8)));
        self.db().with_conn(|conn| {
            let cookbook_id = cookbook_of_person(conn, person_id)?;
            if let Some(existing) = tag_id_for_word(conn, &cookbook_id, language, &name)? {
                return tag_summary(conn, &existing, person_id);
            }
            conn.execute(
                "INSERT INTO tags (id, cookbook_id) VALUES (?1, ?2)",
                params![tag_id, cookbook_id],
            )
            .map_err(|e| OpError::internal(format!("cannot create Tag: {e}")))?;
            conn.execute(
                "INSERT INTO tag_names (tag_id, cookbook_id, language, name, name_folded) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![tag_id, cookbook_id, language, name, folded_word(&name)],
            )
            .map_err(|e| OpError::internal(format!("cannot name Tag: {e}")))?;
            tag_summary(conn, &tag_id, person_id)
        })
    }

    /// Every Tag the caller's own Cookbook files by, each shown in the
    /// reader's Reading Language and falling back to whatever name it does
    /// have (#51).
    ///
    /// `everywhere` widens it to every Cookbook the caller may see — the words
    /// a shelf mixing several Cookbooks can be filtered by. There one word is
    /// one entry, whichever Cookbooks use it: "Dessert" from two Cookbooks is
    /// one filter, and filtering by it finds both (#131, question 3).
    ///
    /// Three reaches: your own Cookbook's Tags (the default, what Settings
    /// renames); with `everywhere`, every word of every Cookbook you may see;
    /// with a Kitchen, every word of the Cookbooks seen in that Kitchen. The
    /// last two answer one entry per word, your own first, since a shelf's
    /// filter matches the word across Cookbooks (#131, answer 3).
    pub fn list_tags(
        &self,
        person_id: &str,
        everywhere: bool,
        kitchen_id: Option<&str>,
    ) -> Result<Vec<Value>, OpError> {
        self.db().with_conn(|conn| {
            let own = cookbook_of_person(conn, person_id)?;
            if let Some(kitchen_id) = kitchen_id {
                ensure_member(conn, kitchen_id, person_id)?;
            }
            let merged = everywhere || kitchen_id.is_some();
            let ids: Vec<String> = {
                let mut statement = conn
                    .prepare(&format!(
                        "SELECT tags.id FROM tags \
                          WHERE CASE \
                                  WHEN ?4 IS NOT NULL THEN tags.cookbook_id IN ({}) \
                                  WHEN ?2 THEN tags.cookbook_id IN \
                                    (SELECT cookbook_id FROM visible_cookbooks WHERE person_id = ?3) \
                                  ELSE tags.cookbook_id = ?1 \
                                END \
                          ORDER BY tags.cookbook_id <> ?1, tags.created_at, tags.id",
                        cookbooks_seen_in("?4"),
                    ))
                    .map_err(|e| OpError::internal(format!("cannot list Tags: {e}")))?;
                statement
                    .query_map(params![own, everywhere, person_id, kitchen_id], |row| {
                        row.get(0)
                    })
                    .map_err(|e| OpError::internal(format!("cannot list Tags: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot list Tags: {e}")))?
            };
            let mut seen: HashSet<String> = HashSet::new();
            let mut tags: Vec<Value> = Vec::new();
            for id in &ids {
                let tag = tag_summary(conn, id, person_id)?;
                let word = folded_word(tag["name"].as_str().unwrap_or_default());
                if merged && !seen.insert(word) {
                    continue;
                }
                tags.push(tag);
            }
            Ok(tags)
        })
    }

    /// Give a Tag its name in one Language — the first name it has there, or a
    /// different one in place of the name it had.
    ///
    /// Because the Tag is kept once and pointed at, this reaches every recipe
    /// carrying it at once, with nothing to walk and nothing to re-save: no
    /// Version is minted and no fingerprint moves (ADR 0035).
    pub fn rename_tag(
        &self,
        person_id: &str,
        tag_id: &str,
        language: &str,
        name: &str,
    ) -> Result<Value, OpError> {
        let language = supported_language(language)?;
        let name = required_text(name, "name")?.to_string();
        self.db().with_conn(|conn| {
            let cookbook_id = cookbook_of_tag(conn, tag_id)?;
            ensure_writes_or_absent(conn, &cookbook_id, person_id, no_such_tag)?;
            // The word may already be this Tag's own — renaming *dessert* to
            // *Dessert* is a change of spelling, not a collision with itself.
            match tag_id_for_word(conn, &cookbook_id, language, &name)? {
                Some(owner) if owner != tag_id => {
                    return Err(OpError::bad_request(
                        "this Cookbook already files under that word in that Language",
                    ));
                }
                _ => {}
            }
            conn.execute(
                "INSERT INTO tag_names (tag_id, cookbook_id, language, name, name_folded) \
                 VALUES (?1, ?2, ?3, ?4, ?5) \
                 ON CONFLICT(tag_id, language) \
                 DO UPDATE SET name = excluded.name, name_folded = excluded.name_folded",
                params![tag_id, cookbook_id, language, name, folded_word(&name)],
            )
            .map_err(|e| OpError::internal(format!("cannot rename Tag: {e}")))?;
            tag_summary(conn, tag_id, person_id)
        })
    }

    /// Merge two of a Cookbook's Tags into one: every recipe filed under the
    /// merged Tag is filed under the kept one instead, and the merged Tag is
    /// gone. The other half of what GLOSSARY.md says a Tag is — "renaming or
    /// merging one reaches all of them at once".
    ///
    /// The kept Tag keeps its own names. Where it has no name in a Language and
    /// the merged Tag did, that name is adopted rather than thrown away: the
    /// merge is how two words are found to have meant one thing, so the word
    /// the other Language had is worth keeping.
    ///
    /// Like every other act of filing, this mints no Version (ADR 0035).
    pub fn merge_tags(
        &self,
        person_id: &str,
        keep_tag_id: &str,
        merge_tag_id: &str,
    ) -> Result<Value, OpError> {
        if keep_tag_id == merge_tag_id {
            return Err(OpError::bad_request("a Tag cannot be merged into itself"));
        }
        self.db().with_conn(|conn| {
            let keep_cookbook = cookbook_of_tag(conn, keep_tag_id)?;
            let merge_cookbook = cookbook_of_tag(conn, merge_tag_id)?;
            ensure_writes_or_absent(conn, &keep_cookbook, person_id, no_such_tag)?;
            // Both Tags are checked before the two Cookbooks are compared, so
            // that the refusal below is only ever reached by someone who writes
            // both — otherwise naming another household's Tag as the one to
            // merge would tell the caller it exists (ADR 0040).
            ensure_writes_or_absent(conn, &merge_cookbook, person_id, no_such_tag)?;
            // Two Cookbooks' filing systems are separate things, and neither is
            // the other's to fold into (ADR 0007).
            if keep_cookbook != merge_cookbook {
                return Err(OpError::bad_request(
                    "two Tags of different Cookbooks cannot be merged",
                ));
            }

            let carried: Vec<(String, String, String)> = {
                let mut statement = conn
                    .prepare("SELECT language, name, name_folded FROM tag_names WHERE tag_id = ?1")
                    .map_err(|e| OpError::internal(format!("cannot read Tag names: {e}")))?;
                statement
                    .query_map(params![merge_tag_id], |row| {
                        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                    })
                    .map_err(|e| OpError::internal(format!("cannot read Tag names: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot read Tag names: {e}")))?
            };

            // Every recipe under the merged Tag is now under the kept one.
            // OR IGNORE for the recipes already carrying both.
            conn.execute(
                "INSERT OR IGNORE INTO branch_tags (branch_id, tag_id) \
                 SELECT branch_id, ?1 FROM branch_tags WHERE tag_id = ?2",
                params![keep_tag_id, merge_tag_id],
            )
            .map_err(|e| OpError::internal(format!("cannot carry recipes across: {e}")))?;

            // The merged Tag's own rows go before the kept Tag adopts any of
            // its words: one word is unique per Cookbook and Language, so the
            // two Tags may not hold the same word even for an instant.
            for statement in [
                "DELETE FROM branch_tags WHERE tag_id = ?1",
                "DELETE FROM tag_names WHERE tag_id = ?1",
                "DELETE FROM tags WHERE id = ?1",
            ] {
                conn.execute(statement, params![merge_tag_id])
                    .map_err(|e| OpError::internal(format!("cannot merge Tag: {e}")))?;
            }

            for (language, name, folded) in carried {
                conn.execute(
                    "INSERT OR IGNORE INTO tag_names \
                     (tag_id, cookbook_id, language, name, name_folded) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![keep_tag_id, keep_cookbook, language, name, folded],
                )
                .map_err(|e| OpError::internal(format!("cannot adopt Tag name: {e}")))?;
            }

            tag_summary(conn, keep_tag_id, person_id)
        })
    }

    /// Take a Tag out of a Cookbook's list, and off every recipe carrying it.
    /// No recipe changes: a Version records what was written, never how it was
    /// filed (ADR 0035).
    pub fn delete_tag(&self, person_id: &str, tag_id: &str) -> Result<(), OpError> {
        self.db().with_conn(|conn| {
            let cookbook_id = cookbook_of_tag(conn, tag_id)?;
            ensure_writes_or_absent(conn, &cookbook_id, person_id, no_such_tag)?;
            for statement in [
                "DELETE FROM branch_tags WHERE tag_id = ?1",
                "DELETE FROM tag_names WHERE tag_id = ?1",
                "DELETE FROM tags WHERE id = ?1",
            ] {
                conn.execute(statement, params![tag_id])
                    .map_err(|e| OpError::internal(format!("cannot delete Tag: {e}")))?;
            }
            Ok(())
        })
    }

    /// File a recipe under a Tag, or take it back out — `carried` says which.
    /// Both are ordinary filing: neither mints a Version, neither appears in
    /// the Thread, and the recipe's fingerprint is untouched (ADR 0035). Only
    /// the recipe's Co-authors file it (#131, question 3): a Kitchen-mate does
    /// not put your recipe under a word of theirs.
    pub fn set_recipe_tag(
        &self,
        person_id: &str,
        branch_id: &str,
        tag_id: &str,
        carried: bool,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let branch_cookbook = branch_cookbook(conn, branch_id)?;
            ensure_writes(conn, &branch_cookbook, person_id)?;

            // A Tag belongs to one Cookbook, so a recipe can only be filed
            // under its own Cookbook's words. Reaching across is a request for
            // a Tag this Cookbook does not have.
            let tag_cookbook = cookbook_of_tag(conn, tag_id)?;
            if tag_cookbook != branch_cookbook {
                // The sentence below says this Tag exists on some other shelf,
                // which is a fact worth having only if that shelf is one the
                // caller may see. To anyone else the Tag is not here at all
                // (ADR 0040).
                ensure_sees_or_absent(conn, &tag_cookbook, person_id, no_such_tag)?;
                return Err(OpError::not_found(
                    "no such Tag in the Cookbook holding this recipe",
                ));
            }

            if carried {
                conn.execute(
                    "INSERT OR IGNORE INTO branch_tags (branch_id, tag_id) VALUES (?1, ?2)",
                    params![branch_id, tag_id],
                )
                .map_err(|e| OpError::internal(format!("cannot file recipe under Tag: {e}")))?;
            } else {
                conn.execute(
                    "DELETE FROM branch_tags WHERE branch_id = ?1 AND tag_id = ?2",
                    params![branch_id, tag_id],
                )
                .map_err(|e| OpError::internal(format!("cannot take recipe off Tag: {e}")))?;
            }
            Ok(json!({ "tags": tags_of_branch(conn, branch_id, person_id)? }))
        })
    }
}

/// A Tag id that names nothing here — and, by ADR 0040, a Tag of a Cookbook
/// the caller may not see.
pub(super) fn no_such_tag() -> OpError {
    OpError::not_found("no such Tag")
}

/// Which Cookbook a Tag belongs to — and, by failing, that it exists at all.
pub(super) fn cookbook_of_tag(
    conn: &rusqlite::Connection,
    tag_id: &str,
) -> Result<String, OpError> {
    conn.query_row(
        "SELECT cookbook_id FROM tags WHERE id = ?1",
        params![tag_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read Tag: {e}")))?
    .ok_or_else(no_such_tag)
}

/// The Tag a Cookbook already files under this word in this Language, if any.
/// Compared on the fold, which is what the schema holds unique.
pub(super) fn tag_id_for_word(
    conn: &rusqlite::Connection,
    cookbook_id: &str,
    language: &str,
    name: &str,
) -> Result<Option<String>, OpError> {
    conn.query_row(
        "SELECT tag_id FROM tag_names \
           WHERE cookbook_id = ?1 AND language = ?2 AND name_folded = ?3",
        params![cookbook_id, language, folded_word(name)],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot look up Tag: {e}")))
}

/// The Tag a recipe arriving from elsewhere is filed under, in the receiving
/// Cookbook's own list (GLOSSARY.md, "Tag"): a word this Cookbook already files
/// by, in that Language, is that Tag; otherwise one is made, named in every
/// Language it arrived in. `None` when there is no name to file by. A Bundle's
/// tags, a Crouton library's (#128), and a Branch copied from another
/// Cookbook's all land through here.
pub(super) fn arriving_tag(
    conn: &rusqlite::Connection,
    cookbook_id: &str,
    names: &[(&str, &str)],
) -> Result<Option<String>, OpError> {
    for (language, name) in names {
        if let Some(found) = tag_id_for_word(conn, cookbook_id, language, name)? {
            return Ok(Some(found));
        }
    }
    if names.is_empty() {
        return Ok(None);
    }
    let tag_id = format!("t_{}", hex::encode(random_bytes(8)));
    conn.execute(
        "INSERT INTO tags (id, cookbook_id) VALUES (?1, ?2)",
        params![tag_id, cookbook_id],
    )
    .map_err(|e| OpError::internal(format!("cannot create Tag: {e}")))?;
    for (language, name) in names {
        conn.execute(
            "INSERT OR IGNORE INTO tag_names (tag_id, cookbook_id, language, name, name_folded) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![tag_id, cookbook_id, language, name, folded_word(name)],
        )
        .map_err(|e| OpError::internal(format!("cannot name Tag: {e}")))?;
    }
    Ok(Some(tag_id))
}

/// File a Branch under a Tag, a no-op when it already is. Filing only ever
/// adds here: nothing that arrives from elsewhere takes a Tag away.
pub(super) fn file_branch_under(
    conn: &rusqlite::Connection,
    branch_id: &str,
    tag_id: &str,
) -> Result<(), OpError> {
    conn.execute(
        "INSERT OR IGNORE INTO branch_tags (branch_id, tag_id) VALUES (?1, ?2)",
        params![branch_id, tag_id],
    )
    .map_err(|e| OpError::internal(format!("cannot file recipe under Tag: {e}")))?;
    Ok(())
}

/// One Tag as a reader sees it: every name it has, and the one to show them —
/// their Reading Language where the Tag has a name there, and otherwise
/// whatever name it does have (#51). `language` says which of the two happened,
/// so a screen can mark a word standing in from another Language rather than
/// having to guess.
fn tag_summary(
    conn: &rusqlite::Connection,
    tag_id: &str,
    viewer_person_id: &str,
) -> Result<Value, OpError> {
    let cookbook_id = cookbook_of_tag(conn, tag_id)?;
    let mut statement = conn
        .prepare("SELECT language, name FROM tag_names WHERE tag_id = ?1")
        .map_err(|e| OpError::internal(format!("cannot read Tag names: {e}")))?;
    let named: Vec<(String, String)> = statement
        .query_map(params![tag_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| OpError::internal(format!("cannot read Tag names: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read Tag names: {e}")))?;

    let reading_language = reading_language_of(conn, viewer_person_id)?;
    let shown = shown_name(&named, &reading_language);
    let names = names_json(&named);

    Ok(json!({
        "id": tag_id,
        "cookbook_id": cookbook_id,
        "name": shown.map(|(_, name)| name.as_str()),
        "language": shown.map(|(language, _)| language.as_str()),
        "names": names,
        "recipes": recipes_with_tag(conn, tag_id)?,
        // Whether the word above is NOT in this reader's Reading Language
        // (#104). Said here rather than worked out by a screen, for the reason
        // a shelf entry's `language_fallback` is: the Reading Language lives on
        // the account, so a screen comparing against the *interface* locale
        // marks every tag wrongly for anybody whose two settings differ. Only
        // the Core knows which name it just chose and why.
        "language_fallback": shown.is_some_and(|(language, _)| language != &reading_language),
    }))
}

/// How many recipes carry a Tag — **distinct Lineages, not Branches** (#104).
///
/// Counted that way because it is a number the screen sets beside the shelf's
/// own. A Tag says *9 recipes* in Settings and the shelf says *9 tagged batch
/// cook*, and the shelf is one entry per Lineage (ADR 0027): counting rows of
/// `branch_tags` would say ten the moment somebody tagged both a recipe and its
/// Translation, which is one recipe on every screen that shows it.
///
/// No permission question here. A Tag belongs to one Cookbook and so does
/// every Branch that can carry it, so a caller who may see the Tag at all may
/// see everything counted.
fn recipes_with_tag(conn: &rusqlite::Connection, tag_id: &str) -> Result<i64, OpError> {
    conn.query_row(
        "SELECT COUNT(DISTINCT branches.lineage_id) FROM branch_tags \
         JOIN branches ON branches.id = branch_tags.branch_id \
         WHERE branch_tags.tag_id = ?1",
        params![tag_id],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot count a Tag's recipes: {e}")))
}

/// The Tags one recipe carries, as its reader sees them.
pub(super) fn tags_of_branch(
    conn: &rusqlite::Connection,
    branch_id: &str,
    viewer_person_id: &str,
) -> Result<Vec<Value>, OpError> {
    let ids: Vec<String> = {
        let mut statement = conn
            .prepare(
                "SELECT branch_tags.tag_id FROM branch_tags \
                 JOIN tags ON tags.id = branch_tags.tag_id \
                 WHERE branch_tags.branch_id = ?1 ORDER BY tags.created_at, tags.id",
            )
            .map_err(|e| OpError::internal(format!("cannot read a recipe's Tags: {e}")))?;
        statement
            .query_map(params![branch_id], |row| row.get(0))
            .map_err(|e| OpError::internal(format!("cannot read a recipe's Tags: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot read a recipe's Tags: {e}")))?
    };
    ids.iter()
        .map(|id| tag_summary(conn, id, viewer_person_id))
        .collect()
}
