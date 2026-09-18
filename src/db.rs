//! The SQLite file under `/data` — the truth (ADR 0003). WAL mode is a
//! requirement, not a detail: a second process (the recovery command) opens the
//! same file while the server runs.
//!
//! The schema moves forward only (ADR 0030): each change is one numbered
//! [`Migration`], applied in order at startup behind a **Snapshot** of the
//! database taken just before. There is no down-migration and no tool for one —
//! going back is putting the Snapshot file back and running the old image.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{Connection, OptionalExtension};

use crate::OpError;

pub const DATABASE_FILE: &str = "kamosu.db";

/// One numbered step on the only path the schema walks: forward (ADR 0030).
/// Written carefully once — there is no inverse to get wrong.
#[derive(Debug, Clone, Copy)]
pub struct Migration {
    /// Where this step leaves the schema. Steps apply strictly in order.
    pub version: i64,
    /// What the step does, named for the log and the failure message.
    pub description: &'static str,
    /// The step itself. Runs inside one transaction, together with the
    /// `schema_version` stamp, so a failure leaves nothing half-done.
    pub sql: &'static str,
}

/// Every migration, oldest first. Appending a new tail entry *is* the upgrade;
/// nothing earlier is ever rewritten.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        description: "the base schema: People and their Access Keys",
        sql: r#"
        CREATE TABLE IF NOT EXISTS people (
            id         TEXT PRIMARY KEY,
            name       TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );

        -- An Access Key: the Secret a Person mints for an agent to act with.
        -- 256 bits of randomness, stored hashed, shown once at minting, revocable,
        -- ending only when spent or revoked — never on a clock (ADR 0031).
        CREATE TABLE IF NOT EXISTS access_keys (
            secret_hash TEXT PRIMARY KEY,
            person_id   TEXT NOT NULL REFERENCES people(id),
            name        TEXT NOT NULL,
            read_only   INTEGER NOT NULL DEFAULT 0,
            revoked     INTEGER NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            last_used_at TEXT
        );
        "#,
    },
    Migration {
        version: 2,
        description: "the Job shape: slow work answers an id at once",
        sql: r#"
        -- A Job: slow work asked for through an Operation. The row is the truth
        -- about the work — its state, its progress, and its result or the reason
        -- it failed — so it survives the request that started it and is read back
        -- through ordinary Operations at both Doors. A Job ends only in a terminal
        -- status; it never vanishes.
        CREATE TABLE IF NOT EXISTS jobs (
            id                TEXT PRIMARY KEY,
            person_id         TEXT REFERENCES people(id),
            read_only         INTEGER NOT NULL DEFAULT 0,
            operation         TEXT NOT NULL,
            input             TEXT NOT NULL,
            status            TEXT NOT NULL DEFAULT 'queued',
            progress_done     INTEGER,
            progress_total    INTEGER,
            progress_message  TEXT,
            result            TEXT,
            error             TEXT,
            error_code        INTEGER,
            created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            updated_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );
        "#,
    },
    Migration {
        version: 3,
        description: "the first Person, their Home Kitchen, and account preferences",
        sql: r#"
        ALTER TABLE people ADD COLUMN password_hash TEXT;
        ALTER TABLE people ADD COLUMN home_kitchen_id TEXT;
        ALTER TABLE people ADD COLUMN reading_language TEXT NOT NULL DEFAULT 'en';
        ALTER TABLE people ADD COLUMN reading_measures TEXT NOT NULL DEFAULT 'us';
        ALTER TABLE people ADD COLUMN is_operator INTEGER NOT NULL DEFAULT 0;
        CREATE TABLE IF NOT EXISTS kitchens (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            hand_id     TEXT NOT NULL UNIQUE,
            created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );
        CREATE TABLE IF NOT EXISTS kitchen_members (
            kitchen_id TEXT NOT NULL REFERENCES kitchens(id),
            person_id  TEXT NOT NULL REFERENCES people(id),
            PRIMARY KEY (kitchen_id, person_id)
        );
        CREATE TABLE IF NOT EXISTS instance_setup (
            singleton          INTEGER PRIMARY KEY CHECK (singleton = 1),
            operator_person_id TEXT REFERENCES people(id)
        );
        CREATE TABLE IF NOT EXISTS sessions (
            id           TEXT PRIMARY KEY,
            secret_hash  TEXT NOT NULL UNIQUE,
            person_id    TEXT NOT NULL REFERENCES people(id),
            name         TEXT NOT NULL,
            revoked      INTEGER NOT NULL DEFAULT 0,
            created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            last_used_at TEXT
        );
        CREATE TABLE IF NOT EXISTS login_failures (
            name             TEXT PRIMARY KEY,
            consecutive_failures INTEGER NOT NULL DEFAULT 0
        );
        "#,
    },
    Migration {
        version: 4,
        description: "a login name names exactly one password-bearing Person",
        sql: r#"
        CREATE UNIQUE INDEX people_login_names_unique
            ON people(name) WHERE password_hash IS NOT NULL;
        "#,
    },
    Migration {
        version: 5,
        description: "an Access Key is known by an id, not its secret hash",
        sql: r#"
        ALTER TABLE access_keys ADD COLUMN id TEXT;
        UPDATE access_keys SET id = 'ak_' || lower(hex(randomblob(8))) WHERE id IS NULL;
        CREATE UNIQUE INDEX access_keys_id_unique ON access_keys(id);
        "#,
    },
    Migration {
        version: 6,
        description: "a Job remembers whether an Access Key asked for it",
        sql: r#"
        ALTER TABLE jobs ADD COLUMN via_access_key INTEGER NOT NULL DEFAULT 0;
        "#,
    },
    Migration {
        version: 7,
        description: "Kitchen nicknames and Kitchen invites",
        sql: r#"
        -- A Nickname: one member's own private relabelling of a Kitchen, seen
        -- by nobody else and never travelling (ADR 0007).
        ALTER TABLE kitchen_members ADD COLUMN nickname TEXT;

        -- A Kitchen Invite: a one-use link a member mints, spent the moment
        -- another Person opens it and joins (CONTEXT.md, "Invite").
        CREATE TABLE IF NOT EXISTS kitchen_invites (
            id          TEXT PRIMARY KEY,
            secret_hash TEXT NOT NULL UNIQUE,
            kitchen_id  TEXT NOT NULL REFERENCES kitchens(id),
            created_by  TEXT NOT NULL REFERENCES people(id),
            created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            used_at     TEXT,
            used_by     TEXT REFERENCES people(id)
        );
        "#,
    },
    Migration {
        version: 8,
        description: "account links and account states",
        sql: r#"
        ALTER TABLE people ADD COLUMN disabled INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE people ADD COLUMN deleted INTEGER NOT NULL DEFAULT 0;
        CREATE TABLE account_links (
            secret_hash TEXT PRIMARY KEY,
            kind TEXT NOT NULL CHECK (kind IN ('invite', 'recovery')),
            person_id TEXT REFERENCES people(id),
            is_operator INTEGER NOT NULL DEFAULT 0,
            spent INTEGER NOT NULL DEFAULT 0,
            revoked INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );
        "#,
    },
    Migration {
        version: 9,
        description: "Lineage, Branch and Version: a recipe's identity (ADR 0004)",
        sql: r#"
        -- A Lineage: a recipe's identity in the world. One id, minted once,
        -- never joined to another (ADR 0004).
        CREATE TABLE lineages (
            id         TEXT PRIMARY KEY,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );

        -- A Version: one saved state, named by a fingerprint of the words and
        -- Photographs alone (ADR 0004, ADR 0021) — never the Reading, the Hand,
        -- its name, its what-changed line, or the time. Content-addressed and
        -- global: two Branches that reach identical content hold the same row
        -- without having communicated, which is what makes a Branch Point a
        -- computable fact rather than an assertion.
        CREATE TABLE versions (
            id         TEXT PRIMARY KEY,
            content    TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );

        -- A Branch: one Kitchen's line of Versions within a Lineage.
        -- `signature` is a slot the acceptance criteria ask for and v1 leaves
        -- unused — nothing here signs it or reads it.
        CREATE TABLE branches (
            id              TEXT PRIMARY KEY,
            lineage_id      TEXT NOT NULL REFERENCES lineages(id),
            kitchen_id      TEXT NOT NULL REFERENCES kitchens(id),
            hand_id         TEXT NOT NULL,
            language        TEXT NOT NULL,
            origin_address  TEXT,
            signature       TEXT,
            head_version_id TEXT NOT NULL REFERENCES versions(id),
            created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );

        -- One Branch's occurrence of one Version in its chain: everything a
        -- content hash cannot carry — who wrote it, what it is named, what
        -- changed and why, and (local only, never shared) which Access Key
        -- wrote it. The same Version id may occur on more than one Branch, or
        -- more than once on the same Branch, each with its own parent and
        -- Hand (ADR 0015, ADR 0021) — convergence is about content, never
        -- about authorship. `sequence` is the append-only chain order; a
        -- rapid re-save collapses by replacing the row at the current head
        -- sequence rather than appending one.
        CREATE TABLE branch_versions (
            branch_id          TEXT NOT NULL REFERENCES branches(id),
            sequence           INTEGER NOT NULL,
            version_id         TEXT NOT NULL REFERENCES versions(id),
            parent_version_id  TEXT REFERENCES versions(id),
            hand_id            TEXT NOT NULL,
            name               TEXT,
            change_note        TEXT,
            access_key_id      TEXT REFERENCES access_keys(id),
            created_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            PRIMARY KEY (branch_id, sequence)
        );
        "#,
    },
    Migration {
        version: 10,
        description: "the Reading: Kamosu's guess about an Ingredient Line (ADR 0021)",
        sql: r#"
        -- A Reading: Kamosu's interpretation of one Ingredient Line, addressed
        -- by its position in that Version's `ingredients` array. It travels
        -- beside the Version it belongs to, never inside its content, so
        -- correcting one mints no Version (ADR 0021) — this row is simply
        -- replaced or deleted in place. `target` names a Food by its written
        -- word alone; Foods themselves (#47) are not built yet.
        CREATE TABLE readings (
            version_id  TEXT NOT NULL REFERENCES versions(id),
            line_index  INTEGER NOT NULL,
            amount      TEXT,
            unit        TEXT,
            target      TEXT,
            updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            PRIMARY KEY (version_id, line_index)
        );
        "#,
    },
    Migration {
        version: 11,
        description: "the Tag: how a Kitchen files its own cookbook (ADR 0035)",
        sql: r#"
        -- A Tag: a word a Kitchen describes its recipes by. Kept once per
        -- Kitchen and pointed at by every recipe of that Kitchen which uses
        -- it, so renaming one reaches all of them at once (ADR 0007). Flat:
        -- no hierarchy, and no parent column to grow one.
        CREATE TABLE tags (
            id         TEXT PRIMARY KEY,
            kitchen_id TEXT NOT NULL REFERENCES kitchens(id),
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );

        -- A Tag's name in one Language. Named per Language rather than split
        -- by it (ADR 0006), so *dessert* and *dessert* are one Tag wearing two
        -- words rather than two Tags. `kitchen_id` is carried here as well as
        -- on `tags` purely so one word can be held unique within a Kitchen by
        -- an index rather than by a read-then-write race.
        --
        -- `name` is what the cook typed, kept exactly as typed and always what
        -- is shown. `name_folded` is that word reduced to the form two spellings
        -- of one word share — Unicode canonical caseless matching, computed in
        -- Rust by `folded_word` — and exists only to be compared. Storing the
        -- fold rather than folding at query time is what lets one word be held
        -- unique by an index: SQLite's own NOCASE folds ASCII alone, which
        -- would file "Été" and "été" as two Tags in a cookbook whose Languages
        -- are English, French and Spanish.
        CREATE TABLE tag_names (
            tag_id      TEXT NOT NULL REFERENCES tags(id),
            kitchen_id  TEXT NOT NULL REFERENCES kitchens(id),
            language    TEXT NOT NULL,
            name        TEXT NOT NULL,
            name_folded TEXT NOT NULL,
            PRIMARY KEY (tag_id, language)
        );

        -- One word, one Tag, within one Kitchen and one Language.
        CREATE UNIQUE INDEX tag_names_one_word_per_kitchen
            ON tag_names(kitchen_id, language, name_folded);

        -- Which recipes carry which Tag. On the Branch — the recipe as this
        -- Kitchen holds it — and deliberately not inside a Version's content:
        -- filing is not what a recipe is, so tagging mints no Version and
        -- moves no fingerprint (ADR 0035).
        CREATE TABLE branch_tags (
            branch_id TEXT NOT NULL REFERENCES branches(id),
            tag_id    TEXT NOT NULL REFERENCES tags(id),
            PRIMARY KEY (branch_id, tag_id)
        );
        CREATE INDEX branch_tags_by_tag ON branch_tags(tag_id);
        "#,
    },
    Migration {
        version: 12,
        description: "Related Recipes: one shelf-local, two-way Lineage link (#52)",
        sql: r#"
        -- A Related Recipe is a shelf note, not part of either recipe: it links
        -- two distinct Lineages in one Kitchen, has neither type nor direction,
        -- and never enters a Version, fingerprint, Bundle, or Share. The saved
        -- names remain readable if either Lineage later leaves this shelf.
        CREATE TABLE related_recipes (
            kitchen_id     TEXT NOT NULL REFERENCES kitchens(id),
            lineage_a_id   TEXT NOT NULL REFERENCES lineages(id),
            lineage_b_id   TEXT NOT NULL REFERENCES lineages(id),
            lineage_a_name TEXT NOT NULL,
            lineage_b_name TEXT NOT NULL,
            created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            PRIMARY KEY (kitchen_id, lineage_a_id, lineage_b_id),
            CHECK (lineage_a_id < lineage_b_id)
        );
        CREATE INDEX related_recipes_by_lineage_a ON related_recipes(kitchen_id, lineage_a_id);
        CREATE INDEX related_recipes_by_lineage_b ON related_recipes(kitchen_id, lineage_b_id);
        "#,
    },
    Migration {
        version: 13,
        description: "the Photograph: known by its own contents (#45, ADR 0017)",
        sql: r#"
        -- A Photograph: identified by the picture itself, never a name or a
        -- place. This row only records that the hash exists; the bytes live
        -- under /data/photographs, named by the same hash, so this table is
        -- what makes "does this Photograph already exist" a cheap question
        -- rather than a directory listing.
        CREATE TABLE photographs (
            hash       TEXT PRIMARY KEY,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );
        "#,
    },
    Migration {
        version: 14,
        description: "the Food: matched from a Reading's word, doubt makes a new one (#47, ADR 0022)",
        sql: r#"
        -- A Food: an edible thing Kamosu knows about, created automatically
        -- from whatever word a Reading found and shared across every recipe
        -- that uses it. Instance-wide — no kitchen_id — because matching a
        -- Food to nutrition is the expensive part and worth doing once
        -- (ADR 0022). `cup_weight_grams` is the one figure that turns a
        -- volume into a weight; `nutrition` is a foundation slot for the
        -- CIQUAL binding #12 deferred past v1 — no Operation writes it yet,
        -- and it always reads back null. Neither ever travels in a Bundle.
        CREATE TABLE foods (
            id               TEXT PRIMARY KEY,
            cup_weight_grams REAL,
            nutrition        TEXT,
            created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );

        -- A Food's name in one Language, at most one per Language (ADR 0006)
        -- — flour and farine are one Food wearing two words. Unlike
        -- tag_names, deliberately carries no unique index on the word: doubt
        -- makes a new Food, never a merge (ADR 0022), so the same folded
        -- word can legitimately end up naming two different Foods once a
        -- collision has happened, or once a name has been typed onto a Food
        -- another already answers to. That duplication is itself the
        -- evidence a Merge Suggestion (#48) will record.
        CREATE TABLE food_names (
            food_id     TEXT NOT NULL REFERENCES foods(id),
            language    TEXT NOT NULL,
            name        TEXT NOT NULL,
            name_folded TEXT NOT NULL,
            PRIMARY KEY (food_id, language)
        );
        CREATE INDEX food_names_by_word ON food_names(language, name_folded);

        -- Which Food a Reading's target word currently resolves to —
        -- internal bookkeeping alone. A Reading's public shape stays the
        -- bare word (ADR 0021: "no id"), so this column is never read by
        -- set_reading's output or carried into a Bundle. It exists so the
        -- busiest-Food tie-break (ADR 0022) can be answered by counting
        -- rather than re-matching.
        ALTER TABLE readings ADD COLUMN food_id TEXT REFERENCES foods(id);
        "#,
    },
    Migration {
        version: 15,
        description: "the Attempt: one cooking, In Progress on the server (#57, ADR 0005, ADR 0010)",
        sql: r#"
        -- An Attempt: one person's record of one cooking. Belongs to a
        -- Lineage rather than a Branch, so cooking the dish is remembered
        -- however it later diverges (ADR 0005), and is pinned by
        -- `version_id` to the fingerprint of the Branch's head Version at
        -- the moment cooking started — a later edit to the recipe never
        -- turns this Attempt into a lie.
        --
        -- `current_step_index`, `ticked_ingredients` (a JSON array of
        -- indices) and `cooking_yield` (a JSON `{amount, noun}` object, or
        -- null) are the In Progress state itself: where the cook has got
        -- to, held on the server so one cooking follows its cook from
        -- phone to iPad (ADR 0010). `finished_at` is null throughout that
        -- and stamped once, deliberately or by simply stopping; an
        -- Attempt is real and counts as a cooking from the moment it is
        -- inserted, finished or not. `note` and `rating` are the free
        -- text and optional five-star score CONTEXT.md's Attempt holds,
        -- editable at any time by the cook. Never soft-deleted: a `DELETE`
        -- is the whole of how an Attempt is undone (ADR 0010).
        CREATE TABLE attempts (
            id                  TEXT PRIMARY KEY,
            lineage_id          TEXT NOT NULL REFERENCES lineages(id),
            person_id           TEXT NOT NULL REFERENCES people(id),
            version_id          TEXT NOT NULL REFERENCES versions(id),
            current_step_index  INTEGER NOT NULL DEFAULT 0,
            ticked_ingredients  TEXT NOT NULL DEFAULT '[]',
            cooking_yield       TEXT,
            note                TEXT,
            rating              INTEGER,
            finished_at         TEXT,
            created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            last_action_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );

        -- One Person may have at most one Attempt In Progress per Lineage
        -- (ADR 0010) — enforced here rather than merely attempted in Rust,
        -- so "which cook do you mean" stays unaskable even under two
        -- devices racing to start at once.
        CREATE UNIQUE INDEX attempts_one_in_progress_per_lineage
            ON attempts(lineage_id, person_id) WHERE finished_at IS NULL;

        CREATE INDEX attempts_by_person_lineage ON attempts(person_id, lineage_id);
        "#,
    },
    Migration {
        version: 16,
        description: "the Import ledger: foreign id to Lineage, matched not doubled (#68, ADR 0025)",
        sql: r#"
        -- An Import: one durable channel a Kitchen brings recipes in through,
        -- for one kind of outside source — a Crouton export, a web page, a
        -- Bundle. Lazily created the first time that Kitchen runs an
        -- importer of that kind, and reused by every run after: that is what
        -- lets an importer be re-run instead of feared while its source is
        -- still around (ADR 0025).
        CREATE TABLE imports (
            id          TEXT PRIMARY KEY,
            kitchen_id  TEXT NOT NULL REFERENCES kitchens(id),
            source_kind TEXT NOT NULL,
            created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            UNIQUE (kitchen_id, source_kind)
        );

        -- The ledger itself: the foreign id an Import read, against the
        -- Lineage and Branch it became. Held by the Import and never by the
        -- recipe (ADR 0025) — in no fingerprint and no Bundle, and gone the
        -- moment its Import is deleted, the recipes it named untouched.
        CREATE TABLE import_ledger (
            import_id  TEXT NOT NULL REFERENCES imports(id),
            foreign_id TEXT NOT NULL,
            lineage_id TEXT NOT NULL REFERENCES lineages(id),
            branch_id  TEXT NOT NULL REFERENCES branches(id),
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            PRIMARY KEY (import_id, foreign_id)
        );
        "#,
    },
    Migration {
        version: 17,
        description: "the orphan sweep's grace mark on a Photograph (#46, ADR 0017)",
        sql: r#"
        -- When the sweep last found this Photograph referenced by nothing, or
        -- NULL while something still points at it.
        --
        -- This is not a reference count, and the distinction is the whole
        -- point of the design. A count is written at attach and detach time,
        -- so a missed decrement leaks a picture for ever and a missed
        -- increment DELETES ONE STILL ON SCREEN. This column is instead
        -- *recomputed from scratch* by every sweep: the sweep works out what
        -- is referenced by reading the Versions, and then writes this column
        -- to match. It records only how long the current unreferenced spell
        -- has lasted, so a Photograph detached and re-attached inside the
        -- grace week has the mark cleared again and survives.
        --
        -- Wrong values are therefore self-correcting: the next sweep
        -- overwrites them from the truth. A count's errors accumulate.
        ALTER TABLE photographs ADD COLUMN unreferenced_since TEXT;
        "#,
    },
    Migration {
        version: 18,
        description: "a Translation's Versions: what each translates, and in which Language (#56, ADR 0006)",
        sql: r#"
        -- Which Version of the source Branch this Version renders. Null on an
        -- ordinary recipe — the original is simply the Branch that translates
        -- nothing, computed rather than declared (ADR 0006). Set on every
        -- Version of a Translation, and carried forward by an ordinary edit,
        -- so how far behind a Translation has fallen is arithmetic over
        -- current facts: the source Branch has moved this many Versions past
        -- the one this points at. It sits on the occurrence rather than on the
        -- Branch precisely so that arithmetic is exact at every point in the
        -- history, and so it travels in a Bundle with the Version it belongs
        -- to.
        ALTER TABLE branch_versions ADD COLUMN translates_version_id TEXT REFERENCES versions(id);

        -- The Language the Branch stood in when this Version was written.
        -- Duplicated from `branches.language` on purpose: changing a Language
        -- makes a Version (ADR 0006), and a Version that recorded no Language
        -- would leave that change indistinguishable from a save that did
        -- nothing — a label alterable without a trace, which is the hole the
        -- rule exists to close.
        ALTER TABLE branch_versions ADD COLUMN language TEXT;

        -- Every Version already written stood in its Branch's Language: there
        -- has been no way to change one until now, so this is the truth and
        -- not a guess.
        UPDATE branch_versions
           SET language = (SELECT language FROM branches WHERE branches.id = branch_versions.branch_id);

        CREATE INDEX branch_versions_by_translated_version
            ON branch_versions(translates_version_id) WHERE translates_version_id IS NOT NULL;
        "#,
    },
    Migration {
        version: 19,
        description: "the Merge Suggestion: evidence two Foods are one, never an instruction (#48, ADR 0022)",
        sql: r#"
        -- A Merge Suggestion: a note that two Foods on this instance are
        -- probably one thing, kept with the reason it was made. Evidence and
        -- never an instruction — nothing in Kamosu reads this table and acts
        -- on it, and only an Operator may merge (ADR 0022).
        --
        -- The pair is unordered, held that way by the same trick
        -- `related_recipes` uses: the smaller id first, enforced by a CHECK,
        -- so "A and B" and "B and A" cannot both be recorded. `words` is the
        -- evidence itself as JSON — the arriving names that hit two Foods, or
        -- the single name typed onto a Food another already answers to — so
        -- an Operator reads *why* rather than comparing names by eye.
        --
        -- Merging clears every suggestion naming either Food, and so does
        -- deleting one: a suggestion pointing at a Food that no longer exists
        -- is not evidence, it is a dangling row.
        CREATE TABLE merge_suggestions (
            food_a_id  TEXT NOT NULL REFERENCES foods(id),
            food_b_id  TEXT NOT NULL REFERENCES foods(id),
            reason     TEXT NOT NULL,
            words      TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            PRIMARY KEY (food_a_id, food_b_id),
            CHECK (food_a_id < food_b_id)
        );
        CREATE INDEX merge_suggestions_by_food_b ON merge_suggestions(food_b_id);
        "#,
    },
    Migration {
        version: 20,
        description: "a rating is a verdict, and an Attempt carries Photographs (#59, ADR 0015)",
        sql: r#"
        -- A rating is one of three verdicts about next time — again, tweak,
        -- no — rather than a score out of five (#59). The cooking work that
        -- shipped first (#57) stored a whole number 1..=5 because a scale had
        -- to be picked before the question was asked; this is the answer.
        --
        -- Three words cannot be averaged, even informally by a reader doing
        -- the arithmetic in their head, which is what makes ADR 0015's refusal
        -- to average self-evident on screen instead of a rule people have to
        -- be told. The public field keeps the name `rating`, because that is
        -- the word the spec and every screen use for it.
        ALTER TABLE attempts RENAME COLUMN rating TO rating_out_of_five;
        ALTER TABLE attempts ADD COLUMN rating TEXT
            CHECK (rating IS NULL OR rating IN ('again', 'tweak', 'no'));

        -- No instance can have had a five-star rating long: the scale existed
        -- only between #57 and this ticket. Converting anyway rather than
        -- dropping the column and declaring the case impossible — a migration
        -- that silently discards somebody's judgement because it was probably
        -- not there is exactly the kind of forward-only step there is no
        -- undoing (ADR 0030). Four and five are enthusiasm, three is the
        -- honest middle, one and two are not again.
        UPDATE attempts SET rating = CASE
            WHEN rating_out_of_five >= 4 THEN 'again'
            WHEN rating_out_of_five  = 3 THEN 'tweak'
            WHEN rating_out_of_five <= 2 THEN 'no'
        END WHERE rating_out_of_five IS NOT NULL;

        ALTER TABLE attempts DROP COLUMN rating_out_of_five;

        -- The Photographs taken during this cooking, as a JSON array of
        -- Photograph ids, in the order they were added — stored the same way
        -- the ticked Ingredients are, since both are a short list belonging to
        -- one Attempt and read back whole.
        --
        -- They hang off the Attempt and NOT off any Version, which is the
        -- whole of why a picture from somebody's kitchen cannot reach a Share
        -- Link: a share renders a Version, and there is no path from a Version
        -- to an Attempt. Promoting one to the Main Photo or to a Step's photo
        -- is the single deliberate way a picture crosses that line, and it is
        -- an ordinary edit making a Version (#59).
        ALTER TABLE attempts ADD COLUMN photographs TEXT NOT NULL DEFAULT '[]';
        "#,
    },
    Migration {
        version: 21,
        description: "Meaning Search: the acceptance, and the index it earns (#63, ADR 0029)",
        sql: r#"
        -- Meaning Search's one row. Kamosu ships no model (ADR 0029), so every
        -- instance begins here, in 'unasked', searching by words and complete.
        --
        --   unasked  — nobody has been asked yet, so the offer is live.
        --   declined — an Operator said no. The offer never appears again.
        --   accepted — the terms are accepted; the weights may not be here yet.
        --   on       — accepted, downloaded, indexed, and answering searches.
        --
        -- The Hand that accepted is kept, and how it arrived: by login or by
        -- Access Key. ADR 0029 records who accepted and deliberately does not
        -- pretend to check who they were — the same house rule ADR 0015 states
        -- about every Hand in Kamosu. An agent holding a Key its Person minted
        -- may accept, because minting the Key was the act of authorising it.
        CREATE TABLE meaning_search (
            id                      INTEGER PRIMARY KEY CHECK (id = 1),
            state                   TEXT NOT NULL
                                    CHECK (state IN ('unasked','declined','accepted','on')),
            accepted_by             TEXT REFERENCES people(id),
            accepted_via_access_key INTEGER,
            accepted_at             TEXT,
            -- Which issue of the terms, and which weights, were agreed to.
            -- Kept so that what somebody said yes to can always be told apart
            -- from what this build asks today — which is the fact a later
            -- revision of Gemma's terms would have to be answered against.
            terms_version           TEXT,
            terms_url               TEXT,
            model_repository        TEXT,
            model_revision          TEXT,
            -- Who declined, and when. A record rather than a switch: what
            -- turns the offer off is the state, and this is the answer to
            -- *who decided that* — the same question `accepted_by` answers on
            -- the other side of it.
            declined_by             TEXT REFERENCES people(id),
            declined_at             TEXT,
            -- What the vectors below were produced by, and when. Written for
            -- somebody reading this database directly: the stamp that actually
            -- governs is the one on each row, so nothing reads this back.
            indexed_with            TEXT,
            indexed_at              TEXT
        );
        INSERT INTO meaning_search (id, state) VALUES (1, 'unasked');

        -- The index. Every row here is **derived** from the recipes and can be
        -- rebuilt at any time, which is the whole of why turning Meaning Search
        -- off discards nothing (ADR 0003, ADR 0009).
        --
        -- Two roles, and the difference is the promise ADR 0027 makes:
        --   'block' — what ranking sees. The recipe's own blocks, cut at its
        --             own Sections, because cutting per line scatters the
        --             signal across many weak vectors.
        --   'line'  — what a matched block quotes. These never enter a ranking;
        --             they only choose which line to show among lines already
        --             known to be inside a relevant recipe. That separation is
        --             why display stays precise however coarsely a recipe is
        --             cut for retrieval.
        --
        -- `person_id` is set only on an Attempt's rows: the index reaches each
        -- Person's own Attempts and nobody else's (ADR 0027). Ownership sits on
        -- the row rather than being asked at query time, so there is no
        -- permission check inside a search and no partial rebuild when somebody
        -- changes their mind.
        CREATE TABLE meaning_vectors (
            id          TEXT PRIMARY KEY,
            role        TEXT NOT NULL CHECK (role IN ('block','line')),
            block_id    TEXT REFERENCES meaning_vectors(id),
            lineage_id  TEXT NOT NULL,
            -- Set on a recipe's rows; null on an Attempt's.
            branch_id   TEXT,
            version_id  TEXT,
            -- Set on an Attempt's rows; null on a recipe's.
            attempt_id  TEXT,
            person_id   TEXT,
            -- What produced this vector: which weights, at which revision, at
            -- which width. A row stamped with anything else belongs to a
            -- different embedding space, and comparing it against today's
            -- would answer nonsense while looking perfectly healthy — so a
            -- model change prunes rather than mixes.
            embedding_space TEXT NOT NULL,
            section     TEXT,
            -- Exactly the text this vector was made from — a block's body on
            -- a 'block' row, one line on a 'line' row. Kept rather than
            -- recomputed because it is what tells a stale row from a current
            -- one: an Attempt's note can be corrected without the Attempt
            -- getting a new id, and without this the index would go on
            -- answering with the sentence somebody rewrote.
            embedded_text TEXT NOT NULL,
            -- On a 'line' row: where this line sits, in the same vocabulary a
            -- word match answers in. A reader cannot tell a meaning match from
            -- a word match by looking, and should not have to.
            where_      TEXT,
            step_number INTEGER,
            vector      BLOB NOT NULL
        );
        CREATE INDEX meaning_vectors_blocks ON meaning_vectors(role, lineage_id);
        CREATE INDEX meaning_vectors_lines ON meaning_vectors(block_id);
        CREATE INDEX meaning_vectors_by_branch ON meaning_vectors(branch_id);
        CREATE INDEX meaning_vectors_by_attempt ON meaning_vectors(attempt_id);
        "#,
    },
    Migration {
        version: 22,
        description: "recently opened: when each Person last opened each Lineage (#64, ADR 0027)",
        sql: r#"
        -- The one fact Home stores. Everything else on that screen is computed
        -- from recipes and Attempts that already exist (ADR 0009); this is the
        -- exception ADR 0027 named when it added the *recently opened* shelf.
        --
        -- It is **private and local, and it never travels**: it is in no
        -- fingerprint, no Vault, no Bundle and no Share Link. An instance that
        -- lost this table entirely would lose the order of one shelf and
        -- nothing else, which is why nothing anywhere reads it back except
        -- Home.
        --
        -- One row per Person per Lineage rather than a log of every opening:
        -- the shelf asks *what was I last looking at*, and the answer to that
        -- is one timestamp. A log would grow without bound to answer a question
        -- nobody asks.
        CREATE TABLE recipe_opens (
            person_id  TEXT NOT NULL REFERENCES people(id),
            lineage_id TEXT NOT NULL REFERENCES lineages(id),
            opened_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            PRIMARY KEY (person_id, lineage_id)
        );

        -- The shelf's one query, exactly: this Person's openings, latest first.
        -- Home reads them in this order rather than sorting them itself, which
        -- is what makes the DESC half of this index load-bearing rather than
        -- decorative.
        CREATE INDEX recipe_opens_by_person ON recipe_opens(person_id, opened_at DESC);
        "#,
    },
    Migration {
        version: 23,
        description: "which Branches hold a Version: the lookup a collapse asks (#82)",
        sql: r#"
        -- Every save now asks whether another Branch of the same Lineage names
        -- the Version it is about to replace, because collapsing one that a
        -- Copy is holding severs the two Branches for good (#82). The Lineage
        -- is reached by joining `branches`, so the row this index finds is what
        -- the join starts from; without it the question is a scan of every
        -- Version of every recipe on the instance, on the save path. The
        -- sibling index on `translates_version_id` answers the other half of
        -- the same condition.
        CREATE INDEX branch_versions_by_version ON branch_versions(version_id);
        "#,
    },
    Migration {
        version: 24,
        description: "Share Links, and the instance's public address (#65, ADR 0026)",
        sql: r#"
        -- A Share Link: one permanent, unguessable address for one Branch.
        -- There are exactly two levels of visibility (ADR 0026) — a recipe is
        -- seen by its Kitchen, or by anyone holding its link — so this table
        -- IS the second level, and its presence or absence is the whole of it.
        -- There is no `visibility` column anywhere because there is no scale.
        --
        -- Stored as a hash of the Secret, like every other Secret in Kamosu:
        -- a stolen database yields no working links. What is stored beside it
        -- is a **token**, never a URL (#65) — the instance's public address
        -- lives in one place below, so setting it later makes every link
        -- already minted render correctly rather than leaving a generation of
        -- links pointing at whatever the address was on the day.
        --
        -- `ended_at` rather than a boolean, and rows are never deleted: ending
        -- a link is permanent, and turning sharing back on mints a NEW row
        -- (ADR 0018). The dead row stays so the old Secret keeps resolving to
        -- "this link was ended" instead of to nothing — the same recipe, and
        -- an honest answer rather than a 404 that reads like a mistake.
        --
        -- `shared_by` is a **Person**, not a Kitchen. A Kitchen's Hand says who
        -- wrote a Branch; sharing is an act somebody takes, and the page names
        -- them ("Shared by Aurélien"). A Kitchen's Nickname could never appear
        -- here in any case — it is private to the member who set it
        -- (CONTEXT.md, "Kitchen").
        CREATE TABLE share_links (
            id          TEXT PRIMARY KEY,
            branch_id   TEXT NOT NULL REFERENCES branches(id),
            secret_hash TEXT NOT NULL UNIQUE,
            shared_by   TEXT NOT NULL REFERENCES people(id),
            created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            ended_at    TEXT
        );

        -- One live link per Branch, enforced rather than remembered. Ended rows
        -- are exempt, which is what lets a Branch accumulate a history of dead
        -- links while never having two that work.
        CREATE UNIQUE INDEX share_links_one_live_per_branch
            ON share_links(branch_id) WHERE ended_at IS NULL;

        -- The instance's public address: where a Share Link is reachable from
        -- outside. Asked once, at the first Share Link, and kept HERE rather
        -- than in an environment variable (#65) — an env var would make the
        -- address a property of how the process happened to be started, and a
        -- link minted under one spelling would render wrongly under the next.
        ALTER TABLE instance_setup ADD COLUMN public_address TEXT;
        "#,
    },
    Migration {
        version: 25,
        description: "the Shopping List: the choosing, and nothing computed (#73, ADR 0024)",
        sql: r#"
        -- **The choosing is stored; the rows are computed** (ADR 0024). There
        -- is no table of Shopping Rows here and there never will be: a stored
        -- row is a second copy of a fact the recipe already holds, and it goes
        -- stale the first time anybody corrects a Reading.
        --
        -- There is no `shopping_lists` table either. One Person has exactly
        -- one list, it has no name, it is never archived, and it is always
        -- there — so a row saying "this list exists" would carry no fact.
        -- The list IS these two tables keyed on the Person.
        CREATE TABLE shopping_choices (
            person_id  TEXT NOT NULL REFERENCES people(id),
            -- A **Branch**, never a Lineage, and never a pinned Version
            -- (ADR 0024): a shopping trip is about one particular text, and a
            -- friend's ratatouille may want anchovies where yours does not.
            -- Always that Branch's latest Version, so a recipe edited on
            -- Monday is right on Tuesday.
            branch_id  TEXT NOT NULL REFERENCES branches(id),
            -- The Yield being shopped for, as the recipe's own two fields:
            -- {"amount", "noun"}, exactly as `attempts.cooking_yield` holds
            -- the Yield being cooked. NULL is the recipe as written, which is
            -- what a recipe added without a thought about it means.
            shopping_yield TEXT,
            -- **The name the recipe was known by when it was chosen.** Read
            -- only once the recipe can no longer be reached — the Kitchen
            -- holding it is no longer one this Person cooks in, or the Branch
            -- is gone. A thing that quietly disappears from a shopping list is
            -- a thing that does not get bought (ADR 0024), so the entry stays,
            -- keeps this name, contributes nothing, and says so.
            known_as   TEXT NOT NULL,
            chosen_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            PRIMARY KEY (person_id, branch_id)
        );

        -- The list's own query: this Person's choosing, in the order they made
        -- it, which is the order the screen shows the recipes in.
        CREATE INDEX shopping_choices_by_person ON shopping_choices(person_id, chosen_at);

        -- A **Loose Item**: a line typed straight onto the list, belonging to
        -- no recipe. Kept exactly as typed and never interpreted, so it has no
        -- Food, no amount and merges with nothing (ADR 0024) — reading `500g
        -- flour` out of something typed at the door means guessing at a number
        -- about to be shopped by. There is no `folded` column for the same
        -- reason: nothing here is ever matched against anything.
        CREATE TABLE shopping_loose_items (
            id         TEXT PRIMARY KEY,
            person_id  TEXT NOT NULL REFERENCES people(id),
            text       TEXT NOT NULL,
            added_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );

        CREATE INDEX shopping_loose_items_by_person
            ON shopping_loose_items(person_id, added_at);
        "#,
    },
    Migration {
        version: 26,
        description: "the Component: a Reading whose target is a Recipe (#50, ADR 0008)",
        sql: r#"
        -- **A Reading's target is either a Food or a Lineage** (ADR 0008,
        -- ADR 0021). `target` holds the Food's written word; this holds the
        -- Lineage a Component names. Exactly one of the two is set, which the
        -- Core enforces — there is no third state where a line claims to be
        -- both a flour and a dough.
        --
        -- **DELIBERATELY NO FOREIGN KEY, and this is the whole point of the
        -- column.** A Lineage id is global by construction (ADR 0004), so it
        -- means the same thing on every instance — and a Component must go on
        -- naming its recipe when this instance does not hold it: deleted with
        -- no ceremony, arrived in a Bundle without its passenger, or never
        -- received at all. `REFERENCES lineages(id)` would refuse to store
        -- exactly the pointer ADR 0008 exists to keep, and turn "the row still
        -- reads correctly" into a write that fails. The dangling pointer is
        -- the feature: the written line carries the human meaning either way
        -- (ADR 0002), so an unresolvable Component degrades into a sentence
        -- rather than a hole.
        ALTER TABLE readings ADD COLUMN lineage_id TEXT;

        -- Answering "which lines of this Version are Components" without
        -- reading every Reading of it. Unfolding walks this on every read of
        -- every recipe, to whatever depth the composition goes.
        CREATE INDEX readings_by_lineage ON readings(lineage_id)
            WHERE lineage_id IS NOT NULL;
        "#,
    },
    Migration {
        version: 27,
        description: "the As Cooked: a Version that never joined a Branch (#58, ADR 0005)",
        sql: r#"
        -- **An As Cooked is a row in `versions` that no `branch_versions` row
        -- points at.** ADR 0005 says it is "structurally identical to a
        -- Version, differing only in that it never joined a Branch, never
        -- becomes part of the recipe's history, and never travels" — and
        -- because a Version is named by a fingerprint of its own content, that
        -- sentence needs no table of its own to be true. This column is the
        -- whole of the storage.
        --
        -- Three things fall out of it rather than being built:
        --
        --   · **The common case stays free.** NULL is cooked as written, which
        --     is the overwhelming majority, and it costs one nullable column.
        --   · **Two cooks who made the identical change hold the identical
        --     row**, without either knowing about the other. The fingerprint
        --     did that, not this ticket.
        --   · **Promotion mints no new identity.** The As Cooked's id already
        --     IS the Version id, so promoting appends a `branch_versions` row
        --     naming a Version that has existed since the cook wrote it.
        --
        -- REFERENCES versions(id), unlike the Component's Lineage pointer next
        -- door, because this one is local by construction: the Version was
        -- written on this instance, into this table, by the cook. Nothing
        -- deletes a Version (ADR 0004), so the reference cannot dangle.
        ALTER TABLE attempts ADD COLUMN as_cooked_version_id TEXT
            REFERENCES versions(id);
        "#,
    },
    Migration {
        version: 28,
        description: "a Promotion declined stays declined (#58)",
        sql: r#"
        -- When the cook said the words they cooked belong in the diary and
        -- **not** in the recipe.
        --
        -- The offer needs an answer that survives, because Kamosu's position on
        -- being asked twice is already written down: a question already
        -- answered, asked twice, is a nag (the same reason `meaning_search`
        -- keeps `declined_at`). Without this the recipe page would ask about
        -- the same Tuesday every time it was opened.
        --
        -- It says nothing about the As Cooked itself, which stays exactly where
        -- it is: declining is a decision about the RECIPE, and the cooking
        -- record is untouched by it. Deciding the other way later is an
        -- ordinary edit of one's own Attempt (ADR 0005), so this clears.
        ALTER TABLE attempts ADD COLUMN promotion_declined_at TEXT;
        "#,
    },
    Migration {
        version: 29,
        description: "every Version re-fingerprinted, once and never again (#89, ADR 0038)",
        sql: r#"
        -- **The one time Kamosu rewrites a Version id**, and the reason there
        -- will not be a second (ADR 0038).
        --
        -- #72 added `nutrition` to a recipe. Nothing rewrote what was already
        -- stored -- every row in this table still hashed to the id it sits
        -- under -- but every recipe saved afterwards carried a field the
        -- earlier ones did not, so the same recipe written either side of that
        -- day held two different ids. Content addressing is the whole of how
        -- two people who never spoke are known to hold the same state
        -- (ADR 0004), and a field addition was quietly breaking it.
        --
        -- ADR 0038 closes it at the root: a field holding nothing is no part
        -- of the fingerprint. A field added later starts out empty on every
        -- recipe already written, so it drops out of the hash and moves no id
        -- -- for nutrition, and for whatever v2 adds after it. This step is
        -- the cost of adopting that rule with 142 Versions already written.
        --
        -- It runs behind the ordinary pre-migration Snapshot, inside the one
        -- transaction every step gets, with foreign keys deferred to COMMIT --
        -- ids move in every table at once or in none of them.
        --
        -- **It is also the one exception to migration 27's "nothing deletes a
        -- Version".** That sentence is still true of everything Kamosu does
        -- while running, and it is why `attempts.as_cooked_version_id` may
        -- reference `versions(id)` at all. Here two rows holding one recipe
        -- become one row, and every reference to the one that goes is moved to
        -- the one that stays before it does -- so nothing is left pointing at
        -- a Version that is not there. Migrations are never rewritten, so that
        -- comment stays where it is and this is where the exception is
        -- recorded.
        PRAGMA defer_foreign_keys = ON;

        -- Old id to new. NOT NULL is the guard: `version_fingerprint` answers
        -- NULL on content it cannot read, and a library with one unreadable
        -- Version must stop here with the Snapshot intact rather than remap
        -- around it.
        CREATE TEMP TABLE refingerprint (
            old_id TEXT PRIMARY KEY,
            new_id TEXT NOT NULL
        );
        INSERT INTO temp.refingerprint (old_id, new_id)
            SELECT id, version_fingerprint(content) FROM versions;

        -- Two rows can land on one new id: the same recipe written before and
        -- after a field appeared is now, correctly, one Version. This picks
        -- which row survives. The candidates hold the same words by
        -- construction, so the ordering only decides which copy of them stays:
        -- the one carrying the most Readings first, since a Reading is
        -- authored data that would otherwise be dropped (ADR 0021), then the
        -- fuller content, then the lower id so the choice is deterministic.
        CREATE TEMP TABLE refingerprint_keep (
            new_id TEXT PRIMARY KEY,
            old_id TEXT NOT NULL
        );
        INSERT INTO temp.refingerprint_keep (new_id, old_id)
            SELECT new_id, old_id FROM (
                SELECT r.new_id AS new_id, r.old_id AS old_id,
                       ROW_NUMBER() OVER (
                           PARTITION BY r.new_id
                           ORDER BY (SELECT COUNT(*) FROM readings g
                                      WHERE g.version_id = r.old_id) DESC,
                                    LENGTH(v.content) DESC,
                                    r.old_id ASC
                       ) AS standing
                  FROM temp.refingerprint r
                  JOIN versions v ON v.id = r.old_id
            ) WHERE standing = 1;

        -- Readings move onto the surviving row. The survivor was chosen for
        -- holding the MOST Readings, not all of them, so this genuinely
        -- carries lines across rather than only tidying up. OR IGNORE skips a
        -- line the survivor already has a Reading for -- the two rows are the
        -- same recipe, so one of the two is redundant either way -- and the
        -- delete below clears whatever stayed behind. It suppresses any
        -- constraint on the row, not only the (version_id, line_index) key.
        UPDATE OR IGNORE readings
           SET version_id = (SELECT k.old_id
                               FROM temp.refingerprint r
                               JOIN temp.refingerprint_keep k ON k.new_id = r.new_id
                              WHERE r.old_id = readings.version_id);
        DELETE FROM readings
         WHERE version_id NOT IN (SELECT old_id FROM temp.refingerprint_keep);

        -- Meaning Search's vectors for a row that is going away are discarded
        -- rather than moved: the index is derived and rebuilds from the
        -- recipes (AGENTS.md), so merging two sets of vectors onto one Version
        -- would double every hit for nothing. A block and its lines share a
        -- version_id and therefore go together, leaving no dangling block_id.
        DELETE FROM meaning_vectors
         WHERE version_id IS NOT NULL
           AND version_id NOT IN (SELECT old_id FROM temp.refingerprint_keep);

        DELETE FROM versions
         WHERE id NOT IN (SELECT old_id FROM temp.refingerprint_keep);

        -- Every reference, then the Versions themselves. A dropped row's old
        -- id still maps, and maps to the id the survivor is about to carry, so
        -- anything that pointed at it now points at the row that replaced it.
        UPDATE branches
           SET head_version_id = (SELECT new_id FROM temp.refingerprint
                                   WHERE old_id = head_version_id);
        UPDATE branch_versions
           SET version_id = (SELECT new_id FROM temp.refingerprint
                              WHERE old_id = version_id);
        -- The nullable pointers are matched against the map rather than merely
        -- tested for NULL. A scalar subquery that finds nothing answers NULL,
        -- so a pointer naming a Version that is not in the map -- there should
        -- be none, but this is forward-only and has no inverse (ADR 0030) --
        -- would be quietly erased rather than left alone. The NOT NULL columns
        -- below need no such guard: NULL fails their constraint, the
        -- transaction rolls back, and the Snapshot is still there.
        UPDATE branch_versions
           SET parent_version_id = (SELECT new_id FROM temp.refingerprint
                                     WHERE old_id = parent_version_id)
         WHERE parent_version_id IN (SELECT old_id FROM temp.refingerprint);
        UPDATE branch_versions
           SET translates_version_id = (SELECT new_id FROM temp.refingerprint
                                         WHERE old_id = translates_version_id)
         WHERE translates_version_id IN (SELECT old_id FROM temp.refingerprint);
        UPDATE readings
           SET version_id = (SELECT new_id FROM temp.refingerprint
                              WHERE old_id = version_id);
        UPDATE attempts
           SET version_id = (SELECT new_id FROM temp.refingerprint
                              WHERE old_id = version_id);
        UPDATE attempts
           SET as_cooked_version_id = (SELECT new_id FROM temp.refingerprint
                                        WHERE old_id = as_cooked_version_id)
         WHERE as_cooked_version_id IN (SELECT old_id FROM temp.refingerprint);
        UPDATE meaning_vectors
           SET version_id = (SELECT new_id FROM temp.refingerprint
                              WHERE old_id = version_id)
         WHERE version_id IN (SELECT old_id FROM temp.refingerprint);
        UPDATE versions
           SET id = (SELECT new_id FROM temp.refingerprint WHERE old_id = versions.id);

        -- **Two saves of one recipe are one entry in the Thread.** Where the
        -- rows that collapsed sat next to each other in a Branch's chain, the
        -- later one now names the same Version as the earlier and records
        -- ITSELF as the Version it came from -- which is not a thing a history
        -- can say (ADR 0004). Such a row is the save that, under ADR 0038,
        -- changed nothing, so it goes.
        --
        -- Its keeper is the nearest earlier row in that Branch holding the
        -- same Version and not itself going away. Nearest rather than first,
        -- because a Branch may legitimately hold one Version at two places --
        -- reverting to an earlier state is an ordinary save -- and a run of
        -- three collapsing rows must all land on the one survivor.
        CREATE TEMP TABLE refingerprint_thread (
            branch_id     TEXT NOT NULL,
            gone_sequence INTEGER NOT NULL,
            gone_name     TEXT,
            gone_note     TEXT,
            keeper_sequence INTEGER
        );
        INSERT INTO temp.refingerprint_thread
            SELECT gone.branch_id, gone.sequence, gone.name, gone.change_note,
                   (SELECT MAX(keeper.sequence) FROM branch_versions keeper
                     WHERE keeper.branch_id = gone.branch_id
                       AND keeper.version_id = gone.version_id
                       AND keeper.sequence < gone.sequence
                       AND (keeper.parent_version_id IS NULL
                            OR keeper.parent_version_id <> keeper.version_id))
              FROM branch_versions gone
             WHERE gone.parent_version_id = gone.version_id;

        -- The *what changed* line is the only place the why of an edit ever
        -- comes from, and nothing can infer it later (ADR 0004) -- so a note
        -- or a name written on the row going away is carried to the keeper
        -- wherever the keeper has none of its own.
        UPDATE branch_versions
           SET name = COALESCE(name, (SELECT t.gone_name FROM temp.refingerprint_thread t
                                       WHERE t.branch_id = branch_versions.branch_id
                                         AND t.keeper_sequence = branch_versions.sequence
                                       ORDER BY t.gone_sequence LIMIT 1)),
               change_note = COALESCE(change_note,
                                      (SELECT t.gone_note FROM temp.refingerprint_thread t
                                        WHERE t.branch_id = branch_versions.branch_id
                                          AND t.keeper_sequence = branch_versions.sequence
                                        ORDER BY t.gone_sequence LIMIT 1))
         WHERE EXISTS (SELECT 1 FROM temp.refingerprint_thread t
                        WHERE t.branch_id = branch_versions.branch_id
                          AND t.keeper_sequence = branch_versions.sequence);

        DELETE FROM branch_versions
         WHERE EXISTS (SELECT 1 FROM temp.refingerprint_thread t
                        WHERE t.branch_id = branch_versions.branch_id
                          AND t.gone_sequence = branch_versions.sequence);

        -- An Attempt pinned to a Version that merged into another now names
        -- the surviving row, and an As Cooked that was only ever a deviation
        -- because of the missing field is now the Version it was cooked from.
        -- That is what "cooked exactly as written stores nothing" (ADR 0005)
        -- was always supposed to mean, so clear it.
        UPDATE attempts
           SET as_cooked_version_id = NULL
         WHERE as_cooked_version_id IS NOT NULL
           AND as_cooked_version_id = version_id;

        DROP TABLE temp.refingerprint_thread;
        DROP TABLE temp.refingerprint_keep;
        DROP TABLE temp.refingerprint;
        "#,
    },
    Migration {
        version: 30,
        description: "the name a Hand arrived under, for a Hand minted elsewhere (#67)",
        sql: r#"
        -- A Hand is a name and a permanent id (CONTEXT.md, "Hand"). On the
        -- instance that minted it the name is looked up live, from `people`
        -- or `kitchens`, so renaming yourself reaches all your history at
        -- once. A Hand that arrived in a Bundle has no row in either: "anywhere
        -- else the name that arrived is all there will ever be" — and this is
        -- where that name is kept, so a received recipe's Thread and every
        -- reshare of it still say who wrote it.
        --
        -- Never a Hand this instance minted: a returning Bundle that carries
        -- one does not get to rename a Person or a Kitchen here.
        CREATE TABLE arrived_hands (
            hand_id TEXT PRIMARY KEY,
            name    TEXT NOT NULL
        );
        "#,
    },
];

/// The newest step [`MIGRATIONS`] carries: what this binary understands.
pub const LATEST_SCHEMA_VERSION: i64 = MIGRATIONS[MIGRATIONS.len() - 1].version;

/// The database handle shared by the Core.
pub struct Db {
    conn: Mutex<Connection>,
    data_dir: std::path::PathBuf,
}

impl Db {
    /// Open (creating if needed) the database inside `data_dir`, turn on WAL and
    /// the pragmas Kamosu depends on, and bring the schema forward along the
    /// shipped [`MIGRATIONS`].
    pub fn open(data_dir: &Path) -> Result<Db, OpError> {
        Self::open_with_migrations(data_dir, MIGRATIONS)
    }

    /// The machinery of [`Self::open`] with the migration steps chosen by the
    /// caller: tests build databases at an *old* schema by truncating the list,
    /// and force failures by appending a broken step. Startup always uses the
    /// full shipped list.
    pub fn open_with_migrations(data_dir: &Path, migrations: &[Migration]) -> Result<Db, OpError> {
        std::fs::create_dir_all(data_dir).map_err(|e| {
            OpError::internal(format!(
                "cannot create data directory {}: {e}",
                data_dir.display()
            ))
        })?;
        let conn = Connection::open(data_dir.join(DATABASE_FILE))
            .map_err(|e| OpError::internal(format!("cannot open database: {e}")))?;
        Self::initialise(conn, data_dir, migrations)
    }

    pub fn initialise(
        conn: Connection,
        data_dir: &Path,
        migrations: &[Migration],
    ) -> Result<Db, OpError> {
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| OpError::internal(format!("WAL mode refused: {e}")))?;
        // The documented pairing for WAL: safe and fast.
        conn.pragma_update(None, "synchronous", "NORMAL")
            .map_err(|e| OpError::internal(format!("synchronous refused: {e}")))?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| OpError::internal(format!("foreign_keys refused: {e}")))?;
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| OpError::internal(format!("busy timeout refused: {e}")))?;
        // `version_fingerprint(content)` — what names a Version, available to
        // SQL. Registered before the migrations run because one of them needs
        // it (version 29), and kept afterwards because the invariant it
        // computes is worth being able to ask a live database about.
        crate::fingerprint::register(&conn)
            .map_err(|e| OpError::internal(format!("cannot teach SQLite the fingerprint: {e}")))?;
        let db = Db {
            conn: Mutex::new(conn),
            data_dir: data_dir.to_path_buf(),
        };
        db.with_conn(|conn| migrate(conn, data_dir, migrations))?;
        Ok(db)
    }

    /// Run one closure with the connection. The database is behind a mutex, so
    /// every access serialises; that is plenty for v1 and honest about it.
    pub fn with_conn<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, OpError>,
    ) -> Result<T, OpError> {
        let conn = self.conn.lock().expect("database lock poisoned");
        f(&conn)
    }

    /// Where Kamosu's truth lives: one SQLite file under the one data directory.
    pub fn database_path(&self) -> std::path::PathBuf {
        self.data_dir.join(DATABASE_FILE)
    }

    /// The one data directory everything durable lives under (ADR 0028) — the
    /// database beside it, Photographs and Display Copies below it.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
}

fn latest_step(migrations: &[Migration]) -> i64 {
    migrations.last().map(|m| m.version).unwrap_or(0)
}

fn stored_schema_version(conn: &Connection) -> Result<i64, OpError> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read schema version: {e}")))?;
    raw.map(|v| v.parse().map_err(|_| ()))
        .unwrap_or(Ok(0))
        .map_err(|_| {
            OpError::internal(
                "the schema_version recorded in meta is not a number; \
                 this file may not be a Kamosu database",
            )
        })
}

fn migrate(conn: &Connection, data_dir: &Path, migrations: &[Migration]) -> Result<(), OpError> {
    // The ledger itself: infrastructure beneath every step, never migrated.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    )
    .map_err(|e| OpError::internal(format!("cannot prepare the schema ledger: {e}")))?;

    let current = stored_schema_version(conn)?;
    let latest = latest_step(migrations);

    // An older binary against a newer database stops here, loudly. Running old
    // code over a new cookbook is how recipes get quietly mangled (ADR 0030).
    if current > latest {
        return Err(OpError::internal(format!(
            "this database stands at schema version {current}, which is NEWER \
             than what this Kamosu understands (up to version {latest}). \
             An upgrade went past this binary. Start the newer Kamosu again — \
             do not run this older image against it"
        )));
    }

    let pending: Vec<&Migration> = migrations.iter().filter(|m| m.version > current).collect();
    if pending.is_empty() {
        return Ok(());
    }

    // Before any migration runs, copy the database aside. It is the database
    // alone because a Photograph cannot change once it exists (ADR 0017) — the
    // file alone is a complete way back.
    let snapshot = write_snapshot(conn, data_dir, current, latest)?;

    for step in pending {
        // Schema change and version stamp commit together or not at all: SQLite
        // rolls them back as one, so a failed step leaves exactly what was there.
        let batch = format!(
            "BEGIN IMMEDIATE;\n{}\nINSERT INTO meta(key, value) VALUES ('schema_version', '{}') \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value;\nCOMMIT;",
            step.sql, step.version
        );
        if let Err(e) = conn.execute_batch(&batch) {
            return Err(OpError::internal(format!(
                "migration {} ({}) failed: {}. Kamosu refuses to serve \
                 half-migrated; nothing was changed. To go back, stop Kamosu and \
                 put the Snapshot back over {} (and remove any -wal/-shm \
                 neighbours of it first) — copy the Snapshot from {}",
                step.version,
                step.description,
                e,
                data_dir.join(DATABASE_FILE).display(),
                snapshot.display()
            )));
        }
    }
    tracing::info!(
        from = current,
        to = latest,
        snapshot = %snapshot.display(),
        "schema migrated forward; Snapshot kept beside the database"
    );
    Ok(())
}

/// Copy the database file aside under `/data`, naming the span of versions it
/// undoes. WAL is checkpointed first so the one file is complete on its own.
fn write_snapshot(
    conn: &Connection,
    data_dir: &Path,
    from: i64,
    to: i64,
) -> Result<PathBuf, OpError> {
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|e| OpError::internal(format!("cannot checkpoint before snapshot: {e}")))?;
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let dest = data_dir.join(format!("kamosu-snapshot-v{from}-to-v{to}-{seconds}.db"));
    std::fs::copy(data_dir.join(DATABASE_FILE), &dest).map_err(|e| {
        OpError::internal(format!(
            "cannot write the pre-migration Snapshot to {}: {e}",
            dest.display()
        ))
    })?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wal_mode_sticks() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(dir.path()).unwrap();
        let mode: String = db
            .with_conn(|c| {
                Ok(c.query_row("PRAGMA journal_mode", [], |r| r.get(0))
                    .unwrap())
            })
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal");
    }
}
