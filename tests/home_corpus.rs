//! Home's computed shelves, put through Aurélien's real 86-recipe Crouton
//! export (#64).
//!
//! Three invented recipes prove nothing about a screen whose whole job is to
//! pick four handfuls out of a real library, so every recipe in the real
//! export is created through the real `create_recipe` Operation — **with the
//! times it actually carries** — and Home is then read back through
//! `home_shelves`.
//!
//! What it pins is the fact the *quick tonight* shelf lives or dies by: **36 of
//! the 86 recipes state no time at all**, and not one of them may be called
//! quick. A threshold measured against a library where seven recipes in ten
//! have a time would be a different threshold; asserting the real figure here
//! is what stops the shelf being tuned against an imagined library.
//!
//! The export lives at `samples/crouton/` and is gitignored — personal, 110 MB,
//! and never committed (see `docs/research/crouton-real-export.md`). This test
//! is `#[ignore]`d so `just test` and CI never need it: run it deliberately
//! with `cargo test --test home_corpus -- --ignored` on a machine that has the
//! export.

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

/// One `.crumb` turned into what `create_recipe` takes — the title and the two
/// times, which is all this test is about. Crouton's `duration` is the prep and
/// its `cookingDuration` the cooking; a recipe may carry either, both or
/// neither, and *neither* is the case under test.
fn recipe_input(crumb: &Value) -> Option<Value> {
    let title = crumb["name"].as_str()?.trim();
    if title.is_empty() {
        return None;
    }
    Some(json!({
        "title": title,
        "prep_time_minutes": crumb["duration"].as_i64(),
        "cook_time_minutes": crumb["cookingDuration"].as_i64(),
    }))
}

fn corpus() -> Option<Vec<Value>> {
    let file = std::fs::File::open(corpus_path()?).expect("open the corpus zip");
    let mut archive = zip::ZipArchive::new(file).expect("read the corpus zip");
    let mut recipes = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).expect("read entry");
        if !entry.name().ends_with(".crumb") {
            continue;
        }
        let mut text = String::new();
        entry.read_to_string(&mut text).expect("crumb is text");
        let crumb: Value = serde_json::from_str(&text).expect("crumb is JSON");
        if let Some(input) = recipe_input(&crumb) {
            recipes.push(input);
        }
    }
    Some(recipes)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs the personal, gitignored samples/crouton export on disk"]
async fn quick_tonight_over_the_real_library_never_guesses_at_a_time_it_does_not_have() {
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

    // **The fixture fact.** 36 of the 86 real recipes state no time at all —
    // not a rounding error at the edge of the library but 42% of it. Kamosu
    // calls none of them quick, for the same reason it shows a quantity it
    // could not read whole rather than guessing at it (ADR 0002).
    let untimed = recipes
        .iter()
        .filter(|recipe| {
            recipe["prep_time_minutes"].is_null() && recipe["cook_time_minutes"].is_null()
        })
        .count();
    assert_eq!(
        untimed, 36,
        "the real library states no time for 36 of its 86 recipes"
    );

    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "corpus home", false)
        .unwrap()
        .secret;

    for recipe in &recipes {
        let input = recipe.clone();
        let (status, created) = app.post_op("create_recipe", Some(&key), &input.to_string());
        assert_eq!(status, 200, "{} — {created}", recipe["title"]);
    }

    let (status, home) = app.post_op("home_shelves", Some(&key), "{}");
    assert_eq!(status, 200, "{home}");
    let result = &home["result"];
    let shelf = |name: &str| -> Option<Vec<String>> {
        result["shelves"].as_array()?.iter().find_map(|shelf| {
            (shelf["name"] == json!(name)).then(|| {
                shelf["recipes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|card| card["title"].as_str().unwrap().to_string())
                    .collect()
            })
        })
    };

    let line = result["quick_tonight_minutes"].as_i64().expect("the line");
    assert_eq!(line, 30, "the line the shelf is drawn at");

    // What the line actually catches in the real library, counted here from
    // the export rather than from the answer — so this checks Kamosu's sum
    // against the corpus rather than against itself.
    let deserved: Vec<&Value> = recipes
        .iter()
        .filter(|recipe| {
            let prep = recipe["prep_time_minutes"].as_i64();
            let cook = recipe["cook_time_minutes"].as_i64();
            (prep.is_some() || cook.is_some()) && prep.unwrap_or(0) + cook.unwrap_or(0) <= line
        })
        .collect();
    assert_eq!(
        deserved.len(),
        27,
        "27 of the real library's 50 timed recipes come in at or under half an hour"
    );

    // Home is a suggestion, not the library: the shelf stops well short of all
    // 27 rather than becoming something to search through.
    let quick = shelf("quick_tonight").expect("a quick tonight shelf");
    assert_eq!(quick.len(), 12, "one shelf, cut to length: {quick:#?}");

    // And not one untimed recipe is on it, however plausibly quick its name.
    let untimed_titles: Vec<&str> = recipes
        .iter()
        .filter(|recipe| {
            recipe["prep_time_minutes"].is_null() && recipe["cook_time_minutes"].is_null()
        })
        .map(|recipe| recipe["title"].as_str().unwrap())
        .collect();
    for title in &untimed_titles {
        assert!(
            !quick.iter().any(|shown| shown == title),
            "{title} states no time, so Kamosu may not call it quick: {quick:#?}"
        );
    }
    // Including the one whose *name* says ten minutes: the words are not a
    // time, and reading a duration out of a title is a guess wearing a fact's
    // clothes.
    assert!(
        untimed_titles
            .iter()
            .any(|title| title.starts_with("10 Minute")),
        "the real library contains '10 Minute Chili Garlic Silken Tofu' with no stated time"
    );

    // Nobody has cooked anything on a library just imported, so *never cooked*
    // is the whole library and *cooked most* is not sent at all.
    assert_eq!(shelf("cooked_most"), None, "nothing has been cooked yet");
    assert_eq!(
        shelf("never_cooked").expect("a never cooked shelf").len(),
        12,
        "every recipe is uncooked, and the shelf is still a shelf"
    );
    assert_eq!(shelf("recently_opened"), None, "nothing has been opened");
}
