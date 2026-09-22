//! Kitchens and their members, and the membership checks every Operation on
//! a Kitchen's recipes leans on.

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

    /// Any member may remove another member, or leave by removing themselves.
    /// Two invariants stand in the way: a Kitchen never drops to zero members,
    /// and a Person never drops to zero Kitchens (ADR 0007).
    pub fn remove_kitchen_member(
        &self,
        caller_person_id: &str,
        kitchen_id: &str,
        target_person_id: &str,
    ) -> Result<(), OpError> {
        self.db().with_conn(|conn| {
            ensure_member(conn, kitchen_id, caller_person_id)?;
            if !is_member(conn, kitchen_id, target_person_id)? {
                return Err(OpError::not_found(
                    "that Person does not cook in this Kitchen",
                ));
            }
            let kitchen_members: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM kitchen_members WHERE kitchen_id = ?1",
                    params![kitchen_id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot count Kitchen members: {e}")))?;
            if kitchen_members <= 1 {
                return Err(OpError::bad_request(
                    "the last member of a Kitchen cannot be removed — invite someone else, or delete it",
                ));
            }
            let person_kitchens: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM kitchen_members WHERE person_id = ?1",
                    params![target_person_id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot count that Person's Kitchens: {e}")))?;
            if person_kitchens <= 1 {
                return Err(OpError::bad_request(
                    "a Person cooks in one or more Kitchens and cannot be removed from their last one",
                ));
            }
            conn.execute(
                "DELETE FROM kitchen_members WHERE kitchen_id = ?1 AND person_id = ?2",
                params![kitchen_id, target_person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot remove Kitchen member: {e}")))?;
            // Leaving your own Home Kitchen hands the default landing spot to
            // the oldest Kitchen you still cook in — the one you have been in
            // longest, rather than an arbitrary remaining one.
            conn.execute(
                "UPDATE people SET home_kitchen_id = (
                    SELECT kitchen_members.kitchen_id FROM kitchen_members
                    JOIN kitchens ON kitchens.id = kitchen_members.kitchen_id
                    WHERE kitchen_members.person_id = ?1
                    ORDER BY kitchens.created_at ASC LIMIT 1
                 ) WHERE id = ?1 AND home_kitchen_id = ?2",
                params![target_person_id, kitchen_id],
            )
            .map_err(|e| OpError::internal(format!("cannot reassign Home Kitchen: {e}")))?;
            Ok(())
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

/// The Hand a Kitchen writes under.
pub(super) fn kitchen_hand(conn: &Connection, kitchen_id: &str) -> Result<String, OpError> {
    conn.query_row(
        "SELECT hand_id FROM kitchens WHERE id = ?1",
        params![kitchen_id],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot read Kitchen's Hand: {e}")))
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

/// The same membership check, **for a Kitchen Kamosu worked out** from a Branch,
/// a Tag, an Import or anything else the caller named (ADR 0040).
///
/// A non-member is refused with `absent` — the very refusal the caller would
/// have got had the thing they named not existed here at all. That is what
/// stops the refusal being an answer: holding an id, nobody can tell whether
/// another household on this instance holds the thing it names.
///
/// `absent` is the same function the not-found path calls, never a sentence
/// written out a second time. Identical text is the whole mechanism, and one
/// source for it is what keeps the two identical as either is edited.
pub(super) fn ensure_member_or_absent(
    conn: &rusqlite::Connection,
    kitchen_id: &str,
    person_id: &str,
    absent: impl FnOnce() -> OpError,
) -> Result<(), OpError> {
    if is_member(conn, kitchen_id, person_id)? {
        Ok(())
    } else {
        Err(absent())
    }
}

// ── Share Links: the pieces (#65) ────────────────────────────────────────────

/// The Kitchen holding a Branch — the circle allowed to share it (ADR 0007).
pub(super) fn branch_kitchen(conn: &Connection, branch_id: &str) -> Result<String, OpError> {
    conn.query_row(
        "SELECT kitchen_id FROM branches WHERE id = ?1",
        params![branch_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?
    .ok_or_else(no_such_branch)
}

/// One Kitchen, as the Person asking sees it: their own Nickname (never
/// anyone else's), whether it is their Home Kitchen, and who else is in it.
fn kitchen_summary(
    conn: &rusqlite::Connection,
    kitchen_id: &str,
    viewer_person_id: &str,
) -> Result<Value, OpError> {
    let (name, hand_id): (String, String) = conn
        .query_row(
            "SELECT name, hand_id FROM kitchens WHERE id = ?1",
            params![kitchen_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| OpError::internal(format!("cannot read Kitchen: {e}")))?;
    let nickname: Option<String> = conn
        .query_row(
            "SELECT nickname FROM kitchen_members WHERE kitchen_id = ?1 AND person_id = ?2",
            params![kitchen_id, viewer_person_id],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot read Nickname: {e}")))?;
    let home_kitchen_id: Option<String> = conn
        .query_row(
            "SELECT home_kitchen_id FROM people WHERE id = ?1",
            params![viewer_person_id],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot read Home Kitchen: {e}")))?;
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

    Ok(json!({
        "id": kitchen_id,
        "name": name,
        "hand_id": hand_id,
        "is_home": home_kitchen_id.as_deref() == Some(kitchen_id),
        "nickname": nickname,
        "members": members,
    }))
}
