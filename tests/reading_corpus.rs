//! Reading Ingredient Lines, put through Aurélien's real 86-recipe Crouton
//! export (#71, ADR 0036).
//!
//! **This corpus is the only honest scoreboard this feature has.** Ten
//! flattering lines prove nothing about a reader that has to survive a real
//! library — and this library is unusually good evidence, because Crouton
//! stored each line's amount, unit and name *separately*. Reassembling them
//! into the one written line a cook would type gives 863 real lines whose
//! correct answer is already known, which is what lets the floors below be
//! measured rather than asserted.
//!
//! Every recipe goes in through the real `create_recipe` Operation and every
//! Reading comes back through the real `get_recipe`, so what is scored is what
//! both Doors serve and not a function called directly.
//!
//! **The floors are floors, not targets.** They are set just under what was
//! measured on the day (ADR 0036: every amount, and every Unit the export
//! itself names), so this fails on a regression and never on an improvement.
//!
//! The export lives at `samples/crouton/` and is gitignored — personal,
//! 110 MB, and never committed (see `docs/research/crouton-real-export.md`).
//! This test is `#[ignore]`d so `just test` and CI never need it: run it
//! deliberately with `cargo test --test reading_corpus -- --ignored` on a
//! machine that has the export.

mod support;

use serde_json::{Value, json};
use std::io::Read;

fn corpus_path() -> Option<std::path::PathBuf> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/crouton");
    std::fs::read_dir(&dir).ok()?.find_map(|entry| {
        let path = entry.ok()?.path();
        (path.extension().and_then(|e| e.to_str()) == Some("zip")).then_some(path)
    })
}

/// How Crouton's `quantityType` is ordinarily written on a line. These are the
/// fifteen the real export actually contains, and no others.
fn unit_word(quantity_type: &str) -> Option<&'static str> {
    Some(match quantity_type {
        "ITEM" => "",
        "CUP" => "cup",
        "TABLESPOON" => "tbsp",
        "GRAMS" => "g",
        "TEASPOON" => "tsp",
        "OUNCE" => "oz",
        "POUND" => "lb",
        "MILLS" => "ml",
        "KGS" => "kg",
        "CAN" => "can",
        "LITRES" => "l",
        "BOTTLE" => "bottle",
        "PACKET" => "packet",
        "PINCH" => "pinch",
        _ => return None,
    })
}

/// An amount as a cook writes it: whole where it is whole, a vulgar fraction
/// where the export holds one of the halves, thirds, quarters and eighths a
/// recipe actually contains.
fn written_amount(amount: f64) -> String {
    if (amount - amount.round()).abs() < 1e-9 {
        return format!("{}", amount.round() as i64);
    }
    for denominator in [2, 3, 4, 8] {
        let scaled = amount * f64::from(denominator);
        if (scaled - scaled.round()).abs() < 1e-9 {
            let numerator = scaled.round() as i64;
            let (whole, part) = (
                numerator / i64::from(denominator),
                numerator % i64::from(denominator),
            );
            return if whole > 0 {
                format!("{whole} {part}/{denominator}")
            } else {
                format!("{part}/{denominator}")
            };
        }
    }
    format!("{amount}")
}

/// One line of the export: the written line a cook would have typed, beside
/// the three parts Crouton itself kept — the answer this test scores against.
#[derive(Clone)]
struct Line {
    written: String,
    amount: Option<f64>,
    unit: Option<String>,
    name: String,
}

fn lines_of(crumb: &Value) -> Option<(String, Vec<Line>)> {
    let title = crumb["name"].as_str()?.trim().to_string();
    if title.is_empty() {
        return None;
    }
    let mut entries: Vec<&Value> = crumb["ingredients"].as_array()?.iter().collect();
    entries.sort_by_key(|line| line["order"].as_i64().unwrap_or(0));

    let lines = entries
        .iter()
        .filter_map(|entry| {
            let name = entry["ingredient"]["name"].as_str()?.trim().to_string();
            if name.is_empty() {
                return None;
            }
            let quantity = &entry["quantity"];
            let amount = quantity["amount"].as_f64().filter(|a| *a != 0.0);
            let Some(amount) = amount else {
                // A line carrying no quantity at all — 28% of this corpus.
                return Some(Line {
                    written: name.clone(),
                    amount: None,
                    unit: None,
                    name,
                });
            };
            let unit = unit_word(quantity["quantityType"].as_str()?)?;
            let written = [written_amount(amount), unit.to_string(), name.clone()]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            Some(Line {
                written,
                amount: Some(amount),
                unit: (!unit.is_empty()).then(|| unit.to_string()),
                name,
            })
        })
        .collect();
    Some((title, lines))
}

fn corpus() -> Option<Vec<(String, Vec<Line>)>> {
    let file = std::fs::File::open(corpus_path()?).expect("open the corpus zip");
    let mut archive = zip::ZipArchive::new(file).expect("read the corpus zip");
    let mut recipes = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).expect("read entry");
        if !entry.name().ends_with(".crumb") {
            continue;
        }
        let mut text = String::new();
        entry.read_to_string(&mut text).expect("crumb is text");
        let crumb: Value = serde_json::from_str(&text).expect("crumb is JSON");
        if let Some(recipe) = lines_of(&crumb) {
            recipes.push(recipe);
        }
    }
    Some(recipes)
}

/// Crouton's own name field is not always clean: a handful of entries have the
/// unit word or a stray comma inside the *name*, so reassembling the line
/// writes the unit twice. Those lines are excluded from the name and unit
/// scores rather than counted as failures of Kamosu's reading, and there are
/// eighteen of them in 863.
fn ground_truth_is_clean(line: &Line) -> bool {
    const UNIT_WORDS: &[&str] = &[
        "cup",
        "cups",
        "tablespoon",
        "tablespoons",
        "teaspoon",
        "teaspoons",
        "gram",
        "grams",
        "ounce",
        "ounces",
        "pound",
        "pounds",
        "ml",
        "g",
        "kg",
        "l",
        "can",
        "bottle",
        "packet",
        "pinch",
    ];
    let first = line.name.split_whitespace().next().unwrap_or("");
    !line.name.trim_start().starts_with(',')
        && !UNIT_WORDS.contains(&first.trim_matches(',').to_lowercase().as_str())
}

/// Whether a written line offers the cook a choice of two things (#148). It
/// does when `or`, `ou` or `o` stands as a word of its own before the first
/// comma and outside any brackets, where the cook's aside begins. Written here from the line
/// itself rather than borrowed from the reader, so the two can disagree.
fn written_offers_a_choice(written: &str) -> bool {
    let mut depth = 0i32;
    let outside: String = written
        .chars()
        .filter(|c| match c {
            '(' | '[' | '{' => {
                depth += 1;
                false
            }
            ')' | ']' | '}' => {
                depth = (depth - 1).max(0);
                false
            }
            _ => depth == 0,
        })
        .collect();
    outside
        .split(',')
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .any(|word| {
            matches!(
                word.to_lowercase().as_str(),
                "or" | "ou" | "o" | "and/or" | "et/ou" | "y/o"
            )
        })
}

/// Two Unit spellings that mean one Unit. Kamosu keeps whatever the cook wrote
/// (ADR 0016), so `tbsp` read off a line written `tbsp` is exactly right and
/// the comparison folds spelling rather than demanding one.
fn same_unit(expected: Option<&str>, read: Option<&str>) -> bool {
    match (expected, read) {
        (None, read) => read.is_none(),
        (Some(expected), Some(read)) => {
            let fold = |word: &str| word.to_lowercase().trim_end_matches('s').to_string();
            fold(expected) == fold(read)
        }
        (Some(_), None) => false,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs the personal, gitignored samples/crouton export on disk"]
async fn the_real_library_is_read_as_well_as_it_was_measured_and_an_unread_line_still_works() {
    let Some(recipes) = corpus() else {
        eprintln!(
            "skipping: no samples/crouton/*.zip on this machine (personal, gitignored export)"
        );
        return;
    };
    assert_eq!(
        recipes.len(),
        86,
        "the corpus is the 86-recipe export the map is measured against"
    );

    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "corpus reading", false)
        .unwrap()
        .secret;

    let mut total = 0usize;
    let mut without_quantity = 0usize;
    let mut amount_right = 0usize;
    let mut unit_right = 0usize;
    let mut scored = 0usize;
    let mut missed_units = 0usize;
    let mut named = 0usize;
    let mut choices = 0usize;
    let mut choices_named_as_foods = Vec::new();
    let mut measures_named_as_foods = Vec::new();
    let mut descriptions_named_as_foods = Vec::new();

    for (title, lines) in &recipes {
        let ingredients: Vec<Value> = lines
            .iter()
            .map(|line| json!({ "kind": "ingredient", "text": line.written }))
            .collect();
        let (status, created) = app.post_op(
            "create_recipe",
            Some(&key),
            &json!({ "title": title, "ingredients": ingredients }).to_string(),
        );
        assert_eq!(status, 200, "creating {title}: {created}");
        let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

        let (status, fetched) = app.post_op(
            "get_recipe",
            Some(&key),
            &json!({ "branch_id": branch_id }).to_string(),
        );
        assert_eq!(status, 200, "reading {title}: {fetched}");
        let readings = fetched["result"]["versions"][0]["readings"]
            .as_array()
            .expect("one Reading slot per Ingredient Line")
            .clone();
        assert_eq!(
            readings.len(),
            lines.len(),
            "{title}: one slot per line, read or not"
        );

        for (line, reading) in lines.iter().zip(readings.iter()) {
            total += 1;
            if line.amount.is_none() {
                without_quantity += 1;
            }
            // **No mark distinguishes a read line from an unread one** (#71):
            // a slot holds the Reading itself or nothing, and a Reading holds
            // the three parts of ADR 0002 and no field about itself. Its
            // target is spelt two ways, a Food's `target` or a Component's
            // `lineage_id` (ADR 0008, #86), which is still the one part.
            if let Some(reading) = reading.as_object() {
                let mut keys: Vec<&str> = reading.keys().map(String::as_str).collect();
                keys.sort_unstable();
                assert_eq!(
                    keys,
                    ["amount", "lineage_id", "target", "unit"],
                    "{title}: a Reading says what it read, never how sure it is"
                );
            }

            let read_amount = reading["amount"]
                .as_str()
                .and_then(kamosu::units::parse_amount);
            let read_unit = reading["unit"].as_str();
            match line.amount {
                Some(expected) => {
                    amount_right +=
                        usize::from(read_amount.is_some_and(|read| (read - expected).abs() < 1e-6));
                }
                // A line with no quantity must be read with none: inventing
                // one would be guessing at a number about to be shopped by.
                None => amount_right += usize::from(read_amount.is_none()),
            }
            if ground_truth_is_clean(line) {
                scored += 1;
                unit_right += usize::from(same_unit(line.unit.as_deref(), read_unit));
                // The stronger claim, and the one that is not at the mercy of
                // Crouton's coarse vocabulary: where the export named a Unit,
                // Kamosu read one too. Crouton files `3 cloves garlic` as a
                // bare count because it has no word for a clove, so the
                // disagreements in the other direction are Kamosu being the
                // more precise of the two and are not counted against it.
                missed_units += usize::from(line.unit.is_some() && read_unit.is_none());
            }
            let names_a_food = reading["target"].as_str().is_some_and(|t| !t.is_empty());
            // A line offering a choice names no Food by design (#148), so it
            // is neither a success nor a failure of `named`. What it must not
            // do is name one, which is what it did before.
            if written_offers_a_choice(&line.written) {
                choices += 1;
                if names_a_food {
                    choices_named_as_foods.push(format!(
                        "{title}: {:?} read as {:?}",
                        line.written, reading["target"]
                    ));
                }
            } else {
                named += usize::from(names_a_food);
            }
            // A Food named by a bare Unit word is a Reading that invented an
            // answer, and it counted towards `named` above as a success. So it
            // is counted here by name instead, where no share can hide it
            // (#160).
            if let Some(target) = reading["target"].as_str()
                && kamosu::reading::is_a_unit_word(target)
            {
                measures_named_as_foods
                    .push(format!("{title}: {:?} read as {target:?}", line.written));
            }
            // The same, for a Food that is only a describing word, `boneless`,
            // or that kept `to taste` in its name, in any Language (#163).
            if let Some(target) = reading["target"].as_str()
                && (kamosu::reading::is_only_describing(target)
                    || kamosu::reading::keeps_to_taste(target))
            {
                descriptions_named_as_foods
                    .push(format!("{title}: {:?} read as {target:?}", line.written));
            }
        }
    }

    assert_eq!(total, 863, "the corpus is 863 real Ingredient Lines");
    let no_quantity_share = without_quantity as f64 / total as f64;
    assert!(
        (no_quantity_share - 0.28).abs() < 0.01,
        "the measured 28%-without-quantity figure is a fixture fact (ADR 0002): got {:.1}%",
        no_quantity_share * 100.0
    );

    let offering_no_choice = total - choices;
    let amount_share = amount_right as f64 / total as f64;
    // Printed rather than only asserted: the floors below are deliberately
    // slack, and the day someone improves this module they want the number.
    eprintln!(
        "read {total} lines: amounts {:.1}%, Units {:.1}% of {scored} scored, \
         a Food named on {:.1}% of lines offering no choice, no quantity on {:.1}%",
        amount_share * 100.0,
        unit_right as f64 / scored as f64 * 100.0,
        named as f64 / offering_no_choice as f64 * 100.0,
        no_quantity_share * 100.0,
    );
    assert!(
        amount_share >= 0.99,
        "amounts were read at 100% when this was measured (ADR 0036); got {:.1}%",
        amount_share * 100.0
    );
    assert_eq!(
        missed_units, 0,
        "every Unit the export names is a Unit Kamosu reads (ADR 0036)"
    );
    let unit_share = unit_right as f64 / scored as f64;
    assert!(
        unit_share >= 0.95,
        "Units agreed with the export on 96.4% of lines when this was measured \
         (ADR 0036), every disagreement being a Unit Crouton had no word for; \
         got {:.1}%",
        unit_share * 100.0
    );
    // Every line that names something nameable names a Food, which is what
    // #47's rules are fed by and what a shopping list is built on.
    let named_share = named as f64 / offering_no_choice as f64;
    assert!(
        named_share >= 0.95,
        "a line names a Food even where it carries no quantity; got {:.1}%",
        named_share * 100.0
    );

    assert!(
        choices_named_as_foods.is_empty(),
        "no line offering a choice names a Food: {choices_named_as_foods:#?}"
    );

    assert!(
        measures_named_as_foods.is_empty(),
        "no Reading names a Unit as its Food: {measures_named_as_foods:#?}"
    );

    assert!(
        descriptions_named_as_foods.is_empty(),
        "no Reading names a Food by how it is prepared or seasoned: \
         {descriptions_named_as_foods:#?}"
    );

    // **An unread line is a working line** (ADR 0002): the whole library came
    // back through a real Operation, every line of it present and in order,
    // whether Kamosu read it or not.
    let (status, shelf) = app.post_op("search_recipes", Some(&key), "{}");
    assert_eq!(status, 200, "{shelf}");
    assert_eq!(
        shelf["result"]["recipes"].as_array().map(Vec::len),
        Some(86),
        "every recipe is on the shelf, read or unread"
    );
}
