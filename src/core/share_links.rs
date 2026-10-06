//! Share Links: minting, ending and reading one, the public address, and
//! the page a stranger holding one sees.

use super::*;

impl Core {
    // ── Share Links (#65, ADR 0026, ADR 0018) ─────────────────────────────
    //
    // There are exactly two levels of visibility: a recipe is seen by its
    // Kitchen, or by anyone holding its Share Link. So there is no visibility
    // setting to read or write anywhere below — a live row in `share_links`
    // *is* the second level.

    /// Turn a recipe's Share Link on, and answer the link.
    ///
    /// Idempotent: a recipe already shared answers the link it already has,
    /// because "share this" asked twice is one intention, not two. Minting a
    /// second live link would be indistinguishable from the first and would
    /// leave the earlier one working, which is precisely what ADR 0018 forbids
    /// — a link that is ended must stay dead.
    ///
    /// `address` is the instance's public address, offered here because the
    /// first Share Link is the first moment Kamosu has any reason to know it
    /// (#65). It is stored once and then ignored on every later call.
    pub fn share_recipe(
        &self,
        person_id: &str,
        branch_id: &str,
        address: Option<&str>,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            ensure_sees_branch(conn, branch_id, person_id)?;

            if let Some(offered) = address {
                let offered = normalise_public_address(offered)?;
                if stored_public_address(conn)?.is_none() {
                    store_public_address(conn, &offered)?;
                }
            }

            if let Some(live) = live_share_link(conn, branch_id)? {
                return share_link_summary(conn, Some(live));
            }

            // The address is required at the first Share Link and never
            // after, because a link is stored as a token: this instance goes
            // on serving every link it ever minted whatever address reaches
            // it. That is not the same as a link already *sent* following a
            // change of address — it cannot, and `set_public_address` says why
            // (#103).
            if stored_public_address(conn)?.is_none() {
                return Err(OpError::bad_request(
                    "this instance has no public address yet, and a Share Link needs one to be \
                     an address somebody can open. Send it with this Operation as \
                     `public_address` — it is stored once, and every link minted \
                     afterwards is built against it.",
                ));
            }

            let secret = generate_secret();
            let id = format!("sl_{}", hex::encode(random_bytes(8)));
            // The Secret is kept beside its hash, so the share screen can show
            // the address for as long as the link lives (#171, ADR 0031 as
            // amended). A visitor is still looked up by the hash alone, which
            // is what keeps a link minted before the Secret was kept opening.
            conn.execute(
                "INSERT INTO share_links (id, branch_id, secret_hash, secret, shared_by) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, branch_id, hash_secret(&secret), secret, person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot mint a Share Link: {e}")))?;

            let live = live_share_link(conn, branch_id)?;
            share_link_summary(conn, live)
        })
    }

    /// End a recipe's Share Link. Permanent: the row stays, dead, and turning
    /// sharing back on mints a new one (ADR 0018).
    pub fn end_share_link(&self, person_id: &str, branch_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            ensure_sees_branch(conn, branch_id, person_id)?;
            conn.execute(
                "UPDATE share_links SET ended_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
                  WHERE branch_id = ?1 AND ended_at IS NULL",
                params![branch_id],
            )
            .map_err(|e| OpError::internal(format!("cannot end the Share Link: {e}")))?;
            share_link_summary(conn, None)
        })
    }

    /// What the share screen reads: whether this recipe is shared, and where.
    pub fn get_share_link(&self, person_id: &str, branch_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            ensure_sees_branch(conn, branch_id, person_id)?;
            let live = live_share_link(conn, branch_id)?;
            share_link_summary(conn, live)
        })
    }

    /// A Photograph on a shared recipe, for a reader holding the token and no
    /// Credential at all.
    ///
    /// The check is here rather than in the route, like every other check in
    /// Kamosu: a live token is a key to **this recipe's** pictures and not to
    /// the instance's. What counts as this recipe's is its Main Photo and the
    /// Photograph on any of its Steps, on the Version being shown — which is
    /// exactly the set the page can put an `<img>` around.
    pub fn read_shared_photograph(
        &self,
        token: &str,
        hash: &str,
        size: photographs::DisplaySize,
    ) -> Result<Vec<u8>, OpError> {
        let shared = self.read_shared_recipe(token)?;
        if shared["ended"].as_bool().unwrap_or(false) {
            return Err(OpError::not_found("no such Share Link"));
        }
        let carried = std::iter::once(&shared["recipe"])
            .chain(shared["translations"].as_array().into_iter().flatten())
            .any(|version| version_shows_photograph(version, hash));
        if !carried {
            return Err(OpError::not_found(
                "this Photograph is not on this shared Recipe",
            ));
        }
        self.display_copy_bytes(hash, size)
    }

    /// The picture a messaging app shows for a Share Link (#65).
    ///
    /// **Rendered once and kept** (ADR 0032). This is the one piece of real
    /// work a stranger can cause on this instance: sending a link into a busy
    /// group chat asks a dozen clients for the same picture at once, and
    /// rasterising a 1200×628 card each time is work that scales with the
    /// strangers rather than with the library.
    ///
    /// The cache is keyed by **Version id**, which is exactly right and needs
    /// no sweeping: a Version is named by a fingerprint of its own words and
    /// Photographs (ADR 0004), so the card for one can never need redrawing,
    /// and a recipe edited after being shared simply asks for a different key.
    /// Losing the directory costs one redraw.
    ///
    /// It draws from the already-resized Display Copy rather than a camera
    /// original, so even the first miss is bounded work.
    pub fn shared_card(&self, token: &str) -> Result<Vec<u8>, OpError> {
        let shared = self.read_shared_recipe(token)?;
        if shared["ended"].as_bool().unwrap_or(false) {
            return Err(OpError::not_found("no such Share Link"));
        }
        let recipe = &shared["recipe"];
        let version_id = recipe["version_id"].as_str().unwrap_or_default();
        let cached = self
            .data_dir()
            .join("cards")
            .join(format!("{version_id}.png"));
        if let Ok(bytes) = std::fs::read(&cached) {
            return Ok(bytes);
        }

        let title = recipe["content"]["title"].as_str().unwrap_or("");
        let lineage_id = recipe["lineage_id"].as_str().unwrap_or("");
        let photograph = match recipe["content"]["main_photo"].as_str() {
            Some(hash) => Some(self.display_copy_bytes(hash, photographs::DisplaySize::Print)?),
            None => None,
        };
        let drawn = crate::share_card::draw(
            title,
            photograph.as_deref(),
            &crate::cover::cover_for(lineage_id),
        )?;

        // A card that cannot be cached is still a card: the drawing succeeded,
        // and refusing to serve it because a directory is unwritable would
        // turn a disk problem into a broken link in somebody's chat.
        if let Some(directory) = cached.parent()
            && std::fs::create_dir_all(directory).is_ok()
        {
            let _ = std::fs::write(&cached, &drawn);
        }
        Ok(drawn)
    }

    /// What is stored, or `null` where nothing is (#103). Null is an ordinary
    /// answer rather than an error: an instance that has never minted a Share
    /// Link has never been asked, and that is the state most instances are in.
    pub fn get_public_address(&self) -> Result<Value, OpError> {
        self.db()
            .with_conn(|conn| Ok(json!({ "public_address": stored_public_address(conn)? })))
    }

    /// The instance's public address, set by the Operator.
    ///
    /// Separate from `share_recipe`'s one-time offer because moving an instance
    /// to a new address is a deliberate act taken long after the first link.
    ///
    /// **It fixes the future, not the past** (#103). This said the opposite
    /// until then — that every link already minted "must follow it, which is
    /// exactly what storing a token rather than a URL buys" — and that is
    /// false. A Share Link is `<address>/s/<secret>`, and a visitor is looked
    /// up by the secret's hash alone (`read_shared_recipe`), the address never
    /// entering the lookup. So a link already sent is a string in somebody
    /// else's phone that nothing here can reach, and if the old address stops
    /// resolving it is dead. What storing a token rather than a URL buys is
    /// that the *instance* keeps serving every link it ever minted, at whatever
    /// address reaches it — not that a link already handed out changes its
    /// own text.
    ///
    /// What does change is the address the share screen shows. Since #171 a
    /// link keeps its secret, and `share_link_summary` builds the URL against
    /// the address stored now, so after a move the screen offers the address
    /// that opens, ready to send again. That is intended (ADR 0031).
    pub fn set_public_address(&self, address: &str) -> Result<Value, OpError> {
        let address = normalise_public_address(address)?;
        self.db().with_conn(|conn| {
            store_public_address(conn, &address)?;
            Ok(json!({ "public_address": address }))
        })
    }

    /// Everything the public Share Link page shows, for a stranger holding a
    /// token and no Credential at all.
    ///
    /// **Never any Attempt** (ADR 0005, ADR 0026): not a rating, not a note,
    /// not an Attempt photograph. That is not a filter applied here so much as
    /// a shape — nothing below reads the `attempts` table.
    ///
    /// The chain is complete back to the first Version, names and *what
    /// changed* lines included, because a share cannot be made to start
    /// part-way along (ADR 0018). There is no parameter here that could
    /// shorten it.
    pub fn read_shared_recipe(&self, token: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let found: Option<(String, String, String, Option<String>)> = conn
                .query_row(
                    "SELECT id, branch_id, shared_by, ended_at FROM share_links \
                      WHERE secret_hash = ?1",
                    params![hash_secret(token)],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read the Share Link: {e}")))?;

            // A token nobody ever minted is not found. A token whose link was
            // *ended* is a different answer and is deliberately not an error:
            // somebody holding a real link that has since been withdrawn is
            // owed "this was ended", because "no such page" reads as a mistake
            // they might keep retrying. Enumeration is answered by the size of
            // the Secret (ADR 0031, ADR 0032), never by being coy about a
            // token the asker demonstrably already holds.
            let Some((share_id, branch_id, shared_by, ended_at)) = found else {
                return Err(OpError::not_found("no such Share Link"));
            };
            if ended_at.is_some() {
                return Ok(json!({
                    "ended": true,
                    "share_id": share_id,
                    "shared_by": Value::Null,
                    "public_address": stored_public_address(conn)?,
                    "recipe": Value::Null,
                    "translations": [],
                    "thread": [],
                }));
            }

            let shared = shared_branch(conn, &branch_id)?;
            Ok(json!({
                "ended": false,
                "share_id": share_id,
                // The page says who shared it, and a Person is who that is —
                // never the Kitchen. The name is looked up live, so renaming
                // yourself reaches every link you have ever minted at once
                // (GLOSSARY.md, "Hand").
                "shared_by": person_name(conn, &shared_by)?,
                "public_address": stored_public_address(conn)?,
                "recipe": shared["recipe"].clone(),
                "translations": shared["translations"].clone(),
                "thread": shared["thread"].clone(),
            }))
        })
    }
}

/// Write the instance's public address.
///
/// An upsert rather than an `UPDATE`, because `instance_setup` holds at most
/// one row and there is no guarantee it exists yet: the row is written when a
/// first Person claims the instance, and an address can be set before that —
/// an `UPDATE` would then quietly change nothing and the first Share Link would
/// have no address to render against.
fn store_public_address(conn: &Connection, address: &str) -> Result<(), OpError> {
    conn.execute(
        "INSERT INTO instance_setup (singleton, public_address) VALUES (1, ?1)          ON CONFLICT(singleton) DO UPDATE SET public_address = excluded.public_address",
        params![address],
    )
    .map_err(|e| OpError::internal(format!("cannot store the public address: {e}")))?;
    Ok(())
}

/// The instance's public address, or nothing if no Share Link has needed one.
fn stored_public_address(conn: &Connection) -> Result<Option<String>, OpError> {
    conn.query_row(
        "SELECT public_address FROM instance_setup WHERE singleton = 1",
        [],
        |row| row.get(0),
    )
    .optional()
    .map(Option::flatten)
    .map_err(|e| OpError::internal(format!("cannot read the public address: {e}")))
}

/// One spelling of the instance's address, so a link built today and a link
/// built next year are the same string.
///
/// Deliberately strict about the two things that would silently break a link —
/// a missing scheme and a trailing slash — and deliberately incurious about
/// everything else. Kamosu cannot tell from inside whether an address is
/// reachable, and refusing one that turns out to work is worse than storing
/// one that does not (ADR 0033: assume the proxy did nothing but carry bytes).
fn normalise_public_address(address: &str) -> Result<String, OpError> {
    let trimmed = address.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(OpError::bad_request("a public address cannot be empty"));
    }
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(OpError::bad_request(
            "a public address must start with http:// or https:// — it is what goes in front of \
             a Share Link, so it has to be the whole of how somebody reaches this instance",
        ));
    }
    Ok(trimmed.to_string())
}

/// The live Share Link on a Branch.
struct LiveShare {
    id: String,
    shared_by: String,
    created_at: String,
    /// Kept since #171, and never on a link minted before it.
    secret: Option<String>,
}

fn live_share_link(conn: &Connection, branch_id: &str) -> Result<Option<LiveShare>, OpError> {
    conn.query_row(
        "SELECT id, shared_by, created_at, secret FROM share_links \
          WHERE branch_id = ?1 AND ended_at IS NULL",
        params![branch_id],
        |row| {
            Ok(LiveShare {
                id: row.get(0)?,
                shared_by: row.get(1)?,
                created_at: row.get(2)?,
                secret: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read the Share Link: {e}")))
}

/// What the share screen and `share_recipe` both answer.
///
/// `url` is answered for as long as a live link's Secret is kept, which is
/// every link minted since #171 (ADR 0031 as amended). It is built against the
/// public address as it stands now, so moving the instance moves the address
/// the screen shows: the new one is the one that opens. A link minted before
/// #171 kept only its hash and answers no `url`, and the screen says why.
fn share_link_summary(conn: &Connection, live: Option<LiveShare>) -> Result<Value, OpError> {
    let address = stored_public_address(conn)?;
    let Some(LiveShare {
        id,
        shared_by,
        created_at,
        secret,
    }) = live
    else {
        return Ok(json!({
            "shared": false,
            "share_id": Value::Null,
            "url": Value::Null,
            "shared_by": Value::Null,
            "created_at": Value::Null,
            "public_address": address,
        }));
    };
    Ok(json!({
        "shared": true,
        "share_id": id,
        "url": match (secret.as_deref(), address.as_deref()) {
            (Some(secret), Some(address)) => json!(format!("{address}/s/{secret}")),
            _ => Value::Null,
        },
        "shared_by": person_name(conn, &shared_by)?,
        "created_at": created_at,
        "public_address": address,
    }))
}

/// One row of a shared Thread as it comes off the database: sequence, name,
/// *what changed* line, the Hand that wrote it, and when.
type ThreadRow = (i64, Option<String>, Option<String>, String, String);

/// Whether one shared Version actually shows this Photograph — its Main Photo,
/// or the picture on one of its Steps.
fn version_shows_photograph(version: &Value, hash: &str) -> bool {
    content_shows_photograph(&version["content"], hash)
}

/// Everything the public page draws: the shared Branch, its Translations and
/// its Thread. No Attempt is read anywhere in here, by construction.
fn shared_branch(conn: &Connection, branch_id: &str) -> Result<Value, OpError> {
    let (lineage_id, language, head_version_id): (String, String, String) = conn
        .query_row(
            "SELECT lineage_id, language, head_version_id FROM branches WHERE id = ?1",
            params![branch_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?
        .ok_or_else(no_such_branch)?;

    let recipe = shared_version(conn, branch_id, &head_version_id, &language)?;

    // Its Translations (ADR 0006, ADR 0018): the other Branches of this
    // Lineage in this Cookbook written in another Language. They are carried
    // whole rather than as links, because each is a Branch of its own and a
    // second token per Translation would be a second link to end.
    let mut statement = conn
        .prepare(
            "SELECT branches.id, branches.language, branches.head_version_id \
               FROM branches \
              WHERE branches.lineage_id = ?1 AND branches.id <> ?2 \
                AND branches.cookbook_id = (SELECT cookbook_id FROM branches WHERE id = ?2) \
              ORDER BY branches.created_at ASC",
        )
        .map_err(|e| OpError::internal(format!("cannot read Translations: {e}")))?;
    let siblings: Vec<(String, String, String)> = statement
        .query_map(params![lineage_id, branch_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| OpError::internal(format!("cannot read Translations: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read Translations: {e}")))?;

    let mut translations = Vec::new();
    for (sibling_id, sibling_language, sibling_head) in siblings {
        // Only a Translation travels — a Copy or a Divergence is somebody
        // else's recipe standing beside this one, and sharing this Branch says
        // nothing about that one.
        let is_translation = translation_of_branch(conn, &lineage_id, &sibling_id)?
            .get("source_branch_id")
            .and_then(Value::as_str)
            == Some(branch_id);
        if !is_translation {
            continue;
        }
        translations.push(shared_version(
            conn,
            &sibling_id,
            &sibling_head,
            &sibling_language,
        )?);
    }

    // The Thread of the Branch shared: every Version in order with its name,
    // its *what changed* line and its author. Complete back to the first
    // Version, because a share cannot be made to start part-way along, and
    // there is no argument here that could shorten it (ADR 0018).
    let mut statement = conn
        .prepare(
            "SELECT sequence, name, change_note, hand_id, created_at \
               FROM branch_versions WHERE branch_id = ?1 ORDER BY sequence ASC",
        )
        .map_err(|e| OpError::internal(format!("cannot read the Thread: {e}")))?;
    let rows: Vec<ThreadRow> = statement
        .query_map(params![branch_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .map_err(|e| OpError::internal(format!("cannot read the Thread: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read the Thread: {e}")))?;
    let mut thread = Vec::with_capacity(rows.len());
    for (sequence, name, change_note, hand_id, created_at) in rows {
        thread.push(json!({
            "sequence": sequence,
            "name": name,
            "change_note": change_note,
            // A Version's Hand is a Person's (ADR 0015), so this is the name a
            // share carries — which is what makes credit travel with a recipe.
            "hand": person_name(conn, &hand_id)?,
            "created_at": created_at,
        }));
    }

    Ok(json!({
        "recipe": recipe,
        "translations": translations,
        "thread": thread,
    }))
}

/// One Branch's current state, as a stranger sees it.
///
/// **There is no `measured` here**, and its absence is the point. Kamosu
/// converts to the kitchen (ADR 0016) and a stranger holding a link has no
/// kitchen, so every conversion would be into a system nobody asked for. The
/// Reading itself still travels — it is Kamosu's structured reading of the
/// line, useful to an agent reading this share at the MCP door — but the words
/// a stranger is shown are the written ones, which is what ADR 0002 says they
/// are: the truth.
fn shared_version(
    conn: &Connection,
    branch_id: &str,
    version_id: &str,
    language: &str,
) -> Result<Value, OpError> {
    // A Cover is derived from the Lineage id and nothing else (#46), so a page
    // drawing one for a recipe with no photograph needs it here.
    let lineage_id: String = conn
        .query_row(
            "SELECT lineage_id FROM branches WHERE id = ?1",
            params![branch_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?
        .ok_or_else(no_such_branch)?;
    let content: String = conn
        .query_row(
            "SELECT content FROM versions WHERE id = ?1",
            params![version_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read a Version: {e}")))?
        .ok_or_else(|| OpError::not_found("no such Version"))?;
    let content: Value = serde_json::from_str(&content)
        .map(content_as_declared)
        .map_err(|e| OpError::internal(format!("a Version's content is unreadable: {e}")))?;

    let line_count = content["ingredients"].as_array().map(Vec::len).unwrap_or(0);
    let readings = readings_for_version(conn, version_id, line_count)?;

    // The Passengers (ADR 0008): every Component this recipe composes, carried
    // through the link because a recipe that cannot tell you how to make its
    // own dough is incomplete. Resolved against the Cookbook holding what is
    // being shared, and changing nothing about who may see the dough.
    let cookbook_id = branch_cookbook(conn, branch_id)?;
    let mut walk = Unfolding {
        open: vec![lineage_id.clone()],
        ..Unfolding::default()
    };
    unfold_components(
        conn,
        &Unfolds::AsPassenger {
            cookbook_id: &cookbook_id,
            language,
        },
        version_id,
        // A stranger holding a link is cooking nothing, so there is no Yield
        // being cooked to carry into the factor.
        1.0,
        &mut walk,
    )?;
    let components = walk.found;

    Ok(json!({
        "branch_id": branch_id,
        "lineage_id": lineage_id,
        "version_id": version_id,
        "language": language,
        "content": content,
        "readings": readings,
        "components": components,
    }))
}
