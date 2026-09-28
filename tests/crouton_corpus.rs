//! The Crouton importer against the library it was built for (#69): all 86
//! recipes of Aurélien's real export, sent the way a browser sends them — the
//! zip staged at `POST /api/uploads`, then `import_crouton` naming it — and
//! landed in one Job.
//!
//! The corpus is the fixture. Every figure the research measured
//! (`docs/research/crouton-real-export.md`) is asserted here, both about the
//! export itself and about what the importer made of it, so a change to either
//! shows up as a number that moved.
//!
//! The export lives at `samples/crouton/` and is gitignored — personal, 110 MB.
//! Since #128 it is the 23 September 2026 export, the first to carry tags; the
//! 20 August one it replaced sits in `samples/crouton-2026-08-20/`.
//! Run deliberately: `cargo test --test crouton_corpus -- --ignored`.

mod support;

use serde_json::{Value, json};
use std::collections::HashSet;
use std::io::Read;
use std::time::{Duration, Instant};

fn corpus_path() -> Option<std::path::PathBuf> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/crouton");
    std::fs::read_dir(&dir).ok()?.find_map(|entry| {
        let path = entry.ok()?.path();
        (path.extension().and_then(|e| e.to_str()) == Some("zip")).then_some(path)
    })
}

/// Every `.crumb` in the zip, parsed — the export as Crouton wrote it, for the
/// facts about the corpus itself.
fn crumbs(zip_path: &std::path::Path) -> Vec<Value> {
    let file = std::fs::File::open(zip_path).expect("open the corpus zip");
    let mut archive = zip::ZipArchive::new(file).expect("read the corpus zip");
    (0..archive.len())
        .filter_map(|i| {
            let mut entry = archive.by_index(i).ok()?;
            if !entry.name().ends_with(".crumb") {
                return None;
            }
            let mut text = String::new();
            entry.read_to_string(&mut text).ok()?;
            serde_json::from_str(&text).ok()
        })
        .collect()
}

fn wait(app: &support::TestApp, key: &str, job_id: &str) -> Value {
    let started = Instant::now();
    let body = json!({ "job_id": job_id }).to_string();
    loop {
        let (_, answer) = app.post_op("get_job", Some(key), &body);
        let job = answer["result"].clone();
        if ["completed", "failed", "cancelled"].contains(&job["status"].as_str().unwrap_or("")) {
            return job;
        }
        assert!(
            started.elapsed() < Duration::from_secs(900),
            "the import is still running after 15 minutes: {job}"
        );
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn import(app: &support::TestApp, key: &str, zip: &[u8]) -> Value {
    let (status, staged) = app.post_bytes("/api/uploads", Some(key), "application/zip", zip);
    assert_eq!(status, 200, "{staged}");
    let (status, asked) = app.post_op(
        "import_crouton",
        Some(key),
        &json!({ "upload_id": staged["result"]["upload_id"] }).to_string(),
    );
    assert_eq!(status, 200, "{asked}");
    let job = wait(app, key, asked["result"]["job_id"].as_str().unwrap());
    assert_eq!(job["status"], json!("completed"), "{job}");
    job["result"].clone()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs the personal, gitignored samples/crouton export on disk"]
async fn the_whole_crouton_library_arrives_in_one_job_as_measured() {
    let Some(zip_path) = corpus_path() else {
        eprintln!("skipping: no samples/crouton/*.zip on this machine");
        return;
    };

    // --- The corpus, as measured -------------------------------------------
    let corpus = crumbs(&zip_path);
    assert_eq!(corpus.len(), 86);
    let rows: Vec<&Value> = corpus
        .iter()
        .flat_map(|crumb| crumb["ingredients"].as_array().unwrap())
        .collect();
    assert_eq!(rows.len(), 863);
    let unquantified = rows.iter().filter(|row| row["quantity"].is_null()).count();
    assert_eq!(unquantified, 239, "28% of 863 rows carry no quantity");
    let imperial = rows
        .iter()
        .filter(|row| {
            matches!(
                row["quantity"]["quantityType"].as_str(),
                Some("CUP" | "TABLESPOON" | "TEASPOON" | "OUNCE" | "POUND")
            )
        })
        .count();
    assert_eq!(imperial, 351, "41% of 863 rows are imperial");
    let photo_bytes: usize = corpus
        .iter()
        .flat_map(|crumb| crumb["images"].as_array().unwrap())
        .map(|b64| {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD
                .decode(b64.as_str().unwrap())
                .unwrap()
                .len()
        })
        .sum();
    assert_eq!(
        (photo_bytes + 500_000) / 1_000_000,
        82,
        "60 photographs at 82 MB"
    );
    let step_rows: usize = corpus
        .iter()
        .map(|c| c["steps"].as_array().unwrap().len())
        .sum();
    assert_eq!(step_rows, 599);

    // --- One Job ------------------------------------------------------------
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "corpus", false)
        .unwrap()
        .secret;
    let zip = std::fs::read(&zip_path).expect("read the zip");
    let report = import(&app, &key, &zip);

    let arrived = report["arrived"].as_array().unwrap();
    assert_eq!(arrived.len(), 86, "{report}");
    assert_eq!(report["unreadable"], json!([]), "{report}");
    assert!(arrived.iter().all(|row| row["status"] == json!("created")));

    // --- Photographs: 59 of 60 arrive, all distinct; the 60th is named ------
    let photos: Vec<&str> = arrived
        .iter()
        .filter_map(|row| row["main_photo"].as_str())
        .collect();
    assert_eq!(photos.len(), 59);
    assert_eq!(
        photos.iter().collect::<HashSet<_>>().len(),
        59,
        "zero duplicates"
    );
    let stored: i64 = app
        .core
        .db()
        .with_conn(|conn| {
            Ok(conn
                .query_row("SELECT COUNT(*) FROM photographs", [], |row| row.get(0))
                .unwrap())
        })
        .unwrap();
    assert_eq!(stored, 59, "no site icon and no extra photo was stored");

    let left_out = report["left_out"].as_array().unwrap();
    let icons: u64 = left_out
        .iter()
        .filter(|row| row["what"] == json!("site_icon"))
        .map(|row| row["count"].as_u64().unwrap())
        .sum();
    assert_eq!(
        icons, 44,
        "every sourceImage favicon was dropped and said so"
    );
    let extra: Vec<(&str, u64)> = left_out
        .iter()
        .filter(|row| row["what"] == json!("extra_photos"))
        .map(|row| {
            (
                row["title"].as_str().unwrap(),
                row["count"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(extra, vec![("Beef Short Ribs", 1)]);
    assert!(
        left_out
            .iter()
            .filter(|row| row["what"] == json!("site_icon"))
            .all(|row| row["icon"]
                .as_str()
                .is_some_and(|i| i.starts_with("data:image/"))),
        "every dropped favicon can be shown in the Report"
    );
    assert!(
        !left_out
            .iter()
            .any(|row| row["what"] == json!("unreadable_photos"))
    );

    // --- Bare recipes arrive as recipes --------------------------------------
    let mut bare: Vec<&str> = arrived
        .iter()
        .filter(|row| row["bare"] == json!(true))
        .map(|row| row["title"].as_str().unwrap())
        .collect();
    bare.sort();
    assert_eq!(
        bare,
        vec![
            "Chef Tyler Noodles",
            "Cozy Af French Chicken Stew - Youtube",
            "Dan Dan Noodles",
            "Gochujang Pasta"
        ]
    );

    // --- The three `-1` pairs, offered to tick and never joined -------------
    let pairs = report["related_candidates"].as_array().unwrap();
    let mut offered: Vec<&str> = pairs
        .iter()
        .map(|pair| {
            let sides = pair["recipes"].as_array().unwrap();
            assert_ne!(sides[0]["lineage_id"], sides[1]["lineage_id"]);
            // Named by the shorter title: "Pandebonos", not "(Version 2)".
            sides
                .iter()
                .map(|side| side["title"].as_str().unwrap().trim())
                .min_by_key(|title| title.len())
                .unwrap()
        })
        .collect();
    offered.sort();
    assert_eq!(
        offered,
        vec![
            "Beef Bourguignon",
            "Dan Dan Noodles",
            "Korean Fried Chicken",
            "Pandebonos"
        ],
        "{pairs:?}"
    );

    // --- What each recipe holds ----------------------------------------------
    //
    // Row for line: Crouton's rows in their own `order` are the recipe's
    // Ingredient list in Kamosu's, one for one — a SECTION row a Section, any
    // other row one rebuilt line — so each imperial row can be checked against
    // exactly the line it became.
    let by_uuid: std::collections::HashMap<&str, &Value> = corpus
        .iter()
        .map(|crumb| (crumb["uuid"].as_str().unwrap(), crumb))
        .collect();
    let mut ingredient_lines = 0;
    let mut sections = 0;
    let mut imperial_lines = 0;
    let mut imperial_read = 0;
    let mut steps = 0;
    let mut decoded = false;
    let mut tagged = 0;
    for row in arrived {
        let (status, recipe) = app.post_op(
            "get_recipe",
            Some(&key),
            &json!({ "branch_id": row["branch_id"] }).to_string(),
        );
        assert_eq!(status, 200, "{recipe}");
        let content = &recipe["result"]["versions"][0]["content"];
        assert_eq!(
            content["nutrition"],
            Value::Null,
            "Crouton's nutrition text is dropped"
        );

        let crumb = by_uuid[row["foreign_id"].as_str().unwrap()];

        // Filed under exactly the tags its `.crumb` names, by name (#128).
        let mut filed: Vec<&str> = recipe["result"]["tags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tag| tag["name"].as_str().unwrap())
            .collect();
        let mut named: Vec<&str> = crumb["tags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tag| tag["name"].as_str().unwrap())
            .collect();
        filed.sort();
        named.sort();
        assert_eq!(filed, named, "{}", row["title"]);
        tagged += usize::from(!named.is_empty());

        let mut crouton_rows: Vec<&Value> =
            crumb["ingredients"].as_array().unwrap().iter().collect();
        crouton_rows.sort_by_key(|r| r["order"].as_i64());
        let lines = content["ingredients"].as_array().unwrap();
        assert_eq!(lines.len(), crouton_rows.len(), "{}", row["title"]);
        for (crouton, line) in crouton_rows.iter().zip(lines) {
            let text = line["text"].as_str().unwrap();
            if line["kind"] == json!("section") {
                sections += 1;
                continue;
            }
            ingredient_lines += 1;
            let unit = match crouton["quantity"]["quantityType"].as_str() {
                Some("CUP") => "cup",
                Some("TABLESPOON") => "tablespoon",
                Some("TEASPOON") => "teaspoon",
                Some("OUNCE") => "ounce",
                Some("POUND") => "pound",
                _ => continue,
            };
            imperial_lines += 1;
            // The rebuilt line says its Unit, right after its amount.
            let written = text.split_whitespace().nth(1).unwrap_or("");
            assert_eq!(
                kamosu::units::recognise(written.trim_end_matches(',')).map(|u| u.id),
                Some(unit),
                "{text}"
            );
            let read = kamosu::reading::read_line(text)
                .and_then(|reading| reading.unit)
                .and_then(|unit| kamosu::units::recognise(&unit).map(|u| u.id));
            // Ranges included, since #167: `380-400 ml` reads as `ml`.
            assert_eq!(read, Some(unit), "{text}");
            imperial_read += 1;
        }
        for step in content["steps"].as_array().unwrap() {
            steps += 1;
            assert_eq!(step["photo"], Value::Null, "zero step photos");
            let text = step["text"].as_str().unwrap();
            assert!(!text.contains("&amp;"), "an entity survived: {text}");
            decoded |= text == "Noodles & choi sum:";
        }
    }
    assert_eq!(
        (ingredient_lines, sections),
        (858, 5),
        "every row became a line or a Section"
    );
    assert_eq!(
        imperial_lines, 351,
        "41% of 863 rows are imperial, and still say so"
    );
    assert_eq!(
        imperial_read, 351,
        "every imperial line, its 3 ranges too, reads back as the Unit it was rebuilt from"
    );
    assert_eq!(steps, 599);
    assert!(decoded, "Dan Dan Noodles' section arrived decoded");
    assert_eq!(tagged, 64, "64 of the 86 recipes are tagged");

    // --- The Kitchen's tag list: 21 English words, no colour -----------------
    let cookbook_tags = |app: &support::TestApp| -> Vec<Value> {
        let (status, listed) = app.post_op("list_tags", Some(&key), &json!({}).to_string());
        assert_eq!(status, 200, "{listed}");
        listed["result"]["tags"].as_array().unwrap().clone()
    };
    let tags = cookbook_tags(&app);
    let mut by_count: Vec<(String, u64)> = tags
        .iter()
        .map(|tag| {
            assert_eq!(tag["language"], json!("en"), "{tag}");
            (
                tag["name"].as_str().unwrap().to_string(),
                tag["recipes"].as_u64().unwrap(),
            )
        })
        .collect();
    by_count.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let expected = [
        ("Hearty", 36),
        ("Vegetarian", 33),
        ("Asian", 22),
        ("Meat", 18),
        ("Dessert", 13),
        ("French", 9),
        ("Japanese", 9),
        ("Beef", 8),
        ("Chicken", 8),
        ("American", 7),
        ("Weeknight dinner", 7),
        ("Vegan", 6),
        ("Chocolate", 5),
        ("Fish", 5),
        ("Stew", 4),
        ("Salads", 3),
        ("South American", 3),
        ("Italian", 2),
        ("Bread", 1),
        ("Colombian", 1),
        ("Middle Eastern", 1),
    ]
    .map(|(name, count)| (name.to_string(), count));
    assert_eq!(
        by_count, expected,
        "21 tags, each on the recipes Crouton put it on"
    );

    // --- Again, through the ledger --------------------------------------------
    let again = import(&app, &key, &zip);
    assert_eq!(again["import_id"], report["import_id"]);
    let unchanged = again["arrived"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["status"] == json!("unchanged"))
        .count();
    assert_eq!(
        unchanged, 86,
        "a re-run matched rather than doubling the library"
    );
    assert_eq!(cookbook_tags(&app).len(), 21, "no new Tag");
    let filings: i64 = app
        .core
        .db()
        .with_conn(|conn| {
            Ok(conn
                .query_row("SELECT COUNT(*) FROM branch_tags", [], |row| row.get(0))
                .unwrap())
        })
        .unwrap();
    assert_eq!(
        filings,
        expected.iter().map(|(_, count)| *count as i64).sum::<i64>(),
        "no recipe filed twice"
    );

    // --- And the ledger, deleted whole ----------------------------------------
    let (status, forgotten) = app.post_op(
        "forget_import",
        Some(&key),
        &json!({ "import_id": report["import_id"] }).to_string(),
    );
    assert_eq!(status, 200, "{forgotten}");
    assert_eq!(forgotten["result"]["forgotten"], json!(86));
}
