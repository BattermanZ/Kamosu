//! People and how they get in: first setup, logins and Sessions, Access
//! Keys, invites, recovery links, a Person's Reading Language, and who is
//! Operator.

use super::*;

/// The answer to minting an Access Key: the raw Secret, shown once and never
/// stored, alongside what the Key is known by afterwards.
pub struct AccessKey {
    pub id: String,
    pub secret: String,
    pub name: String,
    pub read_only: bool,
}

impl Core {
    /// Mint an Access Key for a Person. Plumbing beneath both Doors and the
    /// terminal — obtaining a Credential is not an Operation. The raw Secret is
    /// returned once and stored only as its hash.
    pub fn mint_access_key(
        &self,
        person_id: &str,
        key_name: &str,
        read_only: bool,
    ) -> Result<AccessKey, OpError> {
        let key_name = required_text(key_name, "name")?.to_string();
        let secret = generate_secret();
        let hash = hash_secret(&secret);
        let id = format!("ak_{}", hex::encode(random_bytes(8)));
        self.db().with_conn(|conn| {
            let exists: bool = conn
                .query_row("SELECT COUNT(*) FROM people WHERE id = ?1", params![person_id], |r| r.get(0))
                .map(|n: i64| n > 0)
                .map_err(|e| OpError::internal(e.to_string()))?;
            if !exists {
                return Err(OpError::bad_request(format!("no Person '{person_id}'")));
            }
            conn.execute(
                "INSERT INTO access_keys (id, secret_hash, person_id, name, read_only) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, hash, person_id, key_name, read_only as i64],
            )
            .map_err(|e| OpError::internal(format!("cannot mint access key: {e}")))?;
            Ok(())
        })?;
        Ok(AccessKey {
            id,
            secret,
            name: key_name,
            read_only,
        })
    }

    /// Access Keys remain knowable by their name and last use until revoked;
    /// their Secret never appears in this record — only its one-time minting
    /// answer carries it.
    pub fn access_keys_of(&self, person_id: &str) -> Result<Vec<Value>, OpError> {
        self.db().with_conn(|conn| {
            let mut statement = conn.prepare(
                "SELECT id, name, read_only, created_at, last_used_at, revoked FROM access_keys WHERE person_id = ?1 ORDER BY created_at DESC",
            ).map_err(|e| OpError::internal(format!("cannot list Access Keys: {e}")))?;
            statement.query_map(params![person_id], |row| Ok(json!({
                "id": row.get::<_, String>(0)?, "name": row.get::<_, String>(1)?,
                "read_only": row.get::<_, i64>(2)? != 0,
                "created_at": row.get::<_, String>(3)?, "last_used_at": row.get::<_, Option<String>>(4)?,
                "revoked": row.get::<_, i64>(5)? != 0,
            }))).map_err(|e| OpError::internal(format!("cannot read Access Keys: {e}")))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| OpError::internal(format!("cannot read Access Keys: {e}")))
        })
    }

    pub fn revoke_access_key(&self, person_id: &str, key_id: &str) -> Result<(), OpError> {
        let changed = self.db().with_conn(|conn| {
            conn.execute(
                "UPDATE access_keys SET revoked = 1 WHERE id = ?1 AND person_id = ?2 AND revoked = 0",
                params![key_id, person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot revoke Access Key: {e}")))
        })?;
        if changed == 0 {
            return Err(OpError::not_found(
                "no live Access Key with that id belongs to this Person",
            ));
        }
        Ok(())
    }

    /// The fresh-instance door: one Person wins it, becomes the Operator, and
    /// receives the Kitchen and Hand that make a Person a complete account.
    pub fn create_first_person(
        &self,
        name: &str,
        password: &str,
        session_name: &str,
    ) -> Result<Value, OpError> {
        let name = required_text(name, "name")?;
        let password_hash = hash_password(password)?;
        let session_name = required_text(session_name, "session_name")?;
        let person_id = format!("p_{}", hex::encode(random_bytes(8)));
        let kitchen_id = format!("k_{}", hex::encode(random_bytes(8)));
        let session = Session {
            id: format!("s_{}", hex::encode(random_bytes(8))),
            secret: generate_secret(),
        };

        self.db().with_conn(|conn| {
            conn.execute_batch("BEGIN IMMEDIATE")
                .map_err(|e| OpError::internal(format!("cannot begin first-person setup: {e}")))?;
            let created = (|| -> Result<(), OpError> {
                if conn.execute(
                    "INSERT OR IGNORE INTO instance_setup(singleton) VALUES (1)",
                    [],
                ).map_err(|e| OpError::internal(format!("cannot reserve first-person setup: {e}")))? == 0 {
                    return Err(OpError::unauthorized("this instance already has its first Person"));
                }
                conn.execute(
                    "INSERT INTO people (id, name, password_hash, home_kitchen_id, is_operator) VALUES (?1, ?2, ?3, ?4, 1)",
                    params![person_id, name, password_hash, kitchen_id],
                ).map_err(|e| OpError::internal(format!("cannot create first Person: {e}")))?;
                conn.execute(
                    "UPDATE instance_setup SET operator_person_id = ?1 WHERE singleton = 1",
                    params![person_id],
                ).map_err(|e| OpError::internal(format!("cannot name first Operator: {e}")))?;
                conn.execute(
                    "INSERT INTO kitchens (id, name, hand_id) VALUES (?1, ?2, ?1)",
                    params![kitchen_id, format!("{}'s Home Kitchen", name)],
                ).map_err(|e| OpError::internal(format!("cannot create Home Kitchen: {e}")))?;
                conn.execute(
                    "INSERT INTO kitchen_members (kitchen_id, person_id) VALUES (?1, ?2)",
                    params![kitchen_id, person_id],
                ).map_err(|e| OpError::internal(format!("cannot add first Person to Home Kitchen: {e}")))?;
                conn.execute(
                    "INSERT INTO sessions (id, secret_hash, person_id, name) VALUES (?1, ?2, ?3, ?4)",
                    params![session.id, hash_secret(&session.secret), person_id, session_name],
                ).map_err(|e| OpError::internal(format!("cannot mint first Session: {e}")))?;
                Ok(())
            })();
            match created {
                Ok(()) => conn.execute_batch("COMMIT").map_err(|e| OpError::internal(format!("cannot finish first-person setup: {e}"))),
                Err(err) => {
                    let _ = conn.execute_batch("ROLLBACK");
                    Err(err)
                }
            }
        })?;

        Ok(json!({
            "person": {
                "id": person_id,
                "name": name,
                "hand_id": person_id,
                "home_kitchen_id": kitchen_id,
                "reading_language": "en",
                "reading_measures": "us",
                "is_operator": true,
            },
            "_session_secret": session.secret,
            "session_id": session.id,
        }))
    }

    /// Check a human-chosen password and mint the browser Session it unlocks.
    pub fn log_in(&self, name: &str, password: &str, session_name: &str) -> Result<Value, OpError> {
        let name = required_text(name, "name")?;
        let session_name = required_text(session_name, "session_name")?;
        self.throttle_login(name)?;
        let password_hash: Option<String> = self.db().with_conn(|conn| {
            conn.query_row(
                "SELECT password_hash FROM people WHERE name = ?1 AND disabled = 0 AND deleted = 0",
                params![name],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| OpError::internal(format!("cannot read Person for login: {e}")))
        })?;
        let verified = password_hash.as_deref().is_some_and(|encoded| {
            PasswordHash::new(encoded)
                .ok()
                .and_then(|parsed| {
                    Argon2::default()
                        .verify_password(password.as_bytes(), &parsed)
                        .ok()
                })
                .is_some()
        });
        if !verified {
            self.record_login_failure(name)?;
            return Err(OpError::unauthorized("that name and password do not match"));
        }
        self.db().with_conn(|conn| {
            conn.execute("DELETE FROM login_failures WHERE name = ?1", params![name])
                .map_err(|e| OpError::internal(format!("cannot clear login throttle: {e}")))?;
            Ok(())
        })?;
        let person_id: String = self.db().with_conn(|conn| {
            conn.query_row(
                "SELECT id FROM people WHERE name = ?1 AND disabled = 0 AND deleted = 0",
                params![name],
                |r| r.get(0),
            )
            .map_err(|e| OpError::internal(format!("cannot read logged-in Person: {e}")))
        })?;
        let session = self.mint_session(&person_id, session_name)?;
        Ok(json!({ "session_id": session.id, "_session_secret": session.secret }))
    }

    pub fn set_reading_preferences(
        &self,
        person_id: &str,
        language: &str,
        measures: &str,
    ) -> Result<(), OpError> {
        if !matches!(language, "en" | "fr" | "es")
            || !matches!(measures, "us" | "metric" | "as_written")
        {
            return Err(OpError::bad_request(
                "reading_language and reading_measures are not supported",
            ));
        }
        self.db().with_conn(|conn| {
            conn.execute(
                "UPDATE people SET reading_language = ?1, reading_measures = ?2 WHERE id = ?3",
                params![language, measures, person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot set reading preferences: {e}")))?;
            Ok(())
        })
    }

    /// How this Person reads: the Language they read Kamosu in and the measures
    /// they measure in. Held on the account rather than in a browser, so a cook
    /// who switches to metric on her phone finds the recipe in metric on the
    /// iPad on the worktop — and so an agent reading through the MCP door gets
    /// the same answer the interface shows.
    pub fn reading_preferences(&self, person_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            Ok(json!({
                "reading_language": reading_language_of(conn, person_id)?,
                "reading_measures": reading_measures_of(conn, person_id)?,
            }))
        })
    }

    /// A Person's name is a reminder, not their Hand's identity: records keep
    /// the permanent Person id and render this current value when read.
    pub fn rename_person(&self, person_id: &str, name: &str) -> Result<(), OpError> {
        let name = required_text(name, "name")?;
        self.db().with_conn(|conn| {
            conn.execute(
                "UPDATE people SET name = ?1 WHERE id = ?2",
                params![name, person_id],
            )
            .map_err(|e| OpError::bad_request(format!("cannot use that name: {e}")))?;
            Ok(())
        })
    }

    /// Sessions remain knowable by their device name and last use until their
    /// owner revokes them; their Secret never appears in this record.
    pub fn sessions_of(&self, person_id: &str) -> Result<Vec<Value>, OpError> {
        self.db().with_conn(|conn| {
            let mut statement = conn.prepare(
                "SELECT id, name, created_at, last_used_at, revoked FROM sessions WHERE person_id = ?1 ORDER BY created_at DESC",
            ).map_err(|e| OpError::internal(format!("cannot list Sessions: {e}")))?;
            statement.query_map(params![person_id], |row| Ok(json!({
                "id": row.get::<_, String>(0)?, "name": row.get::<_, String>(1)?,
                "created_at": row.get::<_, String>(2)?, "last_used_at": row.get::<_, Option<String>>(3)?,
                "revoked": row.get::<_, i64>(4)? != 0,
            }))).map_err(|e| OpError::internal(format!("cannot read Sessions: {e}")))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| OpError::internal(format!("cannot read Sessions: {e}")))
        })
    }

    pub fn revoke_session(&self, person_id: &str, session_id: &str) -> Result<(), OpError> {
        let changed = self.db().with_conn(|conn| {
            conn.execute(
                "UPDATE sessions SET revoked = 1 WHERE id = ?1 AND person_id = ?2 AND revoked = 0",
                params![session_id, person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot revoke Session: {e}")))
        })?;
        if changed == 0 {
            return Err(OpError::not_found(
                "no live Session by that name belongs to this Person",
            ));
        }
        Ok(())
    }

    fn mint_session(&self, person_id: &str, name: &str) -> Result<Session, OpError> {
        let secret = generate_secret();
        let id = format!("s_{}", hex::encode(random_bytes(8)));
        self.db().with_conn(|conn| {
            conn.execute(
                "INSERT INTO sessions (id, secret_hash, person_id, name) VALUES (?1, ?2, ?3, ?4)",
                params![id, hash_secret(&secret), person_id, name],
            )
            .map_err(|e| OpError::internal(format!("cannot mint Session: {e}")))?;
            Ok(())
        })?;
        Ok(Session { id, secret })
    }

    /// A wrong password slows only the next attempt for this name: two free
    /// misses, then 1, 2, 4… seconds capped at 30. A correct password deletes
    /// the counter, so a stranger cannot lock an account out (ADR 0031).
    fn throttle_login(&self, name: &str) -> Result<(), OpError> {
        let failures: i64 = self.db().with_conn(|conn| {
            conn.query_row(
                "SELECT consecutive_failures FROM login_failures WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )
            .optional()
            .map(|count| count.unwrap_or(0))
            .map_err(|e| OpError::internal(format!("cannot read login throttle: {e}")))
        })?;
        let seconds = if failures < 2 {
            0
        } else {
            1u64 << (failures - 2).min(5)
        };
        if seconds > 0 {
            std::thread::sleep(std::time::Duration::from_secs(seconds));
        }
        Ok(())
    }

    fn record_login_failure(&self, name: &str) -> Result<(), OpError> {
        self.db().with_conn(|conn| {
            conn.execute("INSERT INTO login_failures(name, consecutive_failures) VALUES (?1, 1) ON CONFLICT(name) DO UPDATE SET consecutive_failures = consecutive_failures + 1", params![name])
                .map_err(|e| OpError::internal(format!("cannot record login failure: {e}")))?;
            Ok(())
        })
    }

    pub fn mint_invite(&self, is_operator: bool) -> Result<String, OpError> {
        let secret = generate_secret();
        self.db().with_conn(|conn| {
            conn.execute("INSERT INTO account_links (secret_hash, kind, is_operator) VALUES (?1, 'invite', ?2)", params![hash_secret(&secret), is_operator as i64])
                .map_err(|e| OpError::internal(format!("cannot mint Invite: {e}")))?;
            Ok(())
        })?;
        Ok(format!("/invite/{secret}"))
    }

    pub fn redeem_invite(
        &self,
        link: &str,
        name: &str,
        password: &str,
        session_name: &str,
    ) -> Result<Value, OpError> {
        let secret = link
            .strip_prefix("/invite/")
            .filter(|s| !s.is_empty())
            .ok_or_else(|| OpError::bad_request("Invite link is invalid"))?;
        let name = required_text(name, "name")?;
        let password_hash = hash_password(password)?;
        let session_name = required_text(session_name, "session_name")?;
        let person_id = format!("p_{}", hex::encode(random_bytes(8)));
        let kitchen_id = format!("k_{}", hex::encode(random_bytes(8)));
        let session = Session {
            id: format!("s_{}", hex::encode(random_bytes(8))),
            secret: generate_secret(),
        };
        let operator = self.db().with_conn(|conn| {
            conn.execute_batch("BEGIN IMMEDIATE").map_err(|e| OpError::internal(e.to_string()))?;
            let result = (|| -> Result<bool, OpError> {
                let role: Option<i64> = conn.query_row("SELECT is_operator FROM account_links WHERE secret_hash=?1 AND kind='invite' AND spent=0 AND revoked=0", params![hash_secret(secret)], |r| r.get(0)).optional().map_err(|e| OpError::internal(e.to_string()))?;
                let role = role.ok_or_else(|| OpError::unauthorized("this Invite has already been spent or revoked"))?;
                conn.execute("UPDATE account_links SET spent=1 WHERE secret_hash=?1", params![hash_secret(secret)]).map_err(|e| OpError::internal(e.to_string()))?;
                conn.execute("INSERT INTO people (id,name,password_hash,home_kitchen_id,is_operator) VALUES (?1,?2,?3,?4,?5)", params![person_id,name,password_hash,kitchen_id,role]).map_err(|e| OpError::bad_request(e.to_string()))?;
                insert_kitchen_with_member(conn, &kitchen_id, &format!("{name}'s Home Kitchen"), &person_id)?;
                conn.execute("INSERT INTO sessions (id,secret_hash,person_id,name) VALUES (?1,?2,?3,?4)", params![session.id,hash_secret(&session.secret),person_id,session_name]).map_err(|e| OpError::internal(e.to_string()))?;
                Ok(role != 0)
            })();
            match result { Ok(v) => { conn.execute_batch("COMMIT").map_err(|e| OpError::internal(e.to_string()))?; Ok(v) }, Err(e) => { let _=conn.execute_batch("ROLLBACK"); Err(e) } }
        })?;
        Ok(
            json!({"person":{"id":person_id,"name":name,"hand_id":person_id,"home_kitchen_id":kitchen_id,"reading_language":"en","reading_measures":"us","is_operator":operator},"session_id":session.id,"_session_secret":session.secret}),
        )
    }

    pub fn mint_recovery_link(&self, name: &str) -> Result<String, OpError> {
        let secret = generate_secret();
        self.db().with_conn(|conn| {
            let person_id: Option<String> = conn.query_row("SELECT id FROM people WHERE name = ?1 AND disabled = 0 AND deleted = 0", params![required_text(name, "name")?], |r| r.get(0)).optional()
                .map_err(|e| OpError::internal(format!("cannot read Person for recovery: {e}")))?;
            let person_id = person_id.ok_or_else(|| OpError::not_found(format!("no active Person '{name}'")))?;
            conn.execute("INSERT INTO account_links (secret_hash, kind, person_id) VALUES (?1, 'recovery', ?2)", params![hash_secret(&secret), person_id])
                .map_err(|e| OpError::internal(format!("cannot mint recovery link: {e}")))?;
            Ok(())
        })?;
        Ok(format!("/recover/{secret}"))
    }

    pub fn redeem_recovery(
        &self,
        link: &str,
        password: &str,
        session_name: &str,
    ) -> Result<Value, OpError> {
        let secret = link
            .strip_prefix("/recover/")
            .filter(|s| !s.is_empty())
            .ok_or_else(|| OpError::bad_request("recovery link is invalid"))?;
        let password_hash = hash_password(password)?;
        let session_name = required_text(session_name, "session_name")?;
        let session = Session {
            id: format!("s_{}", hex::encode(random_bytes(8))),
            secret: generate_secret(),
        };
        self.db().with_conn(|conn| {
            conn.execute_batch("BEGIN IMMEDIATE").map_err(|e| OpError::internal(e.to_string()))?;
            let result = (|| -> Result<(), OpError> {
                let person_id: Option<String> = conn.query_row("SELECT people.id FROM account_links JOIN people ON people.id=account_links.person_id WHERE secret_hash=?1 AND kind='recovery' AND spent=0 AND revoked=0 AND disabled=0 AND deleted=0", params![hash_secret(secret)], |r| r.get(0)).optional().map_err(|e| OpError::internal(e.to_string()))?;
                let person_id=person_id.ok_or_else(|| OpError::unauthorized("this recovery link has already been spent or revoked"))?;
                conn.execute("UPDATE account_links SET spent=1 WHERE secret_hash=?1 AND spent=0",params![hash_secret(secret)]).map_err(|e| OpError::internal(e.to_string()))?;
                conn.execute("UPDATE people SET password_hash=?1 WHERE id=?2",params![password_hash,person_id]).map_err(|e| OpError::internal(e.to_string()))?;
                conn.execute("INSERT INTO sessions (id,secret_hash,person_id,name) VALUES (?1,?2,?3,?4)",params![session.id,hash_secret(&session.secret),person_id,session_name]).map_err(|e| OpError::internal(e.to_string()))?;
                Ok(())
            })();
            match result { Ok(()) => conn.execute_batch("COMMIT").map_err(|e| OpError::internal(e.to_string())), Err(e) => { let _=conn.execute_batch("ROLLBACK"); Err(e) } }
        })?;
        Ok(json!({"session_id":session.id,"_session_secret":session.secret}))
    }

    /// Who holds an account here (#103). Names only, with the two facts an
    /// Operator administers by: whether they administer too, and whether the
    /// account still opens. Nothing about what anybody cooks — ADR 0007's
    /// boundary is kept by this query answering no question about a recipe, an
    /// Attempt or a Kitchen, rather than by a screen choosing not to ask.
    ///
    /// Only password-bearing rows are people who can sign in — the unique
    /// index of migration 4 is on exactly those — so the list matches what
    /// `disable_account` and its siblings can act on by name.
    pub fn list_accounts(&self, caller_person_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let mut statement = conn
                .prepare(
                    "SELECT id, name, is_operator, disabled, created_at FROM people \
                      WHERE deleted = 0 AND password_hash IS NOT NULL \
                      ORDER BY is_operator DESC, name COLLATE NOCASE",
                )
                .map_err(|e| OpError::internal(format!("cannot list accounts: {e}")))?;
            let rows = statement
                .query_map([], |row| {
                    let id: String = row.get(0)?;
                    Ok(json!({
                        "name": row.get::<_, String>(1)?,
                        "is_operator": row.get::<_, i64>(2)? != 0,
                        "disabled": row.get::<_, i64>(3)? != 0,
                        "is_you": id == caller_person_id,
                        "created_at": row.get::<_, String>(4)?,
                    }))
                })
                .map_err(|e| OpError::internal(format!("cannot list accounts: {e}")))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| OpError::internal(format!("cannot list accounts: {e}")))?;
            Ok(json!({ "accounts": rows }))
        })
    }

    /// Make a Person an Operator, or stand them down (#103).
    ///
    /// Taken under `BEGIN IMMEDIATE` so the last-Operator guard cannot be
    /// raced: two Operators standing each other down at the same instant would
    /// both read "somebody else remains" and both be right, and the instance
    /// would end up with nobody.
    pub fn set_operator(&self, name: &str, is_operator: bool) -> Result<Value, OpError> {
        let name = required_text(name, "name")?;
        self.db().with_conn(|conn| {
            conn.execute_batch("BEGIN IMMEDIATE")
                .map_err(|e| OpError::internal(e.to_string()))?;
            let result = (|| -> Result<Value, OpError> {
                if !is_operator {
                    ensure_an_operator_remains(conn, name, "standing them down")?;
                }
                let changed = conn
                    .execute(
                        "UPDATE people SET is_operator = ?1 WHERE name = ?2 AND deleted = 0",
                        params![is_operator as i64, name],
                    )
                    .map_err(|e| {
                        OpError::internal(format!("cannot change who administers: {e}"))
                    })?;
                if changed == 0 {
                    return Err(OpError::not_found(format!("no active Person '{name}'")));
                }
                Ok(json!({ "name": name, "is_operator": is_operator }))
            })();
            match result {
                Ok(value) => {
                    conn.execute_batch("COMMIT")
                        .map_err(|e| OpError::internal(e.to_string()))?;
                    Ok(value)
                }
                Err(error) => {
                    let _ = conn.execute_batch("ROLLBACK");
                    Err(error)
                }
            }
        })
    }

    pub fn end_account(&self, name: &str, deleted: bool) -> Result<(), OpError> {
        let name = required_text(name, "name")?;
        self.db().with_conn(|conn| {
            conn.execute_batch("BEGIN IMMEDIATE").map_err(|e| OpError::internal(e.to_string()))?;
            let result = (|| -> Result<(), OpError> {
                // #103: an instance whose last Operator is ended can never
                // appoint another, because every Operation that could is one
                // only an Operator may call. Guarded here in the Core, where
                // both Doors inherit it, rather than in the screen that shows
                // the refusal.
                ensure_an_operator_remains(
                    conn,
                    name,
                    if deleted {
                        "deleting the account"
                    } else {
                        "disabling their account"
                    },
                )?;
                let changed = conn.execute("UPDATE people SET disabled = 1, deleted = CASE WHEN ?1 THEN 1 ELSE deleted END WHERE name = ?2 AND deleted = 0", params![deleted as i64, name])
                    .map_err(|e| OpError::internal(format!("cannot end account: {e}")))?;
                if changed == 0 { return Err(OpError::not_found(format!("no active Person '{name}'"))); }
                conn.execute("UPDATE sessions SET revoked = 1 WHERE person_id = (SELECT id FROM people WHERE name = ?1)", params![name]).map_err(|e| OpError::internal(e.to_string()))?;
                conn.execute("UPDATE access_keys SET revoked = 1 WHERE person_id = (SELECT id FROM people WHERE name = ?1)", params![name]).map_err(|e| OpError::internal(e.to_string()))?;
                if deleted { conn.execute("DELETE FROM kitchen_members WHERE person_id = (SELECT id FROM people WHERE name = ?1)", params![name]).map_err(|e| OpError::internal(e.to_string()))?; }
                Ok(())
            })();
            match result {
                Ok(()) => conn.execute_batch("COMMIT").map_err(|e| OpError::internal(e.to_string())),
                Err(error) => { let _ = conn.execute_batch("ROLLBACK"); Err(error) }
            }
        })
    }

    /// Create a Person with their Home Kitchen. Test plumbing uses this until
    /// Invites arrive; real account creation enters through `create_first_person`
    /// or an Invite. A Person on their own is a Kitchen of one (ADR 0007), so
    /// this never leaves a Person without one.
    pub fn create_person(&self, name: &str) -> Result<String, OpError> {
        let id = format!("p_{}", hex::encode(random_bytes(8)));
        let kitchen_id = format!("k_{}", hex::encode(random_bytes(8)));
        self.db().with_conn(|conn| {
            conn.execute(
                "INSERT INTO people (id, name, home_kitchen_id) VALUES (?1, ?2, ?3)",
                params![id, name, kitchen_id],
            )
            .map_err(|e| OpError::internal(format!("cannot create Person: {e}")))?;
            insert_kitchen_with_member(conn, &kitchen_id, &format!("{name}'s Home Kitchen"), &id)
        })?;
        Ok(id)
    }

    /// Whether setup has happened on this instance. Nothing sets it yet — the
    /// first-visitor-wins flow is a later ticket — so a fresh install says false.
    pub fn setup_complete(&self) -> Result<bool, OpError> {
        self.db().with_conn(|conn| {
            let setup: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM instance_setup WHERE singleton = 1)",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot read instance setup: {e}")))?;
            Ok(setup)
        })
    }

    pub fn mark_setup_complete(&self) -> Result<(), OpError> {
        self.db().with_conn(|conn| {
            conn.execute(
                "INSERT OR IGNORE INTO instance_setup(singleton) VALUES (1)",
                [],
            )
            .map_err(|e| OpError::internal(format!("cannot record setup flag: {e}")))?;
            Ok(())
        })
    }
}

/// Refuse an act that would leave this instance with no Operator (#103).
///
/// ADR 0007 puts it as *the last cannot be demoted*, and the same sentence has
/// to cover disabling and deleting: an Operator who cannot sign in administers
/// nothing. All three end at the same place, and it is a place with no way
/// back — every Operation that could appoint an Operator is one only an
/// Operator may call, so an instance that loses its last one cannot be given
/// another through either Door. The disk is the only remedy, which is precisely
/// the position ADR 0007 says a self-hosted app should be honest about rather
/// than walk its owner into.
///
/// `act` names what was being attempted, so the refusal reads as a sentence
/// about what they did rather than a rule number.
fn ensure_an_operator_remains(conn: &Connection, name: &str, act: &str) -> Result<(), OpError> {
    let is_operator: Option<bool> = conn
        .query_row(
            "SELECT is_operator FROM people WHERE name = ?1 AND deleted = 0",
            params![name],
            |row| row.get::<_, i64>(0).map(|held| held != 0),
        )
        .optional()
        .map_err(|e| {
            OpError::internal(format!("cannot read whether '{name}' is an Operator: {e}"))
        })?;
    // Somebody who is not an Operator, and somebody who is not here at all,
    // are both no threat to the last one. The second is left to the caller's
    // own "no active Person" answer rather than pre-empted here.
    if is_operator != Some(true) {
        return Ok(());
    }
    let others: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM people \
              WHERE is_operator = 1 AND disabled = 0 AND deleted = 0 AND name <> ?1",
            params![name],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot count this instance's Operators: {e}")))?;
    if others == 0 {
        return Err(OpError::bad_request(format!(
            "'{name}' is the only Operator this instance has, and {act} would leave it with \
             nobody able to administer it and no way to appoint anybody. Make somebody else \
             an Operator first."
        )));
    }
    Ok(())
}

/// A Person's own Reading Language — the first thing consulted whenever
/// something named per Language (a Tag, a Food) is shown back to them.
pub(super) fn reading_language_of(
    conn: &rusqlite::Connection,
    person_id: &str,
) -> Result<String, OpError> {
    conn.query_row(
        "SELECT reading_language FROM people WHERE id = ?1",
        params![person_id],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot read reading Language: {e}")))
}

/// Hash a Secret for storage/lookup. The Secret itself is never stored.
pub fn hash_secret(secret: &str) -> String {
    hex::encode(Sha256::digest(secret.as_bytes()))
}

/// 256 bits of randomness — every Secret in Kamosu is this size (ADR 0031).
pub fn generate_secret() -> String {
    hex::encode(random_bytes(32))
}

/// Passwords are short human-chosen secrets, not Secrets: Argon2id salts and
/// hashes them separately at the cost ADR 0031 names (20 MiB, roughly a tenth s).
fn hash_password(password: &str) -> Result<String, OpError> {
    let password = required_text(password, "password")?;
    let params = Params::new(20 * 1024, 2, 1, None)
        .map_err(|e| OpError::internal(format!("cannot configure password hashing: {e}")))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let salt = SaltString::encode_b64(&random_bytes(16))
        .map_err(|e| OpError::internal(format!("cannot generate password salt: {e}")))?;
    argon
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| OpError::internal(format!("cannot hash password: {e}")))
}
