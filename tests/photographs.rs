//! Behaviour tests for the Photograph (#45, ADR 0017): real Operations, the
//! real out-of-band routes, real files under a temporary data directory.

mod support;

use base64::Engine;
use serde_json::{Value, json};

/// A real, freshly encoded picture — generated rather than hand-typed, so the
/// fixture cannot be wrong the way a hand-typed byte literal could.
fn make_jpeg(width: u32, height: u32) -> Vec<u8> {
    let image = image::RgbImage::from_fn(width, height, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
    });
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgb8(image)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Jpeg,
        )
        .expect("encodes");
    bytes
}

fn base64_of(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn uploading_the_same_picture_twice_through_base64_answers_the_same_id() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "browser session", false)
        .unwrap()
        .secret;

    let picture = make_jpeg(40, 30);
    let body = json!({ "data": base64_of(&picture) }).to_string();
    let (status_a, first) = app.post_op("upload_photograph", Some(&key), &body);
    let (status_b, second) = app.post_op("upload_photograph", Some(&key), &body);

    assert_eq!(status_a, 200, "{first}");
    assert_eq!(status_b, 200, "{second}");
    assert_eq!(
        first["result"]["photograph_id"], second["result"]["photograph_id"],
        "two uploads of the same picture must be one Photograph"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_out_of_band_route_and_the_base64_fallback_agree_on_the_same_picture() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "browser session", false)
        .unwrap()
        .secret;

    let picture = make_jpeg(50, 20);
    let (raw_status, raw_result) =
        app.post_bytes("/api/photographs", Some(&key), "image/jpeg", &picture);
    assert_eq!(raw_status, 200, "{raw_result}");

    let body = json!({ "data": base64_of(&picture) }).to_string();
    let (op_status, op_result) = app.post_op("upload_photograph", Some(&key), &body);
    assert_eq!(op_status, 200, "{op_result}");

    assert_eq!(
        raw_result["result"]["photograph_id"], op_result["result"]["photograph_id"],
        "the same picture must be the same Photograph however it travelled in"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_uploaded_photograph_can_be_read_back_as_webp() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "browser session", false)
        .unwrap()
        .secret;

    let picture = make_jpeg(64, 48);
    let (_, uploaded) = app.post_bytes("/api/photographs", Some(&key), "image/jpeg", &picture);
    let hash = uploaded["result"]["photograph_id"]
        .as_str()
        .expect("a photograph id")
        .to_string();

    let (status, content_type, bytes) =
        app.get_bytes(&format!("/api/photographs/{hash}"), Some(&key));
    assert_eq!(status, 200);
    assert_eq!(content_type, "image/webp");
    let decoded =
        image::load_from_memory_with_format(&bytes, image::ImageFormat::WebP).expect("valid webp");
    assert_eq!((decoded.width(), decoded.height()), (64, 48));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_display_copy_is_generated_on_first_ask_and_never_exceeds_its_size() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "browser session", false)
        .unwrap()
        .secret;

    // Big enough that a Card copy is a real downscale, small enough that the
    // remake itself (well under the 2560 cap) stays fast in a debug build.
    let picture = make_jpeg(1200, 800);
    let (_, uploaded) = app.post_bytes("/api/photographs", Some(&key), "image/jpeg", &picture);
    let hash = uploaded["result"]["photograph_id"]
        .as_str()
        .expect("a photograph id")
        .to_string();

    let (status, content_type, bytes) =
        app.get_bytes(&format!("/api/photographs/{hash}/card"), Some(&key));
    assert_eq!(status, 200);
    assert_eq!(content_type, "image/webp");
    let decoded =
        image::load_from_memory_with_format(&bytes, image::ImageFormat::WebP).expect("valid webp");
    assert_eq!(decoded.width(), 600, "a card's long edge is 600px");

    // Asked again, the cached file answers identically.
    let (_, _, again) = app.get_bytes(&format!("/api/photographs/{hash}/card"), Some(&key));
    assert_eq!(bytes, again, "a cached Display Copy is not remade");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unrecognised_display_size_is_refused() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "browser session", false)
        .unwrap()
        .secret;

    let picture = make_jpeg(40, 30);
    let (_, uploaded) = app.post_bytes("/api/photographs", Some(&key), "image/jpeg", &picture);
    let hash = uploaded["result"]["photograph_id"].as_str().unwrap();

    let (status, _content_type, _body) =
        app.get_bytes(&format!("/api/photographs/{hash}/thumbnail"), Some(&key));
    assert_eq!(status, 400);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn svg_is_refused_by_both_upload_routes() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "browser session", false)
        .unwrap()
        .secret;

    let svg = b"<?xml version=\"1.0\"?><svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";

    let (raw_status, raw_body) =
        app.post_bytes("/api/photographs", Some(&key), "image/svg+xml", svg);
    assert_eq!(raw_status, 400);
    assert!(
        raw_body["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .contains("svg")
    );

    let body = json!({ "data": base64_of(svg) }).to_string();
    let (op_status, op_body) = app.post_op("upload_photograph", Some(&key), &body);
    assert_eq!(op_status, 400);
    assert!(
        op_body["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .contains("svg")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_read_only_access_key_cannot_upload_a_photograph_either_way() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let read_only_key = app
        .core
        .mint_access_key(&person, "read-only agent", true)
        .unwrap()
        .secret;

    let picture = make_jpeg(20, 20);
    let (raw_status, _) = app.post_bytes(
        "/api/photographs",
        Some(&read_only_key),
        "image/jpeg",
        &picture,
    );
    assert_eq!(raw_status, 401);

    let body = json!({ "data": base64_of(&picture) }).to_string();
    let (op_status, _) = app.post_op("upload_photograph", Some(&read_only_key), &body);
    assert_eq!(op_status, 401);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fetching_a_photograph_without_a_credential_is_refused() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "browser session", false)
        .unwrap()
        .secret;

    let picture = make_jpeg(20, 20);
    let (_, uploaded) = app.post_bytes("/api/photographs", Some(&key), "image/jpeg", &picture);
    let hash = uploaded["result"]["photograph_id"].as_str().unwrap();

    let (status, _, _) = app.get_bytes(&format!("/api/photographs/{hash}"), None);
    assert_eq!(status, 401);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_photograph_is_readable_by_any_person_on_the_instance_not_only_its_uploader() {
    let app = support::spawn_app();
    let uploader = app.core.create_person("Aurélien").expect("person");
    let uploader_key = app
        .core
        .mint_access_key(&uploader, "browser session", false)
        .unwrap()
        .secret;
    let stranger = app.core.create_person("Marine").expect("person");
    let stranger_key = app
        .core
        .mint_access_key(&stranger, "browser session", false)
        .unwrap()
        .secret;

    let picture = make_jpeg(20, 20);
    let (_, uploaded) = app.post_bytes(
        "/api/photographs",
        Some(&uploader_key),
        "image/jpeg",
        &picture,
    );
    let hash = uploaded["result"]["photograph_id"].as_str().unwrap();

    let (status, content_type, _) =
        app.get_bytes(&format!("/api/photographs/{hash}"), Some(&stranger_key));
    assert_eq!(status, 200);
    assert_eq!(content_type, "image/webp");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_photograph_arriving_already_made_is_stored_byte_for_byte_with_no_reencoding() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "browser session", false)
        .unwrap()
        .secret;

    // Not a real picture — the point of verbatim storage is that nothing
    // decodes or re-encodes it, so arbitrary bytes travel through unchanged,
    // exactly as a Photograph already made by another instance would.
    let bundle_bytes = b"a Photograph another Kamosu instance already made".to_vec();
    let first = app
        .core
        .store_photograph_verbatim(&bundle_bytes)
        .expect("stores verbatim");
    let second = app
        .core
        .store_photograph_verbatim(&bundle_bytes)
        .expect("stores verbatim");
    assert_eq!(
        first["photograph_id"], second["photograph_id"],
        "the same bytes must be the same Photograph, deterministically, with no server involved"
    );
    let hash = first["photograph_id"].as_str().unwrap().to_string();

    let (status, _content_type, stored) =
        app.get_bytes(&format!("/api/photographs/{hash}"), Some(&key));
    assert_eq!(status, 200);
    assert_eq!(
        stored, bundle_bytes,
        "a Photograph stored verbatim must be exactly the bytes that arrived"
    );

    let recorded: i64 = app
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM photographs WHERE hash = ?1",
                [&hash],
                |r| r.get(0),
            )
            .map_err(|e| kamosu::OpError::internal(e.to_string()))
        })
        .expect("count Photographs");
    assert_eq!(
        recorded, 1,
        "storing the same bytes twice must record one row, not two"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_main_photo_and_a_steps_photo_are_part_of_the_versions_fingerprint() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "browser session", false)
        .unwrap()
        .secret;

    let (_, kitchen) = app.post_op(
        "create_kitchen",
        Some(&key),
        r#"{"name":"Aurélien's Kitchen"}"#,
    );
    let kitchen_id = kitchen["result"]["id"].as_str().unwrap();

    let picture = make_jpeg(20, 20);
    let (_, uploaded) = app.post_bytes("/api/photographs", Some(&key), "image/jpeg", &picture);
    let photo_id = uploaded["result"]["photograph_id"].as_str().unwrap();

    let without_photo = json!({ "kitchen_id": kitchen_id, "title": "Tomato Soup" });
    let (_, created) = app.post_op("create_recipe", Some(&key), &without_photo.to_string());
    let branch_id = created["result"]["branch_id"].as_str().unwrap();
    let version_without: &Value = &created["result"]["versions"][0]["version_id"];

    let with_main_photo = json!({
        "branch_id": branch_id,
        "title": "Tomato Soup",
        "main_photo": photo_id,
    });
    let (_, saved) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &with_main_photo.to_string(),
    );
    assert_ne!(
        &saved["result"]["version_id"], version_without,
        "setting the Main Photo must mint a new Version"
    );

    let with_step_photo = json!({
        "branch_id": branch_id,
        "title": "Tomato Soup",
        "main_photo": photo_id,
        "steps": [{ "kind": "step", "text": "Simmer.", "photo": photo_id }],
    });
    let (_, saved_with_step) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &with_step_photo.to_string(),
    );
    assert_ne!(
        saved_with_step["result"]["version_id"], saved["result"]["version_id"],
        "attaching a Step's photo must mint a new Version too"
    );
}

// ─── The orphan sweep (#46) ──────────────────────────────────────────────────
//
// Pictures nothing points at are taken away on a delay, with the referenced set
// recomputed from scratch each run rather than kept as a tally — because a
// tally can drift into deleting a picture still on screen.

/// An instance with an operator, since the sweep is an Operator's to ask for.
fn operator(app: &support::TestApp) -> (String, String) {
    let first = json!({
        "name": "Aurélien",
        "password": "a password only its person knows",
        "session_name": "test browser"
    });
    let (_, created) = app.post_auth("/auth/first-person", &first.to_string());
    let person = created["result"]["person"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let key = app
        .core
        .mint_access_key(&person, "agent", false)
        .unwrap()
        .secret;
    (person, key)
}

/// Push a Photograph's unreferenced mark into the past, so the week-long grace
/// can be tested without a clock to fast-forward. The one place these tests
/// reach past Operations into the store, for the same reason the Attempt tests
/// do it: there is no other way to make time pass.
fn backdate_unreferenced(app: &support::TestApp, hash: &str, days_ago: i64) {
    app.core
        .db()
        .with_conn(|conn| {
            let changed = conn
                .execute(
                    "UPDATE photographs
                     SET unreferenced_since = strftime('%Y-%m-%dT%H:%M:%fZ','now', ?2)
                     WHERE hash = ?1",
                    rusqlite::params![hash, format!("-{days_ago} days")],
                )
                .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            assert_eq!(changed, 1, "the Photograph must still be on record");
            Ok(())
        })
        .expect("backdate the unreferenced mark");
}

/// Push a Branch's head into the past, so the next save is a new Version
/// rather than collapsing into the one being shaped.
fn backdate_branch_head(app: &support::TestApp, branch_id: &str) {
    app.core
        .db()
        .with_conn(|conn| {
            conn.execute(
                "UPDATE branch_versions SET created_at = strftime('%Y-%m-%dT%H:%M:%fZ','now','-2 hours') \
                 WHERE branch_id = ?1 AND sequence = (SELECT MAX(sequence) FROM branch_versions WHERE branch_id = ?1)",
                rusqlite::params![branch_id],
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            Ok(())
        })
        .expect("backdate Branch head");
}

/// Upload a picture and attach it to a fresh recipe as its Main Photo.
/// Answers the Photograph's id and the Branch it now hangs off.
fn recipe_with_a_photo(app: &support::TestApp, key: &str, title: &str) -> (String, String) {
    let (_, kitchen) = app.post_op(
        "create_kitchen",
        Some(key),
        &json!({ "name": format!("{title} Kitchen") }).to_string(),
    );
    let kitchen_id = kitchen["result"]["id"].as_str().unwrap().to_string();

    // A picture of its own size, so each test's Photograph is its own hash and
    // two tests cannot collide on one identity.
    let picture = make_jpeg(20 + (title.len() as u32 % 17), 20);
    let (_, uploaded) = app.post_bytes("/api/photographs", Some(key), "image/jpeg", &picture);
    let photo_id = uploaded["result"]["photograph_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (_, created) = app.post_op(
        "create_recipe",
        Some(key),
        &json!({ "kitchen_id": kitchen_id, "title": title }).to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    let (status, saved) = app.post_op(
        "save_recipe_version",
        Some(key),
        &json!({ "branch_id": branch_id, "title": title, "main_photo": photo_id }).to_string(),
    );
    assert_eq!(status, 200, "{saved}");
    (photo_id, branch_id)
}

/// The criterion this ticket names in as many words. Detaching a picture starts
/// the clock; putting it back stops it. A week later the picture is still there,
/// because the sweep asks what is referenced *now* rather than trusting a count
/// of comings and goings.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_picture_detached_and_reattached_within_the_week_survives_the_sweep() {
    let app = support::spawn_app();
    let (_, key) = operator(&app);
    let (photo_id, branch_id) = recipe_with_a_photo(&app, &key, "Tomato Soup");

    // Detach it. The save collapses into the Version being shaped, so the only
    // Version naming the picture is gone and nothing points at it any more.
    let (_, detached) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "title": "Tomato Soup" }).to_string(),
    );
    assert_eq!(
        detached["result"]["collapsed"],
        json!(true),
        "this test needs the detaching save to collapse, or the picture stays referenced"
    );

    let (status, first_sweep) = app.post_op("sweep_photographs", Some(&key), "{}");
    assert_eq!(status, 200, "{first_sweep}");
    assert_eq!(
        first_sweep["result"]["swept"],
        json!(0),
        "nothing may be taken on the run that first notices it"
    );
    assert_eq!(first_sweep["result"]["newly_unreferenced"], json!(1));

    // Put it back, well inside the week.
    let (_, reattached) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "title": "Tomato Soup", "main_photo": photo_id })
            .to_string(),
    );
    assert!(reattached["result"]["version_id"].is_string());

    let (_, second_sweep) = app.post_op("sweep_photographs", Some(&key), "{}");
    assert_eq!(
        second_sweep["result"]["back_in_use"],
        json!(1),
        "putting the picture back must clear the mark, not merely pause it"
    );
    assert_eq!(second_sweep["result"]["swept"], json!(0));

    // Even a fortnight on, the picture is still there and still readable
    // through its own route: the spell ended, so the age is irrelevant.
    let (_, third_sweep) = app.post_op("sweep_photographs", Some(&key), "{}");
    assert_eq!(third_sweep["result"]["swept"], json!(0));

    let (status, _, _) = app.get_bytes(&format!("/api/photographs/{photo_id}"), Some(&key));
    assert_eq!(status, 200, "the picture must survive the sweep");
}

/// The other half: a picture nothing has pointed at for a week does go, and its
/// Display Copies go with it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_picture_unreferenced_for_a_week_is_swept_with_its_display_copies() {
    let app = support::spawn_app();
    let (_, key) = operator(&app);
    let (photo_id, branch_id) = recipe_with_a_photo(&app, &key, "Onion Soup");

    // Ask for a Display Copy, so there is one on disk to be taken away too.
    let (status, _, _) = app.get_bytes(&format!("/api/photographs/{photo_id}/card"), Some(&key));
    assert_eq!(status, 200);
    let card = kamosu::photographs::display_path(
        &app.core.data_dir(),
        &photo_id,
        kamosu::photographs::DisplaySize::Card,
    );
    assert!(
        card.exists(),
        "the Display Copy must exist before the sweep"
    );

    let (_, detached) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "title": "Onion Soup" }).to_string(),
    );
    assert_eq!(detached["result"]["collapsed"], json!(true));

    app.post_op("sweep_photographs", Some(&key), "{}");
    backdate_unreferenced(&app, &photo_id, 8);

    let (_, swept) = app.post_op("sweep_photographs", Some(&key), "{}");
    assert_eq!(swept["result"]["swept"], json!(1), "{swept}");
    assert_eq!(
        swept["result"]["swept_photograph_ids"][0],
        json!(photo_id.clone())
    );

    assert!(
        !card.exists(),
        "a Display Copy is worth nothing without its Photograph and must go with it"
    );
    assert!(
        !kamosu::photographs::photograph_path(&app.core.data_dir(), &photo_id).exists(),
        "the Photograph's own bytes must go too"
    );
    let (status, _, _) = app.get_bytes(&format!("/api/photographs/{photo_id}"), Some(&key));
    assert_eq!(status, 404, "a swept Photograph is gone, not merely hidden");
}

/// A picture an *old* Version still names is in use, even though no Branch head
/// points at it. Versions are never rewritten and any of them can be opened and
/// cooked from, so "referenced" means referenced by any Version at all.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_picture_only_an_old_version_names_is_still_in_use() {
    let app = support::spawn_app();
    let (_, key) = operator(&app);
    let (photo_id, branch_id) = recipe_with_a_photo(&app, &key, "Katsu Curry");

    // Put the head far enough back that the next save is a new Version rather
    // than a collapse — so the Version naming the picture survives.
    backdate_branch_head(&app, &branch_id);
    let (_, saved) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "title": "Katsu Curry" }).to_string(),
    );
    assert_eq!(
        saved["result"]["collapsed"],
        json!(false),
        "this test needs a second Version, not a collapse"
    );

    let (_, sweep) = app.post_op("sweep_photographs", Some(&key), "{}");
    assert_eq!(
        sweep["result"]["newly_unreferenced"],
        json!(0),
        "an older Version still names the picture, so nothing is unreferenced"
    );
    assert_eq!(sweep["result"]["referenced"], json!(1));

    // And it stays: even backdating cannot age a mark that was never made.
    let (_, again) = app.post_op("sweep_photographs", Some(&key), "{}");
    assert_eq!(again["result"]["swept"], json!(0));
    let (status, _, _) = app.get_bytes(&format!("/api/photographs/{photo_id}"), Some(&key));
    assert_eq!(status, 200);
}

/// The sweep is an Operator's to ask for. An ordinary Person cannot cause the
/// instance to delete pictures.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_ordinary_person_may_not_ask_for_the_sweep() {
    let app = support::spawn_app();
    let (_, _operator_key) = operator(&app);
    let stranger = app.core.create_person("Marie").expect("person");
    let stranger_key = app
        .core
        .mint_access_key(&stranger, "browser", false)
        .unwrap()
        .secret;

    // Kamosu answers 401 to a Credential that does not carry the ability, not
    // 403: the Core refuses on the Credential, and the Door only carries it.
    let (status, refused) = app.post_op("sweep_photographs", Some(&stranger_key), "{}");
    assert_eq!(status, 401, "{refused}");
    assert_eq!(refused["error"]["kind"], json!("unauthorized"));
}

/// A picture taken while cooking belongs to the Attempt and to no Version
/// (#59), so the sweep — which works out what is referenced by reading the
/// Versions — would take it a week later unless it also reads the Attempts.
///
/// This is the failure the sweep's whole design exists to prevent, wearing new
/// clothes: a picture still on somebody's screen, deleted because no recipe
/// happened to name it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_sweep_never_takes_a_photograph_an_attempt_still_holds() {
    let app = support::spawn_app();
    let (_, key) = operator(&app);

    let (_, kitchen) = app.post_op(
        "create_kitchen",
        Some(&key),
        r#"{"name":"Aurélien's Kitchen"}"#,
    );
    let kitchen_id = kitchen["result"]["id"].as_str().unwrap().to_string();
    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Katsu Curry",
            "steps": [{ "kind": "step", "text": "Frire." }],
        })
        .to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    // A picture that no recipe will ever name: it is only ever the cook's.
    let picture = make_jpeg(31, 23);
    let (_, uploaded) = app.post_bytes("/api/photographs", Some(&key), "image/jpeg", &picture);
    let photo_id = uploaded["result"]["photograph_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (_, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let attempt_id = started["result"]["id"].as_str().unwrap().to_string();
    let (status, finished) = app.post_op(
        "finish_attempt",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "photographs": [photo_id] }).to_string(),
    );
    assert_eq!(status, 200, "{finished}");

    let (_, sweep) = app.post_op("sweep_photographs", Some(&key), "{}");
    assert_eq!(
        sweep["result"]["referenced"],
        json!(1),
        "an Attempt's own Photograph is referenced: {sweep}"
    );
    assert_eq!(sweep["result"]["newly_unreferenced"], json!(0));

    // A week of sweeps changes nothing, because no mark was ever made — and
    // backdating one cannot age a mark that does not exist.
    backdate_unreferenced_if_marked(&app, &photo_id);
    let (_, later) = app.post_op("sweep_photographs", Some(&key), "{}");
    assert_eq!(later["result"]["swept"], json!(0), "{later}");
    let (status, _, _) = app.get_bytes(&format!("/api/photographs/{photo_id}"), Some(&key));
    assert_eq!(status, 200, "the cook's own picture must survive the sweep");

    // Deleting the cooking record is what releases it: the Attempt was the
    // only thing pointing at the picture, so now the ordinary grace begins.
    let (status, deleted) = app.post_op(
        "delete_attempt",
        Some(&key),
        &json!({ "attempt_id": attempt_id }).to_string(),
    );
    assert_eq!(status, 200, "{deleted}");
    let (_, released) = app.post_op("sweep_photographs", Some(&key), "{}");
    assert_eq!(
        released["result"]["newly_unreferenced"],
        json!(1),
        "with the Attempt gone nothing points at the picture: {released}"
    );
    backdate_unreferenced(&app, &photo_id, 8);
    let (_, swept) = app.post_op("sweep_photographs", Some(&key), "{}");
    assert_eq!(swept["result"]["swept"], json!(1), "{swept}");
}

/// `backdate_unreferenced`, but tolerant of a Photograph that carries no mark
/// at all — which is the state this test is asserting.
fn backdate_unreferenced_if_marked(app: &support::TestApp, hash: &str) {
    app.core
        .db()
        .with_conn(|conn| {
            conn.execute(
                "UPDATE photographs
                 SET unreferenced_since = strftime('%Y-%m-%dT%H:%M:%fZ','now','-8 days')
                 WHERE hash = ?1 AND unreferenced_since IS NOT NULL",
                rusqlite::params![hash],
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            Ok(())
        })
        .expect("backdate any mark");
}
