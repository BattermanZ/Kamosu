//! Reading a pasted recipe, put through Aurélien's real 86-recipe Crouton
//! export (#94).
//!
//! **This corpus is the only honest scoreboard the split has.** Crouton stored
//! each recipe's ingredients and steps as two separate lists, so writing them
//! out one after another rebuilds the paste a cook would have made from the
//! same page — and the correct boundary is already known, because it is where
//! one list stopped and the other started. Eighty recipes carry both.
//!
//! The paste is written the hard way on purpose: no blank line between the two
//! blocks, and no `Ingredients` or `Instructions` above either. A real paste
//! often carries those, and every one of them makes this easier. What is
//! measured here is the case with nothing to go on but the lines themselves.
//!
//! **The floors are floors, not targets.** They sit just under what was
//! measured on the day, so this fails on a regression and never on an
//! improvement. #94 set the bar at the 92% within one line that a throwaway
//! fifteen-line scorer reached without `reading.rs`.
//!
//! The export lives at `samples/crouton/` and is gitignored — personal, 110 MB,
//! and never committed (see `docs/research/crouton-real-export.md`). This test
//! is `#[ignore]`d so `just test` and CI never need it: run it deliberately
//! with `cargo test --test pasting_corpus -- --ignored` on a machine that has
//! the export.

use std::io::Read;

use serde_json::Value;

use kamosu::pasting::read_paste;

fn corpus_path() -> Option<std::path::PathBuf> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/crouton");
    std::fs::read_dir(&dir).ok()?.find_map(|entry| {
        let path = entry.ok()?.path();
        (path.extension().and_then(|e| e.to_str()) == Some("zip")).then_some(path)
    })
}

/// How Crouton's `quantityType` is ordinarily written on a line — the fifteen
/// the real export contains, and no others. The same table `reading_corpus.rs`
/// uses, for the same reason: this is Crouton's vocabulary, not Kamosu's.
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

/// An amount as a cook writes it — whole where it is whole, a vulgar fraction
/// where the export holds one of the halves, thirds, quarters and eighths.
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

/// One recipe of the export, rebuilt as the paste somebody would have made:
/// the title, a blank line, then both lists one after the other. The answer is
/// how many lines the first list had.
struct Pasted {
    title: String,
    text: String,
    boundary: usize,
}

fn ordered<'a>(crumb: &'a Value, field: &str) -> Vec<&'a Value> {
    let mut entries: Vec<&Value> = crumb[field]
        .as_array()
        .map(|a| a.iter().collect())
        .unwrap_or_default();
    entries.sort_by_key(|entry| entry["order"].as_i64().unwrap_or(0));
    entries
}

fn pasted(crumb: &Value) -> Option<Pasted> {
    let title = crumb["name"].as_str()?.trim().to_string();
    if title.is_empty() {
        return None;
    }

    let ingredients: Vec<String> = ordered(crumb, "ingredients")
        .iter()
        .filter_map(|entry| {
            let name = entry["ingredient"]["name"].as_str()?.trim().to_string();
            if name.is_empty() {
                return None;
            }
            let quantity = &entry["quantity"];
            let Some(amount) = quantity["amount"].as_f64().filter(|a| *a != 0.0) else {
                // A line carrying no quantity at all — 28% of this corpus, and
                // the hardest kind to keep on the right side of the boundary.
                return Some(name);
            };
            let unit = unit_word(quantity["quantityType"].as_str()?)?;
            Some(
                [written_amount(amount), unit.to_string(), name]
                    .into_iter()
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        })
        .collect();

    let steps: Vec<String> = ordered(crumb, "steps")
        .iter()
        .filter_map(|entry| {
            let step = entry["step"].as_str()?.trim().to_string();
            (!step.is_empty()).then_some(step)
        })
        .collect();

    if ingredients.is_empty() || steps.is_empty() {
        return None;
    }
    let boundary = ingredients.len();
    let body = ingredients
        .into_iter()
        .chain(steps)
        .collect::<Vec<_>>()
        .join("\n");
    Some(Pasted {
        title: title.clone(),
        text: format!("{title}\n\n{body}"),
        boundary,
    })
}

fn corpus() -> Option<Vec<Pasted>> {
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
        if let Some(recipe) = pasted(&crumb) {
            recipes.push(recipe);
        }
    }
    Some(recipes)
}

/// Measured on 21 September 2026 against the 20 August 2026 export: the
/// boundary is found **exactly in 77.5%** of the eighty recipes (62 of 80) and
/// **within one line in 95.0%** (76 of 80). The floors sit two recipes under
/// each, so this fails on a regression and never on a wobble.
///
/// The within-one figure is the one #94 set a bar for — 92%, what a throwaway
/// fifteen-line scorer reached without `reading.rs` — and it clears it by four
/// recipes. The four it misses by more than a line are named in the output:
/// three have methods clipped short enough to read like a list, and one is a
/// corrupt entry whose whole recipe is inside its title.
const FOUND_EXACTLY: f64 = 0.75;
const FOUND_WITHIN_ONE: f64 = 0.92;

#[test]
#[ignore = "needs samples/crouton/, which is personal and gitignored"]
fn the_boundary_is_found_on_the_real_corpus() {
    let Some(corpus) = corpus() else {
        panic!("no corpus at samples/crouton/ — see the module header");
    };
    assert!(
        corpus.len() >= 78,
        "expected the eighty recipes carrying both lists, found {}",
        corpus.len()
    );

    let mut exactly = 0usize;
    let mut within_one = 0usize;
    let mut missed = Vec::new();
    for recipe in &corpus {
        let read = read_paste(&recipe.text);
        // The title is taken because it stands alone above a blank line, so
        // the boundary indexes the lines underneath it.
        assert_eq!(
            read.title.as_deref(),
            Some(recipe.title.as_str()),
            "the title of {} was not taken",
            recipe.title
        );
        let off_by = read.boundary.abs_diff(recipe.boundary);
        if off_by == 0 {
            exactly += 1;
        }
        if off_by <= 1 {
            within_one += 1;
        } else {
            missed.push(format!(
                "{}: the method starts at {} and was put at {}",
                recipe.title, recipe.boundary, read.boundary
            ));
        }
    }

    let total = corpus.len() as f64;
    let exact_share = exactly as f64 / total;
    let within_share = within_one as f64 / total;
    let of = corpus.len();
    println!(
        "boundary found exactly {exactly}/{of} = {:.1}%, within one line {within_one}/{of} = {:.1}%",
        exact_share * 100.0,
        within_share * 100.0
    );
    for miss in &missed {
        println!("  missed by more than one — {miss}");
    }

    assert!(
        exact_share >= FOUND_EXACTLY,
        "the boundary is found exactly in {:.1}%, under the {:.0}% floor",
        exact_share * 100.0,
        FOUND_EXACTLY * 100.0
    );
    assert!(
        within_share >= FOUND_WITHIN_ONE,
        "the boundary is found within one line in {:.1}%, under the {:.0}% floor",
        within_share * 100.0,
        FOUND_WITHIN_ONE * 100.0
    );
}

/// **Every line arrives exactly as pasted**, on all 1,473 real lines rather
/// than on the handful a unit test can hold. Nothing is reordered, nothing is
/// dropped, and no amount is lifted out of a line (ADR 0002).
///
/// The one repair allowed is the one every importer makes (#69): Crouton
/// scraped its pages without decoding them, so the real export carries
/// `Noodles &amp; choi sum:`, and that is not what anybody typed. Comparing
/// against the decoded text rather than skipping the check keeps the rule
/// exact — any *other* change to a line fails here.
#[test]
#[ignore = "needs samples/crouton/, which is personal and gitignored"]
fn every_line_of_the_real_corpus_arrives_as_it_was_pasted() {
    let Some(corpus) = corpus() else {
        panic!("no corpus at samples/crouton/ — see the module header");
    };
    let mut lines = 0usize;
    for recipe in &corpus {
        let read = read_paste(&recipe.text);
        let written: Vec<String> = recipe
            .text
            .lines()
            .skip(2)
            .filter(|line| !line.trim().is_empty())
            .map(kamosu::entities::decode_entities)
            .collect();
        let came_back: Vec<&str> = read.lines.iter().map(|row| row.text.as_str()).collect();
        assert_eq!(
            written, came_back,
            "{} did not come back line for line",
            recipe.title
        );
        lines += came_back.len();
    }
    println!("{lines} real lines came back exactly as they were pasted");
}
