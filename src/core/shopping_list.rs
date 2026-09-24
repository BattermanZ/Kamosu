//! The shopping list: what goes on it, the loose items beside it, and the sum
//! of every recipe's lines.

use super::*;

impl Core {
    // ── The Shopping List (#73, ADR 0024) ────────────────────────────────────
    //
    // **The choosing is stored; the rows are computed.** Every Operation below
    // that changes the choosing answers the whole list, worked out again from
    // scratch — which is not a convenience for the screen but the guarantee
    // itself: there is no other copy of a row anywhere to fall out of step.

    /// Choose a recipe to shop for, at the Yield being shopped for or as it is
    /// written. Choosing one already on the list moves nothing and is not an
    /// error — a list is a set, and asking twice for the ratatouille is not two
    /// ratatouilles.
    pub fn add_to_shopping_list(
        &self,
        person_id: &str,
        branch_id: &str,
        shopping_yield: Option<&Value>,
        written_at: Option<&str>,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let head = branch_head(conn, branch_id)?;
            ensure_sees_or_absent(conn, &head.cookbook_id, person_id, no_such_branch)?;
            let title = branch_title(conn, &head.head_version_id)?;
            let Some(moment) = shopping_moment(conn, person_id, written_at)? else {
                return shopping_list(conn, person_id);
            };
            conn.execute(
                // Choosing one already on the list makes no second entry — and
                // it does not quietly ignore what was asked for either. The
                // Yield is part of the choosing this call declares, so *add
                // the coq au vin for eight* moves a list that already holds it
                // to eight, and adding it with no Yield puts it back to the
                // recipe as written. Keeping the old figure would shop for
                // four while saying nothing, which is the one failure a
                // shopping list must never have.
                "INSERT INTO shopping_choices (person_id, branch_id, shopping_yield, known_as, chosen_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5) \
                 ON CONFLICT (person_id, branch_id) DO UPDATE SET \
                     known_as = excluded.known_as, \
                     shopping_yield = excluded.shopping_yield",
                params![
                    person_id,
                    branch_id,
                    stored_yield(shopping_yield)?,
                    title.as_str(),
                    moment
                ],
            )
            .map_err(|e| OpError::internal(format!("cannot choose a recipe to shop for: {e}")))?;
            shopping_list(conn, person_id)
        })
    }

    /// **The list as the text that leaves** (ADR 0024).
    ///
    /// Nothing is ticked inside Kamosu, and this is why that is not a gap:
    /// Kamosu decides what to buy, and something else — Apple Notes, through a
    /// Shortcut — carries it round the shop and holds the ticks. A list with
    /// no way out would have made the missing tick a refusal instead of a
    /// boundary.
    ///
    /// It is a read like any other. Emptying the list is a separate Operation
    /// that the caller may or may not go on to ask for: Kamosu offers and does
    /// not act, because a list that emptied itself on the way out would be
    /// silent and unrecoverable.
    pub fn shopping_list_as_text(&self, person_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let reader = Reader::of(conn, person_id)?;
            let list = shopping_list(conn, person_id)?;
            // The date the note is dividing on is today where the instance is,
            // and SQLite is already the one clock every stored time in Kamosu
            // is read from — so it is asked here too rather than a second
            // clock being introduced to disagree with it.
            let today: String = conn
                .query_row("SELECT strftime('%Y-%m-%d','now')", [], |row| row.get(0))
                .map_err(|e| OpError::internal(format!("cannot read today's date: {e}")))?;
            Ok(json!({ "text": shopping::as_text(&list, &today, &reader.language) }))
        })
    }

    /// **Empty the list**, once it has left as text.
    ///
    /// Offered after sending and never done on the way out (ADR 0024): a list
    /// that emptied itself when it was sent would be silent and unrecoverable,
    /// and Kamosu offers rather than acts — the same rule that makes a Merge
    /// Suggestion evidence and never an instruction.
    ///
    /// It takes the choosing and the Loose Items together, because a half-empty
    /// list is not a state anybody asked for.
    pub fn empty_shopping_list(
        &self,
        person_id: &str,
        written_at: Option<&str>,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            if shopping_moment(conn, person_id, written_at)?.is_none() {
                return shopping_list(conn, person_id);
            }
            conn.execute(
                "DELETE FROM shopping_choices WHERE person_id = ?1",
                params![person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot empty the Shopping List: {e}")))?;
            conn.execute(
                "DELETE FROM shopping_loose_items WHERE person_id = ?1",
                params![person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot empty the Shopping List: {e}")))?;
            shopping_list(conn, person_id)
        })
    }

    /// Take a recipe off the list. Works whether or not it can still be read:
    /// an entry that has gone away is exactly the one somebody most wants gone.
    ///
    /// Removing something that is not there is not an error, here or for a
    /// Loose Item: what is written offline is added, moved and removed on one
    /// afternoon and replayed later (ADR 0013), and a replayed removal that
    /// failed because it had already worked would be Kamosu inventing a
    /// problem out of its own success.
    pub fn remove_from_shopping_list(
        &self,
        person_id: &str,
        branch_id: &str,
        written_at: Option<&str>,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            if shopping_moment(conn, person_id, written_at)?.is_none() {
                return shopping_list(conn, person_id);
            }
            conn.execute(
                "DELETE FROM shopping_choices WHERE person_id = ?1 AND branch_id = ?2",
                params![person_id, branch_id],
            )
            .map_err(|e| OpError::internal(format!("cannot take a recipe off the list: {e}")))?;
            shopping_list(conn, person_id)
        })
    }

    /// Say how much of a chosen recipe is being shopped for. `None` is the
    /// recipe as written.
    pub fn set_shopping_yield(
        &self,
        person_id: &str,
        branch_id: &str,
        shopping_yield: Option<&Value>,
        written_at: Option<&str>,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let head = branch_head(conn, branch_id)?;
            ensure_sees_or_absent(conn, &head.cookbook_id, person_id, no_such_branch)?;
            if shopping_moment(conn, person_id, written_at)?.is_none() {
                return shopping_list(conn, person_id);
            }
            let changed = conn
                .execute(
                    "UPDATE shopping_choices SET shopping_yield = ?3, known_as = ?4 \
                      WHERE person_id = ?1 AND branch_id = ?2",
                    params![
                        person_id,
                        branch_id,
                        stored_yield(shopping_yield)?,
                        branch_title(conn, &head.head_version_id)?
                    ],
                )
                .map_err(|e| OpError::internal(format!("cannot set the Yield: {e}")))?;
            if changed == 0 {
                return Err(OpError::not_found("this recipe is not on your list"));
            }
            shopping_list(conn, person_id)
        })
    }

    /// Type a **Loose Item** onto the list. Kept exactly as typed and never
    /// interpreted (ADR 0024): typing *flour* beside a recipe that wants flour
    /// gives two lines, which is the accepted cost of never guessing at a
    /// number somebody is about to shop by.
    ///
    /// `item_id` is how a line typed with no network arrives (#77): the phone
    /// names it, so taking it off again before the phone is back online can
    /// say which line it meant, and a line sent twice is one line.
    pub fn add_loose_item(
        &self,
        person_id: &str,
        text: &str,
        item_id: Option<&str>,
        written_at: Option<&str>,
    ) -> Result<Value, OpError> {
        let text = required_text(text, "text")?.to_string();
        if let Some(id) = item_id
            && !is_minted_item_id(id)
        {
            return Err(OpError::bad_request(
                "item_id must be i_ followed by sixteen lower-case hex digits",
            ));
        }
        self.db().with_conn(|conn| {
            let Some(moment) = shopping_moment(conn, person_id, written_at)? else {
                return shopping_list(conn, person_id);
            };
            let id = item_id
                .map(str::to_string)
                .unwrap_or_else(|| format!("i_{}", hex::encode(random_bytes(8))));
            let owner: Option<String> = conn
                .query_row(
                    "SELECT person_id FROM shopping_loose_items WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read a Loose Item: {e}")))?;
            match owner {
                Some(owner) if owner == person_id => {}
                Some(_) => return Err(OpError::bad_request("that item_id is taken")),
                None => {
                    conn.execute(
                        "INSERT INTO shopping_loose_items (id, person_id, text, added_at) \
                         VALUES (?1, ?2, ?3, ?4)",
                        params![id, person_id, text, moment],
                    )
                    .map_err(|e| OpError::internal(format!("cannot add a Loose Item: {e}")))?;
                }
            }
            shopping_list(conn, person_id)
        })
    }

    /// Take a Loose Item off the list. Removing one that is already gone is not
    /// an error, for the reason `remove_from_shopping_list` gives: the two are
    /// siblings and a caller should not have to remember which of them is
    /// strict.
    pub fn remove_loose_item(
        &self,
        person_id: &str,
        item_id: &str,
        written_at: Option<&str>,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            if shopping_moment(conn, person_id, written_at)?.is_none() {
                return shopping_list(conn, person_id);
            }
            conn.execute(
                "DELETE FROM shopping_loose_items WHERE id = ?1 AND person_id = ?2",
                params![item_id, person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot remove a Loose Item: {e}")))?;
            shopping_list(conn, person_id)
        })
    }

    /// The whole list: the choosing, and the rows worked out from it.
    pub fn shopping_list(&self, person_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| shopping_list(conn, person_id))
    }

    /// **What one recipe puts on a Shopping List, before anything is added
    /// up** (#77, ADR 0024): the facts a phone with no network needs to work
    /// the rows out itself.
    ///
    /// ADR 0024 says the rows compute offline for the recipes the phone
    /// holds, and Aurélien chose on #77 to have the phone do that sum rather
    /// than leave it until the server is back. The sum is the easy half. What
    /// the phone cannot know by itself is everything behind it: which Food
    /// each line was read as, the name that Food goes by for this reader, how
    /// much the line said, which of Kamosu's Units its word is, and what a cup
    /// of that Food weighs. Those are read here, exactly as `shopping_list`
    /// reads them, and the phone adds them up in `ui/src/lib/offline/shopping.ts`.
    /// `shopping_parity` fails the build if the two ever add differently.
    ///
    /// Always the Branch's latest Version, as the list itself is (ADR 0024).
    pub fn shopping_basis(&self, person_id: &str, branch_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let head = branch_head(conn, branch_id)?;
            ensure_sees_or_absent(conn, &head.cookbook_id, person_id, no_such_branch)?;
            let reader = Reader::of(conn, person_id)?;
            let content = version_content(conn, &head.head_version_id)?;
            let lines = basis_lines(
                conn,
                &reader,
                person_id,
                &head.lineage_id,
                &head.head_version_id,
                &content,
            )?;
            Ok(json!({
                "branch_id": branch_id,
                "title": content["title"],
                "written_yield": content["yield"],
                "lines": lines,
            }))
        })
    }
}

/// **The last device to write a Shopping List wins** (#77, ADR 0024).
///
/// The moment this write was made, and a note that the list was written then —
/// or nothing, where the list has since been written later by another device
/// and this write, sent late by a phone that had no signal, is older than what
/// the list now says. The write is then dropped whole, and the caller answers
/// the list as it stands: the list the iPad left this morning is not undone by
/// what the phone did yesterday.
///
/// Kept per list rather than per recipe on it: a write older than the list's
/// last one changes nothing anywhere on it, not only where the two touched the
/// same recipe. Each write is judged on its own, in the order it was made, so
/// a phone that wrote before *and* after the iPad keeps the change it made
/// after — that one is the last write, and it is made to the list as the iPad
/// left it.
fn shopping_moment(
    conn: &Connection,
    person_id: &str,
    written_at: Option<&str>,
) -> Result<Option<String>, OpError> {
    let moment = written_moment(conn, written_at)?;
    let last: Option<String> = conn
        .query_row(
            "SELECT shopping_written_at FROM people WHERE id = ?1",
            params![person_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read the Shopping List: {e}")))?
        .flatten();
    if last.as_deref().is_some_and(|last| moment.as_str() < last) {
        return Ok(None);
    }
    conn.execute(
        "UPDATE people SET shopping_written_at = ?2 WHERE id = ?1",
        params![person_id, moment],
    )
    .map_err(|e| OpError::internal(format!("cannot write the Shopping List: {e}")))?;
    Ok(Some(moment))
}

/// Whether a Loose Item id is one a phone may mint: the shape the server
/// mints its own in.
fn is_minted_item_id(id: &str) -> bool {
    id.strip_prefix("i_").is_some_and(is_sixteen_hex)
}

// ── The Shopping List, worked out (#73, ADR 0024) ────────────────────────────

/// What is stored for a Yield being shopped for: the recipe's own two fields,
/// or nothing at all for the recipe as written.
///
/// Stored as JSON in one column exactly as `attempts.cooking_yield` is, because
/// it is the same fact — how much of this recipe somebody means — and two
/// spellings of one fact is how the two drift apart.
fn stored_yield(shopping_yield: Option<&Value>) -> Result<Option<String>, OpError> {
    match shopping_yield {
        None | Some(Value::Null) => Ok(None),
        // The same rule a cooking's Yield is held to, multiplier included
        // (#109): one fact, one check.
        Some(value) => Ok(Some(parse_wanted_yield(value)?.to_string())),
    }
}

/// One entry in the choosing, once it has been looked up.
struct Chosen {
    branch_id: String,
    /// What the recipe is called: its live title where it can still be read,
    /// and otherwise the name it was known by when it was chosen.
    title: String,
    /// Whether it can still be read at all (ADR 0024). A Kitchen this Person no
    /// longer cooks in, or a Branch that is gone: either way the entry stays,
    /// keeps its name, contributes nothing, and says so — because a thing that
    /// quietly disappears from a shopping list is a thing that does not get
    /// bought.
    gone: bool,
    shopping_yield: Value,
    written_yield: Value,
    /// The recipe's own content, where it could be read.
    content: Option<Value>,
    head_version_id: Option<String>,
    /// Which Lineage the chosen Branch is a Branch of, so that a recipe
    /// composing itself stops at the first repeat rather than unfolding for
    /// ever (ADR 0008).
    lineage_id: Option<String>,
}

/// **The whole Shopping List**: the choosing as stored, and the rows worked out
/// from it on every single read (ADR 0024).
///
/// Nothing computed here is written anywhere. Correct a Reading, edit a recipe,
/// move a Yield, change your Reading Measures — the next read simply says
/// something else, which is what a stored row could never do without going
/// stale first.
fn shopping_list(conn: &Connection, person_id: &str) -> Result<Value, OpError> {
    let reader = Reader::of(conn, person_id)?;
    let chosen = chosen_recipes(conn, person_id)?;

    // What each readable recipe puts on the list — the same answer
    // `shopping_basis` hands a phone (#77) — and then the rows added up from
    // those, in `shopping::rows`, which is the one sum the phone's copy is
    // checked against.
    let mut bases = Vec::with_capacity(chosen.len());
    for entry in &chosen {
        let (Some(content), Some(version_id), Some(lineage_id)) =
            (&entry.content, &entry.head_version_id, &entry.lineage_id)
        else {
            continue;
        };
        bases.push((
            entry,
            basis_lines(conn, &reader, person_id, lineage_id, version_id, content)?,
        ));
    }
    let gathered = bases
        .iter()
        .map(|(entry, lines)| shopping::Chosen {
            branch_id: &entry.branch_id,
            title: &entry.title,
            scale: yield_scale(&entry.shopping_yield, &entry.written_yield),
            lines,
        })
        .collect::<Vec<_>>();
    let rows = shopping::rows(
        &gathered,
        &loose_items(conn, person_id)?,
        reader.measures,
        &reader.language,
    );

    Ok(json!({
        "chosen": chosen
            .iter()
            .map(|entry| json!({
                "branch_id": entry.branch_id,
                "title": entry.title,
                "gone": entry.gone,
                "shopping_yield": entry.shopping_yield,
                "written_yield": entry.written_yield,
            }))
            .collect::<Vec<_>>(),
        "rows": rows,
    }))
}

/// **What one chosen recipe puts on a Shopping List, Components and all**
/// (ADR 0024, ADR 0008, #86).
///
/// Its own Ingredient Lines, and then every line of every recipe it composes,
/// to the bottom — because ADR 0008 says a shopping list unfolds Components to
/// the bottom and then merges Foods, which is a Food question rather than a
/// link question. The pizza's flour and its dough's flour are the same row.
///
/// **The unfolding is the very walk the recipe page uses**, repeat guard and
/// all ([`unfold_components`]). Only what is done with each Component differs,
/// and that is the whole of what this function decides:
///
/// - **A Component that opened contributes its inner recipe's Foods and no
///   line of its own.** Its written line — *500 g de pâte à pizza* — comes off
///   the list, because the dough is now on it as flour, water, salt and yeast.
/// - **One that did not keeps its written line and contributes nothing.** A
///   recipe nobody in view holds, and a repeat the walk stopped at, are both
///   this case. A thing that quietly disappears from a shopping list is a
///   thing that does not get bought (ADR 0024), and the written line already
///   carries the human meaning (ADR 0002).
/// - **One Kamosu could not work out a factor for contributes its Foods
///   carrying no amount** — Aurélien's call on #86. `2 poignées de pâte`
///   cannot be measured against the dough's Yield, so the dough's flour rides
///   in the *some* bucket ADR 0024 already gives the 28% of Ingredient Lines
///   written with no quantity. Quoting the line instead would leave that flour
///   off the list; folding the whole dough in at full strength would put a
///   figure Kamosu declined to compute inside a row that reads as certain,
///   which is the one thing a merged row cannot say. **Once the factor is lost
///   it stays lost**: a Component nested under an unmeasured one is unmeasured
///   too, because there is nothing honest left to multiply it by.
///
/// Every amount here is for the recipe **as written**. The Yield being shopped
/// for is the outer scale and `shopping::rows` applies it to the lot, so the
/// factor and the Yield compound the way the recipe page's do.
///
/// This is what `shopping_basis` answers (#77) and what `shopping_list` adds
/// up, so the phone and the server start from the same facts — and a phone
/// with no network gets the Components for free, having never been told there
/// were any.
fn basis_lines(
    conn: &Connection,
    reader: &Reader,
    person_id: &str,
    lineage_id: &str,
    version_id: &str,
    content: &Value,
) -> Result<Vec<Value>, OpError> {
    let mut lines = version_basis_lines(conn, reader, version_id, content, Reached::chosen())?;

    let unfolds = Unfolds::ForShopping { person_id, reader };
    let mut walk = Unfolding {
        open: vec![lineage_id.to_string()],
        ..Unfolding::default()
    };
    unfold_components(conn, &unfolds, version_id, 1.0, &mut walk)?;

    // The Components whose own written line comes off, and the ones beneath
    // which no amount can honestly be worked out. Both are held as the `path`
    // of line indexes that reaches each — the same path the walk hands back —
    // so a Component nested four deep is named as precisely as a top-level one.
    let mut unfolded: Vec<Value> = Vec::new();
    let mut unmeasured: Vec<Vec<i64>> = Vec::new();

    for component in &walk.found {
        let Some(path) = component_path(&component["path"]) else {
            continue;
        };
        // A Component Kamosu could not open — a recipe nobody in view holds,
        // or a repeat the walk stopped at — keeps its written line and
        // contributes nothing. `inside` holds only the ones that opened, so
        // finding this path there *is* the test.
        let Some((_, inner_version_id)) = walk.inside.iter().find(|(at, _)| *at == path) else {
            continue;
        };
        let share = if beneath(&unmeasured, &path) {
            None
        } else {
            component["share"].as_f64()
        };
        if share.is_none() {
            unmeasured.push(path.clone());
        }
        let inner = version_basis_lines(
            conn,
            reader,
            inner_version_id,
            &component["content"],
            Reached {
                path: &path,
                from: Some(ArrivedAs {
                    branch_id: component["branch_id"].as_str().unwrap_or_default(),
                    title: component["title"].as_str().unwrap_or_default(),
                }),
                share,
            },
        )?;
        // **Its written line comes off only once something has replaced it.**
        // A recipe whose Ingredients are empty — or are all Sections — unfolds
        // to nothing, and taking the line off for it would leave the pizza
        // saying nothing at all about its dough. A thing that quietly
        // disappears from a shopping list is a thing that does not get bought
        // (ADR 0024), and that holds however the disappearing happens.
        if !inner.is_empty() {
            unfolded.push(component["path"].clone());
        }
        lines.extend(inner);
    }

    // The written lines of the Components that opened, now that everything
    // beneath them is on the list.
    lines.retain(|line| !unfolded.iter().any(|opened| *opened == line["path"]));

    // **And every Component line still standing says why it is standing**
    // (ADR 0008, Aurélien's call on #86). A line that buys nothing otherwise
    // reads exactly like a line Kamosu could not interpret, and the shopper
    // cannot tell from the list which they are looking at — so the one
    // sentence the Core already words for the recipe page goes beneath it
    // here too. *Kamosu does not have this recipe* is the one that earns it:
    // it says the flour and the water are nobody's job but yours.
    for component in &walk.found {
        let at = &component["path"];
        if unfolded.iter().any(|opened| opened == at) {
            continue;
        }
        if let Some(line) = lines.iter_mut().find(|line| line["path"] == *at) {
            line["said"] = component["said"].clone();
        }
    }
    Ok(lines)
}

/// **The Component a Version arrived as**: which Branch it resolved to, and
/// what that Branch is called. Named rather than a pair, because two strings
/// side by side are two strings that can be swapped without anything noticing.
#[derive(Clone, Copy)]
struct ArrivedAs<'a> {
    branch_id: &'a str,
    title: &'a str,
}

/// **How a Version was reached**, for the Ingredient Lines it puts on a
/// Shopping List: where it sits, what it arrived as, and how much of it is
/// wanted. The three travel together because they are one answer — *this
/// recipe, reached this way* — and separating them is how a line ends up
/// carrying one recipe's path and another's name.
struct Reached<'a> {
    /// The chain of Component line indexes that reaches it. Empty for the
    /// chosen recipe, which was reached by being chosen.
    path: &'a [i64],
    /// The Component it arrived as, or nothing for the chosen recipe itself.
    from: Option<ArrivedAs<'a>>,
    /// How much of it is wanted. `None` is Kamosu saying it could not work
    /// that out, and every amount then goes out unstated rather than as a
    /// number nobody computed (#86).
    share: Option<f64>,
}

impl Reached<'_> {
    /// The recipe somebody put on their list: the whole of it, arrived at by
    /// nothing.
    fn chosen() -> Reached<'static> {
        Reached {
            path: &[],
            from: None,
            share: Some(1.0),
        }
    }
}

/// The `path` of line indexes an unfolded Component carries, as numbers.
fn component_path(path: &Value) -> Option<Vec<i64>> {
    path.as_array()?.iter().map(Value::as_i64).collect()
}

/// Whether a Component sits under one Kamosu could work out no share for.
///
/// `unmeasured` holds the path of each such Component, and depth-first order
/// means every one above this point has already been met — so a prefix match
/// is the whole test.
fn beneath(unmeasured: &[Vec<i64>], path: &[i64]) -> bool {
    unmeasured
        .iter()
        .any(|above| path.len() > above.len() && path.starts_with(above))
}

/// One Version's own Ingredient Lines as a Shopping List reads them: the Food
/// each was read as and that Food's name for this reader, how much it said,
/// and its Unit — or, where no Food was read, only its words (ADR 0024).
///
/// [`Reached`] says how this Version was arrived at: where it sits, what
/// Component it came in as, and how much of it is wanted.
fn version_basis_lines(
    conn: &Connection,
    reader: &Reader,
    version_id: &str,
    content: &Value,
    reached: Reached<'_>,
) -> Result<Vec<Value>, OpError> {
    let readings = readings_to_measure(conn, version_id)?;
    let food_ids = readings
        .iter()
        .filter_map(|reading| reading.food_id.clone())
        .collect::<Vec<_>>();
    let named = food_names_of(conn, &food_ids)?;

    // Where each line sits, as the chain of line indexes that reaches it: `[3]`
    // in the chosen recipe, `[3, 1]` for the second line of the dough its
    // fourth line names. It is what names a row that merges with nothing, and
    // a plain index could not do that job any more — the pizza's line 0 and
    // its dough's line 0 are two different things to buy.
    let at = |index: usize| {
        reached
            .path
            .iter()
            .copied()
            .chain(std::iter::once(index as i64))
            .collect::<Vec<_>>()
    };
    // Which recipe a line is really from, for a row that breaks open to say so
    // (ADR 0002): the dough by name, not the pizza that composes it.
    let source = reached
        .from
        .map(|from| json!({ "branch_id": from.branch_id, "title": from.title }))
        .unwrap_or(Value::Null);

    let empty = Vec::new();
    let mut lines = Vec::new();
    for (index, line) in content["ingredients"]
        .as_array()
        .unwrap_or(&empty)
        .iter()
        .enumerate()
    {
        // A Section heads a list; it is not a thing to buy.
        if line.get("kind").and_then(Value::as_str) == Some("section") {
            continue;
        }
        let text = line.get("text").and_then(Value::as_str).unwrap_or_default();
        // An Ingredient Line nobody ever read, and one whose Reading found no
        // Food, are the same thing on a list: there is no Food to merge it
        // under, so the line stands exactly as written (ADR 0024).
        let Some(reading) = readings
            .iter()
            .find(|reading| reading.line_index == index as i64)
            .filter(|reading| reading.food_id.is_some())
        else {
            lines.push(json!({
                "path": at(index),
                "from": source,
                "text": text,
                "food": Value::Null,
                // Filled in by `basis_lines` for a Component that kept its
                // written line, and null on every other line.
                "said": Value::Null,
            }));
            continue;
        };
        let food_id = reading.food_id.clone().expect("filtered to a Food above");
        let names = named.get(&food_id).cloned().unwrap_or_default();
        let shown = shown_name(&names, &reader.language);
        let unit = reading.unit.as_deref();
        lines.push(json!({
            "path": at(index),
            "from": source,
            "text": text,
            // A line that named a Food is a line Kamosu read; it is the ones
            // standing for a Component it could not open that need a sentence.
            "said": Value::Null,
            "food": {
                "id": food_id,
                // A Shopping Row is the one place a Food's name is read
                // instead of an Ingredient Line (ADR 0024), which is what
                // makes flour and farine one row.
                "name": shown.map(|(_, name)| name.as_str()),
                "name_language": shown.map(|(language, _)| language.as_str()),
                // An amount Kamosu could not read is an amount nobody stated,
                // as far as adding up goes — and the written line, one tap
                // away, still says whatever it says. So is every amount of an
                // inner recipe Kamosu could work out no factor for (#86):
                // there is no honest number to put here, and *some* is a
                // truthful answer where a guess would not be.
                "amount": reached.share.and_then(|share| reading
                    .amount
                    .as_deref()
                    .and_then(units::parse_amount)
                    .map(|amount| amount * share)),
                "unit": unit,
                "unit_id": unit.and_then(units::recognise).map(|known| known.id),
                "unit_key": unit.map(shopping::unit_key),
                // The very same reader the recipe page's subordinate lines
                // are built from, Cup Weights and all — because a cup of flour
                // must not weigh one thing on a recipe page and another in a
                // shop.
                "cup_weight_grams": reading.cup_weight,
            },
        }));
    }
    Ok(lines)
}

/// The choosing as stored, in the order it was made, each entry looked up.
fn chosen_recipes(conn: &Connection, person_id: &str) -> Result<Vec<Chosen>, OpError> {
    let mut statement = conn
        .prepare(
            "SELECT branch_id, shopping_yield, known_as FROM shopping_choices \
              WHERE person_id = ?1 ORDER BY chosen_at ASC, branch_id ASC",
        )
        .map_err(|e| OpError::internal(format!("cannot read the Shopping List: {e}")))?;
    let stored: Vec<(String, Option<String>, String)> = statement
        .query_map(params![person_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| OpError::internal(format!("cannot read the Shopping List: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read the Shopping List: {e}")))?;

    let mut chosen = Vec::with_capacity(stored.len());
    for (branch_id, shopping_yield, known_as) in stored {
        let shopping_yield = shopping_yield
            .as_deref()
            .and_then(|stored| serde_json::from_str(stored).ok())
            .unwrap_or(Value::Null);
        // A Branch always at its latest Version, never pinned (ADR 0024): a
        // list describes what is about to be bought, so a recipe edited
        // between Sunday and Tuesday is right on Tuesday.
        let head = branch_head(conn, &branch_id).ok();
        let readable = match &head {
            Some(head) => sees_cookbook(conn, &head.cookbook_id, person_id)?,
            None => false,
        };
        let content = match (&head, readable) {
            (Some(head), true) => Some(version_content(conn, &head.head_version_id)?),
            _ => None,
        };
        chosen.push(Chosen {
            title: content
                .as_ref()
                .and_then(|content| content["title"].as_str())
                // The name it was known by, which is the whole point of
                // storing one (ADR 0024).
                .unwrap_or(&known_as)
                .to_string(),
            written_yield: content
                .as_ref()
                .map(|content| content["yield"].clone())
                .unwrap_or(Value::Null),
            head_version_id: head.as_ref().map(|head| head.head_version_id.clone()),
            lineage_id: head.as_ref().map(|head| head.lineage_id.clone()),
            gone: !readable,
            branch_id,
            shopping_yield,
            content,
        });
    }
    Ok(chosen)
}

/// Every name of *many* Foods at once, by Food.
///
/// A Shopping Row names a Food rather than an Ingredient Line (ADR 0024), so
/// building a list asks this question once per row — and asking it one Food at
/// a time made a list of three recipes several hundred queries deep. The
/// choosing is small and the answer is one statement, so it is read in a single
/// pass and handed to the rows already gathered.
fn food_names_of(
    conn: &Connection,
    food_ids: &[String],
) -> Result<HashMap<String, Vec<(String, String)>>, OpError> {
    let mut names: HashMap<String, Vec<(String, String)>> = HashMap::new();
    if food_ids.is_empty() {
        return Ok(names);
    }
    // One placeholder per Food. The ids are Kamosu's own, never a cook's text,
    // and they still go in as bound parameters rather than as SQL.
    let placeholders = std::iter::repeat_n("?", food_ids.len())
        .collect::<Vec<_>>()
        .join(", ");
    let mut statement = conn
        .prepare(&format!(
            "SELECT food_id, language, name FROM food_names WHERE food_id IN ({placeholders})"
        ))
        .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(food_ids), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| OpError::internal(format!("cannot read Food names: {e}")))?;
    for (food_id, language, name) in rows {
        names.entry(food_id).or_default().push((language, name));
    }
    Ok(names)
}

/// This Person's Loose Items, as rows. They carry no amount and no Food, so
/// there is nothing to compute: what is stored is what is shown, exactly as it
/// was typed (ADR 0024).
fn loose_items(conn: &Connection, person_id: &str) -> Result<Vec<Value>, OpError> {
    let mut statement = conn
        .prepare(
            "SELECT id, text FROM shopping_loose_items \
              WHERE person_id = ?1 ORDER BY added_at ASC, id ASC",
        )
        .map_err(|e| OpError::internal(format!("cannot read Loose Items: {e}")))?;
    statement
        .query_map(params![person_id], |row| {
            let id: String = row.get(0)?;
            let text: String = row.get(1)?;
            Ok(json!({
                "id": id,
                "kind": "loose",
                "name": text,
                "name_language": Value::Null,
                "parts": Vec::<Value>::new(),
                "lines": Vec::<Value>::new(),
                // Kamosu has nothing to say about words somebody typed at the
                // door: a Loose Item is never interpreted, deliberately
                // (ADR 0024).
                "said": Value::Null,
            }))
        })
        .map_err(|e| OpError::internal(format!("cannot read Loose Items: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read Loose Items: {e}")))
}
