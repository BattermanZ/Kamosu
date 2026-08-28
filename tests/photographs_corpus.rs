//! The corpus check ADR 0017 asks for (#45): every photograph in Aurélien's
//! real 86-recipe Crouton export, uploaded through the real `upload_photograph`
//! Operation, asserting the two facts the ADR measured — no duplicate
//! photographs, and none of them a Step's photo (Crouton has no such field).
//!
//! The export lives at `samples/crouton/` and is gitignored — personal, 110 MB,
//! and never committed (see `docs/research/crouton-real-export.md`). This test
//! is `#[ignore]`d so `just test`/CI never needs it: run it deliberately with
//! `cargo test --test photographs_corpus -- --ignored` on a machine that has
//! the export.

mod support;

use base64::Engine;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::io::Read;

fn corpus_path() -> Option<std::path::PathBuf> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/crouton");
    std::fs::read_dir(&dir).ok()?.find_map(|entry| {
        let path = entry.ok()?.path();
        (path.extension().and_then(|e| e.to_str()) == Some("zip")).then_some(path)
    })
}

/// One recipe's picture, decoded from base64 — `images`, never `sourceImage`
/// (a site favicon, not a photograph: see the research doc and ADR 0017).
fn images_in(crumb: &Value) -> Vec<Vec<u8>> {
    crumb["images"]
        .as_array()
        .map(|images| {
            images
                .iter()
                .filter_map(Value::as_str)
                .filter_map(|b64| base64::engine::general_purpose::STANDARD.decode(b64).ok())
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs the personal, gitignored samples/crouton export on disk"]
async fn the_corpus_produces_zero_duplicate_photographs_and_zero_step_photos() {
    let Some(zip_path) = corpus_path() else {
        eprintln!(
            "skipping: no samples/crouton/*.zip on this machine (personal, gitignored export)"
        );
        return;
    };

    let file = std::fs::File::open(&zip_path).expect("open the corpus zip");
    let mut archive = zip::ZipArchive::new(file).expect("read the corpus zip");

    let mut recipes: Vec<(String, Vec<Vec<u8>>)> = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).expect("read entry");
        if !entry.name().ends_with(".crumb") {
            continue;
        }
        let mut text = String::new();
        entry.read_to_string(&mut text).expect("crumb is text");
        let crumb: Value = serde_json::from_str(&text).expect("crumb is JSON");
        let images = images_in(&crumb);
        if images.is_empty() {
            continue;
        }
        let name = crumb["name"].as_str().unwrap_or("untitled").to_string();
        recipes.push((name, images));
    }
    assert!(
        !recipes.is_empty(),
        "the corpus carried no photographs at all"
    );

    // The ADR's own measured fact, reproduced here as a regression guard on
    // the fixture: every photograph in this library is distinct.
    let mut raw_seen = HashSet::new();
    let mut raw_total = 0usize;
    for (_, images) in &recipes {
        for image in images {
            raw_total += 1;
            assert!(
                raw_seen.insert(image.clone()),
                "the corpus fixture itself now carries a duplicate photograph — ADR 0017 measured zero"
            );
        }
    }

    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "corpus import", false)
        .unwrap()
        .secret;
    let (_, kitchen) = app.post_op("create_kitchen", Some(&key), r#"{"name":"Corpus Kitchen"}"#);
    let kitchen_id = kitchen["result"]["id"].as_str().unwrap().to_string();

    let mut photograph_ids = HashSet::new();
    let mut branch_ids = Vec::new();
    for (name, images) in &recipes {
        let main = &images[0];
        let body = json!({ "data": base64::engine::general_purpose::STANDARD.encode(main) });
        let (status, main_uploaded) =
            app.post_op("upload_photograph", Some(&key), &body.to_string());
        assert_eq!(status, 200, "{main_uploaded}");
        let main_photo_id = main_uploaded["result"]["photograph_id"]
            .as_str()
            .unwrap()
            .to_string();
        photograph_ids.insert(main_photo_id.clone());

        // Crouton carries no per-step photo (docs/research/crouton-real-export.md);
        // every extra image in the array is a second Main-Photo-shaped picture
        // for the same recipe, so it is uploaded too but never attached to a Step.
        for extra in &images[1..] {
            let body = json!({ "data": base64::engine::general_purpose::STANDARD.encode(extra) });
            let (status, uploaded) =
                app.post_op("upload_photograph", Some(&key), &body.to_string());
            assert_eq!(status, 200, "{uploaded}");
            photograph_ids.insert(
                uploaded["result"]["photograph_id"]
                    .as_str()
                    .unwrap()
                    .to_string(),
            );
        }

        let create = json!({
            "kitchen_id": kitchen_id,
            "title": name,
            "main_photo": main_photo_id,
        });
        let (status, created) = app.post_op("create_recipe", Some(&key), &create.to_string());
        assert_eq!(status, 200, "{created}");
        branch_ids.push(created["result"]["branch_id"].as_str().unwrap().to_string());
    }

    let recorded: i64 = app
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row("SELECT COUNT(*) FROM photographs", [], |r| r.get(0))
                .map_err(|e| kamosu::OpError::internal(e.to_string()))
        })
        .expect("count Photographs");
    assert_eq!(
        recorded as usize,
        photograph_ids.len(),
        "every distinct upload must record exactly one Photograph row — no accidental merges or drops"
    );
    assert_eq!(
        photograph_ids.len(),
        raw_total,
        "zero duplicate photographs: {raw_total} distinct pictures went in, \
         {} distinct Photographs came out",
        photograph_ids.len()
    );
    eprintln!(
        "corpus check: {} recipes, {raw_total} photographs, {} distinct Photograph rows",
        recipes.len(),
        photograph_ids.len()
    );

    // Zero step photos: nothing in this corpus was ever attached to a Step.
    let mut step_photo_count = 0usize;
    for branch_id in &branch_ids {
        let (status, recipe) = app.post_op(
            "get_recipe",
            Some(&key),
            &json!({ "branch_id": branch_id }).to_string(),
        );
        assert_eq!(status, 200, "{recipe}");
        for version in recipe["result"]["versions"].as_array().unwrap() {
            for step in version["content"]["steps"].as_array().unwrap() {
                if !step["photo"].is_null() {
                    step_photo_count += 1;
                }
            }
        }
    }
    assert_eq!(
        step_photo_count, 0,
        "the Crouton corpus carries no per-step photo; this must stay zero"
    );
}
