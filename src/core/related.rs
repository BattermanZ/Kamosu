//! Related Recipes: the link between two Lineages, and reading a recipe's
//! links back as shelf entries (#105).

use super::*;

/// How `set_related_recipe`'s far end was named (#105).
///
/// A Related Recipe link is between two Lineages, so either name resolves to
/// the same row. A Branch is the ordinary way to say it, because that is what
/// a screen and an agent are both holding. A Lineage is the way to say it
/// about a recipe that has since been deleted, which has no Branch left to
/// name and would otherwise leave a link nobody could ever break.
pub enum RelatedSide<'a> {
    Branch(&'a str),
    Lineage(&'a str),
}

impl Core {
    /// Relate two Recipes held on this Kitchen's shelf, or take that one
    /// shelf-local relation back off. The stored pair is canonically ordered,
    /// so asking from either end changes the same untyped, two-way link (#52).
    ///
    /// **The other side may be named by its Lineage as well as by its Branch**
    /// (#105), which is what makes a link breakable after the recipe at the far
    /// end is deleted. A deleted Branch leaves its Lineage behind, and the link
    /// was always stored between two Lineages, so naming one names the thing
    /// the link is actually made of. Relating still requires a Recipe this
    /// Kitchen holds: a Lineage with no Branch here can only be unlinked, never
    /// linked, because there is no recipe to point at.
    pub fn set_related_recipe(
        &self,
        person_id: &str,
        branch_id: &str,
        other: RelatedSide<'_>,
        related: bool,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let no_such_recipe = || OpError::not_found("no such Recipe on this shelf");
            let no_such_related = || OpError::not_found("no such related Recipe on this shelf");

            let (kitchen_id, lineage_id, title) =
                recipe_shelf_identity(conn, branch_id)?.ok_or_else(no_such_recipe)?;
            ensure_member_or_absent(conn, &kitchen_id, person_id, no_such_recipe)?;

            // The far end's Lineage, and its title where this Kitchen still
            // holds it. A title is only ever written on the way in, so the one
            // case with no title to offer is the one case that cannot insert.
            let (related_lineage_id, related_title) = match other {
                RelatedSide::Branch(related_branch_id) => {
                    let (related_kitchen_id, related_lineage_id, related_title) =
                        recipe_shelf_identity(conn, related_branch_id)?
                            .ok_or_else(no_such_related)?;
                    if related_kitchen_id != kitchen_id {
                        // The sentence below says the recipe is on some shelf but not
                        // this one, which is a fact about another Kitchen's shelf. A
                        // caller who cooks in that Kitchen too may have it — they knew
                        // already. Anyone else gets the plain not-found (ADR 0040).
                        ensure_member_or_absent(
                            conn,
                            &related_kitchen_id,
                            person_id,
                            no_such_related,
                        )?;
                        return Err(OpError::not_found(
                            "no such related Recipe on the Kitchen's shelf",
                        ));
                    }
                    (related_lineage_id, Some(related_title))
                }
                RelatedSide::Lineage(related_lineage_id) => {
                    // Nothing here reads another Kitchen's shelf: the row this
                    // touches is keyed on the caller's own `kitchen_id`, so a
                    // Lineage id from anywhere can only ever reach their links.
                    //
                    // The name stored is the one the shelf would show this
                    // reader, which is the same one `get_recipe` will answer
                    // with, so the two cannot disagree about what a link is
                    // called.
                    let reading_language = reading_language_of(conn, person_id)?;
                    let on_shelf = related_branch_on_shelf(
                        conn,
                        &kitchen_id,
                        related_lineage_id,
                        &reading_language,
                    )?;
                    match on_shelf {
                        Some(branch) => (related_lineage_id.to_string(), Some(branch.title)),
                        None if !related => (related_lineage_id.to_string(), None),
                        None => return Err(no_such_related()),
                    }
                }
            };
            if lineage_id == related_lineage_id {
                return Err(OpError::bad_request("a Recipe cannot be Related to itself"));
            }
            // Relating needs both names, since the row remembers them precisely
            // so a line stays readable when either recipe later leaves.
            let related_title = match (related, related_title) {
                (true, None) => return Err(no_such_related()),
                (_, title) => title.unwrap_or_default(),
            };

            let (lineage_a_id, lineage_a_name, lineage_b_id, lineage_b_name) =
                if lineage_id < related_lineage_id {
                    (lineage_id.as_str(), title.as_str(), related_lineage_id.as_str(), related_title.as_str())
                } else {
                    (related_lineage_id.as_str(), related_title.as_str(), lineage_id.as_str(), title.as_str())
                };
            if related {
                conn.execute(
                    "INSERT OR IGNORE INTO related_recipes \
                     (kitchen_id, lineage_a_id, lineage_b_id, lineage_a_name, lineage_b_name) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![kitchen_id, lineage_a_id, lineage_b_id, lineage_a_name, lineage_b_name],
                )
                .map_err(|e| OpError::internal(format!("cannot relate Recipes: {e}")))?;
            } else {
                conn.execute(
                    "DELETE FROM related_recipes WHERE kitchen_id = ?1 AND lineage_a_id = ?2 AND lineage_b_id = ?3",
                    params![kitchen_id, lineage_a_id, lineage_b_id],
                )
                .map_err(|e| OpError::internal(format!("cannot remove Related Recipe: {e}")))?;
            }
            Ok(json!({
                "related_recipes": related_recipes_of_lineage(conn, &kitchen_id, &lineage_id, person_id)?
            }))
        })
    }
}

/// The Kitchen, Lineage, and current title of one Branch. These are the three
/// facts a shelf link needs from either end, so the two lookups in
/// `set_related_recipe` cannot drift apart.
fn recipe_shelf_identity(
    conn: &rusqlite::Connection,
    branch_id: &str,
) -> Result<Option<(String, String, String)>, OpError> {
    conn.query_row(
        "SELECT branches.kitchen_id, branches.lineage_id, json_extract(versions.content, '$.title') \
         FROM branches JOIN versions ON versions.id = branches.head_version_id WHERE branches.id = ?1",
        params![branch_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read Recipe: {e}")))
}

/// The Branch a Related Recipe names: its id, its title, its Main Photo where
/// it has one, and the Language it is written in.
struct RelatedBranch {
    branch_id: String,
    title: String,
    main_photo: Option<String>,
    language: String,
}

/// The Branch a Related Recipe names, on one Kitchen's shelf, or `None` where
/// that Kitchen holds no Branch of the Lineage any more.
///
/// **Which Branch is `shelf_of`'s rule**, written here as an ordering rather
/// than a second copy of its reasoning: the one in the reader's own Language,
/// and failing that the oldest, which the caller then marks as a fallback. A
/// Language preference may never hide a recipe from its owner (ADR 0006).
///
/// One function because there are two questions with one answer — whether a
/// link still points at something, and whether something can be pointed at —
/// and when they were two functions they promptly disagreed about Language.
fn related_branch_on_shelf(
    conn: &rusqlite::Connection,
    kitchen_id: &str,
    lineage_id: &str,
    reading_language: &str,
) -> Result<Option<RelatedBranch>, OpError> {
    conn.query_row(
        "SELECT branches.id, json_extract(versions.content, '$.title'), \
                json_extract(versions.content, '$.main_photo'), branches.language \
         FROM branches JOIN versions ON versions.id = branches.head_version_id \
         WHERE branches.kitchen_id = ?1 AND branches.lineage_id = ?2 \
         ORDER BY branches.language = ?3 DESC, branches.created_at, branches.id \
         LIMIT 1",
        params![kitchen_id, lineage_id, reading_language],
        |row| {
            Ok(RelatedBranch {
                branch_id: row.get(0)?,
                title: row.get(1)?,
                main_photo: row.get(2)?,
                language: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read related Recipe: {e}")))
}

/// The Related Recipes one Lineage shows on one Kitchen's shelf. A Lineage
/// that is no longer held there is deliberately returned with its remembered
/// title and no Branch id: text is better than a broken pointer (#52).
pub(super) fn related_recipes_of_lineage(
    conn: &rusqlite::Connection,
    kitchen_id: &str,
    lineage_id: &str,
    person_id: &str,
) -> Result<Vec<Value>, OpError> {
    // Which Branch a related recipe names is the shelf's question, answered the
    // shelf's way: the one written in the reader's own Language, and where the
    // Lineage has none, the oldest, marked as a fallback. A preference may never
    // hide a recipe from its owner (ADR 0006), and #105 is what first put these
    // on a screen where the difference is visible.
    let reading_language = reading_language_of(conn, person_id)?;
    let links: Vec<(String, String)> = {
        let mut statement = conn
            .prepare(
                "SELECT CASE WHEN lineage_a_id = ?2 THEN lineage_b_id ELSE lineage_a_id END, \
                        CASE WHEN lineage_a_id = ?2 THEN lineage_b_name ELSE lineage_a_name END \
                 FROM related_recipes \
                 WHERE kitchen_id = ?1 AND (lineage_a_id = ?2 OR lineage_b_id = ?2) \
                 ORDER BY created_at, lineage_a_id, lineage_b_id",
            )
            .map_err(|e| OpError::internal(format!("cannot read Related Recipes: {e}")))?;
        statement
            .query_map(params![kitchen_id, lineage_id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|e| OpError::internal(format!("cannot read Related Recipes: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot read Related Recipes: {e}")))?
    };

    links
        .into_iter()
        .map(|(related_lineage_id, remembered_title)| {
            let present =
                related_branch_on_shelf(conn, kitchen_id, &related_lineage_id, &reading_language)?;
            // A Lineage that has left the shelf keeps its remembered name and
            // gives up everything that pointed at a recipe: no Branch to open,
            // no picture, and no Language to mark, because there is no recipe
            // here to have any of them.
            let (branch_id, title, main_photo, language) = match present {
                Some(branch) => (
                    Some(branch.branch_id),
                    branch.title,
                    branch.main_photo,
                    Some(branch.language),
                ),
                None => (None, remembered_title, None, None),
            };
            Ok(json!({
                "lineage_id": related_lineage_id,
                "branch_id": branch_id,
                "title": title,
                "main_photo": main_photo,
                "language": language,
                // The mark, and the whole of the mark, on `shelf_entry`'s rule:
                // an Unknown recipe is honestly more than one Language, so it
                // is not in any Language the reader failed to ask for and is
                // never marked (ADR 0006).
                "language_fallback": language.as_deref().is_some_and(|language| {
                    language != reading_language && !crate::language::is_stated_unknown(language)
                }),
            }))
        })
        .collect()
}
