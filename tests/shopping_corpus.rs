//! The Shopping List, put through Aurélien's real 86-recipe Crouton export
//! (#73, ADR 0024).
//!
//! **A shopping list built from ten flattering lines proves nothing.** What it
//! has to survive is a real library: Foods that merge across recipes, Units
//! that will not convert into one another, and the 28% of Ingredient Lines
//! that carry no quantity at all. All three are here in quantity, and none of
//! them was arranged.
//!
//! Every recipe goes in through the real `create_recipe`, every recipe is
//! chosen through the real `add_to_shopping_list`, and the list comes back
//! through the real `get_shopping_list` — so what is scored is what both Doors
//! serve, not a function called directly.
//!
//! **The floors are floors, not targets.** They sit just under what was
//! measured on the day, so this fails on a regression and never on an
//! improvement.
//!
//! The export lives at `samples/crouton/` and is gitignored — personal,
//! 110 MB, never committed (see `docs/research/crouton-real-export.md`). This
//! test is `#[ignore]`d so `just test` and CI never need it: run it
//! deliberately with `cargo test --test shopping_corpus -- --ignored` on a
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

/// How Crouton's `quantityType` is ordinarily written on a line — the fifteen
/// the real export contains, and no others. Four of them (`can`, `bottle`,
/// `packet`, `pinch`) are outside Kamosu's closed convertible set, which is
/// exactly what makes this corpus able to test a row that will not add.
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

/// An amount as a cook writes it.
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

/// One recipe of the export: its title and the written lines a cook would have
/// typed, reassembled from the three parts Crouton kept separately.
fn recipe_of(crumb: &Value) -> Option<(String, Vec<String>)> {
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
            let Some(amount) = quantity["amount"].as_f64().filter(|a| *a != 0.0) else {
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
    Some((title, lines))
}

fn corpus() -> Option<Vec<(String, Vec<String>)>> {
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
        if let Some(recipe) = recipe_of(&crumb) {
            recipes.push(recipe);
        }
    }
    Some(recipes)
}

fn kinds_of(row: &Value) -> Vec<&str> {
    row["parts"]
        .as_array()
        .expect("parts")
        .iter()
        .map(|part| part["kind"].as_str().expect("a kind"))
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs the personal, gitignored samples/crouton export on disk"]
async fn the_whole_real_library_on_one_list_merges_adds_and_admits_what_it_cannot() {
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
        .mint_access_key(&person, "corpus shopping", false)
        .unwrap()
        .secret;
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        r#"{"reading_language":"en","reading_measures":"metric"}"#,
    );

    // **Several corpus recipes chosen at once** — all of them, which is the
    // hardest version of the question and the one that finds real merges.
    let mut written_lines = 0usize;
    for (title, lines) in &recipes {
        let ingredients: Vec<Value> = lines
            .iter()
            .map(|text| json!({ "kind": "ingredient", "text": text }))
            .collect();
        written_lines += lines.len();
        let (status, created) = app.post_op(
            "create_recipe",
            Some(&key),
            &json!({ "title": title, "ingredients": ingredients }).to_string(),
        );
        assert_eq!(status, 200, "creating {title}: {created}");
        let (status, list) = app.post_op(
            "add_to_shopping_list",
            Some(&key),
            &json!({ "branch_id": created["result"]["branch_id"] }).to_string(),
        );
        assert_eq!(status, 200, "choosing {title}: {list}");
    }

    let (status, list) = app.post_op("get_shopping_list", Some(&key), "{}");
    assert_eq!(status, 200, "{list}");
    let list = &list["result"];
    let rows = list["rows"].as_array().expect("rows");

    assert_eq!(
        list["chosen"].as_array().unwrap().len(),
        86,
        "every recipe chosen, and none of them twice"
    );

    // **Merging is the whole point.** 863 written lines become far fewer
    // things to buy, and a good many rows were fed by more than one recipe.
    assert!(
        rows.len() < written_lines,
        "a merged list is shorter than the lines it was made from: \
         {} rows from {written_lines} lines",
        rows.len()
    );
    let merged = rows
        .iter()
        .filter(|row| row["lines"].as_array().unwrap().len() > 1)
        .count();
    assert!(
        merged >= 105,
        "measured at 115 rows fed by more than one written line; got {merged}"
    );

    let side_by_side: Vec<&Value> = rows
        .iter()
        .filter(|row| row["parts"].as_array().unwrap().len() > 1)
        .collect();
    let unstated: Vec<&Value> = rows
        .iter()
        .filter(|row| kinds_of(row) == ["no_amount"])
        .collect();

    // The scoreboard, printed rather than only asserted: a floor says what
    // must not regress, and this says where the feature actually stands on the
    // day it is run. 560 rows out of 863 written lines, 115 of them fed by
    // more than one line, 45 the arithmetic could not close, 138 riding as
    // `some` — measured 2026-09-01.
    eprintln!(
        "shopping corpus: rows={} lines={written_lines} merged={merged} \
         side_by_side={} unstated={}",
        rows.len(),
        side_by_side.len(),
        unstated.len()
    );

    // **The side-by-side case is real, not hypothetical.** Rows where the
    // Units genuinely will not convert into one another — a can and a gram, a
    // pinch and a teaspoon — carry both amounts rather than one wrong one.
    assert!(
        side_by_side.len() >= 40,
        "measured at 45 rows the arithmetic could not close; got {}",
        side_by_side.len()
    );
    for row in &side_by_side {
        assert!(
            !row["lines"].as_array().unwrap().is_empty(),
            "a row that could not add still shows what it was made from"
        );
        for part in row["parts"].as_array().unwrap() {
            assert!(
                !part["sources"].as_array().unwrap().is_empty(),
                "every amount on a broken-open row names the recipes that fed it"
            );
        }
    }

    // **The *some* case is a quarter of a real library** (#5, ADR 0024): rows
    // built only from lines nobody put a number on. They ride rather than
    // being dropped, because dropping them means coming home short.
    assert!(
        unstated.len() >= 125,
        "measured at 138 rows riding as `some`; got {}",
        unstated.len()
    );

    // **The written lines are always one tap away, whole** (ADR 0002). Not a
    // paraphrase, not the Food's name again: what the cook wrote.
    for row in rows {
        for line in row["lines"].as_array().unwrap() {
            let text = line["text"].as_str().expect("the written line");
            assert!(!text.is_empty(), "a row was made from a line with no text");
            assert!(
                line["recipe"].as_str().is_some_and(|name| !name.is_empty()),
                "every written line says which recipe it came from"
            );
        }
        // **Nothing is ticked** (ADR 0024, ADR 0019): a row has no name to
        // staple a tick to, so it carries exactly these fields and none about
        // itself. `said` is why a Component's line still stands on the list
        // (ADR 0008, #86): a sentence about the recipe, not about the row.
        let mut keys: Vec<&str> = row
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "id",
                "kind",
                "lines",
                "name",
                "name_language",
                "parts",
                "said"
            ],
            "a Shopping Row says what to buy, never how sure it is or whether it is done"
        );
    }

    // **Every row can be bought.** A row with an amount beside a blank name is
    // the one thing a shopping list cannot afford: it cannot be bought and it
    // cannot be asked about.
    for row in rows {
        assert!(
            row["name"]
                .as_str()
                .is_some_and(|name| !name.trim().is_empty()),
            "a row reached the list with nothing to call it: {row}"
        );
    }

    // **The list leaves as text** (ADR 0024). This is why nothing here is
    // ticked: Kamosu decides what to buy and something else carries it round
    // the shop, so the whole real library has to survive the trip out.
    let (status, sent) = app.post_op("shopping_list_as_text", Some(&key), "{}");
    assert_eq!(status, 200, "{sent}");
    let text = sent["result"]["text"].as_str().expect("text").to_string();
    let mut lines = text.lines();
    let header = lines.next().expect("a header line");
    assert!(
        header.len() > 10 && header.contains(" · "),
        "the header carries the date and the recipes it was built from: {header:?}"
    );
    // Every row reached the note as exactly one checklist line (#74), and the
    // note invents none of its own.
    let checklist: Vec<&str> = lines.collect();
    assert_eq!(checklist.len(), rows.len(), "one line per row: {text:?}");
    for (line, row) in checklist.iter().zip(rows) {
        let name = row["name"].as_str().unwrap();
        assert!(
            line.starts_with(&format!("- [ ] {name}")),
            "the note dropped or reordered a row: {name:?} against {line:?}"
        );
    }

    // And reading it changed nothing: Kamosu offers to empty and does not act.
    let (_, after) = app.post_op("get_shopping_list", Some(&key), "{}");
    assert_eq!(
        after["result"]["chosen"].as_array().unwrap().len(),
        86,
        "sending the list does not empty it"
    );

    // Every row says something. A row with no amounts at all is one of the two
    // verbatim kinds — a Loose Item, or a line with no Food to merge under —
    // and never a Food row that quietly lost its amount.
    for row in rows {
        if row["parts"].as_array().unwrap().is_empty() {
            assert_ne!(
                row["kind"], "food",
                "a Food row always carries at least one amount, even if it is `some`"
            );
        }
    }
}
