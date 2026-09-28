//! The home screen's shelves, and the shelf entry a recipe is shown as
//! wherever a list of recipes appears.

use super::*;

impl Core {
    /// **Home**: the computed shelves that answer *show me something* —
    /// recently added, cooked most, quick tonight, never cooked, recently
    /// opened (ADR 0011, ADR 0027, ADR 0042).
    ///
    /// Every shelf is one card per Lineage in the reader's Reading Language,
    /// built by the same [`shelf_entry`] the library is, so a recipe is the same
    /// object on both screens rather than two treatments of one.
    ///
    /// **An empty shelf is left out rather than shown empty.** A row that is
    /// sometimes there and sometimes not is honest; a permanently empty one
    /// teaches people to stop reading the screen. Where nothing is on the shelf
    /// at all, every one of the five is empty and the answer carries none —
    /// which is what lets the screen say *your shelf is empty* once instead of
    /// five times.
    ///
    /// *Never cooked* is **shuffled**, not sorted (#151). It used to be newest
    /// first, and then it repeated *recently added* card for card on any
    /// library nobody had cooked from yet. Shuffled, it is a different dozen
    /// of the recipes you have not made each time Home is asked for.
    ///
    /// All five are counted from what already exists — Attempts and the recipes
    /// themselves — except *recently opened*, which reads the one fact Home
    /// stores ([`Self::note_recipe_opened`]). So the whole screen is
    /// computed-on-top under ADR 0009 and free to be redesigned over a library
    /// that never changes for it.
    pub fn home_shelves(&self, person_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let reading_language = reading_language_of(conn, person_id)?;
            let (lineages, branches) = shelf_of(conn, person_id, None, &reading_language)?;
            let own_cookbook_id = cookbook_of_person(conn, person_id)?;

            // This Person's own cooking, and nobody else's. An Attempt is on the
            // Person rather than on the Kitchen (ADR 0005), and every other
            // screen that reads them — the diary, *My recipes* — is already
            // own-only, so *cooked most* means *I cook this most* and *never
            // cooked* means *I have never made this*. A Kitchen-wide count would
            // put a housemate's favourite on your Home under your name, and
            // empty *never cooked* of exactly the recipes you were owed.
            //
            // **Unfinished cookings count**, and so do In Progress ones: what
            // makes a cooking real is starting it (ADR 0010), which is the same
            // rule the diary shows them under.
            let mut cooked: HashMap<String, i64> = HashMap::new();
            {
                let mut statement = conn
                    .prepare(
                        "SELECT lineage_id, COUNT(*) FROM attempts \
                          WHERE person_id = ?1 GROUP BY lineage_id",
                    )
                    .map_err(|e| OpError::internal(format!("cannot count cookings: {e}")))?;
                let rows = statement
                    .query_map(params![person_id], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                    })
                    .map_err(|e| OpError::internal(format!("cannot count cookings: {e}")))?;
                for row in rows {
                    let (lineage_id, count) =
                        row.map_err(|e| OpError::internal(format!("cannot count cookings: {e}")))?;
                    cooked.insert(lineage_id, count);
                }
            }

            // What this Person opened, latest first — the one fact this screen
            // stores, and the only thing here that is not computed.
            //
            // Ordered by the database rather than in Rust, which is what the
            // index on (person_id, opened_at DESC) exists for. What is kept is
            // each Lineage's **place** in that order, not its timestamp: the
            // shelf only ever asks *which came before which*, and a position is
            // that question already answered — no instant has to be parsed back
            // out of a string to compare two of them.
            let mut opened: HashMap<String, usize> = HashMap::new();
            {
                let mut statement = conn
                    .prepare(
                        "SELECT lineage_id FROM recipe_opens WHERE person_id = ?1 \
                          ORDER BY opened_at DESC, lineage_id ASC",
                    )
                    .map_err(|e| OpError::internal(format!("cannot read what was opened: {e}")))?;
                let rows = statement
                    .query_map(params![person_id], |row| row.get::<_, String>(0))
                    .map_err(|e| OpError::internal(format!("cannot read what was opened: {e}")))?;
                for (place, row) in rows.enumerate() {
                    let lineage_id = row.map_err(|e| {
                        OpError::internal(format!("cannot read what was opened: {e}"))
                    })?;
                    opened.insert(lineage_id, place);
                }
            }

            // One pass over the shelf, building each recipe's card once and
            // sorting it into whichever shelves it belongs on. A recipe may
            // stand on several — quick *and* never cooked is the most useful
            // suggestion there is, so nothing here is exclusive.
            let mut added: Vec<ShelfCandidate> = Vec::new();
            let mut most: Vec<ShelfCandidate> = Vec::new();
            let mut quick: Vec<ShelfCandidate> = Vec::new();
            let mut never: Vec<ShelfCandidate> = Vec::new();
            let mut lately: Vec<ShelfCandidate> = Vec::new();

            for (rank, lineage_id) in lineages.iter().enumerate() {
                let of_lineage = &branches[lineage_id];
                let shown = &of_lineage[0];
                let content = version_content(conn, &shown.head_version_id)?;
                let title = content["title"].as_str().unwrap_or_default().to_string();
                let sorts_as = folded_for_search(&title);
                let card = shelf_entry(
                    conn,
                    &own_cookbook_id,
                    shown,
                    &title,
                    &reading_language,
                    &content,
                    // Nothing was searched for, so nothing matched. Home is a
                    // suggestion, not an answer to a question.
                    None,
                )?;

                let claim = |by: i64| ShelfCandidate {
                    by,
                    sorts_as: sorts_as.clone(),
                    card: card.clone(),
                };

                // Every recipe arrived once, so every one stands here and the
                // cap decides which. `lineages` arrives oldest first, so the
                // position in it is an age, and sorting that largest-first is
                // newest-first. The age is the Lineage's, from its oldest
                // Branch: a Translation or a variation of a recipe you already
                // had is not an arrival, and does not move it forward.
                added.push(claim(rank as i64));

                match cooked.get(lineage_id) {
                    Some(&count) => most.push(claim(count)),
                    // Placed by the shuffle below, once every candidate is in.
                    None => never.push(claim(0)),
                }

                // **A recipe Kamosu does not know the time for is not quick.**
                // 36 of the 86 real recipes carry no time at all, and calling
                // one of them quick would be guessing — the same refusal as
                // showing a quantity Kamosu could not read as though it had
                // (ADR 0002). One of the two times is enough to answer with.
                let prep = content["prep_time_minutes"].as_i64();
                let cook = content["cook_time_minutes"].as_i64();
                if prep.is_some() || cook.is_some() {
                    let total = prep.unwrap_or(0) + cook.unwrap_or(0);
                    if total <= QUICK_TONIGHT_MINUTES {
                        quick.push(claim(total));
                    }
                }

                if let Some(&place) = opened.get(lineage_id) {
                    lately.push(claim(place as i64));
                }
            }

            shuffle(&mut never, self.shelf_seed());

            // *Recently added* first, where Aurélien put it (#151): nobody has
            // to have cooked anything for it to have something to say, and a
            // recipe that just came in is the likeliest reason to open Home.
            // Then the spec's order, each shelf ordered by its own fact:
            // most-cooked first, soonest-ready first, shuffled, last-opened
            // first. An empty shelf is dropped here rather than at the screen,
            // so there is one place that decides it and both Doors get the
            // same answer.
            let mut shelves: Vec<Value> = Vec::new();
            if !added.is_empty() {
                shelves.push(home_shelf("recently_added", added, true));
            }
            if !most.is_empty() {
                shelves.push(home_shelf("cooked_most", most, true));
            }
            if !quick.is_empty() {
                shelves.push(home_shelf("quick_tonight", quick, false));
            }
            if !never.is_empty() {
                // The shuffle's place, smallest first.
                shelves.push(home_shelf("never_cooked", never, false));
            }
            if !lately.is_empty() {
                // Smallest place first — place 0 is the one opened last.
                shelves.push(home_shelf("recently_opened", lately, false));
            }

            Ok(json!({
                "quick_tonight_minutes": QUICK_TONIGHT_MINUTES,
                "shelves": shelves,
            }))
        })
    }

    /// What *never cooked* is shuffled by this time: a fresh seed from the
    /// operating system on every ask, unless a test pinned one.
    fn shelf_seed(&self) -> u64 {
        #[cfg(feature = "test-jobs")]
        if let Some(seed) = *self.shelf_seed.lock().expect("the shelf seed is poisoned") {
            return seed;
        }
        u64::from_le_bytes(random_bytes(8).try_into().expect("eight bytes"))
    }

    /// Test-only, compiled only under the `test-jobs` feature: shuffle *never
    /// cooked* by this seed from now on rather than a fresh one per ask, so a
    /// test can say which order it expects instead of asserting on chance.
    #[cfg(feature = "test-jobs")]
    pub fn pin_shelf_seed(&self, seed: u64) {
        *self.shelf_seed.lock().expect("the shelf seed is poisoned") = Some(seed);
    }
}

/// One Branch as the shelf holds it while it works out which Lineage it belongs
/// to and which of its Branches the card opens. The head Version never reaches
/// the answer; the Cookbook does, as the one the entry's Branch sits in (#177).
pub(super) struct ShelfBranch {
    pub(super) branch_id: String,
    pub(super) lineage_id: String,
    language: String,
    pub(super) head_version_id: String,
    cookbook_id: String,
    /// Held in the reader's own Cookbook, arrived there or written there,
    /// rather than in somebody else's: the same test as `own_first`.
    own: bool,
    /// Not a variation: the Branch a Cookbook keeps of a recipe without a name.
    unnamed: bool,
}

impl ShelfBranch {
    /// How strongly the card should open this Branch, largest first: the
    /// reader's Language, then their own, then an unnamed one before a
    /// variation — `own_first`'s order with the Language put ahead of it.
    /// Ties go to the oldest, which is the order the Branches arrive in.
    fn claim(&self, reading_language: &str) -> (bool, bool, bool) {
        (self.language == reading_language, self.own, self.unnamed)
    }
}

/// A shelf before anything is asked of it: the Lineages in the order their
/// oldest Branch was created, and every Branch of each one, the Branch its card
/// opens kept first.
type Shelf = (Vec<String>, HashMap<String, Vec<ShelfBranch>>);

/// Every Cookbook this Person may see — their own, and every one in a Kitchen
/// they cook in — gathered into one entry per Lineage: the shelf itself,
/// before anybody has asked anything of it (ADR 0027, ADR 0041).
///
/// Answers the Lineages in the order their oldest Branch was created, and for
/// each of them **every** Branch it has, the one the card opens kept first.
/// Those are two different questions: which Branch an entry *opens* is settled
/// here, and which Branches a search *reads* is the caller's business.
///
/// The card opens the Branch written in the reader's own Language; where the
/// Lineage has none, the oldest Branch answers and the entry says it fell back
/// — a Language preference must never hide a recipe from its owner (ADR 0006).
/// Within a Language it opens **the reader's own** Branch — the unnamed one,
/// before any variation — and otherwise the original, the oldest (ADR 0041).
/// Never the most recently changed, for the reason ADR 0027 refused
/// reordering behind your back. Filtered to one Kitchen this is the Cookbooks
/// seen in that Kitchen alone, because no other was selected.
///
/// Who may see a Cookbook is the whole boundary on a shelf and the only one
/// (ADR 0026), which is why neither caller asks a permission question again
/// further down: everything this returns is already what the reader may see.
pub(super) fn shelf_of(
    conn: &Connection,
    person_id: &str,
    kitchen_id: Option<&str>,
    reading_language: &str,
) -> Result<Shelf, OpError> {
    let mut statement = conn
        .prepare(&format!(
            "SELECT branches.id, branches.lineage_id, branches.language, \
                    branches.head_version_id, \
                    branches.cookbook_id IN \
                        (SELECT cookbook_id FROM cookbook_authors WHERE person_id = ?1), \
                    branches.name IS NULL, branches.cookbook_id \
               FROM branches \
              WHERE {} \
                AND (?2 IS NULL OR branches.cookbook_id IN ({})) \
              ORDER BY branches.created_at ASC, branches.id ASC",
            visible_to("branches", "?1"),
            cookbooks_seen_in("?2"),
        ))
        .map_err(|e| OpError::internal(format!("cannot read the shelf: {e}")))?;
    let held: Vec<ShelfBranch> = statement
        .query_map(params![person_id, kitchen_id], |row| {
            Ok(ShelfBranch {
                branch_id: row.get(0)?,
                lineage_id: row.get(1)?,
                language: row.get(2)?,
                head_version_id: row.get(3)?,
                own: row.get(4)?,
                unnamed: row.get(5)?,
                cookbook_id: row.get(6)?,
            })
        })
        .map_err(|e| OpError::internal(format!("cannot read the shelf: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read the shelf: {e}")))?;

    let mut lineages: Vec<String> = Vec::new();
    let mut branches: HashMap<String, Vec<ShelfBranch>> = HashMap::new();
    for branch in held {
        let lineage_id = branch.lineage_id.clone();
        let of_lineage = branches.entry(lineage_id.clone()).or_insert_with(|| {
            lineages.push(lineage_id);
            Vec::new()
        });
        // The Branch that opens the card is kept first, so choosing it is this
        // one comparison rather than a second pass. Only a strictly stronger
        // claim moves it, so among equals the oldest keeps its place.
        if of_lineage
            .first()
            .is_some_and(|first| branch.claim(reading_language) > first.claim(reading_language))
        {
            of_lineage.insert(0, branch);
        } else {
            of_lineage.push(branch);
        }
    }
    Ok((lineages, branches))
}

/// One recipe's claim on one of Home's shelves: the card itself, the fact that
/// shelf is ordered by, and the folded title that breaks a tie.
///
/// Each shelf sorts by something different — a count, a duration, an age, a
/// place in the opening order or in a shuffle — and none of those facts
/// reaches the answer. Rather than five differently-shaped tuples and a
/// function generic over which, the ordering fact is widened to one `i64` and
/// named here: every shelf's sort is then *largest first* or *smallest first*
/// over that one number.
struct ShelfCandidate {
    /// What this shelf orders by. Read only by the sort.
    by: i64,
    /// The title, folded the way the library's shelf folds it — the tie-break,
    /// so a shelf never reorders itself between two visits that changed
    /// nothing.
    sorts_as: String,
    card: Value,
}

/// Put *never cooked* in a random order, drawn from `seed`, by writing each
/// candidate's place into the number its shelf sorts by.
///
/// This runs **before** [`home_shelf`] cuts the shelf to length, so the dozen
/// shown are drawn from every recipe nobody has cooked, not the same dozen in
/// a new order. Every place is distinct, so the title never has to break a
/// tie. The seed is the Core's to hand in, which is how a test pins it.
fn shuffle(of: &mut [ShelfCandidate], seed: u64) {
    use rand::SeedableRng;
    use rand::seq::SliceRandom;

    of.shuffle(&mut rand::rngs::StdRng::seed_from_u64(seed));
    for (place, candidate) in of.iter_mut().enumerate() {
        candidate.by = place as i64;
    }
}

/// One of Home's shelves, ordered, named and cut to length.
///
/// `largest_first` is the whole difference between the five: *recently
/// added*, *cooked most* and *recently opened* want the largest number,
/// *quick tonight* and the shuffled *never cooked* the smallest, and
/// alphabetical breaks every tie either way.
fn home_shelf(name: &str, mut of: Vec<ShelfCandidate>, largest_first: bool) -> Value {
    of.sort_by(|a, b| {
        let by = if largest_first {
            b.by.cmp(&a.by)
        } else {
            a.by.cmp(&b.by)
        };
        by.then_with(|| a.sorts_as.cmp(&b.sorts_as))
    });
    json!({
        "name": name,
        "recipes": of
            .into_iter()
            .take(HOME_SHELF_SHOWN)
            .map(|candidate| candidate.card)
            .collect::<Vec<_>>(),
    })
}

/// How long a recipe may take, prep and cooking together, and still stand on
/// Home's *quick tonight* shelf.
///
/// Thirty minutes is the weeknight line in the kitchen and it earns its place
/// in the real library: of the 50 recipes in the 86-recipe export that state a
/// time at all, 27 come in at or under it — about half the timed library, which
/// is a shelf worth reading rather than a shelf of three or a shelf of forty.
const QUICK_TONIGHT_MINUTES: i64 = 30;

/// How many cards one Home shelf carries.
///
/// Home is a suggestion, not the library — that is what Recipes is for — so a
/// shelf stops well before it becomes something to search through. Twelve is
/// six rows of the two-column shelf, or a rail long enough to be worth pushing.
const HOME_SHELF_SHOWN: usize = 12;

/// One entry on the shelf, built the one way, so a match, a near miss and an
/// unsearched shelf cannot drift into describing the same recipe differently.
///
/// `own_cookbook_id` is the reader's own Cookbook, read once by the caller
/// rather than once per entry.
pub(super) fn shelf_entry(
    conn: &Connection,
    own_cookbook_id: &str,
    shown: &ShelfBranch,
    title: &str,
    reading_language: &str,
    content: &Value,
    matched: Option<Value>,
) -> Result<Value, OpError> {
    Ok(json!({
        "lineage_id": shown.lineage_id,
        "branch_id": shown.branch_id,
        "title": title,
        "language": shown.language,
        // Whether this entry is being read in a Language the reader did not ask
        // for — the mark, and the whole of the mark. There is nothing to say
        // when it is false.
        //
        // An **Unknown** recipe is never marked: it is honestly more than one
        // Language, so it is not in any Language the reader failed to ask for.
        // It shows to everyone exactly as it is, which is the whole of what
        // Unknown buys — no prompt, no badge, no nag (ADR 0006).
        "language_fallback": shown.language != reading_language
            && !crate::language::is_stated_unknown(&shown.language),
        "main_photo": content["main_photo"].clone(),
        "yield": content["yield"].clone(),
        "matched": matched,
        // Whose recipe this is and what the reader may do with it, worked out
        // exactly as `get_recipe` works them out, so a caller reading the whole
        // shelf knows in advance what the Core will decide (#177).
        "cookbook": cookbook_label(conn, &shown.cookbook_id)?,
        "writes": cookbook_writes_branch(conn, own_cookbook_id, &shown.branch_id)?,
        "mine": shown.cookbook_id == own_cookbook_id,
    }))
}
