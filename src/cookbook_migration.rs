//! Migration 36's data half: every recipe moves from the Kitchen that held it
//! to a **Cookbook** (ADR 0041, #131).
//!
//! The SQL half of the step builds the new tables empty. This moves what the
//! old ones held into them by the rules settled on #131 (question 9), then
//! swaps the tables over. It runs inside the step's one transaction with
//! foreign keys off, and the runner checks every reference before it commits,
//! so nothing here can leave a row pointing at nothing without the whole step
//! failing.
//!
//! What it promises, and the behaviour tests hold it to: **no Version id
//! moves** (nothing here touches `versions`), and **no Branch, Attempt, Tag
//! filing or Photograph is lost**. Every Branch is copied across under its own
//! id; `attempts`, `branch_versions` and `branch_tags` keep pointing at the
//! same ids; Photographs hang off Versions, which are untouched.

use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, params};

type Step<T> = Result<T, String>;

fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", hex::encode(crate::core::random_bytes(8)))
}

fn rows<T>(
    conn: &Connection,
    sql: &str,
    read: impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> Step<Vec<T>> {
    let mut statement = conn.prepare(sql).map_err(|e| e.to_string())?;
    statement
        .query_map([], read)
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())
}

/// Everything about the old Kitchens this step decides by.
struct Kitchens {
    hand: HashMap<String, String>,
    name: HashMap<String, String>,
    /// Each Kitchen's members, oldest account first.
    members: HashMap<String, Vec<String>>,
    /// Whose Home Kitchen each one was.
    home_owner: HashMap<String, String>,
}

impl Kitchens {
    /// The Person whose Cookbook something the Kitchen held goes to when
    /// nothing about the thing itself says: whose Home it was, else its
    /// oldest member.
    fn keeper(&self, kitchen_id: &str) -> Option<&String> {
        self.home_owner
            .get(kitchen_id)
            .or_else(|| self.members.get(kitchen_id).and_then(|m| m.first()))
    }
}

pub fn move_to_cookbooks(conn: &Connection) -> Result<(), String> {
    let people: Vec<(String, Option<String>)> = rows(
        conn,
        "SELECT id, home_kitchen_id FROM people ORDER BY created_at, id",
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let operator: Option<String> = rows(
        conn,
        "SELECT operator_person_id FROM instance_setup WHERE operator_person_id IS NOT NULL",
        |r| r.get(0),
    )?
    .into_iter()
    .next();

    let mut kitchens = Kitchens {
        hand: HashMap::new(),
        name: HashMap::new(),
        members: HashMap::new(),
        home_owner: HashMap::new(),
    };
    for (id, name, hand) in rows(conn, "SELECT id, name, hand_id FROM kitchens", |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })? {
        kitchens.hand.insert(id.clone(), hand);
        kitchens.name.insert(id, name);
    }
    for (kitchen_id, person_id) in rows(
        conn,
        "SELECT kitchen_members.kitchen_id, kitchen_members.person_id \
           FROM kitchen_members JOIN people ON people.id = kitchen_members.person_id \
          ORDER BY people.created_at, people.id",
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
    )? {
        kitchens
            .members
            .entry(kitchen_id)
            .or_default()
            .push(person_id);
    }
    for (person_id, home) in &people {
        if let Some(home) = home {
            kitchens.home_owner.insert(home.clone(), person_id.clone());
        }
    }

    // ── One Cookbook for every Person ────────────────────────────────────────
    // Its Hand is the one their Home Kitchen wrote under, where they had one,
    // so every Branch their Home wrote keeps being named after them.
    let mut cookbook_of: HashMap<String, String> = HashMap::new();
    let mut hands_taken: HashSet<String> = HashSet::new();
    for (person_id, home) in &people {
        let cookbook_id = new_id("c");
        let hand = home
            .as_ref()
            .and_then(|home| kitchens.hand.get(home))
            .filter(|hand| hands_taken.insert((*hand).clone()))
            .cloned()
            .unwrap_or_else(|| cookbook_id.clone());
        conn.execute(
            "INSERT INTO cookbooks (id, hand_id) VALUES (?1, ?2)",
            params![cookbook_id, hand],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO cookbook_authors (cookbook_id, person_id) VALUES (?1, ?2)",
            params![cookbook_id, person_id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO cookbook_hands (hand_id, cookbook_id, person_id) VALUES (?1, ?2, ?3)",
            params![hand, cookbook_id, person_id],
        )
        .map_err(|e| e.to_string())?;
        cookbook_of.insert(person_id.clone(), cookbook_id);
    }
    let fallback_person = operator
        .clone()
        .or_else(|| people.first().map(|(id, _)| id.clone()));

    // ── Every Branch to the Cookbook of whoever started it ───────────────────
    struct OldBranch {
        id: String,
        lineage_id: String,
        kitchen_id: String,
        hand_id: String,
        language: String,
        origin_address: Option<String>,
        signature: Option<String>,
        head_version_id: String,
        created_at: String,
        travelling_id: Option<String>,
    }
    let old: Vec<OldBranch> = rows(
        conn,
        "SELECT id, lineage_id, kitchen_id, hand_id, language, origin_address, signature, \
                head_version_id, created_at, travelling_id \
           FROM branches ORDER BY created_at, id",
        |r| {
            Ok(OldBranch {
                id: r.get(0)?,
                lineage_id: r.get(1)?,
                kitchen_id: r.get(2)?,
                hand_id: r.get(3)?,
                language: r.get(4)?,
                origin_address: r.get(5)?,
                signature: r.get(6)?,
                head_version_id: r.get(7)?,
                created_at: r.get(8)?,
                travelling_id: r.get(9)?,
            })
        },
    )?;
    // The Hands that wrote on each Branch, in order. A Copy carries the whole
    // chain behind it with each row's own date, so the rows dated before the
    // Branch was made are history it carried, not writing done on it: whoever
    // started a Copy is the first Hand on a row written after it began. Where
    // no row is that late, the chain is all there is to go on.
    let hands_by_branch: HashMap<String, Vec<String>> = {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        let mut carried: HashMap<String, Vec<String>> = HashMap::new();
        for (branch_id, hand_id, written_here) in rows(
            conn,
            "SELECT branch_versions.branch_id, branch_versions.hand_id, \
                    branch_versions.created_at >= branches.created_at \
               FROM branch_versions JOIN branches ON branches.id = branch_versions.branch_id \
              ORDER BY branch_versions.branch_id, branch_versions.sequence",
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, bool>(2)?,
                ))
            },
        )? {
            if written_here {
                map.entry(branch_id.clone())
                    .or_default()
                    .push(hand_id.clone());
            }
            carried.entry(branch_id).or_default().push(hand_id);
        }
        for (branch_id, chain) in carried {
            map.entry(branch_id).or_insert(chain);
        }
        map
    };
    let person_ids: HashSet<&String> = people.iter().map(|(id, _)| id).collect();

    let mut branch_cookbook: HashMap<String, String> = HashMap::new();
    for branch in &old {
        let members = kitchens
            .members
            .get(&branch.kitchen_id)
            .cloned()
            .unwrap_or_default();
        let hands = hands_by_branch.get(&branch.id).cloned().unwrap_or_default();
        // A Kitchen of one: that one. A shared Kitchen: the first member whose
        // Hand wrote on the Branch, which for a Branch the Kitchen wrote is
        // whoever wrote its first Version and for a Copy is whoever first
        // changed it. A Kitchen nobody is left in: the first Person who wrote
        // on it, the same way. A Branch that arrived carries nobody's Hand from here,
        // so it goes to whose Home the Kitchen was, else its oldest member.
        //
        // Answer 9 on #131 asks for "the Person who ran the Import that
        // brought it" before the oldest member. No table ever recorded who
        // ran an Import, but every importer landed its recipes in the Home
        // Kitchen of whoever asked, so the Home's owner is that Person
        // wherever it can be known at all.
        // A Branch that arrived was written by its sender and started here
        // by whoever received it, so its Hands say nothing about whose it is.
        let arrived = kitchens.hand.get(&branch.kitchen_id) != Some(&branch.hand_id);
        let writer = |among: &dyn Fn(&String) -> bool| {
            if arrived {
                None
            } else {
                hands.iter().find(|hand| among(hand)).cloned()
            }
        };
        let home_owner = kitchens.home_owner.get(&branch.kitchen_id).cloned();
        let starter = if members.len() == 1 {
            Some(members[0].clone())
        } else if !members.is_empty() {
            writer(&|hand| members.contains(hand))
                .or_else(|| home_owner.clone().filter(|owner| members.contains(owner)))
                .or_else(|| members.first().cloned())
        } else {
            writer(&|hand| person_ids.contains(hand))
                .or_else(|| home_owner.clone())
                .or_else(|| hands.iter().find(|hand| person_ids.contains(hand)).cloned())
        }
        .or_else(|| fallback_person.clone())
        .ok_or_else(|| format!("Branch {} has nobody to belong to", branch.id))?;
        let cookbook_id = cookbook_of[&starter].clone();
        conn.execute(
            "INSERT INTO branches_new \
             (id, lineage_id, cookbook_id, hand_id, language, origin_address, signature, \
              head_version_id, created_at, travelling_id, name, started_by, arrived) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, ?11, ?12)",
            params![
                branch.id,
                branch.lineage_id,
                cookbook_id,
                branch.hand_id,
                branch.language,
                branch.origin_address,
                branch.signature,
                branch.head_version_id,
                branch.created_at,
                branch.travelling_id,
                starter,
                arrived as i64,
            ],
        )
        .map_err(|e| e.to_string())?;
        branch_cookbook.insert(branch.id.clone(), cookbook_id);
    }

    // Two Kitchens of one Person may each have received the same friend's
    // Branch. In one Cookbook a Travelling id names one Branch, so the later
    // one travels on under a new id of its own (#131, question 6).
    let mut travelling_seen: HashSet<(String, String)> = HashSet::new();
    for branch in &old {
        let key = (
            branch
                .travelling_id
                .clone()
                .unwrap_or_else(|| branch.id.clone()),
            branch_cookbook[&branch.id].clone(),
        );
        if !travelling_seen.insert(key) {
            conn.execute(
                "UPDATE branches_new SET travelling_id = ?1 WHERE id = ?2",
                params![new_id("b"), branch.id],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    // And a Cookbook keeps one unnamed Branch of a recipe in each Language.
    // Where two Kitchens both held one, the later one is named after the
    // Kitchen it came from, which is how it was told apart until now.
    let mut unnamed_seen: HashSet<(String, String, String)> = HashSet::new();
    for branch in &old {
        // A Branch that arrived is labelled by its sender and never counted.
        if kitchens.hand.get(&branch.kitchen_id) != Some(&branch.hand_id) {
            continue;
        }
        let key = (
            branch_cookbook[&branch.id].clone(),
            branch.lineage_id.clone(),
            branch.language.clone(),
        );
        if !unnamed_seen.insert(key) {
            let name = kitchens
                .name
                .get(&branch.kitchen_id)
                .cloned()
                .unwrap_or_else(|| "Another".to_string());
            conn.execute(
                "UPDATE branches_new SET name = ?1 WHERE id = ?2",
                params![name, branch.id],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    let cookbook_for_kitchen = |kitchen_id: &str| -> Option<String> {
        kitchens
            .keeper(kitchen_id)
            .or(fallback_person.as_ref())
            .map(|person| cookbook_of[person].clone())
    };

    // ── Tags follow the recipes they file ────────────────────────────────────
    // A Kitchen's Tag goes into every Cookbook one of its recipes went to, so
    // nothing loses the Tag it was filed under. Two Kitchens bringing the same
    // word into one Cookbook make one Tag of it, as typing it twice would.
    let old_tags: Vec<(String, String, String)> =
        rows(conn, "SELECT id, kitchen_id, created_at FROM tags", |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?;
    let mut tag_names: HashMap<String, Vec<(String, String, String)>> = HashMap::new();
    for (tag_id, language, name, folded) in rows(
        conn,
        "SELECT tag_id, language, name, name_folded FROM tag_names ORDER BY tag_id, language",
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        },
    )? {
        tag_names
            .entry(tag_id)
            .or_default()
            .push((language, name, folded));
    }
    let tag_created: HashMap<String, String> = old_tags
        .iter()
        .map(|(id, _, at)| (id.clone(), at.clone()))
        .collect();
    let mut tag_map: HashMap<(String, String), String> = HashMap::new();
    let mut word_in_cookbook: HashMap<(String, String, String), String> = HashMap::new();
    let mut languages_of_tag: HashMap<String, HashSet<String>> = HashMap::new();
    let mut old_ids_used: HashSet<String> = HashSet::new();
    let mut tag_in = |cookbook_id: &str, old_tag: &str| -> Step<String> {
        let key = (old_tag.to_string(), cookbook_id.to_string());
        if let Some(found) = tag_map.get(&key) {
            return Ok(found.clone());
        }
        let names = tag_names.get(old_tag).cloned().unwrap_or_default();
        let existing = names.iter().find_map(|(language, _, folded)| {
            word_in_cookbook
                .get(&(cookbook_id.to_string(), language.clone(), folded.clone()))
                .cloned()
        });
        let target = match existing {
            Some(target) => target,
            None => {
                let id = if old_ids_used.insert(old_tag.to_string()) {
                    old_tag.to_string()
                } else {
                    new_id("t")
                };
                conn.execute(
                    "INSERT INTO tags_new (id, cookbook_id, created_at) VALUES (?1, ?2, ?3)",
                    params![id, cookbook_id, tag_created.get(old_tag)],
                )
                .map_err(|e| e.to_string())?;
                id
            }
        };
        for (language, name, folded) in &names {
            let word = (cookbook_id.to_string(), language.clone(), folded.clone());
            if word_in_cookbook.contains_key(&word)
                || !languages_of_tag
                    .entry(target.clone())
                    .or_default()
                    .insert(language.clone())
            {
                continue;
            }
            conn.execute(
                "INSERT INTO tag_names_new (tag_id, cookbook_id, language, name, name_folded) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![target, cookbook_id, language, name, folded],
            )
            .map_err(|e| e.to_string())?;
            word_in_cookbook.insert(word, target.clone());
        }
        tag_map.insert(key, target.clone());
        Ok(target)
    };
    let filings: Vec<(String, String)> =
        rows(conn, "SELECT branch_id, tag_id FROM branch_tags", |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?;
    let mut refiled: Vec<(String, String)> = Vec::new();
    let mut filed_tags: HashSet<String> = HashSet::new();
    for (branch_id, tag_id) in &filings {
        let cookbook_id = branch_cookbook
            .get(branch_id)
            .ok_or_else(|| format!("branch_tags names a Branch that is not here: {branch_id}"))?;
        refiled.push((branch_id.clone(), tag_in(cookbook_id, tag_id)?));
        filed_tags.insert(tag_id.clone());
    }
    for (tag_id, kitchen_id, _) in &old_tags {
        if filed_tags.contains(tag_id) {
            continue;
        }
        if let Some(cookbook_id) = cookbook_for_kitchen(kitchen_id) {
            tag_in(&cookbook_id, tag_id)?;
        }
    }
    conn.execute("DELETE FROM branch_tags", [])
        .map_err(|e| e.to_string())?;
    for (branch_id, tag_id) in refiled {
        conn.execute(
            "INSERT OR IGNORE INTO branch_tags (branch_id, tag_id) VALUES (?1, ?2)",
            params![branch_id, tag_id],
        )
        .map_err(|e| e.to_string())?;
    }

    // ── Related Recipes go with their first recipe ───────────────────────────
    // Into one Cookbook: the one the older of the two recipes went to from
    // the Kitchen the link was made in, else to that Kitchen's keeper.
    let related: Vec<(String, String, String, String, String, String)> = rows(
        conn,
        "SELECT kitchen_id, lineage_a_id, lineage_b_id, lineage_a_name, lineage_b_name, created_at \
           FROM related_recipes",
        |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        },
    )?;
    // A link goes with the Cookbook of its first Lineage (#131, answer 9):
    // of the two recipes it joins, the one the Kitchen held first, and the
    // Cookbook its oldest Branch there went to. `old` is in the order the
    // Branches were made, so the first Branch found of either Lineage is
    // that one. Where the Kitchen held neither, its own Cookbook.
    for (kitchen_id, a, b, a_name, b_name, created_at) in related {
        let into = old
            .iter()
            .find(|branch| {
                branch.kitchen_id == kitchen_id
                    && (branch.lineage_id == a || branch.lineage_id == b)
            })
            .map(|branch| branch_cookbook[&branch.id].clone())
            .or_else(|| cookbook_for_kitchen(&kitchen_id));
        if let Some(cookbook_id) = into {
            conn.execute(
                "INSERT OR IGNORE INTO related_recipes_new \
                 (cookbook_id, lineage_a_id, lineage_b_id, lineage_a_name, lineage_b_name, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![cookbook_id, a, b, a_name, b_name, created_at],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    // ── The import ledger goes with the recipes it names ─────────────────────
    // One Import per Cookbook and kind of source, so re-running an importer
    // still finds what it brought in before (ADR 0025).
    let imports: Vec<(String, String, String, String)> = rows(
        conn,
        "SELECT id, kitchen_id, source_kind, created_at FROM imports",
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;
    let import_of: HashMap<String, (String, String, String)> = imports
        .iter()
        .map(|(id, kitchen, kind, at)| (id.clone(), (kitchen.clone(), kind.clone(), at.clone())))
        .collect();
    let mut import_map: HashMap<(String, String), String> = HashMap::new();
    let mut import_ids_used: HashSet<String> = HashSet::new();
    let mut import_in = |cookbook_id: &str, old_import: &str| -> Step<String> {
        let (_, kind, created_at) = &import_of[old_import];
        let key = (cookbook_id.to_string(), kind.clone());
        if let Some(found) = import_map.get(&key) {
            return Ok(found.clone());
        }
        let id = if import_ids_used.insert(old_import.to_string()) {
            old_import.to_string()
        } else {
            new_id("imp")
        };
        conn.execute(
            "INSERT INTO imports_new (id, cookbook_id, source_kind, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, cookbook_id, kind, created_at],
        )
        .map_err(|e| e.to_string())?;
        import_map.insert(key, id.clone());
        Ok(id)
    };
    let ledger: Vec<(String, String, String, String, String)> = rows(
        conn,
        "SELECT import_id, foreign_id, lineage_id, branch_id, created_at FROM import_ledger",
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    )?;
    let mut relogged: Vec<(String, String, String, String, String)> = Vec::new();
    let mut logged_imports: HashSet<String> = HashSet::new();
    for (import_id, foreign_id, lineage_id, branch_id, created_at) in ledger {
        let cookbook_id = branch_cookbook.get(&branch_id).ok_or_else(|| {
            format!("the import ledger names a Branch that is not here: {branch_id}")
        })?;
        relogged.push((
            import_in(cookbook_id, &import_id)?,
            foreign_id,
            lineage_id,
            branch_id,
            created_at,
        ));
        logged_imports.insert(import_id);
    }
    for (import_id, kitchen_id, _, _) in &imports {
        if logged_imports.contains(import_id) {
            continue;
        }
        if let Some(cookbook_id) = cookbook_for_kitchen(kitchen_id) {
            import_in(&cookbook_id, import_id)?;
        }
    }
    conn.execute("DELETE FROM import_ledger", [])
        .map_err(|e| e.to_string())?;
    for (import_id, foreign_id, lineage_id, branch_id, created_at) in relogged {
        conn.execute(
            "INSERT OR IGNORE INTO import_ledger (import_id, foreign_id, lineage_id, branch_id, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![import_id, foreign_id, lineage_id, branch_id, created_at],
        )
        .map_err(|e| e.to_string())?;
    }

    // ── The Home Kitchen goes ────────────────────────────────────────────────
    // A Home Kitchen nobody else ever joined was only ever the answer to
    // "where does this recipe go", and a Kitchen now holds nothing. One that
    // somebody else cooks in, or that has an Invite still waiting, stays as
    // the group it is.
    for (kitchen_id, owner) in &kitchens.home_owner {
        let members = kitchens
            .members
            .get(kitchen_id)
            .cloned()
            .unwrap_or_default();
        if members.iter().any(|member| member != owner) {
            continue;
        }
        let waiting: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM kitchen_invites WHERE kitchen_id = ?1 AND used_at IS NULL",
                params![kitchen_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if waiting > 0 {
            continue;
        }
        for statement in [
            "DELETE FROM kitchen_invites WHERE kitchen_id = ?1",
            "DELETE FROM kitchen_members WHERE kitchen_id = ?1",
            "DELETE FROM kitchens WHERE id = ?1",
        ] {
            conn.execute(statement, params![kitchen_id])
                .map_err(|e| e.to_string())?;
        }
    }

    // ── The new tables take the old ones' names ──────────────────────────────
    conn.execute_batch(
        r#"
        DROP TABLE branches;
        ALTER TABLE branches_new RENAME TO branches;
        CREATE UNIQUE INDEX branches_travelling_id_per_cookbook
            ON branches (COALESCE(travelling_id, id), cookbook_id);
        CREATE INDEX branches_by_cookbook ON branches (cookbook_id);
        CREATE INDEX branches_by_lineage ON branches (lineage_id);

        DROP TABLE tag_names;
        DROP TABLE tags;
        ALTER TABLE tags_new RENAME TO tags;
        ALTER TABLE tag_names_new RENAME TO tag_names;
        CREATE UNIQUE INDEX tag_names_one_word_per_cookbook
            ON tag_names (cookbook_id, language, name_folded);

        DROP TABLE related_recipes;
        ALTER TABLE related_recipes_new RENAME TO related_recipes;
        CREATE INDEX related_recipes_by_lineage_a ON related_recipes (cookbook_id, lineage_a_id);
        CREATE INDEX related_recipes_by_lineage_b ON related_recipes (cookbook_id, lineage_b_id);

        DROP TABLE imports;
        ALTER TABLE imports_new RENAME TO imports;

        ALTER TABLE people DROP COLUMN home_kitchen_id;

        -- Who may see which Cookbook (ADR 0041): its own Co-authors, and
        -- everybody who cooks in a Kitchen with one of them. The one place
        -- that answers *may see*, so the rule is said once and every
        -- Operation asks it the same way.
        CREATE VIEW visible_cookbooks (person_id, cookbook_id) AS
            SELECT person_id, cookbook_id FROM cookbook_authors
            UNION
            SELECT mine.person_id, authors.cookbook_id
              FROM kitchen_members AS mine
              JOIN kitchen_members AS theirs ON theirs.kitchen_id = mine.kitchen_id
              JOIN cookbook_authors AS authors ON authors.person_id = theirs.person_id;
        "#,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
