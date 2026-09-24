//! Kitchens and their members. A Kitchen is a group of People who see and
//! cook from each other's Cookbooks; it holds no recipe (ADR 0041).

use super::*;

impl Core {
    /// Create a Kitchen: anyone may, and its creator is its first (and, until
    /// someone is invited in, only) member (ADR 0007).
    pub fn create_kitchen(&self, person_id: &str, name: &str) -> Result<Value, OpError> {
        let name = required_text(name, "name")?.to_string();
        let kitchen_id = format!("k_{}", hex::encode(random_bytes(8)));
        self.db().with_conn(|conn| {
            insert_kitchen_with_member(conn, &kitchen_id, &name, person_id)?;
            kitchen_summary(conn, &kitchen_id, person_id)
        })
    }

    /// Every Kitchen a Person cooks in.
    pub fn list_kitchens(&self, person_id: &str) -> Result<Vec<Value>, OpError> {
        self.db().with_conn(|conn| {
            let ids: Vec<String> = {
                let mut statement = conn
                    .prepare("SELECT kitchen_id FROM kitchen_members WHERE person_id = ?1")
                    .map_err(|e| OpError::internal(format!("cannot list Kitchens: {e}")))?;
                statement
                    .query_map(params![person_id], |row| row.get(0))
                    .map_err(|e| OpError::internal(format!("cannot list Kitchens: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot list Kitchens: {e}")))?
            };
            ids.iter()
                .map(|id| kitchen_summary(conn, id, person_id))
                .collect()
        })
    }

    /// Any member may change the Kitchen's shared Name.
    pub fn rename_kitchen(
        &self,
        person_id: &str,
        kitchen_id: &str,
        name: &str,
    ) -> Result<(), OpError> {
        let name = required_text(name, "name")?;
        self.db().with_conn(|conn| {
            ensure_member(conn, kitchen_id, person_id)?;
            conn.execute(
                "UPDATE kitchens SET name = ?1 WHERE id = ?2",
                params![name, kitchen_id],
            )
            .map_err(|e| OpError::internal(format!("cannot rename Kitchen: {e}")))?;
            Ok(())
        })
    }

    /// A member's own private relabelling of a Kitchen — theirs alone, never
    /// seen by anyone else and never travelling with it (ADR 0007). An empty or
    /// absent Nickname clears it back to the shared Name.
    pub fn set_kitchen_nickname(
        &self,
        person_id: &str,
        kitchen_id: &str,
        nickname: Option<&str>,
    ) -> Result<(), OpError> {
        let nickname = nickname.map(str::trim).filter(|n| !n.is_empty());
        self.db().with_conn(|conn| {
            ensure_member(conn, kitchen_id, person_id)?;
            conn.execute(
                "UPDATE kitchen_members SET nickname = ?1 WHERE kitchen_id = ?2 AND person_id = ?3",
                params![nickname, kitchen_id, person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot set Nickname: {e}")))?;
            Ok(())
        })
    }

    /// Mint a one-use link a member can pass to another Person to join this
    /// Kitchen. The raw Secret is returned once and stored only as its hash.
    pub fn invite_to_kitchen(
        &self,
        person_id: &str,
        kitchen_id: &str,
    ) -> Result<(String, String), OpError> {
        let secret = generate_secret();
        let hash = hash_secret(&secret);
        let id = format!("ki_{}", hex::encode(random_bytes(8)));
        self.db().with_conn(|conn| {
            ensure_member(conn, kitchen_id, person_id)?;
            conn.execute(
                "INSERT INTO kitchen_invites (id, secret_hash, kitchen_id, created_by) VALUES (?1, ?2, ?3, ?4)",
                params![id, hash, kitchen_id, person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot mint Kitchen Invite: {e}")))?;
            Ok(())
        })?;
        Ok((id, secret))
    }

    /// Open a Kitchen Invite: the Person presenting it joins the Kitchen, and
    /// the Invite is spent. Already being a member is not an error — the
    /// Invite is still spent, and the Kitchen is simply handed back.
    pub fn accept_kitchen_invite(&self, person_id: &str, secret: &str) -> Result<Value, OpError> {
        let hash = hash_secret(secret);
        self.db().with_conn(|conn| {
            let (invite_id, kitchen_id): (String, String) = conn
                .query_row(
                    "SELECT id, kitchen_id FROM kitchen_invites WHERE secret_hash = ?1 AND used_at IS NULL",
                    params![hash],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot resolve Kitchen Invite: {e}")))?
                .ok_or_else(|| {
                    OpError::unauthorized("this Invite does not name a live Kitchen invitation")
                })?;

            conn.execute(
                "UPDATE kitchen_invites SET used_at = strftime('%Y-%m-%dT%H:%M:%fZ','now'), used_by = ?1 WHERE id = ?2",
                params![person_id, invite_id],
            )
            .map_err(|e| OpError::internal(format!("cannot spend Kitchen Invite: {e}")))?;
            conn.execute(
                "INSERT OR IGNORE INTO kitchen_members (kitchen_id, person_id) VALUES (?1, ?2)",
                params![kitchen_id, person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot join Kitchen: {e}")))?;
            kitchen_summary(conn, &kitchen_id, person_id)
        })
    }

    /// Any member may remove another member, or leave by removing themselves
    /// (ADR 0041). A Kitchen holds no recipe, so there is nothing to strand:
    /// the last member may leave, and a Person may cook in no Kitchen at all.
    ///
    /// **Leaving takes the leaver's Cookbook out, and costs nobody a recipe
    /// they cooked** (#131, question 2). Each member who stays keeps a Branch,
    /// in their own Cookbook, of every recipe of the leaver's they cooked;
    /// and the leaver keeps one of every recipe of theirs they cooked. What
    /// they only read goes with the Cookbook it belongs to.
    pub fn remove_kitchen_member(
        &self,
        caller_person_id: &str,
        kitchen_id: &str,
        target_person_id: &str,
    ) -> Result<(), OpError> {
        self.db().with_conn(|conn| {
            let transaction = conn
                .unchecked_transaction()
                .map_err(|e| OpError::internal(format!("cannot begin: {e}")))?;
            let kept = leaving(conn, caller_person_id, kitchen_id, target_person_id)?;
            keep_branches(conn, &kept)?;
            transaction
                .commit()
                .map_err(|e| OpError::internal(format!("cannot commit: {e}")))?;
            Ok(())
        })
    }

    /// What removing `target_person_id` from a Kitchen would leave each side,
    /// before anybody does it: how many recipes the members who stay keep, and
    /// how many the one leaving keeps (#131, screen choice 4). Worked out by
    /// doing it and undoing it, so it cannot disagree with the real thing.
    pub fn preview_leaving_kitchen(
        &self,
        caller_person_id: &str,
        kitchen_id: &str,
        target_person_id: &str,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            conn.execute_batch("SAVEPOINT preview_leaving")
                .map_err(|e| OpError::internal(format!("cannot begin: {e}")))?;
            // Counted in recipes, as the sheet words them (#131, screen
            // choice 4): two members who both cooked one of yours, or one
            // recipe cooked in two Languages, is still one recipe. Read
            // before the rollback, while every Branch named still exists.
            let counted =
                leaving(conn, caller_person_id, kitchen_id, target_person_id).and_then(|kept| {
                    let mut theirs = std::collections::BTreeSet::new();
                    let mut yours = std::collections::BTreeSet::new();
                    for (person, branch_id) in kept {
                        let lineage: String = conn
                            .query_row(
                                "SELECT lineage_id FROM branches WHERE id = ?1",
                                params![branch_id],
                                |row| row.get(0),
                            )
                            .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?;
                        if person == target_person_id {
                            yours.insert(lineage);
                        } else {
                            theirs.insert(lineage);
                        }
                    }
                    Ok((theirs.len(), yours.len()))
                });
            conn.execute_batch("ROLLBACK TO preview_leaving; RELEASE preview_leaving")
                .map_err(|e| OpError::internal(format!("cannot undo the preview: {e}")))?;
            let (they_keep, you_keep) = counted?;
            Ok(json!({
                "they_keep": they_keep,
                "you_keep": you_keep,
            }))
        })
    }

    /// The Operator's one power over a Kitchen: deleting one nobody is left in.
    pub fn delete_kitchen(&self, caller_person_id: &str, kitchen_id: &str) -> Result<(), OpError> {
        self.db().with_conn(|conn| {
            let is_operator: bool = conn
                .query_row(
                    "SELECT is_operator FROM people WHERE id = ?1",
                    params![caller_person_id],
                    |row| row.get::<_, i64>(0),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot check Operator status: {e}")))?
                .map(|v| v != 0)
                .unwrap_or(false);
            if !is_operator {
                return Err(OpError::unauthorized(
                    "only an Operator may delete a Kitchen",
                ));
            }
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM kitchens WHERE id = ?1",
                    params![kitchen_id],
                    |row| row.get::<_, i64>(0),
                )
                .map(|n| n > 0)
                .map_err(|e| OpError::internal(format!("cannot look up Kitchen: {e}")))?;
            if !exists {
                return Err(OpError::not_found("no such Kitchen"));
            }
            let members: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM kitchen_members WHERE kitchen_id = ?1",
                    params![kitchen_id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot count Kitchen members: {e}")))?;
            if members > 0 {
                return Err(OpError::bad_request(
                    "a Kitchen may be deleted only once nobody is left in it",
                ));
            }
            conn.execute("DELETE FROM kitchens WHERE id = ?1", params![kitchen_id])
                .map_err(|e| OpError::internal(format!("cannot delete Kitchen: {e}")))?;
            Ok(())
        })
    }
}

pub(super) fn insert_kitchen_with_member(
    conn: &rusqlite::Connection,
    kitchen_id: &str,
    name: &str,
    person_id: &str,
) -> Result<(), OpError> {
    conn.execute(
        "INSERT INTO kitchens (id, name, hand_id) VALUES (?1, ?2, ?1)",
        params![kitchen_id, name],
    )
    .map_err(|e| OpError::internal(format!("cannot create Kitchen: {e}")))?;
    conn.execute(
        "INSERT INTO kitchen_members (kitchen_id, person_id) VALUES (?1, ?2)",
        params![kitchen_id, person_id],
    )
    .map_err(|e| OpError::internal(format!("cannot add Person to Kitchen: {e}")))?;
    Ok(())
}

/// Whether a Person cooks in a Kitchen.
pub(super) fn is_member(
    conn: &rusqlite::Connection,
    kitchen_id: &str,
    person_id: &str,
) -> Result<bool, OpError> {
    conn.query_row(
        "SELECT COUNT(*) FROM kitchen_members WHERE kitchen_id = ?1 AND person_id = ?2",
        params![kitchen_id, person_id],
        |row| row.get::<_, i64>(0),
    )
    .map(|n| n > 0)
    .map_err(|e| OpError::internal(format!("cannot check Kitchen membership: {e}")))
}

/// Only a member may change a Kitchen — the only circle that may (ADR 0007).
///
/// **For a Kitchen the caller named.** Being told you do not cook in a Kitchen
/// whose id you just supplied tells you nothing you did not already know. Where
/// Kamosu worked the Kitchen out from something else, use
/// [`ensure_member_or_absent`] instead (ADR 0040).
pub(super) fn ensure_member(
    conn: &rusqlite::Connection,
    kitchen_id: &str,
    person_id: &str,
) -> Result<(), OpError> {
    if is_member(conn, kitchen_id, person_id)? {
        Ok(())
    } else {
        Err(OpError::unauthorized(
            "this Person does not cook in this Kitchen",
        ))
    }
}

/// Take `target_person_id` out of a Kitchen, answering what everybody it
/// touches keeps for it: `(person, branch)`. Only a member may, and only a
/// member may be taken out.
fn leaving(
    conn: &Connection,
    caller_person_id: &str,
    kitchen_id: &str,
    target_person_id: &str,
) -> Result<Vec<(String, String)>, OpError> {
    ensure_member(conn, kitchen_id, caller_person_id)?;
    if !is_member(conn, kitchen_id, target_person_id)? {
        return Err(OpError::not_found(
            "that Person does not cook in this Kitchen",
        ));
    }
    let members: Vec<String> = {
        let mut statement = conn
            .prepare(
                "SELECT person_id FROM kitchen_members WHERE kitchen_id = ?1 ORDER BY person_id",
            )
            .map_err(|e| OpError::internal(format!("cannot list Kitchen members: {e}")))?;
        statement
            .query_map(params![kitchen_id], |row| row.get(0))
            .map_err(|e| OpError::internal(format!("cannot list Kitchen members: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot list Kitchen members: {e}")))?
    };
    keeps_after(
        conn,
        &members,
        |conn| {
            conn.execute(
                "DELETE FROM kitchen_members WHERE kitchen_id = ?1 AND person_id = ?2",
                params![kitchen_id, target_person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot remove Kitchen member: {e}")))?;
            Ok(())
        },
        |_| true,
    )
}

/// One Kitchen, as the Person asking sees it: their own Nickname (never
/// anyone else's), who else is in it, and whose Cookbooks it sees.
fn kitchen_summary(
    conn: &rusqlite::Connection,
    kitchen_id: &str,
    viewer_person_id: &str,
) -> Result<Value, OpError> {
    let name: String = conn
        .query_row(
            "SELECT name FROM kitchens WHERE id = ?1",
            params![kitchen_id],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot read Kitchen: {e}")))?;
    let nickname: Option<String> = conn
        .query_row(
            "SELECT nickname FROM kitchen_members WHERE kitchen_id = ?1 AND person_id = ?2",
            params![kitchen_id, viewer_person_id],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot read Nickname: {e}")))?;
    let mut statement = conn
        .prepare(
            "SELECT people.id, people.name FROM kitchen_members \
             JOIN people ON people.id = kitchen_members.person_id \
             WHERE kitchen_members.kitchen_id = ?1 ORDER BY people.name",
        )
        .map_err(|e| OpError::internal(format!("cannot list Kitchen members: {e}")))?;
    let members: Vec<Value> = statement
        .query_map(params![kitchen_id], |row| {
            Ok(json!({
                "person_id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
            }))
        })
        .map_err(|e| OpError::internal(format!("cannot list Kitchen members: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot list Kitchen members: {e}")))?;

    let cookbook_ids: Vec<String> = {
        let mut statement = conn
            .prepare(&cookbooks_seen_in("?1"))
            .map_err(|e| OpError::internal(format!("cannot list Cookbooks: {e}")))?;
        statement
            .query_map(params![kitchen_id], |row| row.get(0))
            .map_err(|e| OpError::internal(format!("cannot list Cookbooks: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot list Cookbooks: {e}")))?
    };
    let mut cookbooks: Vec<Value> = cookbook_ids
        .iter()
        .map(|cookbook_id| cookbook_label(conn, cookbook_id))
        .collect::<Result<_, _>>()?;
    // The viewer's own first, then by who writes it, so the list reads the
    // same on every visit.
    let own = cookbook_of_person(conn, viewer_person_id)?;
    cookbooks.sort_by_key(|cookbook| {
        (
            cookbook["id"] != json!(own),
            cookbook["authors"][0]["name"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        )
    });

    Ok(json!({
        "id": kitchen_id,
        "name": name,
        "nickname": nickname,
        "members": members,
        "cookbooks": cookbooks,
    }))
}
