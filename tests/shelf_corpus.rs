//! The shelf, put through Aurélien's real 86-recipe Crouton export (#62).
//!
//! Three flattering examples prove nothing about a screen that has to hold a
//! whole library, so every recipe in the real export is created through the
//! real `create_recipe` Operation and the real shelf is then read back through
//! `search_recipes`. What it asserts is what ADR 0027 promised over a real
//! library: one card per Lineage, alphabetical, an exact title first, and a
//! result that can quote the line that matched — including for the nine
//! recipes that are not written in English.
//!
//! The export lives at `samples/crouton/` and is gitignored — personal,
//! 110 MB, and never committed (see `docs/research/crouton-real-export.md`).
//! This test is `#[ignore]`d so `just test` and CI never need it: run it
//! deliberately with `cargo test --test shelf_corpus -- --ignored` on a
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

/// One `.crumb` turned into what `create_recipe` takes. Photographs are left
/// out deliberately: #45 already drives every one of them through the real
/// upload, and what is under test here is the shelf, not the pictures.
fn recipe_input(crumb: &Value) -> Option<Value> {
    let title = crumb["name"].as_str()?.trim();
    if title.is_empty() {
        return None;
    }

    let mut lines: Vec<&Value> = crumb["ingredients"].as_array()?.iter().collect();
    lines.sort_by_key(|line| line["order"].as_i64().unwrap_or(0));
    let ingredients: Vec<Value> = lines
        .iter()
        .filter_map(|line| {
            let name = line["ingredient"]["name"].as_str()?.trim();
            (!name.is_empty()).then(|| json!({ "kind": "ingredient", "text": name }))
        })
        .collect();

    let mut written: Vec<&Value> = crumb["steps"].as_array()?.iter().collect();
    written.sort_by_key(|step| step["order"].as_i64().unwrap_or(0));
    let steps: Vec<Value> = written
        .iter()
        .filter_map(|step| {
            let text = step["step"].as_str()?.trim();
            let kind = if step["isSection"].as_bool().unwrap_or(false) {
                "section"
            } else {
                "step"
            };
            (!text.is_empty()).then(|| json!({ "kind": kind, "text": text }))
        })
        .collect();

    Some(json!({ "title": title, "ingredients": ingredients, "steps": steps }))
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

fn titles(entries: &[Value]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| entry["title"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs the personal, gitignored samples/crouton export on disk"]
async fn the_real_library_reads_as_one_alphabetical_shelf_and_every_result_says_what_matched() {
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
        .mint_access_key(&person, "corpus shelf", false)
        .unwrap()
        .secret;
    let (_, kitchen) = app.post_op("create_kitchen", Some(&key), r#"{"name":"Corpus Kitchen"}"#);
    let kitchen_id = kitchen["result"]["id"].as_str().unwrap().to_string();

    for recipe in &recipes {
        let mut input = recipe.clone();
        input["kitchen_id"] = json!(kitchen_id);
        let (status, created) = app.post_op("create_recipe", Some(&key), &input.to_string());
        assert_eq!(status, 200, "{} — {created}", recipe["title"]);
    }

    let search = |body: Value| -> Value {
        let (status, answer) = app.post_op("search_recipes", Some(&key), &body.to_string());
        assert_eq!(status, 200, "{answer}");
        answer["result"].clone()
    };

    // One card per Lineage, over the whole real library.
    let shelf = search(json!({}));
    let entries = shelf["recipes"].as_array().unwrap();
    assert_eq!(
        entries.len(),
        86,
        "86 recipes, none of them doubled and none of them missing"
    );

    // Alphabetical — asserted against the app's own fold rather than a copy of
    // it here, so changing the rule cannot leave this test quietly checking the
    // old one.
    let shown = titles(entries);
    let mut expected = shown.clone();
    expected.sort_by_key(|title| kamosu::core::folded_for_search(title));
    assert_eq!(shown, expected, "the real shelf is alphabetical");

    // A word that runs right through the library: every hit explains itself.
    let chocolate = search(json!({ "query": "chocolate" }));
    let hits = chocolate["recipes"].as_array().unwrap();
    assert!(
        hits.len() >= 5,
        "the real library is full of chocolate: {:#?}",
        titles(hits)
    );
    assert!(
        hits.iter().all(|hit| {
            hit["matched"]["line"]
                .as_str()
                .is_some_and(|line| !line.trim().is_empty())
        }),
        "every result quotes the line that matched: {hits:#?}"
    );

    // The exact title is what most searching is for, and it wins.
    let exact = search(json!({ "query": "Chocolate Chip Cookies" }));
    assert_eq!(
        exact["recipes"][0]["title"],
        json!("Chocolate Chip Cookies"),
        "an exact title ranks first even among its own near-namesakes: {:#?}",
        titles(exact["recipes"].as_array().unwrap())
    );
    assert_eq!(exact["recipes"][0]["matched"]["where"], json!("title"));

    // The nine non-English recipes are found in their own words — the half of
    // a real library a word search built only for English silently loses
    // (docs/research/crouton-real-export.md, §9).
    for (query, expected) in [
        ("Îles Flottantes", "Îles Flottantes"),
        ("iles flottantes", "Îles Flottantes"),
        ("Purée de Pommes de Terre", "Purée de Pommes de Terre"),
        ("Bollo limpio", "Bollo limpio"),
    ] {
        let found = search(json!({ "query": query }));
        assert_eq!(
            found["recipes"][0]["title"],
            json!(expected),
            "searching {query:?} found {:#?}",
            titles(found["recipes"].as_array().unwrap())
        );
    }

    // Nothing found is an ordinary answer over a real library too.
    let nothing = search(json!({ "query": "osso buco alla milanese" }));
    assert_eq!(nothing["recipes"], json!([]));
    assert_eq!(nothing["query"], json!("osso buco alla milanese"));
}
