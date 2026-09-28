//! Translations, put through Aurélien's real 86-recipe Crouton export
//! (#56, ADR 0006).
//!
//! Three invented examples prove nothing about a rule whose whole justification
//! is a real library that is one-tenth not in English. So every recipe in the
//! real export is created through the real `create_recipe` Operation with **no
//! Language given**, and what Kamosu read out of each one's own text is then
//! read back through `get_recipe` and `search_recipes`.
//!
//! What it asserts is what ADR 0006 promised over that library: the nine
//! recipes whose text is French are filed as French and nothing else is; the
//! one that mixes Languages inside itself can be called **Unknown** and is then
//! never nagged, never badged and never hidden; and a Translation of a real
//! French recipe is an ordinary Branch of its Lineage whose staleness is
//! arithmetic.
//!
//! The export lives at `samples/crouton/` and is gitignored — personal, 110 MB,
//! and never committed (see `docs/research/crouton-real-export.md`). This test
//! is `#[ignore]`d so `just test` and CI never need it: run it deliberately
//! with `cargo test --test translations_corpus -- --ignored` on a machine that
//! has the export.

mod support;

use serde_json::{Value, json};
use std::io::Read;

/// The recipes whose **own text** is French. Seven of them are the ones
/// `docs/research/crouton-real-export.md` §9 names; *Oven Potatoes* and *Du
/// sucre glace (optionnel)* are two more the research counted as English
/// because it was reading titles.
///
/// *Bollo limpio* is deliberately absent, and it is the interesting one. §9
/// counted it Spanish on the strength of its title; its ingredients and steps
/// are written in English throughout. Kamosu files a recipe by what it is
/// written in, not by what it is called, so English is the right answer and
/// this list records why.
const FRENCH: [&str; 9] = [
    "Curry Japonais sans eau (Musui Curry)",
    "Du sucre glace (optionnel)",
    "Gâteau Au Chocolat",
    "Meringue : recette facile",
    "Oven Potatoes",
    "Purée de Pommes de Terre",
    "Réussir Un Biscuit Roulé : Recette, Astuces Et Photos Pas À Pas - Un Déjeuner De Soleil",
    "Sukiyaki Udon",
    "Îles Flottantes",
];

/// The one that mixes Languages inside itself: an English title and an English
/// ingredient over a French method. Detection reads it as French — confidently,
/// and it is not wrong about the sentences it read. Unknown is the cook's
/// answer to a question no detector can settle.
const MIXED: &str = "Sukiyaki Udon";

fn corpus_path() -> Option<std::path::PathBuf> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/crouton");
    std::fs::read_dir(&dir).ok()?.find_map(|entry| {
        let path = entry.ok()?.path();
        (path.extension().and_then(|e| e.to_str()) == Some("zip")).then_some(path)
    })
}

/// One `.crumb` turned into what `create_recipe` takes — the same reading
/// `shelf_corpus` uses, and deliberately without Photographs: what is under
/// test here is the Language of the words.
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs the personal, gitignored samples/crouton export on disk"]
async fn the_real_librarys_languages_are_read_off_its_own_words_and_never_written_over() {
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
        .mint_access_key(&person, "corpus translations", false)
        .unwrap()
        .secret;
    // The reader reads English, which is what makes the French recipes worth
    // asserting: nothing here is French because the account asked for French.
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );

    // Created with no Language given, so every one of these is Kamosu reading
    // the recipe's own text (ADR 0025: an import has none to disagree with).
    let mut branches: Vec<(String, String, String)> = Vec::new();
    for recipe in &recipes {
        let input = recipe.clone();
        let (status, created) = app.post_op("create_recipe", Some(&key), &input.to_string());
        assert_eq!(status, 200, "{} — {created}", recipe["title"]);
        branches.push((
            created["result"]["branch_id"].as_str().unwrap().to_string(),
            recipe["title"].as_str().unwrap().to_string(),
            created["result"]["language"].as_str().unwrap().to_string(),
        ));
    }

    let mut french: Vec<&str> = branches
        .iter()
        .filter(|(_, _, language)| language == "fr")
        .map(|(_, title, _)| title.as_str())
        .collect();
    french.sort_by_key(|title| kamosu::core::folded_for_search(title));

    let mut expected: Vec<&str> = FRENCH.to_vec();
    expected.sort_by_key(|title| kamosu::core::folded_for_search(title));
    assert_eq!(
        french, expected,
        "nine of the real 86 are written in French, and these are the nine"
    );

    // No recipe in the real library is read as Spanish. *Bollo limpio* has a
    // Spanish title and an English method, and Kamosu files what is written.
    assert!(
        branches.iter().all(|(_, _, language)| language != "es"),
        "nothing in this library is written in Spanish"
    );
    let (_, bollo, bollo_language) = branches
        .iter()
        .find(|(_, title, _)| title == "Bollo limpio")
        .expect("Bollo limpio is in the corpus");
    assert_eq!(
        bollo_language, "en",
        "{bollo} is written in English however it is named"
    );

    // ── The one that mixes Languages inside itself ────────────────────────────
    let (mixed_branch, _, mixed_language) = branches
        .iter()
        .find(|(_, title, _)| title == MIXED)
        .expect("Sukiyaki Udon is in the corpus");
    assert_eq!(
        mixed_language, "fr",
        "the detector reads the sentences, and its sentences are French"
    );

    let (status, set) = app.post_op(
        "set_recipe_language",
        Some(&key),
        &json!({ "branch_id": mixed_branch, "language": "unknown" }).to_string(),
    );
    assert_eq!(status, 200, "{set}");

    // From here: no prompt. Saving it again reads the same French sentences
    // and says nothing about them.
    let mut resaved = recipes
        .iter()
        .find(|recipe| recipe["title"] == json!(MIXED))
        .expect("Sukiyaki Udon's input")
        .clone();
    resaved["branch_id"] = json!(mixed_branch);
    resaved["note"] = json!("Ajoutez un jaune d'oeuf sur le dessus juste avant de servir.");
    let (status, saved) = app.post_op("save_recipe_version", Some(&key), &resaved.to_string());
    assert_eq!(status, 200, "{saved}");
    assert_eq!(saved["result"]["language"], json!("unknown"));
    assert_eq!(
        saved["result"]["language_offer"],
        json!(null),
        "a recipe honestly written in two Languages is never nagged"
    );

    // No badge, and no hiding, on a shelf read in English.
    let (status, shelf) = app.post_op("search_recipes", Some(&key), "{}");
    assert_eq!(status, 200, "{shelf}");
    let entries = shelf["result"]["recipes"].as_array().unwrap();
    assert_eq!(
        entries.len(),
        86,
        "one card per Lineage over the real library"
    );
    let mixed_entry = entries
        .iter()
        .find(|entry| entry["title"] == json!(MIXED))
        .expect("shown to a reader whose Language it is not in");
    assert_eq!(mixed_entry["language"], json!("unknown"));
    assert_eq!(mixed_entry["language_fallback"], json!(false));

    // The other eight French recipes are shown too, and marked — a preference
    // never hides a recipe from its owner.
    let marked = entries
        .iter()
        .filter(|entry| entry["language_fallback"] == json!(true))
        .count();
    assert_eq!(
        marked,
        FRENCH.len() - 1,
        "every French recipe but the Unknown one is shown to an English reader, marked"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs the personal, gitignored samples/crouton export on disk"]
async fn a_real_french_recipe_translates_into_an_ordinary_branch_that_falls_behind_exactly() {
    let Some(recipes) = corpus() else {
        eprintln!(
            "skipping: no samples/crouton/*.zip on this machine (personal, gitignored export)"
        );
        return;
    };
    let source = recipes
        .iter()
        .find(|recipe| recipe["title"] == json!("Îles Flottantes"))
        .expect("Îles Flottantes is in the corpus")
        .clone();

    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "corpus translations", false)
        .unwrap()
        .secret;

    let input = source.clone();
    let (status, created) = app.post_op("create_recipe", Some(&key), &input.to_string());
    assert_eq!(status, 200, "{created}");
    let french = created["result"]["branch_id"].as_str().unwrap().to_string();
    assert_eq!(created["result"]["language"], json!("fr"));
    let french_head = created["result"]["head_version_id"]
        .as_str()
        .unwrap()
        .to_string();

    // An English rendering, written as an agent under Aurélien's own Credential
    // would write it: an ordinary Branch of the same Lineage.
    let (status, translated) = app.post_op(
        "start_translation",
        Some(&key),
        &json!({
            "branch_id": french,
            "language": "en",
            "title": "Floating Islands",
            "ingredients": [
                { "kind": "ingredient", "text": "6 fresh eggs, whites and yolks separated" },
                { "kind": "ingredient", "text": "150 g of caster sugar for the custard" },
                { "kind": "ingredient", "text": "1 litre of whole milk" },
            ],
            "steps": [
                { "kind": "step", "text": "Warm the milk gently with a split vanilla pod." },
                { "kind": "step", "text": "Whisk the whites to stiff peaks and poach them in the milk." },
                { "kind": "step", "text": "Thicken the custard with the yolks over a low heat." },
            ],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{translated}");
    let english = translated["result"]["branch_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        translated["result"]["lineage_id"], created["result"]["lineage_id"],
        "the same recipe written differently, not a second recipe"
    );
    assert_eq!(
        translated["result"]["translation"]["translates_version_id"],
        json!(french_head)
    );
    assert_eq!(
        translated["result"]["translation"]["versions_behind"],
        json!(0)
    );

    // The French moves on. Nobody marks the English stale; it becomes stale.
    app.core
        .db()
        .with_conn(|conn| {
            conn.execute(
                "UPDATE branch_versions \
                    SET created_at = strftime('%Y-%m-%dT%H:%M:%fZ','now','-2 hours') \
                  WHERE branch_id = ?1",
                rusqlite::params![french],
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            Ok(())
        })
        .unwrap();
    let mut edit = source.clone();
    edit["branch_id"] = json!(french);
    edit["note"] = json!("Servez bien frais, le jour même de préférence.");
    let (status, saved) = app.post_op("save_recipe_version", Some(&key), &edit.to_string());
    assert_eq!(status, 200, "{saved}");

    let (_, read) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": english }).to_string(),
    );
    assert_eq!(
        read["result"]["translation"]["versions_behind"],
        json!(1),
        "one Version behind, counted rather than recorded"
    );
    assert_eq!(
        read["result"]["translation"]["source_branch_id"],
        json!(french)
    );

    // And searching in either Language finds the one recipe, once.
    for query in ["floating islands", "îles flottantes"] {
        let (status, shelf) = app.post_op(
            "search_recipes",
            Some(&key),
            &json!({ "query": query }).to_string(),
        );
        assert_eq!(status, 200, "{shelf}");
        let entries = shelf["result"]["recipes"].as_array().unwrap();
        assert_eq!(entries.len(), 1, "searching {query:?} found {entries:#?}");
    }
}
