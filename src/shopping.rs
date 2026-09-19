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
fn unit_key(word: &str) -> String {
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
}
