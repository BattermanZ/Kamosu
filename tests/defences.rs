//! What Kamosu defends, asserted in one place (#79, ADR 0031–0034).
//!
//! Every guarantee here cuts across the whole program rather than belonging to
//! one feature, which is why none of them could be written until the features
//! existed. They are the properties an Operator is promised, checked against
//! the running instance through its real Doors.
//!
//! The companion to this file is prose: `README.md` says what Kamosu does
//! *not* defend, and that list is maintained alongside these tests. An entry
//! there that stops being true is worse than one that never existed
//! (ADR 0034).

mod support;

use serde_json::json;

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

    // A Share Link's token.
    let (token, _branch_id) = a_shared_recipe(app, key);
    secrets.push(Secret {
        what: "a Share Link",
        raw: token,
        table: "share_links",
    });

    secrets
}

/// A recipe in a Kitchen, shared, as a stranger would meet it. Answers the
/// Share Link's token and the Branch it names.
fn a_shared_recipe(app: &support::TestApp, key: &str) -> (String, String) {
    let (status, kitchen) = app.post_op(
        "create_kitchen",
        Some(key),
        &json!({ "name": "Sunday Kitchen" }).to_string(),
    );
    assert_eq!(status, 200, "{kitchen}");
    let kitchen_id = kitchen["result"]["id"].as_str().expect("a Kitchen");

    let (status, created) = app.post_op(
        "create_recipe",
        Some(key),
        &json!({ "kitchen_id": kitchen_id, "title": "Tarte aux pommes" }).to_string(),
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

/// ADR 0031's whole rule, checked against every Secret the instance can hand
/// out: 256 bits, kept only as a hash, answered exactly once, ended one at a
/// time, and never ended by a clock.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_secret_is_256_bits_hashed_at_rest_shown_once_and_on_no_clock() {
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
        // stores, so what it stores must not be the Secret itself.
        assert!(
            !stored.contains(&secret.raw),
            "{} is stored in the clear somewhere in the database",
            secret.what
        );
    }

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
/// is the whole answer to a leak (ADR 0031).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_secret_is_answered_once_and_never_read_back() {
    let app = support::spawn_app();
    let (_person, key) = operator(&app);
    let secrets = every_secret(&app, &key);

    for (operation, input) in [
        ("list_access_keys", json!({})),
        ("list_sessions", json!({})),
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

    // A Share Link is the same: its address is answered once at minting, and
    // afterwards only the fact that sharing is on.
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

/// A password is short and chosen by a person, so a wrong one slows the next
/// try. Every other Secret is 256 bits, which cannot be guessed, so throttling
/// one would defend a door with no handle while implying the size was not
/// enough. Nothing but the password path may slow down.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn throttling_exists_only_where_the_secret_is_human_chosen() {
    let app = support::spawn_app();
    let (_person, _key) = operator(&app);

    // Wrong passwords: the door slows. Two misses are free, then the delay
    // doubles from one second, so four wrong answers cost at least three
    // seconds in total.
    let wrong = json!({
        "name": "Aurélien",
        "password": "not the password",
        "session_name": "somewhere else",
    })
    .to_string();
    let started = std::time::Instant::now();
    for _ in 0..4 {
        assert_eq!(app.post_auth("/auth/login", &wrong).0, 401);
    }
    let slowed = started.elapsed();
    assert!(
        slowed >= std::time::Duration::from_secs(3),
        "wrong passwords must slow the door, took {slowed:?}"
    );

    // A correct password is always accepted. The door slows, it never shuts,
    // because a lockout is a weapon handed to whoever knows an account name.
    // The delay already accrued is still paid on the way through; what the
    // right password buys is that the *next* try is free again.
    let right = json!({
        "name": "Aurélien",
        "password": "a password only its person knows",
        "session_name": "laptop",
    })
    .to_string();
    assert_eq!(
        app.post_auth("/auth/login", &right).0,
        200,
        "wrong guesses must never lock the household out"
    );
    let started = std::time::Instant::now();
    assert_eq!(app.post_auth("/auth/login", &right).0, 200);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "a correct password clears the count: the next login must be free again"
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

// --- A stranger causes work, never work that scales (ADR 0032) ---------------

/// Work nobody signed in for runs one piece at a time behind a short line.
/// When the line is full the ask is refused outright, so the worst a stranger
/// can inflict is other strangers waiting. A member never queues behind them.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unauthenticated_job_work_runs_in_one_lane() {
    let app = support::spawn_app();
    let (_person, key) = operator(&app);

    let (token, branch_id) = a_shared_recipe(&app, &key);

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

/// The seven things Kamosu does not defend, shipped where the people who need
/// them will see them.
///
/// ADR 0034 says the list must be maintained, and an entry that stops being
/// true is worse than one that never existed. It now lives in two places a
/// reader meets, so this fails if one of them loses an item the other kept.
/// The wording differs between them on purpose, since a settings screen is
/// read standing up; what must not differ is what is on the list.
#[test]
fn the_honest_list_is_shipped_whole_in_both_places() {
    let readme = std::fs::read_to_string("README.md").expect("README.md");
    let messages = std::fs::read_to_string("ui/messages/en.json").expect("the English messages");

    let list = readme
        .split_once("## What Kamosu does not defend")
        .expect("the README ships the list")
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
            "an agent",
            "including by text it reads",
            "settings_agent_reads",
        ),
        ("an audit", "has not been audited", "settings_unaudited"),
    ] {
        assert!(
            list.contains(in_readme),
            "the README stopped naming {what} ({in_readme:?})"
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
