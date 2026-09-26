//! Shopping Rows: what a **Shopping List** says once its recipes have been
//! read (ADR 0024, #73).
//!
//! **The choosing is stored; the rows are computed.** Nothing here is ever
//! written down — `core` holds the choosing, hands this module one
//! [`Contribution`] per Ingredient Line of every chosen recipe, and gets back
//! the rows to answer with. Run it twice and it says the same thing twice;
//! correct a Reading in between and it says something else, which is the whole
//! reason a row is not stored.
//!
//! ## The one thing this file decides
//!
//! **How many amounts a row carries, and which.** Contributions fall into
//! buckets that can honestly be added to each other and never across:
//!
//! - **mass** — grams, however they were written. A customary volume of a Food
//!   with a Cup Weight lands here too, which is the one crossing ADR 0016
//!   allows and the whole reason a Cup Weight exists.
//! - **volume** — millilitres.
//! - **count** — a number with no Unit at all: `3 large eggs`.
//! - **as written** — a number with a Unit outside the closed convertible set,
//!   kept per word: `4 cloves`, `2 poignées`. Two recipes' *cloves* add — and
//!   so do one recipe's *clove* and another's *cloves*, which is all
//!   [`unit_key`] exists for. A clove and a handful never do. A reader who
//!   asked for her measures to be
//!   left **as written** puts every Unit here, known or not: leaving a Unit
//!   alone and converting it are the same act, and she asked for neither.
//! - **no amount** — a line carrying no quantity at all, which is 28% of real
//!   Ingredient Lines (#5). It says *some* and rides as one more thing that
//!   will not add, rather than being dropped: dropping it means coming home
//!   short.
//!
//! A row therefore carries between one and several [`Part`]s. One is the
//! ordinary case; more than one is the row that could not be added, and
//! ADR 0024's rule is that it says both rather than one wrong number.
//!
//! ## Where *about* goes, and where it does not
//!
//! **The mass and volume buckets always say *about*; the count, as-written and
//! no-amount buckets never do.** ADR 0016 makes *about* unconditional on the subordinate
//! line because Kamosu rounded, every time — and mass and volume are exactly
//! the buckets Kamosu converted and rounded. A count and a cook's own word
//! passed through untouched: `4 cloves` is four cloves, and saying *about four
//! cloves* would claim an arithmetic that never happened. ADR 0024 and #73 both
//! word it this way — "added together… saying *about*; side by side where they
//! do not".
//!
//! This is not the mark ADR 0015 warns about. *about* does not fire sometimes
//! within one kind of amount; it fires for exactly the kind of amount Kamosu
//! computed, always, and a row never shows the same amount both ways.

use serde_json::{Value, json};

use crate::units::{self, Measures};

/// What one Ingredient Line of one chosen recipe puts into the list.
///
/// `amount` is already parsed and **already scaled** to the Yield being
/// shopped for: scaling is a fact about the choosing, which is `core`'s, and
/// by the time a contribution arrives here there is no Yield left to consult.
#[derive(Debug, Clone)]
pub struct Contribution {
    /// The recipe this came from, by the title it carries now. Shown on a row
    /// that had to break open, so the shopper can see which dish wants which.
    pub recipe: String,
    /// The written line, verbatim (ADR 0002) — what the tap reveals.
    pub text: String,
    pub branch_id: String,
    /// The amount, scaled. `None` is a line nobody put a number on.
    pub amount: Option<f64>,
    /// The Unit exactly as the cook wrote it, whether or not Kamosu knows it.
    pub unit: Option<String>,
    /// The Food's effective Cup Weight, where it has one.
    pub cup_weight_grams: Option<f64>,
}

/// One amount on a row, and which recipes put it there.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    /// `about` · `count` · `as_written` · `no_amount`. Named rather than styled:
    /// the screen decides what each looks like, this decides what each *is*.
    pub kind: &'static str,
    /// The amount as a reader reads it, in their Language and measures.
    pub text: String,
    /// The recipes that fed this amount, in the order they were chosen.
    pub sources: Vec<String>,
}

impl Part {
    fn to_json(&self) -> Value {
        json!({ "kind": self.kind, "text": self.text, "sources": self.sources })
    }
}

/// Lay a Food's contributions out as the amounts its row carries.
///
/// The order is fixed rather than incidental — mass, volume, count, the cook's
/// own words, then *some* — so two readings of one list put the same amount
/// first, and so the amount Kamosu is most confident about leads.
pub fn parts_for(contributions: &[Contribution], measures: Measures, language: &str) -> Vec<Part> {
    let mut grams = 0.0f64;
    let mut millilitres = 0.0f64;
    let mut count = 0.0f64;
    // A Unit Kamosu does not convert, kept per folded word so two spellings of
    // one word add and two different words never do. Accents are not folded
    // away, here or anywhere else in Kamosu: `maïs` is not `mais`.
    let mut as_written: Vec<AsWritten> = Vec::new();
    let mut mass_from: Vec<String> = Vec::new();
    let mut volume_from: Vec<String> = Vec::new();
    let mut count_from: Vec<String> = Vec::new();
    let mut unstated_from: Vec<String> = Vec::new();

    for contribution in contributions {
        let Some(amount) = contribution.amount else {
            remember(&mut unstated_from, &contribution.recipe);
            continue;
        };
        let word = contribution.unit.as_deref();
        // A reader who asked for her Units left alone gets them left alone: a
        // known Unit takes the same road as an unknown one, so nothing is
        // converted and identical words still add.
        let known = word
            .filter(|_| measures != Measures::AsWritten)
            .and_then(units::recognise);
        match (known, word) {
            // A number with no Unit at all is a count of things.
            (None, None) => {
                count += amount;
                remember(&mut count_from, &contribution.recipe);
            }
            // The number adds, the word is untouched (ADR 0016).
            (None, Some(word)) => {
                add_as_written(&mut as_written, word, amount, &contribution.recipe)
            }
            (Some(unit), _) => {
                let base = amount * unit.base;
                match units::in_grams(base, unit, measures, contribution.cup_weight_grams) {
                    Some(mass) => {
                        grams += mass;
                        remember(&mut mass_from, &contribution.recipe);
                    }
                    None => {
                        millilitres += base;
                        remember(&mut volume_from, &contribution.recipe);
                    }
                }
            }
        }
    }

    let mut parts = Vec::new();
    if !mass_from.is_empty() {
        parts.push(Part {
            kind: "about",
            text: units::worded_mass(grams, measures, language),
            sources: mass_from,
        });
    }
    if !volume_from.is_empty() {
        parts.push(Part {
            kind: "about",
            text: units::worded_volume(millilitres, measures, language),
            sources: volume_from,
        });
    }
    if !count_from.is_empty() {
        parts.push(Part {
            kind: "count",
            text: units::plain_number(count),
            sources: count_from,
        });
    }
    for entry in as_written {
        parts.push(Part {
            kind: "as_written",
            text: format!("{} {}", units::plain_number(entry.total), entry.word),
            sources: entry.sources,
        });
    }
    if !unstated_from.is_empty() {
        parts.push(Part {
            kind: "no_amount",
            text: units::no_amount_written(language).to_string(),
            sources: unstated_from,
        });
    }
    parts
}

/// One Unit Kamosu leaves alone, and everything written in it so far.
struct AsWritten {
    /// The word folded to what it is matched on: see [`unit_key`].
    folded: String,
    /// The first spelling seen, which is what the row says.
    word: String,
    total: f64,
    sources: Vec<String>,
}

/// **What makes two of a cook's own Units the same Unit.**
///
/// The fold every word in Kamosu gets — case away, accents kept, because `maïs`
/// is not `mais` — and then one plural marker off the end, so that `1 clove` in
/// one recipe and `4 cloves` in another make *5 cloves* rather than two amounts
/// side by side. That mattered more than it looks: on this list, more than one
/// amount **means** *Kamosu could not add this*, so leaving a singular beside
/// its own plural would have been the row admitting to a failure that had not
/// happened.
///
/// It is deliberately the smallest rule that does that job — never a stemmer.
/// A stemmer welds words a cook meant to keep apart and needs one per Language;
/// this takes a final `s`, or `x` for the French `choux`, or `es` where English
/// puts it after a sibilant (`pinch` · `pinches`), and nothing else. Two-letter
/// stems are left whole, so `os` stays `os`.
pub fn unit_key(word: &str) -> String {
    let folded = units::fold(word);
    let sibilant = |stem: &str| {
        stem.ends_with(['s', 'x', 'z']) || stem.ends_with("ch") || stem.ends_with("sh")
    };
    if let Some(stem) = folded.strip_suffix("es")
        && stem.chars().count() >= 2
        && sibilant(stem)
    {
        return stem.to_string();
    }
    if let Some(stem) = folded.strip_suffix(['s', 'x'])
        && stem.chars().count() >= 2
    {
        return stem.to_string();
    }
    folded
}

fn add_as_written(entries: &mut Vec<AsWritten>, word: &str, amount: f64, recipe: &str) {
    let folded = unit_key(word);
    match entries.iter_mut().find(|entry| entry.folded == folded) {
        Some(entry) => {
            entry.total += amount;
            remember(&mut entry.sources, recipe);
        }
        None => entries.push(AsWritten {
            folded,
            word: word.to_string(),
            total: amount,
            sources: vec![recipe.to_string()],
        }),
    }
}

/// Note a recipe as a source of one amount, once however many of its lines fed
/// it: a recipe wanting garlic twice is still one recipe wanting garlic.
fn remember(sources: &mut Vec<String>, recipe: &str) {
    if !sources.iter().any(|seen| seen == recipe) {
        sources.push(recipe.to_string());
    }
}

/// One chosen recipe, as `rows` adds it up: what `shopping_basis` answers for
/// it (#77), and how far its Yield scales it.
pub struct Chosen<'a> {
    pub branch_id: &'a str,
    /// The title it carries now, which is what a row that breaks open names.
    pub title: &'a str,
    /// The Yield being shopped for over the Yield written — 1 where the two
    /// cannot honestly be compared.
    pub scale: f64,
    /// `shopping_basis`'s `lines`.
    pub lines: &'a [Value],
}

/// **Every row of a Shopping List, worked out from the choosing** (ADR 0024).
///
/// Two kinds of row: a Food row that merges every mention of one Food, and a
/// verbatim line that merges with nothing. The Loose Items join them, and the
/// whole is sorted into one list.
///
/// **It takes nothing but what `shopping_basis` answers**, which is why it
/// lives here rather than beside the database: a phone with no network adds
/// the same facts up in `ui/src/lib/offline/shopping.ts` (#77), and
/// `shopping_parity` runs both over the same cases. If this changes, that test
/// fails until the phone's copy says the same thing.
pub fn rows(
    chosen: &[Chosen<'_>],
    loose: &[Value],
    measures: Measures,
    language: &str,
) -> Vec<Value> {
    // The Foods are held as a map beside the order they were first met, rather
    // than as a list searched from the top for every line: a list of a few
    // recipes is already several hundred Ingredient Lines, and the order still
    // has to be the order they arrived in so that two reads of one list agree.
    let mut foods: std::collections::HashMap<&str, (Vec<Contribution>, &Value)> =
        std::collections::HashMap::new();
    let mut food_order: Vec<&str> = Vec::new();
    let mut out: Vec<Value> = Vec::new();

    for entry in chosen {
        for line in entry.lines {
            let text = line["text"].as_str().unwrap_or_default();
            let food = &line["food"];
            // Which recipe the line is really from. A line that arrived by
            // unfolding a Component names the inner recipe — the dough, not
            // the pizza that composes it — so a row that breaks open says
            // which dish wants which, at whatever depth it came from (#86).
            let from = &line["from"];
            let recipe = from["title"].as_str().unwrap_or(entry.title);
            let branch_id = from["branch_id"].as_str().unwrap_or(entry.branch_id);
            let Some(food_id) = food["id"].as_str() else {
                out.push(json!({
                    "id": format!("{}:{}", entry.branch_id, line_key(&line["path"])),
                    "kind": "line",
                    "name": text,
                    "name_language": Value::Null,
                    "parts": Vec::<Value>::new(),
                    "lines": [{ "branch_id": branch_id, "recipe": recipe, "text": text }],
                    // Set only on a line standing for a Component Kamosu could
                    // not open, where it says which of the ways that happened.
                    "said": line["said"].clone(),
                }));
                continue;
            };
            let contribution = Contribution {
                recipe: recipe.to_string(),
                text: text.to_string(),
                branch_id: branch_id.to_string(),
                amount: food["amount"].as_f64().map(|amount| amount * entry.scale),
                unit: food["unit"].as_str().map(str::to_string),
                cup_weight_grams: food["cup_weight_grams"].as_f64(),
            };
            foods
                .entry(food_id)
                .or_insert_with(|| {
                    food_order.push(food_id);
                    (Vec::new(), food)
                })
                .0
                .push(contribution);
        }
    }

    for food_id in food_order {
        let (contributions, food) = foods.remove(food_id).expect("gathered above");
        out.push(json!({
            "id": food_id,
            "kind": "food",
            // A Food with no name at all should not exist — one is created
            // from whatever word a Reading found — but if ever one does, the
            // row falls back to the line that put it here rather than printing
            // an amount beside nothing. A blank row is the one thing a
            // shopping list cannot afford: it cannot be bought and it cannot
            // be asked about.
            "name": food["name"].as_str().unwrap_or_else(|| contributions
                .first()
                .map(|first| first.text.as_str())
                .unwrap_or_default()),
            // Which Language that name is in, so a name borrowed from another
            // Language can be marked as borrowed (CONTEXT.md, "Shopping Row").
            "name_language": food["name_language"],
            "parts": parts_json(&parts_for(&contributions, measures, language)),
            "lines": contributions
                .iter()
                .map(|contribution| json!({
                    "branch_id": contribution.branch_id,
                    "recipe": contribution.recipe,
                    "text": contribution.text,
                }))
                .collect::<Vec<_>>(),
            // A Food row is every mention of one Food and belongs to no single
            // line, so there is nothing here for a sentence to be about.
            "said": Value::Null,
        }));
    }
    out.extend(loose.iter().cloned());

    // One list, one order: a Loose Item and a line nobody read sort among the
    // Foods rather than into a block of their own, because a heading over the
    // rows that merged would teach that the others are somehow less true.
    out.sort_by_key(|row| sort_key(row["name"].as_str().unwrap_or_default()));
    out
}

/// **What names a row that merges with nothing**, from the `path` of line
/// indexes that reaches its line: `3` for the chosen recipe's fourth line,
/// `3.1` for the second line of the dough that line names (#86).
///
/// A plain index stopped being enough the moment a Component unfolded: the
/// pizza's first line and its dough's first line are two different things to
/// buy, and two rows sharing an id is a list that cannot be drawn. A recipe
/// composing nothing still gets exactly the id it got before, because a path
/// one deep joins to the index it holds.
fn line_key(path: &Value) -> String {
    path.as_array()
        .map(|steps| {
            steps
                .iter()
                .map(|step| step.as_i64().unwrap_or_default().to_string())
                .collect::<Vec<_>>()
                .join(".")
        })
        .unwrap_or_default()
}

/// The rows as the Catalogue declares them.
pub fn parts_json(parts: &[Part]) -> Vec<Value> {
    parts.iter().map(Part::to_json).collect()
}

/// How a row sorts against another: by what it says, with case folded away and
/// accents kept — the same fold every other word in Kamosu gets, so *échalote*
/// files under E rather than after Z, and files just after the plain *e* words
/// rather than exactly among them. That last part is a real if small cost, and
/// it is the price of the rule the rest of the file keeps: an accent is
/// meaning, never noise.
///
/// One list, one order. A Loose Item and an Ingredient Line nobody read sort
/// among the Foods rather than being swept into a block of their own — a list
/// that grouped them would be teaching that the ungrouped rows are somehow
/// more trustworthy, which is ADR 0015's trap wearing a heading.
pub fn sort_key(name: &str) -> String {
    units::fold(name)
}

/// **The list, as the text that leaves** (ADR 0024).
///
/// Kamosu decides what to buy and something else carries it round the shop —
/// Apple Notes, through a Shortcut, with the checkboxes this list refuses to
/// grow. So this is the whole feature's other end, and it is rendered from the
/// same answer the screen draws: the note and the screen cannot disagree about
/// a number, because there is only one number.
///
/// **Every line under the header is one thing to buy, written as a Markdown
/// checklist item** — the shape Aurélien chose on #74. The Shortcut appends it
/// to a note as Markdown, or strips the `- [ ] ` and hands each line to Notes'
/// *Append Checklist Item*; either way each line becomes a checkbox, so no
/// line may be anything but a thing to buy. That is why a row that could not
/// be added stays on one line, naming the dish behind each amount so *which
/// dish goes short* is still on the paper, and why a recipe that has gone
/// away is said in the header rather than on a line of its own: a thing that
/// quietly disappears from a shopping list is a thing that does not get
/// bought, and a line saying so must not be something to tick.
///
/// **The header line is a divider before it is a label.** The note accumulates,
/// and three trips appended with no divider are a wall — so the date leads and
/// the recipes it was built from follow.
pub fn as_text(list: &Value, today: &str, language: &str) -> String {
    let empty = Vec::new();
    let chosen = list["chosen"].as_array().unwrap_or(&empty);
    let title_of = |entry: &Value| entry["title"].as_str().unwrap_or_default().to_string();

    let (gone, here): (Vec<&Value>, Vec<&Value>) = chosen
        .iter()
        .partition(|entry| entry["gone"].as_bool().unwrap_or(false));

    let mut out = today.to_string();
    let names = here.iter().map(|entry| title_of(entry)).collect::<Vec<_>>();
    if !names.is_empty() {
        out.push_str(&format!(" · {}", names.join(", ")));
    }
    for entry in gone {
        out.push_str(&format!(
            " · {} {}",
            title_of(entry),
            gone_written(language)
        ));
    }

    for row in list["rows"].as_array().unwrap_or(&empty) {
        let name = row["name"].as_str().unwrap_or_default();
        let parts = row["parts"].as_array().unwrap_or(&empty);
        let said = |part: &Value| part["text"].as_str().unwrap_or_default().to_string();
        out.push_str(&format!("\n- [ ] {name}"));
        match parts.len() {
            // A Loose Item, and a line nobody ever read: what is written is
            // the whole of what is known, so it goes over whole and alone.
            0 => {}
            1 => out.push_str(&format!(" — {}", said(&parts[0]))),
            _ => {
                let amounts = parts
                    .iter()
                    .map(|part| {
                        let sources = part["sources"]
                            .as_array()
                            .unwrap_or(&empty)
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("{} {} {sources}", said(part), for_written(language))
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                out.push_str(&format!(" — {amounts}"));
            }
        }
    }
    out.push('\n');
    out
}

/// What a chosen recipe that can no longer be read says, once it is out of
/// Kamosu and has no screen to say it on.
fn gone_written(language: &str) -> &'static str {
    match language {
        "fr" => "ne peut plus être lu",
        "es" => "ya no se puede leer",
        _ => "can no longer be read",
    }
}

/// The word between an amount and the dish that wants it, on a row that could
/// not be added.
fn for_written(language: &str) -> &'static str {
    match language {
        "fr" => "pour",
        "es" => "para",
        _ => "for",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(recipe: &str, amount: Option<f64>, unit: Option<&str>) -> Contribution {
        Contribution {
            recipe: recipe.to_string(),
            text: format!("{} {}", amount.unwrap_or_default(), unit.unwrap_or("")),
            branch_id: "b_1".to_string(),
            amount,
            unit: unit.map(str::to_string),
            cup_weight_grams: None,
        }
    }

    #[test]
    fn amounts_that_convert_add_into_one_part_that_says_about() {
        let parts = parts_for(
            &[
                line("Coq au Vin", Some(150.0), Some("g")),
                line("Loaf Cake", Some(1.0), Some("kg")),
            ],
            Measures::Metric,
            "en",
        );
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].kind, "about");
        assert_eq!(parts[0].text, "about 1.15 kg");
        assert_eq!(parts[0].sources, vec!["Coq au Vin", "Loaf Cake"]);
    }

    #[test]
    fn amounts_that_do_not_convert_ride_side_by_side() {
        // The real corpus case: 2 tbsp of minced garlic in one recipe, 4 cloves
        // of it in another. Two true amounts beat one wrong one (ADR 0024).
        let parts = parts_for(
            &[
                line("Korean Fried Chicken", Some(2.0), Some("tbsp")),
                line("Coq au Vin", Some(4.0), Some("cloves")),
            ],
            Measures::Metric,
            "en",
        );
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].text, "about 30 ml");
        assert_eq!(parts[0].sources, vec!["Korean Fried Chicken"]);
        assert_eq!(parts[1].kind, "as_written");
        assert_eq!(parts[1].text, "4 cloves");
        assert_eq!(parts[1].sources, vec!["Coq au Vin"]);
    }

    #[test]
    fn a_line_with_no_amount_rides_as_some_rather_than_being_dropped() {
        let parts = parts_for(
            &[
                line("Dan Dan Noodles", Some(2.0), None),
                line("Korean Fried Chicken", None, None),
            ],
            Measures::Metric,
            "en",
        );
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].kind, "count");
        assert_eq!(parts[0].text, "2");
        assert_eq!(parts[1].kind, "no_amount");
        assert_eq!(parts[1].text, "some");
        assert_eq!(parts[1].sources, vec!["Korean Fried Chicken"]);
    }

    #[test]
    fn a_row_made_only_of_unstated_amounts_still_says_something() {
        let parts = parts_for(
            &[line("Coq au Vin", None, None), line("Orzo", None, None)],
            Measures::Metric,
            "en",
        );
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].kind, "no_amount");
        assert_eq!(parts[0].sources, vec!["Coq au Vin", "Orzo"]);
    }

    #[test]
    fn a_cup_weight_carries_a_customary_volume_into_grams() {
        let mut honey = line("Korean Fried Chicken", Some(0.25), Some("cup"));
        honey.cup_weight_grams = Some(340.0);
        let parts = parts_for(&[honey], Measures::Metric, "en");
        assert_eq!(parts[0].text, "about 85 g");
    }

    #[test]
    fn without_a_cup_weight_a_customary_volume_stays_a_volume() {
        let parts = parts_for(
            &[line("Korean Fried Chicken", Some(1.0), Some("cup"))],
            Measures::Metric,
            "en",
        );
        assert_eq!(parts[0].text, "about 240 ml");
    }

    #[test]
    fn one_recipe_wanting_a_food_twice_is_named_once() {
        let parts = parts_for(
            &[
                line("Dan Dan Noodles", Some(1.0), Some("tsp")),
                line("Dan Dan Noodles", Some(1.0), Some("tsp")),
            ],
            Measures::Metric,
            "en",
        );
        assert_eq!(parts[0].sources, vec!["Dan Dan Noodles"]);
        assert_eq!(parts[0].text, "about 10 ml");
    }

    #[test]
    fn one_unknown_unit_written_two_ways_adds_and_two_words_do_not() {
        let parts = parts_for(
            &[
                line("A", Some(2.0), Some("Cloves")),
                line("B", Some(1.0), Some("cloves")),
                line("C", Some(3.0), Some("poignées")),
            ],
            Measures::Metric,
            "en",
        );
        assert_eq!(parts.len(), 2);
        // The first spelling seen is the one the row says.
        assert_eq!(parts[0].text, "3 Cloves");
        assert_eq!(parts[0].sources, vec!["A", "B"]);
        assert_eq!(parts[1].text, "3 poignées");
    }

    #[test]
    fn one_unknown_unit_and_its_own_plural_are_one_unit() {
        // The row that made this necessary: `1 clove of garlic` in one recipe
        // and `4 cloves` in another. Two Parts here would have been the row
        // saying *Kamosu could not add this*, which would have been a lie.
        let parts = parts_for(
            &[
                line("Coq au Vin", Some(1.0), Some("clove")),
                line("Korean Fried Chicken", Some(4.0), Some("cloves")),
            ],
            Measures::Metric,
            "en",
        );
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].text, "5 clove");
        assert_eq!(parts[0].sources, vec!["Coq au Vin", "Korean Fried Chicken"]);
    }

    #[test]
    fn a_plural_after_a_sibilant_folds_too_and_a_short_word_is_left_whole() {
        let pinches = parts_for(
            &[
                line("A", Some(1.0), Some("pinch")),
                line("B", Some(2.0), Some("pinches")),
            ],
            Measures::Metric,
            "en",
        );
        assert_eq!(pinches.len(), 1);
        assert_eq!(pinches[0].text, "3 pinch");

        // `os` is a word, not a plural: two letters are left alone.
        assert_eq!(unit_key("os"), "os");
        assert_eq!(unit_key("choux"), "chou");
    }

    #[test]
    fn a_plural_fold_still_keeps_two_different_words_apart() {
        // The rule takes one ending off, and is not a stemmer: *pincée* and
        // *poignée* remain two things a cook meant differently.
        let parts = parts_for(
            &[
                line("A", Some(1.0), Some("pincées")),
                line("B", Some(1.0), Some("poignées")),
            ],
            Measures::Metric,
            "en",
        );
        assert_eq!(parts.len(), 2);
    }

    #[test]
    fn an_accent_is_meaning_here_as_everywhere_else() {
        // ADR 0022's rule about Foods holds for a cook's own Unit too: `maïs`
        // is not `mais`, so two words that differ by an accent stay two rows'
        // worth of amount rather than being welded together.
        let parts = parts_for(
            &[
                line("A", Some(1.0), Some("poignée")),
                line("B", Some(1.0), Some("poignee")),
            ],
            Measures::Metric,
            "en",
        );
        assert_eq!(parts.len(), 2);
    }

    #[test]
    fn a_reader_in_american_measures_is_answered_in_her_own_drawer() {
        let parts = parts_for(
            &[line("Coq au Vin", Some(500.0), Some("ml"))],
            Measures::Us,
            "en",
        );
        assert_eq!(parts[0].text, "about 2⅛ cups");
    }

    #[test]
    fn as_written_adds_within_a_unit_and_converts_nothing() {
        let parts = parts_for(
            &[
                line("A", Some(150.0), Some("g")),
                line("B", Some(100.0), Some("g")),
                line("C", Some(2.0), Some("cups")),
            ],
            Measures::AsWritten,
            "en",
        );
        assert_eq!(parts.len(), 2);
        // No conversion happened, so nothing says *about*.
        assert_eq!(parts[0].kind, "as_written");
        assert_eq!(parts[0].text, "250 g");
        assert_eq!(parts[1].text, "2 cups");
    }

    #[test]
    fn an_american_keeps_a_weight_and_a_volume_apart() {
        // ADR 0016: nothing crosses into her cups, because she is not short of
        // a cup. So her row carries two true amounts where a metric reader's
        // carries one — the same list, read by two kitchens.
        let mut flour = line("A", Some(2.0), Some("cups"));
        flour.cup_weight_grams = Some(125.0);
        let mut weighed = line("B", Some(500.0), Some("g"));
        weighed.cup_weight_grams = Some(125.0);

        let hers = parts_for(&[flour.clone(), weighed.clone()], Measures::Us, "en");
        assert_eq!(hers.len(), 2);

        let his = parts_for(&[flour, weighed], Measures::Metric, "en");
        assert_eq!(his.len(), 1);
        assert_eq!(his[0].text, "about 750 g");
    }

    #[test]
    fn accents_do_not_send_a_row_to_the_end_of_the_list() {
        let mut names = ["zeste", "échalote", "ail"];
        names.sort_by_key(|name| sort_key(name));
        assert_eq!(names, ["ail", "échalote", "zeste"]);
    }

    fn sample_list() -> Value {
        json!({
            "chosen": [
                { "title": "Korean Fried Chicken", "gone": false },
                { "title": "Coq au Vin", "gone": false },
                { "title": "Ratatouille", "gone": true },
            ],
            "rows": [
                { "name": "bin bags", "parts": [] },
                { "name": "minced garlic", "parts": [
                    { "text": "about 30 ml", "sources": ["Korean Fried Chicken"] },
                    { "text": "4 cloves", "sources": ["Coq au Vin", "Orzo"] },
                ] },
                { "name": "soy sauce", "parts": [
                    { "text": "about 45 ml", "sources": ["Korean Fried Chicken", "Coq au Vin"] },
                ] },
            ],
        })
    }

    #[test]
    fn the_text_is_a_header_then_one_markdown_checklist_line_per_thing_to_buy() {
        // The shape chosen on #74: a Shortcut hands this to Notes, and every
        // line under the header has to be exactly one thing to buy, because
        // every one becomes a checkbox.
        let text = as_text(&sample_list(), "2026-09-19", "en");
        assert_eq!(
            text,
            "2026-09-19 · Korean Fried Chicken, Coq au Vin · Ratatouille can no longer be read\n\
             - [ ] bin bags\n\
             - [ ] minced garlic — about 30 ml for Korean Fried Chicken; 4 cloves for Coq au Vin, Orzo\n\
             - [ ] soy sauce — about 45 ml\n"
        );
    }

    #[test]
    fn a_list_with_nothing_on_it_is_the_header_alone() {
        let text = as_text(&json!({ "chosen": [], "rows": [] }), "2026-09-19", "en");
        assert_eq!(text, "2026-09-19\n");
    }

    #[test]
    fn the_words_the_text_adds_follow_the_reader() {
        let text = as_text(&sample_list(), "2026-09-19", "fr");
        assert!(
            text.contains("· Ratatouille ne peut plus être lu\n"),
            "{text}"
        );
        assert!(
            text.contains("about 30 ml pour Korean Fried Chicken; 4 cloves pour"),
            "{text}"
        );
    }

    // ── The phone's copy (#77) ────────────────────────────────────────────

    /// A Food as one test line gives it: its id, its name, the amount, the
    /// Unit as written, and a Cup Weight.
    type TestFood<'a> = (
        &'a str,
        Option<&'a str>,
        Option<f64>,
        Option<&'a str>,
        Option<f64>,
    );

    /// A line of `shopping_basis`, as a phone receives it: one of the chosen
    /// recipe's own.
    fn basis_line(index: usize, text: &str, food: Option<TestFood<'_>>) -> Value {
        inner_line(&[index], None, text, food)
    }

    /// A Component's own written line, kept on the list because Kamosu could
    /// not open the recipe it names, carrying the sentence that says why (#86).
    fn unopened_line(index: usize, text: &str, said: &str) -> Value {
        let mut line = basis_line(index, text, None);
        line["said"] = json!(said);
        line
    }

    /// A line that arrived by unfolding a Component, at the `path` of line
    /// indexes that reaches it and naming the recipe it is really from (#86).
    fn inner_line(
        path: &[usize],
        from: Option<&str>,
        text: &str,
        food: Option<TestFood<'_>>,
    ) -> Value {
        let path = path.iter().map(|step| *step as i64).collect::<Vec<_>>();
        let from = from
            .map(|title| json!({ "branch_id": format!("b_{title}"), "title": title }))
            .unwrap_or(Value::Null);
        match food {
            None => json!({
                "path": path,
                "from": from,
                "said": Value::Null,
                "text": text,
                "food": Value::Null,
            }),
            Some((id, name, amount, unit, cup_weight)) => json!({
                "path": path,
                "from": from,
                "said": Value::Null,
                "text": text,
                "food": {
                    "id": id,
                    "name": name,
                    "name_language": name.map(|_| "en"),
                    "amount": amount,
                    "unit": unit,
                    "unit_id": unit.and_then(units::recognise).map(|known| known.id),
                    "unit_key": unit.map(unit_key),
                    "cup_weight_grams": cup_weight,
                },
            }),
        }
    }

    /// Every case the phone's copy is held to. Built rather than listed, so
    /// that every Unit Kamosu knows, every kitchen and every Language is in it.
    fn parity_cases() -> Vec<Value> {
        let measures = [
            ("us", Measures::Us),
            ("metric", Measures::Metric),
            ("as_written", Measures::AsWritten),
        ];
        let languages = ["en", "fr", "es"];
        let amounts = [0.3, 1.0, 7.0, 99.0, 1234.5];

        // One recipe per Unit, at every amount, with and without a Cup Weight
        // — each on its own Food, so each is its own row.
        let mut singles = Vec::new();
        for (at, unit) in UNITS_FOR_PARITY.iter().enumerate() {
            let mut lines = Vec::new();
            for (n, amount) in amounts.iter().enumerate() {
                let food = format!("f_{at}_{n}");
                let cup = if n % 2 == 0 { Some(125.0) } else { None };
                lines.push(basis_line(
                    lines.len(),
                    &format!("{amount} {unit} of thing {at} {n}"),
                    Some((food.as_str(), Some("thing"), Some(*amount), Some(unit), cup)),
                ));
            }
            singles.push(lines);
        }

        // The rows that merge, break open, or stand alone.
        let flour_a = vec![
            basis_line(0, "Pastry", None),
            basis_line(
                1,
                "500 g flour",
                Some((
                    "f_flour",
                    Some("flour"),
                    Some(500.0),
                    Some("g"),
                    Some(125.0),
                )),
            ),
            basis_line(
                2,
                "2 cloves garlic",
                Some(("f_garlic", Some("Garlic"), Some(2.0), Some("cloves"), None)),
            ),
            basis_line(3, "salt", Some(("f_salt", Some("sel"), None, None, None))),
            basis_line(
                4,
                "3 large eggs",
                Some(("f_egg", Some("Œufs"), Some(3.0), None, None)),
            ),
            basis_line(
                5,
                "une poignée de persil",
                Some((
                    "f_parsley",
                    Some("persil"),
                    Some(1.0),
                    Some("poignée"),
                    None,
                )),
            ),
            basis_line(6, "a pinch of cumin", None),
            basis_line(
                7,
                "1 unnamed thing",
                Some(("f_unnamed", None, Some(1.0), None, None)),
            ),
        ];
        let flour_b = vec![
            basis_line(
                0,
                "2 cups flour",
                Some((
                    "f_flour",
                    Some("flour"),
                    Some(2.0),
                    Some("cups"),
                    Some(125.0),
                )),
            ),
            basis_line(
                1,
                "1 clove garlic",
                Some(("f_garlic", Some("Garlic"), Some(1.0), Some("clove"), None)),
            ),
            basis_line(
                2,
                "2 tbsp minced garlic",
                Some(("f_garlic", Some("Garlic"), Some(2.0), Some("tbsp"), None)),
            ),
            basis_line(
                3,
                "2 poignées de persil",
                Some((
                    "f_parsley",
                    Some("persil"),
                    Some(2.0),
                    Some("poignées"),
                    None,
                )),
            ),
            basis_line(
                4,
                "3 pinches salt",
                Some(("f_salt", Some("sel"), Some(3.0), Some("pinches"), None)),
            ),
            basis_line(
                5,
                "½ tasse de lait",
                Some(("f_milk", Some("lait"), Some(0.5), Some("tasse"), None)),
            ),
            basis_line(
                6,
                "250 ml lait",
                Some(("f_milk", Some("lait"), Some(250.0), Some("ml"), None)),
            ),
            basis_line(
                7,
                "2 choux",
                Some(("f_cabbage", Some("chou"), Some(2.0), Some("choux"), None)),
            ),
            basis_line(
                8,
                "1 chou",
                Some(("f_cabbage", Some("chou"), Some(1.0), Some("chou"), None)),
            ),
            basis_line(
                9,
                "Échalote",
                Some(("f_shallot", Some("échalote"), Some(2.0), None, None)),
            ),
            basis_line(
                10,
                "1 lb butter",
                Some(("f_butter", Some("butter"), Some(1.0), Some("lb"), None)),
            ),
            basis_line(
                11,
                "8 oz butter",
                Some(("f_butter", Some("butter"), Some(8.0), Some("oz"), None)),
            ),
            basis_line(
                12,
                "Straße salt",
                Some(("f_strasse", Some("Straße"), Some(1.0), Some("kg"), None)),
            ),
            basis_line(13, "zucchini", None),
        ];
        // **A recipe that composes others** (#86), as `shopping_basis` hands
        // it over once the unfolding is done: the pizza's own flour, the
        // dough's flour and water at the dough's share, a starter inside the
        // dough at the compounded share, a sauce Kamosu could work out no
        // factor for whose tomatoes ride unmeasured, and a mozzarella nobody
        // in view holds whose written line simply stays.
        let composed = vec![
            basis_line(
                0,
                "200 g flour",
                Some((
                    "f_flour",
                    Some("flour"),
                    Some(200.0),
                    Some("g"),
                    Some(125.0),
                )),
            ),
            inner_line(
                &[1, 0],
                Some("Pizza Dough"),
                "500 g de farine T55",
                Some((
                    "f_flour",
                    Some("flour"),
                    Some(250.0),
                    Some("g"),
                    Some(125.0),
                )),
            ),
            inner_line(
                &[1, 1],
                Some("Pizza Dough"),
                "300 ml d'eau",
                Some(("f_water", Some("water"), Some(150.0), Some("ml"), None)),
            ),
            inner_line(
                &[1, 2, 0],
                Some("Levain"),
                "100 g de farine",
                Some(("f_flour", Some("flour"), Some(25.0), Some("g"), Some(125.0))),
            ),
            inner_line(
                &[2, 0],
                Some("Sauce tomate"),
                "400 g de tomates pelées",
                Some(("f_tomato", Some("tomatoes"), None, Some("g"), None)),
            ),
            inner_line(&[2, 1], Some("Sauce tomate"), "une pincée de sucre", None),
            unopened_line(
                3,
                "250 g de mozzarella di bufala",
                "Kamosu does not have this recipe.",
            ),
            basis_line(4, "2 pincées d'origan", None),
        ];
        let loose = vec![
            json!({ "id": "i_0000000000000001", "kind": "loose", "name": "bin bags", "name_language": Value::Null, "parts": [], "lines": [] }),
            json!({ "id": "i_0000000000000002", "kind": "loose", "name": "Éponges", "name_language": Value::Null, "parts": [], "lines": [] }),
            json!({ "id": "i_0000000000000003", "kind": "loose", "name": "  coffee.  beans ", "name_language": Value::Null, "parts": [], "lines": [] }),
        ];

        let mut cases = Vec::new();
        for (measures_name, measures) in measures {
            for language in languages {
                for (at, lines) in singles.iter().enumerate() {
                    // Scaling is the same arithmetic in every Language, so
                    // it is held to once rather than three times.
                    let scales: &[f64] = if language == "en" {
                        &[1.0, 2.0]
                    } else {
                        &[1.0]
                    };
                    for &scale in scales {
                        let chosen = [Chosen {
                            branch_id: "b_one",
                            title: "One",
                            scale,
                            lines,
                        }];
                        cases.push(json!({
                            "name": format!("unit {} ×{scale} {measures_name} {language}", UNITS_FOR_PARITY[at]),
                            "measures": measures_name,
                            "language": language,
                            "chosen": [{ "branch_id": "b_one", "title": "One", "scale": scale, "lines": lines }],
                            "loose": [],
                            "rows": rows(&chosen, &[], measures, language),
                        }));
                    }
                }
                for (scale_a, scale_b) in [(1.0, 1.0), (1.5, 0.5), (3.0, 2.0 / 3.0)] {
                    let chosen = [
                        Chosen {
                            branch_id: "b_a",
                            title: "Tarte",
                            scale: scale_a,
                            lines: &flour_a,
                        },
                        Chosen {
                            branch_id: "b_b",
                            title: "Gratin",
                            scale: scale_b,
                            lines: &flour_b,
                        },
                    ];
                    cases.push(json!({
                        "name": format!("merging ×{scale_a}/×{scale_b} {measures_name} {language}"),
                        "measures": measures_name,
                        "language": language,
                        "chosen": [
                            { "branch_id": "b_a", "title": "Tarte", "scale": scale_a, "lines": flour_a },
                            { "branch_id": "b_b", "title": "Gratin", "scale": scale_b, "lines": flour_b },
                        ],
                        "loose": loose,
                        "rows": rows(&chosen, &loose, measures, language),
                    }));
                }
                // The Shopping Yield scaling a whole chain of Components, on
                // top of the share each already carries (#86).
                for scale in [1.0, 2.0] {
                    let chosen = [Chosen {
                        branch_id: "b_pizza",
                        title: "Pizza",
                        scale,
                        lines: &composed,
                    }];
                    cases.push(json!({
                        "name": format!("composed ×{scale} {measures_name} {language}"),
                        "measures": measures_name,
                        "language": language,
                        "chosen": [
                            { "branch_id": "b_pizza", "title": "Pizza", "scale": scale, "lines": composed },
                        ],
                        "loose": [],
                        "rows": rows(&chosen, &[], measures, language),
                    }));
                }
            }
        }
        cases
    }

    /// One written spelling of every Unit, and a few Kamosu does not know.
    const UNITS_FOR_PARITY: &[&str] = &[
        "g",
        "kg",
        "oz",
        "lb",
        "ml",
        "cl",
        "dl",
        "l",
        "tsp",
        "tbsp",
        "cup",
        "cups",
        "fl oz",
        "imperial tbsp",
        "aus tbsp",
        "c. à s.",
        "cuillère à café",
        "cucharada",
        "tasse",
        "sachet",
        "pincée",
        "",
    ];

    fn parity_file() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/shopping-parity.json")
    }

    /// **The phone adds a Shopping List up exactly as the server does** (#77).
    ///
    /// Aurélien chose, on #77, to have a phone with no network work the rows
    /// out itself rather than wait for the server, which means the sum exists
    /// twice: here, and in `ui/src/lib/offline/shopping.ts`. This is what keeps
    /// the two one sum. It writes every case above, with what this file
    /// answers for it, into `tests/fixtures/shopping-parity.json`, and fails if what is
    /// committed there is not what this code says today. The screen tests read
    /// the same file and fail if the phone says anything else.
    ///
    /// **If this fails, the server's sum changed.** Rewrite the file with
    /// `KAMOSU_WRITE_PARITY=1 cargo test shopping_parity`, then make the
    /// phone's copy agree until `just test` passes — both halves, in the same
    /// commit.
    #[test]
    fn shopping_parity() {
        let folds = [
            "Flour",
            "  Crème   fraîche. ",
            "ÉCHALOTE",
            "Œufs",
            "Straße",
            "c. à c.",
            "ΣΊΣΥΦΟΣ",
            "maïs",
            "mais",
            "e\u{301}chalote",
            "ﬁne sugar",
        ]
        .iter()
        .map(|text| json!({ "text": text, "folded": units::fold(text) }))
        .collect::<Vec<_>>();
        let amounts = [
            "2", "2.5", "2,5", "1/2", "1 1/2", "½", "4½", "1 ½", " 3 ", "a dozen", "2-3", "", "0",
            "1/0",
        ]
        .iter()
        .map(|text| json!({ "text": text, "value": units::parse_amount(text) }))
        .collect::<Vec<_>>();
        let yields = [
            (
                json!({"amount": "8", "noun": "servings"}),
                json!({"amount": "4", "noun": "servings"}),
            ),
            (
                json!({"amount": "2", "noun": "loaves"}),
                json!({"amount": "4", "noun": "servings"}),
            ),
            (
                json!({"amount": "1½", "noun": "tarts"}),
                json!({"amount": "1", "noun": "tarts"}),
            ),
            (
                json!({"amount": "a dozen", "noun": "cookies"}),
                json!({"amount": "24", "noun": "cookies"}),
            ),
            (Value::Null, json!({"amount": "4", "noun": "servings"})),
            (json!({"amount": "6", "noun": "servings"}), Value::Null),
            (
                json!({"amount": "0", "noun": "servings"}),
                json!({"amount": "4", "noun": "servings"}),
            ),
            // A multiplier (#109): an empty noun, with or without a written Yield.
            (json!({"amount": "2", "noun": ""}), Value::Null),
            (
                json!({"amount": "½", "noun": ""}),
                json!({"amount": "4", "noun": "servings"}),
            ),
            (json!({"amount": "lots", "noun": ""}), Value::Null),
        ]
        .into_iter()
        .map(|(wanted, written)| {
            let scale = crate::core::yield_scale(&wanted, &written);
            json!({ "wanted": wanted, "written": written, "scale": scale })
        })
        .collect::<Vec<_>>();
        let cup_plurals = units::CUP_PLURALS
            .iter()
            .map(|&(cups, language, _)| {
                let millilitres = units::cups_in_millilitres(cups);
                json!({
                    "millilitres": millilitres,
                    "language": language,
                    "worded": units::worded_volume(millilitres, Measures::Us, language),
                })
            })
            .collect::<Vec<_>>();
        let cases = parity_cases();
        let texts = cases
            .iter()
            .filter(|case| case["name"].as_str().is_some_and(|name| name.starts_with("merging")))
            .map(|case| {
                let list = json!({
                    "chosen": [
                        { "branch_id": "b_a", "title": "Tarte", "gone": false, "shopping_yield": Value::Null, "written_yield": Value::Null },
                        { "branch_id": "b_b", "title": "Gratin", "gone": false, "shopping_yield": Value::Null, "written_yield": Value::Null },
                        { "branch_id": "b_c", "title": "Old soup", "gone": true, "shopping_yield": Value::Null, "written_yield": Value::Null },
                    ],
                    "rows": case["rows"],
                });
                let language = case["language"].as_str().unwrap_or("en");
                json!({
                    "list": list,
                    "today": "2026-09-19",
                    "language": language,
                    "text": as_text(&list, "2026-09-19", language),
                })
            })
            .collect::<Vec<_>>();

        // One case to a line, so a change to the sum reads as a diff of the
        // cases it moved rather than of one enormous line.
        let lines = |items: &[Value]| {
            items
                .iter()
                .map(|item| serde_json::to_string(item).expect("serialisable"))
                .collect::<Vec<_>>()
                .join(",\n")
        };
        let said = format!(
            "{{\"about\": {},\n\"folds\": [\n{}\n],\n\"amounts\": [\n{}\n],\n\"yield_scales\": [\n{}\n],\n\"cup_plurals\": [\n{}\n],\n\"cases\": [\n{}\n],\n\"texts\": [\n{}\n]}}\n",
            json!(
                "Written by shopping_parity in src/shopping.rs (#77). Do not edit: rewrite it with KAMOSU_WRITE_PARITY=1 cargo test shopping_parity."
            ),
            lines(&folds),
            lines(&amounts),
            lines(&yields),
            lines(&cup_plurals),
            lines(&cases),
            lines(&texts),
        );

        let path = parity_file();
        if std::env::var_os("KAMOSU_WRITE_PARITY").is_some() {
            std::fs::write(&path, &said).expect("write the parity cases");
            return;
        }
        let committed = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            committed == said,
            "{} is not what the server's Shopping List sum says today. Rewrite it with \
             KAMOSU_WRITE_PARITY=1 cargo test shopping_parity, then make \
             ui/src/lib/offline/shopping.ts agree.",
            path.display()
        );
    }
}
