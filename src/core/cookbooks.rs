//! Cookbooks: where a recipe belongs, the circle that may change it, and the
//! two checks every Operation on a recipe asks (ADR 0041, #131).
//!
//! **May see** is a Co-author of the Cookbook, or anyone who cooks in a
//! Kitchen with one of them. The `visible_cookbooks` view says it, once.
//! **May change** is a Co-author. Everything that decides who reads or writes
//! a recipe comes through here, which is ADR 0040's rule kept in one place: a
//! Branch the caller may not see is answered exactly as one that is not here.

use super::*;

impl Core {
    /// The caller's own Cookbook, as its settings card shows it.
    pub fn get_cookbook(&self, person_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let cookbook_id = cookbook_of_person(conn, person_id)?;
            cookbook_summary(conn, &cookbook_id, Some(person_id))
        })
    }

    /// Give the caller's Cookbook a name of its own, or clear it back to its
    /// Co-authors' names. Any Co-author may (ADR 0041).
    pub fn rename_cookbook(&self, person_id: &str, name: Option<&str>) -> Result<Value, OpError> {
        let name = name.map(str::trim).filter(|name| !name.is_empty());
        self.db().with_conn(|conn| {
            let cookbook_id = cookbook_of_person(conn, person_id)?;
            conn.execute(
                "UPDATE cookbooks SET name = ?1 WHERE id = ?2",
                params![name, cookbook_id],
            )
            .map_err(|e| OpError::internal(format!("cannot rename Cookbook: {e}")))?;
            cookbook_summary(conn, &cookbook_id, Some(person_id))
        })
    }

    /// Mint a one-use link to write the caller's Cookbook with somebody (#131,
    /// screen choice 2). The raw Secret is returned once and kept only hashed.
    pub fn invite_to_cookbook(&self, person_id: &str) -> Result<(String, String), OpError> {
        let secret = generate_secret();
        let id = format!("ci_{}", hex::encode(random_bytes(8)));
        self.db().with_conn(|conn| {
            let cookbook_id = cookbook_of_person(conn, person_id)?;
            conn.execute(
                "INSERT INTO cookbook_invites (id, secret_hash, cookbook_id, created_by) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![id, hash_secret(&secret), cookbook_id, person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot mint Cookbook Invite: {e}")))?;
            Ok(())
        })?;
        Ok((id, secret))
    }

    /// End a Cookbook Invite nobody has used yet. Any Co-author may end one of
    /// their Cookbook's; anybody else is answered as though it were not here.
    pub fn cancel_cookbook_invite(&self, person_id: &str, invite_id: &str) -> Result<(), OpError> {
        self.db().with_conn(|conn| {
            let cookbook_id = cookbook_of_person(conn, person_id)?;
            let ended = conn
                .execute(
                    "UPDATE cookbook_invites SET ended_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
                      WHERE id = ?1 AND cookbook_id = ?2 AND used_at IS NULL AND ended_at IS NULL",
                    params![invite_id, cookbook_id],
                )
                .map_err(|e| OpError::internal(format!("cannot end Cookbook Invite: {e}")))?;
            if ended == 0 {
                return Err(no_such_cookbook_invite());
            }
            Ok(())
        })
    }

    /// What opening a Cookbook Invite would do, before anybody says yes: whose
    /// it is, how many recipes on each side become one Cookbook, and who else
    /// will be asked first (#135).
    ///
    /// The one who accepted it can read it again while it waits, and is told
    /// who has still to answer. To anybody else a waiting Invite is spent.
    pub fn read_cookbook_invite(&self, person_id: &str, secret: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let own = cookbook_of_person(conn, person_id)?;
            let (cookbook_id, waiting) = match waiting_join_accepted_with(conn, secret, person_id)?
            {
                Some(join) => (join.into.clone(), Some(join)),
                None => (live_cookbook_invite(conn, secret)?.1, None),
            };
            let sender = invite_sender(conn, secret)?;
            let asks = match &waiting {
                Some(join) => still_to_answer(conn, join)?,
                None if own == cookbook_id => Vec::new(),
                None => people_to_ask(conn, &own, &cookbook_id, person_id, &sender)?,
            };
            Ok(json!({
                "cookbook": cookbook_summary(conn, &cookbook_id, None)?,
                "invited_by": person_named(conn, &sender)?,
                "their_recipes": recipe_count(conn, &cookbook_id)?,
                "your_recipes": recipe_count(conn, &own)?,
                "together_recipes": together_count(conn, &own, &cookbook_id)?,
                "already_yours": own == cookbook_id,
                "asks": people_named(conn, &asks)?,
                "waiting": waiting.is_some(),
            }))
        })
    }

    /// Open a Cookbook Invite: the caller's Cookbook joins the one it names,
    /// and from then on either may change any recipe in it (ADR 0041).
    ///
    /// Only once nobody else is left to ask (#135, choice C). Where either
    /// Cookbook has other writers, accepting is the caller's yes and the
    /// Invite's sending was the sender's, and the join waits for everyone
    /// else writing either one. The answer is then the caller's own Cookbook,
    /// saying who it waits for.
    ///
    /// Everything the caller's Cookbook held moves across: its Branches, its
    /// Tags, its Related Recipes and its import ledger. Two Branches of one
    /// recipe that meet here are settled as question 6 on #131 says: an
    /// unnamed one the joiner brings is named after the joiner wherever the
    /// Cookbook already holds one of its own, named or not (#136), and the same
    /// friend's Branch received on both sides is folded into the longer where
    /// one history holds the other, and otherwise kept under a new Travelling
    /// id.
    pub fn accept_cookbook_invite(&self, person_id: &str, secret: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let transaction = conn
                .unchecked_transaction()
                .map_err(|e| OpError::internal(format!("cannot begin: {e}")))?;
            let (invite_id, into) = live_cookbook_invite(conn, secret)?;
            let from = cookbook_of_person(conn, person_id)?;
            if from != into {
                ensure_free_to_join(conn, &from, &into)?;
            }
            conn.execute(
                "UPDATE cookbook_invites SET used_at = strftime('%Y-%m-%dT%H:%M:%fZ','now'), used_by = ?1 \
                  WHERE id = ?2",
                params![person_id, invite_id],
            )
            .map_err(|e| OpError::internal(format!("cannot spend Cookbook Invite: {e}")))?;
            if from != into {
                let join_id = format!("cj_{}", hex::encode(random_bytes(8)));
                conn.execute(
                    "INSERT INTO cookbook_joins (id, invite_id, from_cookbook, into_cookbook, accepted_by) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![join_id, invite_id, from, into, person_id],
                )
                .map_err(|e| OpError::internal(format!("cannot begin the join: {e}")))?;
                settle_join(conn, &join_id)?;
            }
            let own = cookbook_of_person(conn, person_id)?;
            let summary = cookbook_summary(conn, &own, Some(person_id))?;
            transaction
                .commit()
                .map_err(|e| OpError::internal(format!("cannot commit: {e}")))?;
            Ok(summary)
        })
    }

    /// Answer a join waiting on the caller (#135). A yes counts towards it,
    /// and the last yes it waited for joins the two Cookbooks there and then.
    /// A no from anybody writing either Cookbook calls it off and opens its
    /// Invite again; from the one who accepted it, that is taking it back.
    ///
    /// A join the caller's Cookbook is not part of is answered as though it
    /// were not here (ADR 0040).
    pub fn answer_cookbook_join(
        &self,
        person_id: &str,
        join_id: &str,
        yes: bool,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let transaction = conn
                .unchecked_transaction()
                .map_err(|e| OpError::internal(format!("cannot begin: {e}")))?;
            let own = cookbook_of_person(conn, person_id)?;
            let join = waiting_join(conn, join_id)?
                .filter(|join| join.from == own || join.into == own)
                .ok_or_else(|| OpError::not_found("no join is waiting on that answer"))?;
            if yes {
                conn.execute(
                    "INSERT OR IGNORE INTO cookbook_join_answers (join_id, person_id) VALUES (?1, ?2)",
                    params![join.id, person_id],
                )
                .map_err(|e| OpError::internal(format!("cannot record the answer: {e}")))?;
                settle_join(conn, &join.id)?;
            } else {
                end_join(conn, &join, Some(person_id))?;
            }
            let own = cookbook_of_person(conn, person_id)?;
            let summary = cookbook_summary(conn, &own, Some(person_id))?;
            transaction
                .commit()
                .map_err(|e| OpError::internal(format!("cannot commit: {e}")))?;
            Ok(summary)
        })
    }

    /// Leave the Cookbook the caller writes with others, taking a Branch of
    /// every recipe in it (ADR 0041). Leaving is what separating is.
    pub fn leave_cookbook(&self, person_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let transaction = conn
                .unchecked_transaction()
                .map_err(|e| OpError::internal(format!("cannot begin: {e}")))?;
            let cookbook_id = cookbook_of_person(conn, person_id)?;
            if authors_of(conn, &cookbook_id)?.len() < 2 {
                return Err(OpError::bad_request(
                    "you write this Cookbook alone, so there is nobody to separate from",
                ));
            }
            let own = separate(conn, &cookbook_id, person_id)?;
            let summary = cookbook_summary(conn, &own, Some(person_id))?;
            transaction
                .commit()
                .map_err(|e| OpError::internal(format!("cannot commit: {e}")))?;
            Ok(summary)
        })
    }

    /// Separate another Co-author from the caller's Cookbook. They leave with
    /// a Branch of every recipe in it, exactly as though they had left.
    pub fn remove_cookbook_author(
        &self,
        person_id: &str,
        target_person_id: &str,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let transaction = conn
                .unchecked_transaction()
                .map_err(|e| OpError::internal(format!("cannot begin: {e}")))?;
            let cookbook_id = cookbook_of_person(conn, person_id)?;
            if target_person_id == person_id {
                return Err(OpError::bad_request(
                    "to separate yourself from this Cookbook, leave it",
                ));
            }
            if !authors_of(conn, &cookbook_id)?
                .iter()
                .any(|author| author == target_person_id)
            {
                return Err(OpError::not_found(
                    "that Person does not write this Cookbook",
                ));
            }
            separate(conn, &cookbook_id, target_person_id)?;
            // Asked afresh: the removal may have let a waiting join go ahead
            // (#135), and then the caller writes a different Cookbook.
            let own = cookbook_of_person(conn, person_id)?;
            let summary = cookbook_summary(conn, &own, Some(person_id))?;
            transaction
                .commit()
                .map_err(|e| OpError::internal(format!("cannot commit: {e}")))?;
            Ok(summary)
        })
    }

    /// Start a Branch of a recipe, unchanged, in the caller's own Cookbook,
    /// under a name they give it (ADR 0041): "Vegetarian" beside the one they
    /// already have. The name is required, because nothing else tells the two
    /// apart.
    pub fn start_variation(
        &self,
        person_id: &str,
        branch_id: &str,
        name: &str,
    ) -> Result<Value, OpError> {
        let name = required_text(name, "name")?.to_string();
        let new_branch_id = self.db().with_conn(|conn| {
            ensure_sees_branch(conn, branch_id, person_id)?;
            let own = cookbook_of_person(conn, person_id)?;
            // Only your own recipe is branched on purpose (#131, screen choice
            // 3; ADR 0041: merely reading a recipe never starts one). Anybody
            // else's, or one that arrived, becomes yours by saving onto it.
            if !cookbook_writes_branch(conn, &own, branch_id)? {
                return Err(OpError::unauthorized(
                    "only the people who write this recipe's Cookbook may start a variation of \
                     it; saving a change to it starts your own copy instead",
                ));
            }
            copy_branch_into(conn, branch_id, &own, person_id, Some(&name))
        })?;
        self.get_recipe(person_id, &new_branch_id, None)
    }

    /// Name a Branch of the caller's Cookbook, or clear its name. A Cookbook
    /// keeps one unnamed Branch of a recipe in each Language, so clearing the
    /// name of a second one is refused in words.
    pub fn rename_branch(
        &self,
        person_id: &str,
        branch_id: &str,
        name: Option<&str>,
    ) -> Result<Value, OpError> {
        let name = name.map(str::trim).filter(|name| !name.is_empty());
        self.db().with_conn(|conn| {
            let cookbook_id = branch_cookbook(conn, branch_id)?;
            ensure_writes(conn, &cookbook_id, person_id)?;
            if name.is_none() {
                let (lineage_id, language): (String, String) = conn
                    .query_row(
                        "SELECT lineage_id, language FROM branches WHERE id = ?1",
                        params![branch_id],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?;
                if unnamed_branch_in(conn, &cookbook_id, &lineage_id, &language, Some(branch_id))?
                    .is_some()
                {
                    return Err(OpError::bad_request(
                        "another version of this recipe in your Cookbook already goes without a \
                         name, so this one needs one",
                    ));
                }
            }
            conn.execute(
                "UPDATE branches SET name = ?1 WHERE id = ?2",
                params![name, branch_id],
            )
            .map_err(|e| OpError::internal(format!("cannot name Branch: {e}")))?;
            Ok(json!({ "branch_id": branch_id, "name": name }))
        })
    }
}

// ── The two checks ───────────────────────────────────────────────────────────

/// The one Cookbook a Person writes in. Every Person has exactly one, which
/// `cookbook_authors_one_each` holds.
pub(super) fn cookbook_of_person(conn: &Connection, person_id: &str) -> Result<String, OpError> {
    conn.query_row(
        "SELECT cookbook_id FROM cookbook_authors WHERE person_id = ?1",
        params![person_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read Cookbook: {e}")))?
    .ok_or_else(|| OpError::internal("this Person writes in no Cookbook"))
}

/// The Cookbook holding a Branch, or the refusal an id naming nothing gets.
pub(super) fn branch_cookbook(conn: &Connection, branch_id: &str) -> Result<String, OpError> {
    conn.query_row(
        "SELECT cookbook_id FROM branches WHERE id = ?1",
        params![branch_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?
    .ok_or_else(no_such_branch)
}

/// Whether a Person may see what a Cookbook holds.
pub(super) fn sees_cookbook(
    conn: &Connection,
    cookbook_id: &str,
    person_id: &str,
) -> Result<bool, OpError> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM visible_cookbooks WHERE person_id = ?1 AND cookbook_id = ?2)",
        params![person_id, cookbook_id],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot check who sees this Cookbook: {e}")))
}

/// **May see**, for a Cookbook Kamosu worked out from something the caller
/// named (ADR 0040): refused with `absent`, the refusal the thing's not being
/// here at all would get, so holding an id tells nobody what another
/// household keeps.
pub(super) fn ensure_sees_or_absent(
    conn: &Connection,
    cookbook_id: &str,
    person_id: &str,
    absent: impl FnOnce() -> OpError,
) -> Result<(), OpError> {
    if sees_cookbook(conn, cookbook_id, person_id)? {
        Ok(())
    } else {
        Err(absent())
    }
}

/// **May see** a Branch: the check nearly every recipe Operation opens with.
/// Answers the Branch's Cookbook, which is what the Operation asks next.
pub(super) fn ensure_sees_branch(
    conn: &Connection,
    branch_id: &str,
    person_id: &str,
) -> Result<String, OpError> {
    let cookbook_id = branch_cookbook(conn, branch_id)?;
    ensure_sees_or_absent(conn, &cookbook_id, person_id, no_such_branch)?;
    Ok(cookbook_id)
}

/// Whether a Person writes in a Cookbook.
pub(super) fn writes_in(
    conn: &Connection,
    cookbook_id: &str,
    person_id: &str,
) -> Result<bool, OpError> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM cookbook_authors WHERE cookbook_id = ?1 AND person_id = ?2)",
        params![cookbook_id, person_id],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot check who writes this Cookbook: {e}")))
}

/// **May change**: only a Co-author. Somebody who may not even see the
/// Cookbook is answered as though it were not here (ADR 0040); somebody who
/// sees it and does not write it is told so, since they already know it is
/// there.
pub(super) fn ensure_writes(
    conn: &Connection,
    cookbook_id: &str,
    person_id: &str,
) -> Result<(), OpError> {
    ensure_writes_or_absent(conn, cookbook_id, person_id, no_such_branch)
}

/// [`ensure_writes`], for a Cookbook worked out from something that is not a
/// Branch — a Tag — whose absence reads differently.
pub(super) fn ensure_writes_or_absent(
    conn: &Connection,
    cookbook_id: &str,
    person_id: &str,
    absent: impl FnOnce() -> OpError,
) -> Result<(), OpError> {
    ensure_sees_or_absent(conn, cookbook_id, person_id, absent)?;
    if writes_in(conn, cookbook_id, person_id)? {
        Ok(())
    } else {
        Err(OpError::unauthorized(
            "only the people who write this recipe's Cookbook may change it",
        ))
    }
}

/// Whether a Cookbook writes a Branch: holds it, and did not merely receive
/// it. A Branch that arrived in a Bundle or from a Share Link sits in the
/// Cookbook under its sender's Hand, and the first change to it starts a
/// Branch of the receiver's own (ADR 0020).
pub(super) fn cookbook_writes_branch(
    conn: &Connection,
    cookbook_id: &str,
    branch_id: &str,
) -> Result<bool, OpError> {
    let (holder, arrived): (String, bool) = conn
        .query_row(
            "SELECT cookbook_id, arrived FROM branches WHERE id = ?1",
            params![branch_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?;
    Ok(holder == cookbook_id && !arrived)
}

/// The Hand a Cookbook writes new Branches under.
pub(super) fn cookbook_hand(conn: &Connection, cookbook_id: &str) -> Result<String, OpError> {
    conn.query_row(
        "SELECT hand_id FROM cookbooks WHERE id = ?1",
        params![cookbook_id],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot read Cookbook's Hand: {e}")))
}

/// Everything a Branch the caller may see is read against: the SQL condition
/// that keeps a query to the Cookbooks `person` may see, `person` being the
/// placeholder holding their id.
pub(super) fn visible_to(branches: &str, person: &str) -> String {
    format!(
        "{branches}.cookbook_id IN (SELECT cookbook_id FROM visible_cookbooks WHERE person_id = {person})"
    )
}

/// **The Cookbooks a Kitchen sees**, as a SQL subquery over the parameter
/// holding its id: every Cookbook one of its members writes (ADR 0041). The
/// question a shelf, a Kitchen's words and a Kitchen's card all ask, kept in
/// one place so the three cannot drift apart.
pub(super) fn cookbooks_seen_in(kitchen: &str) -> String {
    format!(
        "SELECT DISTINCT cookbook_authors.cookbook_id FROM kitchen_members \
           JOIN cookbook_authors ON cookbook_authors.person_id = kitchen_members.person_id \
          WHERE kitchen_members.kitchen_id = {kitchen}"
    )
}

/// **Which of a recipe's Branches a reader opens**, as the tail of a SQL
/// `ORDER BY`: the one in their own Cookbook, arrived ones included, before
/// anybody else's; their unnamed one before a variation; then the original,
/// the oldest (ADR 0041). Never the most recently changed (ADR 0027). One
/// place, so the recipe page, a Related Recipe and the diary cannot each
/// open a different Branch of the same recipe; the shelf's `claim` reads the
/// same facts in the same order, with the reader's Language put first.
pub(super) fn own_first(branches: &str, person: &str) -> String {
    format!(
        "{branches}.cookbook_id IN \
             (SELECT cookbook_id FROM cookbook_authors WHERE person_id = {person}) DESC, \
         {branches}.name IS NULL DESC, {branches}.created_at ASC, {branches}.id ASC"
    )
}

/// What a Cookbook is called, as a SQL expression over the column holding its
/// id: its own name, or its Co-authors' names in the order they joined.
pub(super) fn cookbook_name_sql(column: &str) -> String {
    format!(
        "(SELECT COALESCE(cookbooks.name, \
                 (SELECT group_concat(named, ' & ') FROM \
                    (SELECT people.name AS named FROM cookbook_authors \
                       JOIN people ON people.id = cookbook_authors.person_id \
                      WHERE cookbook_authors.cookbook_id = cookbooks.id \
                      ORDER BY cookbook_authors.joined_at, people.name))) \
            FROM cookbooks WHERE cookbooks.id = {column})"
    )
}

/// A Cookbook as a screen needs it to label a recipe: its id, the name its
/// Co-authors gave it (null until they do) and who writes it, so the screen
/// can say "Hélène's" in the reader's own Language.
pub(super) fn cookbook_label(conn: &Connection, cookbook_id: &str) -> Result<Value, OpError> {
    let name: Option<String> = conn
        .query_row(
            "SELECT name FROM cookbooks WHERE id = ?1",
            params![cookbook_id],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot read Cookbook: {e}")))?;
    let mut statement = conn
        .prepare(
            "SELECT people.id, people.name FROM cookbook_authors \
               JOIN people ON people.id = cookbook_authors.person_id \
              WHERE cookbook_authors.cookbook_id = ?1 \
              ORDER BY cookbook_authors.joined_at, people.name",
        )
        .map_err(|e| OpError::internal(format!("cannot list Co-authors: {e}")))?;
    let authors: Vec<Value> = statement
        .query_map(params![cookbook_id], |row| {
            Ok(json!({ "person_id": row.get::<_, String>(0)?, "name": row.get::<_, String>(1)? }))
        })
        .map_err(|e| OpError::internal(format!("cannot list Co-authors: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot list Co-authors: {e}")))?;
    Ok(json!({ "id": cookbook_id, "name": name, "authors": authors }))
}

/// A Cookbook's settings card: its label, how many recipes it holds, which
/// Kitchens see it, and — for one of its own Co-authors — the Invites still
/// waiting and the joins waiting on answers (#135).
fn cookbook_summary(
    conn: &Connection,
    cookbook_id: &str,
    viewer: Option<&str>,
) -> Result<Value, OpError> {
    let mut summary = cookbook_label(conn, cookbook_id)?;
    summary["recipe_count"] = json!(recipe_count(conn, cookbook_id)?);
    let mut statement = conn
        .prepare(
            "SELECT DISTINCT kitchens.id, kitchens.name FROM kitchens \
               JOIN kitchen_members ON kitchen_members.kitchen_id = kitchens.id \
               JOIN cookbook_authors ON cookbook_authors.person_id = kitchen_members.person_id \
              WHERE cookbook_authors.cookbook_id = ?1 ORDER BY kitchens.name, kitchens.id",
        )
        .map_err(|e| OpError::internal(format!("cannot list Kitchens: {e}")))?;
    let kitchens: Vec<Value> = statement
        .query_map(params![cookbook_id], |row| {
            Ok(json!({ "id": row.get::<_, String>(0)?, "name": row.get::<_, String>(1)? }))
        })
        .map_err(|e| OpError::internal(format!("cannot list Kitchens: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot list Kitchens: {e}")))?;
    summary["kitchens"] = json!(kitchens);
    let invites: Vec<Value> = match viewer {
        Some(viewer) if writes_in(conn, cookbook_id, viewer)? => {
            let mut statement = conn
                .prepare(
                    "SELECT id, created_at FROM cookbook_invites \
                      WHERE cookbook_id = ?1 AND used_at IS NULL AND ended_at IS NULL \
                      ORDER BY created_at",
                )
                .map_err(|e| OpError::internal(format!("cannot list Invites: {e}")))?;
            statement
                .query_map(params![cookbook_id], |row| {
                    Ok(json!({ "invite_id": row.get::<_, String>(0)?, "created_at": row.get::<_, String>(1)? }))
                })
                .map_err(|e| OpError::internal(format!("cannot list Invites: {e}")))?
                .collect::<Result<_, _>>()
                .map_err(|e| OpError::internal(format!("cannot list Invites: {e}")))?
        }
        _ => Vec::new(),
    };
    summary["invites"] = json!(invites);
    summary["joins"] = match viewer {
        Some(viewer) if writes_in(conn, cookbook_id, viewer)? => {
            json!(joins_seen_by(conn, cookbook_id, viewer)?)
        }
        _ => json!([]),
    };
    Ok(summary)
}

/// How many recipes a Cookbook holds: its Lineages, so a Translation or a
/// variation is not a second recipe.
fn recipe_count(conn: &Connection, cookbook_id: &str) -> Result<i64, OpError> {
    conn.query_row(
        "SELECT COUNT(DISTINCT lineage_id) FROM branches WHERE cookbook_id = ?1",
        params![cookbook_id],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot count recipes: {e}")))
}

fn authors_of(conn: &Connection, cookbook_id: &str) -> Result<Vec<String>, OpError> {
    let mut statement = conn
        .prepare(
            "SELECT person_id FROM cookbook_authors WHERE cookbook_id = ?1 ORDER BY joined_at, person_id",
        )
        .map_err(|e| OpError::internal(format!("cannot list Co-authors: {e}")))?;
    statement
        .query_map(params![cookbook_id], |row| row.get(0))
        .map_err(|e| OpError::internal(format!("cannot list Co-authors: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot list Co-authors: {e}")))
}

fn no_such_cookbook_invite() -> OpError {
    OpError::unauthorized("this Invite does not name a live Cookbook invitation")
}

/// The Invite a Secret names, while it is still waiting: `(invite, cookbook)`.
fn live_cookbook_invite(conn: &Connection, secret: &str) -> Result<(String, String), OpError> {
    conn.query_row(
        "SELECT id, cookbook_id FROM cookbook_invites \
          WHERE secret_hash = ?1 AND used_at IS NULL AND ended_at IS NULL",
        params![hash_secret(secret)],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read Cookbook Invite: {e}")))?
    .ok_or_else(no_such_cookbook_invite)
}

// ── Joins waiting on answers (#135) ──────────────────────────────────────────

/// A join an accepted Cookbook Invite started: waiting on answers, or ended.
struct Join {
    id: String,
    invite_id: String,
    from: String,
    into: String,
    accepted_by: String,
    invited_by: String,
    refused_by: Option<String>,
}

const JOIN_SQL: &str = "SELECT cookbook_joins.id, invite_id, from_cookbook, into_cookbook, \
        accepted_by, cookbook_invites.created_by, refused_by \
   FROM cookbook_joins JOIN cookbook_invites ON cookbook_invites.id = cookbook_joins.invite_id";

fn join_row(row: &rusqlite::Row) -> rusqlite::Result<Join> {
    Ok(Join {
        id: row.get(0)?,
        invite_id: row.get(1)?,
        from: row.get(2)?,
        into: row.get(3)?,
        accepted_by: row.get(4)?,
        invited_by: row.get(5)?,
        refused_by: row.get(6)?,
    })
}

fn waiting_join(conn: &Connection, join_id: &str) -> Result<Option<Join>, OpError> {
    conn.query_row(
        &format!("{JOIN_SQL} WHERE cookbook_joins.ended_at IS NULL AND cookbook_joins.id = ?1"),
        params![join_id],
        join_row,
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read the waiting join: {e}")))
}

/// The join a Person started by accepting the Invite a Secret names, while
/// it waits.
fn waiting_join_accepted_with(
    conn: &Connection,
    secret: &str,
    person_id: &str,
) -> Result<Option<Join>, OpError> {
    conn.query_row(
        &format!(
            "{JOIN_SQL} WHERE cookbook_joins.ended_at IS NULL \
                          AND cookbook_invites.secret_hash = ?1 \
                          AND cookbook_joins.accepted_by = ?2"
        ),
        params![hash_secret(secret), person_id],
        join_row,
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read the waiting join: {e}")))
}

/// Every join waiting with this Cookbook on either side, oldest first.
fn waiting_joins_of(conn: &Connection, cookbook_id: &str) -> Result<Vec<Join>, OpError> {
    let mut statement = conn
        .prepare(&format!(
            "{JOIN_SQL} WHERE cookbook_joins.ended_at IS NULL \
                          AND (from_cookbook = ?1 OR into_cookbook = ?1) \
             ORDER BY cookbook_joins.created_at, cookbook_joins.id"
        ))
        .map_err(|e| OpError::internal(format!("cannot read waiting joins: {e}")))?;
    statement
        .query_map(params![cookbook_id], join_row)
        .map_err(|e| OpError::internal(format!("cannot read waiting joins: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read waiting joins: {e}")))
}

/// Who sent the Invite a Secret names, used or not.
fn invite_sender(conn: &Connection, secret: &str) -> Result<String, OpError> {
    conn.query_row(
        "SELECT created_by FROM cookbook_invites WHERE secret_hash = ?1",
        params![hash_secret(secret)],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot read Cookbook Invite: {e}")))
}

/// Who joining two Cookbooks has to ask: everyone writing either, less the
/// one accepting and the one who sent the Invite, whose yes those were. An
/// account that is disabled cannot sign in to answer, so it is not asked:
/// otherwise it would hold the join open for good.
fn people_to_ask(
    conn: &Connection,
    from: &str,
    into: &str,
    acceptor: &str,
    sender: &str,
) -> Result<Vec<String>, OpError> {
    let mut statement = conn
        .prepare(
            "SELECT person_id FROM cookbook_authors \
               JOIN people ON people.id = cookbook_authors.person_id \
              WHERE cookbook_id IN (?1, ?2) AND person_id NOT IN (?3, ?4) \
                AND people.disabled = 0 \
              ORDER BY cookbook_id <> ?1, joined_at, person_id",
        )
        .map_err(|e| OpError::internal(format!("cannot list Co-authors: {e}")))?;
    statement
        .query_map(params![from, into, acceptor, sender], |row| row.get(0))
        .map_err(|e| OpError::internal(format!("cannot list Co-authors: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot list Co-authors: {e}")))
}

/// Who a waiting join still waits for, worked out afresh each time, so that
/// somebody who left meanwhile is no longer asked and somebody who joined
/// either Cookbook is.
fn still_to_answer(conn: &Connection, join: &Join) -> Result<Vec<String>, OpError> {
    let answered: Vec<String> = {
        let mut statement = conn
            .prepare("SELECT person_id FROM cookbook_join_answers WHERE join_id = ?1")
            .map_err(|e| OpError::internal(format!("cannot read answers: {e}")))?;
        statement
            .query_map(params![join.id], |row| row.get(0))
            .map_err(|e| OpError::internal(format!("cannot read answers: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot read answers: {e}")))?
    };
    Ok(people_to_ask(
        conn,
        &join.from,
        &join.into,
        &join.accepted_by,
        &join.invited_by,
    )?
    .into_iter()
    .filter(|person| !answered.contains(person))
    .collect())
}

fn person_named(conn: &Connection, person_id: &str) -> Result<Value, OpError> {
    let name: String = conn
        .query_row(
            "SELECT name FROM people WHERE id = ?1",
            params![person_id],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot read Person: {e}")))?;
    Ok(json!({ "person_id": person_id, "name": name }))
}

fn people_named(conn: &Connection, people: &[String]) -> Result<Vec<Value>, OpError> {
    people
        .iter()
        .map(|person| person_named(conn, person))
        .collect()
}

/// How many recipes two Cookbooks hold between them: fewer than their two
/// counts added up wherever both hold a version of the same recipe.
fn together_count(conn: &Connection, a: &str, b: &str) -> Result<i64, OpError> {
    conn.query_row(
        "SELECT COUNT(DISTINCT lineage_id) FROM branches WHERE cookbook_id IN (?1, ?2)",
        params![a, b],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot count recipes: {e}")))
}

/// A Cookbook waits on one join at a time, on either side (#135). Whoever
/// accepts an Invite while either Cookbook waits is refused in words, and
/// may accept once that join is settled.
fn ensure_free_to_join(conn: &Connection, from: &str, into: &str) -> Result<(), OpError> {
    if !waiting_joins_of(conn, from)?.is_empty() {
        return Err(OpError::bad_request(
            "your Cookbook is already waiting on answers to another join; accept this one once \
             that is settled",
        ));
    }
    if !waiting_joins_of(conn, into)?.is_empty() {
        return Err(OpError::bad_request(
            "that Cookbook is already waiting on answers to another join; accept this once that \
             is settled",
        ));
    }
    Ok(())
}

/// Whether a Person can still sign in: neither disabled nor deleted.
fn signs_in(conn: &Connection, person_id: &str) -> Result<bool, OpError> {
    conn.query_row(
        "SELECT disabled = 0 FROM people WHERE id = ?1",
        params![person_id],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot read Person: {e}")))
}

/// Bring a waiting join to where it now stands. It is called off once the
/// one who accepted it can no longer sign in or no longer writes the
/// Cookbook it would bring, or once nobody who can sign in writes the one it
/// would join. It joins once nobody is left to ask, and otherwise waits.
fn settle_join(conn: &Connection, join_id: &str) -> Result<(), OpError> {
    let Some(join) = waiting_join(conn, join_id)? else {
        return Ok(());
    };
    let anyone_left_into = authors_of(conn, &join.into)?
        .iter()
        .map(|person| signs_in(conn, person))
        .collect::<Result<Vec<_>, _>>()?
        .contains(&true);
    if !signs_in(conn, &join.accepted_by)?
        || !writes_in(conn, &join.from, &join.accepted_by)?
        || !anyone_left_into
    {
        return end_join(conn, &join, None);
    }
    if still_to_answer(conn, &join)?.is_empty() {
        join_cookbooks(conn, &join.from, &join.into, &join.accepted_by)?;
    }
    Ok(())
}

/// Settle every join waiting with this Cookbook, after its writers changed.
pub(super) fn settle_joins_of(conn: &Connection, cookbook_id: &str) -> Result<(), OpError> {
    for join in waiting_joins_of(conn, cookbook_id)? {
        settle_join(conn, &join.id)?;
    }
    Ok(())
}

/// Call a waiting join off. Its Invite opens again, so the one who accepted
/// it can open the same link once whatever stood in the way has gone (#135,
/// answer 3). Where the no is the sender's own, the Invite ends instead:
/// sending it was their yes, and they have taken it back.
fn end_join(conn: &Connection, join: &Join, refused_by: Option<&str>) -> Result<(), OpError> {
    conn.execute(
        "UPDATE cookbook_joins SET ended_at = strftime('%Y-%m-%dT%H:%M:%fZ','now'), refused_by = ?2 \
          WHERE id = ?1",
        params![join.id, refused_by],
    )
    .map_err(|e| OpError::internal(format!("cannot call the join off: {e}")))?;
    let invite = if refused_by == Some(join.invited_by.as_str()) {
        "UPDATE cookbook_invites SET ended_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
          WHERE id = ?1 AND ended_at IS NULL"
    } else {
        "UPDATE cookbook_invites SET used_at = NULL, used_by = NULL \
          WHERE id = ?1 AND ended_at IS NULL"
    };
    conn.execute(invite, params![join.invite_id])
        .map_err(|e| OpError::internal(format!("cannot settle the Invite: {e}")))?;
    Ok(())
}

/// The joins a Cookbook's card shows one of its writers: every join it is
/// waiting on, and, to the one who accepted it, a join somebody said no to,
/// for as long as its Invite stays open for them to try again.
fn joins_seen_by(
    conn: &Connection,
    cookbook_id: &str,
    viewer: &str,
) -> Result<Vec<Value>, OpError> {
    let waiting = waiting_joins_of(conn, cookbook_id)?;
    let mut seen = waiting
        .iter()
        .map(|join| join_as_seen(conn, join, viewer))
        .collect::<Result<Vec<_>, _>>()?;
    if waiting.iter().any(|join| join.from == cookbook_id) {
        return Ok(seen);
    }
    let refused = conn
        .query_row(
            &format!(
                "{JOIN_SQL} \
                  WHERE cookbook_joins.from_cookbook = ?1 AND cookbook_joins.accepted_by = ?2 \
                    AND cookbook_joins.refused_by IS NOT NULL AND cookbook_joins.refused_by <> ?2 \
                    AND cookbook_invites.used_at IS NULL AND cookbook_invites.ended_at IS NULL \
                    AND NOT EXISTS (SELECT 1 FROM cookbook_joins AS later \
                                     WHERE later.invite_id = cookbook_joins.invite_id \
                                       AND later.created_at > cookbook_joins.created_at) \
                  ORDER BY cookbook_joins.ended_at DESC LIMIT 1"
            ),
            params![cookbook_id, viewer],
            join_row,
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read refused joins: {e}")))?;
    if let Some(join) = refused {
        seen.push(join_as_seen(conn, &join, viewer)?);
    }
    Ok(seen)
}

/// One join as a writer of either Cookbook sees it: who is in it, whom it
/// still waits for, and their own part in it.
fn join_as_seen(conn: &Connection, join: &Join, viewer: &str) -> Result<Value, OpError> {
    let waiting_on = match join.refused_by {
        None => still_to_answer(conn, join)?,
        Some(_) => Vec::new(),
    };
    let you = if viewer == join.accepted_by {
        "accepted"
    } else if viewer == join.invited_by {
        "invited"
    } else if waiting_on.iter().any(|person| person == viewer) {
        "asked"
    } else {
        "answered"
    };
    Ok(json!({
        "join_id": join.id,
        "state": if join.refused_by.is_some() { "refused" } else { "waiting" },
        "accepted_by": person_named(conn, &join.accepted_by)?,
        "invited_by": person_named(conn, &join.invited_by)?,
        "joining": cookbook_label(conn, &join.from)?,
        "into": cookbook_label(conn, &join.into)?,
        "together_recipes": together_count(conn, &join.from, &join.into)?,
        "waiting_on": people_named(conn, &waiting_on)?,
        "you": you,
        "refused_by": join.refused_by.as_deref().map(|person| person_named(conn, person)).transpose()?,
        "refused_by_co_author": match &join.refused_by {
            Some(person) => writes_in(conn, &join.from, person)?,
            None => false,
        },
    }))
}

/// A Person's own Cookbook, made the moment their account is. It starts with
/// them alone, under a Hand minted for it.
pub(super) fn insert_person_cookbook(
    conn: &Connection,
    person_id: &str,
) -> Result<String, OpError> {
    let cookbook_id = format!("c_{}", hex::encode(random_bytes(8)));
    insert_cookbook(conn, &cookbook_id, &cookbook_id, person_id)?;
    Ok(cookbook_id)
}

fn insert_cookbook(
    conn: &Connection,
    cookbook_id: &str,
    hand_id: &str,
    person_id: &str,
) -> Result<(), OpError> {
    conn.execute(
        "INSERT INTO cookbooks (id, hand_id) VALUES (?1, ?2)",
        params![cookbook_id, hand_id],
    )
    .map_err(|e| OpError::internal(format!("cannot make Cookbook: {e}")))?;
    conn.execute(
        "INSERT INTO cookbook_authors (cookbook_id, person_id) VALUES (?1, ?2)",
        params![cookbook_id, person_id],
    )
    .map_err(|e| OpError::internal(format!("cannot make Cookbook: {e}")))?;
    conn.execute(
        "INSERT INTO cookbook_hands (hand_id, cookbook_id, person_id) VALUES (?1, ?2, ?3) \
         ON CONFLICT(hand_id) DO UPDATE SET cookbook_id = excluded.cookbook_id",
        params![hand_id, cookbook_id, person_id],
    )
    .map_err(|e| OpError::internal(format!("cannot record Cookbook's Hand: {e}")))?;
    Ok(())
}

// ── Branches moving between Cookbooks ────────────────────────────────────────

/// The unnamed Branch a Cookbook holds of a recipe in one Language, if any,
/// leaving `except` out. A Branch that arrived is not counted: it is somebody
/// else's writing, labelled by its sender's Hand, so it never needs a name to
/// be told apart from the Cookbook's own.
pub(super) fn unnamed_branch_in(
    conn: &Connection,
    cookbook_id: &str,
    lineage_id: &str,
    language: &str,
    except: Option<&str>,
) -> Result<Option<String>, OpError> {
    conn.query_row(
        "SELECT id FROM branches \
          WHERE cookbook_id = ?1 AND lineage_id = ?2 AND language = ?3 AND name IS NULL \
            AND arrived = 0 AND (?4 IS NULL OR id <> ?4) \
          ORDER BY created_at, id LIMIT 1",
        params![cookbook_id, lineage_id, language, except],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read Branches: {e}")))
}

/// Whether a Cookbook already holds a Branch of its own of a recipe in one
/// Language, named or not. A Branch that arrived is not counted, as in
/// [`unnamed_branch_in`].
fn holds_own_branch(
    conn: &Connection,
    cookbook_id: &str,
    lineage_id: &str,
    language: &str,
) -> Result<bool, OpError> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM branches \
          WHERE cookbook_id = ?1 AND lineage_id = ?2 AND language = ?3 AND arrived = 0)",
        params![cookbook_id, lineage_id, language],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot read Branches: {e}")))
}

/// The name a Branch arriving in `cookbook_id` needs, if any: none when the
/// Cookbook holds no Branch of its own of that recipe in that Language yet,
/// and otherwise `fallback` — whose it was — so the two can be told apart.
///
/// **Any own Branch, named or not** (#136): the recipe page compares every
/// version with the cook's own, which is their unnamed one where they have
/// one. Once they name it, a Branch landing here unnamed would quietly become
/// the one the marks compare against.
pub(super) fn name_on_arrival(
    conn: &Connection,
    cookbook_id: &str,
    lineage_id: &str,
    language: &str,
    fallback: &str,
) -> Result<Option<String>, OpError> {
    Ok(holds_own_branch(conn, cookbook_id, lineage_id, language)?.then(|| fallback.to_string()))
}

/// Whose a Branch is, in words, for naming a Copy of it after: the sender's
/// name where it arrived from elsewhere, and otherwise the Cookbook holding
/// it.
pub(super) fn whose_branch(conn: &Connection, branch_id: &str) -> Result<String, OpError> {
    let (cookbook_id, arrived, hand_id): (String, bool, String) = conn
        .query_row(
            "SELECT cookbook_id, arrived, hand_id FROM branches WHERE id = ?1",
            params![branch_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?;
    if arrived {
        let sender: Option<String> = conn
            .query_row(
                &format!("SELECT {}", hand_name_sql("?1")),
                params![hand_id],
                |row| row.get(0),
            )
            .map_err(|e| OpError::internal(format!("cannot name a Hand: {e}")))?;
        if let Some(sender) = sender {
            return Ok(sender);
        }
    }
    cookbook_display_name(conn, &cookbook_id)
}

/// What a Cookbook is called right now, in words, for naming a Branch after
/// it: its own name or its Co-authors'.
pub(super) fn cookbook_display_name(
    conn: &Connection,
    cookbook_id: &str,
) -> Result<String, OpError> {
    conn.query_row(
        &format!("SELECT {}", cookbook_name_sql("?1")),
        params![cookbook_id],
        |row| row.get::<_, Option<String>>(0),
    )
    .map_err(|e| OpError::internal(format!("cannot name Cookbook: {e}")))
    .map(|name| name.unwrap_or_default())
}

/// Put the whole chain behind `from` onto the new Branch `onto`, row for row:
/// every Version with its Hand, name, note and date, which is what makes a
/// Copy carry its history rather than start one. Shared by every way a Branch
/// is copied, so the chain cannot be carried two ways.
pub(super) fn carry_chain(conn: &Connection, from: &str, onto: &str) -> Result<(), OpError> {
    conn.execute(
        "INSERT INTO branch_versions \
         (branch_id, sequence, version_id, parent_version_id, hand_id, name, change_note, access_key_id, created_at, translates_version_id, language) \
         SELECT ?1, sequence, version_id, parent_version_id, hand_id, name, change_note, access_key_id, created_at, translates_version_id, language \
           FROM branch_versions WHERE branch_id = ?2",
        params![onto, from],
    )
    .map_err(|e| OpError::internal(format!("cannot carry the chain onto the new Branch: {e}")))?;
    Ok(())
}

/// **A Copy that asks for no change** (ADR 0041): a new Branch of the same
/// recipe in `cookbook_id`, carrying the whole chain of `source` behind it
/// verbatim, its Tags filed in the receiving Cookbook's own list. How a
/// variation starts, and what somebody keeps of a recipe they cooked when its
/// Cookbook leaves them.
///
/// A Branch that arrived stays the sender's: its copy keeps their Hand and
/// the Travelling id their next Bundle continues, one per Cookbook, exactly
/// as two households receiving it would each hold it. A Branch written here
/// becomes the receiver's own, under their Cookbook's Hand. `name` names it
/// where given; otherwise it is named after the Cookbook it came from only if
/// the receiver already holds one of its own of that recipe (#136).
pub(super) fn copy_branch_into(
    conn: &Connection,
    source: &str,
    cookbook_id: &str,
    started_by: &str,
    name: Option<&str>,
) -> Result<String, OpError> {
    let source_row = conn
        .query_row(
            "SELECT lineage_id, language, head_version_id, hand_id, arrived, \
                    COALESCE(travelling_id, id), origin_address, name \
               FROM branches WHERE id = ?1",
            params![source],
            |row| {
                Ok(SourceBranch {
                    lineage_id: row.get(0)?,
                    language: row.get(1)?,
                    head: row.get(2)?,
                    hand: row.get(3)?,
                    arrived: row.get(4)?,
                    travelling: row.get(5)?,
                    origin: row.get(6)?,
                    name: row.get(7)?,
                })
            },
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?
        .ok_or_else(no_such_branch)?;
    let SourceBranch {
        lineage_id,
        language,
        head,
        hand,
        arrived,
        travelling,
        origin,
        name: source_name,
        ..
    } = source_row;
    let new_branch_id = format!("b_{}", hex::encode(random_bytes(8)));
    // A variation keeps the name it was given wherever it goes; an unnamed
    // Branch is named after the Cookbook it came from only where the
    // receiver already holds one of its own of the same recipe (#136).
    let name = match name.map(str::to_string).or(source_name) {
        Some(name) => Some(name),
        None => name_on_arrival(
            conn,
            cookbook_id,
            &lineage_id,
            &language,
            &whose_branch(conn, source)?,
        )?,
    };
    // The sender's Branch, received again: one per Cookbook under their id.
    let keeps_travelling = arrived && {
        let taken: bool = conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM branches \
                  WHERE COALESCE(travelling_id, id) = ?1 AND cookbook_id = ?2)",
                params![travelling, cookbook_id],
                |row| row.get(0),
            )
            .map_err(|e| OpError::internal(format!("cannot read Branches: {e}")))?;
        !taken
    };
    let hand = if keeps_travelling {
        hand
    } else {
        cookbook_hand(conn, cookbook_id)?
    };
    conn.execute(
        "INSERT INTO branches \
         (id, travelling_id, lineage_id, cookbook_id, hand_id, language, origin_address, \
          head_version_id, name, started_by, arrived) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            new_branch_id,
            keeps_travelling.then_some(&travelling),
            lineage_id,
            cookbook_id,
            hand,
            language,
            keeps_travelling.then_some(origin).flatten(),
            head,
            name,
            started_by,
            keeps_travelling as i64,
        ],
    )
    .map_err(|e| OpError::internal(format!("cannot start Branch: {e}")))?;
    carry_chain(conn, source, &new_branch_id)?;
    refile_tags(conn, source, &new_branch_id, cookbook_id)?;
    Ok(new_branch_id)
}

/// One Branch as a Copy of it needs it.
struct SourceBranch {
    lineage_id: String,
    language: String,
    head: String,
    hand: String,
    arrived: bool,
    travelling: String,
    origin: Option<String>,
    name: Option<String>,
}

/// File `to` under the Tags `from` carries, by name, in `cookbook_id`'s own
/// list — a word it already files by is that Tag, otherwise one is made.
fn refile_tags(conn: &Connection, from: &str, to: &str, cookbook_id: &str) -> Result<(), OpError> {
    let tags: Vec<String> = {
        let mut statement = conn
            .prepare("SELECT tag_id FROM branch_tags WHERE branch_id = ?1")
            .map_err(|e| OpError::internal(format!("cannot read Tags: {e}")))?;
        statement
            .query_map(params![from], |row| row.get(0))
            .map_err(|e| OpError::internal(format!("cannot read Tags: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot read Tags: {e}")))?
    };
    for tag_id in tags {
        let names: Vec<(String, String)> = {
            let mut statement = conn
                .prepare("SELECT language, name FROM tag_names WHERE tag_id = ?1 ORDER BY language")
                .map_err(|e| OpError::internal(format!("cannot read Tag names: {e}")))?;
            statement
                .query_map(params![tag_id], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(|e| OpError::internal(format!("cannot read Tag names: {e}")))?
                .collect::<Result<_, _>>()
                .map_err(|e| OpError::internal(format!("cannot read Tag names: {e}")))?
        };
        let names: Vec<(&str, &str)> = names
            .iter()
            .map(|(language, name)| (language.as_str(), name.as_str()))
            .collect();
        if let Some(tag) = arriving_tag(conn, cookbook_id, &names)? {
            file_branch_under(conn, to, &tag)?;
        }
    }
    Ok(())
}

/// Take a Branch off the shelf for good, with everything keyed on it and
/// nothing else (#120): what `delete_recipe` does once it has decided who
/// keeps a copy.
pub(super) fn delete_branch_rows(conn: &Connection, branch_id: &str) -> Result<(), OpError> {
    for statement in [
        "DELETE FROM branch_versions WHERE branch_id = ?1",
        "DELETE FROM branch_tags WHERE branch_id = ?1",
        "DELETE FROM share_links WHERE branch_id = ?1",
        // The ledger belongs to the Import, not to the recipe (ADR 0025), so
        // re-running the importer that first brought this in creates it
        // afresh rather than matching what was deleted.
        "DELETE FROM import_ledger WHERE branch_id = ?1",
        // Derived, never truth (ADR 0029). Gone from Meaning Search the moment
        // the Branch is, without waiting for a rebuild.
        "DELETE FROM meaning_vectors WHERE branch_id = ?1",
        "DELETE FROM branches WHERE id = ?1",
    ] {
        conn.execute(statement, params![branch_id])
            .map_err(|e| OpError::internal(format!("cannot delete Recipe: {e}")))?;
    }
    Ok(())
}

/// Every Branch in `cookbook_id` that `person_id` cooked: holds an Attempt on
/// a Version the Branch carries. What they keep a Branch of when the Cookbook
/// stops being theirs to see (ADR 0041) — unless their own Cookbook already
/// holds the Version they cooked, as it does when they saved a Copy of the
/// recipe after cooking it: they keep a Branch of it, not a second one.
pub(super) fn cooked_from(
    conn: &Connection,
    cookbook_id: &str,
    person_id: &str,
) -> Result<Vec<String>, OpError> {
    let mut statement = conn
        .prepare(
            "SELECT branches.id, branches.lineage_id, branches.language FROM branches \
              WHERE branches.cookbook_id = ?1 \
                AND EXISTS (SELECT 1 FROM attempts \
                             WHERE attempts.person_id = ?2 \
                               AND attempts.lineage_id = branches.lineage_id \
                               AND attempts.version_id IN \
                                   (SELECT version_id FROM branch_versions \
                                     WHERE branch_versions.branch_id = branches.id) \
                               AND NOT EXISTS ( \
                                   SELECT 1 FROM branch_versions AS held \
                                     JOIN branches AS own ON own.id = held.branch_id \
                                     JOIN cookbook_authors ON cookbook_authors.cookbook_id = own.cookbook_id \
                                    WHERE cookbook_authors.person_id = ?2 \
                                      AND own.cookbook_id <> ?1 \
                                      AND held.version_id = attempts.version_id)) \
              ORDER BY branches.name IS NULL DESC, branches.created_at, branches.id",
        )
        .map_err(|e| OpError::internal(format!("cannot read what was cooked: {e}")))?;
    let cooked: Vec<(String, String, String)> = statement
        .query_map(params![cookbook_id, person_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| OpError::internal(format!("cannot read what was cooked: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read what was cooked: {e}")))?;
    // A Branch of what was cooked, not one of every Branch that holds it:
    // an unchanged variation holds the very Version that was cooked too.
    // The unnamed one first, then the oldest, as opening a recipe chooses;
    // a later one is kept too only where it holds a Version they cooked that
    // nothing kept so far does — a variation they cooked after it changed.
    let chain = |branch: &str| -> Result<HashSet<String>, OpError> {
        let mut statement = conn
            .prepare("SELECT version_id FROM branch_versions WHERE branch_id = ?1")
            .map_err(|e| OpError::internal(format!("cannot read what was cooked: {e}")))?;
        statement
            .query_map(params![branch], |row| row.get(0))
            .map_err(|e| OpError::internal(format!("cannot read what was cooked: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot read what was cooked: {e}")))
    };
    let cooked_versions = |lineage: &str| -> Result<HashSet<String>, OpError> {
        let mut statement = conn
            .prepare("SELECT version_id FROM attempts WHERE person_id = ?1 AND lineage_id = ?2")
            .map_err(|e| OpError::internal(format!("cannot read what was cooked: {e}")))?;
        statement
            .query_map(params![person_id, lineage], |row| row.get(0))
            .map_err(|e| OpError::internal(format!("cannot read what was cooked: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot read what was cooked: {e}")))
    };
    let mut covered: HashMap<(String, String), HashSet<String>> = HashMap::new();
    let mut kept = Vec::new();
    for (branch_id, lineage, language) in cooked {
        let held = chain(&branch_id)?;
        let cooked_here: HashSet<String> = cooked_versions(&lineage)?
            .into_iter()
            .filter(|version| held.contains(version))
            .collect();
        let so_far = covered.entry((lineage, language)).or_default();
        if !cooked_here.is_subset(so_far) {
            so_far.extend(held);
            kept.push(branch_id);
        }
    }
    Ok(kept)
}

/// Every Cookbook each of `people` may see right now.
fn sight_of(
    conn: &Connection,
    people: &[String],
) -> Result<HashMap<String, HashSet<String>>, OpError> {
    let mut sight: HashMap<String, HashSet<String>> = HashMap::new();
    let mut statement = conn
        .prepare("SELECT cookbook_id FROM visible_cookbooks WHERE person_id = ?1")
        .map_err(|e| OpError::internal(format!("cannot read who sees what: {e}")))?;
    for person in people {
        let seen: HashSet<String> = statement
            .query_map(params![person], |row| row.get(0))
            .map_err(|e| OpError::internal(format!("cannot read who sees what: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot read who sees what: {e}")))?;
        sight.insert(person.clone(), seen);
    }
    Ok(sight)
}

/// What a change of who sees what costs each person it touches, and what they
/// keep for it (ADR 0041, #131 question 2): `change` is run, and every Branch
/// of a Cookbook somebody could see before and cannot now, that they cooked,
/// is theirs to keep — `(person, branch)`, in the order they are kept.
/// `keeping` is false for anybody who keeps nothing, which is a deleted
/// account.
pub(super) fn keeps_after(
    conn: &Connection,
    people: &[String],
    change: impl FnOnce(&Connection) -> Result<(), OpError>,
    keeping: impl Fn(&str) -> bool,
) -> Result<Vec<(String, String)>, OpError> {
    let before = sight_of(conn, people)?;
    change(conn)?;
    let after = sight_of(conn, people)?;
    let mut kept: Vec<(String, String)> = Vec::new();
    for person in people {
        if !keeping(person) {
            continue;
        }
        let mut lost: Vec<&String> = before[person].difference(&after[person]).collect();
        lost.sort();
        for cookbook_id in lost {
            for branch_id in cooked_from(conn, cookbook_id, person)? {
                kept.push((person.clone(), branch_id));
            }
        }
    }
    Ok(kept)
}

/// Give each `(person, branch)` its Branch, in the person's own Cookbook.
pub(super) fn keep_branches(conn: &Connection, kept: &[(String, String)]) -> Result<(), OpError> {
    for (person, branch_id) in kept {
        let own = cookbook_of_person(conn, person)?;
        let copy = copy_branch_into(conn, branch_id, &own, person, None)?;
        mark_copied(conn, &copy, branch_id)?;
        repoint_shopping(conn, person, branch_id, &copy)?;
    }
    Ok(())
}

/// Remember that `copy` was made from `source` as two people parted, so that
/// writing together again can fold the one back into the other.
fn mark_copied(conn: &Connection, copy: &str, source: &str) -> Result<(), OpError> {
    conn.execute(
        "UPDATE branches SET copied_from = ?2 WHERE id = ?1",
        params![copy, source],
    )
    .map_err(|e| OpError::internal(format!("cannot record a copy: {e}")))?;
    Ok(())
}

/// A Shopping List entry follows a recipe to the Branch its reader keeps of
/// it (#131, question 12), rather than going dead on one they can no longer
/// read.
fn repoint_shopping(
    conn: &Connection,
    person_id: &str,
    from: &str,
    to: &str,
) -> Result<(), OpError> {
    conn.execute(
        "UPDATE OR IGNORE shopping_choices SET branch_id = ?3 WHERE person_id = ?1 AND branch_id = ?2",
        params![person_id, from, to],
    )
    .map_err(|e| OpError::internal(format!("cannot move a Shopping List entry: {e}")))?;
    // What the move left behind is only a list that already held the Branch
    // kept: the old entry would be a second line for the same recipe, and one
    // its reader can no longer open.
    conn.execute(
        "DELETE FROM shopping_choices WHERE person_id = ?1 AND branch_id = ?2",
        params![person_id, from],
    )
    .map_err(|e| OpError::internal(format!("cannot move a Shopping List entry: {e}")))?;
    Ok(())
}

/// Two Cookbooks become one: everything `from` holds moves into `into`, and
/// `joiner` writes there from now on (#131, questions 5 and 6).
fn join_cookbooks(conn: &Connection, from: &str, into: &str, joiner: &str) -> Result<(), OpError> {
    let joiner_name: String = conn
        .query_row(
            "SELECT name FROM people WHERE id = ?1",
            params![joiner],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot read Person: {e}")))?;

    #[allow(clippy::type_complexity)]
    let arriving: Vec<(
        String,
        String,
        String,
        Option<String>,
        String,
        bool,
        bool,
        Option<String>,
    )> = {
        let mut statement = conn
            .prepare(
                "SELECT id, lineage_id, language, name, COALESCE(travelling_id, id), \
                        arrived = 0, \
                        NOT EXISTS (SELECT 1 FROM share_links \
                          WHERE share_links.branch_id = branches.id \
                            AND share_links.ended_at IS NULL), \
                        copied_from \
                   FROM branches WHERE cookbook_id = ?1 ORDER BY created_at, id",
            )
            .map_err(|e| OpError::internal(format!("cannot read Branches: {e}")))?;
        statement
            .query_map(params![from], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            })
            .map_err(|e| OpError::internal(format!("cannot read Branches: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot read Branches: {e}")))?
    };

    // Tags first, so every Branch arrives into a list that already has its
    // words: one word is one Tag in a Cookbook, and the joiner's Tag of the
    // same word folds into the one already there.
    let tags: Vec<String> = {
        let mut statement = conn
            .prepare("SELECT id FROM tags WHERE cookbook_id = ?1 ORDER BY created_at, id")
            .map_err(|e| OpError::internal(format!("cannot read Tags: {e}")))?;
        statement
            .query_map(params![from], |row| row.get(0))
            .map_err(|e| OpError::internal(format!("cannot read Tags: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot read Tags: {e}")))?
    };
    for tag_id in tags {
        move_tag(conn, &tag_id, into)?;
    }

    for (branch_id, lineage_id, language, name, travelling, written_here, unlinked, copied_from) in
        arriving
    {
        let same_travelling: Option<String> = conn
            .query_row(
                "SELECT id FROM branches WHERE COALESCE(travelling_id, id) = ?1 AND cookbook_id = ?2",
                params![travelling, into],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| OpError::internal(format!("cannot read Branches: {e}")))?;
        if let Some(held) = same_travelling {
            match longer_of(conn, &held, &branch_id)? {
                Some(keep) => {
                    let drop = if keep == held { &branch_id } else { &held };
                    fold_branch(conn, drop, &keep, into)?;
                    if keep == held {
                        continue;
                    }
                }
                None => {
                    conn.execute(
                        "UPDATE branches SET travelling_id = ?1 WHERE id = ?2",
                        params![format!("b_{}", hex::encode(random_bytes(8))), branch_id],
                    )
                    .map_err(|e| OpError::internal(format!("cannot re-mint Travelling id: {e}")))?;
                }
            }
        }
        // The same Branch twice. When these two last parted, one side kept a
        // copy of the other's Branch; writing together again, a copy nobody
        // has changed since is that Branch held twice, and folds back into
        // the one it came from, whatever each has been named since. Without
        // this, every leave and join again would add a copy of the whole
        // library. Only an identical history: a copy somebody changed holds
        // writing the other lacks. Never one that arrived, which a sender's
        // next Bundle continues, nor one carrying a live Share Link, which
        // goes with whoever started it (#131, screen choice 2).
        if written_here {
            // The joiner's copy of a Branch the inviting Cookbook holds: the
            // copy is what goes, so not while it carries a live link.
            if let Some(source) = copied_from.as_ref().filter(|_| unlinked) {
                let held: Option<String> = conn
                    .query_row(
                        "SELECT id FROM branches WHERE id = ?1 AND cookbook_id = ?2",
                        params![source, into],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(|e| OpError::internal(format!("cannot read Branches: {e}")))?;
                if let Some(held) = held
                    && same_history(conn, &held, &branch_id)?
                {
                    fold_branch(conn, &branch_id, &held, into)?;
                    continue;
                }
            }
            // The inviting Cookbook's copy of this very Branch: the original
            // comes in, and the copy folds into it — a link on the original
            // stays where it is, since the original is what is kept.
            let copies_of_it: Vec<String> = {
                let mut statement = conn
                    .prepare(
                        "SELECT id FROM branches \
                          WHERE cookbook_id = ?1 AND copied_from = ?2 AND arrived = 0 \
                            AND NOT EXISTS (SELECT 1 FROM share_links \
                                  WHERE share_links.branch_id = branches.id \
                                    AND share_links.ended_at IS NULL)",
                    )
                    .map_err(|e| OpError::internal(format!("cannot read Branches: {e}")))?;
                statement
                    .query_map(params![into, branch_id], |row| row.get(0))
                    .map_err(|e| OpError::internal(format!("cannot read Branches: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot read Branches: {e}")))?
            };
            for copy in copies_of_it {
                if same_history(conn, &copy, &branch_id)? {
                    fold_branch(conn, &copy, &branch_id, into)?;
                }
            }
        }
        if name.is_none() && holds_own_branch(conn, into, &lineage_id, &language)? {
            conn.execute(
                "UPDATE branches SET name = ?1 WHERE id = ?2",
                params![joiner_name, branch_id],
            )
            .map_err(|e| OpError::internal(format!("cannot name Branch: {e}")))?;
        }
        conn.execute(
            "UPDATE branches SET cookbook_id = ?1 WHERE id = ?2",
            params![into, branch_id],
        )
        .map_err(|e| OpError::internal(format!("cannot move Branch: {e}")))?;
    }

    copy_related(conn, from, into)?;
    conn.execute(
        "DELETE FROM related_recipes WHERE cookbook_id = ?1",
        params![from],
    )
    .map_err(|e| OpError::internal(format!("cannot carry Related Recipes: {e}")))?;

    move_imports(conn, from, into)?;

    // The joiner's Hands answer to the joined Cookbook now, so what their own
    // Cookbook wrote is still named after where it is kept.
    conn.execute(
        "UPDATE cookbook_hands SET cookbook_id = ?2 WHERE cookbook_id = ?1",
        params![from, into],
    )
    .map_err(|e| OpError::internal(format!("cannot carry Hands: {e}")))?;
    conn.execute(
        "UPDATE cookbook_authors SET cookbook_id = ?2, \
                joined_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
          WHERE cookbook_id = ?1",
        params![from, into],
    )
    .map_err(|e| OpError::internal(format!("cannot join Cookbooks: {e}")))?;
    for statement in [
        // Every join the Cookbook that goes was part of, this one included:
        // nothing is left for them to join or wait on (#135).
        "DELETE FROM cookbook_joins WHERE from_cookbook = ?1 OR into_cookbook = ?1",
        "DELETE FROM cookbook_invites WHERE cookbook_id = ?1",
        "DELETE FROM cookbooks WHERE id = ?1",
    ] {
        conn.execute(statement, params![from])
            .map_err(|e| OpError::internal(format!("cannot join Cookbooks: {e}")))?;
    }
    Ok(())
}

/// Move one Tag into `into`: into the Tag already filing by one of its words
/// there, or across whole where there is none.
fn move_tag(conn: &Connection, tag_id: &str, into: &str) -> Result<(), OpError> {
    let names: Vec<(String, String, String)> = {
        let mut statement = conn
            .prepare("SELECT language, name, name_folded FROM tag_names WHERE tag_id = ?1 ORDER BY language")
            .map_err(|e| OpError::internal(format!("cannot read Tag names: {e}")))?;
        statement
            .query_map(params![tag_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(|e| OpError::internal(format!("cannot read Tag names: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot read Tag names: {e}")))?
    };
    let mut same: Option<String> = None;
    for (language, _, folded) in &names {
        same = conn
            .query_row(
                "SELECT tag_id FROM tag_names WHERE cookbook_id = ?1 AND language = ?2 AND name_folded = ?3",
                params![into, language, folded],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| OpError::internal(format!("cannot read Tag names: {e}")))?;
        if same.is_some() {
            break;
        }
    }
    match same {
        None => {
            conn.execute(
                "UPDATE tags SET cookbook_id = ?1 WHERE id = ?2",
                params![into, tag_id],
            )
            .map_err(|e| OpError::internal(format!("cannot move Tag: {e}")))?;
            conn.execute(
                "UPDATE tag_names SET cookbook_id = ?1 WHERE tag_id = ?2",
                params![into, tag_id],
            )
            .map_err(|e| OpError::internal(format!("cannot move Tag: {e}")))?;
        }
        Some(keep) => {
            conn.execute(
                "INSERT OR IGNORE INTO branch_tags (branch_id, tag_id) \
                 SELECT branch_id, ?1 FROM branch_tags WHERE tag_id = ?2",
                params![keep, tag_id],
            )
            .map_err(|e| OpError::internal(format!("cannot carry recipes across: {e}")))?;
            for statement in [
                "DELETE FROM branch_tags WHERE tag_id = ?1",
                "DELETE FROM tag_names WHERE tag_id = ?1",
                "DELETE FROM tags WHERE id = ?1",
            ] {
                conn.execute(statement, params![tag_id])
                    .map_err(|e| OpError::internal(format!("cannot fold Tag: {e}")))?;
            }
            for (language, name, folded) in names {
                conn.execute(
                    "INSERT OR IGNORE INTO tag_names (tag_id, cookbook_id, language, name, name_folded) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![keep, into, language, name, folded],
                )
                .map_err(|e| OpError::internal(format!("cannot adopt Tag name: {e}")))?;
            }
        }
    }
    Ok(())
}

/// Of two holdings of one sender's Branch, the one whose history contains the
/// other's, or `None` when the two really differ. Where the two histories are
/// the same, a Branch written in a Cookbook beats one received into it, so
/// the original is never folded into a copy of itself (#131, answer 7).
fn longer_of(conn: &Connection, a: &str, b: &str) -> Result<Option<String>, OpError> {
    let chain = |branch: &str| -> Result<Vec<String>, OpError> {
        let mut statement = conn
            .prepare(
                "SELECT version_id FROM branch_versions WHERE branch_id = ?1 ORDER BY sequence",
            )
            .map_err(|e| OpError::internal(format!("cannot read Branch chain: {e}")))?;
        statement
            .query_map(params![branch], |row| row.get(0))
            .map_err(|e| OpError::internal(format!("cannot read Branch chain: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot read Branch chain: {e}")))
    };
    let (chain_a, chain_b) = (chain(a)?, chain(b)?);
    if chain_a == chain_b {
        let arrived = |branch: &str| -> Result<bool, OpError> {
            conn.query_row(
                "SELECT arrived FROM branches WHERE id = ?1",
                params![branch],
                |row| row.get(0),
            )
            .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))
        };
        let keep = if arrived(a)? && !arrived(b)? { b } else { a };
        return Ok(Some(keep.to_string()));
    }
    let (shorter, longer, long_id) = if chain_a.len() >= chain_b.len() {
        (&chain_b, &chain_a, a)
    } else {
        (&chain_a, &chain_b, b)
    };
    Ok(longer.starts_with(shorter).then(|| long_id.to_string()))
}

/// Whether two Branches hold exactly the same history, Version for Version.
/// The only case two Branches that are not the same sender's are folded
/// into one: anything less and one of them holds writing the other lacks.
fn same_history(conn: &Connection, a: &str, b: &str) -> Result<bool, OpError> {
    let chain = |branch: &str| -> Result<Vec<String>, OpError> {
        let mut statement = conn
            .prepare(
                "SELECT version_id FROM branch_versions WHERE branch_id = ?1 ORDER BY sequence",
            )
            .map_err(|e| OpError::internal(format!("cannot read Branch chain: {e}")))?;
        statement
            .query_map(params![branch], |row| row.get(0))
            .map_err(|e| OpError::internal(format!("cannot read Branch chain: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot read Branch chain: {e}")))
    };
    Ok(chain(a)? == chain(b)?)
}

/// Fold `drop` into `keep`, two holdings of one history: whatever pointed at
/// the one now points at the other, and the one goes.
///
/// `into` is the Cookbook whose Tag list the kept one is filed in, which is
/// where both are headed rather than where either stands right now.
fn fold_branch(conn: &Connection, drop: &str, keep: &str, into: &str) -> Result<(), OpError> {
    refile_tags(conn, drop, keep, into)?;
    // A copy of the one that goes is a copy of the one kept now.
    conn.execute(
        "UPDATE branches SET copied_from = ?2 WHERE copied_from = ?1",
        params![drop, keep],
    )
    .map_err(|e| OpError::internal(format!("cannot move a copy's source: {e}")))?;
    conn.execute(
        "UPDATE OR IGNORE shopping_choices SET branch_id = ?2 WHERE branch_id = ?1",
        params![drop, keep],
    )
    .map_err(|e| OpError::internal(format!("cannot move Shopping List entries: {e}")))?;
    // What the move left behind is only ever a list that held both, which
    // now holds the kept one: the old entry would be a second, dead line.
    conn.execute(
        "DELETE FROM shopping_choices WHERE branch_id = ?1",
        params![drop],
    )
    .map_err(|e| OpError::internal(format!("cannot move Shopping List entries: {e}")))?;
    let keep_has_link: bool = conn
        .query_row(
            "SELECT EXISTS (SELECT 1 FROM share_links WHERE branch_id = ?1 AND ended_at IS NULL)",
            params![keep],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot read Share Links: {e}")))?;
    if keep_has_link {
        conn.execute(
            "UPDATE share_links SET ended_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
              WHERE branch_id = ?1 AND ended_at IS NULL",
            params![drop],
        )
        .map_err(|e| OpError::internal(format!("cannot end Share Link: {e}")))?;
    }
    conn.execute(
        "UPDATE share_links SET branch_id = ?2 WHERE branch_id = ?1",
        params![drop, keep],
    )
    .map_err(|e| OpError::internal(format!("cannot move Share Links: {e}")))?;
    conn.execute(
        "UPDATE OR IGNORE import_ledger SET branch_id = ?2 WHERE branch_id = ?1",
        params![drop, keep],
    )
    .map_err(|e| OpError::internal(format!("cannot move the import ledger: {e}")))?;
    delete_branch_rows(conn, drop)
}

/// Move a Cookbook's Imports into another, one per kind of source: the ledger
/// of one the other already has joins it.
fn move_imports(conn: &Connection, from: &str, into: &str) -> Result<(), OpError> {
    let imports: Vec<(String, String)> = {
        let mut statement = conn
            .prepare("SELECT id, source_kind FROM imports WHERE cookbook_id = ?1")
            .map_err(|e| OpError::internal(format!("cannot read Imports: {e}")))?;
        statement
            .query_map(params![from], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| OpError::internal(format!("cannot read Imports: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot read Imports: {e}")))?
    };
    for (import_id, kind) in imports {
        let there: Option<String> = conn
            .query_row(
                "SELECT id FROM imports WHERE cookbook_id = ?1 AND source_kind = ?2",
                params![into, kind],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| OpError::internal(format!("cannot read Imports: {e}")))?;
        match there {
            None => {
                conn.execute(
                    "UPDATE imports SET cookbook_id = ?1 WHERE id = ?2",
                    params![into, import_id],
                )
                .map_err(|e| OpError::internal(format!("cannot move Import: {e}")))?;
            }
            Some(there) => {
                conn.execute(
                    "UPDATE OR IGNORE import_ledger SET import_id = ?2 WHERE import_id = ?1",
                    params![import_id, there],
                )
                .map_err(|e| OpError::internal(format!("cannot move the import ledger: {e}")))?;
                conn.execute(
                    "DELETE FROM import_ledger WHERE import_id = ?1",
                    params![import_id],
                )
                .map_err(|e| OpError::internal(format!("cannot move the import ledger: {e}")))?;
                conn.execute("DELETE FROM imports WHERE id = ?1", params![import_id])
                    .map_err(|e| OpError::internal(format!("cannot move Import: {e}")))?;
            }
        }
    }
    Ok(())
}

/// The Import of one kind a Cookbook brings recipes in through, made the
/// first time it is asked for (ADR 0025).
pub(super) fn find_or_create_import(
    conn: &Connection,
    cookbook_id: &str,
    source_kind: &str,
) -> Result<String, OpError> {
    if let Some(id) = conn
        .query_row(
            "SELECT id FROM imports WHERE cookbook_id = ?1 AND source_kind = ?2",
            params![cookbook_id, source_kind],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read Import: {e}")))?
    {
        return Ok(id);
    }
    let id = format!("imp_{}", hex::encode(random_bytes(8)));
    conn.execute(
        "INSERT INTO imports (id, cookbook_id, source_kind) VALUES (?1, ?2, ?3)",
        params![id, cookbook_id, source_kind],
    )
    .map_err(|e| OpError::internal(format!("cannot create Import: {e}")))?;
    Ok(id)
}

/// Give `into` every Related Recipe `from` keeps, leaving `from`'s as they
/// are: what joining two Cookbooks and separating one both do with links.
fn copy_related(conn: &Connection, from: &str, into: &str) -> Result<(), OpError> {
    conn.execute(
        "INSERT OR IGNORE INTO related_recipes \
         (cookbook_id, lineage_a_id, lineage_b_id, lineage_a_name, lineage_b_name, created_at) \
         SELECT ?2, lineage_a_id, lineage_b_id, lineage_a_name, lineage_b_name, created_at \
           FROM related_recipes WHERE cookbook_id = ?1",
        params![from, into],
    )
    .map_err(|e| OpError::internal(format!("cannot carry Related Recipes: {e}")))?;
    Ok(())
}

/// `leaver` separates from `cookbook_id` (#131, questions 5 and 7), and the
/// Cookbook they write alone from now on is answered.
///
/// **Nothing is lost.** Each side leaves with a Branch of every recipe, with
/// its whole history. Whoever started a Branch keeps the Branch itself, with
/// its ids and its Share Link; the other side gets a Copy. The leaver takes
/// back the Hands their own Cookbook had before it joined, and writes under
/// the first of them again.
pub(super) fn separate(
    conn: &Connection,
    cookbook_id: &str,
    leaver: &str,
) -> Result<String, OpError> {
    // A Cookbook's Hand is minted with it and never changes (#131, answer
    // 6): the Cookbook that stays keeps its own, and the leaver's new one is
    // made with a new Hand, like any Cookbook, under its own id.
    let own = format!("c_{}", hex::encode(random_bytes(8)));
    let staying_hand = cookbook_hand(conn, cookbook_id)?;
    conn.execute(
        "DELETE FROM cookbook_authors WHERE cookbook_id = ?1 AND person_id = ?2",
        params![cookbook_id, leaver],
    )
    .map_err(|e| OpError::internal(format!("cannot separate: {e}")))?;
    insert_cookbook(conn, &own, &own, leaver)?;
    // Versions already written keep the Hand they carry. The Hands the
    // leaver brought when they joined still name them, so they resolve to
    // the leaver's new Cookbook; the staying Cookbook's own Hand never
    // moves, even where the leaver was the one who minted it.
    conn.execute(
        "UPDATE cookbook_hands SET cookbook_id = ?1 \
          WHERE cookbook_id = ?2 AND person_id = ?3 AND hand_id <> ?4",
        params![own, cookbook_id, leaver, staying_hand],
    )
    .map_err(|e| OpError::internal(format!("cannot carry Hands: {e}")))?;
    let stayer: String = authors_of(conn, cookbook_id)?
        .into_iter()
        .next()
        .ok_or_else(|| OpError::internal("a Cookbook was left with nobody writing it"))?;

    let branches: Vec<(String, Option<String>)> = {
        let mut statement = conn
            .prepare("SELECT id, started_by FROM branches WHERE cookbook_id = ?1 ORDER BY created_at, id")
            .map_err(|e| OpError::internal(format!("cannot read Branches: {e}")))?;
        statement
            .query_map(params![cookbook_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| OpError::internal(format!("cannot read Branches: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot read Branches: {e}")))?
    };
    // What went with the leaver as it was, and the copies made for them: the
    // two `give_back_own_names` tells apart, since both now say the leaver
    // started them.
    let mut moved: Vec<String> = Vec::new();
    let mut copies: Vec<String> = Vec::new();
    for (branch_id, started_by) in branches {
        if started_by.as_deref() == Some(leaver) {
            moved.push(branch_id.clone());
            // Theirs: the Branch goes with them, and the Cookbook that stays
            // keeps a Copy in its place — made after the move, so it takes
            // the place the Branch left rather than standing beside it. The
            // Copy is filed by the Tags the Branch had here, and the Branch
            // itself is refiled in the leaver's own list.
            conn.execute(
                "UPDATE branches SET cookbook_id = ?1 WHERE id = ?2",
                params![own, branch_id],
            )
            .map_err(|e| OpError::internal(format!("cannot move Branch: {e}")))?;
            let copy = copy_branch_into(conn, &branch_id, cookbook_id, &stayer, None)?;
            mark_copied(conn, &copy, &branch_id)?;
            conn.execute(
                "DELETE FROM branch_tags WHERE branch_id = ?1",
                params![branch_id],
            )
            .map_err(|e| OpError::internal(format!("cannot refile Branch: {e}")))?;
            refile_tags(conn, &copy, &branch_id, &own)?;
            move_ledger_rows(conn, &branch_id, &own)?;
            // Everybody else's Shopping List follows the recipe to the Copy
            // they can still read.
            conn.execute(
                "UPDATE OR IGNORE shopping_choices SET branch_id = ?2 \
                  WHERE branch_id = ?1 AND person_id <> ?3",
                params![branch_id, copy, leaver],
            )
            .map_err(|e| OpError::internal(format!("cannot move Shopping List entries: {e}")))?;
            conn.execute(
                "DELETE FROM shopping_choices WHERE branch_id = ?1 AND person_id <> ?2",
                params![branch_id, leaver],
            )
            .map_err(|e| OpError::internal(format!("cannot move Shopping List entries: {e}")))?;
        } else {
            let copy = copy_branch_into(conn, &branch_id, &own, leaver, None)?;
            mark_copied(conn, &copy, &branch_id)?;
            repoint_shopping(conn, leaver, &branch_id, &copy)?;
            copies.push(copy);
        }
    }
    copy_related(conn, cookbook_id, &own)?;
    give_back_own_names(conn, cookbook_id, leaver, &moved, &copies)?;
    // Whoever left no longer has a say in a join this Cookbook waits on, and
    // may have been the last one it waited for (#135).
    settle_joins_of(conn, cookbook_id)?;
    Ok(own)
}

/// **The recipes a leaver started are theirs again, unnamed** (#131, answers
/// 6 and 7). Joining named the joiner's Branch after them wherever the
/// inviting Cookbook already held an unnamed one of the recipe, so that each
/// Cookbook keeps one. Leaving, that Branch goes back with them still wearing
/// their own name, while the unnamed place in their new Cookbook is taken by
/// their copy of the other side's. Opening "your own" would then open the
/// copy. So the two swap: the Branch they started loses the name, and the
/// copy is named after the Cookbook it came from, as any copy arriving beside
/// one of your own is. Where every copy already has a name, the Branch they
/// started simply loses its own (#136).
fn give_back_own_names(
    conn: &Connection,
    stayed: &str,
    leaver: &str,
    moved: &[String],
    copies: &[String],
) -> Result<(), OpError> {
    let leaver_name: String = conn
        .query_row(
            "SELECT name FROM people WHERE id = ?1",
            params![leaver],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot read Person: {e}")))?;
    let recipe_of = |branch_id: &str| -> Result<(String, String, Option<String>, bool), OpError> {
        conn.query_row(
            "SELECT lineage_id, language, name, arrived FROM branches WHERE id = ?1",
            params![branch_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))
    };
    let came_from = cookbook_display_name(conn, stayed)?;
    for theirs in moved {
        let (lineage, language, name, arrived) = recipe_of(theirs)?;
        if arrived || name.as_deref() != Some(leaver_name.as_str()) {
            continue;
        }
        let mut swapped = false;
        for copy in copies {
            let (copy_lineage, copy_language, copy_name, copy_arrived) = recipe_of(copy)?;
            if copy_lineage != lineage
                || copy_language != language
                || copy_name.is_some()
                || copy_arrived
            {
                continue;
            }
            conn.execute(
                "UPDATE branches SET name = ?2 WHERE id = ?1",
                params![copy, came_from],
            )
            .map_err(|e| OpError::internal(format!("cannot name Branch: {e}")))?;
            conn.execute(
                "UPDATE branches SET name = NULL WHERE id = ?1",
                params![theirs],
            )
            .map_err(|e| OpError::internal(format!("cannot name Branch: {e}")))?;
            swapped = true;
            break;
        }
        // No unnamed copy to swap with: the copies carry the names the other
        // side gave theirs (#136). Theirs still goes without a name, as the
        // one their new Cookbook compares the others with, unless it already
        // keeps an unnamed one of that recipe.
        if !swapped {
            let cookbook = branch_cookbook(conn, theirs)?;
            if unnamed_branch_in(conn, &cookbook, &lineage, &language, Some(theirs))?.is_none() {
                conn.execute(
                    "UPDATE branches SET name = NULL WHERE id = ?1",
                    params![theirs],
                )
                .map_err(|e| OpError::internal(format!("cannot name Branch: {e}")))?;
            }
        }
    }
    Ok(())
}

/// A Branch leaving a Cookbook takes its import ledger rows to the Import of
/// the same kind in the Cookbook it goes to, so re-running that importer there
/// still finds it.
fn move_ledger_rows(conn: &Connection, branch_id: &str, into: &str) -> Result<(), OpError> {
    let rows: Vec<(String, String)> = {
        let mut statement = conn
            .prepare(
                "SELECT import_ledger.import_id, imports.source_kind FROM import_ledger \
                   JOIN imports ON imports.id = import_ledger.import_id \
                  WHERE import_ledger.branch_id = ?1",
            )
            .map_err(|e| OpError::internal(format!("cannot read the import ledger: {e}")))?;
        statement
            .query_map(params![branch_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| OpError::internal(format!("cannot read the import ledger: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot read the import ledger: {e}")))?
    };
    for (import_id, kind) in rows {
        let there = find_or_create_import(conn, into, &kind)?;
        conn.execute(
            "UPDATE OR IGNORE import_ledger SET import_id = ?1 WHERE import_id = ?2 AND branch_id = ?3",
            params![there, import_id, branch_id],
        )
        .map_err(|e| OpError::internal(format!("cannot move the import ledger: {e}")))?;
    }
    Ok(())
}
