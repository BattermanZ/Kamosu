//! The cooking screen's two derivations, put through Aurélien's real 86-recipe
//! Crouton export (#61, ADR 0011).
//!
//! ADR 0011's whole argument is a measured one — a cooking screen showing only
//! the instruction sends the cook back to the ingredient list, because 28% of
//! real Ingredient Lines carry no quantity and effectively none of the 599
//! Steps repeats one. The figures this test asserts are the other half of that
//! measurement, taken from the same export: **how many real Steps offer a
//! timer**, and that the reader never invents one out of a temperature, a
//! weight or an oven dial.
//!
//! Three invented steps prove nothing about a reader that has to face a French
//! *laisser reposer 1 heure* and an English *bake at 450F/230C for 20-30 min*
//! on the same afternoon, so every Step in the real export goes through the
//! real `create_recipe` and is read back through the real `get_recipe`.
//!
//! The export lives at `samples/crouton/` and is gitignored — personal, 110 MB,
//! and never committed (see `docs/research/crouton-real-export.md`). This test
//! is `#[ignore]`d so `just test` and CI never need it: run it deliberately
//! with `cargo test --test cooking_corpus -- --ignored` on a machine that has
//! the export.

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

/// One `.crumb` turned into what `create_recipe` takes. Only the Steps matter
/// here — the amounts panel is driven by Readings, which no parser writes yet
/// (#71) — so the ingredient list comes along whole and is not read.
fn recipe_input(crumb: &Value) -> Option<Value> {
    let title = crumb["name"].as_str()?.trim();
    if title.is_empty() {
        return None;
    }
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
    Some(json!({ "title": title, "steps": steps }))
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
async fn a_quarter_of_the_real_steps_offer_a_timer_and_none_of_the_rest_is_invented() {
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
        .mint_access_key(&person, "corpus cooking", false)
        .unwrap()
        .secret;
    let (_, kitchen) = app.post_op("create_kitchen", Some(&key), r#"{"name":"Corpus Kitchen"}"#);
    let kitchen_id = kitchen["result"]["id"].as_str().unwrap().to_string();

    let mut branch_ids = Vec::new();
    for recipe in &recipes {
        let mut input = recipe.clone();
        input["kitchen_id"] = json!(kitchen_id);
        let (status, created) = app.post_op("create_recipe", Some(&key), &input.to_string());
        assert_eq!(status, 200, "{} — {created}", recipe["title"]);
        branch_ids.push(created["result"]["branch_id"].as_str().unwrap().to_string());
    }

    // Every real Step, beside the timer Kamosu read out of it.
    let mut steps: Vec<(String, Option<i64>)> = Vec::new();
    for branch_id in &branch_ids {
        let (status, fetched) = app.post_op(
            "get_recipe",
            Some(&key),
            &json!({ "branch_id": branch_id }).to_string(),
        );
        assert_eq!(status, 200, "{fetched}");
        let version = &fetched["result"]["versions"][0];
        let written = version["content"]["steps"].as_array().unwrap();
        let cooking = version["cooking"]["steps"].as_array().unwrap();
        assert_eq!(
            written.len(),
            cooking.len(),
            "one slot per row of the Version's own steps, in the same order"
        );
        for (row, slot) in written.iter().zip(cooking) {
            if row["kind"] != "step" {
                assert_eq!(*slot, json!(null), "a Section is not a Step");
                continue;
            }
            steps.push((
                row["text"].as_str().unwrap().to_string(),
                slot["timer_seconds"].as_i64(),
            ));
        }
    }

    assert_eq!(
        steps.len(),
        579,
        "the 579 real Steps the cooking screen's type scale was fitted against"
    );

    // **The measured fixture fact.** 154 of the 579 real Steps name a duration
    // Kamosu can read — a quarter of them — which is what makes the timer worth
    // offering at all and what makes the other three quarters' silence normal
    // rather than a failure. A change to the reader that moved this figure far
    // is either a real improvement or a regression, and either way it should be
    // looked at rather than absorbed.
    let offered = steps.iter().filter(|(_, timer)| timer.is_some()).count();
    assert_eq!(
        offered, 154,
        "154 of 579 real Steps offer a timer; the reader now offers {offered}"
    );

    // Real Steps, quoted from the export, with the timer each must offer. They
    // are the shapes the reader was built against: a plain duration, a range in
    // three different dashes, a `more` between the number and its unit, an hour
    // written in French, and a duration Kamosu cannot read because the cook
    // never wrote a number ("a few minutes"), which is an ordinary answer of
    // nothing rather than a miss.
    let expected: &[(&str, Option<i64>)] = &[
        ("Knead the dough for 3–4 minutes.", Some(180)),
        ("Steam for 30–40 minutes.", Some(1800)),
        ("Bake at 450F/230C for 20-30 min.", Some(1200)),
        ("cover and refrigerate 8 hours or overnight", Some(28800)),
        ("Add choi sum for last 1 minute of cooking.", Some(60)),
        ("Cook at 120 degrees for 2 hours covered", Some(7200)),
        (
            "Spread the rice in an even layer, gently pressing down. Let it cook for a few \
             minutes so that the bottom starts to develop a crisp crust.",
            None,
        ),
    ];
    for (text, timer) in expected {
        let found = steps
            .iter()
            .find(|(written, _)| written == text)
            .unwrap_or_else(|| panic!("the export no longer contains the Step {text:?}"));
        assert_eq!(
            found.1, *timer,
            "the timer read out of the real Step {text:?}"
        );
    }

    // No timer is ever invented out of a number that is not a duration.
    for (text, timer) in &steps {
        if let Some(seconds) = timer {
            assert!(
                *seconds >= 1,
                "a timer is never zero or negative: {text:?} → {seconds}"
            );
            assert!(
                *seconds <= 60 * 60 * 24,
                "a duration longer than a day was read out of {text:?}: {seconds}s — \
                 that is a number the reader mistook for a duration"
            );
        }
    }

    // The oven ladder's own numbers are not durations. Every real Step with a
    // temperature and no duration word must offer nothing.
    let oven: Vec<&(String, Option<i64>)> = steps
        .iter()
        .filter(|(text, _)| {
            let lower = text.to_lowercase();
            (lower.contains("degrees") || lower.contains("°"))
                && ![
                    "minute", "min", "hour", "hr", "second", "sec", "heure", "mn",
                ]
                .iter()
                .any(|word| lower.contains(word))
        })
        .collect();
    assert!(
        oven.len() >= 5,
        "the corpus really does carry oven temperatures with no duration beside them"
    );
    for (text, timer) in oven {
        assert_eq!(
            *timer, None,
            "a temperature was read as a duration: {text:?}"
        );
    }
}
