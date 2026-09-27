//! People and how they get in: first setup, logins and Sessions, Access
//! Keys, invites, recovery links, a Person's Reading Language, and who is
//! Operator.

use std::sync::{Condvar, LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant};

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
    /// receives the Cookbook and Hand that make a Person a complete account.
    pub fn create_first_person(
        &self,
        name: &str,
        password: &str,
        session_name: &str,
    ) -> Result<Value, OpError> {
        let name = required_text(name, "name")?;
        let session_name = required_text(session_name, "session_name")?;
        // Refused before the hash, which is the costly part (#138). The
        // reservation below still decides it, so two racing requests still
        // make one Operator.
        if self.setup_complete()? {
            return Err(OpError::unauthorized(
                "this instance already has its first Person",
            ));
        }
        let password_hash = self.passwords.work.hash(new_password(password)?)?;
        let person_id = format!("p_{}", hex::encode(random_bytes(8)));
        let session = Session {
            id: format!("s_{}", hex::encode(random_bytes(8))),
            secret: generate_secret(),
        };

        let cookbook_id = self.db().with_conn(|conn| {
            conn.execute_batch("BEGIN IMMEDIATE")
                .map_err(|e| OpError::internal(format!("cannot begin first-person setup: {e}")))?;
            let created = (|| -> Result<String, OpError> {
                if conn.execute(
                    "INSERT OR IGNORE INTO instance_setup(singleton) VALUES (1)",
                    [],
                ).map_err(|e| OpError::internal(format!("cannot reserve first-person setup: {e}")))? == 0 {
                    return Err(OpError::unauthorized("this instance already has its first Person"));
                }
                conn.execute(
                    "INSERT INTO people (id, name, password_hash, is_operator) VALUES (?1, ?2, ?3, 1)",
                    params![person_id, name, password_hash],
                ).map_err(|e| OpError::internal(format!("cannot create first Person: {e}")))?;
                conn.execute(
                    "UPDATE instance_setup SET operator_person_id = ?1 WHERE singleton = 1",
                    params![person_id],
                ).map_err(|e| OpError::internal(format!("cannot name first Operator: {e}")))?;
                // Their own Cookbook, and no Kitchen: a Person cooking alone
                // needs none (ADR 0041).
                let cookbook_id = insert_person_cookbook(conn, &person_id)?;
                conn.execute(
                    "INSERT INTO sessions (id, secret_hash, person_id, name) VALUES (?1, ?2, ?3, ?4)",
                    params![session.id, hash_secret(&session.secret), person_id, session_name],
                ).map_err(|e| OpError::internal(format!("cannot mint first Session: {e}")))?;
                Ok(cookbook_id)
            })();
            match created {
                Ok(cookbook_id) => conn
                    .execute_batch("COMMIT")
                    .map(|()| cookbook_id)
                    .map_err(|e| OpError::internal(format!("cannot finish first-person setup: {e}"))),
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
                "cookbook_id": cookbook_id,
                "reading_language": "en",
                "reading_measures": "us",
                "is_operator": true,
            },
            "_session_secret": session.secret,
            "session_id": session.id,
        }))
    }

    /// Check a human-chosen password and mint the browser Session it unlocks.
    ///
    /// One try per name is checked at a time, and none while the name waits
    /// out its wrong passwords: those are refused at once as busy, with the
    /// seconds left (#138). Nothing here sleeps.
    pub fn log_in(&self, name: &str, password: &str, session_name: &str) -> Result<Value, OpError> {
        let name = required_text(name, "name")?;
        let session_name = required_text(session_name, "session_name")?;
        let turn = self.passwords.take_turn(name)?;
        let person: Option<(String, String)> = self.db().with_conn(|conn| {
            conn.query_row(
                "SELECT id, password_hash FROM people \
                  WHERE name = ?1 AND password_hash IS NOT NULL AND disabled = 0 AND deleted = 0",
                params![name],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| OpError::internal(format!("cannot read Person for login: {e}")))
        })?;
        let verified = self
            .passwords
            .work
            .verify(password, person.as_ref().map(|(_, hash)| hash.as_str()))?;
        let Some((person_id, _)) = person.filter(|_| verified) else {
            turn.missed(self.record_login_failure(name)?);
            return Err(OpError::unauthorized("that name and password do not match"));
        };
        self.db().with_conn(|conn| {
            conn.execute("DELETE FROM login_failures WHERE name = ?1", params![name])
                .map_err(|e| OpError::internal(format!("cannot clear login throttle: {e}")))?;
            Ok(())
        })?;
        turn.matched();
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
    ///
    /// It is also the name this Person signs in with, which is why two people
    /// here cannot hold one: migration 35's index refuses it, and the refusal
    /// is said in words rather than passed on as SQLite's. A deleted Person
    /// holds no name (#101), so a name they left behind may be taken.
    pub fn rename_person(&self, person_id: &str, name: &str) -> Result<String, OpError> {
        let name = required_text(name, "name")?;
        self.db().with_conn(|conn| {
            conn.execute(
                "UPDATE people SET name = ?1 WHERE id = ?2",
                params![name, person_id],
            )
            .map_err(|e| refusal_for_a_taken_name(e, name, "cannot rename this Person"))?;
            Ok(())
        })?;
        Ok(name.to_string())
    }

    /// Who this Credential names: the Person's permanent id, which is also
    /// their Hand, and the name they currently go by. What lets a screen say
    /// *you* without matching names, since a name is a reminder (ADR 0015).
    pub fn person(&self, person_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let name: String = conn
                .query_row(
                    "SELECT name FROM people WHERE id = ?1",
                    params![person_id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot read this Person: {e}")))?;
            Ok(json!({ "person_id": person_id, "name": name }))
        })
    }

    /// Sessions remain knowable by their device name and last use until their
    /// owner revokes them; their Secret never appears in this record. The one
    /// asking is marked `current`, so a Person signed in on three devices can
    /// tell which row is in their hand before they end one (#114).
    pub fn sessions_of(
        &self,
        person_id: &str,
        current: Option<&str>,
    ) -> Result<Vec<Value>, OpError> {
        self.db().with_conn(|conn| {
            let mut statement = conn.prepare(
                "SELECT id, name, created_at, last_used_at, revoked FROM sessions WHERE person_id = ?1 ORDER BY created_at DESC",
            ).map_err(|e| OpError::internal(format!("cannot list Sessions: {e}")))?;
            statement.query_map(params![person_id], |row| Ok(json!({
                "id": row.get::<_, String>(0)?, "name": row.get::<_, String>(1)?,
                "created_at": row.get::<_, String>(2)?, "last_used_at": row.get::<_, Option<String>>(3)?,
                "revoked": row.get::<_, i64>(4)? != 0,
                "current": current == Some(row.get::<_, String>(0)?.as_str()),
            }))).map_err(|e| OpError::internal(format!("cannot read Sessions: {e}")))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| OpError::internal(format!("cannot read Sessions: {e}")))
        })
    }

    /// A Session is named for its device when it is minted, from what the
    /// browser says about itself; this corrects a wrong guess, or names one
    /// minted before that, still called "this browser" (#114). An ended
    /// Session is not renamed: it is gone, not merely unnamed.
    pub fn rename_session(
        &self,
        person_id: &str,
        session_id: &str,
        name: &str,
    ) -> Result<String, OpError> {
        let name = required_text(name, "name")?;
        let changed = self.db().with_conn(|conn| {
            conn.execute(
                "UPDATE sessions SET name = ?1 WHERE id = ?2 AND person_id = ?3 AND revoked = 0",
                params![name, session_id, person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot rename Session: {e}")))
        })?;
        if changed == 0 {
            return Err(OpError::not_found(
                "no live Session with that id belongs to this Person",
            ));
        }
        Ok(name.to_string())
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

    /// Count one more wrong password for this name, and answer how many there
    /// have been since its last right one. A correct password deletes the
    /// count (ADR 0031).
    fn record_login_failure(&self, name: &str) -> Result<i64, OpError> {
        self.db().with_conn(|conn| {
            conn.query_row(
                "INSERT INTO login_failures(name, consecutive_failures) VALUES (?1, 1) \
                 ON CONFLICT(name) DO UPDATE SET consecutive_failures = consecutive_failures + 1 \
                 RETURNING consecutive_failures",
                params![name],
                |row| row.get(0),
            )
            .map_err(|e| OpError::internal(format!("cannot record login failure: {e}")))
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
        let session_name = required_text(session_name, "session_name")?;
        // Refused before the hash (#138); the transaction below looks again.
        if !self.link_is_live(secret, "invite")? {
            return Err(OpError::unauthorized(
                "this Invite has already been spent or revoked",
            ));
        }
        let password_hash = self.passwords.work.hash(new_password(password)?)?;
        let person_id = format!("p_{}", hex::encode(random_bytes(8)));
        let session = Session {
            id: format!("s_{}", hex::encode(random_bytes(8))),
            secret: generate_secret(),
        };
        let (operator, cookbook_id) = self.db().with_conn(|conn| {
            conn.execute_batch("BEGIN IMMEDIATE").map_err(|e| OpError::internal(e.to_string()))?;
            let result = (|| -> Result<(bool, String), OpError> {
                let role: Option<i64> = conn.query_row("SELECT is_operator FROM account_links WHERE secret_hash=?1 AND kind='invite' AND spent=0 AND revoked=0", params![hash_secret(secret)], |r| r.get(0)).optional().map_err(|e| OpError::internal(e.to_string()))?;
                let role = role.ok_or_else(|| OpError::unauthorized("this Invite has already been spent or revoked"))?;
                conn.execute("UPDATE account_links SET spent=1 WHERE secret_hash=?1", params![hash_secret(secret)]).map_err(|e| OpError::internal(e.to_string()))?;
                conn.execute("INSERT INTO people (id,name,password_hash,is_operator) VALUES (?1,?2,?3,?4)", params![person_id,name,password_hash,role]).map_err(|e| refusal_for_a_taken_name(e, name, "cannot create this Person"))?;
                let cookbook_id = insert_person_cookbook(conn, &person_id)?;
                conn.execute("INSERT INTO sessions (id,secret_hash,person_id,name) VALUES (?1,?2,?3,?4)", params![session.id,hash_secret(&session.secret),person_id,session_name]).map_err(|e| OpError::internal(e.to_string()))?;
                Ok((role != 0, cookbook_id))
            })();
            match result { Ok(v) => { conn.execute_batch("COMMIT").map_err(|e| OpError::internal(e.to_string()))?; Ok(v) }, Err(e) => { let _=conn.execute_batch("ROLLBACK"); Err(e) } }
        })?;
        Ok(
            json!({"person":{"id":person_id,"name":name,"hand_id":person_id,"cookbook_id":cookbook_id,"reading_language":"en","reading_measures":"us","is_operator":operator},"session_id":session.id,"_session_secret":session.secret}),
        )
    }

    /// Whether an Invite or recovery link can still be spent: unspent,
    /// unrevoked, and for recovery, its Person still active. Read before any
    /// password is hashed, so a wrong link costs a lookup and nothing more.
    fn link_is_live(&self, secret: &str, kind: &str) -> Result<bool, OpError> {
        self.db().with_conn(|conn| {
            conn.query_row(
                "SELECT 1 FROM account_links LEFT JOIN people ON people.id = account_links.person_id \
                  WHERE secret_hash = ?1 AND kind = ?2 AND spent = 0 AND revoked = 0 \
                    AND (kind = 'invite' OR (people.disabled = 0 AND people.deleted = 0))",
                params![hash_secret(secret), kind],
                |_| Ok(()),
            )
            .optional()
            .map(|found| found.is_some())
            .map_err(|e| OpError::internal(format!("cannot read this link: {e}")))
        })
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
        let session_name = required_text(session_name, "session_name")?;
        // Refused before the hash (#138); the transaction below looks again.
        if !self.link_is_live(secret, "recovery")? {
            return Err(OpError::unauthorized(
                "this recovery link has already been spent or revoked",
            ));
        }
        let password_hash = self.passwords.work.hash(new_password(password)?)?;
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
    /// index of migration 35 is on exactly those, less the deleted, so the
    /// list matches what `disable_account` and its siblings can act on by name.
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
                let person_id = undeleted_person_named(conn, name)?;
                if !is_operator {
                    ensure_an_operator_remains(conn, &person_id, name, "standing them down")?;
                }
                conn.execute(
                    "UPDATE people SET is_operator = ?1 WHERE id = ?2",
                    params![is_operator as i64, person_id],
                )
                .map_err(|e| OpError::internal(format!("cannot change who administers: {e}")))?;
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
                // #101: a deleted namesake may share this Name, so the Person
                // is found once and every write below is keyed on their id.
                let person_id = undeleted_person_named(conn, name)?;
                // #103: an instance whose last Operator is ended can never
                // appoint another, because every Operation that could is one
                // only an Operator may call. Guarded here in the Core, where
                // both Doors inherit it, rather than in the screen that shows
                // the refusal.
                ensure_an_operator_remains(
                    conn,
                    &person_id,
                    name,
                    if deleted {
                        "deleting the account"
                    } else {
                        "disabling their account"
                    },
                )?;
                conn.execute("UPDATE people SET disabled = 1, deleted = CASE WHEN ?1 THEN 1 ELSE deleted END WHERE id = ?2", params![deleted as i64, person_id])
                    .map_err(|e| OpError::internal(format!("cannot end account: {e}")))?;
                conn.execute("UPDATE sessions SET revoked = 1 WHERE person_id = ?1", params![person_id]).map_err(|e| OpError::internal(e.to_string()))?;
                conn.execute("UPDATE access_keys SET revoked = 1 WHERE person_id = ?1", params![person_id]).map_err(|e| OpError::internal(e.to_string()))?;
                if deleted {
                    forget_cookbook_of(conn, &person_id)?;
                } else {
                    // A disabled account is no longer asked about a waiting
                    // join, and may have been the last one it waited for (#135).
                    let cookbook_id = cookbook_of_person(conn, &person_id)?;
                    settle_joins_of(conn, &cookbook_id)?;
                }
                Ok(())
            })();
            match result {
                Ok(()) => conn.execute_batch("COMMIT").map_err(|e| OpError::internal(e.to_string())),
                Err(error) => { let _ = conn.execute_batch("ROLLBACK"); Err(error) }
            }
        })
    }

    /// Create a Person with their own Cookbook. Test plumbing uses this;
    /// real account creation enters through `create_first_person` or an
    /// Invite. Every Person has exactly one Cookbook (ADR 0041), so this never
    /// leaves a Person without one.
    pub fn create_person(&self, name: &str) -> Result<String, OpError> {
        let id = format!("p_{}", hex::encode(random_bytes(8)));
        self.db().with_conn(|conn| {
            conn.execute(
                "INSERT INTO people (id, name) VALUES (?1, ?2)",
                params![id, name],
            )
            .map_err(|e| OpError::internal(format!("cannot create Person: {e}")))?;
            insert_person_cookbook(conn, &id)?;
            Ok(())
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
/// about what they did rather than a rule number, and `name` is what the
/// Operator called them.
fn ensure_an_operator_remains(
    conn: &Connection,
    person_id: &str,
    name: &str,
    act: &str,
) -> Result<(), OpError> {
    let is_operator: bool = conn
        .query_row(
            "SELECT is_operator FROM people WHERE id = ?1",
            params![person_id],
            |row| row.get::<_, i64>(0).map(|held| held != 0),
        )
        .map_err(|e| {
            OpError::internal(format!("cannot read whether '{name}' is an Operator: {e}"))
        })?;
    // Somebody who is not an Operator is no threat to the last one.
    if !is_operator {
        return Ok(());
    }
    let others: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM people \
              WHERE is_operator = 1 AND disabled = 0 AND deleted = 0 AND id <> ?1",
            params![person_id],
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

/// The one Person an Operator means by `name`: the one with that name who has
/// not been deleted. Deleting frees a name (#101), so a deleted namesake may
/// sit beside them, and anything acting "on Marc" has to find this id once and
/// act on it rather than on the name again.
fn undeleted_person_named(conn: &Connection, name: &str) -> Result<String, OpError> {
    conn.query_row(
        "SELECT id FROM people WHERE name = ?1 AND deleted = 0",
        params![name],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read Person '{name}': {e}")))?
    .ok_or_else(|| OpError::not_found(format!("no active Person '{name}'")))
}

/// Turn a failed write to `people` into what the caller is told. A unique
/// constraint there can only be migration 35's index on sign-in names, so it
/// becomes a sentence saying somebody here already signs in as `name`, never
/// SQLite's own `UNIQUE constraint failed: people.name` (#101). A live or
/// disabled Person holds their name; a deleted one does not. Any other
/// failure, another kind of constraint included, is ours and not theirs.
fn refusal_for_a_taken_name(error: rusqlite::Error, name: &str, doing: &str) -> OpError {
    match &error {
        rusqlite::Error::SqliteFailure(failure, _)
            if failure.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE =>
        {
            OpError::bad_request(format!(
                "somebody here already signs in as {name}; choose another name"
            ))
        }
        _ => OpError::internal(format!("{doing}: {error}")),
    }
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
/// What deleting an account does to what it wrote (#131, question 11).
///
/// Its Cookbook leaves every Kitchen by the ordinary rule, so each Kitchen-mate
/// keeps a Branch, in their own Cookbook, of every recipe of it they cooked.
/// Then, where the Person wrote the Cookbook alone, its recipes go with them;
/// the Cookbook itself stays, named as it was, because Versions elsewhere
/// still carry its Hand. Where they wrote it with others, the others carry on
/// and the Person simply stops being one of its Co-authors, taking nothing.
fn forget_cookbook_of(conn: &Connection, person_id: &str) -> Result<(), OpError> {
    let mut touched: Vec<String> = {
        let mut statement = conn
            .prepare(
                "SELECT DISTINCT theirs.person_id FROM kitchen_members AS mine \
                   JOIN kitchen_members AS theirs ON theirs.kitchen_id = mine.kitchen_id \
                  WHERE mine.person_id = ?1",
            )
            .map_err(|e| OpError::internal(format!("cannot read Kitchens: {e}")))?;
        statement
            .query_map(params![person_id], |row| row.get(0))
            .map_err(|e| OpError::internal(format!("cannot read Kitchens: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| OpError::internal(format!("cannot read Kitchens: {e}")))?
    };
    touched.sort();
    let kept = keeps_after(
        conn,
        &touched,
        |conn| {
            conn.execute(
                "DELETE FROM kitchen_members WHERE person_id = ?1",
                params![person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot leave Kitchens: {e}")))?;
            Ok(())
        },
        |person| person != person_id,
    )?;
    keep_branches(conn, &kept)?;

    let cookbook_id = cookbook_of_person(conn, person_id)?;
    let alone: bool = conn
        .query_row(
            "SELECT COUNT(*) = 1 FROM cookbook_authors WHERE cookbook_id = ?1",
            params![cookbook_id],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot read Co-authors: {e}")))?;
    if alone {
        let name = cookbook_display_name(conn, &cookbook_id)?;
        conn.execute(
            "UPDATE cookbooks SET name = COALESCE(name, ?1) WHERE id = ?2",
            params![name, cookbook_id],
        )
        .map_err(|e| OpError::internal(format!("cannot keep the Cookbook's name: {e}")))?;
        let branches: Vec<String> = {
            let mut statement = conn
                .prepare("SELECT id FROM branches WHERE cookbook_id = ?1")
                .map_err(|e| OpError::internal(format!("cannot read Branches: {e}")))?;
            statement
                .query_map(params![cookbook_id], |row| row.get(0))
                .map_err(|e| OpError::internal(format!("cannot read Branches: {e}")))?
                .collect::<Result<_, _>>()
                .map_err(|e| OpError::internal(format!("cannot read Branches: {e}")))?
        };
        for branch_id in branches {
            delete_branch_rows(conn, &branch_id)?;
        }
    } else {
        conn.execute(
            "DELETE FROM cookbook_authors WHERE cookbook_id = ?1 AND person_id = ?2",
            params![cookbook_id, person_id],
        )
        .map_err(|e| OpError::internal(format!("cannot leave the Cookbook: {e}")))?;
    }
    // A join waiting on them waits no longer, and one they accepted is off
    // (#135).
    settle_joins_of(conn, &cookbook_id)?;
    Ok(())
}

pub fn hash_secret(secret: &str) -> String {
    hex::encode(Sha256::digest(secret.as_bytes()))
}

/// 256 bits of randomness — every Secret in Kamosu is this size (ADR 0031).
pub fn generate_secret() -> String {
    hex::encode(random_bytes(32))
}

/// The shortest password Kamosu accepts where one is set (#138): NIST SP
/// 800-63B-4 requires fifteen characters of a password that is the only
/// factor, and a Kamosu password is. Login never checks it, so a password set
/// before the minimum keeps working.
pub const PASSWORD_MINIMUM: usize = 15;

/// How many passwords may be hashed or checked at once (#138). Each is ~20 MiB
/// and roughly a tenth of a second of Argon2id (ADR 0031), so a burst of
/// sign-ins costs at most this many cores, and never a request worker.
const PASSWORD_WORKERS: usize = 2;

/// How many more may wait for one of those. A sign-in that finds every worker
/// busy **waits its turn**, because two people signing in at the same moment
/// is ordinary; one that finds this line full too is refused as busy, because
/// a line that grows without end is a crowd holding threads.
const PASSWORD_LINE: usize = 32;

/// The longest a name waits after its wrong passwords, per ADR 0031.
const LONGEST_WAIT: Duration = Duration::from_secs(30);

/// Password state that is not in the database: whose login tries are waiting
/// or being checked right now, and the few threads allowed to hash or check a
/// password.
#[derive(Default)]
pub(super) struct Passwords {
    names: Mutex<HashMap<String, NameGate>>,
    work: PasswordWork,
}

/// One name's place in the throttle (ADR 0031, #138).
///
/// Kept in memory, not in `login_failures`: the count of misses is durable,
/// but when this name may next be tried is not, so a restart lets one more try
/// through at the wait the count already earned. A restart is not something a
/// stranger at the door can cause.
struct NameGate {
    /// A try at this name is being checked now.
    checking: bool,
    /// The earliest the next try at this name is checked.
    open_at: Instant,
}

impl Passwords {
    /// The name gates. A panic while holding them leaves nothing half-written
    /// that matters, so a poisoned lock is simply taken.
    fn names(&self) -> std::sync::MutexGuard<'_, HashMap<String, NameGate>> {
        self.names.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Start making the stand-in hash at startup, so the first unknown name
    /// is not the one that pays for it and is slower than a known one. Made
    /// beside the startup rather than in it: nothing waits for it but a login.
    pub(super) fn prepare(&self) {
        std::thread::spawn(|| LazyLock::force(&STAND_IN_HASH));
    }

    /// Take this name's one turn to be checked, or be refused at once with
    /// the seconds left. At most one try per name is ever being checked, and
    /// none while it waits, so guesses sent together are no faster than
    /// guesses sent one at a time. The cost is the narrow lockout ADR 0031
    /// accepts: someone hammering one exact name keeps its sign-in refused
    /// while they do.
    fn take_turn<'a>(&'a self, name: &str) -> Result<Turn<'a>, OpError> {
        let mut names = self.names();
        let now = Instant::now();
        if let Some(gate) = names.get_mut(name) {
            if gate.checking {
                // Said apart from a wait, since this name may have no wrong
                // password against it at all. The screen says the wait's words
                // for both: the second of two taps a tenth of a second apart
                // is the only way a browser meets this.
                return Err(OpError::wait(
                    1,
                    "another try at this name is being checked — try again in 1 s",
                ));
            }
            if gate.open_at > now {
                let seconds = whole_seconds(gate.open_at - now);
                return Err(OpError::wait(
                    seconds,
                    format!("too many wrong passwords for this name — try again in {seconds} s"),
                ));
            }
            gate.checking = true;
        } else {
            // A crowd of made-up names would otherwise grow this forever; a
            // gate whose wait is over holds nothing worth keeping.
            if names.len() >= 1024 {
                names.retain(|_, gate| gate.checking || gate.open_at > now);
            }
            names.insert(
                name.to_string(),
                NameGate {
                    checking: true,
                    open_at: now,
                },
            );
        }
        Ok(Turn {
            passwords: self,
            name: name.to_string(),
        })
    }
}

/// A name's turn at being checked. Ended by `missed` or `matched`; dropped
/// without either (the database failed, say), the name is free again at the
/// wait it had.
struct Turn<'a> {
    passwords: &'a Passwords,
    name: String,
}

impl Turn<'_> {
    /// A wrong password: this name waits before its next try, as long as the
    /// misses so far have earned. Two are free, then 1, 2, 4, 8, 16 and 30
    /// seconds (ADR 0031).
    fn missed(self, failures: i64) {
        let wait = if failures < 2 {
            Duration::ZERO
        } else {
            Duration::from_secs(1 << (failures - 2).min(5)).min(LONGEST_WAIT)
        };
        let mut names = self.passwords.names();
        if let Some(gate) = names.get_mut(&self.name) {
            gate.open_at = Instant::now() + wait;
        }
    }

    /// The right password: the name owes nothing.
    fn matched(self) {
        let mut names = self.passwords.names();
        names.remove(&self.name);
    }
}

impl Drop for Turn<'_> {
    fn drop(&mut self) {
        let mut names = self.passwords.names();
        if let Some(gate) = names.get_mut(&self.name) {
            gate.checking = false;
        }
    }
}

/// Seconds to say, rounded up, and never zero: a refusal that says "try again
/// in 0 s" invites the very try it just refused.
fn whole_seconds(left: Duration) -> u64 {
    left.as_secs() + u64::from(left.subsec_nanos() > 0)
}

/// The threads allowed to hash or check a password: `PASSWORD_WORKERS` at
/// once, and a line of `PASSWORD_LINE` behind them. The web door calls
/// sign-in on the blocking pool, so waiting here holds no request worker.
#[derive(Default)]
struct PasswordWork {
    line: Mutex<PasswordLine>,
    freed: Condvar,
}

#[derive(Default)]
struct PasswordLine {
    running: usize,
    waiting: usize,
    /// Passwords hashed and passwords checked since this Core opened. Read
    /// only by the tests, which prove with them that a refused sign-in did no
    /// password work and an unknown name did the same work as a known one.
    hashed: u64,
    verified: u64,
}

impl PasswordWork {
    fn run<T>(
        &self,
        count: impl FnOnce(&mut PasswordLine),
        work: impl FnOnce() -> T,
    ) -> Result<T, OpError> {
        let mut line = self.line.lock().unwrap_or_else(PoisonError::into_inner);
        if line.running >= PASSWORD_WORKERS {
            if line.waiting >= PASSWORD_LINE {
                return Err(OpError::busy());
            }
            line.waiting += 1;
            while line.running >= PASSWORD_WORKERS {
                line = self
                    .freed
                    .wait(line)
                    .unwrap_or_else(PoisonError::into_inner);
            }
            line.waiting -= 1;
        }
        line.running += 1;
        count(&mut line);
        drop(line);
        // Released on the way out whatever happens, a panic in Argon2 included,
        // or one crash would shrink the pool for good.
        struct Release<'a>(&'a PasswordWork);
        impl Drop for Release<'_> {
            fn drop(&mut self) {
                let mut line = self.0.line.lock().unwrap_or_else(PoisonError::into_inner);
                line.running -= 1;
                self.0.freed.notify_one();
            }
        }
        let _release = Release(self);
        Ok(work())
    }

    /// Salt and hash a new password.
    fn hash(&self, password: &str) -> Result<String, OpError> {
        self.run(|line| line.hashed += 1, || argon2_hash(password))?
    }

    /// Whether `password` matches `encoded`. With no hash to check against,
    /// because no Person has this name, it checks against a stand-in hashed
    /// the same way and answers no: an unknown name costs what a known one
    /// does, so the time taken does not say who has an account here
    /// (ADR 0031).
    fn verify(&self, password: &str, encoded: Option<&str>) -> Result<bool, OpError> {
        self.run(
            |line| line.verified += 1,
            || {
                let stand_in = STAND_IN_HASH
                    .as_deref()
                    .map_err(|e| OpError::internal(e.clone()))?;
                // A stored hash that cannot be read matches nothing, as it
                // always has: it is a wrong password, counted like one.
                let real = encoded.and_then(|encoded| PasswordHash::new(encoded).ok());
                let checked = match &real {
                    Some(parsed) => parsed.clone(),
                    None => PasswordHash::new(stand_in).map_err(|e| {
                        OpError::internal(format!("cannot read the stand-in hash: {e}"))
                    })?,
                };
                let matched = Argon2::default()
                    .verify_password(password.as_bytes(), &checked)
                    .is_ok();
                Ok(matched && real.is_some())
            },
        )?
    }
}

/// What an unknown name's password is checked against. Made once, at the same
/// cost as every real one, of a password nobody chose.
static STAND_IN_HASH: LazyLock<Result<String, String>> =
    LazyLock::new(|| argon2_hash(&generate_secret()).map_err(|e| e.message));

/// Argon2id at the cost ADR 0031 names: 20 MiB, two passes, roughly a tenth of
/// a second. Passwords are short human-chosen secrets, not Secrets, so each is
/// salted and hashed on its own. Called only through `PasswordWork`.
fn argon2_hash(password: &str) -> Result<String, OpError> {
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

/// A password being set, checked for its length first: that is free, and the
/// hash is not.
fn new_password(password: &str) -> Result<&str, OpError> {
    let password = required_text(password, "password")?;
    if password.chars().count() < PASSWORD_MINIMUM {
        return Err(OpError::bad_request(format!(
            "a password needs at least {PASSWORD_MINIMUM} characters"
        )));
    }
    Ok(password)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Core on a fresh database of its own, with nobody in it yet.
    fn a_core() -> (tempfile::TempDir, Core) {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Db::open(dir.path()).unwrap());
        (dir, Core::open(db))
    }

    /// Passwords hashed and checked so far: how a test sees password work
    /// without timing it, since a timed test flakes on a busy machine.
    fn work(core: &Core) -> (u64, u64) {
        let line = core.passwords.work.line.lock().unwrap();
        (line.hashed, line.verified)
    }

    const PASSWORD: &str = "a password only its person knows";

    /// Every door that sets a password refuses a request it will refuse
    /// anyway before hashing anything (#138): first-person on an instance
    /// already set up, and an Invite or recovery link that is wrong or spent.
    #[test]
    fn a_refused_door_hashes_nothing() {
        let (_dir, core) = a_core();
        core.create_first_person("cook", PASSWORD, "laptop")
            .unwrap();
        assert_eq!(work(&core), (1, 0));

        let refused = core
            .create_first_person("stranger", PASSWORD, "x")
            .unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Unauthorized, "{refused}");
        let refused = core
            .redeem_invite("/invite/not-a-real-one", "stranger", PASSWORD, "x")
            .unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Unauthorized, "{refused}");
        let refused = core
            .redeem_recovery("/recover/not-a-real-one", PASSWORD, "x")
            .unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Unauthorized, "{refused}");

        // A spent link is refused unhashed too.
        let invite = core.mint_invite(false).unwrap();
        core.redeem_invite(&invite, "Marie", PASSWORD, "x").unwrap();
        assert_eq!(work(&core), (2, 0));
        let refused = core
            .redeem_invite(&invite, "Paul", PASSWORD, "x")
            .unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Unauthorized, "{refused}");
        assert_eq!(work(&core), (2, 0), "no refusal above hashed a password");
    }

    /// A name nobody holds is checked against a password exactly as a real
    /// one is, so the time a wrong answer takes does not say who has an
    /// account here (ADR 0031, #138).
    #[test]
    fn an_unknown_name_is_checked_like_a_known_one() {
        let (_dir, core) = a_core();
        core.create_first_person("cook", PASSWORD, "laptop")
            .unwrap();
        let before = work(&core).1;
        core.log_in("cook", "not the password", "x").unwrap_err();
        assert_eq!(work(&core).1, before + 1, "a known name is checked once");
        let refused = core
            .log_in("nobody here", "not the password", "x")
            .unwrap_err();
        assert_eq!(
            work(&core).1,
            before + 2,
            "an unknown name is checked once too"
        );
        assert_eq!(refused.message, "that name and password do not match");
        // And the stand-in never lets anyone in, whatever they type.
        core.log_in("nobody at all", "", "x").unwrap_err();
    }

    /// The minimum is for a password being set. One set before it existed
    /// still signs in, because login never measures a password (#138).
    #[test]
    fn a_short_password_set_before_the_minimum_still_signs_in() {
        let (_dir, core) = a_core();
        core.create_first_person("cook", PASSWORD, "laptop")
            .unwrap();
        let old = argon2_hash("short").unwrap();
        core.db()
            .with_conn(|conn| {
                conn.execute("UPDATE people SET password_hash = ?1", params![old])
                    .map_err(|e| OpError::internal(e.to_string()))
            })
            .unwrap();
        core.log_in("cook", "short", "phone").unwrap();
    }

    /// A stored hash that cannot be read is a wrong password, counted and
    /// checked like one, not a failure of Kamosu's own.
    #[test]
    fn an_unreadable_hash_is_a_wrong_password() {
        let (_dir, core) = a_core();
        core.create_first_person("cook", PASSWORD, "laptop")
            .unwrap();
        core.db()
            .with_conn(|conn| {
                conn.execute("UPDATE people SET password_hash = 'not a hash'", [])
                    .map_err(|e| OpError::internal(e.to_string()))
            })
            .unwrap();
        let refused = core.log_in("cook", PASSWORD, "phone").unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Unauthorized, "{refused}");
        assert_eq!(work(&core).1, 1);
    }

    /// Never "try again in 0 s": a part of a second left is a whole one.
    #[test]
    fn the_wait_is_said_in_whole_seconds_rounded_up() {
        assert_eq!(whole_seconds(Duration::from_millis(1)), 1);
        assert_eq!(whole_seconds(Duration::from_secs(2)), 2);
        assert_eq!(whole_seconds(Duration::from_millis(2001)), 3);
    }
}
