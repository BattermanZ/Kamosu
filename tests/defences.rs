//! What Kamosu defends, asserted in one place (#79, ADR 0031–0034).
//!
//! Every guarantee here cuts across the whole program rather than belonging to
//! one feature, which is why none of them could be written until the features
//! existed. They are the properties an Operator is promised, checked against
//! the running instance through its real Doors.
//!
//! The companion to this file is prose: `SECURITY.md` says what Kamosu does
//! *not* defend, and that list is maintained alongside these tests. An entry
//! there that stops being true is worse than one that never existed
//! (ADR 0034).

mod support;

use serde_json::{Value, json};

/// The stranger-lane test drives `probe_job`, so it exists only under
/// `test-jobs`. Without the feature this failure stands in for it, as the one
/// in `behaviour.rs` does for that suite (#123): a defence compiled out in
/// silence reads the same as a defence that holds.
#[cfg(not(feature = "test-jobs"))]
#[test]
fn the_defences_need_the_test_jobs_feature() {
    panic!(
        "\n\n    The stranger-lane defence needs `--features test-jobs`: run `just test`, \
         or `cargo test --features test-jobs`.\n\n"
    );
}

/// The first Person on a fresh instance, who is its Operator, and an Access
/// Key that acts as them.
fn operator(app: &support::TestApp) -> (String, String) {
    let (status, created) = app.post_auth(
        "/auth/first-person",
        &json!({
            "name": "Aurélien",
            "password": "a password only its person knows",
            "session_name": "first browser",
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{created}");
    let person = created["result"]["person"]["id"]
        .as_str()
        .expect("the Operator's id")
        .to_string();
    let key = app
        .core
        .mint_access_key(&person, "agent", false)
        .expect("an Access Key")
        .secret;
    (person, key)
}

/// Every value stored anywhere in the database, as text. Used to ask the one
/// question that matters about a Secret at rest: whether the instance kept a
/// copy of it.
fn everything_stored(app: &support::TestApp) -> String {
    app.core
        .db()
        .with_conn(|conn| {
            let tables: Vec<String> = conn
                .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
                .and_then(|mut q| {
                    q.query_map([], |row| row.get::<_, String>(0))?
                        .collect::<Result<Vec<_>, _>>()
                })
                .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;

            let mut everything = String::new();
            for table in tables {
                let mut q = conn
                    .prepare(&format!("SELECT * FROM \"{table}\""))
                    .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
                let columns = q.column_count();
                let mut rows = q
                    .query([])
                    .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
                while let Some(row) = rows
                    .next()
                    .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?
                {
                    for index in 0..columns {
                        if let Ok(value) = row.get::<_, rusqlite::types::Value>(index) {
                            everything.push_str(&format!("{value:?}"));
                            everything.push('\n');
                        }
                    }
                }
            }
            Ok(everything)
        })
        .expect("read the whole database")
}

/// Every table that carries a Secret, found by looking rather than by being
/// told. A sixth Secret added later is swept into these assertions without
/// anyone remembering to add it here.
fn tables_holding_a_secret(app: &support::TestApp) -> Vec<String> {
    app.core
        .db()
        .with_conn(|conn| {
            let tables: Vec<String> = conn
                .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
                .and_then(|mut q| {
                    q.query_map([], |row| row.get::<_, String>(0))?
                        .collect::<Result<Vec<_>, _>>()
                })
                .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            let mut carrying = Vec::new();
            for table in tables {
                let columns: Vec<String> = conn
                    .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
                    .and_then(|mut q| {
                        q.query_map([], |row| row.get::<_, String>(0))?
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
                if columns.iter().any(|column| column == "secret_hash") {
                    carrying.push(table);
                }
            }
            Ok(carrying)
        })
        .expect("find the tables carrying a Secret")
}

fn columns_of(app: &support::TestApp, table: &str) -> Vec<String> {
    app.core
        .db()
        .with_conn(|conn| {
            conn.prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
                .and_then(|mut q| {
                    q.query_map([], |row| row.get::<_, String>(0))?
                        .collect::<Result<Vec<_>, _>>()
                })
                .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .expect("read a table's columns")
}

// --- A header is never authority (ADR 0033) ----------------------------------

/// The headers a reverse proxy conventionally sets, and which a visitor can
/// therefore write for themselves. Kamosu publishes one plain HTTP port and
/// assumes the proxy did nothing but carry the bytes, so not one of these may
/// stand in for a Credential or widen one.
const FORGEABLE: &[(&str, &str)] = &[
    ("X-Forwarded-For", "127.0.0.1"),
    ("X-Real-IP", "127.0.0.1"),
    ("X-Forwarded-Proto", "https"),
    ("X-Forwarded-Host", "kamosu.example"),
    ("X-Forwarded-User", "Aurélien"),
    ("X-Authenticated-User", "Aurélien"),
    ("X-Remote-User", "Aurélien"),
    ("X-Auth-Request-User", "Aurélien"),
    ("X-Forwarded-Access-Token", "anything at all"),
    ("Remote-User", "Aurélien"),
    ("Forwarded", "for=127.0.0.1;proto=https"),
];

/// The test ADR 0033 sets itself: Kamosu on its own port with no proxy in
/// front is exactly as safe as Kamosu behind one, minus encryption in
/// transit. Nothing a request carries, other than the Secret itself, is read
/// as authority to do anything.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_request_header_is_read_as_authority() {
    let app = support::spawn_app();
    let (person, key) = operator(&app);

    // An Operation needing a Person, asked with every forgeable header at once
    // and no Credential. Sent together rather than one at a time because a
    // header that were trusted would be trusted alongside the others too.
    let all_at_once: Vec<(&str, &str)> = FORGEABLE.to_vec();
    let (status, refused) = app.post_op_with_headers("list_jobs", &all_at_once, "{}");
    assert_eq!(
        status, 401,
        "a pile of proxy headers is not a Credential: {refused}"
    );

    // And one at a time, so a single trusted header cannot hide behind the
    // others being ignored. The Person's own id and name are used as the
    // values, which is the most a forged header could ever hope to assert.
    for (field, _) in FORGEABLE {
        for value in [person.as_str(), "Aurélien"] {
            let (status, refused) = app.post_op_with_headers("list_jobs", &[(field, value)], "{}");
            assert_eq!(
                status, 401,
                "{field}: {value} must not authorise anything: {refused}"
            );
        }
    }

    // The same headers must not widen a Credential that *is* real. A read-only
    // Access Key stays read-only however the request is dressed up.
    let read_only = app
        .core
        .mint_access_key(&person, "a read-only agent", true)
        .expect("a read-only Access Key")
        .secret;
    let with_key: Vec<(&str, &str)> = FORGEABLE
        .iter()
        .copied()
        .chain([("Authorization", "Bearer placeholder")])
        .collect();
    let mut dressed_up = with_key.clone();
    let bearer = format!("Bearer {read_only}");
    dressed_up.last_mut().expect("the Authorization header").1 = bearer.as_str();
    let (status, refused) = app.post_op_with_headers(
        "create_kitchen",
        &dressed_up,
        &json!({ "name": "A Kitchen a read-only Key may not create" }).to_string(),
    );
    assert_eq!(
        status, 401,
        "a read-only Key stays read-only whatever headers arrive: {refused}"
    );
    assert!(
        refused["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("read-only"),
        "and is refused for being read-only, not for the headers: {refused}"
    );

    // A real Credential still works, so the test above is not passing because
    // the Operation is simply broken.
    assert_eq!(app.post_op("list_jobs", Some(&key), "{}").0, 200);

    // And the same at the other Door. Both are built by walking one Catalogue
    // (ADR 0001) and `tests/parity.rs` exists because they can still drift, so
    // the header rule is asked of the MCP door in its own words rather than
    // inferred from the web door's answer.
    let call = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": { "name": "list_jobs", "arguments": {} },
    })
    .to_string();
    // The positive control first: the same call, under a real Key, succeeds.
    // Without this the loop below would pass just as happily against a Door
    // that refused everything.
    let (_, allowed) = app.post_mcp(&call, Some(&key));
    assert!(
        !allowed["error"].is_object() && !allowed["result"]["isError"].as_bool().unwrap_or(false),
        "a real Key must work at the MCP door, or the loop below proves nothing: {allowed}"
    );

    for (field, value) in FORGEABLE {
        let (_, answered) = app.post_mcp_with_headers(&call, &[(field, value)]);
        let refused = answered["error"].is_object()
            || answered["result"]["isError"].as_bool().unwrap_or(false);
        assert!(
            refused,
            "{field}: {value} must not authorise anything at the MCP door: {answered}"
        );
    }
}

// --- Every Secret is the same shape (ADR 0031) -------------------------------

/// One Secret as some Operation handed it out, and how to end it.
struct Secret {
    /// What it is called in the glossary, for a failure message that names the
    /// thing rather than a row.
    what: &'static str,
    /// The raw Secret, exactly once, as its caller received it.
    raw: String,
    /// The table it is recorded in, so the sweep below can account for every
    /// Secret-bearing table by the end.
    table: &'static str,
}

/// Mint one of every Secret Kamosu has, each through a real Door.
///
/// Minting a Secret that lets an agent act as you is deliberately closed to
/// agents: `mint_access_key`, `mint_invite` and `mint_recovery_link` are all
/// `session_only`, so a leaked Key cannot be turned into an account
/// (ADR 0031). That is why the Session logged in below does most of the work
/// here rather than the Access Key.
fn every_secret(app: &support::TestApp, key: &str) -> Vec<Secret> {
    let mut secrets = Vec::new();

    // A Session, from logging in.
    let logged_in = app.post_auth_response(
        "/auth/login",
        &json!({
            "name": "Aurélien",
            "password": "a password only its person knows",
            "session_name": "laptop",
        })
        .to_string(),
    );
    assert_eq!(logged_in.status, 200, "{}", logged_in.text());
    let session = logged_in
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("set-cookie"))
        .and_then(|(_, value)| value.split(';').next())
        .and_then(|pair| pair.strip_prefix("kamosu_session="))
        .expect("an HttpOnly Session cookie")
        .to_string();
    secrets.push(Secret {
        what: "a Session",
        raw: session.clone(),
        table: "sessions",
    });

    // An Access Key, minted by the logged-in Person rather than by another
    // Key: an agent cannot mint itself a second pair of hands.
    let (status, minted) = app.post_op(
        "mint_access_key",
        Some(&session),
        &json!({ "name": "another agent" }).to_string(),
    );
    assert_eq!(status, 200, "{minted}");
    secrets.push(Secret {
        what: "an Access Key",
        raw: minted["result"]["secret"]
            .as_str()
            .expect("a Secret")
            .into(),
        table: "access_keys",
    });

    // An Invite for a new Person, and a recovery link for an existing one.
    // Both are account links, and both are spent on first use.
    for (what, operation, input) in [
        ("an Invite", "mint_invite", json!({})),
        (
            "a recovery link",
            "mint_recovery_link",
            json!({ "name": "Aurélien" }),
        ),
    ] {
        let (status, minted) = app.post_op(operation, Some(&session), &input.to_string());
        assert_eq!(status, 200, "{minted}");
        let link = minted["result"]["link"].as_str().expect("a link");
        secrets.push(Secret {
            what,
            raw: link.rsplit('/').next().expect("a Secret").to_string(),
            table: "account_links",
        });
    }

    // A Kitchen Invite.
    let (status, kitchen) = app.post_op(
        "create_kitchen",
        Some(key),
        &json!({ "name": "Sunday Kitchen" }).to_string(),
    );
    assert_eq!(status, 200, "{kitchen}");
    let kitchen_id = kitchen["result"]["id"].as_str().expect("a Kitchen");
    let (status, invited) = app.post_op(
        "invite_to_kitchen",
        Some(key),
        &json!({ "kitchen_id": kitchen_id }).to_string(),
    );
    assert_eq!(status, 200, "{invited}");
    secrets.push(Secret {
        what: "a Kitchen Invite",
        raw: invited["result"]["secret"]
            .as_str()
            .expect("a Secret")
            .to_string(),
        table: "kitchen_invites",
    });

    // A Cookbook Invite: the one-use link to write one Cookbook together
    // (#131).
    let (status, invited) = app.post_op("invite_to_cookbook", Some(key), "{}");
    assert_eq!(status, 200, "{invited}");
    secrets.push(Secret {
        what: "a Cookbook Invite",
        raw: invited["result"]["secret"]
            .as_str()
            .expect("a Secret")
            .to_string(),
        table: "cookbook_invites",
    });

    // A Share Link's token.
    let (token, _branch_id) = a_shared_recipe(app, key);
    secrets.push(Secret {
        what: "a Share Link",
        raw: token,
        table: "share_links",
    });

    secrets
}

/// A recipe in its writer's Cookbook, shared, as a stranger would meet it.
/// Answers the Share Link's token and the Branch it names.
fn a_shared_recipe(app: &support::TestApp, key: &str) -> (String, String) {
    let (status, created) = app.post_op(
        "create_recipe",
        Some(key),
        &json!({ "title": "Tarte aux pommes" }).to_string(),
    );
    assert_eq!(status, 200, "{created}");
    let branch_id = created["result"]["branch_id"]
        .as_str()
        .expect("a Branch")
        .to_string();

    let (status, shared) = app.post_op(
        "share_recipe",
        Some(key),
        &json!({ "branch_id": branch_id, "public_address": "https://kamosu.example" }).to_string(),
    );
    assert_eq!(status, 200, "{shared}");
    let token = shared["result"]["url"]
        .as_str()
        .expect("a link")
        .rsplit('/')
        .next()
        .expect("a token")
        .to_string();
    (token, branch_id)
}

/// ADR 0031's rule, checked against every Secret the instance can hand out:
/// 256 bits, and kept only as a hash — save the Share Link, the one Secret
/// kept readable (#171, ADR 0031 as amended), and only in its own row.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_secret_is_256_bits_and_all_but_a_share_link_hashed_at_rest() {
    let app = support::spawn_app();
    let (_person, key) = operator(&app);
    let secrets = every_secret(&app, &key);

    let stored = everything_stored(&app);
    for secret in &secrets {
        // 256 bits, written as hex.
        assert_eq!(
            secret.raw.len(),
            64,
            "{} must be 256 bits, got {} characters",
            secret.what,
            secret.raw.len()
        );
        assert!(
            secret.raw.chars().all(|c| c.is_ascii_hexdigit()),
            "{} must be hex: {}",
            secret.what,
            secret.raw
        );

        // Hashed at rest. Whoever holds the disk holds everything Kamosu
        // stores, so what it stores must not be the Secret itself — except a
        // Share Link's, which reads one recipe that the disk already holds, and
        // is kept so the share screen can show its address again (#171).
        if secret.what == "a Share Link" {
            continue;
        }
        assert!(
            !stored.contains(&secret.raw),
            "{} is stored in the clear somewhere in the database",
            secret.what
        );
    }

    // The Share Link's Secret is kept in its own row and nowhere else, and it
    // is still looked up by its hash.
    let link = &secrets
        .iter()
        .find(|s| s.what == "a Share Link")
        .expect("a Share Link")
        .raw;
    let (kept, hashed): (String, String) = app
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row("SELECT secret, secret_hash FROM share_links", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .expect("the Share Link's row");
    assert_eq!(&kept, link);
    assert_ne!(
        &hashed, link,
        "the lookup column holds a hash, not the Secret"
    );
    assert_eq!(
        stored.matches(link.as_str()).count(),
        1,
        "a Share Link's Secret is kept once, in its own row, and nowhere else"
    );

    // No two Secrets are alike, which is the entropy claim made concrete
    // rather than asserted about a generator.
    let mut all: Vec<&str> = secrets.iter().map(|s| s.raw.as_str()).collect();
    all.sort_unstable();
    let before = all.len();
    all.dedup();
    assert_eq!(before, all.len(), "two Secrets came out the same");

    // Every kind of Secret the instance can hand out is one this test actually
    // minted. A new kind added later fails here until it is swept in, which is
    // what stops this file quietly becoming a partial sweep.
    let carrying = tables_holding_a_secret(&app);
    assert!(
        carrying.len() >= 5,
        "expected every kind of Secret to be found, got {carrying:?}"
    );
    for table in &carrying {
        assert!(
            secrets.iter().any(|s| s.table == table),
            "a kind of Secret this test never mints exists ({table}); add it to `every_secret`"
        );
    }
}

/// Never on a clock, asked of the Secret rather than of the schema.
///
/// Every Secret is aged a year and then used. A Secret that stopped working
/// because time passed would fail here whatever the column that did it was
/// called, which a sweep for names like `expires_at` could not promise: a
/// column named `dies_at` would sail straight through one.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_secret_still_works_a_year_after_it_was_made() {
    let app = support::spawn_app();
    let (_person, key) = operator(&app);
    let secrets = every_secret(&app, &key);

    // Push everything the instance recorded about when these were made, and
    // when they were last used, a year into the past. There is no clock to
    // fast-forward, so the rows move instead.
    for table in tables_holding_a_secret(&app) {
        let columns = columns_of(&app, &table);
        for column in ["created_at", "last_used_at"] {
            if !columns.iter().any(|held| held == column) {
                continue;
            }
            app.core
                .db()
                .with_conn(|conn| {
                    conn.execute(
                        &format!(
                            "UPDATE \"{table}\" SET \"{column}\" = \
                             strftime('%Y-%m-%dT%H:%M:%fZ','now','-1 year') \
                             WHERE \"{column}\" IS NOT NULL"
                        ),
                        [],
                    )
                    .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
                    Ok(())
                })
                .expect("age the Secrets");
        }
    }

    // A year-old Cookbook Invite is still one somebody may open.
    let cookbook_invite = &secrets
        .iter()
        .find(|s| s.what == "a Cookbook Invite")
        .expect("a Cookbook Invite")
        .raw;
    let invitee = app.core.create_person("Camille").expect("person");
    let invitee_key = app
        .core
        .mint_access_key(&invitee, "browser", false)
        .unwrap()
        .secret;
    let (status, read) = app.post_op(
        "read_cookbook_invite",
        Some(&invitee_key),
        &json!({ "secret": cookbook_invite }).to_string(),
    );
    assert_eq!(
        status, 200,
        "a year-old Cookbook Invite stopped working: {read}"
    );

    // The two that are Credentials in their own right still open the door.
    for secret in &secrets {
        match secret.what {
            "a Session" | "an Access Key" => assert_eq!(
                app.post_op("list_jobs", Some(&secret.raw), "{}").0,
                200,
                "{} stopped working because a year passed",
                secret.what
            ),
            _ => {}
        }
    }

    // And the Share Link, which is the one a stranger holds, still opens.
    let token = &secrets
        .iter()
        .find(|s| s.what == "a Share Link")
        .expect("a Share Link")
        .raw;
    let (status, _, page) = app.get(&format!("/s/{token}"));
    assert_eq!(status, 200, "a year-old Share Link must still open");
    assert!(
        page.contains("Tarte aux pommes"),
        "and must still show the recipe: {page}"
    );
}

/// Shown once: a Secret is answered at the moment it is minted and never
/// again. Listing what exists names it and says when it was last used, which
/// is the whole answer to a leak (ADR 0031). A Share Link's address is read
/// again only through its own recipe's `get_share_link` (#171), never in a
/// listing like these.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_secret_is_never_read_back_in_a_listing() {
    let app = support::spawn_app();
    let (_person, key) = operator(&app);
    let secrets = every_secret(&app, &key);

    for (operation, input) in [
        ("list_access_keys", json!({})),
        ("list_sessions", json!({})),
        ("get_cookbook", json!({})),
    ] {
        let (status, listed) = app.post_op(operation, Some(&key), &input.to_string());
        assert_eq!(status, 200, "{listed}");
        let text = listed.to_string();
        for secret in &secrets {
            assert!(
                !text.contains(&secret.raw),
                "{operation} answered {} a second time",
                secret.what
            );
        }
    }

    // A Share Link's address is read again only by asking for that recipe's
    // link (`get_share_link`, #171), never in an ordinary answer like this.
    let (status, standing) = app.post_op("list_kitchens", Some(&key), &json!({}).to_string());
    assert_eq!(status, 200, "{standing}");
    assert!(
        !standing
            .to_string()
            .contains(&secrets.last().expect("a Share Link").raw),
        "a Share Link's token must not come back in an ordinary answer"
    );
}

/// Individually revocable: ending one Secret ends that one and leaves every
/// other working.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ending_one_secret_leaves_every_other_one_working() {
    let app = support::spawn_app();
    let (person, _key) = operator(&app);

    let first = app
        .core
        .mint_access_key(&person, "first agent", false)
        .expect("a Key")
        .secret;
    let second = app
        .core
        .mint_access_key(&person, "second agent", false)
        .expect("a Key")
        .secret;

    let (status, listed) = app.post_op("list_access_keys", Some(&first), "{}");
    assert_eq!(status, 200, "{listed}");
    let first_id = listed["result"]["access_keys"]
        .as_array()
        .expect("the Keys")
        .iter()
        .find(|k| k["name"] == json!("first agent"))
        .and_then(|k| k["id"].as_str())
        .expect("the first Key's id")
        .to_string();

    let (status, revoked) = app.post_op(
        "revoke_access_key",
        Some(&second),
        &json!({ "access_key_id": first_id }).to_string(),
    );
    assert_eq!(status, 200, "{revoked}");

    assert_eq!(
        app.post_op("list_jobs", Some(&first), "{}").0,
        401,
        "the revoked Key is finished"
    );
    assert_eq!(
        app.post_op("list_jobs", Some(&second), "{}").0,
        200,
        "the other Key is untouched"
    );
}

// --- Throttling only where the secret is human-chosen (ADR 0031) -------------

/// A login body for the Operator `operator` creates, with the password given.
fn login_as_the_operator(password: &str) -> String {
    json!({
        "name": "Aurélien",
        "password": password,
        "session_name": "somewhere else",
    })
    .to_string()
}

/// The seconds a `busy` refusal says to wait, which the sign-in screen counts
/// down (#138). Absent means the refusal was not the throttle's.
fn seconds_to_wait(refused: &Value) -> Option<u64> {
    refused["error"]["retry_after_seconds"].as_u64()
}

/// A password is short and chosen by a person, so a wrong one makes the next
/// try for that name wait. Every other Secret is 256 bits, which cannot be
/// guessed, so throttling one would defend a door with no handle while
/// implying the size was not enough. Nothing but the password path may slow
/// down.
///
/// The wait holds nothing (#138). A try that arrives before it is over is
/// refused at once as busy, saying how long is left, rather than parked on a
/// thread until then — so a crowd of tries cannot freeze the instance, and
/// the throttle limits them however many arrive together.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn throttling_exists_only_where_the_secret_is_human_chosen() {
    let app = support::spawn_app();
    let (_person, _key) = operator(&app);
    let wrong = login_as_the_operator("not the password");
    let right = login_as_the_operator("a password only its person knows");

    // Two misses are free.
    assert_eq!(app.post_auth("/auth/login", &wrong).0, 401);
    assert_eq!(app.post_auth("/auth/login", &wrong).0, 401);

    // The third, straight after, is refused unchecked: the name waits a
    // second first. Even the right password waits it out, which is the narrow
    // lockout ADR 0031 accepts: devices already signed in keep their Session.
    for body in [&wrong, &right] {
        let (status, refused) = app.post_auth("/auth/login", body);
        assert_eq!(status, 503, "{refused}");
        assert_eq!(refused["error"]["kind"], "busy", "{refused}");
        assert_eq!(seconds_to_wait(&refused), Some(1), "{refused}");
    }

    // Once the second has passed, the next try is checked, and a miss doubles
    // the wait.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    assert_eq!(app.post_auth("/auth/login", &wrong).0, 401);
    let (status, refused) = app.post_auth("/auth/login", &right);
    assert_eq!(status, 503, "{refused}");
    assert_eq!(seconds_to_wait(&refused), Some(2), "{refused}");

    // The right password after the wait gets in, and clears the count: the
    // try after it is free again.
    std::thread::sleep(std::time::Duration::from_millis(2100));
    assert_eq!(
        app.post_auth("/auth/login", &right).0,
        200,
        "wrong guesses must never lock the household out for longer than the wait"
    );
    assert_eq!(app.post_auth("/auth/login", &wrong).0, 401);
    assert_eq!(
        app.post_auth("/auth/login", &right).0,
        200,
        "a correct password clears the count"
    );

    // Wrong bearer Secrets: refused at full speed, however many times. A
    // 256-bit Secret is answered by its size, not by a rate limit.
    let started = std::time::Instant::now();
    for attempt in 0..30 {
        let guess = format!("{attempt:064x}");
        assert_eq!(app.post_op("list_jobs", Some(&guess), "{}").0, 401);
    }
    let guessing = started.elapsed();
    assert!(
        guessing < std::time::Duration::from_secs(3),
        "guessing a Secret must not be throttled, 30 tries took {guessing:?}"
    );

    // And a Share Link token, which a stranger presents with no account at
    // all, is the same.
    let started = std::time::Instant::now();
    for attempt in 0..30 {
        let guess = format!("{attempt:064x}");
        let (status, _, _) = app.get(&format!("/s/{guess}"));
        assert_eq!(status, 404, "an unknown token is simply not found");
    }
    let enumerating = started.elapsed();
    assert!(
        enumerating < std::time::Duration::from_secs(3),
        "Share Link lookups must not be throttled, 30 tries took {enumerating:?}"
    );
}

/// Wrong passwords sent together, more of them than the runtime has workers,
/// leave every other request answering (#138). The wait used to sleep on a
/// request worker, so four tries at a throttled name froze the instance for
/// everyone for thirty seconds; and checking a password is ~20 MiB of Argon2
/// that used to run on those same workers.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wrong_passwords_in_a_crowd_leave_every_other_request_answering() {
    let app = support::spawn_app();
    operator(&app);
    let wrong = login_as_the_operator("not the password");
    assert_eq!(app.post_auth("/auth/login", &wrong).0, 401);
    assert_eq!(app.post_auth("/auth/login", &wrong).0, 401);

    // The name now waits. Eight more tries at it, and eight at names nobody
    // holds, each of which is checked against a password all the same.
    let strangers: Vec<String> = (0..8)
        .map(|n| {
            json!({ "name": format!("nobody {n}"), "password": "a guess at it", "session_name": "x" })
                .to_string()
        })
        .collect();
    std::thread::scope(|scope| {
        let crowd: Vec<_> = std::iter::repeat_n(&wrong, 8)
            .chain(strangers.iter())
            .map(|body| scope.spawn(|| app.post_auth("/auth/login", body).0))
            .collect();
        std::thread::sleep(std::time::Duration::from_millis(100));

        let started = std::time::Instant::now();
        let (status, answer) = app.post_op("instance_status", None, "{}");
        let took = started.elapsed();
        assert_eq!(status, 200, "{answer}");
        assert!(
            took < std::time::Duration::from_secs(1),
            "a crowd of wrong passwords froze an unrelated request for {took:?}"
        );
        for attempt in crowd {
            let status = attempt.join().expect("a sign-in attempt");
            assert!(
                matches!(status, 401 | 503),
                "a wrong password answered {status}"
            );
        }
    });
}

/// However many tries at one name arrive together, at most one is checked
/// (#138, choice A). The rest are refused at once as busy, with the seconds
/// left to wait — so sending guesses in parallel buys nothing over sending
/// them one at a time.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tries_at_one_name_sent_together_are_checked_one_at_a_time() {
    let app = support::spawn_app();
    operator(&app);
    let wrong = login_as_the_operator("not the password");
    assert_eq!(app.post_auth("/auth/login", &wrong).0, 401);
    assert_eq!(app.post_auth("/auth/login", &wrong).0, 401);
    // Past the one-second wait, so exactly one of what follows may be checked.
    std::thread::sleep(std::time::Duration::from_millis(1100));

    let answers: Vec<(u16, Value)> = std::thread::scope(|scope| {
        let tries: Vec<_> = (0..12)
            .map(|_| scope.spawn(|| app.post_auth("/auth/login", &wrong)))
            .collect();
        tries
            .into_iter()
            .map(|t| t.join().expect("a try"))
            .collect()
    });
    let checked = answers.iter().filter(|(status, _)| *status == 401).count();
    assert_eq!(checked, 1, "exactly one try is checked: {answers:?}");
    for (status, answer) in answers.iter().filter(|(status, _)| *status != 401) {
        assert_eq!(*status, 503, "{answer}");
        assert_eq!(answer["error"]["kind"], "busy", "{answer}");
        let seconds = seconds_to_wait(answer).expect("a busy refusal says how long");
        assert!((1..=2).contains(&seconds), "{answer}");
    }
}

/// A new password has a minimum length, NIST SP 800-63B-4's fifteen
/// characters, wherever one is set (#138). The refusal names the number.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_new_password_must_be_fifteen_characters() {
    let app = support::spawn_app();
    let short = "fourteen chars";
    assert_eq!(short.chars().count(), 14);

    let (status, refused) = app.post_auth(
        "/auth/first-person",
        &json!({ "name": "Aurélien", "password": short, "session_name": "x" }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");
    assert!(
        refused["error"]["message"].as_str().unwrap().contains("15"),
        "{refused}"
    );

    operator(&app);
    let invite = app.core.mint_invite(false).expect("an Invite");
    let (status, refused) = app.post_auth(
        "/auth/invite",
        &json!({ "link": invite, "name": "Marie", "password": short, "session_name": "x" })
            .to_string(),
    );
    assert_eq!(status, 400, "{refused}");
    assert!(
        refused["error"]["message"].as_str().unwrap().contains("15"),
        "{refused}"
    );

    let recovery = app
        .core
        .mint_recovery_link("Aurélien")
        .expect("a recovery link");
    let (status, refused) = app.post_auth(
        "/auth/recover",
        &json!({ "link": recovery, "password": short, "session_name": "x" }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");
    assert!(
        refused["error"]["message"].as_str().unwrap().contains("15"),
        "{refused}"
    );

    // Neither link was spent by a refusal: fifteen characters goes through.
    let long_enough = "fifteen charact";
    assert_eq!(long_enough.chars().count(), 15);
    let (status, answer) = app.post_auth(
        "/auth/invite",
        &json!({ "link": invite, "name": "Marie", "password": long_enough, "session_name": "x" })
            .to_string(),
    );
    assert_eq!(status, 200, "{answer}");
    let (status, answer) = app.post_auth(
        "/auth/recover",
        &json!({ "link": recovery, "password": long_enough, "session_name": "x" }).to_string(),
    );
    assert_eq!(status, 200, "{answer}");
}

// --- A stranger causes work, never work that scales (ADR 0032) ---------------

/// Work nobody signed in for runs one piece at a time behind a short line.
/// When the line is full the ask is refused outright, so the worst a stranger
/// can inflict is other strangers waiting. A member never queues behind them.
///
/// It needs `probe_job` to hold the lane's one worker, so it exists only under
/// `test-jobs`, which `just test` builds with.
///
/// The hold is a second, and filling the line takes five asks of a few
/// milliseconds each. That margin is wide but it is still a margin: a probe
/// that waited to be released would remove it, at the cost of a test-only
/// control nothing else needs.
#[cfg(feature = "test-jobs")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unauthenticated_job_work_runs_in_one_lane() {
    let app = support::spawn_app();
    let (_person, key) = operator(&app);

    let (token, branch_id) = a_shared_recipe(&app, &key);

    // Hold the stranger lane's one worker first. A Sheet of this small recipe
    // is made faster than the next ask arrives, so a crowd of Sheets alone
    // never filled the line and the test passed or failed on timing (#154).
    // With the worker busy for a second, the line can only fill.
    let (status, answer) = app.post_op("probe_job", None, r#"{"steps":40,"delay_ms":25}"#);
    assert_eq!(status, 200, "a stranger's probe is taken: {answer}");

    // A crowd of strangers, each asking for a Sheet with no Credential. Asking
    // answers a job id at once, so these stack up rather than taking turns.
    let mut accepted = 0;
    let mut refused = 0;
    for _ in 0..40 {
        let (status, answer) = app.post_op(
            "make_shared_sheet",
            None,
            &json!({ "token": token }).to_string(),
        );
        match status {
            200 => accepted += 1,
            503 => {
                assert_eq!(
                    answer["error"]["kind"], "busy",
                    "a full line says busy: {answer}"
                );
                refused += 1;
            }
            other => panic!("unexpected {other} from a stranger's Sheet: {answer}"),
        }
    }
    assert!(
        refused > 0,
        "the stranger lane must refuse rather than grow: {accepted} accepted, none refused"
    );
    assert!(
        accepted > 0,
        "a stranger may still cause work: none of {refused} asks was accepted"
    );

    // With the stranger lane saturated, a member's own work is still taken.
    let (status, mine) = app.post_op(
        "make_sheet",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(
        status, 200,
        "a member never waits behind strangers (ADR 0032): {mine}"
    );
}

/// The other work a stranger can cause, and the reason it is bounded by
/// something other than a lane. A Share Link sent into a chat is fetched by
/// every messaging app that sees it, so the card those apps show is drawn once
/// and kept against the Version it shows. Varying the picture is not something
/// a stranger can do: a Version is frozen, so the same link is the same card
/// for ever.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_share_links_card_is_drawn_once_and_kept() {
    let app = support::spawn_app();
    let (_person, key) = operator(&app);
    let (token, _branch_id) = a_shared_recipe(&app, &key);

    let (status, content_type, first) = app.get_bytes(&format!("/s/{token}/card"), None);
    assert_eq!(status, 200, "a stranger gets the card with no account");
    assert_eq!(content_type, "image/png");
    assert!(!first.is_empty(), "the card has to be a picture");

    // Asked for again, byte for byte the same, and without being drawn again:
    // the kept file is what answers.
    let kept = app
        .data_dir()
        .expect("the test owns the data directory")
        .join("cards");
    let drawn: Vec<_> = std::fs::read_dir(&kept)
        .expect("the card was kept")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(drawn.len(), 1, "one card was drawn, got {drawn:?}");
    assert!(
        drawn[0].starts_with("v_"),
        "the card is kept against the Version it shows, got {}",
        drawn[0]
    );

    for _ in 0..5 {
        let (status, _, again) = app.get_bytes(&format!("/s/{token}/card"), None);
        assert_eq!(status, 200);
        assert_eq!(again, first, "the same link must answer the same card");
    }
    let after: Vec<_> = std::fs::read_dir(&kept)
        .expect("the cards are still there")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        after, drawn,
        "asking six times must not draw six cards (ADR 0032)"
    );
}

// --- The list is part of the product (ADR 0034) ------------------------------

/// The things Kamosu does not defend, shipped where the people who need
/// them will see them.
///
/// ADR 0034 says the list must be maintained, and an entry that stops being
/// true is worse than one that never existed. It now lives in two places a
/// reader meets, so this fails if one of them loses an item the other kept.
/// The wording differs between them on purpose, since a settings screen is
/// read standing up; what must not differ is what is on the list.
#[test]
fn the_honest_list_is_shipped_whole_in_both_places() {
    let security = std::fs::read_to_string("SECURITY.md").expect("SECURITY.md");
    let messages = std::fs::read_to_string("ui/messages/en.json").expect("the English messages");

    let list = security
        .split_once("## What Kamosu does not defend")
        .expect("SECURITY.md ships the list")
        .1
        .split("\n## ")
        .next()
        .expect("the section ends somewhere");

    for (what, in_readme, key) in [
        ("the disk", "holds everything", "settings_operator_boundary"),
        (
            "a Hand",
            "A Hand is not checked",
            "settings_hand_unverified",
        ),
        (
            "a Share Link",
            "cannot be recalled",
            "settings_share_no_unsay",
        ),
        (
            "a stranger waiting",
            "make another stranger wait",
            "settings_stranger_waits",
        ),
        (
            "a stolen phone",
            "is a logged-in phone",
            "settings_stolen_phone",
        ),
        (
            "a name held off",
            "keep you from signing in",
            "settings_name_held_off",
        ),
        (
            "the first setup",
            "whoever sets Kamosu up",
            "settings_first_person_taken",
        ),
        (
            "an agent",
            "including by text it reads",
            "settings_agent_reads",
        ),
        ("an audit", "has not been audited", "settings_unaudited"),
    ] {
        assert!(
            list.contains(in_readme),
            "SECURITY.md stopped naming {what} ({in_readme:?})"
        );
        assert!(
            messages.contains(&format!("\"{key}\"")),
            "the app stopped naming {what} ({key})"
        );
    }
}

// --- An uploaded picture is checked before it is opened (#31) ----------------

/// A picture arriving from anywhere is decided on by its header, before a
/// decoder is handed the bytes. Decoding is the one place an untrusted upload
/// makes Kamosu do arbitrary work, so everything cheap happens first.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_uploaded_picture_is_header_checked_before_it_is_decoded() {
    let app = support::spawn_app();
    let (_person, key) = operator(&app);

    use base64::Engine as _;
    let base64_of = |bytes: &[u8]| base64::engine::general_purpose::STANDARD.encode(bytes);

    // A picture arrives two ways, as raw bytes out of band or as base64 for a
    // Door that cannot carry them (ADR 0001). Both land on the same check,
    // which is what the pairs below assert rather than assume.

    // SVG is a document that can carry script, not a picture, and Kamosu has
    // no renderer for one.
    let svg = b"<?xml version=\"1.0\"?><svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";
    let (status, refused) = app.post_bytes("/api/photographs", Some(&key), "image/svg+xml", svg);
    assert_eq!(status, 400, "SVG must be refused out of band: {refused}");
    assert!(
        refused.to_string().to_ascii_lowercase().contains("svg"),
        "the refusal names SVG: {refused}"
    );
    let (status, refused) = app.post_op(
        "upload_photograph",
        Some(&key),
        &json!({ "data": base64_of(svg) }).to_string(),
    );
    assert_eq!(status, 400, "SVG must be refused as base64 too: {refused}");

    // A decompression bomb: small on disk, enormous once unpacked. The header
    // declares the size, which is the only thing available before paying the
    // cost of decoding, so it is what Kamosu decides on.
    let bomb = png_claiming_to_be(50_000, 50_000);
    assert!(bomb.len() < 1024, "the bomb is small on disk");
    for (how, answer) in [
        (
            "out of band",
            app.post_bytes("/api/photographs", Some(&key), "image/png", &bomb),
        ),
        (
            "as base64",
            app.post_op(
                "upload_photograph",
                Some(&key),
                &json!({ "data": base64_of(&bomb) }).to_string(),
            ),
        ),
    ] {
        let (status, refused) = answer;
        assert_eq!(
            status, 400,
            "a picture claiming 2.5 gigapixels must be refused {how}: {refused}"
        );
        assert!(
            refused.to_string().contains("50000"),
            "the refusal says what the picture claimed: {refused}"
        );
    }

    // An ordinary picture still goes in, so none of the above passes because
    // uploading is broken.
    let ordinary = png_claiming_to_be(0, 0);
    let (status, stored) = app.post_bytes("/api/photographs", Some(&key), "image/png", &ordinary);
    assert_eq!(status, 200, "an ordinary picture is stored: {stored}");
    assert!(
        stored["result"]["photograph_id"].is_string(),
        "and answers its id: {stored}"
    );
}

/// CRC-32 as PNG defines it, over a chunk's type and data.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// A real 4 × 4 PNG whose header claims to be `width` × `height`. Passing
/// `0, 0` leaves it honest, which is how the same helper provides the ordinary
/// picture the bomb is compared against.
fn png_claiming_to_be(width: u32, height: u32) -> Vec<u8> {
    use image::{DynamicImage, ImageFormat};

    let image = DynamicImage::ImageRgb8(image::RgbImage::from_fn(4, 4, |x, y| {
        image::Rgb([(x * 60) as u8, (y * 60) as u8, 128])
    }));
    let mut png = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png), ImageFormat::Png)
        .expect("encodes");
    if width == 0 && height == 0 {
        return png;
    }
    // 8 bytes of signature, then IHDR: length, type, and data opening with the
    // two dimensions. The chunk's CRC follows its 13 bytes of data.
    png[16..20].copy_from_slice(&width.to_be_bytes());
    png[20..24].copy_from_slice(&height.to_be_bytes());
    let crc = crc32(&png[12..29]);
    png[29..33].copy_from_slice(&crc.to_be_bytes());
    png
}

/// Everything a stranger can reach, listed rather than counted.
///
/// Five Operations answer with no Credential at all, and each is here for a
/// reason worth being able to recite: the instance says whether it has been
/// set up (a browser has to ask before there is anyone to ask as), a Share
/// Link's recipe and its Sheet are the point of a Share Link, and a Job can be
/// watched and called off by whoever asked for it, since the Job id they hold
/// is the only thing that names it.
///
/// Only the last of those writes. A new name appearing in this list is a new
/// way into the instance, which is a thing to decide on rather than to find
/// out about later.
///
/// `probe_job` is left out on purpose. It is the demonstration Job that exists
/// only behind the `test-jobs` feature, so it is in no instance anybody runs,
/// and counting it here would make this test describe the test build rather
/// than the program.
#[test]
fn only_five_operations_answer_without_a_credential() {
    let shipped = || {
        kamosu::catalogue::OPERATIONS
            .iter()
            .filter(|op| op.name != "probe_job")
    };

    let public: Vec<&str> = shipped()
        .filter(|op| op.permission == kamosu::catalogue::Permission::Public)
        .map(|op| op.name)
        .collect();
    assert_eq!(
        public,
        vec![
            "instance_status",
            "make_shared_sheet",
            "read_shared_recipe",
            "get_job",
            "cancel_job",
        ],
        "the way in for a stranger has changed"
    );

    let writing: Vec<&str> = shipped()
        .filter(|op| op.permission == kamosu::catalogue::Permission::Public && op.write)
        .map(|op| op.name)
        .collect();
    assert_eq!(
        writing,
        vec!["cancel_job"],
        "a stranger may call off their own Job, and change nothing else"
    );
}

// --- A refusal never says whether a thing exists (#97, ADR 0040) -------------

/// One household: a Person, a Credential acting as them, and a Kitchen of their
/// own. Two of these on one instance is the whole setting for the leak below.
fn a_household(app: &support::TestApp, name: &str) -> Household {
    let person = app.core.create_person(name).expect("a Person");
    let key = app
        .core
        .mint_access_key(&person, "browser", false)
        .expect("an Access Key")
        .secret;
    let (status, made) = app.post_op(
        "create_kitchen",
        Some(&key),
        &json!({ "name": format!("{name}'s Kitchen") }).to_string(),
    );
    assert_eq!(status, 200, "{made}");
    let kitchen = made["result"]["id"]
        .as_str()
        .expect("a Kitchen")
        .to_string();
    Household { key, kitchen }
}

struct Household {
    key: String,
    kitchen: String,
}

/// One recipe on a household's own shelf, by its Branch id. It carries one
/// Ingredient Line, because `set_reading` needs a line to read.
fn a_recipe(app: &support::TestApp, who: &Household, title: &str) -> String {
    let (status, made) = app.post_op(
        "create_recipe",
        Some(&who.key),
        &json!({
            "title": title,
            "ingredients": [{ "kind": "ingredient", "text": "200 g flour" }],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{made}");
    made["result"]["branch_id"]
        .as_str()
        .expect("a Branch")
        .to_string()
}

/// One Tag on a household's own shelf, by its Tag id.
fn a_tag(app: &support::TestApp, who: &Household, word: &str) -> String {
    let (status, made) = app.post_op(
        "create_tag",
        Some(&who.key),
        &json!({ "language": "en", "name": word }).to_string(),
    );
    assert_eq!(status, 200, "{made}");
    made["result"]["id"].as_str().expect("a Tag").to_string()
}

/// One Import ledger on a household's own shelf, by its Import id. The `import`
/// Operation is a Job, so this waits for it.
fn an_import(app: &support::TestApp, who: &Household) -> String {
    let (status, asked) = app.post_op(
        "import",
        Some(&who.key),
        &json!({
            "source_kind": "a place recipes came from",
            "candidates": [{ "foreign_id": "one", "title": "Focaccia" }],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{asked}");
    let job_id = asked["result"]["job_id"].as_str().expect("a job id");
    let finished = support::wait_terminal(app, Some(&who.key), job_id);
    assert_eq!(finished["status"], json!("completed"), "{finished}");
    finished["result"]["import_id"]
        .as_str()
        .expect("an Import")
        .to_string()
}

/// What one Operation answers, reduced to exactly what the caller can see of a
/// refusal: the status and the error, kind and sentence both. A `200` reduces to
/// a marker rather than its body, because two different successes are not the
/// question here — only whether two refusals can be told apart.
fn answer(
    app: &support::TestApp,
    bearer: &str,
    operation: &str,
    input: serde_json::Value,
) -> String {
    let (status, body) = app.post_op(operation, Some(bearer), &input.to_string());
    if status == 200 {
        return "200 — answered".to_string();
    }
    format!(
        "{status} {} — {}",
        body["error"]["kind"].as_str().unwrap_or("?"),
        body["error"]["message"].as_str().unwrap_or("?"),
    )
}

/// **ADR 0040: where Kamosu worked the Kitchen out, a refusal never says
/// whether the thing exists.** Two households on one instance. Holding a Branch
/// id — which a Bundle hands out, so this is not hypothetical — Nadia must not
/// be able to tell whether Marc's household here holds that recipe.
///
/// Every Operation that takes an id and looks a Kitchen up from it is swept,
/// and each is asked twice: once about something Marc holds, once about an id
/// nobody ever minted. The two answers must be the same string.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_refusal_never_says_whether_another_household_here_holds_the_thing() {
    let app = support::spawn_app();
    let marc = a_household(&app, "Marc");
    let nadia = a_household(&app, "Nadia");

    // What Marc holds, and the ids of things that were never here. A minted id
    // is random, so these name nothing and can never come to.
    let his_recipe = a_recipe(&app, &marc, "Pizza");
    let his_tag = a_tag(&app, &marc, "sunday");
    let his_import = an_import(&app, &marc);
    let his_cookbook_invite = {
        let (status, minted) = app.post_op("invite_to_cookbook", Some(&marc.key), "{}");
        assert_eq!(status, 200, "{minted}");
        minted["result"]["invite_id"]
            .as_str()
            .expect("an Invite")
            .to_string()
    };
    // A join into Marc's Cookbook waiting on Paul, who writes it with him
    // (#135): answering it is for the people in it.
    let his_waiting_join = {
        let person = |name: &str| {
            let id = app.core.create_person(name).expect("a Person");
            app.core
                .mint_access_key(&id, "browser", false)
                .expect("an Access Key")
                .secret
        };
        let (paul, lea) = (person("Paul"), person("Léa"));
        let mut join_id = String::new();
        for guest in [&paul, &lea] {
            let (status, minted) = app.post_op("invite_to_cookbook", Some(&marc.key), "{}");
            assert_eq!(status, 200, "{minted}");
            let (status, accepted) = app.post_op(
                "accept_cookbook_invite",
                Some(guest),
                &json!({ "secret": minted["result"]["secret"] }).to_string(),
            );
            assert_eq!(status, 200, "{accepted}");
            join_id = accepted["result"]["joins"][0]["join_id"]
                .as_str()
                .unwrap_or_default()
                .to_string();
        }
        assert!(join_id.starts_with("cj_"), "Léa's join waits on Paul");
        join_id
    };
    let no_recipe = "b_ffffffffffffffff";
    let no_tag = "t_ffffffffffffffff";
    let no_import = "imp_ffffffffffffffff";

    // Nadia's own shelf, so that the Operations taking two ids have one honest
    // id to pair the probe with — and so the same sweep can be run again as a
    // member, where every one of them must still answer.
    let her_recipe = a_recipe(&app, &nadia, "Soupe");
    let her_other_recipe = a_recipe(&app, &nadia, "Tarte");
    let her_tag = a_tag(&app, &nadia, "weeknight");
    // A second Branch of the *same* Lineage, which is what `divergence` reads
    // between: a variation of her own recipe (ADR 0041).
    let her_copy = {
        let (status, varied) = app.post_op(
            "start_variation",
            Some(&nadia.key),
            &json!({ "branch_id": her_recipe, "name": "Copied" }).to_string(),
        );
        assert_eq!(status, 200, "{varied}");
        varied["result"]["branch_id"]
            .as_str()
            .expect("a variation's Branch")
            .to_string()
    };

    // An Attempt of Marc's: his own private record of a cooking. Every
    // Operation that reaches one does so through a single gate, and it must
    // answer about Marc's exactly as it answers about an id nobody minted.
    let his_attempt = {
        let (status, started) = app.post_op(
            "start_attempt",
            Some(&marc.key),
            &json!({ "branch_id": his_recipe }).to_string(),
        );
        assert_eq!(status, 200, "{started}");
        started["result"]["id"]
            .as_str()
            .expect("an Attempt")
            .to_string()
    };
    let no_attempt = "at_ffffffffffffffff";

    // Nadia's own cooking of her own recipe, sound in every way a promotion
    // checks — an As Cooked to keep, a picture to promote — so that promoting
    // it is refused, if at all, only for the Branch it is aimed at.
    let her_picture = an_upload(&app, &nadia.key, 9);
    let her_attempt = a_finished_cooking(&app, &nadia, &her_recipe, "Soupe", &her_picture);

    // Every Operation that works a Kitchen out from a Branch id, with the
    // Branch under probe first. `save_recipe_version` is here twice: a change,
    // and content identical to Marc's head, which once answered with his head's
    // Version id rather than refusing (#100). So is `edit_recipe` (#164), whose
    // empty edit is that same identical content.
    let by_branch: Vec<(&str, serde_json::Value)> = vec![
        ("save_recipe_version", json!({ "title": "Mine now" })),
        ("edit_recipe", json!({ "title": "Mine now" })),
        ("edit_recipe", json!({})),
        (
            "save_recipe_version",
            json!({
                "title": "Pizza",
                "ingredients": [{ "kind": "ingredient", "text": "200 g flour" }],
            }),
        ),
        ("get_recipe", json!({})),
        ("get_thread", json!({})),
        ("note_recipe_opened", json!({})),
        ("rename_version", json!({ "sequence": 1, "name": "a name" })),
        ("set_recipe_language", json!({ "language": "fr" })),
        (
            "start_translation",
            json!({ "language": "es", "title": "Pizza" }),
        ),
        (
            "set_recipe_tag",
            json!({ "tag_id": her_tag, "carried": true }),
        ),
        (
            "set_related_recipe",
            json!({ "related_branch_id": her_other_recipe, "related": true }),
        ),
        (
            "share_recipe",
            json!({ "public_address": "https://kamosu.example" }),
        ),
        ("end_share_link", json!({})),
        ("get_share_link", json!({})),
        ("export_bundle", json!({})),
        ("divergence", json!({ "other_branch_id": her_copy })),
        ("set_reading", json!({ "line_index": 0 })),
        ("start_attempt", json!({})),
        ("shopping_basis", json!({})),
        ("add_to_shopping_list", json!({})),
        ("set_shopping_yield", json!({})),
        ("start_variation", json!({ "name": "a name" })),
        ("rename_branch", json!({ "name": "a name" })),
        ("delete_recipe", json!({})),
    ];
    for (operation, rest) in &by_branch {
        let mut held = rest.clone();
        held["branch_id"] = json!(his_recipe);
        let mut absent = rest.clone();
        absent["branch_id"] = json!(no_recipe);
        assert_eq!(
            answer(&app, &nadia.key, operation, held),
            answer(&app, &nadia.key, operation, absent),
            "{operation} tells Nadia whether Marc's household holds that recipe"
        );
    }

    // Every Operation that reaches an Attempt by its id. An Attempt is one
    // Person's private record, so whose it is must be no part of the answer.
    let by_attempt: Vec<(&str, serde_json::Value)> = vec![
        ("advance_attempt", json!({ "current_step_index": 1 })),
        ("finish_attempt", json!({ "rating": "again" })),
        ("edit_attempt", json!({ "rating": "again" })),
        ("delete_attempt", json!({})),
        ("set_as_cooked", json!({ "as_cooked": Value::Null })),
        ("decline_promotion", json!({ "declined": true })),
        (
            "promote_as_cooked",
            json!({ "branch_id": her_recipe, "name": "a name" }),
        ),
        (
            "promote_attempt_photograph",
            json!({ "branch_id": her_recipe, "photograph_id": "ph_ffffffffffffffff" }),
        ),
    ];
    for (operation, rest) in &by_attempt {
        let mut held = rest.clone();
        held["attempt_id"] = json!(his_attempt);
        let mut absent = rest.clone();
        absent["attempt_id"] = json!(no_attempt);
        assert_eq!(
            answer(&app, &nadia.key, operation, held),
            answer(&app, &nadia.key, operation, absent),
            "{operation} tells Nadia whether that id names another Person's cooking"
        );
    }

    // The remaining paths, where the id under probe is not `branch_id`: the Tag
    // and Import ids, and the *second* id of the Operations that take two. The
    // second-id cases pair Marc's id with Nadia's own, so the refusal is reached
    // from the Kitchen *comparison* rather than from the first check — which is
    // the same oracle by another route, and the one easiest to leave open.
    struct Probe {
        operation: &'static str,
        /// The rest of the input, with the field under probe left out.
        rest: serde_json::Value,
        field: &'static str,
        /// The id Marc holds, and an id nobody ever minted.
        his: String,
        absent: &'static str,
    }
    let probes = vec![
        // The Branch a promotion is aimed at, with Nadia's own sound cooking:
        // Marc's recipe is not the dish she cooked, and saying so would say it
        // exists (#100).
        Probe {
            operation: "promote_as_cooked",
            rest: json!({ "attempt_id": her_attempt }),
            field: "branch_id",
            his: his_recipe.clone(),
            absent: no_recipe,
        },
        Probe {
            operation: "promote_attempt_photograph",
            rest: json!({ "attempt_id": her_attempt, "photograph_id": her_picture }),
            field: "branch_id",
            his: his_recipe.clone(),
            absent: no_recipe,
        },
        Probe {
            operation: "set_related_recipe",
            rest: json!({ "branch_id": her_recipe, "related": true }),
            field: "related_branch_id",
            his: his_recipe.clone(),
            absent: no_recipe,
        },
        Probe {
            operation: "divergence",
            rest: json!({ "branch_id": her_recipe }),
            field: "other_branch_id",
            his: his_recipe.clone(),
            absent: no_recipe,
        },
        // `branch_point` names neither of its ids `branch_id`, so it appears
        // here for both ends rather than in the sweep above. It is the Operation
        // most easily missed: nothing about its input says "branch_id".
        Probe {
            operation: "branch_point",
            rest: json!({ "branch_b_id": her_copy }),
            field: "branch_a_id",
            his: his_recipe.clone(),
            absent: no_recipe,
        },
        Probe {
            operation: "branch_point",
            rest: json!({ "branch_a_id": her_recipe }),
            field: "branch_b_id",
            his: his_recipe.clone(),
            absent: no_recipe,
        },
        Probe {
            operation: "set_recipe_tag",
            rest: json!({ "branch_id": her_recipe, "carried": true }),
            field: "tag_id",
            his: his_tag.clone(),
            absent: no_tag,
        },
        Probe {
            operation: "merge_tags",
            rest: json!({ "keep_tag_id": her_tag }),
            field: "merge_tag_id",
            his: his_tag.clone(),
            absent: no_tag,
        },
        Probe {
            operation: "merge_tags",
            rest: json!({ "merge_tag_id": her_tag }),
            field: "keep_tag_id",
            his: his_tag.clone(),
            absent: no_tag,
        },
        Probe {
            operation: "rename_tag",
            rest: json!({ "language": "en", "name": "monday" }),
            field: "tag_id",
            his: his_tag.clone(),
            absent: no_tag,
        },
        Probe {
            operation: "delete_tag",
            rest: json!({}),
            field: "tag_id",
            his: his_tag.clone(),
            absent: no_tag,
        },
        Probe {
            operation: "forget_import",
            rest: json!({}),
            field: "import_id",
            his: his_import.clone(),
            absent: no_import,
        },
        // A Cookbook Invite of Marc's, still waiting: ending it is his to do,
        // and whether one exists is nothing Nadia may learn (#131).
        Probe {
            operation: "cancel_cookbook_invite",
            rest: json!({}),
            field: "invite_id",
            his: his_cookbook_invite.clone(),
            absent: "ci_ffffffffffffffff",
        },
        // A join waiting on Marc's Cookbook: a no would call it off.
        Probe {
            operation: "answer_cookbook_join",
            rest: json!({ "yes": false }),
            field: "join_id",
            his: his_waiting_join.clone(),
            absent: "cj_ffffffffffffffff",
        },
    ];
    for probe in &probes {
        let mut held = probe.rest.clone();
        held[probe.field] = json!(probe.his);
        let mut absent = probe.rest.clone();
        absent[probe.field] = json!(probe.absent);
        assert_eq!(
            answer(&app, &nadia.key, probe.operation, held),
            answer(&app, &nadia.key, probe.operation, absent),
            "{}'s {} tells Nadia whether Marc's household holds it",
            probe.operation,
            probe.field,
        );
    }

    // `make_sheet` is a Job, so its refusal arrives as the Job's own failure
    // rather than in the answer to the ask. The rule holds there too.
    let sheet_refusal = |branch: &str| {
        let (status, asked) = app.post_op(
            "make_sheet",
            Some(&nadia.key),
            &json!({ "branch_id": branch }).to_string(),
        );
        assert_eq!(status, 200, "{asked}");
        let job_id = asked["result"]["job_id"].as_str().expect("a job id");
        let record = support::wait_terminal(&app, Some(&nadia.key), job_id);
        format!("{} — {}", record["status"], record["error"])
    };
    assert_eq!(
        sheet_refusal(&his_recipe),
        sheet_refusal(no_recipe),
        "make_sheet's failure tells Nadia whether Marc's household holds the recipe"
    );

    // A Job of Marc's, and a Job id nobody minted, at the **web** Door — where
    // the collapse used to be missing because it was written inside the MCP one.
    let (status, asked) = app.post_op(
        "make_sheet",
        Some(&marc.key),
        &json!({ "branch_id": his_recipe }).to_string(),
    );
    assert_eq!(status, 200, "{asked}");
    let his_job = asked["result"]["job_id"]
        .as_str()
        .expect("a job id")
        .to_string();
    support::wait_terminal(&app, Some(&marc.key), &his_job);
    let no_job = "job-nobody-ever-minted";
    for operation in ["get_job", "cancel_job"] {
        // The Job id is in the sentence, and that is no leak: the caller
        // supplied it. Normalised so the two sentences can be compared at all.
        let his = answer(&app, &nadia.key, operation, json!({ "job_id": his_job }))
            .replace(&his_job, "<the id asked for>");
        let none = answer(&app, &nadia.key, operation, json!({ "job_id": no_job }))
            .replace(no_job, "<the id asked for>");
        assert_eq!(
            his, none,
            "{operation} tells Nadia whether that id names another Person's work"
        );
    }

    // The rule lives in the Core, so it is the same refusal at the MCP Door —
    // which is the point. The collapse it used to hand-roll for Jobs is gone.
    let at_mcp = |operation: &str, arguments: serde_json::Value| {
        let call = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": operation, "arguments": arguments },
        });
        let (_, answered) = app.post_mcp(&call.to_string(), Some(&nadia.key));
        answered["result"].clone()
    };
    for (operation, field, his, none) in [
        ("get_recipe", "branch_id", his_recipe.as_str(), no_recipe),
        ("get_job", "job_id", his_job.as_str(), no_job),
    ] {
        let held = at_mcp(operation, json!({ field: his }));
        let absent = at_mcp(operation, json!({ field: none }));
        assert_eq!(held["isError"], json!(true), "{held}");
        assert_eq!(
            held.to_string().replace(his, "<the id asked for>"),
            absent.to_string().replace(none, "<the id asked for>"),
            "{operation} at the MCP Door tells Nadia the difference"
        );
    }

    // `/api/sheets/<id>` is not an Operation but an out-of-band route, and it
    // resolved a **Sheet** id. Marc's finished Sheet and an id naming no Sheet
    // must therefore answer alike — and about Sheets, not about Jobs, or the
    // sentence would itself say the id names somebody's Job.
    let sheet_route = |id: &str| {
        let (status, _type, body) = app.get_bytes(&format!("/api/sheets/{id}"), Some(&nadia.key));
        format!("{status} {}", String::from_utf8_lossy(&body))
    };
    assert_eq!(
        sheet_route(&his_job),
        sheet_route("j_ffffffffffffffffffffffff"),
        "/api/sheets tells Nadia whether that id names another Person's Sheet"
    );

    // --- And the other half of the rule: what must NOT have changed ----------

    // A Kitchen the caller **named** still gets the membership refusal. Being
    // told you do not cook in a Kitchen whose id you just supplied says nothing
    // you did not already know.
    let (status, refused) = app.post_op(
        "rename_kitchen",
        Some(&nadia.key),
        &json!({ "kitchen_id": marc.kitchen, "name": "Not hers" }).to_string(),
    );
    assert_eq!(status, 401, "{refused}");
    assert_eq!(
        refused["error"]["message"],
        json!("this Person does not cook in this Kitchen"),
        "{refused}"
    );

    // And a member still gets the real result from the Operations above.
    //
    // `delete_recipe` gets a recipe of its own rather than the one every other
    // Operation here is run against: it really performs, and a sweep whose
    // correctness depended on it being last in the list would break the first
    // time somebody appended a line below it.
    let her_spare = a_recipe(&app, &nadia, "Soupe de trop");
    for (operation, rest) in &by_branch {
        let mut mine = rest.clone();
        mine["branch_id"] = if *operation == "delete_recipe" {
            json!(her_spare)
        } else {
            json!(her_recipe)
        };
        let (status, answered) = app.post_op(operation, Some(&nadia.key), &mine.to_string());
        assert_eq!(
            status, 200,
            "{operation} refused its own Kitchen: {answered}"
        );
    }
}

// --- May see is not may change (ADR 0041, #131) ----------------------------

/// **A Kitchen-mate sees a recipe and may not change it** (ADR 0041). Nadia
/// cooks in Marc's Kitchen, so his Cookbook is hers to read, cook and share —
/// and nothing of it is hers to change, delete or file. Every Operation that
/// needs a Co-author refuses her in words, as unauthorised, and changes
/// nothing; a save is not refused but starts her own Branch instead. What she
/// may do still answers.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_kitchen_mate_may_see_and_cook_a_recipe_and_change_none_of_it() {
    let app = support::spawn_app();
    let marc = a_household(&app, "Marc");
    let nadia = a_household(&app, "Nadia");
    let his_recipe = a_recipe(&app, &marc, "Pizza");
    let his_other = a_recipe(&app, &marc, "Focaccia");
    let his_tag = a_tag(&app, &marc, "sunday");
    let his_import = an_import(&app, &marc);
    let her_recipe = a_recipe(&app, &nadia, "Soupe");
    let her_tag = a_tag(&app, &nadia, "weeknight");

    let (status, invite) = app.post_op(
        "invite_to_kitchen",
        Some(&marc.key),
        &json!({ "kitchen_id": marc.kitchen }).to_string(),
    );
    assert_eq!(status, 200, "{invite}");
    let (status, joined) = app.post_op(
        "accept_kitchen_invite",
        Some(&nadia.key),
        &json!({ "secret": invite["result"]["secret"] }).to_string(),
    );
    assert_eq!(status, 200, "{joined}");

    let before = recipe_rows(&app);
    let refused_as_not_hers: Vec<(&str, serde_json::Value)> = vec![
        ("delete_recipe", json!({ "branch_id": his_recipe })),
        (
            "rename_branch",
            json!({ "branch_id": his_recipe, "name": "Hers" }),
        ),
        (
            "set_recipe_tag",
            json!({ "branch_id": his_recipe, "tag_id": his_tag, "carried": true }),
        ),
        (
            "set_related_recipe",
            json!({ "branch_id": his_recipe, "related_branch_id": his_other, "related": true }),
        ),
        (
            "rename_tag",
            json!({ "tag_id": his_tag, "language": "en", "name": "monday" }),
        ),
        ("delete_tag", json!({ "tag_id": his_tag })),
        (
            "merge_tags",
            json!({ "keep_tag_id": his_tag, "merge_tag_id": her_tag }),
        ),
        ("forget_import", json!({ "import_id": his_import })),
        // A variation is your own recipe branched on purpose; reading his
        // never starts one (ADR 0041). A save starts her own copy instead.
        (
            "start_variation",
            json!({ "branch_id": his_recipe, "name": "Hers" }),
        ),
    ];
    for (operation, input) in &refused_as_not_hers {
        let (status, refused) = app.post_op(operation, Some(&nadia.key), &input.to_string());
        assert_eq!(
            status, 401,
            "{operation} let a reader change Marc's Cookbook: {refused}"
        );
        assert_eq!(refused["error"]["kind"], json!("unauthorized"), "{refused}");
    }
    assert_eq!(recipe_rows(&app), before, "and nothing was written");

    // Her own recipe filed under his word is a request for a Tag her Cookbook
    // does not have — which she may be told, since she sees it.
    let (status, refused) = app.post_op(
        "set_recipe_tag",
        Some(&nadia.key),
        &json!({ "branch_id": her_recipe, "tag_id": his_tag, "carried": true }).to_string(),
    );
    assert_eq!(status, 404, "{refused}");

    // What seeing allows still answers: reading, cooking, sharing, shopping
    // and translating, the last of them, and a save, into her own Cookbook.
    let allowed: Vec<(&str, serde_json::Value)> = vec![
        ("get_recipe", json!({ "branch_id": his_recipe })),
        ("get_thread", json!({ "branch_id": his_recipe })),
        ("start_attempt", json!({ "branch_id": his_recipe })),
        (
            "share_recipe",
            json!({ "branch_id": his_recipe, "public_address": "https://kamosu.example" }),
        ),
        ("add_to_shopping_list", json!({ "branch_id": his_recipe })),
        ("export_bundle", json!({ "branch_id": his_recipe })),
        (
            "set_reading",
            json!({ "branch_id": his_recipe, "line_index": 0 }),
        ),
        (
            "start_translation",
            json!({ "branch_id": his_recipe, "language": "fr", "title": "Pizza" }),
        ),
        (
            "save_recipe_version",
            json!({ "branch_id": his_recipe, "title": "Pizza, hers" }),
        ),
        (
            "edit_recipe",
            json!({ "branch_id": his_recipe, "note": "Hers too." }),
        ),
    ];
    for (operation, input) in &allowed {
        let (status, answered) = app.post_op(operation, Some(&nadia.key), &input.to_string());
        assert_eq!(status, 200, "{operation} refused a reader: {answered}");
    }
    let (_, his) = app.post_op(
        "get_recipe",
        Some(&marc.key),
        &json!({ "branch_id": his_recipe }).to_string(),
    );
    assert_eq!(
        his["result"]["versions"].as_array().unwrap().len(),
        1,
        "untouched"
    );
}

// --- A Copy starts only from a Branch a Kitchen of yours holds (#100) --------

/// A whole answer, status and body, with nothing reduced. `answer` keeps only
/// what tells two refusals apart; this keeps everything, so two answers equal
/// here are byte for byte the same reply.
fn whole_answer(
    app: &support::TestApp,
    bearer: &str,
    operation: &str,
    input: serde_json::Value,
) -> (u16, String) {
    let (status, body) = app.post_op(operation, Some(bearer), &input.to_string());
    (status, body.to_string())
}

/// How many Branches, Versions on a Branch, and Versions the instance holds:
/// everything a save or a promotion writes.
fn recipe_rows(app: &support::TestApp) -> (i64, i64, i64) {
    app.core
        .db()
        .with_conn(|conn| {
            conn.query_row(
                "SELECT (SELECT COUNT(*) FROM branches), \
                        (SELECT COUNT(*) FROM branch_versions), \
                        (SELECT COUNT(*) FROM versions)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .expect("count the recipe rows")
}

/// A cooking a household finished, of a recipe on its own shelf, sound in every
/// way a promotion checks: an As Cooked to keep and a picture to promote.
fn a_finished_cooking(
    app: &support::TestApp,
    who: &Household,
    branch_id: &str,
    title: &str,
    picture: &str,
) -> String {
    let (status, started) = app.post_op(
        "start_attempt",
        Some(&who.key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 200, "{started}");
    let attempt = started["result"]["id"]
        .as_str()
        .expect("an Attempt")
        .to_string();
    let (status, cooked) = app.post_op(
        "set_as_cooked",
        Some(&who.key),
        &json!({
            "attempt_id": attempt,
            "as_cooked": { "title": title, "ingredients": [
                { "kind": "ingredient", "text": "250 g flour" }
            ] },
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{cooked}");
    let (status, finished) = app.post_op(
        "finish_attempt",
        Some(&who.key),
        &json!({ "attempt_id": attempt, "photographs": [picture] }).to_string(),
    );
    assert_eq!(status, 200, "{finished}");
    attempt
}

/// Receive one household's Bundle into another's shelf, the way a Share Link
/// reader keeps a recipe, and answer the Branch id it landed under.
fn received(app: &support::TestApp, from: &Household, to: &Household, branch_id: &str) -> String {
    use base64::Engine;
    let (status, _type, bytes) =
        app.get_bytes(&format!("/api/bundles/{branch_id}"), Some(&from.key));
    assert_eq!(status, 200, "the Bundle is fetched");
    let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let (status, asked) = app.post_op(
        "import_bundle",
        Some(&to.key),
        &json!({ "data": data }).to_string(),
    );
    assert_eq!(status, 200, "{asked}");
    let job_id = asked["result"]["job_id"].as_str().expect("a job id");
    let finished = support::wait_terminal(app, Some(&to.key), job_id);
    assert_eq!(finished["status"], json!("completed"), "{finished}");
    finished["result"]["arrived"]
        .as_array()
        .expect("what arrived")
        .iter()
        .find(|row| row["foreign_id"] == json!(branch_id))
        .and_then(|row| row["branch_id"].as_str())
        .unwrap_or_else(|| panic!("{branch_id} arrived: {finished}"))
        .to_string()
}

/// **#100: a Copy starts only from a Branch a Kitchen of yours holds.** A Copy
/// carries the whole chain behind it, so a save onto a Branch nobody gave you
/// used to hand over every Version of another household's recipe: title, notes,
/// lines and their Readings. Now it answers exactly what an id naming nothing
/// answers, and writes nothing. So do both promotions, whether the Branch is
/// of the dish the cook really cooked or of another.
///
/// The legitimate routes into a Copy are untouched, and are run here too: a
/// received Bundle, edited, still makes a Copy carrying the sender's chain.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_copy_starts_only_from_a_branch_a_kitchen_of_yours_holds() {
    let app = support::spawn_app();
    let marc = a_household(&app, "Marc");
    let nadia = a_household(&app, "Nadia");
    let his_recipe = a_recipe(&app, &marc, "Pizza");
    let his_other_recipe = a_recipe(&app, &marc, "Focaccia");
    let no_recipe = "b_ffffffffffffffff";

    // The body's two-call reproduction, both with a change and with Marc's head
    // word for word: the second once answered with his head's Version id.
    // `edit_recipe` is the same save said field by field (#164), and its
    // empty edit is Marc's head word for word.
    let before = recipe_rows(&app);
    for (operation, content) in [
        ("save_recipe_version", json!({ "title": "Mine now" })),
        (
            "save_recipe_version",
            json!({
                "title": "Pizza",
                "ingredients": [{ "kind": "ingredient", "text": "200 g flour" }],
            }),
        ),
        ("edit_recipe", json!({ "title": "Mine now" })),
        ("edit_recipe", json!({})),
    ] {
        let mut held = content.clone();
        held["branch_id"] = json!(his_recipe);
        let mut absent = content;
        absent["branch_id"] = json!(no_recipe);
        let refused = whole_answer(&app, &nadia.key, operation, held);
        assert_eq!(refused.0, 404, "{}", refused.1);
        assert_eq!(
            refused,
            whole_answer(&app, &nadia.key, operation, absent),
            "{operation} tells Nadia Marc's recipe is here"
        );
    }
    assert_eq!(recipe_rows(&app), before, "a refused save writes nothing");

    // Nadia keeps Marc's recipe the legitimate way, and cooks it. Her cooking
    // is of his Lineage, so only the Kitchen holding his own Branch stands
    // between her promotion and his recipe.
    let hers = received(&app, &marc, &nadia, &his_recipe);
    let picture = an_upload(&app, &nadia.key, 11);
    let attempt = a_finished_cooking(&app, &nadia, &hers, "Pizza", &picture);

    let before = recipe_rows(&app);
    for (operation, rest) in [
        ("promote_as_cooked", json!({ "attempt_id": attempt })),
        (
            "promote_attempt_photograph",
            json!({ "attempt_id": attempt, "photograph_id": picture }),
        ),
    ] {
        // His Branch of the dish she cooked, and one of a dish she did not:
        // the second must not answer with the Lineage-mismatch sentence.
        for target in [&his_recipe, &his_other_recipe] {
            let mut held = rest.clone();
            held["branch_id"] = json!(target);
            let mut absent = rest.clone();
            absent["branch_id"] = json!(no_recipe);
            let refused = whole_answer(&app, &nadia.key, operation, held);
            assert_eq!(refused.0, 404, "{operation}: {}", refused.1);
            assert_eq!(
                refused,
                whole_answer(&app, &nadia.key, operation, absent),
                "{operation} tells Nadia Marc's recipe is here"
            );
        }
    }
    assert_eq!(
        recipe_rows(&app),
        before,
        "a refused promotion writes nothing"
    );

    // What she was given, she may change: promoting into the Branch that
    // arrived makes a Copy of her own, and so does editing it. The Copy carries
    // Marc's chain, so the Branch Point and the Divergence both still answer.
    let (status, promoted) = app.post_op(
        "promote_as_cooked",
        Some(&nadia.key),
        &json!({ "attempt_id": attempt, "branch_id": hers }).to_string(),
    );
    assert_eq!(status, 200, "{promoted}");
    assert_eq!(promoted["result"]["copied"], json!(true), "{promoted}");
    backdate_the_head(&app, &hers);
    let (status, saved) = app.post_op(
        "save_recipe_version",
        Some(&nadia.key),
        &json!({ "branch_id": hers, "title": "Pizza de Nadia" }).to_string(),
    );
    assert_eq!(status, 200, "{saved}");
    assert_eq!(saved["result"]["copied"], json!(true), "{saved}");
    let copy = saved["result"]["branch_id"]
        .as_str()
        .expect("a Copy")
        .to_string();
    let (status, read) = app.post_op(
        "get_recipe",
        Some(&nadia.key),
        &json!({ "branch_id": copy }).to_string(),
    );
    assert_eq!(status, 200, "{read}");
    assert_eq!(
        read["result"]["versions"][0]["content"]["title"],
        json!("Pizza"),
        "the Copy carries the chain it forked from"
    );
    for (operation, input) in [
        (
            "branch_point",
            json!({ "branch_a_id": hers, "branch_b_id": copy }),
        ),
        (
            "divergence",
            json!({ "branch_id": copy, "other_branch_id": hers }),
        ),
    ] {
        let (status, answered) = app.post_op(operation, Some(&nadia.key), &input.to_string());
        assert_eq!(status, 200, "{operation}: {answered}");
    }
}

// --- A Photograph is seen by whoever can already see it (#99, ADR 0026) ------

/// A real, freshly encoded picture, different for each `seed`, so every
/// Photograph below is its own hash.
fn a_picture(seed: u32) -> Vec<u8> {
    let image = image::RgbImage::from_fn(24 + seed, 20, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, (seed * 40 % 256) as u8])
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

/// Upload a picture through the out-of-band route, as the writing screen does,
/// and answer its Photograph id.
fn an_upload(app: &support::TestApp, key: &str, seed: u32) -> String {
    let (status, uploaded) = app.post_bytes(
        "/api/photographs",
        Some(key),
        "image/jpeg",
        &a_picture(seed),
    );
    assert_eq!(status, 200, "{uploaded}");
    uploaded["result"]["photograph_id"]
        .as_str()
        .expect("a Photograph id")
        .to_string()
}

/// What one picture route answers, reduced to what the caller can tell apart:
/// the status and the whole body of a refusal, or a marker for bytes served.
fn picture_answer(app: &support::TestApp, key: Option<&str>, path: &str) -> String {
    let (status, _type, body) = app.get_bytes(path, key);
    if status == 200 {
        return "200 — a picture".to_string();
    }
    format!("{status} {}", String::from_utf8_lossy(&body))
}

/// A save more than the collapse window after the last one, so it adds a
/// Version to the Thread rather than replacing the head (ADR 0005).
fn backdate_the_head(app: &support::TestApp, branch_id: &str) {
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
        .expect("backdate the head");
}

fn save_version(app: &support::TestApp, key: &str, input: serde_json::Value) -> String {
    let (status, saved) = app.post_op("save_recipe_version", Some(key), &input.to_string());
    assert_eq!(status, 200, "{saved}");
    saved["result"]["branch_id"]
        .as_str()
        .expect("a Branch")
        .to_string()
}

/// **#99: a Photograph is readable by a Person who can already see it
/// somewhere, and by nobody else.** Two households on one instance. A
/// Photograph's id is the hash of its bytes (ADR 0017), so Nadia needs no help
/// to name Marc's picture: holding the same image file is enough. Asking for it
/// must then answer exactly what asking for a picture this instance never held
/// answers, at both routes, and must cost the instance nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_photograph_is_seen_only_by_whoever_can_already_see_it() {
    let app = support::spawn_app();
    let data_dir = app.data_dir().expect("a data directory").to_path_buf();
    let marc = a_household(&app, "Marc");
    let nadia = a_household(&app, "Nadia");

    // Léa cooks in Marc's Kitchen and uploads nothing herself: every picture
    // she reads below, she reads because the household can see it.
    let lea = {
        let person = app.core.create_person("Léa").expect("a Person");
        app.core
            .mint_access_key(&person, "browser", false)
            .expect("an Access Key")
            .secret
    };
    let (status, invite) = app.post_op(
        "invite_to_kitchen",
        Some(&marc.key),
        &json!({ "kitchen_id": marc.kitchen }).to_string(),
    );
    assert_eq!(status, 200, "{invite}");
    let (status, joined) = app.post_op(
        "accept_kitchen_invite",
        Some(&lea),
        &json!({ "secret": invite["result"]["secret"] }).to_string(),
    );
    assert_eq!(status, 200, "{joined}");

    // What a hash this instance has never held answers, at each route. Every
    // refusal below is held to exactly this.
    let never = "0".repeat(64);
    let unknown = picture_answer(&app, Some(&nadia.key), &format!("/api/photographs/{never}"));
    let unknown_card = picture_answer(
        &app,
        Some(&nadia.key),
        &format!("/api/photographs/{never}/card"),
    );
    assert!(unknown.starts_with("404"), "{unknown}");
    let refused_like_nothing = |who: &str, hash: &str, why: &str| {
        assert_eq!(
            picture_answer(&app, Some(who), &format!("/api/photographs/{hash}")),
            unknown,
            "the Photograph route {why}"
        );
        assert_eq!(
            picture_answer(&app, Some(who), &format!("/api/photographs/{hash}/card")),
            unknown_card,
            "the Display Copy route {why}"
        );
    };
    let readable = |who: &str, hash: &str, why: &str| {
        for suffix in ["", "/card", "/page", "/print"] {
            assert_eq!(
                picture_answer(&app, Some(who), &format!("/api/photographs/{hash}{suffix}")),
                "200 — a picture",
                "{suffix} {why}"
            );
        }
    };

    // --- The writing screen: the uploader reads a picture no recipe names yet.
    let main = an_upload(&app, &marc.key, 1);
    readable(
        &marc.key,
        &main,
        "refuses the uploader their own picture before the save",
    );
    refused_like_nothing(
        &nadia.key,
        &main,
        "hands Nadia a picture only Marc has uploaded",
    );
    refused_like_nothing(
        &lea,
        &main,
        "hands the household a picture no recipe shows yet",
    );

    // --- A saved recipe: its Kitchen sees it, nobody else does.
    let recipe = a_recipe(&app, &marc, "Pizza");
    save_version(
        &app,
        &marc.key,
        json!({ "branch_id": recipe, "title": "Pizza", "main_photo": main }),
    );
    readable(
        &lea,
        &main,
        "refuses a member the Main Photo of her own Kitchen's recipe",
    );
    refused_like_nothing(
        &nadia.key,
        &main,
        "tells Nadia that Marc's household holds this picture",
    );

    // A refused ask draws nothing. Nobody has asked for a `print` copy of it
    // yet, so one on disk after Nadia's ask would be work she caused.
    let print_copy = kamosu::photographs::display_path(
        &data_dir,
        &main,
        kamosu::photographs::DisplaySize::Print,
    );
    let _ = std::fs::remove_file(&print_copy);
    assert_eq!(
        picture_answer(
            &app,
            Some(&nadia.key),
            &format!("/api/photographs/{main}/print")
        ),
        picture_answer(
            &app,
            Some(&nadia.key),
            &format!("/api/photographs/{never}/print")
        ),
    );
    assert!(
        !print_copy.exists(),
        "a refused request generated a Display Copy"
    );

    // --- A Step's picture, shown only by an older Version in the Thread.
    let step = an_upload(&app, &marc.key, 2);
    save_version(
        &app,
        &marc.key,
        json!({
            "branch_id": recipe,
            "title": "Pizza",
            "main_photo": main,
            "steps": [{ "kind": "step", "text": "Stretch the dough.", "photo": step }],
        }),
    );
    backdate_the_head(&app, &recipe);
    save_version(
        &app,
        &marc.key,
        json!({ "branch_id": recipe, "title": "Pizza", "main_photo": main }),
    );
    readable(&lea, &step, "forgets a picture the Thread still shows");
    refused_like_nothing(
        &nadia.key,
        &step,
        "hands Nadia a picture from Marc's Thread",
    );

    // --- A picture shown only by a Translation of the recipe.
    let (status, started) = app.post_op(
        "start_translation",
        Some(&marc.key),
        &json!({ "branch_id": recipe, "language": "fr", "title": "Pizza" }).to_string(),
    );
    assert_eq!(status, 200, "{started}");
    let translation = started["result"]["branch_id"]
        .as_str()
        .expect("a Translation")
        .to_string();
    let translated = an_upload(&app, &marc.key, 3);
    save_version(
        &app,
        &marc.key,
        json!({ "branch_id": translation, "title": "Pizza", "main_photo": translated }),
    );
    readable(
        &lea,
        &translated,
        "refuses a member the picture on a Translation",
    );
    refused_like_nothing(
        &nadia.key,
        &translated,
        "hands Nadia a Translation's picture",
    );

    // --- A picture an Attempt holds and no recipe names (#59).
    let (status, attempt) = app.post_op(
        "start_attempt",
        Some(&marc.key),
        &json!({ "branch_id": recipe }).to_string(),
    );
    assert_eq!(status, 200, "{attempt}");
    let plate = an_upload(&app, &marc.key, 4);
    let (status, edited) = app.post_op(
        "edit_attempt",
        Some(&marc.key),
        &json!({ "attempt_id": attempt["result"]["id"], "add_photographs": [plate] }).to_string(),
    );
    assert_eq!(status, 200, "{edited}");
    readable(
        &lea,
        &plate,
        "refuses the household a picture of its own cooking",
    );
    refused_like_nothing(
        &nadia.key,
        &plate,
        "hands Nadia a picture of Marc's cooking",
    );

    // --- Detached from everything: the household stops seeing it at once,
    // rather than a week later when the sweep takes it. A save inside the
    // collapse window replaces the head, so the picture is on nothing.
    let passing = an_upload(&app, &marc.key, 5);
    let pasta = a_recipe(&app, &marc, "Pasta");
    save_version(
        &app,
        &marc.key,
        json!({ "branch_id": pasta, "title": "Pasta", "main_photo": passing }),
    );
    readable(
        &lea,
        &passing,
        "refuses a member a picture on her Kitchen's recipe",
    );
    save_version(
        &app,
        &marc.key,
        json!({ "branch_id": pasta, "title": "Pasta" }),
    );
    refused_like_nothing(
        &lea,
        &passing,
        "keeps a detached picture readable by the household",
    );

    // --- Nobody without a Credential, as before.
    let (status, _, _) = app.get_bytes(&format!("/api/photographs/{main}"), None);
    assert_eq!(status, 401);
    let (status, _, _) = app.get_bytes(&format!("/api/photographs/{main}/card"), None);
    assert_eq!(status, 401);

    // --- The Share Link is its own route with its own check, untouched: a
    // stranger with a live link reads the shared recipe's picture, and an
    // ended link refuses.
    let (status, shared) = app.post_op(
        "share_recipe",
        Some(&marc.key),
        &json!({ "branch_id": recipe, "public_address": "https://kamosu.example" }).to_string(),
    );
    assert_eq!(status, 200, "{shared}");
    let token = shared["result"]["url"]
        .as_str()
        .expect("a link")
        .rsplit('/')
        .next()
        .expect("a token")
        .to_string();
    let (status, content_type, _) = app.get_bytes(&format!("/s/{token}/photo/{main}"), None);
    assert_eq!((status, content_type.as_str()), (200, "image/webp"));
    let (status, ended) = app.post_op(
        "end_share_link",
        Some(&marc.key),
        &json!({ "branch_id": recipe }).to_string(),
    );
    assert_eq!(status, 200, "{ended}");
    let (status, _, _) = app.get_bytes(&format!("/s/{token}/photo/{main}"), None);
    assert_eq!(status, 404, "an ended link still hands out the picture");
}

// --- Every answer tells the browser what the page may do (#139) --------------

/// The inline scripts a page carries, each as the `'sha256-…'` source a
/// Content-Security-Policy must name for the browser to run it.
///
/// A second copy of what `interface.rs` does, on purpose: this one reads the
/// page as it was served, so the test catches a policy worked out from a
/// different shell than the one the browser got.
fn inline_script_hashes(html: &str) -> Vec<String> {
    use base64::Engine;
    use sha2::Digest;
    let mut hashes = Vec::new();
    let mut rest = html;
    while let Some(open) = rest.find("<script") {
        let tag_end = open + rest[open..].find('>').expect("a closed tag");
        let close = tag_end + rest[tag_end..].find("</script>").expect("a closing tag");
        if !rest[open..tag_end].contains("src=") {
            let digest = sha2::Sha256::digest(&rest.as_bytes()[tag_end + 1..close]);
            hashes.push(format!(
                "'sha256-{}'",
                base64::engine::general_purpose::STANDARD.encode(digest)
            ));
        }
        rest = &rest[close..];
    }
    hashes
}

/// Content-Security-Policy, nosniff and a same-origin Referrer-Policy ride on
/// every kind of answer Kamosu gives, from one layer over the whole app. Each
/// backs up a defence that already holds (SameSite cookies, no raw HTML,
/// re-encoded pictures), for the day one of those slips. And the one inline
/// script the interface's shell carries is named by its hash, so the policy
/// that forbids inline script still lets the app start.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_answer_carries_the_browser_safety_headers() {
    let app = support::spawn_app();
    let (_person, key) = operator(&app);
    let (token, branch_id) = a_shared_recipe(&app, &key);
    let photograph = an_upload(&app, &key, 1);
    save_version(
        &app,
        &key,
        json!({ "branch_id": branch_id, "title": "Tarte aux pommes", "main_photo": photograph }),
    );

    let shell = kamosu::http_min::get(app.addr, "/").expect("the shell");
    let deep = kamosu::http_min::get(app.addr, "/recipes").expect("a screen");
    let share = kamosu::http_min::get(app.addr, &format!("/s/{token}")).expect("a Share Link");
    let operation =
        kamosu::http_min::post_json(app.addr, "/api/op/list_kitchens", Some(&key), "{}")
            .expect("an Operation");
    let picture = kamosu::http_min::get_with_bearer(
        app.addr,
        &format!("/api/photographs/{photograph}"),
        Some(&key),
    )
    .expect("a Photograph");
    let stylesheet = kamosu::http_min::get(app.addr, "/assets/app.css").expect("the stylesheet");

    for (what, reply) in [
        ("the interface's index.html", &shell),
        ("a screen served from the shell", &deep),
        ("a Share Link page", &share),
        ("an Operation's answer", &operation),
        ("a Photograph", &picture),
        ("the stylesheet", &stylesheet),
    ] {
        assert_eq!(reply.status, 200, "{what}: {}", reply.text());
        assert_eq!(
            reply.header("x-content-type-options"),
            Some("nosniff"),
            "{what}"
        );
        assert_eq!(
            reply.header("referrer-policy"),
            Some("same-origin"),
            "{what}"
        );
        let policy = reply
            .header("content-security-policy")
            .unwrap_or_else(|| panic!("{what} carries no Content-Security-Policy"));
        let directives: Vec<&str> = policy.split(';').map(str::trim).collect();
        for wanted in [
            "default-src 'self'",
            "object-src 'none'",
            "base-uri 'self'",
            "form-action 'self'",
            "frame-ancestors 'none'",
        ] {
            assert!(directives.contains(&wanted), "{what}: {policy}");
        }
        let scripts = directives
            .iter()
            .find(|d| d.starts_with("script-src "))
            .unwrap_or_else(|| panic!("{what}: no script-src in {policy}"));
        assert!(scripts.contains("'self'"), "{what}: {policy}");
        assert!(!scripts.contains("unsafe"), "{what}: {policy}");
    }

    // The shell's bootstrap is inline; without its hash in the policy the
    // browser would refuse it and the app would never start.
    let wanted = inline_script_hashes(&shell.text());
    assert!(!wanted.is_empty(), "the shell carries its bootstrap inline");
    let policy = shell.header("content-security-policy").unwrap();
    for hash in wanted {
        assert!(policy.contains(&hash), "{hash} missing from {policy}");
    }
}

// --- The MCP door answers no web page (#145) ---------------------------------

/// The set of accepted origins is empty, for the reasons `refuse_any_origin`
/// gives. A rebound page could reach little here, since the browser sends it
/// no Kamosu cookie, so this asks only that the door refuses, not that
/// anything was at stake.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_mcp_door_refuses_any_request_that_carries_an_origin() {
    let app = support::spawn_app();
    let (_, key) = operator(&app);
    let bearer = format!("Bearer {key}");

    let discover = r#"{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{}}"#;
    let notification = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    let initialize = r#"{"jsonrpc":"2.0","id":4,"method":"initialize","params":{}}"#;
    let create_tag = |id: i64, name: &str| {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": "create_tag", "arguments": { "language": "en", "name": name } },
        })
        .to_string()
    };

    // The positive controls first: without an Origin each is served, so the
    // refusals below are the header's doing and not a Door that refuses all.
    assert_eq!(app.post_mcp_with_headers(discover, &[]).0, 200);
    assert_eq!(app.post_mcp_with_headers(notification, &[]).0, 202);
    let (_, created) =
        app.post_mcp_with_headers(&create_tag(3, "control"), &[("Authorization", &bearer)]);
    assert_eq!(created["result"]["isError"], json!(false), "{created}");

    for (what, origin, body) in [
        ("a foreign page", "https://evil.example", discover),
        ("a sandboxed or file:// page", "null", discover),
        (
            "an Origin naming Kamosu's own address",
            "http://127.0.0.1:5266",
            discover,
        ),
        ("a notification", "https://evil.example", notification),
        ("a legacy initialize", "https://evil.example", initialize),
        ("an empty body", "https://evil.example", ""),
        (
            "a body that is not JSON",
            "https://evil.example",
            "{not json",
        ),
    ] {
        let (status, refused) = app.post_mcp_with_headers(body, &[("Origin", origin)]);
        assert_eq!(status, 403, "{what}: {refused}");
        assert_eq!(refused["error"]["code"], json!(-32600), "{what}: {refused}");
        assert!(
            refused.get("id").is_none_or(Value::is_null),
            "{what}: a refusal that read nothing names no id: {refused}"
        );
    }

    // A real Access Key does not carry a request past the check, and the
    // Operation it asked for never runs.
    let (status, refused) = app.post_mcp_with_headers(
        &create_tag(2, "rebound"),
        &[
            ("Authorization", &bearer),
            ("Origin", "https://evil.example"),
        ],
    );
    assert_eq!(status, 403, "{refused}");
    let (_, listed) = app.post_op("list_tags", Some(&key), "{}");
    let names: Vec<&str> = listed["result"]["tags"]
        .as_array()
        .expect("the Tags")
        .iter()
        .filter_map(|tag| tag["name"].as_str())
        .collect();
    assert_eq!(names, ["control"], "{listed}");
}
