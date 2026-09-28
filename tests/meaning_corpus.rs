//! Meaning Search, put through Aurélien's real 86-recipe Crouton export (#63).
//!
//! Three flattering examples prove nothing about a search that has to hold a
//! whole library, and a similarity threshold argued in the abstract is a number
//! somebody made up. So every recipe in the real export is created through the
//! real `create_recipe`, the real model is downloaded and the real index built,
//! and the searches below go through `search_recipes` at a real Door.
//!
//! What it asserts is what ADR 0027 and ADR 0029 promised: that meaning finds a
//! recipe words cannot, that an exact title still wins over anything the model
//! thinks, that a result quotes the line it matched on, and that *nothing
//! found* shows the closest anyway rather than an empty screen.
//!
//! The export lives at `samples/crouton/` and is gitignored — personal, 110 MB,
//! never committed. The model is a further ~220 MB fetched from Hugging Face.
//! So this is `#[ignore]`d and `just test` never needs either: run it
//! deliberately with `cargo test --test meaning_corpus -- --ignored` on a
//! machine that has the export and a network.

mod support;

use serde_json::{Value, json};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Where the model is kept between runs of this test.
///
/// The instance under test gets a fresh temporary data directory like every
/// other behaviour test, and `/data/model` inside it is linked here — so the
/// database is new on every run and the 220 MB is fetched once on this machine
/// and never again. Gitignored, like everything else under `.dev/`.
fn shared_model_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(".dev/meaning-model")
}

fn corpus_path() -> Option<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/crouton");
    std::fs::read_dir(&dir).ok()?.find_map(|entry| {
        let path = entry.ok()?.path();
        (path.extension().and_then(|e| e.to_str()) == Some("zip")).then_some(path)
    })
}

/// One `.crumb` turned into what `create_recipe` takes — the same reading
/// `tests/shelf_corpus.rs` uses, so the two tests are looking at one library.
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
        .map(|entry| entry["title"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// The whole library, indexed, behind one Access Key belonging to the Operator.
/// `None` where this machine has no corpus to read.
struct Library {
    app: support::TestApp,
    key: String,
    kitchen_id: String,
}

impl Library {
    fn search(&self, body: Value) -> Value {
        self.search_as(&self.key, body)
    }

    fn search_as(&self, key: &str, body: Value) -> Value {
        let (status, answer) = self
            .app
            .post_op("search_recipes", Some(key), &body.to_string());
        assert_eq!(status, 200, "{answer}");
        answer["result"].clone()
    }

    fn ask(&self, name: &str) -> Value {
        let (status, answer) = self.app.post_op(name, Some(&self.key), "{}");
        assert_eq!(status, 200, "{name}: {answer}");
        answer["result"].clone()
    }

    /// Ask for a Job and wait it out. Both Meaning Search Jobs are minutes long
    /// on a first run, and both are watched exactly as any caller watches one:
    /// through `get_job` (ADR 0032).
    fn job(&self, name: &str) -> Value {
        let asked = self.ask(name);
        let job_id = asked["job_id"].as_str().expect("a job id").to_string();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(45 * 60);
        loop {
            let (_, answer) = self.app.post_op(
                "get_job",
                Some(&self.key),
                &json!({ "job_id": job_id }).to_string(),
            );
            let job = &answer["result"];
            match job["status"].as_str() {
                Some("completed") => return job["result"].clone(),
                Some("failed") | Some("cancelled") => panic!("{name} did not finish: {job}"),
                _ => {}
            }
            assert!(
                std::time::Instant::now() < deadline,
                "{name} never finished"
            );
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }
}

fn library_with_meaning_search() -> Option<Library> {
    let recipes = corpus()?;
    assert_eq!(
        recipes.len(),
        86,
        "the corpus is the 86-recipe export the map is measured against"
    );

    let app = support::spawn_app();
    let data_dir = app.data_dir().expect("the helper made the directory");
    let shared = shared_model_dir();
    std::fs::create_dir_all(&shared).expect("the shared model directory");
    std::os::unix::fs::symlink(&shared, data_dir.join("model"))
        .expect("link this instance's model directory at the shared one");

    let first = json!({
        "name": "Aurélien",
        "password": "a password only its person knows",
        "session_name": "test browser",
    });
    let (status, created) = app.post_auth("/auth/first-person", &first.to_string());
    assert_eq!(status, 200, "{created}");
    let operator_id = created["result"]["person"]["id"].as_str().unwrap();
    let key = app
        .core
        .mint_access_key(operator_id, "corpus meaning", false)
        .unwrap()
        .secret;
    // A Kitchen of the Operator's own, for a Kitchen-mate to be asked into:
    // the library is his Cookbook, and a Kitchen is how it is seen (ADR 0041).
    let (_, kitchen) = app.post_op("create_kitchen", Some(&key), r#"{"name":"Corpus Kitchen"}"#);
    let kitchen_id = kitchen["result"]["id"].as_str().unwrap().to_string();

    let library = Library {
        app,
        key,
        kitchen_id: kitchen_id.clone(),
    };
    for recipe in &recipes {
        let (status, made) =
            library
                .app
                .post_op("create_recipe", Some(&library.key), &recipe.to_string());
        assert_eq!(status, 200, "{} — {made}", recipe["title"]);
    }

    library.ask("accept_meaning_search_terms");
    library.job("download_meaning_model");
    let built = library.job("build_meaning_index");
    assert!(
        built["indexed"].as_i64().unwrap_or(0) >= 86,
        "the whole library was read: {built}"
    );

    Some(library)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs the personal samples/crouton export, a network, and ~220 MB of model"]
async fn meaning_finds_what_words_cannot_and_a_title_still_wins() {
    let Some(library) = library_with_meaning_search() else {
        eprintln!("skipping: no samples/crouton/*.zip on this machine");
        return;
    };

    let status = library.ask("meaning_search_status");
    assert_eq!(status["state"], json!("on"));
    assert_eq!(status["on"], json!(true));
    // Accepting is recorded with the Hand that did it and how it arrived — by
    // Access Key here, which ADR 0029 allows on purpose: an agent acts as its
    // Person, and minting it a Key was the act of authorising that.
    assert_eq!(status["accepted_via_access_key"], json!(true));
    assert!(status["accepted_by"].is_string());
    // And the offer is gone, because it has been answered.
    assert_eq!(status["offer"], json!(false));

    // **Meaning finds a recipe the words never reach.** None of these queries
    // appears anywhere in the recipe each one must find — that is the whole
    // point, and it is checked below rather than asserted here.
    // Not one of these queries appears as words anywhere in the recipe it must
    // find — a word search matches the typed phrase, and none of these phrases
    // is written down in this library. Every hit below is the model's doing.
    for (asked, wanted) in [
        (
            "something with shellfish",
            "The Best Moules Marinières (Sailor-Style Mussels)",
        ),
        ("something with shellfish", "Camarones al Ajillo"),
        (
            "a comforting bowl of noodle soup",
            "Aurélien’s Creamy Miso Shin Ramyun with Mushrooms and Cheddar",
        ),
        ("what can I make with leftover rice", "Avocado Rice"),
        ("a rich beef stew cooked in wine", "Beef Bourguignon"),
        (
            "a light summer salad",
            "SHREDDED HISPI CABBAGE & CHARRED LEMON SALAD",
        ),
        // And in the reader's own words rather than the recipe's. The library
        // is nine-in-eighty-six non-English and the model is multilingual, so
        // a French or Spanish question reaches an English recipe — which is
        // the half of a real library that an English-only search loses
        // (ADR 0006, ADR 0027).
        (
            "quelque chose avec du chocolat",
            "The Best Fudgy Brownies Recipe by Tasty",
        ),
        ("algo con pollo frito", "Korean Fried Chicken"),
    ] {
        let found = library.search(json!({ "query": asked }));
        let shown = titles(found["recipes"].as_array().unwrap());
        assert!(
            shown.iter().any(|title| title == wanted),
            "searching {asked:?} did not reach {wanted:?} — got {shown:#?}"
        );
        assert_eq!(
            found["closest"],
            json!(false),
            "{asked:?} found real matches, so they are not offered as near misses"
        );
        // And the answer stays an answer rather than most of the library:
        // the threshold is what keeps a meaning search from being a mood.
        assert!(
            shown.len() <= 20,
            "{asked:?} returned {} of 86 recipes, which is not a search: {shown:#?}",
            shown.len()
        );
    }

    // **An exact title still wins**, whatever the model thinks. Most searching
    // is navigation: you know the recipe's name and you are typing it to get
    // there (ADR 0027).
    for wanted in [
        "Chocolate Chip Cookies",
        "Katsu Curry (Japanese Curry with Chicken Cutlet)",
        "Îles Flottantes",
        "Bollo limpio",
    ] {
        let found = library.search(json!({ "query": wanted }));
        let shown = titles(found["recipes"].as_array().unwrap());
        assert_eq!(
            found["recipes"][0]["title"],
            json!(wanted),
            "an exact title must rank first: {shown:#?}"
        );
        assert_eq!(found["recipes"][0]["matched"]["where"], json!("title"));
        assert_eq!(found["recipes"][0]["matched"]["by"], json!("words"));
        assert_eq!(found["closest"], json!(false));
    }

    // **Every result quotes the line it matched on**, meaning matches included:
    // a result that cannot explain itself is noise (ADR 0027).
    let found = library.search(json!({ "query": "a rich beef stew cooked in wine" }));
    let hits = found["recipes"].as_array().unwrap();
    assert!(!hits.is_empty(), "the library is full of beef stews");
    for hit in hits {
        let matched = &hit["matched"];
        assert!(
            matched["line"]
                .as_str()
                .is_some_and(|l| !l.trim().is_empty()),
            "a result with nothing to quote: {hit}"
        );
        assert!(
            matched["by"] == json!("words") || matched["by"] == json!("meaning"),
            "a result that will not say which half found it: {hit}"
        );
    }
    assert!(
        hits.iter()
            .any(|hit| hit["matched"]["by"] == json!("meaning")),
        "a query with no literal match in the library must be reached by meaning: {:#?}",
        titles(hits)
    );

    // **Nothing found is not an empty screen.** Meaning-matching always has a
    // nearest neighbour, so "nothing found" means "nothing close enough" —
    // Kamosu says exactly that and shows the closest under that label, never
    // letting a weak match pass as a good one (ADR 0027).
    let nothing = library.search(json!({ "query": "how to fix a flat bicycle tyre" }));
    assert_eq!(
        nothing["closest"],
        json!(true),
        "with a model there is always a nearest neighbour: {nothing}"
    );
    let shown = nothing["recipes"].as_array().unwrap();
    assert!(
        !shown.is_empty() && shown.len() <= 3,
        "the closest few, not a list of results: {:#?}",
        titles(shown)
    );
    assert!(
        shown
            .iter()
            .all(|hit| hit["matched"]["by"] == json!("meaning")),
        "the closest are the model's opinion and say so: {shown:#?}"
    );

    // **The index reaches this Person's own Attempts and nobody else's.**
    //
    // Not only because a diary is a diary, but because the alternative puts a
    // flippable private flag inside a prepared index. Own-only is a fact of the
    // rows, so there is no permission check at query time and no partial
    // rebuild when somebody changes their mind (ADR 0027).
    let shelf = library.search(json!({}));
    let a_recipe = shelf["recipes"][0]["branch_id"]
        .as_str()
        .expect("a recipe to cook")
        .to_string();
    let (_, started) = library.app.post_op(
        "start_attempt",
        Some(&library.key),
        &json!({ "branch_id": a_recipe }).to_string(),
    );
    let attempt_id = started["result"]["id"].as_str().unwrap().to_string();
    let (status, finished) = library.app.post_op(
        "finish_attempt",
        Some(&library.key),
        &json!({
            "attempt_id": attempt_id,
            "note": "I let it catch on the bottom of the pan and it tasted burnt",
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{finished}");

    // A Kitchen-mate, who may read every recipe on that shelf.
    let camille = library.app.core.create_person("Camille").expect("person");
    let camille_key = library
        .app
        .core
        .mint_access_key(&camille, "camille's agent", false)
        .unwrap()
        .secret;
    let (_, invite) = library.app.post_op(
        "invite_to_kitchen",
        Some(&library.key),
        &json!({ "kitchen_id": library.kitchen_id }).to_string(),
    );
    let secret = invite["result"]["secret"].as_str().unwrap().to_string();
    let (status, joined) = library.app.post_op(
        "accept_kitchen_invite",
        Some(&camille_key),
        &json!({ "secret": secret }).to_string(),
    );
    assert_eq!(status, 200, "{joined}");

    library.job("build_meaning_index");

    let mine = library.search(json!({ "query": "the time I scorched something" }));
    let asked_for = a_recipe.clone();
    assert!(
        mine["recipes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|hit| hit["branch_id"] == json!(asked_for)),
        "my own cooking note is mine to find: {:#?}",
        titles(mine["recipes"].as_array().unwrap())
    );

    // Camille sees the same shelf and none of that diary. Her *burnt it again,
    // honestly* is a different act from her showing you.
    let hers = library.search_as(
        &camille_key,
        json!({ "query": "the time I scorched something" }),
    );
    let her_hits = hers["recipes"].as_array().unwrap();
    assert!(
        her_hits.iter().all(|hit| {
            hit["matched"]["line"].as_str()
                != Some("I let it catch on the bottom of the pan and it tasted burnt")
        }),
        "somebody else's Attempt is not in her index at all: {her_hits:#?}"
    );
    assert_eq!(
        library.search_as(&camille_key, json!({}))["recipes"]
            .as_array()
            .unwrap()
            .len(),
        86,
        "and she reads the whole shelf, which is the point of a Kitchen"
    );

    // **Turning it off discards nothing that cannot be rebuilt.** The whole
    // library is still there, still searchable by words, and the weights stay
    // on disk so turning it back on is a rebuild rather than a download.
    library.ask("turn_off_meaning_search");
    let off = library.ask("meaning_search_status");
    assert_eq!(off["state"], json!("accepted"));
    assert_eq!(off["on"], json!(false));
    assert_eq!(off["model_present"], json!(true));

    let shelf = library.search(json!({}));
    assert_eq!(
        shelf["recipes"].as_array().unwrap().len(),
        86,
        "every recipe survived turning Meaning Search off"
    );
    let words = library.search(json!({ "query": "chocolate" }));
    assert!(
        words["recipes"].as_array().unwrap().len() >= 5,
        "word search is untouched and complete on its own"
    );
    assert_eq!(
        library.search(json!({ "query": "how to fix a flat bicycle tyre" }))["closest"],
        json!(false),
        "with no model there is no nearest neighbour, and Kamosu does not invent one"
    );
}
