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
