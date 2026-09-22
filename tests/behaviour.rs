//! Behaviour tests: they drive real Operations with a real Credential against a
//! real SQLite file in a temporary directory. No mocks, no stubs, no in-memory
//! doubles. They describe what an operator or an agent can do, not how Kamosu is
//! written.

mod support;

use kamosu::MCP_PROTOCOL_VERSION;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::{Duration, Instant};

// --- The Job shape (issue #34) -----------------------------------------------

/// Per-request capability declaration for the long-running-task extension.
fn tasks_meta() -> Value {
    json!({
        "_meta": {
            "io.modelcontextprotocol/clientCapabilities": {
                "extensions": {
                    "io.modelcontextprotocol/tasks": {},
                },
            },
        },
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn asking_for_a_job_returns_an_id_at_once_and_the_result_is_read_at_both_doors() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "browser session", false)
        .unwrap()
        .secret;

    // Asking answers immediately — long before the work it names has finished.
    let started = Instant::now();
    let (status, ask) = app.post_op("probe_job", Some(&key), r#"{"steps":30,"delay_ms":50}"#);
    assert_eq!(status, 200, "{ask}");
    assert_eq!(ask["ok"], json!(true), "{ask}");
    let job_id = ask["result"]["job_id"]
        .as_str()
        .expect("a job id")
        .to_string();
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "asking took too long: the work must answer with an id at once"
    );

    // Progress is visible while it runs: ticks arrive before the work ends.
    let get = json!({ "job_id": job_id }).to_string();
    let mut saw_progress = false;
    loop {
        let (_, body) = app.post_op("get_job", Some(&key), &get);
        let result = &body["result"];
        match result["status"].as_str().unwrap_or("") {
            "queued" | "running" => {
                if result["progress"]["done"].as_i64().unwrap_or(0) > 0 {
                    saw_progress = true;
                    assert!(result["operation"] == "probe_job");
                    break;
                }
            }
            _ => break,
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        saw_progress,
        "no progress was ever visible while the job ran"
    );

    // And the eventual result arrives through the same ordinary Operation.
    let finished = support::wait_terminal(&app, Some(&key), &job_id);
    assert_eq!(finished["status"], json!("completed"));
    assert_eq!(finished["result"]["steps"], json!(30));
    assert_eq!(finished["error"], Value::Null);

    // The MCP door reads the identical truth through tasks/get: asking for a
    // Job answers a CreateTaskResult whose taskId is the job id.
    let call = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "probe_job",
            "arguments": { "steps": 2 },
            "_meta": tasks_meta()["_meta"],
        },
    });
    let (status, created) = app.post_mcp(&call.to_string(), None);
    assert_eq!(status, 200, "{created}");
    let task = &created["result"];
    assert_eq!(task["resultType"], json!("task"), "{created}");
    let task_id = task["taskId"]
        .as_str()
        .expect("CreateTaskResult carries a taskId")
        .to_string();
    assert!(
        task["pollIntervalMs"].is_i64(),
        "clients are told how often to look back"
    );

    // A client that did not declare the extension is refused before any work is
    // caused: it could never see what it asked for.
    let bare_call = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": { "name": "probe_job", "arguments": {} },
    });
    let (_, refused) = app.post_mcp(&bare_call.to_string(), None);
    assert_eq!(refused["error"]["code"], json!(-32003), "{refused}");
    assert!(
        refused["error"]["data"]["requiredCapabilities"]["extensions"]
            .get("io.modelcontextprotocol/tasks")
            .is_some(),
        "the refusal names the capability it needs"
    );
    let (_, tasks_get_refused) = app.post_mcp(
        r#"{"jsonrpc":"2.0","id":2,"method":"tasks/get","params":{"taskId":"whatever"}}"#,
        None,
    );
    assert_eq!(tasks_get_refused["error"]["code"], json!(-32003));

    // Polling reaches completion, carrying the result in CallToolResult shape.
    let poll = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tasks/get",
        "params": {
            "taskId": task_id,
            "_meta": tasks_meta()["_meta"],
        },
    });
    for _ in 0..400 {
        let (_, body) = app.post_mcp(&poll.to_string(), None);
        let task = &body["result"];
        match task["status"].as_str().unwrap_or("") {
            "working" => std::thread::sleep(Duration::from_millis(25)),
            "completed" => {
                assert_eq!(task["result"]["isError"], json!(false), "{task}");
                assert_eq!(task["result"]["structuredContent"]["steps"], json!(2));
                break;
            }
            other => panic!("unexpected task state {other}: {task}"),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn instance_status_says_the_version_and_whether_setup_has_happened() {
    let app = support::spawn_app();

    let (status, body) = app.post_op("instance_status", None, "{}");
    assert_eq!(status, 200);
    let result = &body["result"];
    assert_eq!(result["version"], json!(env!("CARGO_PKG_VERSION")));
    // A fresh install has not been set up.
    assert_eq!(result["setup_complete"], json!(false));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_first_visitor_becomes_the_operator_with_a_home_kitchen_and_hand() {
    let app = support::spawn_app();
    let first = json!({
        "name": "Aurélien",
        "password": "a password only its person knows",
        "session_name": "test browser",
    });

    let (status, created) = app.post_auth("/auth/first-person", &first.to_string());
    assert_eq!(status, 200, "{created}");
    let person = &created["result"]["person"];
    assert_eq!(person["name"], json!("Aurélien"));
    assert!(person["id"].as_str().is_some_and(|id| id.starts_with("p_")));
    assert_eq!(person["hand_id"], person["id"]);
    assert!(
        person["home_kitchen_id"]
            .as_str()
            .is_some_and(|id| id.starts_with("k_"))
    );
    assert_eq!(person["reading_language"], json!("en"));
    assert_eq!(person["reading_measures"], json!("us"));
    assert_eq!(person["is_operator"], json!(true));

    // Becoming the Operator closes the first-visitor door forever: another
    // stranger cannot race an account onto a live instance.
    let (status, refused) = app.post_auth("/auth/first-person", &first.to_string());
    assert_eq!(status, 401, "{refused}");
    assert_eq!(refused["error"]["kind"], json!("unauthorized"));

    let (_, status) = app.post_op("instance_status", None, "{}");
    assert_eq!(status["result"]["setup_complete"], json!(true));
}

// --- Kitchens (issue #40) ----------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_person_alone_is_a_kitchen_of_one_with_its_own_hand() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "browser", false)
        .unwrap()
        .secret;

    let (status, listed) = app.post_op("list_kitchens", Some(&key), "{}");
    assert_eq!(status, 200, "{listed}");
    let kitchens = listed["result"]["kitchens"].as_array().expect("kitchens");
    assert_eq!(kitchens.len(), 1, "a Person alone is a Kitchen of one");
    let home = &kitchens[0];
    assert_eq!(home["is_home"], json!(true));
    assert_eq!(home["nickname"], json!(null));
    assert!(home["hand_id"].as_str().is_some(), "its own Hand");
    let members = home["members"].as_array().expect("members");
    assert_eq!(members.len(), 1);
    assert_eq!(members[0]["person_id"], json!(person));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn any_person_may_create_a_kitchen_named_by_its_creator() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "browser", false)
        .unwrap()
        .secret;

    let (status, created) = app.post_op(
        "create_kitchen",
        Some(&key),
        &json!({ "name": "Supper Club" }).to_string(),
    );
    assert_eq!(status, 200, "{created}");
    let kitchen = &created["result"];
    assert_eq!(kitchen["name"], json!("Supper Club"));
    assert_eq!(kitchen["is_home"], json!(false));
    let members = kitchen["members"].as_array().expect("members");
    assert_eq!(members.len(), 1);
    assert_eq!(members[0]["person_id"], json!(person));

    // Now cooking in more than one Kitchen: the never-asked-when-you-have-one
    // rule turns on exactly here, and list_kitchens is where a caller reads it.
    let (_, listed) = app.post_op("list_kitchens", Some(&key), "{}");
    assert_eq!(listed["result"]["kitchens"].as_array().unwrap().len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn any_member_may_rename_a_kitchen_and_set_their_own_private_nickname() {
    let app = support::spawn_app();
    let alice = app.core.create_person("Alice").expect("person");
    let alice_key = app
        .core
        .mint_access_key(&alice, "browser", false)
        .unwrap()
        .secret;
    let bob = app.core.create_person("Bob").expect("person");
    let bob_key = app
        .core
        .mint_access_key(&bob, "browser", false)
        .unwrap()
        .secret;

    let (_, created) = app.post_op(
        "create_kitchen",
        Some(&alice_key),
        &json!({ "name": "Home" }).to_string(),
    );
    let kitchen_id = created["result"]["id"].as_str().unwrap().to_string();

    // Bob is not a member yet — every write on this Kitchen refuses him.
    let (status, _) = app.post_op(
        "rename_kitchen",
        Some(&bob_key),
        &json!({ "kitchen_id": kitchen_id, "name": "Chez Bob" }).to_string(),
    );
    assert_eq!(status, 401);

    let invite = app.core.invite_to_kitchen(&alice, &kitchen_id).unwrap().1;
    let (status, joined) = app.post_op(
        "accept_kitchen_invite",
        Some(&bob_key),
        &json!({ "secret": invite }).to_string(),
    );
    assert_eq!(status, 200, "{joined}");
    assert_eq!(joined["result"]["members"].as_array().unwrap().len(), 2);

    // Any member may rename the shared Name — Bob included.
    let (status, renamed) = app.post_op(
        "rename_kitchen",
        Some(&bob_key),
        &json!({ "kitchen_id": kitchen_id, "name": "Chez Nous" }).to_string(),
    );
    assert_eq!(status, 200, "{renamed}");

    // A Nickname is seen only by the Person who set it.
    let (status, _) = app.post_op(
        "set_kitchen_nickname",
        Some(&bob_key),
        &json!({ "kitchen_id": kitchen_id, "nickname": "Chez Bob" }).to_string(),
    );
    assert_eq!(status, 200);

    let (_, bobs_view) = app.post_op("list_kitchens", Some(&bob_key), "{}");
    let bobs_kitchen = bobs_view["result"]["kitchens"]
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["id"] == json!(kitchen_id))
        .unwrap();
    assert_eq!(bobs_kitchen["name"], json!("Chez Nous"));
    assert_eq!(bobs_kitchen["nickname"], json!("Chez Bob"));

    let (_, alices_view) = app.post_op("list_kitchens", Some(&alice_key), "{}");
    let alices_kitchen = alices_view["result"]["kitchens"]
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["id"] == json!(kitchen_id))
        .unwrap();
    assert_eq!(alices_kitchen["name"], json!("Chez Nous"));
    assert_eq!(
        alices_kitchen["nickname"],
        json!(null),
        "Bob's Nickname is his alone"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_invite_is_spent_on_first_use_and_refused_afterwards() {
    let app = support::spawn_app();
    let alice = app.core.create_person("Alice").expect("person");
    let alice_key = app
        .core
        .mint_access_key(&alice, "browser", false)
        .unwrap()
        .secret;
    let bob = app.core.create_person("Bob").expect("person");
    let bob_key = app
        .core
        .mint_access_key(&bob, "browser", false)
        .unwrap()
        .secret;
    let carol = app.core.create_person("Carol").expect("person");
    let carol_key = app
        .core
        .mint_access_key(&carol, "browser", false)
        .unwrap()
        .secret;

    let (_, created) = app.post_op(
        "create_kitchen",
        Some(&alice_key),
        &json!({ "name": "Home" }).to_string(),
    );
    let kitchen_id = created["result"]["id"].as_str().unwrap().to_string();
    let (_, minted) = app.post_op(
        "invite_to_kitchen",
        Some(&alice_key),
        &json!({ "kitchen_id": kitchen_id }).to_string(),
    );
    let secret = minted["result"]["secret"].as_str().unwrap().to_string();

    assert_eq!(
        app.post_op(
            "accept_kitchen_invite",
            Some(&bob_key),
            &json!({ "secret": secret }).to_string(),
        )
        .0,
        200
    );

    let (status, refused) = app.post_op(
        "accept_kitchen_invite",
        Some(&carol_key),
        &json!({ "secret": secret }).to_string(),
    );
    assert_eq!(status, 401, "{refused}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn leaving_is_not_a_deletion_and_the_last_member_cannot_be_removed() {
    let app = support::spawn_app();
    let alice = app.core.create_person("Alice").expect("person");
    let alice_key = app
        .core
        .mint_access_key(&alice, "browser", false)
        .unwrap()
        .secret;
    let bob = app.core.create_person("Bob").expect("person");
    let bob_key = app
        .core
        .mint_access_key(&bob, "browser", false)
        .unwrap()
        .secret;

    let (_, created) = app.post_op(
        "create_kitchen",
        Some(&alice_key),
        &json!({ "name": "Home" }).to_string(),
    );
    let kitchen_id = created["result"]["id"].as_str().unwrap().to_string();
    let invite = app.core.invite_to_kitchen(&alice, &kitchen_id).unwrap().1;
    app.core.accept_kitchen_invite(&bob, &invite).unwrap();

    // Bob leaves by removing himself. His own Home Kitchen is untouched, and
    // this shared Kitchen still stands with Alice in it.
    let (status, left) = app.post_op(
        "remove_kitchen_member",
        Some(&bob_key),
        &json!({ "kitchen_id": kitchen_id, "person_id": bob }).to_string(),
    );
    assert_eq!(status, 200, "{left}");
    let (_, remaining) = app.post_op("list_kitchens", Some(&alice_key), "{}");
    let shared = remaining["result"]["kitchens"]
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["id"] == json!(kitchen_id))
        .unwrap();
    assert_eq!(shared["members"].as_array().unwrap().len(), 1);

    // Alice is now the last member of this Kitchen — she cannot be removed
    // from it, by herself or anyone else.
    let (status, refused) = app.post_op(
        "remove_kitchen_member",
        Some(&alice_key),
        &json!({ "kitchen_id": kitchen_id, "person_id": alice }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_person_cannot_be_removed_from_their_own_last_kitchen() {
    let app = support::spawn_app();
    let alice = app.core.create_person("Alice").expect("person");
    let alice_key = app
        .core
        .mint_access_key(&alice, "browser", false)
        .unwrap()
        .secret;
    let bob = app.core.create_person("Bob").expect("person");

    // Bob's Home Kitchen is his only Kitchen; Alice invites him into hers, then
    // he is asked to leave the shared one — untouched, that leaves him with
    // his Home Kitchen alone, which is fine. But nobody may strip him of the
    // last Kitchen he cooks in at all: his own Home Kitchen.
    let (_, created) = app.post_op(
        "create_kitchen",
        Some(&alice_key),
        &json!({ "name": "Home" }).to_string(),
    );
    let kitchen_id = created["result"]["id"].as_str().unwrap().to_string();
    let bobs_home_kitchen: String = app
        .core
        .list_kitchens(&bob)
        .unwrap()
        .into_iter()
        .next()
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    app.core
        .invite_to_kitchen(&alice, &bobs_home_kitchen)
        .expect_err("only a member may invite into a Kitchen");

    // Add Bob to Alice's Kitchen directly (test plumbing), then try to strip
    // him of his Home Kitchen — his last remaining one.
    let invite = app.core.invite_to_kitchen(&alice, &kitchen_id).unwrap().1;
    app.core.accept_kitchen_invite(&bob, &invite).unwrap();
    app.core
        .remove_kitchen_member(&alice, &kitchen_id, &bob)
        .expect("Bob still cooks in his Home Kitchen after leaving this one");

    let bob_key = app
        .core
        .mint_access_key(&bob, "browser", false)
        .unwrap()
        .secret;
    let (status, refused) = app.post_op(
        "remove_kitchen_member",
        Some(&bob_key),
        &json!({ "kitchen_id": bobs_home_kitchen, "person_id": bob }).to_string(),
    );
    assert_eq!(
        status, 400,
        "a Person cooks in one or more Kitchens and cannot be stripped of the last one: {refused}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_operator_may_delete_a_kitchen_nobody_is_left_in_and_nothing_else() {
    let app = support::spawn_app();
    let first = json!({ "name": "Aurélien", "password": "a password only its person knows", "session_name": "test browser" });
    let (_, created) = app.post_auth("/auth/first-person", &first.to_string());
    let operator_id = created["result"]["person"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let operator_key = app
        .core
        .mint_access_key(&operator_id, "agent", false)
        .unwrap()
        .secret;

    let stranger = app.core.create_person("Marie").expect("person");
    let stranger_key = app
        .core
        .mint_access_key(&stranger, "browser", false)
        .unwrap()
        .secret;

    let (_, created) = app.post_op(
        "create_kitchen",
        Some(&stranger_key),
        &json!({ "name": "Supper Club" }).to_string(),
    );
    let kitchen_id = created["result"]["id"].as_str().unwrap().to_string();

    // A Kitchen with a member in it refuses deletion, even for an Operator.
    let (status, refused) = app.post_op(
        "delete_kitchen",
        Some(&operator_key),
        &json!({ "kitchen_id": kitchen_id }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");

    // A Kitchen reaches zero members only the way account deletion (issue #39)
    // will empty one: not through `remove_kitchen_member`, which refuses to
    // strip a Kitchen's last member. Test plumbing stands in for that cascade.
    app.core
        .db()
        .with_conn(|conn| {
            conn.execute(
                "DELETE FROM kitchen_members WHERE kitchen_id = ?1",
                rusqlite::params![kitchen_id],
            )
            .map(|_| ())
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .expect("empty the Kitchen directly");

    // A non-Operator may never delete a Kitchen, even an empty one.
    let (status, refused) = app.post_op(
        "delete_kitchen",
        Some(&stranger_key),
        &json!({ "kitchen_id": kitchen_id }).to_string(),
    );
    assert_eq!(status, 401, "{refused}");

    // Now nobody is left in it, and an Operator may delete it.
    let (status, deleted) = app.post_op(
        "delete_kitchen",
        Some(&operator_key),
        &json!({ "kitchen_id": kitchen_id }).to_string(),
    );
    assert_eq!(status, 200, "{deleted}");
    assert_eq!(deleted["result"]["deleted"], json!(true));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn logging_in_mints_a_revocable_session_credential() {
    let app = support::spawn_app();
    let create = json!({ "name": "Aurélien", "password": "the right password", "session_name": "first browser" });
    assert_eq!(
        app.post_auth("/auth/first-person", &create.to_string()).0,
        200
    );

    let login =
        json!({ "name": "Aurélien", "password": "the right password", "session_name": "laptop" });
    let logged_in = app.post_auth_response("/auth/login", &login.to_string());
    assert_eq!(logged_in.status, 200, "{}", logged_in.text());
    let session_id: Value = serde_json::from_str(&logged_in.text()).expect("auth envelope");
    let session_id = session_id["result"]["session_id"]
        .as_str()
        .expect("Session id");
    let secret = support::session_cookie_secret(&logged_in);
    let secret = secret.as_str();
    assert_eq!(app.post_op("list_jobs", Some(secret), "{}").0, 200);
    let (status, revoked) = app.post_op(
        "revoke_session",
        Some(secret),
        &json!({ "session_id": session_id }).to_string(),
    );
    assert_eq!(status, 200, "{revoked}");
    assert_eq!(app.post_op("list_jobs", Some(secret), "{}").0, 401);
}

/// Every `Set-Cookie` on one answer.
fn set_cookies(headers: &[(String, String)]) -> Vec<&str> {
    headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("set-cookie"))
        .map(|(_, value)| value.as_str())
        .collect()
}

/// A Session that has ended takes its cookie with it (#91).
///
/// Ending a Session revoked it on the server and left the browser holding the
/// cookie, and holding it was what made even a Public Operation come back
/// refused — so the sign-in screen asked the one question it needs answered,
/// was refused, and waited forever. The Door now takes back the cookie it set.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_ended_session_takes_its_cookie_with_it() {
    let app = support::spawn_app();
    let create = json!({ "name": "Aurélien", "password": "the right password", "session_name": "this browser" });
    let signed_in = app.post_auth_response("/auth/first-person", &create.to_string());
    assert_eq!(signed_in.status, 200, "{}", signed_in.text());
    let envelope: Value = serde_json::from_str(&signed_in.text()).expect("auth envelope");
    let session_id = envelope["result"]["session_id"]
        .as_str()
        .expect("Session id")
        .to_string();
    let secret = support::session_cookie_secret(&signed_in);
    let cookie = format!("kamosu_session={secret}");

    // While the Session lives, the cookie is a Credential like any other and
    // nothing about it is taken back.
    let (status, headers, body) =
        app.post_op_with_headers_reply("instance_status", &[("Cookie", &cookie)], "{}");
    assert_eq!(status, 200, "{body}");
    assert!(
        set_cookies(&headers).is_empty(),
        "a living Session's cookie is left alone: {headers:?}"
    );

    let (status, revoked) = app.post_op(
        "revoke_session",
        Some(&secret),
        &json!({ "session_id": session_id }).to_string(),
    );
    assert_eq!(status, 200, "{revoked}");

    // The sign-in screen's first question, asked by a browser still holding the
    // dead cookie. Refused — and the refusal expires the cookie.
    let (status, headers, body) =
        app.post_op_with_headers_reply("instance_status", &[("Cookie", &cookie)], "{}");
    assert_eq!(status, 401, "{body}");
    let expired = set_cookies(&headers);
    assert_eq!(expired.len(), 1, "exactly one Set-Cookie: {headers:?}");
    let expired = expired[0];
    assert!(
        expired.starts_with("kamosu_session=;"),
        "the cookie is emptied: {expired}"
    );
    assert!(expired.contains("Max-Age=0"), "and expired: {expired}");
    // Matching the attributes it was set with, or the browser keeps the
    // original and the fault survives while looking fixed.
    for attribute in ["Path=/", "HttpOnly", "Secure", "SameSite=Lax"] {
        assert!(
            expired.contains(attribute),
            "the expiring cookie must carry {attribute}: {expired}"
        );
    }

    // Which is the whole point: the next request arrives as a stranger, and the
    // sign-in screen learns what it came to learn.
    let (status, stranger) = app.post_op("instance_status", None, "{}");
    assert_eq!(status, 200, "{stranger}");
    assert_eq!(stranger["result"]["setup_complete"], json!(true));

    // The Core's rule is untouched. A dead Secret still names nobody, and an
    // Operation that needs a Person is still never performed for it.
    let (status, refused) = app.post_op_with_headers("list_jobs", &[("Cookie", &cookie)], "{}");
    assert_eq!(status, 401, "{refused}");
    assert_eq!(refused["error"]["kind"], json!("unauthorized"));

    // A bearer token is not this Door's to take back, so a refused one expires
    // no cookie.
    let (status, headers, body) = app.post_op_with_headers_reply(
        "instance_status",
        &[("Authorization", &format!("Bearer {secret}"))],
        "{}",
    );
    assert_eq!(status, 401, "{body}");
    assert!(
        set_cookies(&headers).is_empty(),
        "a refused bearer token must expire no cookie: {headers:?}"
    );
}

/// The Door takes back the Secret that was refused, and no other (#91).
///
/// A bearer token wins over the cookie when a request carries both, so a
/// refused bearer token says nothing about the cookie sitting beside it. Were
/// the Door to go by "a cookie was present" rather than "the cookie was what
/// the Core was handed", one dead Access Key would sign a perfectly good
/// browser Session out.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_refused_bearer_token_leaves_a_living_session_cookie_alone() {
    let app = support::spawn_app();
    let create = json!({ "name": "Aurélien", "password": "the right password", "session_name": "this browser" });
    let signed_in = app.post_auth_response("/auth/first-person", &create.to_string());
    assert_eq!(signed_in.status, 200, "{}", signed_in.text());
    let cookie = format!(
        "kamosu_session={}",
        support::session_cookie_secret(&signed_in)
    );

    // Both presented at once: a Secret that names nobody as the bearer token,
    // and the living Session as the cookie.
    let (status, headers, body) = app.post_op_with_headers_reply(
        "instance_status",
        &[
            ("Authorization", "Bearer a-secret-that-names-nobody"),
            ("Cookie", &cookie),
        ],
        "{}",
    );
    assert_eq!(status, 401, "{body}");
    assert!(
        set_cookies(&headers).is_empty(),
        "the bearer token was refused, so the cookie is not the Door's to take: {headers:?}"
    );

    // And the Session is still a Session.
    let (status, body) = app.post_op_with_headers("list_jobs", &[("Cookie", &cookie)], "{}");
    assert_eq!(status, 200, "{body}");
}

/// The same for the other two ways a browser's Person stops being one: the
/// account is disabled, or it is deleted. Both revoke every Session, so both
/// leave a browser holding a cookie that names nobody.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_disabled_or_deleted_persons_browser_is_let_go_of_too() {
    let app = support::spawn_app();
    let create =
        json!({ "name": "Aurélien", "password": "the right password", "session_name": "operator" });
    let operator = app.post_auth_response("/auth/first-person", &create.to_string());
    assert_eq!(operator.status, 200, "{}", operator.text());
    let operator_secret = support::session_cookie_secret(&operator);

    for (who, ending) in [("Camille", "disable_account"), ("Robin", "delete_account")] {
        let (status, minted) = app.post_op("mint_invite", Some(&operator_secret), "{}");
        assert_eq!(status, 200, "{minted}");
        let link = minted["result"]["link"].as_str().expect("an Invite link");

        let joined = app.post_auth_response(
            "/auth/invite",
            &json!({ "link": link, "name": who, "password": "their own password",
                     "session_name": "their phone" })
            .to_string(),
        );
        assert_eq!(joined.status, 200, "{}", joined.text());
        let cookie = format!("kamosu_session={}", support::session_cookie_secret(&joined));

        let (status, ended) = app.post_op(
            ending,
            Some(&operator_secret),
            &json!({ "name": who }).to_string(),
        );
        assert_eq!(status, 200, "{ended}");

        let (status, headers, body) =
            app.post_op_with_headers_reply("instance_status", &[("Cookie", &cookie)], "{}");
        assert_eq!(status, 401, "{who} after {ending}: {body}");
        let expired = set_cookies(&headers);
        assert_eq!(expired.len(), 1, "{who} after {ending}: {headers:?}");
        assert!(
            expired[0].starts_with("kamosu_session=;") && expired[0].contains("Max-Age=0"),
            "{who} after {ending}: {}",
            expired[0]
        );
    }
}

// --- Lineage, Branch, Version (issue #42) ------------------------------------

/// A Person, their Access Key, and a Kitchen they belong to — the setup every
/// recipe test starts from.
fn person_with_kitchen(app: &support::TestApp, name: &str) -> (String, String, String) {
    let person = app.core.create_person(name).expect("person");
    let key = app
        .core
        .mint_access_key(&person, "browser", false)
        .unwrap()
        .secret;
    let (_, created) = app.post_op(
        "create_kitchen",
        Some(&key),
        &json!({ "name": format!("{name}'s Kitchen") }).to_string(),
    );
    let kitchen_id = created["result"]["id"].as_str().unwrap().to_string();
    (person, key, kitchen_id)
}

/// A Person's Home Kitchen — the one `create_person` seats them in. Saving a
/// recipe into a *second* Kitchen the same Person cooks in is what makes a
/// Copy, so every test that needs two Branches of one Lineage needs this one
/// too.
fn home_kitchen_of(app: &support::TestApp, person_id: &str) -> String {
    app.core
        .db()
        .with_conn(|conn| {
            conn.query_row(
                "SELECT home_kitchen_id FROM people WHERE id = ?1",
                rusqlite::params![person_id],
                |row| row.get(0),
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .unwrap()
}

/// Push a Branch's current head further into the past, so the next save
/// falls outside the collapse window instead of being read as a rapid
/// re-save. The one place these tests *write* past Operations into the Core's
/// own store — there is no clock to fast-forward otherwise.
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn creating_a_recipe_needs_only_a_title_and_produces_a_lineage_branch_and_first_version() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    let (status, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Tarte aux pommes" }).to_string(),
    );
    assert_eq!(status, 200, "{created}");
    let recipe = &created["result"];
    assert_eq!(recipe["kitchen_id"], json!(kitchen_id));
    let branch_id = recipe["branch_id"].as_str().unwrap().to_string();
    let lineage_id = recipe["lineage_id"].as_str().unwrap();
    assert!(lineage_id.starts_with("l_"));
    assert!(branch_id.starts_with("b_"));
    assert!(
        recipe["head_version_id"]
            .as_str()
            .unwrap()
            .starts_with("v_")
    );

    let versions = recipe["versions"].as_array().unwrap();
    assert_eq!(versions.len(), 1, "exactly one first Version");
    assert_eq!(versions[0]["version_id"], recipe["head_version_id"]);
    assert_eq!(versions[0]["parent_version_id"], json!(null));
    assert_eq!(versions[0]["content"]["title"], json!("Tarte aux pommes"));
    assert_eq!(versions[0]["name"], json!(null));

    // Recorded locally for its author to read (CONTEXT.md, "Version") — but
    // never surfaced through get_recipe, since it never travels off-instance.
    let access_key_id: Option<String> = app
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row(
                "SELECT access_key_id FROM branch_versions WHERE branch_id = ?1",
                rusqlite::params![branch_id],
                |row| row.get(0),
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .unwrap();
    assert!(access_key_id.unwrap().starts_with("ak_"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn identical_text_on_two_instances_produces_the_same_fingerprint() {
    // "Two instances" — two entirely separate databases, never having
    // communicated, that both happen to be handed the identical title.
    let app_a = support::spawn_app();
    let app_b = support::spawn_app();
    let (_, key_a, kitchen_a) = person_with_kitchen(&app_a, "Aurélien");
    let (_, key_b, kitchen_b) = person_with_kitchen(&app_b, "Marc");

    let (_, created_a) = app_a.post_op(
        "create_recipe",
        Some(&key_a),
        &json!({ "kitchen_id": kitchen_a, "title": "Ratatouille" }).to_string(),
    );
    let (_, created_b) = app_b.post_op(
        "create_recipe",
        Some(&key_b),
        &json!({ "kitchen_id": kitchen_b, "title": "Ratatouille" }).to_string(),
    );

    assert_eq!(
        created_a["result"]["head_version_id"], created_b["result"]["head_version_id"],
        "identical text must fingerprint to the same Version id, unprompted"
    );
    // But the Lineage and Branch each instance minted are its own.
    assert_ne!(
        created_a["result"]["lineage_id"],
        created_b["result"]["lineage_id"]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn renaming_a_version_never_changes_its_identity_hand_or_parent() {
    let app = support::spawn_app();
    let (person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Soupe" }).to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();
    let version_id = created["result"]["head_version_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, renamed) = app.post_op(
        "rename_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "sequence": 1, "name": "Original" }).to_string(),
    );
    assert_eq!(status, 200, "{renamed}");
    assert_eq!(renamed["result"]["name"], json!("Original"));

    let (_, read_back) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let version = &read_back["result"]["versions"][0];
    assert_eq!(
        version["version_id"],
        json!(version_id),
        "renaming does not mint a new Version"
    );
    assert_eq!(version["name"], json!("Original"));
    assert_eq!(version["hand_id"], json!(person));
    assert_eq!(version["parent_version_id"], json!(null));

    // Clears back to unnamed.
    let (_, cleared) = app.post_op(
        "rename_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "sequence": 1, "name": null }).to_string(),
    );
    assert_eq!(cleared["result"]["name"], json!(null));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_recurring_version_id_is_named_per_occurrence_not_globally() {
    // The same content can land on a Branch more than once — save something
    // else, then save back to the exact original words. Both occurrences
    // share a Version id (it is a pure content fingerprint), but each is its
    // own row in the chain, and each must be nameable on its own.
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Soupe" }).to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();
    let original_version = created["result"]["head_version_id"]
        .as_str()
        .unwrap()
        .to_string();

    backdate_branch_head(&app, &branch_id);
    app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "title": "Soupe froide" }).to_string(),
    );

    backdate_branch_head(&app, &branch_id);
    let (_, reverted) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "title": "Soupe" }).to_string(),
    );
    assert_eq!(
        reverted["result"]["version_id"],
        json!(original_version),
        "reverting to the exact original words reproduces the same fingerprint"
    );
    let recurrence_sequence = reverted["result"]["sequence"].as_i64().unwrap();
    assert_eq!(recurrence_sequence, 3);

    app.post_op(
        "rename_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "sequence": 1, "name": "First take" }).to_string(),
    );
    app.post_op(
        "rename_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "sequence": recurrence_sequence, "name": "Back to basics" })
            .to_string(),
    );

    let (_, read_back) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let versions = read_back["result"]["versions"].as_array().unwrap();
    let first = versions.iter().find(|v| v["sequence"] == json!(1)).unwrap();
    let third = versions
        .iter()
        .find(|v| v["sequence"] == json!(recurrence_sequence))
        .unwrap();
    assert_eq!(
        first["version_id"], third["version_id"],
        "same content, same fingerprint"
    );
    assert_eq!(
        first["name"],
        json!("First take"),
        "renaming one occurrence must not rename the other"
    );
    assert_eq!(third["name"], json!("Back to basics"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rapid_re_saves_collapse_and_history_stays_append_only() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Tarte" }).to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();
    let first_version = created["result"]["head_version_id"]
        .as_str()
        .unwrap()
        .to_string();

    // A save identical to what is already there mints nothing.
    let (_, no_op) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "title": "Tarte" }).to_string(),
    );
    assert_eq!(no_op["result"]["version_id"], json!(first_version));
    assert_eq!(no_op["result"]["collapsed"], json!(false));

    // The first genuine edit falls outside the (backdated) collapse window,
    // so it is its own Version.
    backdate_branch_head(&app, &branch_id);
    let (_, edited) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "title": "Tarte aux pomme" }).to_string(),
    );
    assert_eq!(edited["result"]["collapsed"], json!(false));
    let second_version = edited["result"]["version_id"].as_str().unwrap().to_string();
    assert_ne!(second_version, first_version);
    assert_eq!(edited["result"]["parent_version_id"], json!(first_version));

    // Two rapid typo fixes right after it collapse into the Version already
    // being shaped — the fingerprint changes, but no new chain entry appears.
    let (_, fix_one) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "title": "Tarte aux pommes" }).to_string(),
    );
    assert_eq!(fix_one["result"]["collapsed"], json!(true));
    assert_eq!(fix_one["result"]["parent_version_id"], json!(first_version));

    let (_, fix_two) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "title": "Tarte aux pommes au four", "name": "Weeknight version" })
            .to_string(),
    );
    assert_eq!(fix_two["result"]["collapsed"], json!(true));
    let third_version = fix_two["result"]["version_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(third_version, second_version);

    // Falling outside the window again starts a genuinely new Version.
    backdate_branch_head(&app, &branch_id);
    let (_, final_edit) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "title": "Tarte aux pommes au four, pâte brisée" })
            .to_string(),
    );
    assert_eq!(final_edit["result"]["collapsed"], json!(false));
    assert_eq!(
        final_edit["result"]["parent_version_id"],
        json!(third_version)
    );
    let fourth_version = final_edit["result"]["version_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Exactly three entries ever became part of the Thread: the first
    // Version, the collapsed edit (however many saves shaped it), and the
    // final one — the two typo-fix saves left no trace of their own.
    let (_, read_back) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let versions = read_back["result"]["versions"].as_array().unwrap();
    let ids: Vec<&str> = versions
        .iter()
        .map(|v| v["version_id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        vec![
            first_version.as_str(),
            third_version.as_str(),
            fourth_version.as_str()
        ],
        "rapid re-saves collapsed; nothing else was ever deleted or rewritten"
    );
    assert_eq!(versions[1]["name"], json!("Weeknight version"));
    assert_eq!(
        versions[1]["content"]["title"],
        json!("Tarte aux pommes au four")
    );

    // Append-only: the first Version's content is exactly what it always was.
    assert_eq!(versions[0]["content"]["title"], json!("Tarte"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn only_a_kitchen_member_may_create_or_read_its_recipes() {
    let app = support::spawn_app();
    let (_owner, owner_key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let stranger = app.core.create_person("Marc").expect("person");
    let stranger_key = app
        .core
        .mint_access_key(&stranger, "browser", false)
        .unwrap()
        .secret;

    let (status, _) = app.post_op(
        "create_recipe",
        Some(&stranger_key),
        &json!({ "kitchen_id": kitchen_id, "title": "Not yours" }).to_string(),
    );
    assert_eq!(status, 401);

    let (_, created) = app.post_op(
        "create_recipe",
        Some(&owner_key),
        &json!({ "kitchen_id": kitchen_id, "title": "Soupe" }).to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    // A recipe held by a Kitchen the caller does not cook in answers exactly as
    // a Branch id this instance has never held does (#97, ADR 0040) — the case
    // still matters, only the answer has moved.
    let (status, refused) = app.post_op(
        "get_recipe",
        Some(&stranger_key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 404, "{refused}");
    let (absent_status, absent) = app.post_op(
        "get_recipe",
        Some(&stranger_key),
        &json!({ "branch_id": "b_ffffffffffffffff" }).to_string(),
    );
    assert_eq!((absent_status, absent), (status, refused));
}

// --- Copy (issue #54) ---------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn editing_a_recipe_your_kitchen_holds_writes_an_ordinary_version() {
    // The unremarkable case, stated as its own acceptance criterion: a save
    // by a member of the Branch's own Kitchen is never a Copy.
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Soupe" }).to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    backdate_branch_head(&app, &branch_id);
    let (status, saved) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "title": "Soupe au pistou" }).to_string(),
    );
    assert_eq!(status, 200, "{saved}");
    assert_eq!(saved["result"]["copied"], json!(false));
    assert_eq!(
        saved["result"]["branch_id"],
        json!(branch_id),
        "an ordinary edit stays on the same Branch"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn editing_a_recipe_your_kitchen_did_not_write_starts_a_copy() {
    let app = support::spawn_app();
    let (owner, owner_key, owner_kitchen) = person_with_kitchen(&app, "Aurélien");
    // Marc's only Kitchen is the Home Kitchen create_person gives him — so
    // the default (no kitchen_id said) lands the Copy there.
    let copier = app.core.create_person("Marc").expect("person");
    let copier_key = app
        .core
        .mint_access_key(&copier, "browser", false)
        .unwrap()
        .secret;
    let copier_kitchen = home_kitchen_of(&app, &copier);

    let (_, created) = app.post_op(
        "create_recipe",
        Some(&owner_key),
        &json!({ "kitchen_id": owner_kitchen, "title": "Soupe" }).to_string(),
    );
    let source_branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();
    let lineage_id = created["result"]["lineage_id"]
        .as_str()
        .unwrap()
        .to_string();
    let first_version = created["result"]["head_version_id"]
        .as_str()
        .unwrap()
        .to_string();

    backdate_branch_head(&app, &source_branch_id);
    let (_, edited) = app.post_op(
        "save_recipe_version",
        Some(&owner_key),
        &json!({ "branch_id": source_branch_id, "title": "Soupe au pistou" }).to_string(),
    );
    let source_head = edited["result"]["version_id"].as_str().unwrap().to_string();

    // Marc's Kitchen has never held this Branch. Changing it is a Copy: it
    // starts Marc's own Branch of the same Lineage, at the Version he
    // changed, rather than writing onto Aurélien's.
    let (status, copied) = app.post_op(
        "save_recipe_version",
        Some(&copier_key),
        &json!({ "branch_id": source_branch_id, "title": "Soupe au pistou, sans ail" }).to_string(),
    );
    assert_eq!(status, 200, "{copied}");
    assert_eq!(copied["result"]["copied"], json!(true));
    let new_branch_id = copied["result"]["branch_id"].as_str().unwrap().to_string();
    assert_ne!(
        new_branch_id, source_branch_id,
        "a Copy carries your Kitchen's Hand and a fresh Branch id"
    );
    assert_eq!(
        copied["result"]["parent_version_id"],
        json!(source_head),
        "starts at the Version being changed"
    );

    let (_, read_back) = app.post_op(
        "get_recipe",
        Some(&copier_key),
        &json!({ "branch_id": new_branch_id }).to_string(),
    );
    let recipe = &read_back["result"];
    assert_eq!(recipe["lineage_id"], json!(lineage_id), "the same Lineage");
    assert_eq!(
        recipe["kitchen_id"],
        json!(copier_kitchen),
        "held by your Kitchen"
    );

    let copier_kitchen_hand: String = app
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row(
                "SELECT hand_id FROM kitchens WHERE id = ?1",
                rusqlite::params![copier_kitchen],
                |row| row.get(0),
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .unwrap();
    assert_eq!(
        recipe["hand_id"],
        json!(copier_kitchen_hand),
        "the new Branch carries your Kitchen's Hand"
    );

    let versions = recipe["versions"].as_array().unwrap();
    assert_eq!(
        versions.len(),
        3,
        "the whole chain behind it, nothing truncated"
    );
    assert_eq!(
        versions[0]["version_id"],
        json!(first_version),
        "the copied chain reaches the first Version"
    );
    assert_eq!(
        versions[0]["hand_id"],
        json!(owner),
        "original authorship is carried across exactly as it was"
    );
    assert_eq!(versions[1]["version_id"], json!(source_head));
    assert_eq!(versions[1]["hand_id"], json!(owner));
    assert_eq!(versions[2]["hand_id"], json!(copier));
    assert_eq!(
        versions[2]["content"]["title"],
        json!("Soupe au pistou, sans ail")
    );

    // The source Branch, in Aurélien's Kitchen, is left exactly as it was.
    let (_, source_read) = app.post_op(
        "get_recipe",
        Some(&owner_key),
        &json!({ "branch_id": source_branch_id }).to_string(),
    );
    assert_eq!(
        source_read["result"]["versions"].as_array().unwrap().len(),
        2,
        "a Copy never touches the Branch it started from"
    );
    assert_eq!(source_read["result"]["head_version_id"], json!(source_head));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn saving_unchanged_content_from_another_kitchen_starts_no_copy() {
    // Merely receiving or viewing a recipe must never create a Branch — and
    // neither must a "save" that changes nothing, even from a Kitchen that
    // has never held this Branch (CONTEXT.md, "Copy": "It happens at the
    // moment of the change, never at the moment of receipt").
    let app = support::spawn_app();
    let (_owner, owner_key, owner_kitchen) = person_with_kitchen(&app, "Aurélien");
    let (_copier, copier_key, copier_kitchen) = person_with_kitchen(&app, "Marc");

    let (_, created) = app.post_op(
        "create_recipe",
        Some(&owner_key),
        &json!({ "kitchen_id": owner_kitchen, "title": "Soupe" }).to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    let (status, unchanged) = app.post_op(
        "save_recipe_version",
        Some(&copier_key),
        &json!({ "branch_id": branch_id, "title": "Soupe" }).to_string(),
    );
    assert_eq!(status, 200, "{unchanged}");
    assert_eq!(unchanged["result"]["copied"], json!(false));
    assert_eq!(unchanged["result"]["branch_id"], json!(branch_id));

    let branches_in_copier_kitchen: i64 = app
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM branches WHERE kitchen_id = ?1",
                rusqlite::params![copier_kitchen],
                |row| row.get(0),
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .unwrap();
    assert_eq!(
        branches_in_copier_kitchen, 0,
        "no Branch was started in the reader's Kitchen"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_copy_may_be_held_by_a_kitchen_named_explicitly() {
    // "A question only ever put to someone who cooks in more than one"
    // (CONTEXT.md, "Home Kitchen") — the default is the Home Kitchen, but a
    // Person cooking in several may say which one holds the Copy.
    let app = support::spawn_app();
    let (_owner, owner_key, owner_kitchen) = person_with_kitchen(&app, "Aurélien");
    let copier = app.core.create_person("Marc").expect("person");
    let copier_key = app
        .core
        .mint_access_key(&copier, "browser", false)
        .unwrap()
        .secret;
    let (_, second_kitchen) = app.post_op(
        "create_kitchen",
        Some(&copier_key),
        &json!({ "name": "Marc's Other Kitchen" }).to_string(),
    );
    let second_kitchen_id = second_kitchen["result"]["id"].as_str().unwrap().to_string();

    let (_, created) = app.post_op(
        "create_recipe",
        Some(&owner_key),
        &json!({ "kitchen_id": owner_kitchen, "title": "Soupe" }).to_string(),
    );
    let source_branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    let (status, copied) = app.post_op(
        "save_recipe_version",
        Some(&copier_key),
        &json!({
            "branch_id": source_branch_id,
            "title": "Soupe, ma version",
            "kitchen_id": second_kitchen_id,
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{copied}");
    assert_eq!(copied["result"]["copied"], json!(true));
    let new_branch_id = copied["result"]["branch_id"].as_str().unwrap().to_string();

    let (_, read_back) = app.post_op(
        "get_recipe",
        Some(&copier_key),
        &json!({ "branch_id": new_branch_id }).to_string(),
    );
    assert_eq!(
        read_back["result"]["kitchen_id"],
        json!(second_kitchen_id),
        "held by the Kitchen named explicitly, not the Home Kitchen"
    );
}

// --- The recipe as written (issue #43) ---------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn saving_a_version_carries_the_whole_written_recipe_verbatim() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "10 Minute Chili Garlic Silken Tofu" })
            .to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();
    backdate_branch_head(&app, &branch_id);

    // "pinch of salt" and "Za'tar" are genuine text from Aurélien's
    // 86-recipe Crouton corpus (samples/crouton/) — two of the 239 real
    // Ingredient Lines that carry no quantity at all (ADR 0002). Crouton
    // never stores a combined line for an amount-bearing ingredient (it
    // splits quantity and name apart, which is the very loss ADR 0002
    // rejects), so "2 tbsp soy sauce" below is ordinary written test input
    // rather than a reconstruction of Crouton's split fields.
    let body = json!({
        "branch_id": branch_id,
        "title": "10 Minute Chili Garlic Silken Tofu",
        "yield": { "amount": "2", "noun": "servings" },
        "prep_time_minutes": 10,
        "cook_time_minutes": 5,
        "note": "This doubles well for a crowd.",
        "source": { "text": "Mum's ring binder, p.40", "link": null },
        "ingredients": [
            { "kind": "section", "text": "For the sauce" },
            { "kind": "ingredient", "text": "2 tbsp soy sauce" },
            { "kind": "ingredient", "text": "pinch of salt" },
            { "kind": "ingredient", "text": "Za’tar" },
        ],
        "steps": [
            { "kind": "section", "text": "Drain the Tofu" },
            {
                "kind": "step",
                "text": "Carefully remove tofu from package. Silken tofu can be quite fragile.",
                "photo": null,
            },
            { "kind": "section", "text": "Prepare the Chili Garlic Sauce" },
            {
                "kind": "step",
                "text": "In a small skillet, combine white parts of scallions, garlic, brown sugar.",
            },
        ],
    });
    let (status, saved) = app.post_op("save_recipe_version", Some(&key), &body.to_string());
    assert_eq!(status, 200, "{saved}");

    let (_, read_back) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let content = &read_back["result"]["versions"][1]["content"];

    assert_eq!(
        content["title"],
        json!("10 Minute Chili Garlic Silken Tofu")
    );
    assert_eq!(
        content["yield"],
        json!({ "amount": "2", "noun": "servings" })
    );
    assert_eq!(content["prep_time_minutes"], json!(10));
    assert_eq!(content["cook_time_minutes"], json!(5));
    assert_eq!(content["note"], json!("This doubles well for a crowd."));
    assert_eq!(
        content["source"],
        json!({ "text": "Mum's ring binder, p.40", "link": null })
    );
    assert_eq!(
        content["ingredients"],
        json!([
            { "kind": "section", "text": "For the sauce" },
            { "kind": "ingredient", "text": "2 tbsp soy sauce" },
            { "kind": "ingredient", "text": "pinch of salt" },
            { "kind": "ingredient", "text": "Za’tar" },
        ]),
        "an Ingredient Line survives exactly as typed, no quantity required (ADR 0002)"
    );
    assert_eq!(
        content["steps"],
        json!([
            { "kind": "section", "text": "Drain the Tofu", "photo": null },
            {
                "kind": "step",
                "text": "Carefully remove tofu from package. Silken tofu can be quite fragile.",
                "photo": null,
            },
            { "kind": "section", "text": "Prepare the Chili Garlic Sauce", "photo": null },
            {
                "kind": "step",
                "text": "In a small skillet, combine white parts of scallions, garlic, brown sugar.",
                "photo": null,
            },
        ]),
        "Sections are real entries in both lists, and a Step has no timer or temperature field"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bare_title_still_produces_a_complete_recipe_with_no_rating_field() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Bare Name Recipe" }).to_string(),
    );
    let content = &created["result"]["versions"][0]["content"];
    assert_eq!(content["title"], json!("Bare Name Recipe"));
    assert_eq!(content["yield"], json!(null));
    assert_eq!(content["prep_time_minutes"], json!(null));
    assert_eq!(content["cook_time_minutes"], json!(null));
    assert_eq!(content["note"], json!(null));
    assert_eq!(content["source"], json!(null));
    assert_eq!(content["ingredients"], json!([]));
    assert_eq!(content["steps"], json!([]));
    assert!(
        content.get("rating").is_none(),
        "no rating field exists on a Recipe"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn editing_any_part_of_the_written_recipe_mints_a_version() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Soupe" }).to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();
    let first_version = created["result"]["head_version_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Title unchanged; only the Note changes. It is still a Version — the
    // fingerprint covers everything written, not the title alone (#43).
    backdate_branch_head(&app, &branch_id);
    let (_, saved) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "title": "Soupe", "note": "Freezes well" }).to_string(),
    );
    assert_ne!(saved["result"]["version_id"], json!(first_version));
    assert_eq!(saved["result"]["collapsed"], json!(false));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn saving_a_version_rejects_malformed_recipe_fields() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Soupe" }).to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    let (status, response) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "title": "Soupe", "prep_time_minutes": -5 }).to_string(),
    );
    assert_eq!(status, 400, "{response}");

    let (status, response) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": branch_id,
            "title": "Soupe",
            "ingredients": [{ "kind": "ingredient", "text": "   " }],
        })
        .to_string(),
    );
    assert_eq!(status, 400, "{response}");

    let (status, response) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": branch_id,
            "title": "Soupe",
            "steps": [{ "kind": "not-a-real-kind", "text": "Mix well" }],
        })
        .to_string(),
    );
    assert_eq!(status, 400, "{response}");
}

// --- The Reading (issue #44) --------------------------------------------------

/// The measured fact this ticket is built to survive (ADR 0002): 239 of 863
/// real Ingredient Lines across Aurélien's 86-recipe Crouton corpus
/// (`docs/research/crouton-real-export.md`) carry no quantity at all — not
/// an edge case, but roughly a quarter of a real library. Pinned here as a
/// fixture fact so a future change cannot silently drift from what was
/// actually measured.
#[test]
fn the_crouton_corpus_no_quantity_figure_is_a_fixture_fact() {
    const LINES_WITHOUT_QUANTITY: u32 = 239;
    const LINES_TOTAL: u32 = 863;
    let fraction = f64::from(LINES_WITHOUT_QUANTITY) / f64::from(LINES_TOTAL);
    assert!(
        (fraction - 0.28).abs() < 0.005,
        "measured at 28%, got {:.1}%",
        fraction * 100.0
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_reading_is_stored_beside_the_line_and_an_unread_line_stays_fully_usable() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    // Genuine text from Aurélien's 86-recipe Crouton corpus
    // (samples/crouton/): "2 tbsp soy sauce" carries a quantity, "pinch of
    // salt" and "Za'tar" are two of the 239 real Ingredient Lines that do
    // not (ADR 0002). The last is real too — the corpus keeps whole cooking
    // steps inside ingredient entries, and no reading of one is honest.
    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "10 Minute Chili Garlic Silken Tofu",
            "ingredients": [
                { "kind": "section", "text": "For the sauce" },
                { "kind": "ingredient", "text": "2 tbsp soy sauce" },
                { "kind": "ingredient", "text": "pinch of salt" },
                { "kind": "ingredient", "text": "Za’tar" },
                { "kind": "ingredient", "text": "I use single cream instead of double cream? Yes it also works very well" },
            ],
        })
        .to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    // Kamosu reads the lines it wrote (#71), and what it reads is a Reading
    // and never a rewrite. A Section is not an Ingredient Line and gets none;
    // the two lines carrying no quantity are read as far as they honestly go,
    // which is the Food and nothing more; and the line that is a paragraph
    // rather than a food is left unread. **An unread line is not an error, it
    // is a working line** — and nothing here says which lines those were.
    assert_eq!(
        created["result"]["versions"][0]["readings"],
        json!([
            null,
            { "amount": "2", "unit": "tbsp", "target": "soy sauce", "lineage_id": null },
            { "amount": null, "unit": "pinch", "target": "salt", "lineage_id": null },
            { "amount": null, "unit": null, "target": "Za’tar", "lineage_id": null },
            null,
        ]),
        "Kamosu reads what it can and declines the rest (ADR 0002)"
    );

    // Reading the soy sauce line (index 1) attaches a Reading beside the
    // Version — the written line above it is untouched.
    let (status, read) = app.post_op(
        "set_reading",
        Some(&key),
        &json!({
            "branch_id": branch_id,
            "line_index": 1,
            "amount": "2",
            "unit": "tbsp",
            "target": "soy sauce",
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{read}");
    assert_eq!(
        read["result"],
        json!({
            "line_index": 1,
            "reading": { "amount": "2", "unit": "tbsp", "target": "soy sauce", "lineage_id": null },
            // This Person reads in American measures, which is what the line
            // is already written in — so there is nothing to say beneath it
            // (#49, ADR 0016).
            "measured": null,
        })
    );

    let (_, fetched) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let version = &fetched["result"]["versions"][0];
    assert_eq!(
        version["content"]["ingredients"][1]["text"],
        json!("2 tbsp soy sauce"),
        "the written line is never rewritten by a Reading"
    );
    assert_eq!(
        version["readings"],
        json!([
            null,
            { "amount": "2", "unit": "tbsp", "target": "soy sauce", "lineage_id": null },
            { "amount": null, "unit": "pinch", "target": "salt", "lineage_id": null },
            { "amount": null, "unit": null, "target": "Za’tar", "lineage_id": null },
            null,
        ]),
        "correcting a Reading changes that Reading and nothing else"
    );

    // Correcting the Reading never mints a Version: still exactly one.
    assert_eq!(fetched["result"]["versions"].as_array().unwrap().len(), 1);

    // Clearing a Reading (every field left out) takes the line back to
    // fully unread.
    let (status, cleared) = app.post_op(
        "set_reading",
        Some(&key),
        &json!({ "branch_id": branch_id, "line_index": 1 }).to_string(),
    );
    assert_eq!(status, 200, "{cleared}");
    assert_eq!(cleared["result"]["reading"], json!(null));

    let (_, fetched_again) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    // Cleared means cleared: the line goes back to fully unread and stays
    // there. Nothing re-reads it behind the cook's back, because a Reading
    // somebody removed on purpose is authored data (ADR 0003).
    assert_eq!(
        fetched_again["result"]["versions"][0]["readings"],
        json!([
            null,
            null,
            { "amount": null, "unit": "pinch", "target": "salt", "lineage_id": null },
            { "amount": null, "unit": null, "target": "Za’tar", "lineage_id": null },
            null,
        ])
    );
    assert_eq!(
        fetched_again["result"]["versions"]
            .as_array()
            .unwrap()
            .len(),
        1,
        "clearing a Reading mints no Version either"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_reading_cannot_land_on_a_section_or_off_the_end_of_the_list() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Soupe",
            "ingredients": [
                { "kind": "section", "text": "For the broth" },
                { "kind": "ingredient", "text": "1 litre stock" },
            ],
        })
        .to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    let (status, response) = app.post_op(
        "set_reading",
        Some(&key),
        &json!({ "branch_id": branch_id, "line_index": 0, "amount": "1" }).to_string(),
    );
    assert_eq!(status, 400, "{response}");

    let (status, response) = app.post_op(
        "set_reading",
        Some(&key),
        &json!({ "branch_id": branch_id, "line_index": 5, "amount": "1" }).to_string(),
    );
    assert_eq!(status, 400, "{response}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn only_a_kitchen_member_may_correct_a_reading() {
    let app = support::spawn_app();
    let (_owner, owner_key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let stranger = app.core.create_person("Marc").expect("person");
    let stranger_key = app
        .core
        .mint_access_key(&stranger, "browser", false)
        .unwrap()
        .secret;

    let (_, created) = app.post_op(
        "create_recipe",
        Some(&owner_key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Soupe",
            "ingredients": [{ "kind": "ingredient", "text": "1 litre stock" }],
        })
        .to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    let (status, _) = app.post_op(
        "set_reading",
        Some(&stranger_key),
        &json!({ "branch_id": branch_id, "line_index": 0, "amount": "1" }).to_string(),
    );
    assert_eq!(status, 404);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn saving_a_new_version_carries_a_reading_forward_for_every_unchanged_line() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Soupe",
            "ingredients": [
                { "kind": "ingredient", "text": "2 tbsp soy sauce" },
                { "kind": "ingredient", "text": "1 litre stock" },
            ],
        })
        .to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    app.post_op(
        "set_reading",
        Some(&key),
        &json!({
            "branch_id": branch_id, "line_index": 0,
            "amount": "2", "unit": "tbsp", "target": "soy sauce",
        })
        .to_string(),
    );
    app.post_op(
        "set_reading",
        Some(&key),
        &json!({
            "branch_id": branch_id, "line_index": 1,
            "amount": "1", "unit": "litre", "target": "stock",
        })
        .to_string(),
    );

    // A save that leaves the soy sauce line untouched but rewrites the
    // stock line: only the changed line's Reading should be left behind
    // (ADR 0002 — re-reading an edited line refreshes its Reading).
    backdate_branch_head(&app, &branch_id);
    app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": branch_id,
            "title": "Soupe",
            "ingredients": [
                { "kind": "ingredient", "text": "2 tbsp soy sauce" },
                { "kind": "ingredient", "text": "2 litres stock" },
            ],
        })
        .to_string(),
    );

    let (_, fetched) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let versions = fetched["result"]["versions"].as_array().unwrap();
    assert_eq!(
        versions.len(),
        2,
        "an unrelated line changing still mints a Version"
    );
    assert_eq!(
        versions[1]["readings"],
        json!([
            { "amount": "2", "unit": "tbsp", "target": "soy sauce", "lineage_id": null },
            { "amount": "2", "unit": "litres", "target": "stock", "lineage_id": null },
        ]),
        "the soy sauce Reading carried forward untouched; the rewritten stock \
         line lost its own and was read afresh, now saying two litres"
    );
    // The earlier Version keeps exactly what it always had.
    assert_eq!(
        versions[0]["readings"],
        json!([
            { "amount": "2", "unit": "tbsp", "target": "soy sauce", "lineage_id": null },
            { "amount": "1", "unit": "litre", "target": "stock", "lineage_id": null },
        ])
    );
}

// --- Reading Ingredient Lines (issue #71, ADR 0036) --------------------------

/// **Reading a line is Kamosu's reading and nothing more** (ADR 0021): it
/// arrives with the recipe, it makes no Version, and the words on the page are
/// the words that were typed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reading_a_line_lays_a_reading_over_it_and_never_mints_a_version() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    let (status, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Crêpes",
            "ingredients": [
                { "kind": "ingredient", "text": "200 g de farine" },
                { "kind": "ingredient", "text": "20 cl de crème fraîche" },
                { "kind": "ingredient", "text": "2 gousses d’ail" },
            ],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{created}");
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    // The French half of ADR 0036: the article is glue and never part of the
    // Food, the elided `d'` is cut the same way, and `cl` is a Unit because
    // `units.rs` already spells the Units of all three Languages. No library
    // measured for #71 read any of these three lines.
    assert_eq!(
        created["result"]["versions"][0]["readings"],
        json!([
            { "amount": "200", "unit": "g", "target": "farine", "lineage_id": null },
            { "amount": "20", "unit": "cl", "target": "crème fraîche", "lineage_id": null },
            { "amount": "2", "unit": "gousses", "target": "ail", "lineage_id": null },
        ])
    );

    let (_, fetched) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(
        fetched["result"]["versions"].as_array().unwrap().len(),
        1,
        "reading three lines minted no Version of its own (ADR 0021)"
    );
    let written: Vec<&str> = fetched["result"]["versions"][0]["content"]["ingredients"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| line["text"].as_str().unwrap())
        .collect();
    assert_eq!(
        written,
        [
            "200 g de farine",
            "20 cl de crème fraîche",
            "2 gousses d’ail"
        ],
        "the written line is the truth and reading it changed no character of it"
    );
}

/// **A Reading names a Food, and a Food is #47's to mint** — same rules,
/// same instance-wide list, whether the word was typed or read.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_read_line_creates_its_food_by_the_ordinary_rules() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    for title in ["Crêpes", "Gâteau"] {
        let (status, created) = app.post_op(
            "create_recipe",
            Some(&key),
            &json!({
                "kitchen_id": kitchen_id,
                "title": title,
                "language": "fr",
                "ingredients": [{ "kind": "ingredient", "text": "200 g de farine" }],
            })
            .to_string(),
        );
        assert_eq!(status, 200, "{created}");
    }

    // Two recipes, one Food: the second recipe's *farine* matched the first
    // rather than minting its own, which is the whole of why a shopping list
    // can add them together.
    let (_, listed) = app.post_op("list_foods", Some(&key), "{}");
    let farine: Vec<&Value> = listed["result"]["foods"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|food| {
            food["names"]
                .as_array()
                .unwrap()
                .iter()
                .any(|n| n["name"] == "farine")
        })
        .collect();
    assert_eq!(
        farine.len(),
        1,
        "one Food, matched not minted twice: {listed}"
    );
    assert_eq!(
        farine[0]["names"][0]["language"], "fr",
        "a Food read off a French Branch is named in French"
    );
}

/// **A line Kamosu cannot read never fails the save**, and a recipe made
/// entirely of such lines is an ordinary recipe (#71, ADR 0002).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_recipe_of_lines_nothing_can_read_saves_and_serves_exactly_as_written() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    let unreadable = [
        "Add the orzo and mix well to coat it in the sauce, then bring to a low boil",
        "———",
        "?",
    ];
    let (status, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Notes to self",
            "ingredients": unreadable
                .iter()
                .map(|text| json!({ "kind": "ingredient", "text": text }))
                .collect::<Vec<_>>(),
        })
        .to_string(),
    );
    assert_eq!(status, 200, "an unread line is a working line: {created}");
    assert_eq!(
        created["result"]["versions"][0]["readings"],
        json!([null, null, null]),
        "nothing was read, and nothing pretended to be"
    );

    let (status, fetched) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": created["result"]["branch_id"].as_str().unwrap() }).to_string(),
    );
    assert_eq!(status, 200, "{fetched}");
    let written: Vec<&str> = fetched["result"]["versions"][0]["content"]["ingredients"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| line["text"].as_str().unwrap())
        .collect();
    assert_eq!(written, unreadable, "every line came back exactly as typed");
}

/// **`read_ingredient_lines` is for the library that predates the reader**:
/// it reads what nothing has read, and leaves alone everything anybody — a
/// person or Kamosu — has already read (ADR 0003).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reading_the_library_reads_what_is_unread_and_leaves_a_correction_alone() {
    let app = support::spawn_app();
    let (key, kitchen_id) = operator_with_kitchen(&app);

    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Chicken Katsu Curry",
            "ingredients": [
                { "kind": "ingredient", "text": "1 cup panko" },
                { "kind": "ingredient", "text": "800 ml water" },
            ],
        })
        .to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    // A person disagrees with how Kamosu read the panko line. That correction
    // is authored data and no later reading may touch it.
    let (status, corrected) = app.post_op(
        "set_reading",
        Some(&key),
        &json!({
            "branch_id": branch_id, "line_index": 0,
            "amount": "2", "unit": "cups", "target": "panko breadcrumbs",
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{corrected}");

    let (status, asked) = app.post_op("read_ingredient_lines", Some(&key), "{}");
    assert_eq!(status, 200, "{asked}");
    let job_id = asked["result"]["job_id"].as_str().expect("a job id");
    let finished = support::wait_terminal(&app, Some(&key), job_id);
    assert_eq!(finished["status"], "completed", "{finished}");
    assert_eq!(
        finished["result"]["read"], 0,
        "both lines were read as they were written, so there was nothing left to read"
    );

    let (_, fetched) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(
        fetched["result"]["versions"][0]["readings"],
        json!([
            { "amount": "2", "unit": "cups", "target": "panko breadcrumbs", "lineage_id": null },
            { "amount": "800", "unit": "ml", "target": "water", "lineage_id": null },
        ]),
        "the correction survived; the line Kamosu read is still as it read it"
    );
    assert_eq!(
        fetched["result"]["versions"].as_array().unwrap().len(),
        1,
        "reading the whole library mints no Version anywhere (ADR 0021)"
    );
}

/// The Job is the Operator's, because it walks every recipe on the instance
/// and not one Kitchen's (ADR 0032's neighbourhood: what a stranger may cause
/// is bounded, and this is not something a stranger may cause at all).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reading_the_library_is_the_operators_alone() {
    let app = support::spawn_app();
    let (_operator_key, _kitchen) = operator_with_kitchen(&app);
    let (_person, key, _kitchen_id) = person_with_kitchen(&app, "Someone else");

    let (status, refused) = app.post_op("read_ingredient_lines", Some(&key), "{}");
    assert_eq!(status, 401, "{refused}");
    assert_eq!(refused["error"]["kind"], "unauthorized");
}

// --- Foods (issue #47) --------------------------------------------------------

/// Create a one-line Recipe in the given Language and set that line's
/// Reading `target` to `word` — the smallest way to make a Food arrive for a
/// test. Returns the Branch id, for tests that go on to clear the Reading.
///
/// The title carries a fresh counter so reading the same word twice (to
/// prove a Food gains a second Reading) still produces two distinct
/// Versions — a Version is content-addressed (ADR 0004), so two Recipes
/// with identical content share one, and Readings key on the Version rather
/// than the Branch (ADR 0021).
fn read_a_word(
    app: &support::TestApp,
    key: &str,
    kitchen_id: &str,
    language: &str,
    word: &str,
) -> String {
    static FIXTURE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let fixture_id = FIXTURE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let (_, created) = app.post_op(
        "create_recipe",
        Some(key),
        &json!({
            "kitchen_id": kitchen_id,
            "language": language,
            "title": format!("Food Match fixture {fixture_id}"),
            "ingredients": [{ "kind": "ingredient", "text": word }],
        })
        .to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();
    let (status, response) = app.post_op(
        "set_reading",
        Some(key),
        &json!({ "branch_id": branch_id, "line_index": 0, "target": word }).to_string(),
    );
    assert_eq!(status, 200, "{response}");
    branch_id
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_reading_naming_an_unseen_word_creates_a_food_automatically() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    let (_, foods_before) = app.post_op("list_foods", Some(&key), "{}");
    assert_eq!(
        foods_before["result"]["foods"],
        json!([]),
        "nothing has been read yet"
    );

    read_a_word(&app, &key, &kitchen_id, "en", "soy sauce");

    let (_, listed) = app.post_op("list_foods", Some(&key), "{}");
    let foods = listed["result"]["foods"].as_array().unwrap();
    assert_eq!(
        foods.len(),
        1,
        "the unseen word 'soy sauce' created exactly one Food"
    );
    assert_eq!(foods[0]["name"], json!("soy sauce"));
    assert_eq!(foods[0]["language"], json!("en"));
    assert_eq!(
        foods[0]["names"],
        json!([{ "language": "en", "name": "soy sauce" }])
    );
    assert_eq!(foods[0]["cup_weight_grams"], json!(null));
    assert_eq!(
        foods[0]["nutrition"],
        json!(null),
        "nutrition never travels and nothing sets it in v1"
    );
    assert_eq!(foods[0]["reading_count"], json!(1));

    // Reading the same word again — even spelled differently — does not
    // create a second Food.
    read_a_word(&app, &key, &kitchen_id, "en", "Soy Sauce");
    let (_, listed_again) = app.post_op("list_foods", Some(&key), "{}");
    let foods_again = listed_again["result"]["foods"].as_array().unwrap();
    assert_eq!(
        foods_again.len(),
        1,
        "case folding: 'Soy Sauce' matches the existing 'soy sauce' Food"
    );
    assert_eq!(foods_again[0]["reading_count"], json!(2));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn matching_folds_case_and_whitespace_but_respects_language_accents_and_plurals() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    // Case and stray whitespace fold: "Flour" and "  flour  " are one Food.
    read_a_word(&app, &key, &kitchen_id, "en", "Flour");
    read_a_word(&app, &key, &kitchen_id, "en", "  flour  ");

    // Accents are meaning, not noise: maïs (corn) is not mais (but) — the
    // worked example in ADR 0022.
    read_a_word(&app, &key, &kitchen_id, "fr", "maïs");
    read_a_word(&app, &key, &kitchen_id, "fr", "mais");

    // Plurals are meaning too, ADR 0022's own example: oeufs is not oeuf.
    read_a_word(&app, &key, &kitchen_id, "fr", "oeuf");
    read_a_word(&app, &key, &kitchen_id, "fr", "oeufs");

    // Language must agree: English raisin (a dried grape) is not French
    // raisin (a fresh one) — ADR 0022.
    read_a_word(&app, &key, &kitchen_id, "en", "raisin");
    read_a_word(&app, &key, &kitchen_id, "fr", "raisin");

    let (_, listed) = app.post_op("list_foods", Some(&key), "{}");
    let foods = listed["result"]["foods"].as_array().unwrap();
    // flour (1) + maïs/mais (2) + oeuf/oeufs (2) + raisin en/fr (2) = 7.
    assert_eq!(foods.len(), 7, "{foods:#?}");

    let flour = foods
        .iter()
        .find(|food| food["language"] == json!("en") && food["name"] == json!("Flour"))
        .expect("the folded Food exists, spelled the way it was first typed");
    assert_eq!(
        flour["reading_count"],
        json!(2),
        "case and whitespace fold to the same Food"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_ambiguous_lone_word_resolves_to_the_food_the_most_readings_already_use() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    // Food A: read twice, so it is the busier of the two once B exists too.
    read_a_word(&app, &key, &kitchen_id, "fr", "farine");
    read_a_word(&app, &key, &kitchen_id, "fr", "farine");
    let (_, listed) = app.post_op("list_foods", Some(&key), "{}");
    let food_a_id = listed["result"]["foods"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(listed["result"]["foods"][0]["reading_count"], json!(2));

    // Food B: an unrelated Food, read once via a different English word.
    read_a_word(&app, &key, &kitchen_id, "en", "rye flour");
    let (_, listed2) = app.post_op("list_foods", Some(&key), "{}");
    let food_b_id = listed2["result"]["foods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|food| food["id"] != json!(food_a_id))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Typing "farine" onto Food B directly is the one deliberate way to
    // make a duplicate name (ADR 0022's Consequences) — both Foods now
    // answer to "farine" in French.
    let (status, named) = app.post_op(
        "set_food_name",
        Some(&key),
        &json!({ "food_id": food_b_id, "language": "fr", "name": "farine" }).to_string(),
    );
    assert_eq!(status, 200, "{named}");

    // A fresh, lone French "farine" is now ambiguous between A and B. It
    // resolves to the busier of the two — A, with two Readings already —
    // rather than ever creating a third Food.
    read_a_word(&app, &key, &kitchen_id, "fr", "farine");

    let (_, listed3) = app.post_op("list_foods", Some(&key), "{}");
    let foods3 = listed3["result"]["foods"].as_array().unwrap();
    assert_eq!(
        foods3.len(),
        2,
        "no third Food is created for a lone ambiguous word"
    );
    let food_a = foods3
        .iter()
        .find(|food| food["id"] == json!(food_a_id))
        .unwrap();
    let food_b = foods3
        .iter()
        .find(|food| food["id"] == json!(food_b_id))
        .unwrap();
    assert_eq!(
        food_a["reading_count"],
        json!(3),
        "the ambiguous word went to the busier Food"
    );
    assert_eq!(
        food_b["reading_count"],
        json!(1),
        "the quieter Food gained no Reading from the ambiguous word"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn correcting_an_already_read_lines_ambiguous_target_does_not_count_its_own_stale_reading() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    // Food A: one Reading elsewhere, plus the line we are about to correct —
    // two Readings in total, but only one that will still be A's once the
    // correction below lands.
    read_a_word(&app, &key, &kitchen_id, "fr", "farine");
    let correcting_branch_id = read_a_word(&app, &key, &kitchen_id, "fr", "farine");
    let (_, listed) = app.post_op("list_foods", Some(&key), "{}");
    let food_a_id = listed["result"]["foods"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(listed["result"]["foods"][0]["reading_count"], json!(2));

    // Food B: busier than Food A's *other* Reading (1), read twice.
    read_a_word(&app, &key, &kitchen_id, "en", "rye flour");
    read_a_word(&app, &key, &kitchen_id, "en", "rye flour");
    let (_, listed2) = app.post_op("list_foods", Some(&key), "{}");
    let food_b_id = listed2["result"]["foods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|food| food["id"] != json!(food_a_id))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        listed2["result"]["foods"]
            .as_array()
            .unwrap()
            .iter()
            .find(|food| food["id"] == json!(food_b_id))
            .unwrap()["reading_count"],
        json!(2)
    );

    // Typing "farine" onto Food B makes it a duplicate of Food A's own name.
    let (status, named) = app.post_op(
        "set_food_name",
        Some(&key),
        &json!({ "food_id": food_b_id, "language": "fr", "name": "farine" }).to_string(),
    );
    assert_eq!(status, 200, "{named}");

    // Re-submitting the very line that already reads "farine" and points at
    // Food A is now ambiguous between A and B. A's *other* Reading (1) is
    // quieter than B's (2), so the correction must move to B — counting the
    // line's own about-to-be-overwritten row toward A would wrongly make it
    // a 2-2 tie and leave it stuck on A instead.
    let (status, corrected) = app.post_op(
        "set_reading",
        Some(&key),
        &json!({ "branch_id": correcting_branch_id, "line_index": 0, "target": "farine" })
            .to_string(),
    );
    assert_eq!(status, 200, "{corrected}");

    let (_, food_a_after) = app.post_op(
        "get_food",
        Some(&key),
        &json!({ "food_id": food_a_id }).to_string(),
    );
    let (_, food_b_after) = app.post_op(
        "get_food",
        Some(&key),
        &json!({ "food_id": food_b_id }).to_string(),
    );
    assert_eq!(
        food_a_after["result"]["reading_count"],
        json!(1),
        "the corrected line's stale Reading is gone from Food A"
    );
    assert_eq!(
        food_b_after["result"]["reading_count"],
        json!(3),
        "the corrected line resolved to the busier Food B, not the one it used to point at"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_food_nothing_points_at_is_kept_rather_than_swept() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let branch_id = read_a_word(&app, &key, &kitchen_id, "en", "cardamom");

    let (_, listed) = app.post_op("list_foods", Some(&key), "{}");
    let food_id = listed["result"]["foods"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(listed["result"]["foods"][0]["reading_count"], json!(1));

    // Clear the one Reading that pointed at it.
    let (status, cleared) = app.post_op(
        "set_reading",
        Some(&key),
        &json!({ "branch_id": branch_id, "line_index": 0 }).to_string(),
    );
    assert_eq!(status, 200, "{cleared}");

    let (status, after_clear) = app.post_op(
        "get_food",
        Some(&key),
        &json!({ "food_id": food_id }).to_string(),
    );
    assert_eq!(
        status, 200,
        "the Food itself is kept, not swept, {after_clear}"
    );
    assert_eq!(
        after_clear["result"]["reading_count"],
        json!(0),
        "nothing points at it any more"
    );
    assert_eq!(after_clear["result"]["name"], json!("cardamom"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_food_holds_an_optional_cup_weight_and_a_nutrition_slot_that_never_travels() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    read_a_word(&app, &key, &kitchen_id, "en", "butter");
    let (_, listed) = app.post_op("list_foods", Some(&key), "{}");
    let food_id = listed["result"]["foods"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        listed["result"]["foods"][0]["cup_weight_grams"],
        json!(null)
    );
    assert_eq!(listed["result"]["foods"][0]["nutrition"], json!(null));

    let (status, updated) = app.post_op(
        "set_food_cup_weight",
        Some(&key),
        &json!({ "food_id": food_id, "cup_weight_grams": 227.0 }).to_string(),
    );
    assert_eq!(status, 200, "{updated}");
    assert_eq!(updated["result"]["cup_weight_grams"], json!(227.0));
    assert_eq!(
        updated["result"]["nutrition"],
        json!(null),
        "nothing in v1 ever sets nutrition — CIQUAL binding is deferred past v1 (#12)"
    );

    // A zero or negative Cup Weight is refused.
    let (status, response) = app.post_op(
        "set_food_cup_weight",
        Some(&key),
        &json!({ "food_id": food_id, "cup_weight_grams": 0 }).to_string(),
    );
    assert_eq!(status, 400, "{response}");

    // Clearing it back to null is allowed.
    let (status, cleared) = app.post_op(
        "set_food_cup_weight",
        Some(&key),
        &json!({ "food_id": food_id, "cup_weight_grams": null }).to_string(),
    );
    assert_eq!(status, 200, "{cleared}");
    assert_eq!(cleared["result"]["cup_weight_grams"], json!(null));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn any_person_may_name_a_food_and_its_last_remaining_name_cannot_be_taken() {
    let app = support::spawn_app();
    let (_owner, owner_key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    read_a_word(&app, &owner_key, &kitchen_id, "fr", "farine");
    let (_, listed) = app.post_op("list_foods", Some(&owner_key), "{}");
    let food_id = listed["result"]["foods"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // A Person who is not a member of the recipe's Kitchen may still name
    // this Food — it is instance-wide, not a Kitchen's to guard (CONTEXT.md).
    let stranger = app.core.create_person("Marc").expect("person");
    let stranger_key = app
        .core
        .mint_access_key(&stranger, "browser", false)
        .unwrap()
        .secret;
    let (status, named) = app.post_op(
        "set_food_name",
        Some(&stranger_key),
        &json!({ "food_id": food_id, "language": "en", "name": "flour" }).to_string(),
    );
    assert_eq!(status, 200, "{named}");
    assert_eq!(
        named["result"]["names"],
        json!([
            { "language": "en", "name": "flour" },
            { "language": "fr", "name": "farine" },
        ])
    );

    // Removing one of its two names is fine...
    let (status, removed) = app.post_op(
        "remove_food_name",
        Some(&owner_key),
        &json!({ "food_id": food_id, "language": "en" }).to_string(),
    );
    assert_eq!(status, 200, "{removed}");
    assert_eq!(
        removed["result"]["names"],
        json!([{ "language": "fr", "name": "farine" }])
    );

    // ...but its last remaining name may not be, too — a Food is known by
    // its words alone (CONTEXT.md).
    let (status, response) = app.post_op(
        "remove_food_name",
        Some(&owner_key),
        &json!({ "food_id": food_id, "language": "fr" }).to_string(),
    );
    assert_eq!(status, 400, "{response}");
}

// --- Merge Suggestions and Merge (issue #48) ---------------------------------

/// The Operator, with an Access Key and their home Kitchen — the only Person
/// who may merge a Food or delete one (CONTEXT.md, ADR 0022).
fn operator_with_kitchen(app: &support::TestApp) -> (String, String) {
    let first = json!({
        "name": "Aurélien",
        "password": "a password only its person knows",
        "session_name": "test browser",
    });
    let (status, created) = app.post_auth("/auth/first-person", &first.to_string());
    assert_eq!(status, 200, "{created}");
    let operator_id = created["result"]["person"]["id"].as_str().unwrap();
    let kitchen_id = created["result"]["person"]["home_kitchen_id"]
        .as_str()
        .unwrap()
        .to_string();
    let key = app
        .core
        .mint_access_key(operator_id, "agent", false)
        .unwrap()
        .secret;
    (key, kitchen_id)
}

/// The id of the one Food answering to `name` in `language`.
fn food_named(app: &support::TestApp, key: &str, language: &str, name: &str) -> String {
    let (_, listed) = app.post_op("list_foods", Some(key), "{}");
    let found: Vec<String> = listed["result"]["foods"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|food| {
            food["names"]
                .as_array()
                .unwrap()
                .iter()
                .any(|held| held["language"] == json!(language) && held["name"] == json!(name))
        })
        .map(|food| food["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(found.len(), 1, "exactly one Food is {language}:{name}");
    found.into_iter().next().unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn typing_a_name_another_food_answers_to_records_a_suggestion_and_merges_nothing() {
    let app = support::spawn_app();
    let (key, kitchen_id) = operator_with_kitchen(&app);

    read_a_word(&app, &key, &kitchen_id, "fr", "farine");
    read_a_word(&app, &key, &kitchen_id, "en", "rye flour");
    let food_a = food_named(&app, &key, "fr", "farine");
    let food_b = food_named(&app, &key, "en", "rye flour");

    let (_, before) = app.post_op("list_merge_suggestions", Some(&key), "{}");
    assert_eq!(
        before["result"]["suggestions"],
        json!([]),
        "nothing has suggested anything yet"
    );

    // The one remaining way to make a duplicate name (ADR 0022): typing
    // "farine" onto Food B, which Food A already answers to.
    let (status, named) = app.post_op(
        "set_food_name",
        Some(&key),
        &json!({ "food_id": food_b, "language": "fr", "name": "farine" }).to_string(),
    );
    assert_eq!(status, 200, "{named}");

    let (status, listed) = app.post_op("list_merge_suggestions", Some(&key), "{}");
    assert_eq!(status, 200, "{listed}");
    let suggestions = listed["result"]["suggestions"].as_array().unwrap();
    assert_eq!(suggestions.len(), 1, "{suggestions:#?}");
    assert_eq!(suggestions[0]["reason"], json!("name_typed_onto_another"));
    assert_eq!(
        suggestions[0]["words"],
        json!([{ "language": "fr", "name": "farine" }]),
        "the suggestion carries the word that made it, not a resemblance"
    );
    let named_pair: Vec<&str> = suggestions[0]["foods"]
        .as_array()
        .unwrap()
        .iter()
        .map(|food| food["id"].as_str().unwrap())
        .collect();
    assert!(named_pair.contains(&food_a.as_str()) && named_pair.contains(&food_b.as_str()));

    // Nothing acted on it: both Foods are still there, still two.
    let (status, still_a) = app.post_op(
        "get_food",
        Some(&key),
        &json!({ "food_id": food_a }).to_string(),
    );
    assert_eq!(status, 200, "{still_a}");
    let (status, still_b) = app.post_op(
        "get_food",
        Some(&key),
        &json!({ "food_id": food_b }).to_string(),
    );
    assert_eq!(status, 200, "{still_b}");

    // Recording the same evidence again does not pile up a second note.
    let (_, again) = app.post_op(
        "set_food_name",
        Some(&key),
        &json!({ "food_id": food_b, "language": "fr", "name": "Farine" }).to_string(),
    );
    assert_eq!(again["result"]["names"].as_array().unwrap().len(), 2);
    let (_, listed_again) = app.post_op("list_merge_suggestions", Some(&key), "{}");
    assert_eq!(
        listed_again["result"]["suggestions"]
            .as_array()
            .unwrap()
            .len(),
        1,
        "suggestions accumulate as evidence, not as duplicates of one event"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn merging_is_the_operators_and_says_what_it_will_move_before_it_moves_it() {
    let app = support::spawn_app();
    let (operator_key, kitchen_id) = operator_with_kitchen(&app);

    // Food A: three Readings, spread across three recipes.
    read_a_word(&app, &operator_key, &kitchen_id, "fr", "farine");
    read_a_word(&app, &operator_key, &kitchen_id, "fr", "farine");
    let edited_branch_id = read_a_word(&app, &operator_key, &kitchen_id, "fr", "farine");
    let food_a = food_named(&app, &operator_key, "fr", "farine");

    // Food B: one Reading, and an English name Food A has none of.
    read_a_word(&app, &operator_key, &kitchen_id, "en", "flour");
    let food_b = food_named(&app, &operator_key, "en", "flour");

    // An ordinary Person may not merge: it is on the Operator's exact list of
    // powers and on no one else's (CONTEXT.md).
    let stranger = app.core.create_person("Marc").expect("person");
    let stranger_key = app
        .core
        .mint_access_key(&stranger, "browser", false)
        .unwrap()
        .secret;
    for operation in ["preview_food_merge", "merge_food"] {
        let (status, refused) = app.post_op(
            operation,
            Some(&stranger_key),
            &json!({
                "survivor_food_id": food_a,
                "absorbed_food_id": food_b,
                "ingredient_lines": 1,
            })
            .to_string(),
        );
        assert_eq!(status, 401, "{operation}: {refused}");
    }
    let (status, refused) = app.post_op("list_merge_suggestions", Some(&stranger_key), "{}");
    assert_eq!(status, 401, "{refused}");

    // One of Food A's three recipes is edited twice after its line was read.
    // The Reading is carried onto each new Version, so Food A now has five
    // Reading rows over three Ingredient Lines — and the figure the Operator
    // confirms must be the three lines they would see move, not the five rows
    // beneath them. An inflated safety net is a broken one.
    for title in ["Fixture, second thoughts", "Fixture, third thoughts"] {
        // Backdated between saves: consecutive edits inside the coalescing
        // window amend the head Version rather than minting a new one.
        backdate_branch_head(&app, &edited_branch_id);
        let (status, saved) = app.post_op(
            "save_recipe_version",
            Some(&operator_key),
            &json!({
                "branch_id": edited_branch_id,
                "title": title,
                "ingredients": [{ "kind": "ingredient", "text": "farine" }],
            })
            .to_string(),
        );
        assert_eq!(status, 200, "{saved}");
    }
    let (_, food_a_now) = app.post_op(
        "get_food",
        Some(&operator_key),
        &json!({ "food_id": food_a }).to_string(),
    );
    assert_eq!(
        food_a_now["result"]["reading_count"],
        json!(5),
        "the carried-forward Readings are real rows"
    );

    // The saying comes first, and moves nothing.
    let (status, preview) = app.post_op(
        "preview_food_merge",
        Some(&operator_key),
        &json!({ "survivor_food_id": food_b, "absorbed_food_id": food_a }).to_string(),
    );
    assert_eq!(status, 200, "{preview}");
    assert_eq!(
        preview["result"]["ingredient_lines"],
        json!(3),
        "three Ingredient Lines move on screen, not the five rows beneath them"
    );
    assert_eq!(
        preview["result"]["readings"],
        json!(5),
        "and five Reading rows change hands in the database"
    );
    assert_eq!(preview["result"]["cup_weight_conflict"], json!(false));
    assert_eq!(preview["result"]["survivor"]["id"], json!(food_b));
    assert_eq!(preview["result"]["absorbed"]["id"], json!(food_a));

    let (_, unchanged) = app.post_op(
        "get_food",
        Some(&operator_key),
        &json!({ "food_id": food_a }).to_string(),
    );
    assert_eq!(
        unchanged["result"]["reading_count"],
        json!(5),
        "a preview moves nothing"
    );

    // The saying is binding: a Merge told the wrong figure is refused rather
    // than performed. That refusal is what makes the announcement a safety net
    // instead of a number nobody had to read.
    let (status, refused) = app.post_op(
        "merge_food",
        Some(&operator_key),
        &json!({
            "survivor_food_id": food_b,
            "absorbed_food_id": food_a,
            "ingredient_lines": 5,
        })
        .to_string(),
    );
    assert_eq!(
        status, 400,
        "a Merge cannot be reached without having read what it moves: {refused}"
    );
    let (_, still_there) = app.post_op(
        "get_food",
        Some(&operator_key),
        &json!({ "food_id": food_a }).to_string(),
    );
    assert_eq!(
        still_there["result"]["reading_count"],
        json!(5),
        "the refused Merge moved nothing"
    );

    // Then the Merge itself, with the figure it was told.
    let (_, survivor_before) = app.post_op(
        "get_food",
        Some(&operator_key),
        &json!({ "food_id": food_b }).to_string(),
    );
    let survivor_before = survivor_before["result"]["reading_count"].as_i64().unwrap();
    let (status, merged) = app.post_op(
        "merge_food",
        Some(&operator_key),
        &json!({
            "survivor_food_id": food_b,
            "absorbed_food_id": food_a,
            "ingredient_lines": preview["result"]["ingredient_lines"],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{merged}");
    assert_eq!(
        merged["result"]["ingredient_lines"], preview["result"]["ingredient_lines"],
        "the announced count is what moved"
    );
    assert_eq!(merged["result"]["readings"], preview["result"]["readings"]);

    // And the announcement is held against what actually moved rather than
    // against itself: the survivor gained exactly the Readings it named.
    assert_eq!(
        merged["result"]["food"]["reading_count"].as_i64().unwrap(),
        survivor_before + merged["result"]["readings"].as_i64().unwrap(),
        "every Reading the count named is a Reading the survivor now has"
    );

    // The survivor carries all names in all Languages.
    assert_eq!(merged["result"]["food"]["id"], json!(food_b));
    assert_eq!(
        merged["result"]["food"]["names"],
        json!([
            { "language": "en", "name": "flour" },
            { "language": "fr", "name": "farine" },
        ])
    );

    // The absorbed Food is gone, and there is no un-merge to bring it back.
    let (status, gone) = app.post_op(
        "get_food",
        Some(&operator_key),
        &json!({ "food_id": food_a }).to_string(),
    );
    assert_eq!(status, 404, "{gone}");
    let (status, no_such) = app.post_op(
        "unmerge_food",
        Some(&operator_key),
        &json!({ "food_id": food_a }).to_string(),
    );
    assert_eq!(
        status, 404,
        "there is no un-merge in v1 and nothing pretends there is: {no_such}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_merge_asks_which_cup_weight_survives_only_when_the_two_disagree() {
    let app = support::spawn_app();
    let (key, kitchen_id) = operator_with_kitchen(&app);

    read_a_word(&app, &key, &kitchen_id, "fr", "farine");
    read_a_word(&app, &key, &kitchen_id, "en", "flour");
    let food_a = food_named(&app, &key, "fr", "farine");
    let food_b = food_named(&app, &key, "en", "flour");

    // Only one of them knows a Cup Weight: nothing to ask, and the figure is
    // kept rather than lost with the Food that held it.
    let (status, weighed) = app.post_op(
        "set_food_cup_weight",
        Some(&key),
        &json!({ "food_id": food_a, "cup_weight_grams": 125.0 }).to_string(),
    );
    assert_eq!(status, 200, "{weighed}");
    let (status, preview) = app.post_op(
        "preview_food_merge",
        Some(&key),
        &json!({ "survivor_food_id": food_b, "absorbed_food_id": food_a }).to_string(),
    );
    assert_eq!(status, 200, "{preview}");
    assert_eq!(preview["result"]["cup_weight_conflict"], json!(false));

    // With nothing in dispute there is nothing to say, and saying something
    // anyway is refused rather than quietly overwriting a known figure.
    let (status, uninvited) = app.post_op(
        "merge_food",
        Some(&key),
        &json!({
            "survivor_food_id": food_b,
            "absorbed_food_id": food_a,
            "ingredient_lines": 1,
            "cup_weight_grams": 200.0,
        })
        .to_string(),
    );
    assert_eq!(
        status, 400,
        "merge_food is not a way to set a Cup Weight: {uninvited}"
    );

    // Now both know one, and they disagree.
    let (status, weighed) = app.post_op(
        "set_food_cup_weight",
        Some(&key),
        &json!({ "food_id": food_b, "cup_weight_grams": 120.0 }).to_string(),
    );
    assert_eq!(status, 200, "{weighed}");
    let (_, conflicted) = app.post_op(
        "preview_food_merge",
        Some(&key),
        &json!({ "survivor_food_id": food_b, "absorbed_food_id": food_a }).to_string(),
    );
    assert_eq!(conflicted["result"]["cup_weight_conflict"], json!(true));

    let (status, refused) = app.post_op(
        "merge_food",
        Some(&key),
        &json!({
            "survivor_food_id": food_b,
            "absorbed_food_id": food_a,
            "ingredient_lines": 1,
        })
        .to_string(),
    );
    assert_eq!(
        status, 400,
        "a disagreement is the Operator's to settle, never Kamosu's to guess: {refused}"
    );

    // And it is settled by choosing between the two, not by naming a third: a
    // Merge answers a disagreement, it is not a way to set a Cup Weight.
    let (status, invented) = app.post_op(
        "merge_food",
        Some(&key),
        &json!({
            "survivor_food_id": food_b,
            "absorbed_food_id": food_a,
            "ingredient_lines": 1,
            "cup_weight_grams": 118.0,
        })
        .to_string(),
    );
    assert_eq!(
        status, 400,
        "a figure neither Food held is not a choice between them: {invented}"
    );

    let (status, merged) = app.post_op(
        "merge_food",
        Some(&key),
        &json!({
            "survivor_food_id": food_b,
            "absorbed_food_id": food_a,
            "ingredient_lines": 1,
            "cup_weight_grams": 125.0,
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{merged}");
    assert_eq!(
        merged["result"]["food"]["cup_weight_grams"],
        json!(125.0),
        "the figure the Operator chose is the one that survives"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_merge_clears_every_suggestion_naming_either_food() {
    let app = support::spawn_app();
    let (key, kitchen_id) = operator_with_kitchen(&app);

    read_a_word(&app, &key, &kitchen_id, "fr", "farine");
    read_a_word(&app, &key, &kitchen_id, "en", "rye flour");
    let food_a = food_named(&app, &key, "fr", "farine");
    let food_b = food_named(&app, &key, "en", "rye flour");
    let (status, named) = app.post_op(
        "set_food_name",
        Some(&key),
        &json!({ "food_id": food_b, "language": "fr", "name": "farine" }).to_string(),
    );
    assert_eq!(status, 200, "{named}");
    let (_, listed) = app.post_op("list_merge_suggestions", Some(&key), "{}");
    assert_eq!(listed["result"]["suggestions"].as_array().unwrap().len(), 1);

    let (_, preview) = app.post_op(
        "preview_food_merge",
        Some(&key),
        &json!({ "survivor_food_id": food_a, "absorbed_food_id": food_b }).to_string(),
    );
    let (status, merged) = app.post_op(
        "merge_food",
        Some(&key),
        &json!({
            "survivor_food_id": food_a,
            "absorbed_food_id": food_b,
            "ingredient_lines": preview["result"]["ingredient_lines"],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{merged}");

    let (_, after) = app.post_op("list_merge_suggestions", Some(&key), "{}");
    assert_eq!(
        after["result"]["suggestions"],
        json!([]),
        "the question the suggestion asked has been answered"
    );

    // A Food cannot be merged into itself, and neither may a Food that is gone.
    let (status, refused) = app.post_op(
        "merge_food",
        Some(&key),
        &json!({
            "survivor_food_id": food_a,
            "absorbed_food_id": food_a,
            "ingredient_lines": 0,
        })
        .to_string(),
    );
    assert_eq!(status, 400, "{refused}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_name_a_merge_adopts_leaves_the_same_trail_a_typed_one_would() {
    let app = support::spawn_app();
    let (key, kitchen_id) = operator_with_kitchen(&app);

    // Three Foods. A and B are about to be merged; C already answers to the
    // English word B holds, so the survivor's adoption of it makes a duplicate
    // name — the one ADR 0022 says must leave a trail however it was made.
    read_a_word(&app, &key, &kitchen_id, "fr", "farine");
    read_a_word(&app, &key, &kitchen_id, "en", "flour");
    read_a_word(&app, &key, &kitchen_id, "es", "harina");
    let food_a = food_named(&app, &key, "fr", "farine");
    let food_b = food_named(&app, &key, "en", "flour");
    let food_c = food_named(&app, &key, "es", "harina");
    let (status, named) = app.post_op(
        "set_food_name",
        Some(&key),
        &json!({ "food_id": food_c, "language": "en", "name": "flour" }).to_string(),
    );
    assert_eq!(status, 200, "{named}");

    // That typing already left its own trail, between B and C. Merge B into A
    // and it is cleared — but A now answers to "flour" too, so the duplicate
    // is still there and must still be recorded.
    let (_, preview) = app.post_op(
        "preview_food_merge",
        Some(&key),
        &json!({ "survivor_food_id": food_a, "absorbed_food_id": food_b }).to_string(),
    );
    let (status, merged) = app.post_op(
        "merge_food",
        Some(&key),
        &json!({
            "survivor_food_id": food_a,
            "absorbed_food_id": food_b,
            "ingredient_lines": preview["result"]["ingredient_lines"],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{merged}");

    let (_, listed) = app.post_op("list_merge_suggestions", Some(&key), "{}");
    let suggestions = listed["result"]["suggestions"].as_array().unwrap();
    assert_eq!(
        suggestions.len(),
        1,
        "the duplicate the merge made is recorded, not swallowed: {suggestions:#?}"
    );
    let pair: Vec<&str> = suggestions[0]["foods"]
        .as_array()
        .unwrap()
        .iter()
        .map(|food| food["id"].as_str().unwrap())
        .collect();
    assert!(
        pair.contains(&food_a.as_str()) && pair.contains(&food_c.as_str()),
        "the trail names the survivor and the Food it now duplicates: {pair:?}"
    );
    assert_eq!(
        suggestions[0]["words"],
        json!([{ "language": "en", "name": "flour" }])
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_operator_may_delete_a_food_nothing_points_at_and_no_other() {
    let app = support::spawn_app();
    let (operator_key, kitchen_id) = operator_with_kitchen(&app);
    let branch_id = read_a_word(&app, &operator_key, &kitchen_id, "en", "cardamom");
    let food_id = food_named(&app, &operator_key, "en", "cardamom");

    // Something still points at it: refused, rather than cascaded. What a Food
    // knows was expensive to learn (ADR 0022).
    let (status, refused) = app.post_op(
        "delete_food",
        Some(&operator_key),
        &json!({ "food_id": food_id }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");

    let (status, cleared) = app.post_op(
        "set_reading",
        Some(&operator_key),
        &json!({ "branch_id": branch_id, "line_index": 0 }).to_string(),
    );
    assert_eq!(status, 200, "{cleared}");

    // An ordinary Person still may not: deleting a Food is the Operator's.
    let stranger = app.core.create_person("Marc").expect("person");
    let stranger_key = app
        .core
        .mint_access_key(&stranger, "browser", false)
        .unwrap()
        .secret;
    let (status, forbidden) = app.post_op(
        "delete_food",
        Some(&stranger_key),
        &json!({ "food_id": food_id }).to_string(),
    );
    assert_eq!(status, 401, "{forbidden}");

    let (status, deleted) = app.post_op(
        "delete_food",
        Some(&operator_key),
        &json!({ "food_id": food_id }).to_string(),
    );
    assert_eq!(status, 200, "{deleted}");
    assert_eq!(deleted["result"]["deleted"], json!(true));

    let (status, gone) = app.post_op(
        "get_food",
        Some(&operator_key),
        &json!({ "food_id": food_id }).to_string(),
    );
    assert_eq!(status, 404, "{gone}");
}

// --- Tags (issue #51) --------------------------------------------------------

/// Create a Tag and hand back its id.
fn tag_in(
    app: &support::TestApp,
    key: &str,
    kitchen_id: &str,
    language: &str,
    name: &str,
) -> String {
    let (status, created) = app.post_op(
        "create_tag",
        Some(key),
        &json!({ "kitchen_id": kitchen_id, "language": language, "name": name }).to_string(),
    );
    assert_eq!(status, 200, "{created}");
    created["result"]["id"].as_str().unwrap().to_string()
}

/// Create a recipe and hand back its Branch id.
fn recipe_in(app: &support::TestApp, key: &str, kitchen_id: &str, title: &str) -> String {
    let (status, created) = app.post_op(
        "create_recipe",
        Some(key),
        &json!({ "kitchen_id": kitchen_id, "title": title }).to_string(),
    );
    assert_eq!(status, 200, "{created}");
    created["result"]["branch_id"].as_str().unwrap().to_string()
}

fn file_under(
    app: &support::TestApp,
    key: &str,
    branch_id: &str,
    tag_id: &str,
    carried: bool,
) -> Value {
    let (status, answer) = app.post_op(
        "set_recipe_tag",
        Some(key),
        &json!({ "branch_id": branch_id, "tag_id": tag_id, "carried": carried }).to_string(),
    );
    assert_eq!(status, 200, "{answer}");
    answer["result"].clone()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_tag_belongs_to_a_kitchen_never_to_the_instance_and_never_to_a_recipe() {
    let app = support::spawn_app();
    let (_, key_a, kitchen_a) = person_with_kitchen(&app, "Aurélien");
    let (_, key_b, kitchen_b) = person_with_kitchen(&app, "Marc");

    // The same word in two Kitchens is two Tags. What one Kitchen means by
    // "quick" is its own business (ADR 0007).
    let quick_a = tag_in(&app, &key_a, &kitchen_a, "en", "quick");
    let quick_b = tag_in(&app, &key_b, &kitchen_b, "en", "quick");
    assert_ne!(quick_a, quick_b, "one word, two Kitchens, two Tags");

    // Neither Kitchen's list mentions the other's.
    let (_, listed) = app.post_op(
        "list_tags",
        Some(&key_a),
        &json!({ "kitchen_id": kitchen_a }).to_string(),
    );
    let tags = listed["result"]["tags"].as_array().unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0]["id"], json!(quick_a));

    // And a stranger to the Kitchen may not read its filing at all.
    let (status, refused) = app.post_op(
        "list_tags",
        Some(&key_b),
        &json!({ "kitchen_id": kitchen_a }).to_string(),
    );
    assert_eq!(status, 401, "{refused}");

    // A recipe cannot be filed under another Kitchen's word.
    let branch = recipe_in(&app, &key_a, &kitchen_a, "Tarte aux pommes");
    let (status, refused) = app.post_op(
        "set_recipe_tag",
        Some(&key_a),
        &json!({ "branch_id": branch, "tag_id": quick_b, "carried": true }).to_string(),
    );
    assert_eq!(status, 404, "{refused}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn one_word_twice_is_one_tag_and_case_does_not_make_a_second() {
    let app = support::spawn_app();
    let (_, key, kitchen) = person_with_kitchen(&app, "Aurélien");

    let first = tag_in(&app, &key, &kitchen, "en", "dessert");
    let again = tag_in(&app, &key, &kitchen, "en", "dessert");
    let shouted = tag_in(&app, &key, &kitchen, "en", "Dessert");
    assert_eq!(first, again, "the same word is the same Tag");
    assert_eq!(first, shouted, "case is not a second Tag");

    let (_, listed) = app.post_op(
        "list_tags",
        Some(&key),
        &json!({ "kitchen_id": kitchen }).to_string(),
    );
    assert_eq!(listed["result"]["tags"].as_array().unwrap().len(), 1);

    // The first spelling is the one kept: a Tag is not renamed by someone
    // reaching for it in a hurry.
    assert_eq!(listed["result"]["tags"][0]["name"], json!("dessert"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn one_word_is_one_tag_past_the_ascii_alphabet() {
    // Kamosu is written in three Languages, two of them accented, so a fold
    // that stops at ASCII is a fold that does not work here.
    let app = support::spawn_app();
    let (_, key, kitchen) = person_with_kitchen(&app, "Aurélien");

    // Case, above the ASCII range.
    let summer = tag_in(&app, &key, &kitchen, "fr", "été");
    assert_eq!(
        tag_in(&app, &key, &kitchen, "fr", "Été"),
        summer,
        "an accented capital is the same word"
    );
    assert_eq!(
        tag_in(&app, &key, &kitchen, "fr", "ÉTÉ"),
        summer,
        "shouted is the same word"
    );

    // Shape: the same word typed two ways. "é" as one character, and "e"
    // followed by a combining acute accent — identical on screen, different
    // bytes, and a French keyboard produces both depending on the machine.
    let precomposed = "crème";
    let decomposed = "cre\u{0300}me";
    assert_ne!(precomposed, decomposed, "these differ as bytes");
    let cream = tag_in(&app, &key, &kitchen, "fr", precomposed);
    assert_eq!(
        tag_in(&app, &key, &kitchen, "fr", decomposed),
        cream,
        "one word typed two ways is one Tag"
    );

    let (_, listed) = app.post_op(
        "list_tags",
        Some(&key),
        &json!({ "kitchen_id": kitchen }).to_string(),
    );
    let tags = listed["result"]["tags"].as_array().unwrap();
    assert_eq!(tags.len(), 2, "été and crème, once each: {listed}");

    // And Spanish, the third Language, folds too.
    let quick = tag_in(&app, &key, &kitchen, "es", "rápido");
    assert_eq!(tag_in(&app, &key, &kitchen, "es", "RÁPIDO"), quick);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_tag_holds_a_name_per_language_and_is_shown_in_the_readers_own() {
    let app = support::spawn_app();
    let (_, key, kitchen) = person_with_kitchen(&app, "Aurélien");
    let (_, reader_key, _) = person_with_kitchen(&app, "Marc");

    let tag = tag_in(&app, &key, &kitchen, "en", "dessert");
    let (status, renamed) = app.post_op(
        "rename_tag",
        Some(&key),
        &json!({ "tag_id": tag, "language": "fr", "name": "dessert sucré" }).to_string(),
    );
    assert_eq!(status, 200, "{renamed}");

    // One Tag, two words — not two Tags split by Language (ADR 0006).
    assert_eq!(
        renamed["result"]["names"],
        json!([
            { "language": "en", "name": "dessert" },
            { "language": "fr", "name": "dessert sucré" },
        ])
    );

    // The reader reads English, so English is what they are shown.
    assert_eq!(renamed["result"]["name"], json!("dessert"));
    assert_eq!(renamed["result"]["language"], json!("en"));

    // Marc joins the Kitchen and reads French: the same Tag, his word.
    let (_, invite) = app.post_op(
        "invite_to_kitchen",
        Some(&key),
        &json!({ "kitchen_id": kitchen }).to_string(),
    );
    let secret = invite["result"]["secret"].as_str().unwrap();
    app.post_op(
        "accept_kitchen_invite",
        Some(&reader_key),
        &json!({ "secret": secret }).to_string(),
    );
    app.post_op(
        "set_reading_preferences",
        Some(&reader_key),
        &json!({ "reading_language": "fr", "reading_measures": "metric" }).to_string(),
    );
    let (_, listed) = app.post_op(
        "list_tags",
        Some(&reader_key),
        &json!({ "kitchen_id": kitchen }).to_string(),
    );
    assert_eq!(listed["result"]["tags"][0]["name"], json!("dessert sucré"));
    assert_eq!(listed["result"]["tags"][0]["language"], json!("fr"));

    // A Tag with no Spanish name still shows a word rather than a blank: it
    // falls back to whatever it does have, and says which Language that is.
    app.post_op(
        "set_reading_preferences",
        Some(&reader_key),
        &json!({ "reading_language": "es", "reading_measures": "metric" }).to_string(),
    );
    let (_, fallen_back) = app.post_op(
        "list_tags",
        Some(&reader_key),
        &json!({ "kitchen_id": kitchen }).to_string(),
    );
    assert_eq!(fallen_back["result"]["tags"][0]["name"], json!("dessert"));
    assert_eq!(fallen_back["result"]["tags"][0]["language"], json!("en"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn renaming_a_tag_reaches_every_recipe_carrying_it_immediately() {
    let app = support::spawn_app();
    let (_, key, kitchen) = person_with_kitchen(&app, "Aurélien");

    let tag = tag_in(&app, &key, &kitchen, "en", "desert");
    let tarte = recipe_in(&app, &key, &kitchen, "Tarte aux pommes");
    let mousse = recipe_in(&app, &key, &kitchen, "Mousse au chocolat");
    file_under(&app, &key, &tarte, &tag, true);
    file_under(&app, &key, &mousse, &tag, true);

    // One rename of the misspelling, no walk over the recipes carrying it.
    let (status, renamed) = app.post_op(
        "rename_tag",
        Some(&key),
        &json!({ "tag_id": tag, "language": "en", "name": "dessert" }).to_string(),
    );
    assert_eq!(status, 200, "{renamed}");

    for branch in [&tarte, &mousse] {
        let (_, read) = app.post_op(
            "get_recipe",
            Some(&key),
            &json!({ "branch_id": branch }).to_string(),
        );
        assert_eq!(
            read["result"]["tags"][0]["name"],
            json!("dessert"),
            "the rename reached this recipe with nothing else asked of it"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn filing_a_recipe_never_touches_its_fingerprint_or_its_thread() {
    // ADR 0035: a Tag is how a Kitchen files a recipe, not what the recipe is.
    // So none of tagging, untagging or renaming may mint a Version, move the
    // head fingerprint, or leave a mark in the Thread.
    let app = support::spawn_app();
    let (_, key, kitchen) = person_with_kitchen(&app, "Aurélien");

    let branch = recipe_in(&app, &key, &kitchen, "Tarte aux pommes");
    let (_, before) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch }).to_string(),
    );
    let head_before = before["result"]["head_version_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        before["result"]["tags"],
        json!([]),
        "filed under nothing yet"
    );

    let tag = tag_in(&app, &key, &kitchen, "en", "desert");
    let filed = file_under(&app, &key, &branch, &tag, true);
    assert_eq!(filed["tags"][0]["id"], json!(tag));

    app.post_op(
        "rename_tag",
        Some(&key),
        &json!({ "tag_id": tag, "language": "en", "name": "dessert" }).to_string(),
    );
    file_under(&app, &key, &branch, &tag, false);
    file_under(&app, &key, &branch, &tag, true);

    let (_, after) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch }).to_string(),
    );
    assert_eq!(
        after["result"]["head_version_id"],
        json!(head_before),
        "filing moved the fingerprint — a Tag has got inside the Version's content"
    );
    assert_eq!(
        after["result"]["versions"].as_array().unwrap().len(),
        1,
        "filing left a mark in the Thread; it must mint no Version at all"
    );
    // And no Version's content carries the word anywhere.
    let content = after["result"]["versions"][0]["content"].to_string();
    assert!(
        !content.contains("dessert") && !content.contains("desert"),
        "a Tag appeared inside a Version's content: {content}"
    );
    assert_eq!(after["result"]["tags"][0]["name"], json!("dessert"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deleting_a_tag_takes_it_off_every_recipe_and_changes_no_recipe() {
    let app = support::spawn_app();
    let (_, key, kitchen) = person_with_kitchen(&app, "Aurélien");

    let tag = tag_in(&app, &key, &kitchen, "en", "quick");
    let branch = recipe_in(&app, &key, &kitchen, "Omelette");
    file_under(&app, &key, &branch, &tag, true);
    let (_, before) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch }).to_string(),
    );
    let head_before = before["result"]["head_version_id"].clone();

    let (status, deleted) = app.post_op(
        "delete_tag",
        Some(&key),
        &json!({ "tag_id": tag }).to_string(),
    );
    assert_eq!(status, 200, "{deleted}");

    let (_, after) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch }).to_string(),
    );
    assert_eq!(after["result"]["tags"], json!([]));
    assert_eq!(
        after["result"]["head_version_id"], head_before,
        "the recipe itself is untouched"
    );
    let (_, listed) = app.post_op(
        "list_tags",
        Some(&key),
        &json!({ "kitchen_id": kitchen }).to_string(),
    );
    assert_eq!(listed["result"]["tags"], json!([]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn merging_two_tags_carries_every_recipe_across_and_mints_no_version() {
    // CONTEXT.md, "Tag": renaming *or merging* one reaches all of them at once.
    let app = support::spawn_app();
    let (_, key, kitchen) = person_with_kitchen(&app, "Aurélien");

    let pudding = tag_in(&app, &key, &kitchen, "en", "pudding");
    let sweet = tag_in(&app, &key, &kitchen, "en", "sweet things");
    // The losing Tag carries a French word the winner has not got.
    app.post_op(
        "rename_tag",
        Some(&key),
        &json!({ "tag_id": sweet, "language": "fr", "name": "sucré" }).to_string(),
    );

    let tarte = recipe_in(&app, &key, &kitchen, "Tarte aux pommes");
    let mousse = recipe_in(&app, &key, &kitchen, "Mousse au chocolat");
    file_under(&app, &key, &tarte, &sweet, true);
    file_under(&app, &key, &mousse, &sweet, true);
    // One recipe already carries both, which the merge must not double up.
    file_under(&app, &key, &mousse, &pudding, true);

    let (_, before) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": tarte }).to_string(),
    );
    let head_before = before["result"]["head_version_id"].clone();

    let (status, merged) = app.post_op(
        "merge_tags",
        Some(&key),
        &json!({ "keep_tag_id": pudding, "merge_tag_id": sweet }).to_string(),
    );
    assert_eq!(status, 200, "{merged}");
    assert_eq!(merged["result"]["id"], json!(pudding));

    // The kept Tag keeps its own word, and adopts the one it did not have.
    assert_eq!(
        merged["result"]["names"],
        json!([
            { "language": "en", "name": "pudding" },
            { "language": "fr", "name": "sucré" },
        ])
    );

    // One Tag left in the Kitchen, and every recipe is under it exactly once.
    let (_, listed) = app.post_op(
        "list_tags",
        Some(&key),
        &json!({ "kitchen_id": kitchen }).to_string(),
    );
    assert_eq!(listed["result"]["tags"].as_array().unwrap().len(), 1);
    for branch in [&tarte, &mousse] {
        let (_, read) = app.post_op(
            "get_recipe",
            Some(&key),
            &json!({ "branch_id": branch }).to_string(),
        );
        let tags = read["result"]["tags"].as_array().unwrap();
        assert_eq!(tags.len(), 1, "one Tag, not two: {read}");
        assert_eq!(tags[0]["id"], json!(pudding));
    }

    // And the recipes themselves never moved (ADR 0035).
    let (_, after) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": tarte }).to_string(),
    );
    assert_eq!(after["result"]["head_version_id"], head_before);
    assert_eq!(after["result"]["versions"].as_array().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_tag_is_merged_only_within_one_kitchen_and_never_into_itself() {
    let app = support::spawn_app();
    let (_, key_a, kitchen_a) = person_with_kitchen(&app, "Aurélien");
    let (_, key_b, kitchen_b) = person_with_kitchen(&app, "Marc");

    let mine = tag_in(&app, &key_a, &kitchen_a, "en", "quick");
    let theirs = tag_in(&app, &key_b, &kitchen_b, "en", "quick");

    let (status, refused) = app.post_op(
        "merge_tags",
        Some(&key_a),
        &json!({ "keep_tag_id": mine, "merge_tag_id": theirs }).to_string(),
    );
    // Marc's Tag is not this caller's to hear about at all, so naming it
    // answers exactly as an unminted Tag id does (ADR 0040). The
    // different-Kitchens refusal below is for two Kitchens you cook in.
    assert_eq!(status, 404, "{refused}");
    assert_eq!(
        refused["error"]["message"],
        json!("no such Tag"),
        "{refused}"
    );

    let (status, refused) = app.post_op(
        "merge_tags",
        Some(&key_a),
        &json!({ "keep_tag_id": mine, "merge_tag_id": mine }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_rename_onto_a_word_the_kitchen_already_files_by_is_refused() {
    let app = support::spawn_app();
    let (_, key, kitchen) = person_with_kitchen(&app, "Aurélien");

    let dessert = tag_in(&app, &key, &kitchen, "en", "dessert");
    let quick = tag_in(&app, &key, &kitchen, "en", "quick");

    let (status, refused) = app.post_op(
        "rename_tag",
        Some(&key),
        &json!({ "tag_id": quick, "language": "en", "name": "dessert" }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");

    // But a Tag may be respelt into its own current word — that is a change of
    // spelling, not a collision with itself.
    let (status, respelt) = app.post_op(
        "rename_tag",
        Some(&key),
        &json!({ "tag_id": dessert, "language": "en", "name": "Dessert" }).to_string(),
    );
    assert_eq!(status, 200, "{respelt}");
    assert_eq!(respelt["result"]["name"], json!("Dessert"));
}

// --- Related Recipes (issue #52) ---------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn related_recipes_are_one_two_way_shelf_local_link_that_keeps_a_departed_name() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_, curry) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Curry" }).to_string(),
    );
    let (_, naan) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Naan" }).to_string(),
    );
    let curry_branch = curry["result"]["branch_id"].as_str().unwrap();
    let naan_branch = naan["result"]["branch_id"].as_str().unwrap();
    let curry_lineage = curry["result"]["lineage_id"].as_str().unwrap();
    let naan_lineage = naan["result"]["lineage_id"].as_str().unwrap();

    let (status, linked) = app.post_op(
        "set_related_recipe",
        Some(&key),
        &json!({
            "branch_id": curry_branch,
            "related_branch_id": naan_branch,
            "related": true,
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{linked}");
    assert_eq!(
        linked["result"]["related_recipes"],
        json!([{ "lineage_id": naan_lineage, "branch_id": naan_branch, "title": "Naan" }]),
    );

    // Repeating the request from the other end still leaves one link, visible
    // from both Lineages rather than two directed pointers.
    let (status, reverse) = app.post_op(
        "set_related_recipe",
        Some(&key),
        &json!({
            "branch_id": naan_branch,
            "related_branch_id": curry_branch,
            "related": true,
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{reverse}");
    assert_eq!(
        reverse["result"]["related_recipes"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let (_, curry_read) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": curry_branch }).to_string(),
    );
    assert_eq!(
        curry_read["result"]["related_recipes"],
        json!([{ "lineage_id": naan_lineage, "branch_id": naan_branch, "title": "Naan" }]),
    );
    assert_ne!(
        curry_lineage, naan_lineage,
        "a Related Recipe never joins Lineages"
    );
    assert_eq!(
        curry_read["result"]["head_version_id"], curry["result"]["head_version_id"],
        "a shelf link never changes a Version fingerprint"
    );

    // A future deletion or move off this shelf leaves the remembered name, not
    // an unusable pointer. Moving the Branch is setup only: every assertion
    // above and below crosses a real Door.
    let (_, elsewhere) = app.post_op(
        "create_kitchen",
        Some(&key),
        &json!({ "name": "Elsewhere" }).to_string(),
    );
    let elsewhere_id = elsewhere["result"]["id"].as_str().unwrap();
    app.core
        .db()
        .with_conn(|conn| {
            conn.execute(
                "UPDATE branches SET kitchen_id = ?1 WHERE id = ?2",
                rusqlite::params![elsewhere_id, naan_branch],
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            Ok(())
        })
        .expect("move related Branch off this shelf");
    let (_, departed) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": curry_branch }).to_string(),
    );
    assert_eq!(
        departed["result"]["related_recipes"],
        json!([{ "lineage_id": naan_lineage, "branch_id": null, "title": "Naan" }]),
    );

    // Either end may remove the one shared link.
    let (status, _refused) = app.post_op(
        "set_related_recipe",
        Some(&key),
        &json!({
            "branch_id": curry_branch,
            "related_branch_id": naan_branch,
            "related": false,
        })
        .to_string(),
    );
    assert_eq!(
        status, 404,
        "a Branch no longer on this shelf cannot be addressed"
    );
    // Put it back, then end the link from Naan's end.
    app.core
        .db()
        .with_conn(|conn| {
            conn.execute(
                "UPDATE branches SET kitchen_id = ?1 WHERE id = ?2",
                rusqlite::params![kitchen_id, naan_branch],
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            Ok(())
        })
        .expect("return related Branch to this shelf");
    let (status, unlinked) = app.post_op(
        "set_related_recipe",
        Some(&key),
        &json!({
            "branch_id": naan_branch,
            "related_branch_id": curry_branch,
            "related": false,
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{unlinked}");
    assert_eq!(unlinked["result"]["related_recipes"], json!([]));
    let (_, final_curry) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": curry_branch }).to_string(),
    );
    assert_eq!(final_curry["result"]["related_recipes"], json!([]));
}

// --- Access Keys and Sessions (issue #41) ------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn minting_an_access_key_shows_the_secret_once_afterwards_known_by_name_and_last_use() {
    let app = support::spawn_app();
    let create = json!({ "name": "Aurélien", "password": "the right password", "session_name": "first browser" });
    let created = app.post_auth_response("/auth/first-person", &create.to_string());
    assert_eq!(created.status, 200, "{}", created.text());
    let session_secret = support::session_cookie_secret(&created);

    // Minting an Access Key is a Person's own act — never an Access Key's, so
    // this is asked with the Session the account creation just minted.
    let (status, minted) = app.post_op(
        "mint_access_key",
        Some(&session_secret),
        r#"{"name":"my agent","read_only":false}"#,
    );
    assert_eq!(status, 200, "{minted}");
    let secret = minted["result"]["secret"]
        .as_str()
        .expect("the secret, shown once")
        .to_string();
    assert_eq!(minted["result"]["name"], json!("my agent"));
    assert_eq!(minted["result"]["read_only"], json!(false));
    assert!(minted["result"]["id"].as_str().is_some());

    // The new Key works as a Credential in its own right.
    assert_eq!(app.post_op("list_jobs", Some(&secret), "{}").0, 200);

    // And it appears in the list by name and last use — never by its Secret,
    // which the listing carries nowhere.
    let (_, listed) = app.post_op("list_access_keys", Some(&session_secret), "{}");
    let keys = listed["result"]["access_keys"].as_array().expect("keys");
    let mine = keys
        .iter()
        .find(|k| k["name"] == json!("my agent"))
        .expect("the minted Key is listed");
    assert!(mine["last_used_at"].is_string(), "used just now, above");
    assert_eq!(mine["revoked"], json!(false));
    for key_entry in keys {
        assert!(
            key_entry.get("secret").is_none(),
            "the listing must never carry a Secret: {key_entry}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_read_only_access_key_is_refused_every_writing_operation_at_both_doors() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let read_only_key = app
        .core
        .mint_access_key(&person, "read-only agent", true)
        .unwrap()
        .secret;

    // Reading still works.
    assert_eq!(
        app.post_op("list_sessions", Some(&read_only_key), "{}").0,
        200
    );

    // Writing is refused — at the web door...
    let (status, refused) = app.post_op(
        "rename_person",
        Some(&read_only_key),
        r#"{"name":"New Name"}"#,
    );
    assert_eq!(status, 401, "{refused}");
    assert_eq!(refused["error"]["kind"], json!("unauthorized"));

    // ...and at the MCP door, the same Operation through the same Core.
    let call = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": { "name": "rename_person", "arguments": { "name": "New Name" } },
    });
    let (_, mcp_refused) = app.post_mcp(&call.to_string(), Some(&read_only_key));
    assert_eq!(
        mcp_refused["result"]["isError"],
        json!(true),
        "{mcp_refused}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_access_key_cannot_mint_an_access_key_however_unrestricted() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let full_power_key = app
        .core
        .mint_access_key(&person, "full agent", false)
        .unwrap()
        .secret;

    let (status, refused) = app.post_op(
        "mint_access_key",
        Some(&full_power_key),
        r#"{"name":"a second key"}"#,
    );
    assert_eq!(
        status, 401,
        "an Access Key minted another Access Key: {refused}"
    );
    assert_eq!(refused["error"]["kind"], json!("unauthorized"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revoking_one_access_key_leaves_the_others_working() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let admin_key = app
        .core
        .mint_access_key(&person, "admin", false)
        .unwrap()
        .secret;
    let doomed = app.core.mint_access_key(&person, "doomed", false).unwrap();
    let survivor = app
        .core
        .mint_access_key(&person, "survivor", false)
        .unwrap()
        .secret;

    let (status, revoked) = app.post_op(
        "revoke_access_key",
        Some(&admin_key),
        &json!({ "access_key_id": doomed.id }).to_string(),
    );
    assert_eq!(status, 200, "{revoked}");
    assert_eq!(revoked["result"]["revoked"], json!(true));

    assert_eq!(
        app.post_op("instance_status", Some(&doomed.secret), "{}").0,
        401,
        "the revoked Key still worked"
    );
    assert_eq!(
        app.post_op("instance_status", Some(&survivor), "{}").0,
        200,
        "revoking one Key must not touch another"
    );

    // Revoking it again finds nothing live to revoke.
    let (status, _) = app.post_op(
        "revoke_access_key",
        Some(&admin_key),
        &json!({ "access_key_id": doomed.id }).to_string(),
    );
    assert_eq!(status, 404);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn listing_mcp_tools_does_not_itself_count_as_using_the_access_key() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let admin_key = app
        .core
        .mint_access_key(&person, "admin", false)
        .unwrap()
        .secret;
    let watched = app.core.mint_access_key(&person, "watched", false).unwrap();

    // Merely asking what tools a Key may use must not read as the Key having
    // been used — otherwise a compromised, idle Key polling tools/list would
    // look active in the Sessions-and-Keys list a Person actually watches.
    app.post_mcp(
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
        Some(&watched.secret),
    );

    let (_, listed) = app.post_op("list_access_keys", Some(&admin_key), "{}");
    let mine = listed["result"]["access_keys"]
        .as_array()
        .expect("keys")
        .iter()
        .find(|k| k["id"] == json!(watched.id))
        .expect("the watched Key is listed");
    assert_eq!(
        mine["last_used_at"],
        Value::Null,
        "listing tools must not bump last_used_at: {mine}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn login_throttling_slows_guesses_but_a_correct_password_clears_it() {
    let app = support::spawn_app();
    let create = json!({ "name": "Aurélien", "password": "the right password", "session_name": "first browser" });
    assert_eq!(
        app.post_auth("/auth/first-person", &create.to_string()).0,
        200
    );
    let wrong =
        json!({ "name": "Aurélien", "password": "wrong", "session_name": "laptop" }).to_string();
    assert_eq!(app.post_auth("/auth/login", &wrong).0, 401);
    assert_eq!(app.post_auth("/auth/login", &wrong).0, 401);
    let started = Instant::now();
    assert_eq!(app.post_auth("/auth/login", &wrong).0, 401);
    assert!(
        started.elapsed() >= Duration::from_secs(1),
        "the third guess must wait"
    );

    let correct =
        json!({ "name": "Aurélien", "password": "the right password", "session_name": "laptop" })
            .to_string();
    assert_eq!(app.post_auth("/auth/login", &correct).0, 200);
    let started = Instant::now();
    assert_eq!(app.post_auth("/auth/login", &wrong).0, 401);
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "a correct password clears the throttle"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn instance_status_refuses_input_it_does_not_declare() {
    let app = support::spawn_app();

    let (status, body) = app.post_op("instance_status", None, r#"{"unexpected":1}"#);
    assert_eq!(status, 400, "{body}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn setup_happening_is_visible_at_both_doors() {
    let app = support::spawn_app();
    app.core.mark_setup_complete().expect("mark setup");

    let (_, body) = app.post_op("instance_status", None, "{}");
    assert_eq!(body["result"]["setup_complete"], json!(true));

    let payload = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"instance_status","arguments":{}}}"#;
    let (_, mcp) = app.post_mcp(payload, None);
    let structured = &mcp["result"]["structuredContent"];
    assert_eq!(
        structured["setup_complete"],
        json!(true),
        "the two doors disagree"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_real_credential_names_a_person_at_the_web_door() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let secret = app
        .core
        .mint_access_key(&person, "my agent", false)
        .expect("key")
        .secret;

    // A recognised Credential passes.
    let (status, _) = app.post_op("instance_status", Some(&secret), "{}");
    assert_eq!(status, 200);

    // An unrecognised Secret fails loudly rather than silently becoming a stranger.
    let (status, body) = app.post_op("instance_status", Some("not-a-real-secret"), "{}");
    assert_eq!(status, 401, "{body}");

    // A revoked Credential stops naming anyone.
    app.core
        .db()
        .with_conn(|conn| {
            conn.execute("UPDATE access_keys SET revoked = 1", [])
                .unwrap();
            Ok(())
        })
        .expect("revoke");
    let (status, body) = app.post_op("instance_status", Some(&secret), "{}");
    assert_eq!(status, 401, "a revoked key still worked ({body})");

    // And no credential at all is simply a stranger — fine for a public Operation.
    let (status, body) = app.post_op("instance_status", None, "{}");
    assert_eq!(status, 200, "{body}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_mcp_door_speaks_one_revision_stateless_with_no_handshake() {
    let app = support::spawn_app();

    // A request with no handshake at all just works.
    let (status, body) =
        app.post_mcp(r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"instance_status","arguments":{}}}"#, None);
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body["result"]["structuredContent"]["version"],
        json!(env!("CARGO_PKG_VERSION"))
    );

    // A legacy initialize receives a courteous error naming the version spoken.
    let (_, body) = app.post_mcp(
        r#"{"jsonrpc":"2.0","id":2,"method":"initialize","params":{}}"#,
        None,
    );
    let message = body["error"]["message"]
        .as_str()
        .expect("courteous message");
    assert!(
        message.contains(MCP_PROTOCOL_VERSION),
        "the refusal must name {MCP_PROTOCOL_VERSION}: {message}"
    );
    assert!(
        message.to_lowercase().contains("stateless"),
        "the refusal should say why: {message}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn everything_durable_lives_under_one_data_directory() {
    let app = support::spawn_app();

    // The database file exists under the data directory and is genuinely SQLite,
    // not an in-memory double.
    let db_path = app.core.database_path();
    assert!(
        db_path.exists(),
        "database must be a file under /data: {}",
        db_path.display()
    );
    assert!(
        db_path.starts_with(app.data_dir().expect("the helper owns a temp dir")),
        "database must live under the one data dir"
    );

    // WAL mode is a requirement, not a detail (ADR 0028): a second process opens
    // the same file while the server runs.
    let mode: String = app
        .core
        .db()
        .with_conn(|conn| {
            Ok(conn
                .query_row("PRAGMA journal_mode", [], |r| r.get(0))
                .unwrap())
        })
        .expect("journal mode");
    assert_eq!(mode.to_lowercase(), "wal");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_screen_is_one_shell_served_by_the_binary_itself() {
    let app = support::spawn_app();

    // The interface is compiled into the binary (ADR 0028), and SvelteKit runs
    // on adapter-static with a fallback: the router runs in the browser, so
    // every screen path is the same document arriving at a different address.
    let (status, content_type, shell) = app.get("/");
    assert_eq!(status, 200, "the app serves without a Credential");
    assert!(content_type.starts_with("text/html"), "{content_type}");
    assert!(
        shell.contains("/_app/immutable/"),
        "the shell names the built app's chunks: {shell:.200}"
    );

    for path in ["/recipes", "/shopping", "/cooked", "/settings", "/tokens"] {
        let (status, content_type, body) = app.get(path);
        assert_eq!(status, 200, "{path}");
        assert!(
            content_type.starts_with("text/html"),
            "{path}: {content_type}"
        );
        assert_eq!(body, shell, "{path} is the same shell");
    }

    // Something that asked for a file is told the file is not there, rather than
    // handed HTML it will fail to parse later and less clearly.
    let (status, _, _) = app.get("/_app/immutable/nothing-like-this.js");
    assert_eq!(status, 404);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_path_that_is_not_an_operation_is_still_answered_as_a_catalogue_question() {
    let app = support::spawn_app();

    // The interface owns the fallback now, but `/api/op/…` is a question about
    // the Catalogue wherever it lands, and is answered as one.
    let (status, body) = app.post_op("not_an_operation", None, "{}");
    assert_eq!(status, 404);
    assert_eq!(body["ok"], false);
    assert_eq!(body["error"]["kind"], "unknown_operation");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_web_manifest_points_at_the_icons_the_binary_serves() {
    let app = support::spawn_app();

    let (status, content_type, body) = app.get("/manifest.webmanifest");
    assert_eq!(status, 200);
    assert!(
        content_type.starts_with("application/manifest+json"),
        "a web manifest has its own registered media type: {content_type}"
    );

    let manifest: serde_json::Value = serde_json::from_str(&body).expect("a manifest");
    let icons = manifest["icons"].as_array().expect("icons");
    assert!(!icons.is_empty());
    for icon in icons {
        let src = icon["src"].as_str().expect("a src");
        assert!(src.starts_with("/assets/icons/"), "{src}");
        let (status, _, _) = app.get(src);
        assert_eq!(status, 200, "the manifest names {src}, so it must serve");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn one_generated_stylesheet_serves_the_whole_look_and_no_tailwind_defaults() {
    let app = support::spawn_app();

    let (status, content_type, body) = app.get("/assets/app.css");
    assert_eq!(status, 200);
    assert!(content_type.starts_with("text/css"), "{content_type}");
    // The identity's tokens are in it...
    assert!(body.contains("--color-ground: #f4efe3"), "kinari ground");
    // The RANKING that ADR 0011 records as spec — the Step is the largest type
    // in the app — is asserted where the stylesheet is parsed properly, in
    // `design_tokens.rs`. This test's job is that the binary SERVES that
    // stylesheet, so it checks the tokens are in what came over the wire and
    // leaves the comparing to the parser that already exists. A second, weaker
    // parser here was how this test came to assert a literal `33px` under a
    // sentence about ranking, and then fail for the wrong reason when #88 moved
    // the size while keeping the ranking intact.
    assert!(body.contains("--text-step:"), "the Step's size is served");
    assert!(body.contains("--text-title:"), "the title's size is served");
    assert!(body.contains("--spacing-gutter: 20px"));
    // ...and Tailwind's own palette is not reachable.
    assert!(!body.contains("--color-red-"), "no default red");
    assert!(!body.contains("--color-slate-"), "no default slate");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fonts_icons_and_mark_are_self_hosted_under_the_binary() {
    let app = support::spawn_app();

    for path in [
        "/assets/fonts/zen-old-mincho-600-latin.woff2",
        "/assets/fonts/zen-kaku-gothic-new-400-latin.woff2",
        "/assets/icons/icon-192.png",
        "/assets/icons/apple-touch-icon.png",
        "/assets/img/kamosu-mark.svg",
        "/favicon.svg",
    ] {
        let (status, content_type, _) = app.get(path);
        assert_eq!(status, 200, "{path}");
        assert!(
            content_type.starts_with("font/") || content_type.starts_with("image/"),
            "{path}: {content_type}"
        );
    }

    // An unknown font is a loud 404, not a silent empty response.
    let (status, _, _) = app.get("/assets/fonts/not-a-font.woff2");
    assert_eq!(status, 404);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_job_that_fails_reports_why_through_the_same_operations() {
    let app = support::spawn_app();

    let (_, ask) = app.post_op(
        "probe_job",
        None,
        r#"{"steps":2,"delay_ms":25,"fail":true}"#,
    );
    let job_id = ask["result"]["job_id"]
        .as_str()
        .expect("a job id")
        .to_string();

    let finished = support::wait_terminal(&app, None, &job_id);
    assert_eq!(finished["status"], json!("failed"), "{finished}");
    let reason = finished["error"].as_str().expect("a failure reason");
    assert!(
        reason.contains("asked to fail"),
        "the reason must say why: {reason}"
    );

    // The MCP door reports the same ending through the task's error field,
    // with the JSON-RPC code the Operation's failure maps onto.
    let call = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "probe_job",
            "arguments": { "steps": 1, "delay_ms": 10, "fail": true },
            "_meta": tasks_meta()["_meta"],
        },
    });
    let (_, created) = app.post_mcp(&call.to_string(), None);
    let task_id = created["result"]["taskId"]
        .as_str()
        .expect("taskId")
        .to_string();

    let poll = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tasks/get",
        "params": { "taskId": task_id, "_meta": tasks_meta()["_meta"] },
    });
    for _ in 0..400 {
        let (_, body) = app.post_mcp(&poll.to_string(), None);
        let task = &body["result"];
        match task["status"].as_str().unwrap_or("") {
            "working" => std::thread::sleep(Duration::from_millis(25)),
            "failed" => {
                assert_eq!(task["error"]["code"], json!(-32602), "{task}");
                assert!(
                    task["error"]["message"]
                        .as_str()
                        .unwrap_or("")
                        .contains("asked to fail")
                );
                return;
            }
            other => panic!("unexpected task state {other}: {task}"),
        }
    }
    panic!("the failing task never ended");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stranger_may_cause_work_but_never_work_that_scales_with_them() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "member", false)
        .unwrap()
        .secret;

    // The strangers' lane carries one at a time behind a short bounded line.
    // Fill it: five asks are accepted (one working, four in line), and every
    // ask beyond that is told busy rather than joining a queue that grows.
    for _ in 0..5 {
        let (status, body) = app.post_op("probe_job", None, r#"{"steps":40,"delay_ms":25}"#);
        assert_eq!(status, 200, "{body}");
        assert!(body["result"]["job_id"].is_string());
    }
    let (status, body) = app.post_op("probe_job", None, r#"{"steps":40,"delay_ms":25}"#);
    assert_eq!(status, 503, "{body}");
    assert_eq!(body["error"]["kind"], json!("busy"));
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("try again")
    );

    // A member never waits behind strangers: their lane is still open while
    // the strangers' lane is saturated.
    let (status, body) = app.post_op("probe_job", Some(&key), r#"{"steps":40,"delay_ms":25}"#);
    assert_eq!(status, 200, "members have their own lane ({body})");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_job_is_read_by_the_person_who_asked_for_it_and_listed_to_them_alone() {
    let app = support::spawn_app();
    let aurelien = app.core.create_person("Aurélien").expect("person");
    let his_key = app
        .core
        .mint_access_key(&aurelien, "his agent", false)
        .unwrap()
        .secret;
    let marie = app.core.create_person("Marie").expect("person");
    let her_key = app
        .core
        .mint_access_key(&marie, "her agent", false)
        .unwrap()
        .secret;

    let (_, ask) = app.post_op("probe_job", Some(&his_key), r#"{"steps":2,"delay_ms":10}"#);
    let job_id = ask["result"]["job_id"]
        .as_str()
        .expect("a job id")
        .to_string();

    support::wait_terminal(&app, Some(&his_key), &job_id);

    // Marie asking about his Job learns nothing: it is not hers to read, and
    // the refusal is word for word the one a Job id naming nothing gets, so she
    // cannot tell the two apart (ADR 0040).
    let get = json!({ "job_id": job_id }).to_string();
    let (status, body) = app.post_op("get_job", Some(&her_key), &get);
    assert_eq!(status, 404, "{body}");
    let (absent_status, absent) = app.post_op(
        "get_job",
        Some(&her_key),
        &json!({ "job_id": "j_nothing" }).to_string(),
    );
    assert_eq!(absent_status, 404, "{absent}");
    assert_eq!(
        body["error"]["kind"], absent["error"]["kind"],
        "his Job and no Job answer differently: {body} / {absent}"
    );

    // And it appears in his list of Jobs, not hers.
    let (_, listed) = app.post_op("list_jobs", Some(&his_key), "{}");
    let ids: Vec<&str> = listed["result"]["jobs"]
        .as_array()
        .expect("jobs array")
        .iter()
        .map(|j| j["id"].as_str().expect("id"))
        .collect();
    assert_eq!(ids.len(), 1, "{listed}");
    assert_eq!(ids[0], job_id);

    let (_, her_list) = app.post_op("list_jobs", Some(&her_key), "{}");
    assert!(
        her_list["result"]["jobs"]
            .as_array()
            .expect("jobs")
            .is_empty(),
        "another Person's Job appeared: {her_list}"
    );

    // Work a stranger caused belongs to no Person to hide it from anyone:
    // its state reads publicly, because nothing in it was ever secret.
    let (_, stranger_ask) = app.post_op("probe_job", None, r#"{"steps":1,"delay_ms":10}"#);
    let stranger_id = stranger_ask["result"]["job_id"]
        .as_str()
        .unwrap()
        .to_string();
    support::wait_terminal(&app, None, &stranger_id);
    let (status, _) = app.post_op(
        "get_job",
        None,
        &json!({ "job_id": stranger_id }).to_string(),
    );
    assert_eq!(status, 200);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelling_a_task_is_acknowledged_and_honoured_while_it_waits_in_line() {
    let app = support::spawn_app();

    // Occupy the strangers' lane so the next ask stays queued.
    let (_, first) = app.post_mcp(
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "probe_job",
                "arguments": { "steps": 60, "delay_ms": 25 },
                "_meta": tasks_meta()["_meta"],
            },
        })
        .to_string(),
        None,
    );
    let busy_task = first["result"]["taskId"]
        .as_str()
        .expect("taskId")
        .to_string();

    // This one queues behind it — and is cancelled before any worker takes it.
    let (_, second) = app.post_mcp(
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "probe_job",
                "arguments": { "steps": 60, "delay_ms": 25 },
                "_meta": tasks_meta()["_meta"],
            },
        })
        .to_string(),
        None,
    );
    let queued_task = second["result"]["taskId"]
        .as_str()
        .expect("taskId")
        .to_string();

    let cancel = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tasks/cancel",
        "params": { "taskId": queued_task, "_meta": tasks_meta()["_meta"] },
    });
    let (_, ack) = app.post_mcp(&cancel.to_string(), None);
    assert_eq!(ack["result"]["resultType"], json!("complete"), "{ack}");

    // An unknown taskId is refused, not acknowledged.
    let unknown = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tasks/get",
        "params": { "taskId": "j_nothing", "_meta": tasks_meta()["_meta"] },
    });
    let (_, missing) = app.post_mcp(&unknown.to_string(), None);
    assert_eq!(missing["error"]["code"], json!(-32602), "{missing}");

    // The queued work ends cancelled; the running work was never interrupted.
    let get = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tasks/get",
        "params": { "taskId": queued_task, "_meta": tasks_meta()["_meta"] },
    });
    for _ in 0..100 {
        let (_, body) = app.post_mcp(&get.to_string(), None);
        if body["result"]["status"] == json!("cancelled") {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let (_, final_body) = app.post_mcp(&get.to_string(), None);
    assert_eq!(final_body["result"]["status"], json!("cancelled"));

    // And the one already running finishes normally, untouched by the cancel
    // aimed at its neighbour.
    let running_poll = json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "tasks/get",
        "params": { "taskId": busy_task, "_meta": tasks_meta()["_meta"] },
    });
    for _ in 0..400 {
        let (_, body) = app.post_mcp(&running_poll.to_string(), None);
        if body["result"]["status"] == json!("completed") {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("the running task never completed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn work_accepted_before_a_restart_never_strands() {
    let dir = tempfile::tempdir().expect("temp dir");
    let db = Arc::new(kamosu::db::Db::open(dir.path()).expect("database"));
    let _first = kamosu::core::Core::start(db.clone());

    // Two Jobs a previous process accepted: one waiting in line, one that the
    // shutdown caught mid-run. Written straight into the truth, as a crash
    // would leave them.
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO jobs (id, person_id, operation, input, status)
             VALUES ('j_waiting', NULL, 'probe_job', '{\"steps\":1,\"delay_ms\":10}', 'queued')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO jobs (id, person_id, operation, input, status)
             VALUES ('j_midflight', NULL, 'probe_job', '{\"steps\":9,\"delay_ms\":10}', 'running')",
            [],
        )
        .unwrap();
        Ok(())
    })
    .expect("seed rows");

    // The process comes back. Its recovery must answer for both.
    let second = kamosu::core::Core::start(db.clone());

    let midflight = loop {
        match second.job("j_midflight").expect("read") {
            Some(r) if r.status == kamosu::jobs::JobStatus::Failed => break r,
            Some(_) => std::thread::sleep(Duration::from_millis(25)),
            None => panic!("the interrupted Job vanished"),
        }
    };
    assert!(
        midflight
            .error
            .as_deref()
            .unwrap_or("")
            .contains("interrupted"),
        "the reason must say what happened: {midflight:?}"
    );

    let waiting = loop {
        match second.job("j_waiting").expect("read") {
            Some(r)
                if matches!(
                    r.status,
                    kamosu::jobs::JobStatus::Completed | kamosu::jobs::JobStatus::Failed
                ) =>
            {
                assert_eq!(r.status, kamosu::jobs::JobStatus::Completed, "{r:?}");
                break r;
            }
            Some(_) => std::thread::sleep(Duration::from_millis(25)),
            None => panic!("the waiting Job vanished"),
        }
    };
    assert_eq!(waiting.result.as_ref().unwrap()["steps"], json!(1));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_operator_mints_a_one_use_invite_that_creates_a_person_and_home_kitchen() {
    let app = support::spawn_app();
    let first = json!({
        "name": "Aurélien",
        "password": "the operator password",
        "session_name": "operator browser",
    });
    let operator = app.post_auth_response("/auth/first-person", &first.to_string());
    assert_eq!(operator.status, 200, "{}", operator.text());
    let operator_secret = support::session_cookie_secret(&operator);
    let operator_secret = operator_secret.as_str();

    let (status, minted) = app.post_op("mint_invite", Some(operator_secret), r#"{}"#);
    assert_eq!(status, 200, "{minted}");
    let link = minted["result"]["link"].as_str().expect("invite link");

    let join = json!({
        "link": link,
        "name": "Marie",
        "password": "a password only Marie knows",
        "session_name": "Marie’s browser",
    });
    let (status, joined) = app.post_auth("/auth/invite", &join.to_string());
    assert_eq!(status, 200, "{joined}");
    assert_eq!(joined["result"]["person"]["name"], json!("Marie"));
    assert!(joined["result"]["person"]["home_kitchen_id"].is_string());
    assert_eq!(joined["result"]["person"]["is_operator"], json!(false));

    let (status, spent) = app.post_auth("/auth/invite", &join.to_string());
    assert_eq!(status, 401, "{spent}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_operator_can_disable_delete_and_recover_accounts_without_reading_them() {
    let app = support::spawn_app();
    let first = json!({ "name": "Aurélien", "password": "operator password", "session_name": "operator browser" });
    let operator = app.post_auth_response("/auth/first-person", &first.to_string());
    let operator_secret = support::session_cookie_secret(&operator);
    let operator_secret = operator_secret.as_str();

    let (_, invite) = app.post_op("mint_invite", Some(operator_secret), "{}");
    let marie = json!({ "link": invite["result"]["link"], "name": "Marie", "password": "old password", "session_name": "Marie’s browser" });
    assert_eq!(app.post_auth("/auth/invite", &marie.to_string()).0, 200);

    let (_, recovery) = app.post_op(
        "mint_recovery_link",
        Some(operator_secret),
        r#"{"name":"Marie"}"#,
    );
    let recovered = json!({ "link": recovery["result"]["link"], "password": "new password", "session_name": "replacement browser" });
    let recovery_session = app.post_auth_response("/auth/recover", &recovered.to_string());
    assert_eq!(recovery_session.status, 200, "{}", recovery_session.text());
    assert_eq!(
        app.post_auth("/auth/recover", &recovered.to_string()).0,
        401,
        "a recovery link must be spent"
    );
    assert_eq!(
        app.post_auth(
            "/auth/login",
            &json!({ "name":"Marie", "password":"new password", "session_name":"laptop" })
                .to_string()
        )
        .0,
        200
    );

    let (status, disabled) = app.post_op(
        "disable_account",
        Some(operator_secret),
        r#"{"name":"Marie"}"#,
    );
    assert_eq!(status, 200, "{disabled}");
    assert_eq!(
        app.post_auth(
            "/auth/login",
            &json!({ "name":"Marie", "password":"new password", "session_name":"laptop" })
                .to_string()
        )
        .0,
        401,
        "a disabled account must not obtain a Credential"
    );

    let (_, second_invite) = app.post_op("mint_invite", Some(operator_secret), "{}");
    let zoe = json!({ "link": second_invite["result"]["link"], "name": "Zoé", "password": "her password", "session_name": "Zoé’s browser" });
    assert_eq!(app.post_auth("/auth/invite", &zoe.to_string()).0, 200);
    let (status, deleted) =
        app.post_op("delete_account", Some(operator_secret), r#"{"name":"Zoé"}"#);
    assert_eq!(status, 200, "{deleted}");
    assert_eq!(
        app.post_auth(
            "/auth/login",
            &json!({ "name":"Zoé", "password":"her password", "session_name":"laptop" })
                .to_string()
        )
        .0,
        401
    );
}

/// The instance keeps somebody able to administer it (#103, ADR 0007).
///
/// Every Operation that could appoint an Operator is one only an Operator may
/// call, so an instance that loses its last one cannot be given another
/// through either Door. Until #103 none of these three acts was refused, and
/// the first `delete_account` a curious Operator typed at `curl` would have
/// ended the instance's administration permanently.
#[tokio::test(flavor = "multi_thread")]
async fn the_last_operator_cannot_be_stood_down_disabled_or_deleted() {
    let app = support::spawn_app();
    let first = json!({ "name": "Aurélien", "password": "operator password", "session_name": "operator browser" });
    let operator = app.post_auth_response("/auth/first-person", &first.to_string());
    let operator_secret = support::session_cookie_secret(&operator);
    let operator_secret = operator_secret.as_str();

    // A second Person who is not an Operator changes nothing: the instance
    // still has exactly one Person who can administer it.
    let (_, invite) = app.post_op("mint_invite", Some(operator_secret), "{}");
    let marie = json!({ "link": invite["result"]["link"], "name": "Marie", "password": "her password", "session_name": "Marie’s browser" });
    assert_eq!(app.post_auth("/auth/invite", &marie.to_string()).0, 200);

    for (op, body) in [
        ("set_operator", r#"{"name":"Aurélien","is_operator":false}"#),
        ("disable_account", r#"{"name":"Aurélien"}"#),
        ("delete_account", r#"{"name":"Aurélien"}"#),
    ] {
        let (status, refused) = app.post_op(op, Some(operator_secret), body);
        assert_eq!(status, 400, "{op} must refuse the last Operator: {refused}");
        let said = refused["error"]["message"].as_str().unwrap_or_default();
        assert!(
            said.contains("only Operator"),
            "{op}'s refusal must say why, and said: {said}"
        );
    }

    // The refusals left the instance exactly as it was.
    assert_eq!(
        app.post_auth(
            "/auth/login",
            &json!({ "name":"Aurélien", "password":"operator password", "session_name":"laptop" })
                .to_string()
        )
        .0,
        200,
        "a refused deletion must not have ended the account anyway"
    );
    let (status, listed) = app.post_op("list_accounts", Some(operator_secret), "{}");
    assert_eq!(status, 200, "{listed}");
    let operators = listed["result"]["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|account| account["is_operator"] == json!(true))
        .count();
    assert_eq!(operators, 1, "the one Operator is still an Operator");
}

/// Once somebody else holds it, an Operator may hand it over and step down —
/// which is the world ADR 0007's "the last cannot be demoted" describes, and
/// which nothing in the Catalogue could do before #103.
#[tokio::test(flavor = "multi_thread")]
async fn an_operator_may_step_down_once_somebody_else_administers() {
    let app = support::spawn_app();
    let first = json!({ "name": "Aurélien", "password": "operator password", "session_name": "operator browser" });
    let operator = app.post_auth_response("/auth/first-person", &first.to_string());
    let operator_secret = support::session_cookie_secret(&operator);
    let operator_secret = operator_secret.as_str();

    let (_, invite) = app.post_op("mint_invite", Some(operator_secret), "{}");
    let noor = json!({ "link": invite["result"]["link"], "name": "Noor", "password": "her password", "session_name": "Noor’s browser" });
    let joined = app.post_auth_response("/auth/invite", &noor.to_string());
    assert_eq!(joined.status, 200, "{}", joined.text());
    let noor_secret = support::session_cookie_secret(&joined);
    let noor_secret = noor_secret.as_str();

    // She arrived an ordinary Person, so the screen is not hers to open.
    let (status, _) = app.post_op("list_accounts", Some(noor_secret), "{}");
    assert_eq!(
        status, 401,
        "a Person who is not an Operator sees nothing here"
    );

    let (status, raised) = app.post_op(
        "set_operator",
        Some(operator_secret),
        r#"{"name":"Noor","is_operator":true}"#,
    );
    assert_eq!(status, 200, "{raised}");
    assert_eq!(raised["result"]["is_operator"], json!(true));
    assert_eq!(
        app.post_op("list_accounts", Some(noor_secret), "{}").0,
        200,
        "an Operation's permission is read live, so she may open it at once"
    );

    // Now that Noor holds it too, Aurélien may stand himself down.
    let (status, stood_down) = app.post_op(
        "set_operator",
        Some(operator_secret),
        r#"{"name":"Aurélien","is_operator":false}"#,
    );
    assert_eq!(status, 200, "{stood_down}");
    assert_eq!(
        app.post_op("list_accounts", Some(operator_secret), "{}").0,
        401,
        "he administers nothing now, and the Core says so rather than the screen"
    );

    // And Noor is now the last one, so the guard has simply moved to her.
    let (status, refused) = app.post_op(
        "set_operator",
        Some(noor_secret),
        r#"{"name":"Noor","is_operator":false}"#,
    );
    assert_eq!(status, 400, "{refused}");
}

/// The accounts list names who is here and says nothing about what they cook
/// (#103, ADR 0007 — the Operator administers and does not read).
#[tokio::test(flavor = "multi_thread")]
async fn the_accounts_list_names_who_is_here_and_nothing_they_cooked() {
    let app = support::spawn_app();
    let first = json!({ "name": "Aurélien", "password": "operator password", "session_name": "operator browser" });
    let operator = app.post_auth_response("/auth/first-person", &first.to_string());
    let operator_secret = support::session_cookie_secret(&operator);
    let operator_secret = operator_secret.as_str();

    for who in ["Camille", "Théo"] {
        let (_, invite) = app.post_op("mint_invite", Some(operator_secret), "{}");
        let joining = json!({ "link": invite["result"]["link"], "name": who, "password": "their password", "session_name": "a browser" });
        assert_eq!(app.post_auth("/auth/invite", &joining.to_string()).0, 200);
    }
    assert_eq!(
        app.post_op(
            "disable_account",
            Some(operator_secret),
            r#"{"name":"Théo"}"#
        )
        .0,
        200
    );

    let (status, listed) = app.post_op("list_accounts", Some(operator_secret), "{}");
    assert_eq!(status, 200, "{listed}");
    let accounts = listed["result"]["accounts"].as_array().unwrap();
    assert_eq!(accounts.len(), 3, "{listed}");

    let named = |who: &str| -> serde_json::Value {
        accounts
            .iter()
            .find(|account| account["name"] == json!(who))
            .unwrap_or_else(|| panic!("'{who}' is missing from {listed}"))
            .clone()
    };
    assert_eq!(named("Aurélien")["is_operator"], json!(true));
    assert_eq!(
        named("Aurélien")["is_you"],
        json!(true),
        "the Core says which one is the caller — a name is a reminder, not \
         identification (ADR 0015)"
    );
    assert_eq!(named("Camille")["is_operator"], json!(false));
    assert_eq!(named("Camille")["is_you"], json!(false));
    assert_eq!(
        named("Théo")["disabled"],
        json!(true),
        "a disabled account is still shown — it is the Operator's to undo"
    );

    // ADR 0007's boundary, asserted rather than assumed: nothing here is a way
    // into anybody's cooking.
    let words = listed.to_string();
    for forbidden in ["branch", "recipe", "attempt", "kitchen"] {
        assert!(
            !words.to_lowercase().contains(forbidden),
            "the accounts list must carry no '{forbidden}': {listed}"
        );
    }
}

// --- The Attempt and In Progress (issue #57) ----------------------------------

/// A Recipe with real Ingredients and Steps to advance through, in a fresh
/// Kitchen of its own — `(key, branch_id, lineage_id)`.
fn recipe_ready_to_cook(
    app: &support::TestApp,
    cook_name: &str,
) -> (String, String, String, String) {
    let (person, key, kitchen_id) = person_with_kitchen(app, cook_name);
    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Katsu Curry",
            "ingredients": [
                { "kind": "ingredient", "text": "2 escalopes de poulet" },
                { "kind": "ingredient", "text": "200 g de riz" },
            ],
            "steps": [
                { "kind": "step", "text": "Paner les escalopes" },
                { "kind": "step", "text": "Frire jusqu'à dorer" },
                { "kind": "step", "text": "Servir avec le riz" },
            ],
        })
        .to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();
    let lineage_id = created["result"]["lineage_id"]
        .as_str()
        .unwrap()
        .to_string();
    let _ = person;
    (key, branch_id, lineage_id, kitchen_id)
}

/// Push an Attempt's `last_action_at` into the past, so the three-day
/// resume window can be tested without a real clock to fast-forward.
fn backdate_attempt_action(app: &support::TestApp, attempt_id: &str, days_ago: i64) {
    app.core
        .db()
        .with_conn(|conn| {
            conn.execute(
                "UPDATE attempts SET last_action_at = strftime('%Y-%m-%dT%H:%M:%fZ','now', ?2) \
                 WHERE id = ?1",
                rusqlite::params![attempt_id, format!("-{days_ago} days")],
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            Ok(())
        })
        .expect("backdate Attempt action");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn starting_to_cook_creates_the_attempt_pinned_to_the_head_version() {
    let app = support::spawn_app();
    let (key, branch_id, lineage_id, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let (_, recipe) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let head_version_id = recipe["result"]["head_version_id"].as_str().unwrap();

    let (status, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 200, "{started}");
    let attempt = &started["result"];
    assert!(attempt["id"].as_str().unwrap().starts_with("at_"));
    assert_eq!(attempt["lineage_id"], json!(lineage_id));
    assert_eq!(
        attempt["version_id"],
        json!(head_version_id),
        "an Attempt is pinned by fingerprint to the Version cooked"
    );
    assert_eq!(attempt["current_step_index"], json!(0));
    assert_eq!(attempt["ticked_ingredients"], json!([]));
    assert_eq!(attempt["cooking_yield"], json!(null));
    assert_eq!(attempt["note"], json!(null));
    assert_eq!(attempt["rating"], json!(null));
    assert_eq!(
        attempt["finished_at"],
        json!(null),
        "there is no separate 'cooking session' object — starting IS the Attempt"
    );
    assert_eq!(attempt["resumable"], json!(true));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn starting_twice_resumes_the_one_in_progress_attempt_rather_than_making_a_second() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");

    let (_, first) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let first_id = first["result"]["id"].as_str().unwrap().to_string();

    let (_, second) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(
        second["result"]["id"],
        json!(first_id),
        "one Person may have only one Attempt In Progress per Lineage"
    );

    let count: i64 = app
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row("SELECT COUNT(*) FROM attempts", [], |r| r.get(0))
                .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .unwrap();
    assert_eq!(count, 1, "starting twice must not insert a second row");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_devices_advancing_the_same_attempt_stay_in_step() {
    let app = support::spawn_app();
    let (key, branch_id, lineage_id, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");

    // Phone starts cooking and moves to step 1, ticking the first Ingredient.
    let (_, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let attempt_id = started["result"]["id"].as_str().unwrap().to_string();
    let (status, advanced) = app.post_op(
        "advance_attempt",
        Some(&key),
        &json!({
            "attempt_id": attempt_id,
            "current_step_index": 1,
            "ticked_ingredients": [0],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{advanced}");

    // iPad picks the identical cooking up mid-stream, reading server state —
    // it never held anything of its own to reconcile.
    let (_, on_ipad) = app.post_op(
        "get_current_attempt",
        Some(&key),
        &json!({ "lineage_id": lineage_id }).to_string(),
    );
    let seen = &on_ipad["result"]["attempt"];
    assert_eq!(seen["id"], json!(attempt_id));
    assert_eq!(seen["current_step_index"], json!(1));
    assert_eq!(seen["ticked_ingredients"], json!([0]));

    // The iPad moves it further and sets the Yield being cooked to — a fact
    // about this afternoon, not a deviation written onto the recipe.
    let (status, advanced_further) = app.post_op(
        "advance_attempt",
        Some(&key),
        &json!({
            "attempt_id": attempt_id,
            "current_step_index": 2,
            "ticked_ingredients": [0, 1],
            "cooking_yield": { "amount": "8", "noun": "servings" },
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{advanced_further}");

    // The phone, polling again, sees exactly what the iPad just did: the
    // last device to move is where the cook is.
    let (_, on_phone_again) = app.post_op(
        "get_current_attempt",
        Some(&key),
        &json!({ "lineage_id": lineage_id }).to_string(),
    );
    let seen_again = &on_phone_again["result"]["attempt"];
    assert_eq!(seen_again["current_step_index"], json!(2));
    assert_eq!(seen_again["ticked_ingredients"], json!([0, 1]));
    assert_eq!(
        seen_again["cooking_yield"],
        json!({ "amount": "8", "noun": "servings" })
    );

    // The recipe itself never grew an Ingredient Line nobody wrote.
    let (_, recipe) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let ingredients = recipe["result"]["versions"][0]["content"]["ingredients"]
        .as_array()
        .unwrap();
    assert_eq!(
        ingredients.len(),
        2,
        "the cooking Yield writes no Ingredient Line"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn advancing_refuses_a_step_or_ingredient_index_outside_the_pinned_versions_content() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let (_, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let attempt_id = started["result"]["id"].as_str().unwrap().to_string();

    let (status, refused) = app.post_op(
        "advance_attempt",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "current_step_index": 99 }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");

    let (status, refused_ingredient) = app.post_op(
        "advance_attempt",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "ticked_ingredients": [99] }).to_string(),
    );
    assert_eq!(status, 400, "{refused_ingredient}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unfinished_attempt_may_be_deleted_to_undo_a_false_start() {
    let app = support::spawn_app();
    let (key, branch_id, lineage_id, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let (_, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let attempt_id = started["result"]["id"].as_str().unwrap().to_string();

    let (status, deleted) = app.post_op(
        "delete_attempt",
        Some(&key),
        &json!({ "attempt_id": attempt_id }).to_string(),
    );
    assert_eq!(status, 200, "{deleted}");
    assert_eq!(deleted["result"]["deleted"], json!(true));

    let (_, current) = app.post_op(
        "get_current_attempt",
        Some(&key),
        &json!({ "lineage_id": lineage_id }).to_string(),
    );
    assert_eq!(current["result"]["attempt"], json!(null));

    // Deleting freed the one In Progress slot: starting again mints a new
    // Attempt rather than being blocked by the one just undone.
    let (_, restarted) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_ne!(restarted["result"]["id"], json!(attempt_id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn finishing_ends_in_progress_and_a_finished_attempt_still_counts_and_may_be_edited() {
    let app = support::spawn_app();
    let (key, branch_id, lineage_id, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let (_, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let attempt_id = started["result"]["id"].as_str().unwrap().to_string();

    let (status, finished) = app.post_op(
        "finish_attempt",
        Some(&key),
        &json!({ "attempt_id": attempt_id }).to_string(),
    );
    assert_eq!(status, 200, "{finished}");
    assert!(finished["result"]["finished_at"].is_string());
    assert_eq!(
        finished["result"]["resumable"],
        json!(false),
        "a finished Attempt is not offered for resuming"
    );

    // No longer In Progress — the slot is free for a fresh cooking.
    let (_, current) = app.post_op(
        "get_current_attempt",
        Some(&key),
        &json!({ "lineage_id": lineage_id }).to_string(),
    );
    assert_eq!(current["result"]["attempt"], json!(null));

    // Advancing a finished Attempt is refused...
    let (status, refused) = app.post_op(
        "advance_attempt",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "current_step_index": 1 }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");

    // ...but it is still real, and still freely editable: a rating and a
    // note attached after the fact, exactly as ADR 0010 expects.
    let (status, edited) = app.post_op(
        "edit_attempt",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "note": "Un peu trop cuit", "rating": "tweak" })
            .to_string(),
    );
    assert_eq!(status, 200, "{edited}");
    assert_eq!(edited["result"]["note"], json!("Un peu trop cuit"));
    assert_eq!(edited["result"]["rating"], json!("tweak"));

    // A rating is one of three words and nothing else (#59). The five-star
    // scale this shipped with before Aurélien chose is refused outright
    // rather than quietly coerced.
    for refused_rating in [json!(4), json!("delicious"), json!(true)] {
        let (status, bad_rating) = app.post_op(
            "edit_attempt",
            Some(&key),
            &json!({ "attempt_id": attempt_id, "rating": refused_rating }).to_string(),
        );
        assert_eq!(status, 400, "{refused_rating} was accepted: {bad_rating}");
    }

    // Still on the books, still findable directly — an Attempt is never
    // deleted merely by finishing.
    let exists: i64 = app
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM attempts WHERE id = ?1",
                rusqlite::params![attempt_id],
                |r| r.get(0),
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .unwrap();
    assert_eq!(exists, 1);
}

// ---------------------------------------------------------------------------
// Finishing a cook (#59, ADR 0015): the judgement that lands at the end, and
// the refusal to average it.
// ---------------------------------------------------------------------------

/// A real, freshly encoded picture, so the fixture cannot be wrong the way a
/// hand-typed byte literal could.
fn a_picture(seed: u8) -> String {
    use base64::Engine;
    let image = image::RgbImage::from_fn(24, 18, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, seed])
    });
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgb8(image)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Jpeg,
        )
        .expect("encodes");
    base64::engine::general_purpose::STANDARD.encode(&bytes)
}

/// Upload a picture and answer its Photograph id.
fn upload_a_picture(app: &support::TestApp, key: &str, seed: u8) -> String {
    let (status, uploaded) = app.post_op(
        "upload_photograph",
        Some(key),
        &json!({ "data": a_picture(seed) }).to_string(),
    );
    assert_eq!(status, 200, "{uploaded}");
    uploaded["result"]["photograph_id"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Push when a cooking *happened* into the past. Two cookings seconds apart
/// can share a millisecond-precision timestamp, so a test about which of them
/// is newer has to space them itself rather than hope.
fn backdate_attempt_created(app: &support::TestApp, attempt_id: &str, days_ago: i64) {
    app.core
        .db()
        .with_conn(|conn| {
            conn.execute(
                "UPDATE attempts SET created_at = strftime('%Y-%m-%dT%H:%M:%fZ','now', ?2) \
                 WHERE id = ?1",
                rusqlite::params![attempt_id, format!("-{days_ago} days")],
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            Ok(())
        })
        .expect("backdate when the cooking happened");
}

/// When one Attempt says it happened, read straight from the store — so a test
/// can assert a date rather than merely that some string arrived.
fn attempt_created_at(app: &support::TestApp, attempt_id: &str) -> Value {
    app.core
        .db()
        .with_conn(|conn| {
            let at: String = conn
                .query_row(
                    "SELECT created_at FROM attempts WHERE id = ?1",
                    rusqlite::params![attempt_id],
                    |row| row.get(0),
                )
                .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            Ok(json!(at))
        })
        .expect("read when the cooking happened")
}

/// Cook a recipe start to finish, answering the Attempt's id.
fn cook_it(app: &support::TestApp, key: &str, branch_id: &str, finish: Value) -> String {
    let (_, started) = app.post_op(
        "start_attempt",
        Some(key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let attempt_id = started["result"]["id"].as_str().unwrap().to_string();
    let mut body = json!({ "attempt_id": attempt_id });
    for (field, value) in finish.as_object().expect("an object of fields") {
        body[field] = value.clone();
    }
    let (status, finished) = app.post_op("finish_attempt", Some(key), &body.to_string());
    assert_eq!(status, 200, "{finished}");
    attempt_id
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn finishing_a_cook_takes_a_rating_a_note_and_photographs_and_every_one_is_optional() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");

    // Saying nothing is finishing. A cook who puts the plate down and walks
    // away still cooked (ADR 0010) — nothing here may become a form to fill.
    let (_, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let silent_id = started["result"]["id"].as_str().unwrap().to_string();
    let (status, silent) = app.post_op(
        "finish_attempt",
        Some(&key),
        &json!({ "attempt_id": silent_id }).to_string(),
    );
    assert_eq!(status, 200, "{silent}");
    assert!(silent["result"]["finished_at"].is_string());
    assert_eq!(silent["result"]["rating"], json!(null));
    assert_eq!(silent["result"]["note"], json!(null));
    assert_eq!(silent["result"]["photographs"], json!([]));

    // And all three at once, in the one call that ends the cooking.
    let first = upload_a_picture(&app, &key, 10);
    let second = upload_a_picture(&app, &key, 200);
    let (_, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let attempt_id = started["result"]["id"].as_str().unwrap().to_string();
    let (status, finished) = app.post_op(
        "finish_attempt",
        Some(&key),
        &json!({
            "attempt_id": attempt_id,
            "rating": "again",
            "note": "Moins de sucre la prochaine fois",
            "photographs": [first, second],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{finished}");
    assert_eq!(finished["result"]["rating"], json!("again"));
    assert_eq!(
        finished["result"]["note"],
        json!("Moins de sucre la prochaine fois")
    );
    assert_eq!(finished["result"]["photographs"], json!([first, second]));

    // A picture this instance does not hold is refused rather than stored as
    // a name pointing at nothing.
    let (_, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let third_id = started["result"]["id"].as_str().unwrap().to_string();
    let (status, refused) = app.post_op(
        "finish_attempt",
        Some(&key),
        &json!({ "attempt_id": third_id, "photographs": ["not-a-photograph"] }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cooking_photograph_can_be_promoted_to_the_main_photo_or_to_a_step() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let plated = upload_a_picture(&app, &key, 40);
    let frying = upload_a_picture(&app, &key, 90);

    let attempt_id = cook_it(
        &app,
        &key,
        &branch_id,
        json!({ "photographs": [plated, frying] }),
    );

    // Out of the collapse window first. Promoting is an ordinary save, so
    // inside the window it would collapse into the Version being shaped — and
    // this test is about promotion, not about collapse.
    backdate_branch_head(&app, &branch_id);

    // The Main Photo. An ordinary edit making a Version — so it answers what
    // an ordinary save answers, and the fingerprint moved.
    let (_, before) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let head_before = before["result"]["head_version_id"].as_str().unwrap();
    let (status, promoted) = app.post_op(
        "promote_attempt_photograph",
        Some(&key),
        &json!({
            "attempt_id": attempt_id,
            "photograph_id": plated,
            "branch_id": branch_id,
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{promoted}");
    assert_eq!(promoted["result"]["copied"], json!(false));
    assert_ne!(
        promoted["result"]["version_id"].as_str().unwrap(),
        head_before,
        "promoting a picture is an edit, so it names a new Version"
    );

    // A Step's photo, on the same Attempt's other picture.
    let (status, stepped) = app.post_op(
        "promote_attempt_photograph",
        Some(&key),
        &json!({
            "attempt_id": attempt_id,
            "photograph_id": frying,
            "branch_id": branch_id,
            "step_index": 1,
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{stepped}");

    let (_, after) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let content = &after["result"]["versions"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()["content"];
    assert_eq!(content["main_photo"], json!(plated));
    assert_eq!(content["steps"][1]["photo"], json!(frying));
    assert_eq!(
        content["steps"][0]["photo"],
        json!(null),
        "promoting onto one Step leaves the others alone"
    );

    // Promoting is not moving: the cooking record keeps its pictures.
    let (_, thread) = app.post_op(
        "get_thread",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let attempt = thread["result"]["attempts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == json!(attempt_id))
        .expect("the Attempt is still on the Thread");
    assert_eq!(attempt["photographs"], json!([plated, frying]));

    // A Photograph this Attempt does not hold is refused — promotion is not a
    // second, quieter way to set the Main Photo.
    let stranger = upload_a_picture(&app, &key, 250);
    let (status, refused) = app.post_op(
        "promote_attempt_photograph",
        Some(&key),
        &json!({
            "attempt_id": attempt_id,
            "photograph_id": stranger,
            "branch_id": branch_id,
        })
        .to_string(),
    );
    assert_eq!(status, 400, "{refused}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_recipe_shows_cook_count_last_cooked_and_each_persons_most_recent_rating_by_name() {
    let app = support::spawn_app();
    let (aurelien, aurelien_key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_, created) = app.post_op(
        "create_recipe",
        Some(&aurelien_key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Katsu Curry",
            "steps": [{ "kind": "step", "text": "Frire." }],
        })
        .to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    let (_marie, marie_key, _marie_kitchen) = person_with_kitchen(&app, "Marie");
    let invite = app
        .core
        .invite_to_kitchen(&aurelien, &kitchen_id)
        .unwrap()
        .1;
    let (status, joined) = app.post_op(
        "accept_kitchen_invite",
        Some(&marie_key),
        &json!({ "secret": invite }).to_string(),
    );
    assert_eq!(status, 200, "{joined}");

    // A poor old verdict from Aurélien...
    let poor = cook_it(&app, &aurelien_key, &branch_id, json!({ "rating": "no" }));
    // ...then a good new one from the same Person. Only the new one shows:
    // a superseded verdict must not drag a fixed recipe down for ever.
    let good = cook_it(
        &app,
        &aurelien_key,
        &branch_id,
        json!({ "rating": "again" }),
    );
    // Marie's own, and one cooking she said nothing about at all.
    cook_it(&app, &marie_key, &branch_id, json!({ "rating": "tweak" }));
    let newest = cook_it(&app, &marie_key, &branch_id, json!({}));

    // Two cookings seconds apart can share a millisecond-precision timestamp,
    // so the two of Aurélien's are pushed apart deliberately: this test is
    // about which verdict wins, and a tie would make it decide nothing.
    backdate_attempt_created(&app, &poor, 3);
    backdate_attempt_created(&app, &good, 2);

    let (status, read) = app.post_op(
        "get_recipe",
        Some(&aurelien_key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 200, "{read}");
    let cooked = &read["result"]["cooked"];

    assert_eq!(cooked["count"], json!(4), "every cooking counts");
    // The newest cooking's own timestamp, not merely "some string": MIN in
    // place of MAX must fail this.
    assert_eq!(
        cooked["last_cooked_at"],
        attempt_created_at(&app, &newest),
        "last-cooked is the newest cooking's date: {cooked}"
    );

    let ratings = cooked["ratings"].as_array().unwrap();
    assert_eq!(
        ratings.len(),
        2,
        "one row per Person, never one per cooking: {cooked}"
    );
    let of = |name: &str| {
        ratings
            .iter()
            .find(|r| r["name"] == json!(name))
            .unwrap_or_else(|| panic!("{name} is missing from {cooked}"))
            .clone()
    };
    assert_eq!(
        of("Aurélien")["rating"],
        json!("again"),
        "the newest verdict wins outright — the older one reaches nobody"
    );
    assert_eq!(
        of("Marie")["rating"],
        json!("tweak"),
        "an unrated later cooking is silence, not a retraction"
    );

    // ADR 0015's refusal, checked as a shape rather than trusted as a rule.
    //
    // First the exact one: the cooking record has these three keys and no
    // others, so an `overall`, a `mean` or a `stars` cannot be slipped in
    // beside them without failing here. The recursive sweep below is a
    // second, looser net over the rest of the answer — a heuristic on names,
    // not a proof, which is why the exact assertion comes first.
    let mut keys: Vec<&String> = cooked.as_object().unwrap().keys().collect();
    keys.sort();
    assert_eq!(
        keys,
        vec!["count", "last_cooked_at", "ratings"],
        "the cooking record grew a field: {cooked}"
    );

    fn no_aggregate_anywhere(value: &Value, path: &str) {
        match value {
            Value::Object(fields) => {
                for (key, child) in fields {
                    let lowered = key.to_lowercase();
                    for forbidden in ["average", "mean", "score", "stars", "total_rating"] {
                        assert!(
                            !lowered.contains(forbidden),
                            "{path}.{key} looks like an aggregate rating"
                        );
                    }
                    no_aggregate_anywhere(child, &format!("{path}.{key}"));
                }
            }
            Value::Array(items) => {
                for (index, child) in items.iter().enumerate() {
                    no_aggregate_anywhere(child, &format!("{path}[{index}]"));
                }
            }
            Value::Number(_) => assert!(
                !path.ends_with("rating"),
                "{path} is a number: a rating must stay a word, or somebody will average it"
            ),
            _ => {}
        }
    }
    no_aggregate_anywhere(&read["result"], "recipe");

    // Marie sees the same record — this is what the recipe is, not a private
    // view of it.
    let (_, hers) = app.post_op(
        "get_recipe",
        Some(&marie_key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(hers["result"]["cooked"]["count"], json!(4));
    assert_eq!(hers["result"]["cooked"]["ratings"], cooked["ratings"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_attempts_note_rating_and_photographs_are_no_part_of_any_version() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let picture = upload_a_picture(&app, &key, 77);
    cook_it(
        &app,
        &key,
        &branch_id,
        json!({
            "rating": "no",
            "note": "Marie a détesté. Ne pas refaire tel quel.",
            "photographs": [picture],
        }),
    );

    // A Share Link renders a Version (ADR 0013/0026), so this is the property
    // that keeps a private cooking note off a page a stranger can open: there
    // is no path from a Version to an Attempt. #65 builds the page itself and
    // inherits this guarantee rather than re-deciding it.
    let (_, read) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    for version in read["result"]["versions"].as_array().unwrap() {
        let content = version["content"].to_string();
        assert!(
            !content.contains("Marie a détesté"),
            "an Attempt's note reached a Version: {content}"
        );
        assert!(
            !content.contains(&picture),
            "an Attempt's Photograph reached a Version: {content}"
        );
        assert!(
            version["content"].get("rating").is_none(),
            "a Version has no rating of its own"
        );
    }

    // The same, at the row store: the fingerprint names recipe state alone,
    // so no Version anywhere in the database carries any of it.
    let leaked: i64 = app
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM versions WHERE content LIKE '%Marie a détesté%' \
                 OR content LIKE '%' || ?1 || '%'",
                rusqlite::params![picture],
                |r| r.get(0),
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .unwrap();
    assert_eq!(leaked, 0, "an Attempt's judgement is in no Version at all");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn only_the_cook_who_owns_an_attempt_may_advance_finish_edit_or_delete_it() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, kitchen_id) = recipe_ready_to_cook(&app, "Aurélien");
    // A second Person in the same Kitchen — able to see the recipe, but the
    // Attempt is still theirs alone until they record their own.
    let intruder = app.core.create_person("Marc").expect("person");
    app.core
        .db()
        .with_conn(|conn| {
            conn.execute(
                "INSERT INTO kitchen_members (kitchen_id, person_id) VALUES (?1, ?2)",
                rusqlite::params![kitchen_id, intruder],
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            Ok(())
        })
        .unwrap();
    let intruder_key = app
        .core
        .mint_access_key(&intruder, "browser", false)
        .unwrap()
        .secret;

    let (_, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let attempt_id = started["result"]["id"].as_str().unwrap().to_string();

    // Each of these is asked twice: once about the Attempt Aurélien is really
    // cooking, once about an id nobody ever minted. Marc cooks in the Kitchen,
    // so he may see the recipe — and the two answers must still be the same
    // one, because whose cooking it is was never his to learn (ADR 0040).
    let absent = "at_ffffffffffffffff";
    for (operation, rest) in [
        ("advance_attempt", json!({ "current_step_index": 1 })),
        ("finish_attempt", json!({})),
        ("edit_attempt", json!({ "note": "not mine to say" })),
        ("delete_attempt", json!({})),
    ] {
        let ask = |id: &str| {
            let mut input = rest.clone();
            input["attempt_id"] = json!(id);
            app.post_op(operation, Some(&intruder_key), &input.to_string())
        };
        let (status, refused) = ask(&attempt_id);
        assert_eq!(status, 404, "{operation}: {refused}");
        assert_eq!(
            refused["error"]["message"],
            json!("no such Attempt"),
            "{operation}: {refused}"
        );
        assert_eq!(
            ask(absent),
            (status, refused),
            "{operation} tells Marc that id names somebody's cooking"
        );
    }

    // And the cook whose Attempt it is still reaches it.
    let (status, advanced) = app.post_op(
        "advance_attempt",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "current_step_index": 1 }).to_string(),
    );
    assert_eq!(status, 200, "{advanced}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn starting_to_cook_requires_being_able_to_see_the_recipe() {
    let app = support::spawn_app();
    let (_key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let stranger = app.core.create_person("Marc").expect("person");
    let stranger_key = app
        .core
        .mint_access_key(&stranger, "browser", false)
        .unwrap()
        .secret;

    let (status, refused) = app.post_op(
        "start_attempt",
        Some(&stranger_key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 404, "{refused}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resuming_is_offered_for_three_days_then_silently_stops_without_deleting_the_attempt() {
    let app = support::spawn_app();
    let (key, branch_id, lineage_id, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let (_, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let attempt_id = started["result"]["id"].as_str().unwrap().to_string();

    backdate_attempt_action(&app, &attempt_id, 4);

    let (_, current) = app.post_op(
        "get_current_attempt",
        Some(&key),
        &json!({ "lineage_id": lineage_id }).to_string(),
    );
    let attempt = &current["result"]["attempt"];
    assert_eq!(
        attempt["id"],
        json!(attempt_id),
        "the Attempt itself is not deleted by the window passing"
    );
    assert_eq!(
        attempt["resumable"],
        json!(false),
        "Kamosu stops offering to resume after three days without interaction"
    );

    // A sourdough touched today never ages out: any action refreshes it.
    app.post_op(
        "advance_attempt",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "current_step_index": 1 }).to_string(),
    );
    let (_, current_again) = app.post_op(
        "get_current_attempt",
        Some(&key),
        &json!({ "lineage_id": lineage_id }).to_string(),
    );
    assert_eq!(current_again["result"]["attempt"]["resumable"], json!(true));

    // Correcting a note mid-cook is itself an action too.
    backdate_attempt_action(&app, &attempt_id, 4);
    app.post_op(
        "edit_attempt",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "note": "needs more salt" }).to_string(),
    );
    let (_, current_after_edit) = app.post_op(
        "get_current_attempt",
        Some(&key),
        &json!({ "lineage_id": lineage_id }).to_string(),
    );
    assert_eq!(
        current_after_edit["result"]["attempt"]["resumable"],
        json!(true),
        "editing an In Progress Attempt's note is itself a last action"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_read_only_access_key_cannot_start_or_advance_an_attempt_but_can_read_it() {
    let app = support::spawn_app();
    let (key, branch_id, lineage_id, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let (_, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let attempt_id = started["result"]["id"].as_str().unwrap().to_string();

    let person: String = app
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row(
                "SELECT person_id FROM attempts WHERE id = ?1",
                rusqlite::params![attempt_id],
                |r| r.get(0),
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .unwrap();
    let read_only_key = app
        .core
        .mint_access_key(&person, "read-only agent", true)
        .unwrap()
        .secret;

    // Reading still works — an agent asked to read out the next step reads
    // server state through an ordinary Operation.
    let (status, read) = app.post_op(
        "get_current_attempt",
        Some(&read_only_key),
        &json!({ "lineage_id": lineage_id }).to_string(),
    );
    assert_eq!(status, 200, "{read}");
    assert_eq!(read["result"]["attempt"]["id"], json!(attempt_id));

    // Advancing, finishing and starting a fresh Attempt are all writes.
    let (status, refused) = app.post_op(
        "advance_attempt",
        Some(&read_only_key),
        &json!({ "attempt_id": attempt_id, "current_step_index": 1 }).to_string(),
    );
    assert_eq!(status, 401, "{refused}");
    assert_eq!(refused["error"]["kind"], json!("unauthorized"));

    let (status, _) = app.post_op(
        "start_attempt",
        Some(&read_only_key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 401);
}

// --- The Thread and the Branch Point (issue #53) ------------------------------

/// One Lineage, two Branches, one Person who cooks in both Kitchens the
/// Branches live in — a Kitchen holding several Branches of one Lineage
/// (CONTEXT.md, "Branch"), built with real Operations rather than raw SQL.
/// The shared trunk runs four Versions deep before the fork, and each side
/// grows two more afterwards, so the chain the Branch Point has to walk is
/// genuinely deep rather than a two-Version toy.
/// Returns `(person_id, key, lineage_id, branch_a, branch_b, branch_point_version_id)`.
fn two_branches_of_one_deep_lineage(
    app: &support::TestApp,
) -> (String, String, String, String, String, String) {
    let (person, key, kitchen_a) = person_with_kitchen(app, "Aurélien");
    let kitchen_b = home_kitchen_of(app, &person);

    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_a, "title": "Poulet Coréen v1" }).to_string(),
    );
    let branch_a = created["result"]["branch_id"].as_str().unwrap().to_string();
    let lineage_id = created["result"]["lineage_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Three more ordinary saves on the trunk before anyone forks — the "deep
    // chain" the Branch Point walk has to cross.
    for n in 2..=4 {
        backdate_branch_head(app, &branch_a);
        app.post_op(
            "save_recipe_version",
            Some(&key),
            &json!({ "branch_id": branch_a, "title": format!("Poulet Coréen v{n}") }).to_string(),
        );
    }

    let (_, at_fork) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_a }).to_string(),
    );
    let branch_point_version_id = at_fork["result"]["head_version_id"]
        .as_str()
        .unwrap()
        .to_string();

    // The fork: saving into a *different* Kitchen the same Person cooks in
    // is a Copy, exactly as an in-instance Translation would be — it starts
    // branch_b at the Version just read, carrying the whole chain behind it.
    backdate_branch_head(app, &branch_a);
    let (_, forked) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": branch_a, "kitchen_id": kitchen_b, "title": "Poulet Coréen, sans friture" })
            .to_string(),
    );
    assert_eq!(forked["result"]["copied"], json!(true));
    let branch_b = forked["result"]["branch_id"].as_str().unwrap().to_string();

    // Two more saves on each side, so both Branches keep growing past the
    // fork rather than stopping the instant they diverge.
    for n in 1..=2 {
        backdate_branch_head(app, &branch_a);
        app.post_op(
            "save_recipe_version",
            Some(&key),
            &json!({ "branch_id": branch_a, "title": format!("Mine, take {n}") }).to_string(),
        );
        backdate_branch_head(app, &branch_b);
        app.post_op(
            "save_recipe_version",
            Some(&key),
            &json!({ "branch_id": branch_b, "kitchen_id": kitchen_b, "title": format!("Sans friture, take {n}") })
                .to_string(),
        );
    }

    (
        person,
        key,
        lineage_id,
        branch_a,
        branch_b,
        branch_point_version_id,
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_thread_shows_every_branch_the_caller_can_see_and_hides_the_rest() {
    let app = support::spawn_app();
    let (aurelien, key, lineage_id, branch_a, branch_b, _fork_version) =
        two_branches_of_one_deep_lineage(&app);

    // An Attempt hangs off the Thread by Lineage, not by Branch (ADR 0005) —
    // cooked from branch_a's head, pinned there regardless of branch_b.
    let (_, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_a }).to_string(),
    );
    let attempt_id = started["result"]["id"].as_str().unwrap().to_string();

    let (status, thread) = app.post_op(
        "get_thread",
        Some(&key),
        &json!({ "branch_id": branch_a }).to_string(),
    );
    assert_eq!(status, 200, "{thread}");
    let result = &thread["result"];
    assert_eq!(result["lineage_id"], json!(lineage_id));

    let branch_ids: Vec<&str> = result["branches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["branch_id"].as_str().unwrap())
        .collect();
    assert_eq!(
        branch_ids.len(),
        2,
        "both Branches of the Lineage are visible to a Person who cooks in both Kitchens: {result}"
    );
    assert!(branch_ids.contains(&branch_a.as_str()));
    assert!(branch_ids.contains(&branch_b.as_str()));

    let versions = result["versions"].as_array().unwrap();
    assert_eq!(
        versions
            .iter()
            .filter(|v| v["branch_id"] == json!(branch_a))
            .count(),
        6,
        "branch_a's whole chain — 4 shared, 2 its own"
    );
    assert_eq!(
        versions
            .iter()
            .filter(|v| v["branch_id"] == json!(branch_b))
            .count(),
        7,
        "branch_b's whole chain — 4 carried across, 1 the fork itself, 2 its own"
    );

    let attempt_ids: Vec<&str> = result["attempts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["id"].as_str().unwrap())
        .collect();
    assert!(
        attempt_ids.contains(&attempt_id.as_str()),
        "the Attempt hangs off the Thread: {result}"
    );

    // A Person who cooks in only one of the two Kitchens sees only that
    // Branch — no Version is hidden from someone who can see it, but a
    // Branch in a Kitchen this Person does not belong to is not this
    // Person's to see at all.
    let branch_a_kitchen: String = app
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row(
                "SELECT kitchen_id FROM branches WHERE id = ?1",
                rusqlite::params![branch_a],
                |row| row.get(0),
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .unwrap();
    let marc = app.core.create_person("Marc").expect("person");
    let marc_key = app
        .core
        .mint_access_key(&marc, "browser", false)
        .unwrap()
        .secret;
    let invite = app
        .core
        .invite_to_kitchen(&aurelien, &branch_a_kitchen)
        .unwrap()
        .1;
    app.core.accept_kitchen_invite(&marc, &invite).unwrap();

    let (status, marcs_view) = app.post_op(
        "get_thread",
        Some(&marc_key),
        &json!({ "branch_id": branch_a }).to_string(),
    );
    assert_eq!(status, 200, "{marcs_view}");
    let marcs_branch_ids: Vec<&str> = marcs_view["result"]["branches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["branch_id"].as_str().unwrap())
        .collect();
    assert_eq!(
        marcs_branch_ids,
        vec![branch_a.as_str()],
        "Marc cooks in branch_a's Kitchen alone, so branch_b stays invisible to him: {marcs_view}"
    );

    // A stranger to both Kitchens is refused outright.
    let stranger = app.core.create_person("Camille").expect("person");
    let stranger_key = app
        .core
        .mint_access_key(&stranger, "browser", false)
        .unwrap()
        .secret;
    let (status, _) = app.post_op(
        "get_thread",
        Some(&stranger_key),
        &json!({ "branch_id": branch_a }).to_string(),
    );
    assert_eq!(status, 404);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn branch_point_is_computed_by_walking_both_chains_across_a_deep_chain() {
    let app = support::spawn_app();
    let (_aurelien, key, _lineage_id, branch_a, branch_b, fork_version) =
        two_branches_of_one_deep_lineage(&app);

    let (status, point) = app.post_op(
        "branch_point",
        Some(&key),
        &json!({ "branch_a_id": branch_a, "branch_b_id": branch_b }).to_string(),
    );
    assert_eq!(status, 200, "{point}");
    assert_eq!(
        point["result"]["version_id"],
        json!(fork_version),
        "the last Version two Branches share, walked rather than declared"
    );

    // Symmetric: asking the other way round answers the same Version.
    let (_, reversed) = app.post_op(
        "branch_point",
        Some(&key),
        &json!({ "branch_a_id": branch_b, "branch_b_id": branch_a }).to_string(),
    );
    assert_eq!(reversed["result"]["version_id"], json!(fork_version));
}

/// **Issue #82.** A collapse rewrites the row at the head sequence in place,
/// which is right for a rapid re-save nobody else has seen and wrong the
/// moment somebody has copied you: a Copy carries the source's chain across
/// verbatim, so the Copy is holding its own row naming that very Version.
/// Rewriting it left the two chains with nothing in common — permanently,
/// silently, and with `get_thread` still answering, which was the worst of it.
///
/// Deliberately no `backdate_branch_head` anywhere in this test: every save
/// here lands *inside* the collapse window, because the window is the whole
/// condition under test.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_rapid_re_save_after_a_copy_appends_instead_of_severing_the_two_branches() {
    let app = support::spawn_app();
    let (person, key, kitchen_a) = person_with_kitchen(&app, "Aurélien");
    let kitchen_b = home_kitchen_of(&app, &person);

    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_a, "title": "Maison Batterman" }).to_string(),
    );
    let branch_t = created["result"]["branch_id"].as_str().unwrap().to_string();
    let shared_version = created["result"]["head_version_id"]
        .as_str()
        .unwrap()
        .to_string();

    // A friend takes a copy: saving into a Kitchen that did not write this
    // Branch starts one of its own, carrying the whole chain behind it.
    let (_, copied) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": branch_t, "kitchen_id": kitchen_b, "title": "Chez Marc" })
            .to_string(),
    );
    assert_eq!(copied["result"]["copied"], json!(true));
    let branch_m = copied["result"]["branch_id"].as_str().unwrap().to_string();

    // The pair is sound before the tweak — this is what the tweak used to
    // destroy, so it has to be proved rather than assumed.
    let (status, point) = app.post_op(
        "branch_point",
        Some(&key),
        &json!({ "branch_a_id": branch_t, "branch_b_id": branch_m }).to_string(),
    );
    assert_eq!(status, 200, "{point}");
    assert_eq!(point["result"]["version_id"], json!(shared_version));

    // You tweak yours a minute later. Same Hand, inside the window — but
    // branch_m's chain names this Version, so it has stopped being the
    // Version being shaped and become a shared fact.
    let (_, tweaked) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": branch_t, "title": "Maison Batterman, sans beurre" }).to_string(),
    );
    assert_eq!(
        tweaked["result"]["collapsed"],
        json!(false),
        "somebody else is holding this Version, so the save appends: {tweaked}"
    );
    assert_eq!(
        tweaked["result"]["parent_version_id"],
        json!(shared_version)
    );
    assert_eq!(tweaked["result"]["sequence"], json!(2));

    // Still related, and still naming the Version the two Branches actually
    // share, in both directions.
    let (status, point) = app.post_op(
        "branch_point",
        Some(&key),
        &json!({ "branch_a_id": branch_t, "branch_b_id": branch_m }).to_string(),
    );
    assert_eq!(status, 200, "{point}");
    assert_eq!(point["result"]["version_id"], json!(shared_version));

    let (status, divergence) = app.post_op(
        "divergence",
        Some(&key),
        &json!({ "branch_id": branch_t, "other_branch_id": branch_m }).to_string(),
    );
    assert_eq!(status, 200, "{divergence}");
    assert_eq!(
        divergence["result"]["theirs"]["content"]["title"],
        json!("Chez Marc")
    );

    // One fork, not two unrelated histories: both chains still hold the
    // Version they parted at, which is the whole of what the Thread draws
    // the fork from.
    let (_, thread) = app.post_op(
        "get_thread",
        Some(&key),
        &json!({ "branch_id": branch_t }).to_string(),
    );
    let versions = thread["result"]["versions"].as_array().unwrap();
    let chain_of = |branch: &str| -> Vec<&str> {
        versions
            .iter()
            .filter(|v| v["branch_id"] == json!(branch))
            .map(|v| v["version_id"].as_str().unwrap())
            .collect()
    };
    assert!(
        chain_of(&branch_t).contains(&shared_version.as_str()),
        "the source still holds the Version it was copied at: {thread}"
    );
    assert!(
        chain_of(&branch_m).contains(&shared_version.as_str()),
        "and so does the Copy — that shared row is the fork: {thread}"
    );

    // The window itself is untouched. branch_t's head is now a Version
    // nobody else holds, so the very next save — same Hand, same minute —
    // collapses exactly as it always did.
    let (_, again) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": branch_t, "title": "Maison Batterman, sans beurre ni crème" })
            .to_string(),
    );
    assert_eq!(
        again["result"]["collapsed"],
        json!(true),
        "nobody else names this Version, so the rapid re-save still collapses: {again}"
    );
    assert_eq!(again["result"]["sequence"], json!(2));
}

/// The other half of #82's condition, and the reason it is scoped to the
/// Lineage: a Version is content-addressed and global, so two people who each
/// start a recipe with the same title hold the very same Version in two
/// unrelated Lineages. Neither is holding the other's history, and neither
/// may close the other's collapse window.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_unrelated_recipes_that_happen_to_match_do_not_close_each_others_window() {
    let app = support::spawn_app();
    let (_aurelien, key, kitchen) = person_with_kitchen(&app, "Aurélien");
    let (_marc, marc_key, marc_kitchen) = person_with_kitchen(&app, "Marc");

    let (_, mine) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen, "title": "Soupe" }).to_string(),
    );
    let (_, theirs) = app.post_op(
        "create_recipe",
        Some(&marc_key),
        &json!({ "kitchen_id": marc_kitchen, "title": "Soupe" }).to_string(),
    );
    assert_eq!(
        mine["result"]["head_version_id"], theirs["result"]["head_version_id"],
        "same content, same fingerprint — convergence is about content, never authorship"
    );
    assert_ne!(
        mine["result"]["lineage_id"], theirs["result"]["lineage_id"],
        "two recipes all the same, and nothing relates them"
    );

    let branch_id = mine["result"]["branch_id"].as_str().unwrap().to_string();
    let (_, edited) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "title": "Soupe de poisson" }).to_string(),
    );
    assert_eq!(
        edited["result"]["collapsed"],
        json!(true),
        "Marc's unrelated Soupe is not holding Aurélien's history: {edited}"
    );
    assert_eq!(edited["result"]["sequence"], json!(1));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_damaged_chain_is_reported_as_damage_rather_than_guessed() {
    let app = support::spawn_app();
    let (_aurelien, key, _lineage_id, branch_a, branch_b, _fork_version) =
        two_branches_of_one_deep_lineage(&app);

    // Sever branch_a's chain: its second Version now claims a parent that is
    // not the row before it — a Bundle arriving broken, simulated directly,
    // since a healthy instance never produces this on its own.
    app.core
        .db()
        .with_conn(|conn| {
            conn.execute(
                "UPDATE branch_versions SET parent_version_id = ( \
                     SELECT version_id FROM branch_versions \
                      WHERE branch_id = ?1 AND sequence = 4 \
                 ) WHERE branch_id = ?1 AND sequence = 2",
                rusqlite::params![branch_a],
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            Ok(())
        })
        .expect("sever the chain");

    let (status, point) = app.post_op(
        "branch_point",
        Some(&key),
        &json!({ "branch_a_id": branch_a, "branch_b_id": branch_b }).to_string(),
    );
    assert_ne!(status, 200, "{point}");
    assert_eq!(point["ok"], json!(false));
    assert!(
        point["error"]["message"]
            .as_str()
            .unwrap()
            .contains("damaged"),
        "a broken chain is reported as damage, not silently accommodated: {point}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn starting_to_cook_can_pin_to_an_old_version_read_back_from_the_thread() {
    let app = support::spawn_app();
    let (key, branch_id, lineage_id, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let (_, first) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let first_version_id = first["result"]["head_version_id"]
        .as_str()
        .unwrap()
        .to_string();

    backdate_branch_head(&app, &branch_id);
    app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "title": "Katsu Curry, épicé" }).to_string(),
    );

    let (status, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id, "version_id": first_version_id }).to_string(),
    );
    assert_eq!(status, 200, "{started}");
    assert_eq!(
        started["result"]["version_id"],
        json!(first_version_id),
        "cooking from a past Version pins the Attempt there, not to the head"
    );
    assert_eq!(started["result"]["lineage_id"], json!(lineage_id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn start_attempt_rejects_a_version_id_that_is_not_this_branchs_own() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage_id, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");

    let (status, refused) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id, "version_id": "v_never_saved" }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["error"]["kind"], json!("bad_request"));
}

// --- The Import, its ledger and its Report (issue #68) -----------------------

/// Ask `import` and wait for its Job to finish, handing back the Report.
fn import_and_wait(app: &support::TestApp, key: &str, body: &Value) -> Value {
    let (status, ask) = app.post_op("import", Some(key), &body.to_string());
    assert_eq!(status, 200, "{ask}");
    let job_id = ask["result"]["job_id"].as_str().expect("a job id");
    let finished = support::wait_terminal(app, Some(key), job_id);
    assert_eq!(finished["status"], json!("completed"), "{finished}");
    finished["result"].clone()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn importing_lands_a_new_recipe_in_the_home_kitchen_with_the_importing_hand() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "importer", false)
        .unwrap()
        .secret;
    let home_kitchen_id = home_kitchen_of(&app, &person);

    let report = import_and_wait(
        &app,
        &key,
        &json!({
            "source_kind": "crouton",
            "candidates": [{
                "foreign_id": "crouton-uuid-1",
                "title": "Katsu Curry",
                "source": { "text": "Crouton", "link": "https://example.com/katsu" },
                "ingredients": [{ "kind": "ingredient", "text": "2 cloves garlic" }],
                "steps": [{ "kind": "step", "text": "Fry it.", "photo": null }],
            }],
        }),
    );

    assert_eq!(report["source_kind"], json!("crouton"));
    assert_eq!(report["kitchen_id"], json!(home_kitchen_id));
    let arrived = report["arrived"].as_array().unwrap();
    assert_eq!(arrived.len(), 1, "{report}");
    assert_eq!(arrived[0]["status"], json!("created"));
    assert_eq!(arrived[0]["title"], json!("Katsu Curry"));
    assert!(report["offered"].as_array().unwrap().is_empty());
    assert!(report["unreadable"].as_array().unwrap().is_empty());
    let branch_id = arrived[0]["branch_id"].as_str().unwrap().to_string();

    // It landed in the Home Kitchen, the imported Person's Hand on the first
    // Version, and carries the Source it really came from — no import mark,
    // no provenance field, no per-line badge (ADR 0025).
    let (status, recipe) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 200, "{recipe}");
    assert_eq!(recipe["result"]["kitchen_id"], json!(home_kitchen_id));
    let first_version = &recipe["result"]["versions"][0];
    assert_eq!(first_version["hand_id"], json!(person));
    assert_eq!(
        first_version["content"]["source"]["link"],
        json!("https://example.com/katsu")
    );
    let content_keys: Vec<&String> = first_version["content"]
        .as_object()
        .unwrap()
        .keys()
        .collect();
    let known = [
        "title",
        "yield",
        "prep_time_minutes",
        "cook_time_minutes",
        "note",
        "main_photo",
        "source",
        "nutrition",
        "ingredients",
        "steps",
    ];
    for key in content_keys {
        assert!(
            known.contains(&key.as_str()),
            "unexpected field on imported content: {key} — an import must staple nothing extra on"
        );
    }

    // Read a second time, through the ordinary Job Operations: the Report
    // survives the screen closing (list_jobs, get_job) rather than being a
    // one-shot answer.
    let (_, listed) = app.post_op("list_jobs", Some(&key), "{}");
    let jobs = listed["result"]["jobs"].as_array().unwrap();
    assert!(
        jobs.iter().any(|j| j["operation"] == json!("import")),
        "{listed}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn re_running_an_import_matches_the_ledger_instead_of_doubling_the_library() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "importer", false)
        .unwrap()
        .secret;

    let candidate = json!({
        "foreign_id": "crouton-uuid-2",
        "title": "Coq au Vin",
    });
    let first = import_and_wait(
        &app,
        &key,
        &json!({ "source_kind": "crouton", "candidates": [candidate] }),
    );
    let first_arrived = &first["arrived"][0];
    assert_eq!(first_arrived["status"], json!("created"));
    let lineage_id = first_arrived["lineage_id"].as_str().unwrap().to_string();

    // Identical content, same foreign id, run again: matched and unchanged,
    // not a second Lineage.
    let second = import_and_wait(
        &app,
        &key,
        &json!({ "source_kind": "crouton", "candidates": [candidate] }),
    );
    let second_arrived = &second["arrived"][0];
    assert_eq!(second_arrived["status"], json!("unchanged"), "{second}");
    assert_eq!(second_arrived["lineage_id"], json!(lineage_id));
    assert!(second["offered"].as_array().unwrap().is_empty());

    let (_, list) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": first_arrived["branch_id"] }).to_string(),
    );
    assert_eq!(
        list["result"]["versions"].as_array().unwrap().len(),
        1,
        "a matched, unchanged re-run mints no second Version: {list}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_previously_seen_recipe_found_changed_is_offered_never_written_over() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "importer", false)
        .unwrap()
        .secret;

    let first = import_and_wait(
        &app,
        &key,
        &json!({
            "source_kind": "crouton",
            "candidates": [{ "foreign_id": "crouton-uuid-3", "title": "Tarte Tatin" }],
        }),
    );
    let branch_id = first["arrived"][0]["branch_id"]
        .as_str()
        .unwrap()
        .to_string();
    let original_head = app.core.get_recipe(&person, &branch_id).unwrap()["head_version_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Re-run with the same foreign id but changed content: offered for
    // review, the Branch's head left exactly where it was.
    let second = import_and_wait(
        &app,
        &key,
        &json!({
            "source_kind": "crouton",
            "candidates": [{ "foreign_id": "crouton-uuid-3", "title": "Tarte Tatin (revisited)" }],
        }),
    );
    assert!(second["arrived"].as_array().unwrap().is_empty(), "{second}");
    let offered = &second["offered"][0];
    assert_eq!(offered["branch_id"], json!(branch_id));
    assert_eq!(offered["title"], json!("Tarte Tatin (revisited)"));
    assert!(offered["candidate_version_id"].as_str().is_some());

    let after = app.core.get_recipe(&person, &branch_id).unwrap();
    assert_eq!(
        after["head_version_id"],
        json!(original_head),
        "an offered change must never be written over the Branch on its own"
    );
    assert_eq!(after["versions"].as_array().unwrap().len(), 1, "{after}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bare_name_and_link_arrives_and_an_unreadable_candidate_is_named_with_its_reason() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "importer", false)
        .unwrap()
        .secret;

    let report = import_and_wait(
        &app,
        &key,
        &json!({
            "source_kind": "web_link",
            "candidates": [
                {
                    "foreign_id": "https://example.com/bare",
                    "title": "Someone's Pasta",
                    "source": { "text": "example.com", "link": "https://example.com/bare" },
                },
                { "foreign_id": "https://example.com/broken", "title": "" },
            ],
        }),
    );

    let arrived = report["arrived"].as_array().unwrap();
    assert_eq!(arrived.len(), 1, "{report}");
    assert_eq!(arrived[0]["status"], json!("created"));
    assert_eq!(arrived[0]["title"], json!("Someone's Pasta"));

    let unreadable = report["unreadable"].as_array().unwrap();
    assert_eq!(unreadable.len(), 1, "{report}");
    assert_eq!(
        unreadable[0]["foreign_id"],
        json!("https://example.com/broken")
    );
    assert!(
        unreadable[0]["reason"].as_str().unwrap().contains("title"),
        "{report}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_read_only_access_key_may_not_import() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let read_only_key = app
        .core
        .mint_access_key(&person, "read only", true)
        .unwrap()
        .secret;

    let (status, refused) = app.post_op(
        "import",
        Some(&read_only_key),
        &json!({
            "source_kind": "crouton",
            "candidates": [{ "foreign_id": "x", "title": "Nope" }],
        })
        .to_string(),
    );
    assert_eq!(status, 401, "{refused}");
    assert_eq!(refused["error"]["kind"], json!("unauthorized"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_ledger_is_deletable_whole_leaving_its_recipes_untouched() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "importer", false)
        .unwrap()
        .secret;

    let report = import_and_wait(
        &app,
        &key,
        &json!({
            "source_kind": "crouton",
            "candidates": [{ "foreign_id": "crouton-uuid-4", "title": "Ratatouille" }],
        }),
    );
    let import_id = report["import_id"].as_str().unwrap().to_string();
    let branch_id = report["arrived"][0]["branch_id"]
        .as_str()
        .unwrap()
        .to_string();

    // The whole migration is finished with: the ledger goes, in no fingerprint
    // and no Bundle to begin with, and every recipe it named is untouched.
    app.core
        .db()
        .with_conn(|conn| {
            conn.execute(
                "DELETE FROM import_ledger WHERE import_id = ?1",
                rusqlite::params![import_id],
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            conn.execute(
                "DELETE FROM imports WHERE id = ?1",
                rusqlite::params![import_id],
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            Ok(())
        })
        .expect("delete the Import whole");

    let (status, recipe) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 200, "{recipe}");
    assert_eq!(
        recipe["result"]["versions"][0]["content"]["title"],
        json!("Ratatouille")
    );

    // Re-importing the same foreign id after the ledger is gone starts a
    // fresh Import and cannot know it has been seen before — matching is
    // scoped to a live ledger, never guessed from the recipes themselves.
    let again = import_and_wait(
        &app,
        &key,
        &json!({
            "source_kind": "crouton",
            "candidates": [{ "foreign_id": "crouton-uuid-4", "title": "Ratatouille" }],
        }),
    );
    assert_eq!(again["arrived"][0]["status"], json!("created"), "{again}");
    assert_ne!(again["import_id"], json!(import_id));
}

// --- Migrations and the Snapshot (issue #35) ---------------------------------

use kamosu::db::{self, Migration};

/// Build a database at an old schema by running the real migration list
/// truncated to version 1, then stamp it as version 1. This is exactly what a
/// pre-jobs Kamosu left on disk.
fn build_v1_database(data_dir: &std::path::Path) {
    std::fs::create_dir_all(data_dir).expect("data dir");
    let only_v1: &[Migration] = &db::MIGRATIONS[..1];
    db::Db::open_with_migrations(data_dir, only_v1).expect("v1 database");
}

fn stored_schema_version(data_dir: &std::path::Path) -> i64 {
    let conn = rusqlite::Connection::open_with_flags(
        data_dir.join(db::DATABASE_FILE),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("read the database file");
    conn.query_row(
        "SELECT value FROM meta WHERE key = 'schema_version'",
        [],
        |r| r.get::<_, String>(0),
    )
    .expect("schema_version row")
    .parse()
    .expect("numeric schema_version")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_database_at_an_old_schema_migrates_forward_and_serves() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().join("data");
    build_v1_database(&data_dir);

    // Opening the current binary against that old database migrates it forward:
    // no down-migration exists to reach for (ADR 0030).
    let app = support::spawn_app_in(&data_dir);
    let person = app.core.create_person("Aurélien").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "browser", false)
        .unwrap()
        .secret;

    // The migrated schema serves real Operations — the Job shape works, which is
    // what migration 2 added.
    let (_, ask) = app.post_op("probe_job", Some(&key), r#"{"steps":2,"delay_ms":1}"#);
    assert_eq!(ask["ok"], json!(true), "{ask}");

    // And the ledger says the database now stands at the newest step.
    assert_eq!(
        stored_schema_version(&data_dir),
        db::LATEST_SCHEMA_VERSION,
        "the database must stand at the newest migration"
    );
}

/// **The step that split a Branch's two identities moves neither** (#90).
///
/// `branches.travelling_id` holds the id a Branch travels under, and it is filled
/// in only on a Branch that arrived from somewhere else. Every Branch already
/// written here travels under its own id, so the step adds a column, fills
/// nothing in, and leaves every row exactly where it was: this builds a library
/// at the schema before it, migrates forward, and reads back that no Branch id
/// moved, that the Share Link, Shopping List entry and Import ledger row still
/// reach their Branch, and that every Version still hashes to its own id.
#[test]
fn splitting_a_branchs_travelling_id_from_its_row_id_moves_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();

    let content = r#"{"steps":[{"text":"Cuire"}],"title":"Coq au Vin"}"#;
    {
        // Everything up to the step before this one.
        let earlier: &[Migration] = &db::MIGRATIONS[..31];
        let old = db::Db::open_with_migrations(&data_dir, earlier).expect("the earlier schema");
        old.with_conn(|conn| {
            // The Version takes the id the fingerprint gives it, so the check
            // below is a real one rather than a tautology about a made-up id.
            conn.execute_batch(&format!(
                "INSERT INTO people (id, name) VALUES ('p_1', 'Aurélien');
                 INSERT INTO kitchens (id, name, hand_id) VALUES ('k_1', 'Home', 'h_1');
                 INSERT INTO kitchen_members (kitchen_id, person_id) VALUES ('k_1', 'p_1');
                 INSERT INTO lineages (id) VALUES ('l_1');
                 INSERT INTO versions (id, content)
                     SELECT version_fingerprint('{content}'), '{content}';
                 INSERT INTO branches (id, lineage_id, kitchen_id, hand_id, language, head_version_id)
                     SELECT 'b_1', 'l_1', 'k_1', 'h_1', 'fr', id FROM versions;
                 INSERT INTO branch_versions (branch_id, sequence, version_id, hand_id)
                     SELECT 'b_1', 1, id, 'h_1' FROM versions;
                 INSERT INTO share_links (id, branch_id, secret_hash, shared_by)
                     VALUES ('sl_1', 'b_1', 'a hash', 'p_1');
                 INSERT INTO shopping_choices (person_id, branch_id, known_as)
                     VALUES ('p_1', 'b_1', 'Coq au Vin');
                 INSERT INTO imports (id, kitchen_id, source_kind)
                     VALUES ('imp_1', 'k_1', 'bundle');
                 INSERT INTO import_ledger (import_id, foreign_id, lineage_id, branch_id)
                     VALUES ('imp_1', 'b_elsewhere', 'l_1', 'b_1');"
            ))
            .expect("a library at the schema before #90");
            Ok(())
        })
        .unwrap();
    }

    let db = db::Db::open(&data_dir).expect("the current schema");
    db.with_conn(|conn| {
        let (id, carried): (String, Option<String>) = conn
            .query_row("SELECT id, travelling_id FROM branches", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        assert_eq!(id, "b_1", "the Branch is where it was");
        assert_eq!(
            carried, None,
            "a Branch written here travels under its own id, so nothing is filled in"
        );

        // Each of the three rows the acceptance criteria name, reached through
        // the join that has to keep working.
        for (table, count) in [
            ("share_links", 1),
            ("shopping_choices", 1),
            ("import_ledger", 1),
        ] {
            let resolved: i64 = conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM {table} \
                           JOIN branches ON branches.id = {table}.branch_id"
                    ),
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(resolved, count, "{table} still reaches its Branch");
        }

        // The live check AGENTS.md documents, asked of the migrated file.
        let adrift: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM versions WHERE id <> version_fingerprint(content)",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(adrift, 0, "every Version still hashes to its own id");
        Ok(())
    })
    .unwrap();

    assert_eq!(
        stored_schema_version(&data_dir),
        db::LATEST_SCHEMA_VERSION,
        "the database stands at the newest migration"
    );
}

/// **The re-fingerprint, on the state that actually caused it** (#89, ADR 0038).
///
/// #72 added `nutrition` to a recipe. Nothing rewrote what was already stored,
/// so a Version saved before that day hashed its content without the field and
/// a Version saved after hashed it with — and the same recipe, written either
/// side of it, held two different ids. On the dev instance that produced
/// exactly what is built here: `Braised Chicken in Red Wine`, word for word
/// identical, under two ids, one of them stored as an As Cooked because
/// cooking the recipe exactly as written compared unequal to it.
///
/// Migration 29 makes the two one Version. This builds the database at the
/// step before it, hands it to the current binary, and reads back what it did.
#[test]
fn the_same_recipe_written_either_side_of_a_new_field_becomes_one_version() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();

    // The one recipe, in its two spellings. `before` is what a Version stored
    // ahead of #72 holds; `after` is the identical recipe with the field that
    // ticket added, empty because nobody typed a calorie figure.
    let before = r#"{"note":"Best hot.","steps":[{"text":"Cuire"}],"title":"Coq au Vin"}"#;
    let after =
        r#"{"note":"Best hot.","nutrition":null,"steps":[{"text":"Cuire"}],"title":"Coq au Vin"}"#;
    // The ids those two texts were given by the fingerprint of the day: two
    // different hashes for one recipe, which is the whole bug. Written as
    // legible placeholders rather than real SHA-256, because the migration
    // reads only `content` — what the old id was is opaque to it, and naming
    // them makes the assertions below readable.
    let id_before = "v_as_written_before_nutrition";
    let id_after = "v_as_written_after_nutrition";

    {
        // Everything up to the step before the re-fingerprint.
        let earlier: &[Migration] = &db::MIGRATIONS[..28];
        let old = db::Db::open_with_migrations(&data_dir, earlier).expect("the earlier schema");
        old.with_conn(|conn| {
            conn.execute_batch(&format!(
                "INSERT INTO people (id, name) VALUES ('p_1', 'Aurélien');
                 INSERT INTO kitchens (id, name, hand_id) VALUES ('k_1', 'Home', 'h_1');
                 INSERT INTO lineages (id) VALUES ('l_1');
                 INSERT INTO versions (id, content) VALUES ('{id_before}', '{before}');
                 INSERT INTO versions (id, content) VALUES ('{id_after}', '{after}');
                 -- Two more that collapse with nothing, so the sweep at the end
                 -- is over a table rather than over the merged pair alone.
                 INSERT INTO versions (id, content)
                     VALUES ('v_a_second_recipe', '{{\"title\":\"Ratatouille\"}}');
                 INSERT INTO versions (id, content)
                     VALUES ('v_a_third_recipe', '{{\"steps\":[],\"title\":\"Soupe\"}}');
                 INSERT INTO branches (id, lineage_id, kitchen_id, hand_id, language, head_version_id)
                     VALUES ('b_1', 'l_1', 'k_1', 'h_1', 'fr', '{id_before}');
                 INSERT INTO branch_versions (branch_id, sequence, version_id, hand_id)
                     VALUES ('b_1', 1, '{id_before}', 'h_1');
                 -- A Reading on the older row, and none on the newer: a Reading
                 -- is authored data (ADR 0021) and must survive the merge.
                 INSERT INTO readings (version_id, line_index, amount, unit, target)
                     VALUES ('{id_before}', 0, '1', NULL, 'poulet');
                 -- The cooking that started this: pinned to the recipe, and
                 -- holding an As Cooked that is the very same words.
                 INSERT INTO attempts (id, lineage_id, person_id, version_id, as_cooked_version_id)
                     VALUES ('a_1', 'l_1', 'p_1', '{id_before}', '{id_after}');"
            ))
            .expect("the state #89 found");
            Ok(())
        })
        .unwrap();
    }

    // The current binary opens it, and migration 29 runs.
    let db = db::Db::open(&data_dir).expect("migrated");
    db.with_conn(|conn| {
        let survivors: Vec<(String, String)> = conn
            .prepare("SELECT id, content FROM versions")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            survivors.len(),
            3,
            "one recipe written twice is one Version, beside the two others: {survivors:?}"
        );
        let (id, content) = survivors
            .iter()
            .find(|(_, content)| content.contains("Coq au Vin"))
            .expect("the merged recipe");

        // The id is the fingerprint of the content it sits under — the whole
        // of what #89 measured and found false.
        let recomputed: String = conn
            .query_row("SELECT version_fingerprint(?1)", [content], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            id, &recomputed,
            "a Version's id must fingerprint its content"
        );
        assert_ne!(id, id_before, "and it is not either id it replaced");
        assert_ne!(id, id_after);

        // Everything that named either of them names the survivor.
        let head: String = conn
            .query_row("SELECT head_version_id FROM branches", [], |row| row.get(0))
            .unwrap();
        assert_eq!(&head, id, "the Branch head moved with it");
        let in_chain: String = conn
            .query_row("SELECT version_id FROM branch_versions", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(&in_chain, id, "so did its place in the Thread");
        let pinned: String = conn
            .query_row("SELECT version_id FROM attempts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(&pinned, id, "and the Version the cook pinned to");

        // The Reading survived, on the row that survived.
        let reading: (String, String) = conn
            .query_row("SELECT version_id, target FROM readings", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        assert_eq!(
            reading,
            (id.clone(), "poulet".to_string()),
            "a Reading is authored data and is never dropped in a merge"
        );

        // And the As Cooked is gone: the two recipes were the same words, so
        // that cooking followed the recipe exactly and stores nothing
        // (ADR 0005) — which is the bug #58's live acceptance ran into.
        let as_cooked: Option<String> = conn
            .query_row("SELECT as_cooked_version_id FROM attempts", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            as_cooked, None,
            "cooked exactly as written must store nothing"
        );

        // **The sweep, over everything the migrations left behind.** The unit
        // gate in `src/core.rs` catches a new field given a non-empty default;
        // this catches the other way in — a future migration that rewrites
        // `versions.content` itself and does not re-fingerprint what it
        // touched. It runs over a database carried forward from an earlier
        // schema, which is the only place that could ever happen.
        let (total, wrong): (i64, i64) = conn
            .query_row(
                "SELECT COUNT(*), COUNT(*) FILTER (WHERE id <> version_fingerprint(content)) \
                 FROM versions",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            total, 3,
            "the merged pair and the two that merged with nothing"
        );
        assert_eq!(
            wrong, 0,
            "a migration must leave every Version hashing to the id it sits under"
        );
        Ok(())
    })
    .unwrap();
}

/// The same collapse, but with both Versions **inside one Branch's chain**
/// rather than one of them off to the side (#89, ADR 0038).
///
/// Saving a recipe, then saving the identical recipe again, is supposed to
/// append nothing — `save_recipe_version` sees the fingerprint already names
/// that state and stops. Before ADR 0038 it did not see that across a field
/// addition, so a Branch could end up with two consecutive Versions holding
/// one recipe. Collapsing them without also mending the Thread would leave the
/// second row naming the same Version as the first and recording *itself* as
/// the Version it came from, which is not a thing a history can say.
#[test]
fn two_consecutive_versions_that_collapse_leave_one_entry_in_the_thread() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();

    let before = r#"{"steps":[{"text":"Cuire"}],"title":"Ratatouille"}"#;
    let after = r#"{"nutrition":null,"steps":[{"text":"Cuire"}],"title":"Ratatouille"}"#;
    let id_before = "v_as_written_before_nutrition";
    let id_after = "v_as_written_after_nutrition";

    {
        let earlier: &[Migration] = &db::MIGRATIONS[..28];
        let old = db::Db::open_with_migrations(&data_dir, earlier).expect("the earlier schema");
        old.with_conn(|conn| {
            conn.execute_batch(&format!(
                "INSERT INTO people (id, name) VALUES ('p_1', 'Aurélien');
                 INSERT INTO kitchens (id, name, hand_id) VALUES ('k_1', 'Home', 'h_1');
                 INSERT INTO lineages (id) VALUES ('l_1');
                 INSERT INTO versions (id, content) VALUES ('{id_before}', '{before}');
                 INSERT INTO versions (id, content) VALUES ('{id_after}', '{after}');
                 INSERT INTO branches (id, lineage_id, kitchen_id, hand_id, language, head_version_id)
                     VALUES ('b_1', 'l_1', 'k_1', 'h_1', 'fr', '{id_after}');
                 INSERT INTO branch_versions (branch_id, sequence, version_id, parent_version_id, hand_id, change_note)
                     VALUES ('b_1', 1, '{id_before}', NULL, 'h_1', NULL);
                 INSERT INTO branch_versions (branch_id, sequence, version_id, parent_version_id, hand_id, change_note)
                     VALUES ('b_1', 2, '{id_after}', '{id_before}', 'h_1', 'Added a calorie figure');"
            ))
            .expect("a Branch holding one recipe twice");
            Ok(())
        })
        .unwrap();
    }

    let db = db::Db::open(&data_dir).expect("migrated");
    db.with_conn(|conn| {
        let chain: Vec<(i64, String, Option<String>, Option<String>)> = conn
            .prepare(
                "SELECT sequence, version_id, parent_version_id, change_note \
                   FROM branch_versions WHERE branch_id = 'b_1' ORDER BY sequence",
            )
            .unwrap()
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            chain.len(),
            1,
            "one recipe saved twice is one entry in the Thread: {chain:?}"
        );
        let (_, version_id, parent, note) = &chain[0];
        assert_eq!(*parent, None, "the first Version came from nothing");
        assert_ne!(
            Some(version_id.as_str()),
            parent.as_deref(),
            "no Version records itself as the Version it came from"
        );
        // The *what changed* line is the only place the why of an edit ever
        // comes from and nothing can infer it later (ADR 0004), so the note
        // written on the row that went away is kept rather than dropped.
        assert_eq!(
            note.as_deref(),
            Some("Added a calorie figure"),
            "an authored note survives the collapse"
        );

        // And the Branch still stands at a Version that exists.
        let head: String = conn
            .query_row("SELECT head_version_id FROM branches", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            &head, version_id,
            "the Branch head is the row that survived"
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn a_failing_migration_refuses_to_serve_and_leaves_a_restorable_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().join("data");
    build_v1_database(&data_dir);
    let db_path = data_dir.join(db::DATABASE_FILE);

    // An upgrade whose second step fails halfway. SQLite rolls the transaction
    // back; Kamosu must refuse rather than serve half-migrated, and say where
    // the Snapshot is (ADR 0030).
    let broken_v2 = Migration {
        version: 2,
        description: "the Job shape",
        sql: "CREATE TABLE this_is_not_sql (",
    };
    let steps: &[Migration] = &[db::MIGRATIONS[0], broken_v2];
    let failure = db::Db::open_with_migrations(&data_dir, steps)
        .err()
        .expect("must refuse");
    let message = failure.to_sentence();

    // The refusal names the failed step and the Snapshot to restore.
    assert!(
        message.contains("the Job shape") && message.contains("migration"),
        "the refusal must say which migration failed: {message}"
    );
    let snapshot_marker = "snapshot";
    assert!(
        message.contains(snapshot_marker),
        "the refusal must point at the Snapshot: {message}"
    );
    let snapshot_path = data_dir.join(
        message
            .split_whitespace()
            .find(|word| word.contains(snapshot_marker))
            .map(|word| word.trim_end_matches(['(', ')', '[', ']', ',']))
            .expect("a snapshot path in the message"),
    );
    assert!(
        snapshot_path.exists(),
        "the Snapshot named in the refusal must exist on disk"
    );

    // The database itself is untouched: still old, not half-migrated.
    assert_eq!(stored_schema_version(&data_dir), 1, "no half-migration");

    // Restoring is putting the file back — nothing more clever than a copy.
    std::fs::copy(&snapshot_path, &db_path).expect("restore the snapshot");
    let restored = db::Db::open(&data_dir).expect("the restored database opens");
    let job_rows: i64 = restored
        .with_conn(|conn| {
            conn.query_row("SELECT COUNT(*) FROM jobs", [], |r| r.get(0))
                .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .expect("the migrated schema serves");
    assert_eq!(
        job_rows, 0,
        "a restored v1 database migrates to an empty jobs table"
    );
    assert_eq!(stored_schema_version(&data_dir), db::LATEST_SCHEMA_VERSION);
}

#[test]
fn an_older_binary_against_a_newer_database_refuses_loudly() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path();
    build_v1_database(data_dir);

    // Someone ran a newer Kamosu once; this older binary meets the result.
    let conn = rusqlite::Connection::open(data_dir.join(db::DATABASE_FILE)).unwrap();
    conn.execute(
        "UPDATE meta SET value = '99' WHERE key = 'schema_version'",
        [],
    )
    .unwrap();
    drop(conn);

    let failure = db::Db::open(data_dir)
        .err()
        .expect("an older binary must refuse");
    let message = failure.to_sentence();
    assert!(
        message.contains("99") && message.contains("newer"),
        "the refusal must say what it met and why it stopped: {message}"
    );
}

// --- The web link importer (issue #70) ---------------------------------------
//
// The seventeen catalogued JSON-LD shapes themselves are exhaustively unit
// tested against the pure extractor in `src/web_import.rs`, with no server
// involved. These tests exercise the other half: the real `import_web_link`
// Operation, through a real fetch, against a real local HTTP server standing
// in for "the internet" (only possible in this test binary, via the
// `test-jobs`-gated escape hatch `web_import::allow_loopback_fetches_for_tests`
// — production's guard denies loopback unconditionally, ADR 0033). The whole
// section is behind the same feature gate as the escape hatch itself, so a
// plain `cargo build`/`cargo clippy` (no `test-jobs`) never needs it to exist.
#[cfg(feature = "test-jobs")]
mod web_link_importer {
    use super::*;
    use axum::response::IntoResponse;

    /// Serve one recipe page (and, optionally, one photo) on an ephemeral loopback
    /// port. Returns the page's own URL.
    fn spawn_recipe_server(html: impl FnOnce(&str) -> String, photo: Option<Vec<u8>>) -> String {
        kamosu::web_import::allow_loopback_fetches_for_tests();

        let std_listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
        std_listener.set_nonblocking(true).unwrap();
        let listener = tokio::net::TcpListener::from_std(std_listener).expect("async listener");
        let addr = listener.local_addr().expect("local addr");
        let base = format!("http://{addr}");
        let html = Arc::new(html(&base));

        let mut router = axum::Router::new().route(
            "/recipe",
            axum::routing::get(move || {
                let html = html.clone();
                async move { axum::response::Html((*html).clone()) }
            }),
        );
        if let Some(photo_bytes) = photo {
            let photo_bytes = Arc::new(photo_bytes);
            router = router.route(
                "/photo.jpg",
                axum::routing::get(move || {
                    let bytes = photo_bytes.clone();
                    async move { ([("content-type", "image/jpeg")], (*bytes).clone()) }
                }),
            );
        }
        tokio::spawn(async move {
            axum::serve(listener, router)
                .await
                .expect("test recipe server");
        });
        format!("{base}/recipe")
    }

    /// Serve several named pages at once, one per `(name, html)` pair, each
    /// reachable at `/pages/<name>` off the returned base URL — for driving
    /// one shape per call through the real Operation without standing up a
    /// server per shape.
    fn spawn_pages_server(pages: &[(&str, String)]) -> String {
        kamosu::web_import::allow_loopback_fetches_for_tests();

        let std_listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
        std_listener.set_nonblocking(true).unwrap();
        let listener = tokio::net::TcpListener::from_std(std_listener).expect("async listener");
        let addr = listener.local_addr().expect("local addr");
        let base = format!("http://{addr}");

        let pages: std::collections::HashMap<String, Arc<String>> = pages
            .iter()
            .map(|(name, html)| (name.to_string(), Arc::new(html.clone())))
            .collect();
        let pages = Arc::new(pages);
        let router = axum::Router::new().route(
            "/pages/{name}",
            axum::routing::get(
                move |axum::extract::Path(name): axum::extract::Path<String>| {
                    let pages = pages.clone();
                    async move {
                        match pages.get(&name) {
                            Some(html) => axum::response::Html((**html).clone()).into_response(),
                            None => axum::http::StatusCode::NOT_FOUND.into_response(),
                        }
                    }
                },
            ),
        );
        tokio::spawn(async move {
            axum::serve(listener, router)
                .await
                .expect("test pages server");
        });
        base
    }

    /// A real, freshly encoded picture, exactly as `tests/photographs.rs` builds
    /// one — generated rather than hand-typed, so the fixture cannot be wrong the
    /// way a hand-typed byte literal could.
    fn make_test_jpeg() -> Vec<u8> {
        let image = image::RgbImage::from_fn(20, 20, |x, y| {
            image::Rgb([(x * 10) as u8, (y * 10) as u8, 128])
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

    /// Ask `import_web_link` and wait for its Job to finish, handing back the
    /// Report — the same shape `import` itself answers, since both land through
    /// `Core::import`.
    fn import_web_link_and_wait(app: &support::TestApp, key: &str, url: &str) -> Value {
        let (status, ask) = app.post_op(
            "import_web_link",
            Some(key),
            &json!({ "url": url }).to_string(),
        );
        assert_eq!(status, 200, "{ask}");
        let job_id = ask["result"]["job_id"].as_str().expect("a job id");
        let finished = support::wait_terminal(app, Some(key), job_id);
        assert_eq!(finished["status"], json!("completed"), "{finished}");
        finished["result"].clone()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn importing_a_web_link_lands_the_recipe_with_its_photo_and_matches_on_reimport() {
        let app = support::spawn_app();
        let person = app.core.create_person("Aurélien").expect("person");
        let key = app
            .core
            .mint_access_key(&person, "importer", false)
            .unwrap()
            .secret;

        let photo = make_test_jpeg();
        let url = spawn_recipe_server(
            |base| {
                format!(
                    r#"<html><head><script type="application/ld+json">{{
                        "@context": "https://schema.org",
                        "@graph": [
                            {{"@type": "WebSite", "name": "Not the recipe"}},
                            {{
                                "@type": "Recipe",
                                "name": "Sectioned Cookies",
                                "recipeIngredient": ["120g butter", "75g sugar"],
                                "recipeInstructions": [
                                    {{"@type": "HowToSection", "name": "The dough", "itemListElement": [
                                        {{"@type": "HowToStep", "text": "Mix"}},
                                        {{"@type": "HowToStep", "text": "Bake"}}
                                    ]}}
                                ],
                                "prepTime": "PT20M",
                                "cookTime": "PT12M",
                                "recipeYield": "24 cookies",
                                "image": "{base}/photo.jpg",
                                "author": {{"@type": "Person", "name": "Jane Cook"}},
                                "nutrition": {{"@type": "NutritionInformation", "calories": "150 calories"}}
                            }}
                        ]
                    }}</script></head><body></body></html>"#
                )
            },
            Some(photo),
        );

        let report = import_web_link_and_wait(&app, &key, &url);
        assert_eq!(report["source_kind"], json!("web"));
        let arrived = report["arrived"].as_array().unwrap();
        assert_eq!(arrived.len(), 1, "{report}");
        assert_eq!(arrived[0]["status"], json!("created"));
        assert_eq!(arrived[0]["title"], json!("Sectioned Cookies"));
        let branch_id = arrived[0]["branch_id"].as_str().unwrap().to_string();

        let (status, recipe) = app.post_op(
            "get_recipe",
            Some(&key),
            &json!({ "branch_id": branch_id }).to_string(),
        );
        assert_eq!(status, 200, "{recipe}");
        let content = &recipe["result"]["versions"][0]["content"];
        assert_eq!(content["prep_time_minutes"], json!(20));
        assert_eq!(content["cook_time_minutes"], json!(12));
        assert_eq!(content["yield"]["amount"], json!("24"));
        assert_eq!(content["yield"]["noun"], json!("cookies"));
        assert_eq!(content["source"]["text"], json!("Jane Cook"));
        assert_eq!(content["source"]["link"], json!(url));
        let ingredients = content["ingredients"].as_array().unwrap();
        assert_eq!(ingredients.len(), 2);
        let steps = content["steps"].as_array().unwrap();
        assert_eq!(steps[0]["kind"], json!("section"));
        assert_eq!(steps[0]["text"], json!("The dough"));
        assert_eq!(steps[1]["kind"], json!("step"));
        assert_eq!(steps[1]["text"], json!("Mix"));
        assert_eq!(steps[2]["text"], json!("Bake"));

        // The photo the page named really did arrive, remade and readable back.
        let photo_id = content["main_photo"].as_str().expect("a stored photo id");
        let (photo_status, _, bytes) =
            app.get_bytes(&format!("/api/photographs/{photo_id}"), Some(&key));
        assert_eq!(photo_status, 200);
        assert!(!bytes.is_empty());

        // The page's own address is the ledger's foreign id: re-importing the
        // same page matches instead of doubling the library (#68, ADR 0025) — the
        // same Branch comes back "unchanged" rather than a second Lineage.
        let second = import_web_link_and_wait(&app, &key, &url);
        let second_arrived = second["arrived"].as_array().unwrap();
        assert_eq!(second_arrived.len(), 1, "{second}");
        assert_eq!(second_arrived[0]["status"], json!("unchanged"));
        assert_eq!(second_arrived[0]["branch_id"], json!(branch_id));
        assert!(second["offered"].as_array().unwrap().is_empty(), "{second}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_page_with_no_structured_data_still_lands_as_a_bare_title_and_source() {
        let app = support::spawn_app();
        let person = app.core.create_person("Aurélien").expect("person");
        let key = app
            .core
            .mint_access_key(&person, "importer", false)
            .unwrap()
            .secret;

        let url = spawn_recipe_server(
            |_base| {
                "<html><head><title>Just a blog post</title></head><body>no recipe here</body></html>"
                    .to_string()
            },
            None,
        );

        let report = import_web_link_and_wait(&app, &key, &url);
        let arrived = report["arrived"].as_array().unwrap();
        assert_eq!(arrived.len(), 1, "{report}");
        assert_eq!(arrived[0]["status"], json!("created"));
        assert_eq!(arrived[0]["title"], json!("Just a blog post"));

        // The Source link is the page itself, not only a title with nowhere
        // to point back to (ADR 0025's "a bare name and a link").
        let branch_id = arrived[0]["branch_id"].as_str().unwrap();
        let (status, recipe) = app.post_op(
            "get_recipe",
            Some(&key),
            &json!({ "branch_id": branch_id }).to_string(),
        );
        assert_eq!(status, 200, "{recipe}");
        assert_eq!(
            recipe["result"]["versions"][0]["content"]["source"]["link"],
            json!(url)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_favicon_named_image_is_dropped_rather_than_stored_as_the_photo() {
        let app = support::spawn_app();
        let person = app.core.create_person("Aurélien").expect("person");
        let key = app
            .core
            .mint_access_key(&person, "importer", false)
            .unwrap()
            .secret;

        let url = spawn_recipe_server(
            |base| {
                format!(
                    r#"<html><head><script type="application/ld+json">
                    {{"@type": "Recipe", "name": "No Real Photo", "image": "{base}/favicon.ico"}}
                    </script></head><body></body></html>"#
                )
            },
            None,
        );

        let report = import_web_link_and_wait(&app, &key, &url);
        let branch_id = report["arrived"][0]["branch_id"]
            .as_str()
            .unwrap()
            .to_string();
        let (_, recipe) = app.post_op(
            "get_recipe",
            Some(&key),
            &json!({ "branch_id": branch_id }).to_string(),
        );
        assert_eq!(
            recipe["result"]["versions"][0]["content"]["main_photo"],
            Value::Null
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn fetching_a_web_link_that_redirects_to_a_private_address_is_refused() {
        let app = support::spawn_app();
        let person = app.core.create_person("Aurélien").expect("person");
        let key = app
            .core
            .mint_access_key(&person, "importer", false)
            .unwrap()
            .secret;
        kamosu::web_import::allow_loopback_fetches_for_tests();

        let router = axum::Router::new().route(
            "/start",
            axum::routing::get(|| async {
                axum::response::Redirect::temporary("http://10.0.0.1/private")
            }),
        );
        let std_listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
        std_listener.set_nonblocking(true).unwrap();
        let listener = tokio::net::TcpListener::from_std(std_listener).expect("async listener");
        let addr = listener.local_addr().expect("local addr");
        tokio::spawn(async move {
            axum::serve(listener, router)
                .await
                .expect("redirecting test server");
        });

        let (status, ask) = app.post_op(
            "import_web_link",
            Some(&key),
            &json!({ "url": format!("http://{addr}/start") }).to_string(),
        );
        assert_eq!(status, 200, "{ask}");
        let job_id = ask["result"]["job_id"].as_str().expect("a job id");
        let finished = support::wait_terminal(&app, Some(&key), job_id);
        assert_eq!(
            finished["status"],
            json!("failed"),
            "a redirect into a private address must fail the Job: {finished}"
        );
    }

    /// One shape's fixture: its page path, its HTML, the title it must land
    /// with, and an optional deeper check against the landed recipe's full
    /// content (`None` when the title alone is what this shape risks
    /// getting wrong).
    struct Shape {
        name: &'static str,
        html: String,
        title: &'static str,
        check: Option<fn(&Value)>,
    }

    /// Every one of the seventeen catalogued JSON-LD shapes, driven through
    /// the real `import_web_link` Operation rather than the pure extractor —
    /// the shapes already exercised by the tests above (`@graph` nesting,
    /// `HowToSection`, image-as-string, yield-as-text, author-object, ISO
    /// durations, the no-Recipe fallback, favicon-dropping, both redirect
    /// cases) are not repeated here; this covers the rest, so all seventeen
    /// are proven end to end at least once, not only against the pure
    /// extractor in `src/web_import.rs`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn every_remaining_catalogued_shape_lands_through_the_real_operation() {
        let app = support::spawn_app();
        let person = app.core.create_person("Aurélien").expect("person");
        let key = app
            .core
            .mint_access_key(&person, "importer", false)
            .unwrap()
            .secret;

        let ld = |json: &str| -> String {
            format!(
                "<html><head><script type=\"application/ld+json\">{json}</script></head><body></body></html>"
            )
        };

        let shapes = vec![
            Shape {
                name: "type-as-array",
                html: ld(r#"{"@type":["Recipe","NewsArticle"],"name":"Array Typed"}"#),
                title: "Array Typed",
                check: None,
            },
            Shape {
                name: "top-level-array",
                html: ld(r#"[{"@type":"WebSite"},{"@type":"Recipe","name":"From Array"}]"#),
                title: "From Array",
                check: None,
            },
            Shape {
                name: "malformed-block-then-valid",
                html: "<html><head>\
                 <script type=\"application/ld+json\">{ not json </script>\
                 <script type=\"application/ld+json\">{\"@type\":\"Recipe\",\"name\":\"Survivor\"}</script>\
                 </head><body></body></html>"
                    .to_string(),
                title: "Survivor",
                check: None,
            },
            Shape {
                name: "instructions-as-howtostep-array",
                html: ld(r#"{"@type":"Recipe","name":"Steps","recipeInstructions":[
                    {"@type":"HowToStep","text":"Mix"},{"@type":"HowToStep","text":"Bake"}
                ]}"#),
                title: "Steps",
                check: Some(|content| {
                    let steps = content["steps"].as_array().unwrap();
                    assert_eq!(steps[0]["text"], json!("Mix"), "{content}");
                    assert_eq!(steps[1]["text"], json!("Bake"), "{content}");
                }),
            },
            Shape {
                name: "instructions-as-single-string",
                html: ld(r#"{"@type":"Recipe","name":"One Line","recipeInstructions":"Mix everything and bake."}"#),
                title: "One Line",
                check: Some(|content| {
                    let steps = content["steps"].as_array().unwrap();
                    assert_eq!(steps.len(), 1, "{content}");
                    assert_eq!(steps[0]["text"], json!("Mix everything and bake."));
                }),
            },
            Shape {
                // The photo fetch to a deliberately unreachable address fails
                // and is dropped (#45's rule) — this proves the object shape
                // parses without crashing, not that a real photo lands (the
                // full download path is proven separately, against a real
                // image server, in `importing_a_web_link_lands_the_recipe_with_its_photo_and_matches_on_reimport`).
                name: "image-as-object",
                html: ld(r#"{"@type":"Recipe","name":"Pic Object","image":{"@type":"ImageObject","url":"https://example.invalid/dish.jpg"}}"#),
                title: "Pic Object",
                check: Some(|content| assert_eq!(content["main_photo"], Value::Null, "{content}")),
            },
            Shape {
                name: "image-as-array",
                html: ld(r#"{"@type":"Recipe","name":"Pic Array","image":["https://example.invalid/one.jpg","https://example.invalid/two.jpg"]}"#),
                title: "Pic Array",
                check: Some(|content| assert_eq!(content["main_photo"], Value::Null, "{content}")),
            },
            Shape {
                name: "yield-as-array",
                html: ld(r#"{"@type":"Recipe","name":"Yield Array","recipeYield":["4 servings","4"]}"#),
                title: "Yield Array",
                check: Some(|content| {
                    assert_eq!(content["yield"]["amount"], json!("4"), "{content}");
                    assert_eq!(content["yield"]["noun"], json!("servings"), "{content}");
                }),
            },
            Shape {
                name: "yield-as-number",
                html: ld(r#"{"@type":"Recipe","name":"Yield Number","recipeYield":6}"#),
                title: "Yield Number",
                check: Some(|content| {
                    assert_eq!(content["yield"]["amount"], json!("6"), "{content}");
                    assert_eq!(content["yield"]["noun"], json!("servings"), "{content}");
                }),
            },
            Shape {
                name: "category-as-array",
                html: ld(r#"{"@type":"Recipe","name":"Category Array","recipeCategory":["Dessert","Snack"]}"#),
                title: "Category Array",
                check: None,
            },
            Shape {
                // Row 17: HTML entities. Proven at the extractor level in
                // `src/web_import.rs`'s own unit tests too, but landed through
                // a real Operation here so all seventeen shapes are proven
                // end to end at least once, not only against the pure
                // extractor.
                name: "html-entities",
                html: ld(r#"{"@type":"Recipe","name":"Salt &amp; Pepper","recipeIngredient":["salt &amp; pepper"]}"#),
                title: "Salt & Pepper",
                check: Some(|content| {
                    let ingredients = content["ingredients"].as_array().unwrap();
                    assert_eq!(ingredients[0]["text"], json!("salt & pepper"), "{content}");
                }),
            },
        ];

        let pages: Vec<(&str, String)> = shapes
            .iter()
            .map(|shape| (shape.name, shape.html.clone()))
            .collect();
        let base = spawn_pages_server(&pages);

        for shape in &shapes {
            let url = format!("{base}/pages/{}", shape.name);
            let report = import_web_link_and_wait(&app, &key, &url);
            let arrived = report["arrived"].as_array().unwrap();
            assert_eq!(arrived.len(), 1, "shape '{}': {report}", shape.name);
            assert_eq!(
                arrived[0]["title"],
                json!(shape.title),
                "shape '{}' did not land with the expected title: {report}",
                shape.name
            );
            if let Some(check) = shape.check {
                let branch_id = arrived[0]["branch_id"].as_str().unwrap();
                let (status, recipe) = app.post_op(
                    "get_recipe",
                    Some(&key),
                    &json!({ "branch_id": branch_id }).to_string(),
                );
                assert_eq!(status, 200, "shape '{}': {recipe}", shape.name);
                check(&recipe["result"]["versions"][0]["content"]);
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_read_only_access_key_may_not_import_a_web_link() {
        let app = support::spawn_app();
        let person = app.core.create_person("Aurélien").expect("person");
        let read_only_key = app
            .core
            .mint_access_key(&person, "read only", true)
            .unwrap()
            .secret;

        let (status, refused) = app.post_op(
            "import_web_link",
            Some(&read_only_key),
            &json!({ "url": "https://example.com/recipe" }).to_string(),
        );
        assert_eq!(status, 401, "{refused}");
        assert_eq!(refused["error"]["kind"], json!("unauthorized"));
    }

    // Nutrition, imported only where the page gave a number (#72, spec item
    // 172). schema.org's `NutritionInformation` is defined per serving, so
    // that is the basis an imported figure lands with — the importer states
    // what the page stated and infers nothing further.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_page_stating_calories_lands_them_and_a_page_stating_none_leaves_the_field_empty() {
        let app = support::spawn_app();
        let person = app.core.create_person("Aurélien").expect("person");
        let key = app
            .core
            .mint_access_key(&person, "importer", false)
            .unwrap()
            .secret;

        let base = spawn_pages_server(&[
            (
                "stated",
                r#"<html><head><script type="application/ld+json">
                {"@type": "Recipe", "name": "Chocolate chunk cookies",
                 "recipeIngredient": ["120g butter softened"],
                 "nutrition": {"@type": "NutritionInformation", "calories": "308 calories"}}
                </script></head><body></body></html>"#
                    .to_string(),
            ),
            (
                "silent",
                r#"<html><head><script type="application/ld+json">
                {"@type": "Recipe", "name": "Plain Loaf",
                 "recipeIngredient": ["500g strong white flour"]}
                </script></head><body></body></html>"#
                    .to_string(),
            ),
        ]);

        let content = |name: &str| {
            let report = import_web_link_and_wait(&app, &key, &format!("{base}/pages/{name}"));
            let branch_id = report["arrived"][0]["branch_id"]
                .as_str()
                .unwrap_or_else(|| panic!("{report}"))
                .to_string();
            let (_, recipe) = app.post_op(
                "get_recipe",
                Some(&key),
                &json!({ "branch_id": branch_id }).to_string(),
            );
            recipe["result"]["versions"][0]["content"].clone()
        };

        assert_eq!(
            content("stated")["nutrition"],
            json!({ "calories": 308.0, "basis": "per_serving" }),
            "a page that stated a number has it landed, per serving"
        );
        assert_eq!(
            content("silent")["nutrition"],
            Value::Null,
            "a page that stated none leaves the field empty rather than \
             inventing a plausible figure"
        );
    }
}

// --- Divergence: two recipes, a switch, and Ghosts (issue #55) ---------------

/// The real fixture: one Lineage, two Branches, one Branch Point.
///
/// Aurélien writes Korean Fried Chicken and splits it into sections. Marc, whose
/// Kitchen has never held the Branch, changes it — which starts his own Branch of
/// the same Lineage (a Copy, ADR 0004). He then asks Aurélien into Chez Marc, so
/// one Person can see both Branches, which is what a Divergence needs.
///
/// Returns (Aurélien's key, his Branch, Marc's Branch).
fn a_lineage_that_forked(app: &support::TestApp) -> (String, String, String) {
    let (_, mine_key, my_kitchen) = person_with_kitchen(app, "Aurélien");
    let (marc, marc_key, marc_kitchen) = person_with_kitchen(app, "Marc");

    // The Branch Point: the recipe as both of them knew it.
    let branch_point = json!({
        "kitchen_id": my_kitchen,
        "title": "Korean Fried Chicken",
        "ingredients": [
            { "kind": "section", "text": "Chicken" },
            { "kind": "ingredient", "text": "1.4 kg whole chicken" },
            { "kind": "ingredient", "text": "1 cup potato starch (or corn starch)" },
            { "kind": "section", "text": "Sauce" },
            { "kind": "ingredient", "text": "¼ cup honey" },
            { "kind": "ingredient", "text": "¼ cup brown sugar" },
            { "kind": "ingredient", "text": "2 Tbsp minced garlic" }
        ],
        "steps": [
            { "kind": "step", "text": "Coat the chicken in the starch and set aside." },
            { "kind": "step", "text": "Deep fry at 175 C until golden and crisp." }
        ],
    });
    let (status, created) =
        app.post_op("create_recipe", Some(&mine_key), &branch_point.to_string());
    assert_eq!(status, 200, "{created}");
    let my_branch = created["result"]["branch_id"].as_str().unwrap().to_string();

    // Marc's Branch. He cuts the sugar, puts chilli flakes in, swaps the deep
    // fry for an air fryer, and adds a resting step.
    backdate_branch_head(app, &my_branch);
    let (status, copied) = app.post_op(
        "save_recipe_version",
        Some(&marc_key),
        &json!({
            "branch_id": my_branch,
            "kitchen_id": marc_kitchen,
            "title": "Korean Fried Chicken",
            "ingredients": [
                { "kind": "section", "text": "Chicken" },
                { "kind": "ingredient", "text": "1.4 kg whole chicken" },
                { "kind": "ingredient", "text": "¾ cup potato starch" },
                { "kind": "section", "text": "Sauce" },
                { "kind": "ingredient", "text": "¼ cup honey" },
                { "kind": "ingredient", "text": "2 Tbsp minced garlic" },
                { "kind": "ingredient", "text": "1 tsp gochugaru" }
            ],
            "steps": [
                { "kind": "step", "text": "Coat the chicken in the starch and set aside." },
                { "kind": "step", "text": "Spray the basket and air fry at 200 C for 18 minutes." },
                { "kind": "step", "text": "Let it sit 5 minutes before saucing." }
            ],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{copied}");
    assert_eq!(
        copied["result"]["copied"],
        json!(true),
        "Marc's change is a Copy"
    );
    let marc_branch = copied["result"]["branch_id"].as_str().unwrap().to_string();

    // My own Branch moves on too, so the Branch Point is genuinely behind both
    // of us rather than being one side's head. `2 Tbsp minced garlic` is
    // retyped here character for character — it must produce nothing (ADR 0019).
    backdate_branch_head(app, &my_branch);
    let (status, mine) = app.post_op(
        "save_recipe_version",
        Some(&mine_key),
        &json!({
            "branch_id": my_branch,
            "title": "Korean Fried Chicken",
            "ingredients": [
                { "kind": "section", "text": "Chicken" },
                { "kind": "ingredient", "text": "1.4 kg whole chicken" },
                { "kind": "ingredient", "text": "1 cup potato starch (or corn starch)" },
                { "kind": "section", "text": "Sauce" },
                { "kind": "ingredient", "text": "¼ cup honey" },
                { "kind": "ingredient", "text": "¼ cup brown sugar" },
                { "kind": "ingredient", "text": "2 Tbsp minced garlic" },
                { "kind": "ingredient", "text": "1 Tbsp rice vinegar" }
            ],
            "steps": [
                { "kind": "step", "text": "Coat the chicken in the starch and set aside." },
                { "kind": "step", "text": "Deep fry at 190 C until golden and crisp." }
            ],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{mine}");
    assert_eq!(
        mine["result"]["copied"],
        json!(false),
        "my own edit stays on my Branch"
    );

    // Marc asks me into his Kitchen, which is how I come to see his Branch at
    // all — the same boundary get_recipe and get_thread enforce (ADR 0007).
    let (_, invite) = app.post_op(
        "invite_to_kitchen",
        Some(&marc_key),
        &json!({ "kitchen_id": marc_kitchen }).to_string(),
    );
    let secret = invite["result"]["secret"].as_str().unwrap().to_string();
    let (status, joined) = app.post_op(
        "accept_kitchen_invite",
        Some(&mine_key),
        &json!({ "secret": secret }).to_string(),
    );
    assert_eq!(status, 200, "{joined}");
    let _ = marc;

    (mine_key, my_branch, marc_branch)
}

fn rows_of<'a>(divergence: &'a Value, list: &str) -> &'a Vec<Value> {
    divergence["result"][list].as_array().unwrap()
}

/// The row whose text on either side is exactly this. Nothing in the answer is
/// addressed by an id, because no line has one.
fn row_saying<'a>(rows: &'a [Value], text: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["mine"]["text"] == json!(text) || row["theirs"]["text"] == json!(text))
        .unwrap_or_else(|| panic!("no row saying {text:?} in {rows:#?}"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_divergence_is_two_whole_recipes_with_the_unshared_lines_marked() {
    let app = support::spawn_app();
    let (key, my_branch, marc_branch) = a_lineage_that_forked(&app);

    let (status, divergence) = app.post_op(
        "divergence",
        Some(&key),
        &json!({ "branch_id": my_branch, "other_branch_id": marc_branch }).to_string(),
    );
    assert_eq!(status, 200, "{divergence}");

    // Both sides arrive whole and cookable — not a list of differences.
    let mine = &divergence["result"]["mine"]["content"];
    let theirs = &divergence["result"]["theirs"]["content"];
    assert_eq!(mine["ingredients"].as_array().unwrap().len(), 8);
    assert_eq!(theirs["ingredients"].as_array().unwrap().len(), 7);
    assert_eq!(mine["steps"].as_array().unwrap().len(), 2);
    assert_eq!(theirs["steps"].as_array().unwrap().len(), 3);
    assert_eq!(
        divergence["result"]["theirs"]["kitchen_name"],
        json!("Marc's Kitchen"),
        "the switch names the Kitchen you cross into"
    );

    let ingredients = rows_of(&divergence, "ingredients");

    // The words nobody touched are the same on both sides and need no reading.
    assert_eq!(
        row_saying(ingredients, "1.4 kg whole chicken")["state"],
        json!("same")
    );
    assert_eq!(
        row_saying(ingredients, "¼ cup honey")["state"],
        json!("same")
    );

    // A quantity altered: paired, because both descend from one Branch Point
    // line — not reported as one line removed and another added.
    let starch = row_saying(ingredients, "¾ cup potato starch");
    assert_eq!(starch["state"], json!("changed"));
    assert_eq!(starch["from_branch_point"], json!(true));
    assert_eq!(
        starch["mine"]["text"],
        json!("1 cup potato starch (or corn starch)")
    );
}

/// ADR 0019's load-bearing claim. Select-all-delete-retype is how a line gets
/// fixed on a phone; if it manufactured a divergence, nothing else here works.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retyping_a_line_identically_produces_no_divergence() {
    let app = support::spawn_app();
    let (key, my_branch, marc_branch) = a_lineage_that_forked(&app);

    let (_, divergence) = app.post_op(
        "divergence",
        Some(&key),
        &json!({ "branch_id": my_branch, "other_branch_id": marc_branch }).to_string(),
    );
    let garlic = row_saying(rows_of(&divergence, "ingredients"), "2 Tbsp minced garlic");
    assert_eq!(
        garlic["state"],
        json!("same"),
        "retyped character for character on my Branch, so there is nothing to show"
    );
}

/// A line present on one side only is a Ghost, and it keeps the position it
/// holds in the recipe that really has it (ADR 0014).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_line_only_one_side_has_is_a_ghost_in_its_own_position() {
    let app = support::spawn_app();
    let (key, my_branch, marc_branch) = a_lineage_that_forked(&app);

    let (_, divergence) = app.post_op(
        "divergence",
        Some(&key),
        &json!({ "branch_id": my_branch, "other_branch_id": marc_branch }).to_string(),
    );
    let ingredients = rows_of(&divergence, "ingredients");

    // Marc took the brown sugar out. It was at the Branch Point, so this can be
    // carried across as a removal rather than only read.
    let sugar = row_saying(ingredients, "¼ cup brown sugar");
    assert_eq!(sugar["state"], json!("only-mine"));
    assert_eq!(sugar["from_branch_point"], json!(true));
    assert_eq!(sugar["theirs"], json!(null));

    // Marc's gochugaru is a Ghost on my recipe for exactly the reason my brown
    // sugar is a Ghost on his: one mechanism, seen from two sides.
    let gochugaru = row_saying(ingredients, "1 tsp gochugaru");
    assert_eq!(gochugaru["state"], json!("only-theirs"));
    assert_eq!(gochugaru["from_branch_point"], json!(false));

    // Position: the honey is the last line both of us can still find above the
    // sugar, so the sugar sits under the honey rather than at the end.
    let texts: Vec<String> = ingredients
        .iter()
        .map(|row| {
            row["mine"]["text"]
                .as_str()
                .or(row["theirs"]["text"].as_str())
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    let honey = texts.iter().position(|t| t == "¼ cup honey").unwrap();
    let sugar_at = texts.iter().position(|t| t == "¼ cup brown sugar").unwrap();
    assert_eq!(
        sugar_at,
        honey + 1,
        "a Ghost sits where it sits over there: {texts:?}"
    );
}

/// ADR 0019's refusal, through a real Operation: two lines that both arrived
/// after the Branch Point and read nothing alike are shown unjoined, and
/// nothing anywhere in the answer labels a Pairing as a guess.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_uncertain_pairing_shows_both_lines_and_never_hedges() {
    let app = support::spawn_app();
    let (key, my_branch, marc_branch) = a_lineage_that_forked(&app);

    let (_, divergence) = app.post_op(
        "divergence",
        Some(&key),
        &json!({ "branch_id": my_branch, "other_branch_id": marc_branch }).to_string(),
    );
    let ingredients = rows_of(&divergence, "ingredients");

    // My rice vinegar and Marc's gochugaru both arrived after we parted and
    // land in the same part of the sauce. They are not joined.
    let vinegar = row_saying(ingredients, "1 Tbsp rice vinegar");
    assert_eq!(vinegar["state"], json!("only-mine"));
    assert_eq!(vinegar["theirs"], json!(null));
    let gochugaru = row_saying(ingredients, "1 tsp gochugaru");
    assert_eq!(gochugaru["state"], json!("only-theirs"));
    assert_eq!(gochugaru["mine"], json!(null));

    // A whole Step rewritten is the same refusal, and ADR 0019's accepted loss:
    // the old text beside the new is more use than "Marc rewrote this".
    let steps = rows_of(&divergence, "steps");
    assert_eq!(
        row_saying(steps, "Deep fry at 190 C until golden and crisp.")["state"],
        json!("only-mine")
    );
    assert_eq!(
        row_saying(
            steps,
            "Spray the basket and air fry at 200 C for 18 minutes."
        )["state"],
        json!("only-theirs")
    );

    // No confidence anywhere in the answer, on any row. A badge on some
    // markings and not others would add a decision to every line while giving
    // the cook nothing to decide with.
    let whole = divergence.to_string().to_lowercase();
    for hedge in [
        "confidence",
        "probably",
        "likely",
        "uncertain",
        "maybe",
        "score",
    ] {
        assert!(
            !whole.contains(hedge),
            "the answer must never hedge, found {hedge:?}"
        );
    }
}

/// The page is symmetric: the same lines are marked and the same Ghosts appear
/// whichever recipe you are standing in. Nothing is knowable only from one side.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_divergence_reads_the_same_from_either_branch() {
    let app = support::spawn_app();
    let (key, my_branch, marc_branch) = a_lineage_that_forked(&app);

    let ask = |a: &str, b: &str| {
        let (status, answer) = app.post_op(
            "divergence",
            Some(&key),
            &json!({ "branch_id": a, "other_branch_id": b }).to_string(),
        );
        assert_eq!(status, 200, "{answer}");
        answer
    };
    let from_mine = ask(&my_branch, &marc_branch);
    let from_theirs = ask(&marc_branch, &my_branch);

    assert_eq!(
        from_mine["result"]["branch_point_version_id"],
        from_theirs["result"]["branch_point_version_id"],
        "one Branch Point, computed the same way from either end"
    );

    // Every row exists on both readings with its sides swapped, so nothing is
    // knowable only by standing in the right place. Row ORDER is deliberately
    // not symmetric: you read your own recipe in your own order, and the other
    // side's unshared lines are Ghosts slotted in beside it — so my rice
    // vinegar sits where I put it, and from Marc's side it is his gochugaru
    // that sits where he put it.
    let swapped = |row: &Value| {
        json!({
            "kind": row["kind"],
            "state": match row["state"].as_str().unwrap() {
                "only-mine" => "only-theirs",
                "only-theirs" => "only-mine",
                other => other,
            },
            "from_branch_point": row["from_branch_point"],
            "mine": row["theirs"],
            "theirs": row["mine"],
        })
    };
    for list in ["ingredients", "steps"] {
        let here = rows_of(&from_mine, list);
        let there = rows_of(&from_theirs, list);
        assert_eq!(here.len(), there.len(), "{list}");
        for row in here {
            assert!(
                there.contains(&swapped(row)),
                "every row is readable from the other side too: {row:#?} missing from {there:#?}"
            );
        }
    }
}

/// A Branch in a Kitchen you do not cook in is not yours to read, and a
/// Divergence is no way around that (ADR 0007).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_divergence_cannot_reach_a_branch_you_could_not_otherwise_read() {
    let app = support::spawn_app();
    let (_, mine_key, my_kitchen) = person_with_kitchen(&app, "Aurélien");
    let (_, marc_key, marc_kitchen) = person_with_kitchen(&app, "Marc");

    let (_, mine) = app.post_op(
        "create_recipe",
        Some(&mine_key),
        &json!({ "kitchen_id": my_kitchen, "title": "Soupe" }).to_string(),
    );
    let my_branch = mine["result"]["branch_id"].as_str().unwrap().to_string();
    let (_, theirs) = app.post_op(
        "create_recipe",
        Some(&marc_key),
        &json!({ "kitchen_id": marc_kitchen, "title": "Soupe" }).to_string(),
    );
    let marc_branch = theirs["result"]["branch_id"].as_str().unwrap().to_string();

    let (status, refused) = app.post_op(
        "divergence",
        Some(&mine_key),
        &json!({ "branch_id": my_branch, "other_branch_id": marc_branch }).to_string(),
    );
    // Marc's Branch was worked out from the id, not named as a Kitchen, so the
    // refusal is the one an id naming nothing gets (ADR 0040).
    assert_eq!(status, 404, "{refused}");
    assert_eq!(refused["error"]["kind"], json!("not_found"));
    assert_eq!(refused["error"]["message"], json!("no such Branch"));
}

/// Two recipes that were never one recipe have no Branch Point and nothing
/// between them to read. Kamosu says so rather than inventing a comparison.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_branches_of_different_lineages_are_not_a_divergence() {
    let app = support::spawn_app();
    let (_, key, kitchen) = person_with_kitchen(&app, "Aurélien");

    let mut branches = Vec::new();
    for title in ["Soupe", "Katsu Curry"] {
        let (_, created) = app.post_op(
            "create_recipe",
            Some(&key),
            &json!({ "kitchen_id": kitchen, "title": title }).to_string(),
        );
        branches.push(created["result"]["branch_id"].as_str().unwrap().to_string());
    }

    let (status, refused) = app.post_op(
        "divergence",
        Some(&key),
        &json!({ "branch_id": branches[0], "other_branch_id": branches[1] }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["error"]["kind"], json!("bad_request"));
}

/// There is no "take all" Operation, and no way to ask for one. The absence is
/// the decision: taking every line one at a time is a person making a recipe,
/// whereas one button doing it is a merge with extra steps (ADR 0014).
#[test]
fn the_catalogue_offers_no_way_to_take_a_whole_branch() {
    for operation in kamosu::catalogue::OPERATIONS.iter() {
        let name = operation.name;
        assert!(
            !(name.contains("merge") && name.contains("branch")),
            "no Operation merges Branches, found {name}"
        );
        assert!(
            !name.contains("take_all")
                && !name.contains("accept_all")
                && !name.contains("apply_all"),
            "no Operation takes a whole Branch at once, found {name}"
        );
    }
}

/// Kind is part of a line's identity. A Section heading never pairs with an
/// Ingredient Line, however alike the words happen to be — otherwise splitting
/// a list into sections would read as rewriting the ingredients.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_section_heading_never_pairs_with_an_ingredient_line() {
    let app = support::spawn_app();
    let (_, mine_key, my_kitchen) = person_with_kitchen(&app, "Aurélien");
    let (_, marc_key, marc_kitchen) = person_with_kitchen(&app, "Marc");

    // The Branch Point has neither line, so both arrive after the parting —
    // the case where only the words are left to go on.
    let (_, created) = app.post_op(
        "create_recipe",
        Some(&mine_key),
        &json!({ "kitchen_id": my_kitchen, "title": "Sauce", "ingredients": [] }).to_string(),
    );
    let my_branch = created["result"]["branch_id"].as_str().unwrap().to_string();

    backdate_branch_head(&app, &my_branch);
    let (_, copied) = app.post_op(
        "save_recipe_version",
        Some(&marc_key),
        &json!({
            "branch_id": my_branch,
            "kitchen_id": marc_kitchen,
            "title": "Sauce",
            "ingredients": [{ "kind": "ingredient", "text": "Sauce" }],
        })
        .to_string(),
    );
    let marc_branch = copied["result"]["branch_id"].as_str().unwrap().to_string();

    backdate_branch_head(&app, &my_branch);
    app.post_op(
        "save_recipe_version",
        Some(&mine_key),
        &json!({
            "branch_id": my_branch,
            "title": "Sauce",
            "ingredients": [{ "kind": "section", "text": "Sauce" }],
        })
        .to_string(),
    );

    let (_, invite) = app.post_op(
        "invite_to_kitchen",
        Some(&marc_key),
        &json!({ "kitchen_id": marc_kitchen }).to_string(),
    );
    app.post_op(
        "accept_kitchen_invite",
        Some(&mine_key),
        &json!({ "secret": invite["result"]["secret"].as_str().unwrap() }).to_string(),
    );

    let (status, divergence) = app.post_op(
        "divergence",
        Some(&mine_key),
        &json!({ "branch_id": my_branch, "other_branch_id": marc_branch }).to_string(),
    );
    assert_eq!(status, 200, "{divergence}");

    // Identical text, different kinds: two rows, not one changed line.
    let rows = rows_of(&divergence, "ingredients");
    assert_eq!(rows.len(), 2, "{rows:#?}");
    assert!(
        rows.iter().all(|row| row["state"] != json!("changed")),
        "a heading and an ingredient are never the same line: {rows:#?}"
    );
}

// --- One shelf and word search (issue #62) -----------------------------------

/// Put one recipe on a Kitchen's shelf and answer its Branch id.
fn shelve_recipe(app: &support::TestApp, key: &str, kitchen: &str, recipe: Value) -> String {
    let mut input = recipe;
    input["kitchen_id"] = json!(kitchen);
    let (status, created) = app.post_op("create_recipe", Some(key), &input.to_string());
    assert_eq!(status, 200, "{created}");
    created["result"]["branch_id"]
        .as_str()
        .expect("a Branch id")
        .to_string()
}

/// The same, for the many recipes here that need nothing but a title — which
/// is all a recipe ever needs (#6).
fn shelve(app: &support::TestApp, key: &str, kitchen: &str, title: &str) -> String {
    shelve_recipe(app, key, kitchen, json!({ "title": title }))
}

/// Search the shelf and answer the entries, failing loudly on any error the
/// Door reported rather than quietly reading `null` as "nothing found".
fn shelf(app: &support::TestApp, key: &str, input: Value) -> Vec<Value> {
    let (status, answer) = app.post_op("search_recipes", Some(key), &input.to_string());
    assert_eq!(status, 200, "{answer}");
    answer["result"]["recipes"]
        .as_array()
        .expect("recipes")
        .clone()
}

fn titles(entries: &[Value]) -> Vec<&str> {
    entries
        .iter()
        .map(|entry| entry["title"].as_str().unwrap())
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_shelf_is_alphabetical_and_carries_no_kitchen_anywhere_on_it() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    for title in ["Sukiyaki Udon", "Airfried Cauliflower", "Miso Soup"] {
        shelve(&app, &key, &kitchen_id, title);
    }

    let entries = shelf(&app, &key, json!({}));
    assert_eq!(
        titles(&entries),
        ["Airfried Cauliflower", "Miso Soup", "Sukiyaki Udon"],
        "the shelf is alphabetical, so the thumb can learn where a recipe sits"
    );

    // No Kitchen name and no Kitchen id: a card cannot print a fence that is
    // not there (ADR 0027), and the surest way is to never send one.
    let entry = entries[0].as_object().unwrap();
    assert!(
        !entry.keys().any(|key| key.contains("kitchen")),
        "a shelf entry names no Kitchen: {entry:#?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn one_lineage_held_by_two_kitchens_is_one_card_and_a_filter_picks_a_branch() {
    let app = support::spawn_app();
    let (_person, key, home) = person_with_kitchen(&app, "Aurélien");
    let (_, second) = app.post_op(
        "create_kitchen",
        Some(&key),
        &json!({ "name": "Chez Marc" }).to_string(),
    );
    let other = second["result"]["id"].as_str().unwrap().to_string();

    let branch = shelve(&app, &key, &home, "Katsu Curry");

    // Editing it on behalf of the second Kitchen is a Copy: one Lineage, two
    // Branches, both on this Person's shelf.
    let (status, copied) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": branch,
            "kitchen_id": other,
            "title": "Katsu Curry",
            "note": "less sauce",
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{copied}");
    assert_eq!(copied["result"]["copied"], json!(true), "{copied}");
    let copied_branch = copied["result"]["branch_id"].as_str().unwrap().to_string();

    let everything = shelf(&app, &key, json!({}));
    assert_eq!(
        titles(&everything),
        ["Katsu Curry"],
        "one card per Lineage, however many Kitchens hold a Branch of it"
    );

    // Filtered to one Kitchen, the card opens that Kitchen's Branch.
    let here = shelf(&app, &key, json!({ "kitchen_id": home }));
    assert_eq!(here.len(), 1, "{here:#?}");
    assert_eq!(here[0]["branch_id"], json!(branch));

    let there = shelf(&app, &key, json!({ "kitchen_id": other }));
    assert_eq!(there.len(), 1, "{there:#?}");
    assert_eq!(there[0]["branch_id"], json!(copied_branch));

    // Nothing was remembered: asking again with no filter is the whole shelf.
    // A filter that persists is a mode, and a mode you forgot you set is the
    // Kitchen switcher wearing a hat.
    assert_eq!(shelf(&app, &key, json!({})).len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_recipe_in_another_language_is_shown_and_marked_rather_than_hidden() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );

    shelve_recipe(
        &app,
        &key,
        &kitchen_id,
        json!({ "title": "Îles Flottantes", "language": "fr" }),
    );
    shelve_recipe(
        &app,
        &key,
        &kitchen_id,
        json!({ "title": "Miso Soup", "language": "en" }),
    );

    let entries = shelf(&app, &key, json!({}));
    assert_eq!(
        titles(&entries),
        ["Îles Flottantes", "Miso Soup"],
        "a Language preference never hides a recipe from its owner (ADR 0006)"
    );
    assert_eq!(entries[0]["language_fallback"], json!(true));
    assert_eq!(entries[0]["language"], json!("fr"));
    assert_eq!(
        entries[1]["language_fallback"],
        json!(false),
        "there is nothing to mark when the reader got the Language they asked for"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_lineage_with_a_branch_in_the_reading_language_opens_that_one_unmarked() {
    let app = support::spawn_app();
    let (_person, key, home) = person_with_kitchen(&app, "Aurélien");
    let (_, second) = app.post_op(
        "create_kitchen",
        Some(&key),
        &json!({ "name": "Chez Marc" }).to_string(),
    );
    let other = second["result"]["id"].as_str().unwrap().to_string();
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );

    // The Lineage starts in French; a second Branch of it is in English.
    let french = shelve_recipe(
        &app,
        &key,
        &home,
        json!({ "title": "Purée de Pommes de Terre", "language": "fr" }),
    );
    let (_, copied) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": french,
            "kitchen_id": other,
            "title": "Purée de Pommes de Terre",
            "note": "the same, mine",
        })
        .to_string(),
    );
    let english_branch = copied["result"]["branch_id"].as_str().unwrap().to_string();
    app.core
        .db()
        .with_conn(|conn| {
            conn.execute(
                "UPDATE branches SET language = 'en' WHERE id = ?1",
                rusqlite::params![english_branch],
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            Ok(())
        })
        .unwrap();

    let entries = shelf(&app, &key, json!({}));
    assert_eq!(entries.len(), 1, "still one card per Lineage: {entries:#?}");
    assert_eq!(
        entries[0]["branch_id"],
        json!(english_branch),
        "the card opens the Branch written in the reader's own Language"
    );
    assert_eq!(entries[0]["language_fallback"], json!(false));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_exact_title_wins_and_every_result_quotes_the_line_that_matched() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    shelve(&app, &key, &kitchen_id, "Chocolate");
    shelve(&app, &key, &kitchen_id, "Chocolate Chip Cookies");
    shelve_recipe(
        &app,
        &key,
        &kitchen_id,
        json!({
            "title": "Chilli con carne",
            "ingredients": [{ "kind": "ingredient", "text": "50 g dark chocolate" }],
        }),
    );
    shelve_recipe(
        &app,
        &key,
        &kitchen_id,
        json!({
            "title": "Braised Beef",
            "steps": [
                { "kind": "step", "text": "Brown the beef." },
                { "kind": "step", "text": "Stir in a square of chocolate." },
            ],
        }),
    );
    shelve(&app, &key, &kitchen_id, "Miso Soup");

    let found = shelf(&app, &key, json!({ "query": "chocolate" }));
    assert_eq!(
        titles(&found),
        [
            "Chocolate",
            "Chocolate Chip Cookies",
            "Chilli con carne",
            "Braised Beef"
        ],
        "the exact title wins, because most searching is navigation (ADR 0027)"
    );

    // Every one of them can say what matched — the case where trust in search
    // is won or lost.
    assert_eq!(found[0]["matched"]["where"], json!("title"));
    assert_eq!(found[2]["matched"]["where"], json!("ingredient"));
    assert_eq!(
        found[2]["matched"]["line"],
        json!("50 g dark chocolate"),
        "the quoted line is what makes a surprising result legible"
    );
    assert_eq!(found[3]["matched"]["where"], json!("step"));
    assert_eq!(
        found[3]["matched"]["step_number"],
        json!(2),
        "which step it was, counted as the recipe page counts them"
    );

    // And the unsearched shelf marks nothing as matched.
    let everything = shelf(&app, &key, json!({}));
    assert!(
        everything.iter().all(|entry| entry["matched"].is_null()),
        "{everything:#?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_accent_left_off_still_finds_the_recipe() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    shelve_recipe(
        &app,
        &key,
        &kitchen_id,
        json!({ "title": "Gâteau Au Chocolat", "language": "fr" }),
    );

    let found = shelf(&app, &key, json!({ "query": "gateau" }));
    assert_eq!(
        titles(&found),
        ["Gâteau Au Chocolat"],
        "refusing this over an accent buried three presses deep is a search that looks broken"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn search_reaches_this_persons_own_attempts_and_nobody_elses() {
    let app = support::spawn_app();
    let (_aurelien, mine_key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let camille = app.core.create_person("Camille").expect("person");
    let camille_key = app
        .core
        .mint_access_key(&camille, "browser", false)
        .unwrap()
        .secret;
    let (_, invite) = app.post_op(
        "invite_to_kitchen",
        Some(&mine_key),
        &json!({ "kitchen_id": kitchen_id }).to_string(),
    );
    app.post_op(
        "accept_kitchen_invite",
        Some(&camille_key),
        &json!({ "secret": invite["result"]["secret"].as_str().unwrap() }).to_string(),
    );

    let branch = shelve(&app, &mine_key, &kitchen_id, "Miso Soup");

    // Camille cooks it and writes her own diary line about it.
    let (_, attempt) = app.post_op(
        "start_attempt",
        Some(&camille_key),
        &json!({ "branch_id": branch }).to_string(),
    );
    app.post_op(
        "edit_attempt",
        Some(&camille_key),
        &json!({
            "attempt_id": attempt["result"]["id"].as_str().unwrap(),
            "note": "burnt it again, honestly",
        })
        .to_string(),
    );

    let hers = shelf(&app, &camille_key, json!({ "query": "burnt" }));
    assert_eq!(titles(&hers), ["Miso Soup"]);
    assert_eq!(hers[0]["matched"]["where"], json!("attempt"));
    assert_eq!(
        hers[0]["matched"]["line"],
        json!("burnt it again, honestly")
    );

    assert!(
        shelf(&app, &mine_key, json!({ "query": "burnt" })).is_empty(),
        "a Kitchen-mate's diary is hers to show you, not the index's"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn my_recipes_means_created_branched_or_cooked() {
    let app = support::spawn_app();
    let (_aurelien, mine_key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let camille = app.core.create_person("Camille").expect("person");
    let camille_key = app
        .core
        .mint_access_key(&camille, "browser", false)
        .unwrap()
        .secret;
    let (_, invite) = app.post_op(
        "invite_to_kitchen",
        Some(&mine_key),
        &json!({ "kitchen_id": kitchen_id }).to_string(),
    );
    app.post_op(
        "accept_kitchen_invite",
        Some(&camille_key),
        &json!({ "secret": invite["result"]["secret"].as_str().unwrap() }).to_string(),
    );

    let cooked = shelve(&app, &mine_key, &kitchen_id, "Miso Soup");
    shelve(&app, &mine_key, &kitchen_id, "Katsu Curry");

    // Camille sees the whole shelf, and none of it is hers yet.
    assert_eq!(shelf(&app, &camille_key, json!({})).len(), 2);
    assert!(
        shelf(&app, &camille_key, json!({ "mine": true })).is_empty(),
        "she has neither written nor cooked any of it"
    );

    // Cooking one is the whole of joining it to her own history — no
    // curating, at exactly the moment she would want it.
    app.post_op(
        "start_attempt",
        Some(&camille_key),
        &json!({ "branch_id": cooked }).to_string(),
    );
    assert_eq!(
        titles(&shelf(&app, &camille_key, json!({ "mine": true }))),
        ["Miso Soup"]
    );

    // And it stays the whole shelf for Aurélien, who wrote both.
    assert_eq!(shelf(&app, &mine_key, json!({ "mine": true })).len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nothing_found_answers_no_entries_and_the_query_it_was_asked() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    shelve(&app, &key, &kitchen_id, "Miso Soup");

    let (status, answer) = app.post_op(
        "search_recipes",
        Some(&key),
        &json!({ "query": "osso buco" }).to_string(),
    );
    assert_eq!(status, 200, "{answer}");
    assert_eq!(
        answer["result"]["recipes"],
        json!([]),
        "nothing found is an ordinary answer, not an error"
    );
    assert_eq!(
        answer["result"]["query"],
        json!("osso buco"),
        "the screen names the query from the answer, not from its own field"
    );

    // A shelf asked for nothing in particular says so, rather than echoing an
    // empty string the screen would then have to explain.
    let (_, all) = app.post_op("search_recipes", Some(&key), &json!({}).to_string());
    assert_eq!(all["result"]["query"], json!(null));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_shelf_holds_only_what_this_persons_kitchens_hold() {
    let app = support::spawn_app();
    let (_aurelien, mine_key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_stranger, stranger_key, _their_kitchen) = person_with_kitchen(&app, "Marc");

    shelve(&app, &mine_key, &kitchen_id, "Miso Soup");

    assert!(
        shelf(&app, &stranger_key, json!({})).is_empty(),
        "everything on a shelf is held by a Kitchen you belong to (ADR 0026)"
    );

    // And asking about a Kitchen you do not cook in is refused outright rather
    // than answered with an empty shelf.
    let (status, refused) = app.post_op(
        "search_recipes",
        Some(&stranger_key),
        &json!({ "kitchen_id": kitchen_id }).to_string(),
    );
    assert_eq!(status, 401, "{refused}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_is_numbered_as_the_recipe_page_numbers_it_and_a_section_is_neither() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    shelve_recipe(
        &app,
        &key,
        &kitchen_id,
        json!({
            "title": "Katsu Curry",
            "steps": [
                { "kind": "section", "text": "For the sauce" },
                { "kind": "step", "text": "Soften the onion." },
                { "kind": "section", "text": "To finish" },
                { "kind": "step", "text": "Fry the katsu until amber." },
            ],
        }),
    );

    // The recipe page counts Steps alone, Sections taking no number — so this
    // is step 2, not the fourth entry in the array. Answering the position
    // would have every Section silently shift the number that follows it.
    let found = shelf(&app, &key, json!({ "query": "amber" }));
    assert_eq!(found[0]["matched"]["where"], json!("step"));
    assert_eq!(found[0]["matched"]["step_number"], json!(2));

    // A Section header is searched with the rest of the recipe, and answers as
    // what it is rather than being called a step it is not.
    let heading = shelf(&app, &key, json!({ "query": "to finish" }));
    assert_eq!(heading[0]["matched"]["where"], json!("section"));
    assert_eq!(heading[0]["matched"]["line"], json!("To finish"));
    assert_eq!(heading[0]["matched"]["step_number"], json!(null));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_word_only_the_other_language_uses_still_finds_the_recipe() {
    let app = support::spawn_app();
    let (_person, key, home) = person_with_kitchen(&app, "Aurélien");
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );

    // One Lineage, two Branches: the English one is what the card opens, and
    // the French one is where the word *chocolat* actually is.
    let english = shelve_recipe(
        &app,
        &key,
        &home,
        json!({
            "title": "Chocolate Mousse",
            "language": "en",
            "ingredients": [{ "kind": "ingredient", "text": "200 g dark chocolate" }],
        }),
    );
    let (status, translated) = app.post_op(
        "start_translation",
        Some(&key),
        &json!({
            "branch_id": english,
            "language": "fr",
            "title": "Mousse au chocolat",
            "ingredients": [{ "kind": "ingredient", "text": "200 g de chocolat noir" }],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{translated}");

    // A Translation is a Branch of the same Lineage (ADR 0006), and it is that
    // multilingual model that makes the match (ADR 0027) — so the French words
    // find the recipe even though the card opens the English rendering.
    let found = shelf(&app, &key, json!({ "query": "chocolat noir" }));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(
        found[0]["branch_id"],
        json!(english),
        "the card still opens the Branch written in the reader's own Language"
    );
    assert_eq!(found[0]["title"], json!("Chocolate Mousse"));
    assert_eq!(
        found[0]["matched"]["line"],
        json!("200 g de chocolat noir"),
        "and it quotes the line that actually matched, in the words it is written in"
    );
}

// ── Translations (#56, ADR 0006) ──────────────────────────────────────────────
//
// A Translation is a Branch of the same Lineage carrying its own Language.
// There is no Translation table, no translation flag and no separate screen —
// what follows exercises ordinary Branches, ordinary Versions and ordinary
// saves, and everything that makes one a Translation is read back off columns
// two Operations write.

/// A French rendering of an English recipe, written where the corpus's own
/// French recipes are written: real sentences, since a Language read off two
/// words would be a Language read off nothing.
fn french_mousse() -> Value {
    json!({
        "title": "Mousse au chocolat",
        "ingredients": [
            { "kind": "ingredient", "text": "200 g de chocolat noir à pâtisser" },
            { "kind": "ingredient", "text": "6 œufs frais, blancs et jaunes séparés" },
            { "kind": "ingredient", "text": "Une pincée de sel fin" },
        ],
        "steps": [
            { "kind": "step", "text": "Faites fondre le chocolat au bain-marie très doucement." },
            { "kind": "step", "text": "Montez les blancs en neige bien ferme avec la pincée de sel." },
            { "kind": "step", "text": "Incorporez délicatement les blancs au chocolat fondu." },
        ],
    })
}

fn english_mousse() -> Value {
    json!({
        "title": "Chocolate Mousse",
        "ingredients": [
            { "kind": "ingredient", "text": "200 g of dark chocolate for baking" },
            { "kind": "ingredient", "text": "6 fresh eggs, whites and yolks separated" },
            { "kind": "ingredient", "text": "A generous pinch of fine salt" },
        ],
        "steps": [
            { "kind": "step", "text": "Melt the chocolate over a pan of barely simmering water." },
            { "kind": "step", "text": "Whisk the whites to stiff peaks with the pinch of salt." },
            { "kind": "step", "text": "Fold the whites gently into the melted chocolate." },
        ],
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_translation_is_an_ordinary_branch_of_the_same_lineage_in_another_language() {
    let app = support::spawn_app();
    let (_person, key, kitchen) = person_with_kitchen(&app, "Aurélien");
    let english = shelve_recipe(&app, &key, &kitchen, english_mousse());

    let (_, source) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": english }).to_string(),
    );
    let source_lineage = source["result"]["lineage_id"].as_str().unwrap().to_string();
    let source_head = source["result"]["head_version_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        source["result"]["translation"],
        json!(null),
        "the original is the Branch that translates nothing — computed, never declared"
    );

    let mut input = french_mousse();
    input["branch_id"] = json!(english);
    input["language"] = json!("fr");
    let (status, translated) = app.post_op("start_translation", Some(&key), &input.to_string());
    assert_eq!(status, 200, "{translated}");
    let french = translated["result"]["branch_id"].as_str().unwrap();

    // The same Lineage. Not a related recipe, not a second recipe: the same
    // recipe written differently, so its Attempts, ratings and history are
    // one recipe's (ADR 0005).
    assert_eq!(translated["result"]["lineage_id"], json!(source_lineage));
    assert_ne!(json!(french), json!(english));
    assert_eq!(translated["result"]["language"], json!("fr"));

    // Each Version records the Version of the source it translates.
    assert_eq!(
        translated["result"]["translation"]["translates_version_id"],
        json!(source_head)
    );
    assert_eq!(
        translated["result"]["translation"]["source_branch_id"],
        json!(english)
    );
    assert_eq!(
        translated["result"]["translation"]["versions_behind"],
        json!(0),
        "written against the source as it stands, so it starts up to date"
    );
    assert_eq!(
        translated["result"]["versions"][0]["translates_version_id"],
        json!(source_head),
        "on the Version itself, so the record travels with it"
    );

    // Its chain starts fresh rather than carrying the source's — the one thing
    // that separates a Translation from a Copy.
    assert_eq!(
        translated["result"]["versions"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        translated["result"]["versions"][0]["parent_version_id"],
        json!(null)
    );

    // And on the Thread it is simply another Branch of this Lineage.
    let (_, thread) = app.post_op(
        "get_thread",
        Some(&key),
        &json!({ "branch_id": english }).to_string(),
    );
    let branches = thread["result"]["branches"].as_array().unwrap();
    assert_eq!(branches.len(), 2, "{branches:#?}");
    let translation = branches
        .iter()
        .find(|branch| branch["branch_id"] == json!(french))
        .expect("the Translation is on the Thread");
    assert_eq!(translation["language"], json!("fr"));
    assert_eq!(
        translation["translation"]["translates_version_id"],
        json!(source_head)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn how_far_behind_a_translation_has_fallen_is_exact_and_moves_as_the_source_moves() {
    let app = support::spawn_app();
    let (_person, key, kitchen) = person_with_kitchen(&app, "Aurélien");
    let english = shelve_recipe(&app, &key, &kitchen, english_mousse());

    let mut input = french_mousse();
    input["branch_id"] = json!(english);
    input["language"] = json!("fr");
    let (_, translated) = app.post_op("start_translation", Some(&key), &input.to_string());
    let french = translated["result"]["branch_id"]
        .as_str()
        .unwrap()
        .to_string();

    let behind = |branch: &str| -> Value {
        let (_, read) = app.post_op(
            "get_recipe",
            Some(&key),
            &json!({ "branch_id": branch }).to_string(),
        );
        read["result"]["translation"]["versions_behind"].clone()
    };
    assert_eq!(behind(&french), json!(0));

    // The English moves on twice. Nobody marks the French as stale; the fact
    // simply becomes true, because it is counted rather than recorded.
    for extra in ["Rest the mousse overnight.", "Serve with a spoon of cream."] {
        backdate_branch_head(&app, &english);
        let mut edit = english_mousse();
        edit["branch_id"] = json!(english);
        edit["steps"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "kind": "step", "text": extra }));
        let (status, saved) = app.post_op("save_recipe_version", Some(&key), &edit.to_string());
        assert_eq!(status, 200, "{saved}");
    }
    assert_eq!(behind(&french), json!(2));

    // Editing the French wording does not claim it has caught up: the pointer
    // carries forward from the Version being replaced.
    backdate_branch_head(&app, &french);
    let mut reword = french_mousse();
    reword["branch_id"] = json!(french);
    reword["steps"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "kind": "step", "text": "Réservez au frais deux heures." }));
    let (status, reworded) = app.post_op("save_recipe_version", Some(&key), &reword.to_string());
    assert_eq!(status, 200, "{reworded}");
    assert_eq!(behind(&french), json!(2), "still two behind: {reworded}");

    // Bringing it up to date is saying which Version it now renders.
    let (_, source) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": english }).to_string(),
    );
    let head_now = source["result"]["head_version_id"].as_str().unwrap();
    backdate_branch_head(&app, &french);
    let mut caught_up = french_mousse();
    caught_up["branch_id"] = json!(french);
    caught_up["translates_version_id"] = json!(head_now);
    caught_up["steps"].as_array_mut().unwrap().extend([
        json!({ "kind": "step", "text": "Laissez reposer la mousse toute une nuit." }),
        json!({ "kind": "step", "text": "Servez avec une cuillerée de crème." }),
    ]);
    let (status, saved) = app.post_op("save_recipe_version", Some(&key), &caught_up.to_string());
    assert_eq!(status, 200, "{saved}");
    assert_eq!(saved["result"]["translates_version_id"], json!(head_now));
    assert_eq!(behind(&french), json!(0));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_language_is_detected_from_the_text_when_there_is_none_to_disagree_with() {
    let app = support::spawn_app();
    let (_person, key, kitchen) = person_with_kitchen(&app, "Aurélien");
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );

    // Written in French by a cook whose Reading Language is English: the
    // recipe's own text decides, not the account's preference.
    let french = shelve_recipe(&app, &key, &kitchen, french_mousse());
    let (_, read) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": french }).to_string(),
    );
    assert_eq!(read["result"]["language"], json!("fr"));

    // A recipe that is nothing but a title has no text to read a Language out
    // of — three of the real 86 are exactly this — so the writer's own
    // Language answers rather than a guess.
    let bare = shelve(&app, &key, &kitchen, "Dan Dan Noodles");
    let (_, bare_read) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": bare }).to_string(),
    );
    assert_eq!(bare_read["result"]["language"], json!("en"));

    // And a Language stated outright is never second-guessed.
    let mut stated = french_mousse();
    stated["language"] = json!("es");
    let spanish = shelve_recipe(&app, &key, &kitchen, stated);
    let (_, spanish_read) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": spanish }).to_string(),
    );
    assert_eq!(spanish_read["result"]["language"], json!("es"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_language_that_disagrees_with_the_text_is_offered_and_never_taken() {
    let app = support::spawn_app();
    let (_person, key, kitchen) = person_with_kitchen(&app, "Aurélien");
    let mut mislabelled = french_mousse();
    mislabelled["language"] = json!("en");
    let branch = shelve_recipe(&app, &key, &kitchen, mislabelled);

    // The save reads the text, disagrees, and says so — and changes nothing.
    backdate_branch_head(&app, &branch);
    let mut edit = french_mousse();
    edit["branch_id"] = json!(branch);
    edit["ingredients"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "kind": "ingredient", "text": "Un peu de sucre glace pour finir" }));
    let (status, saved) = app.post_op("save_recipe_version", Some(&key), &edit.to_string());
    assert_eq!(status, 200, "{saved}");
    assert_eq!(saved["result"]["language_offer"], json!("fr"));
    assert_eq!(
        saved["result"]["language"],
        json!("en"),
        "offered, not taken: a Language is never changed without the cook saying so"
    );

    // Saying so is its own Operation, and it makes a Version — a label
    // alterable without a trace would be a hole in an append-only history.
    let (_, before) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch }).to_string(),
    );
    let versions_before = before["result"]["versions"].as_array().unwrap().len();

    let (status, set) = app.post_op(
        "set_recipe_language",
        Some(&key),
        &json!({ "branch_id": branch, "language": "fr" }).to_string(),
    );
    assert_eq!(status, 200, "{set}");
    assert_eq!(set["result"]["language"], json!("fr"));

    let (_, after) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch }).to_string(),
    );
    assert_eq!(after["result"]["language"], json!("fr"));
    let versions = after["result"]["versions"].as_array().unwrap();
    assert_eq!(
        versions.len(),
        versions_before + 1,
        "changing a Language makes a Version"
    );
    assert_eq!(
        versions.last().unwrap()["language"],
        json!("fr"),
        "and the Version says which Language it left the recipe in"
    );

    // Saved again, the text and the label now agree, so nothing is offered.
    backdate_branch_head(&app, &branch);
    let mut again = french_mousse();
    again["branch_id"] = json!(branch);
    again["ingredients"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "kind": "ingredient", "text": "Un peu de sucre glace, si vous voulez" }));
    let (_, quiet) = app.post_op("save_recipe_version", Some(&key), &again.to_string());
    assert_eq!(quiet["result"]["language_offer"], json!(null));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_is_permanent_and_unremarkable_no_prompt_no_badge_no_nag() {
    let app = support::spawn_app();
    let (_person, key, kitchen) = person_with_kitchen(&app, "Aurélien");
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );

    // The real corpus's *Sukiyaki Udon*: an English title over a French recipe.
    // No detector can be right about it, so the cook says so instead.
    let bilingual = json!({
        "title": "Sukiyaki Udon",
        "ingredients": [
            { "kind": "section", "text": "Pour la sauce" },
            { "kind": "ingredient", "text": "2 portions of udon noodles" },
            { "kind": "ingredient", "text": "3 c.s. de sauce soja" },
            { "kind": "ingredient", "text": "1 c.s. de sucre" },
        ],
        "steps": [
            { "kind": "step", "text": "Préchauffez l'huile dans une poêle." },
            { "kind": "step", "text": "Add the beef and cook it until browned." },
            { "kind": "step", "text": "Versez la sauce soja et le sucre dans la poêle." },
        ],
    });
    let branch = shelve_recipe(&app, &key, &kitchen, bilingual.clone());

    let (status, set) = app.post_op(
        "set_recipe_language",
        Some(&key),
        &json!({ "branch_id": branch, "language": "unknown" }).to_string(),
    );
    assert_eq!(status, 200, "{set}");
    assert_eq!(set["result"]["language"], json!("unknown"));

    // No prompt. Saving it again offers nothing, however the detector reads it.
    backdate_branch_head(&app, &branch);
    let mut edit = bilingual.clone();
    edit["branch_id"] = json!(branch);
    edit["steps"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "kind": "step", "text": "Servez dans des bols bien chauds." }));
    let (_, saved) = app.post_op("save_recipe_version", Some(&key), &edit.to_string());
    assert_eq!(saved["result"]["language"], json!("unknown"));
    assert_eq!(
        saved["result"]["language_offer"],
        json!(null),
        "a recipe honestly written in two Languages is never nagged"
    );

    // No badge, and no hiding: it shows to an English reader unmarked.
    let entries = shelf(&app, &key, json!({}));
    let entry = entries
        .iter()
        .find(|entry| entry["title"] == json!("Sukiyaki Udon"))
        .expect("shown to a reader whose Language it is not in");
    assert_eq!(entry["language"], json!("unknown"));
    assert_eq!(
        entry["language_fallback"],
        json!(false),
        "it is not in a Language the reader failed to ask for; it is in no single Language"
    );

    // It can neither be a Translation nor have one.
    let mut attempt = french_mousse();
    attempt["branch_id"] = json!(branch);
    attempt["language"] = json!("fr");
    let (status, refused) = app.post_op("start_translation", Some(&key), &attempt.to_string());
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["error"]["kind"], json!("bad_request"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_recipe_that_is_translated_cannot_then_be_called_unknown() {
    let app = support::spawn_app();
    let (_person, key, kitchen) = person_with_kitchen(&app, "Aurélien");
    let english = shelve_recipe(&app, &key, &kitchen, english_mousse());
    let mut input = french_mousse();
    input["branch_id"] = json!(english);
    input["language"] = json!("fr");
    let (_, translated) = app.post_op("start_translation", Some(&key), &input.to_string());
    let french = translated["result"]["branch_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Both ends of the pointer: a Translation renders one source text, and a
    // recipe honestly written in two Languages is no such text.
    for branch in [&english, &french] {
        let (status, refused) = app.post_op(
            "set_recipe_language",
            Some(&key),
            &json!({ "branch_id": branch, "language": "unknown" }).to_string(),
        );
        assert_eq!(status, 400, "{refused}");
    }

    // And a Translation is in a *different* Language from what it renders.
    let mut same = english_mousse();
    same["branch_id"] = json!(english);
    same["language"] = json!("en");
    let (status, refused) = app.post_op("start_translation", Some(&key), &same.to_string());
    assert_eq!(status, 400, "{refused}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_translating_writes_under_the_persons_credential_and_the_kitchens_hand() {
    let app = support::spawn_app();
    let (person, _key, kitchen) = person_with_kitchen(&app, "Aurélien");
    // The agent holds an Access Key of Aurélien's — a Credential naming him,
    // never one of its own (ADR 0006: a scribe, not an author).
    let agent_key = app
        .core
        .mint_access_key(&person, "translation agent", false)
        .unwrap()
        .secret;

    let english = shelve_recipe(&app, &agent_key, &kitchen, english_mousse());
    let (_, source) = app.post_op(
        "get_recipe",
        Some(&agent_key),
        &json!({ "branch_id": english }).to_string(),
    );
    let kitchen_hand = source["result"]["hand_id"].as_str().unwrap().to_string();

    let mut input = french_mousse();
    input["branch_id"] = json!(english);
    input["language"] = json!("fr");
    let (status, translated) =
        app.post_op("start_translation", Some(&agent_key), &input.to_string());
    assert_eq!(status, 200, "{translated}");

    // The Hand on the Branch is the Person's Kitchen's — the Translation lands
    // on his own shelf, not somewhere an agent owns.
    assert_eq!(translated["result"]["hand_id"], json!(kitchen_hand));
    assert_eq!(translated["result"]["kitchen_id"], json!(kitchen));
    // And the Hand on the Version is the Person himself.
    assert_eq!(
        translated["result"]["versions"][0]["hand_id"],
        json!(person),
        "the author is the Person; the agent only held the pen"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_shelf_shows_one_card_per_lineage_in_the_readers_language_and_marks_a_fallback() {
    let app = support::spawn_app();
    let (_person, key, kitchen) = person_with_kitchen(&app, "Aurélien");
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "fr", "reading_measures": "metric" }).to_string(),
    );

    // One recipe with a French Translation, and one with none.
    let english = shelve_recipe(&app, &key, &kitchen, english_mousse());
    let mut input = french_mousse();
    input["branch_id"] = json!(english);
    input["language"] = json!("fr");
    let (_, translated) = app.post_op("start_translation", Some(&key), &input.to_string());
    let french = translated["result"]["branch_id"]
        .as_str()
        .unwrap()
        .to_string();
    let english_only = shelve_recipe(
        &app,
        &key,
        &kitchen,
        json!({
            "title": "Yogurt Flatbread",
            "steps": [
                { "kind": "step", "text": "Stir the yoghurt into the flour until it comes together." },
                { "kind": "step", "text": "Rest the dough for half an hour under a cloth." },
            ],
        }),
    );

    let entries = shelf(&app, &key, json!({}));
    assert_eq!(entries.len(), 2, "one card per Lineage: {entries:#?}");

    let mousse = entries
        .iter()
        .find(|entry| entry["branch_id"] == json!(french))
        .expect("the card opens the Branch written in the reader's own Language");
    assert_eq!(mousse["title"], json!("Mousse au chocolat"));
    assert_eq!(mousse["language_fallback"], json!(false));

    // No French rendering exists, so the recipe is shown anyway and marked —
    // a preference never hides a recipe from its owner.
    let flatbread = entries
        .iter()
        .find(|entry| entry["branch_id"] == json!(english_only))
        .expect("shown, not hidden");
    assert_eq!(flatbread["language"], json!("en"));
    assert_eq!(flatbread["language_fallback"], json!(true));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_version_a_language_change_makes_leaves_the_chain_walkable() {
    // A Language change appends the head content again under a new sequence.
    // That is the same Version occurring twice on one Branch — allowed, and
    // told apart by `sequence` — but `branch_point` walks parent chains and
    // insists they are contiguous, so this is the shape most likely to have
    // been broken by making the change a Version at all.
    let app = support::spawn_app();
    let (_person, key, kitchen) = person_with_kitchen(&app, "Aurélien");
    let english = shelve_recipe(&app, &key, &kitchen, english_mousse());

    let (_, second) = app.post_op(
        "create_kitchen",
        Some(&key),
        &json!({ "name": "The Other Kitchen" }).to_string(),
    );
    let second_kitchen = second["result"]["id"].as_str().unwrap().to_string();

    // Saved on behalf of a Kitchen that does not hold this Branch: a Copy, and
    // so two Branches of one Lineage to find a Branch Point between.
    let mut copy = english_mousse();
    copy["branch_id"] = json!(english);
    copy["kitchen_id"] = json!(second_kitchen);
    copy["title"] = json!("Chocolate Mousse, the other way");
    let (status, copied) = app.post_op("save_recipe_version", Some(&key), &copy.to_string());
    assert_eq!(status, 200, "{copied}");
    assert_eq!(copied["result"]["copied"], json!(true));
    let theirs = copied["result"]["branch_id"].as_str().unwrap().to_string();

    let (status, before) = app.post_op(
        "branch_point",
        Some(&key),
        &json!({ "branch_a_id": english, "branch_b_id": theirs }).to_string(),
    );
    assert_eq!(status, 200, "{before}");
    let shared = before["result"]["version_id"].as_str().unwrap().to_string();

    // Now say the original is French. The chain grows a repeated Version.
    let (status, set) = app.post_op(
        "set_recipe_language",
        Some(&key),
        &json!({ "branch_id": english, "language": "fr" }).to_string(),
    );
    assert_eq!(status, 200, "{set}");

    let (status, after) = app.post_op(
        "branch_point",
        Some(&key),
        &json!({ "branch_a_id": english, "branch_b_id": theirs }).to_string(),
    );
    assert_eq!(status, 200, "{after}");
    assert_eq!(
        after["result"]["version_id"],
        json!(shared),
        "where they diverged is a fact about the words, and no word changed"
    );

    let (status, divergence) = app.post_op(
        "divergence",
        Some(&key),
        &json!({ "branch_id": theirs, "other_branch_id": english }).to_string(),
    );
    assert_eq!(status, 200, "{divergence}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_rapid_re_save_never_collapses_away_the_version_a_translation_renders() {
    // Found by live acceptance, not by these tests, because every other test
    // here backdates the head to step outside the collapse window. A collapse
    // *replaces* the Version at the head sequence; do that to a Version some
    // Translation says it renders and the pointer is left naming text that no
    // longer occurs anywhere.
    let app = support::spawn_app();
    let (_person, key, kitchen) = person_with_kitchen(&app, "Aurélien");
    let english = shelve_recipe(&app, &key, &kitchen, english_mousse());

    let mut input = french_mousse();
    input["branch_id"] = json!(english);
    input["language"] = json!("fr");
    let (_, translated) = app.post_op("start_translation", Some(&key), &input.to_string());
    let french = translated["result"]["branch_id"]
        .as_str()
        .unwrap()
        .to_string();
    let rendered = translated["result"]["translation"]["translates_version_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Immediately — same Hand, well inside the collapse window.
    let mut edit = english_mousse();
    edit["branch_id"] = json!(english);
    edit["steps"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "kind": "step", "text": "Rest the mousse overnight in the fridge." }));
    let (status, saved) = app.post_op("save_recipe_version", Some(&key), &edit.to_string());
    assert_eq!(status, 200, "{saved}");
    assert_eq!(
        saved["result"]["collapsed"],
        json!(false),
        "the collapse window closes once somebody translates you"
    );
    assert_eq!(saved["result"]["sequence"], json!(2));

    // The Translation still knows what it renders, and is honestly one behind
    // rather than unable to say.
    let (_, read) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": french }).to_string(),
    );
    assert_eq!(
        read["result"]["translation"]["translates_version_id"],
        json!(rendered)
    );
    assert_eq!(
        read["result"]["translation"]["source_branch_id"],
        json!(english),
        "the Version it renders is still on the source Branch"
    );
    assert_eq!(read["result"]["translation"]["versions_behind"], json!(1));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn saying_what_language_a_recipe_is_in_does_not_make_its_translations_stale() {
    // Changing a Language appends the head content again, unchanged. That is a
    // second occurrence of one Version id on one Branch — so staleness has to
    // be counted from the newest occurrence, or a relabel would put every
    // Translation permanently and unrecoverably one Version behind.
    let app = support::spawn_app();
    let (_person, key, kitchen) = person_with_kitchen(&app, "Aurélien");
    let mut mislabelled = english_mousse();
    mislabelled["language"] = json!("es");
    let english = shelve_recipe(&app, &key, &kitchen, mislabelled);

    let mut input = french_mousse();
    input["branch_id"] = json!(english);
    input["language"] = json!("fr");
    let (_, translated) = app.post_op("start_translation", Some(&key), &input.to_string());
    let french = translated["result"]["branch_id"]
        .as_str()
        .unwrap()
        .to_string();

    let behind = |branch: &str| -> Value {
        let (_, read) = app.post_op(
            "get_recipe",
            Some(&key),
            &json!({ "branch_id": branch }).to_string(),
        );
        read["result"]["translation"]["versions_behind"].clone()
    };
    assert_eq!(behind(&french), json!(0));

    let (status, set) = app.post_op(
        "set_recipe_language",
        Some(&key),
        &json!({ "branch_id": english, "language": "en" }).to_string(),
    );
    assert_eq!(status, 200, "{set}");
    assert_eq!(
        behind(&french),
        json!(0),
        "no word of the recipe changed, so the Translation has not fallen behind anything"
    );

    // And a real edit after the relabel is still counted.
    backdate_branch_head(&app, &english);
    let mut edit = english_mousse();
    edit["branch_id"] = json!(english);
    edit["steps"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "kind": "step", "text": "Rest the mousse overnight in the fridge." }));
    let (status, saved) = app.post_op("save_recipe_version", Some(&key), &edit.to_string());
    assert_eq!(status, 200, "{saved}");
    assert_eq!(behind(&french), json!(1));
}

/// The bug the live pass caught: scoping the cooking record by *which Versions
/// a Branch carries right now* loses cookings the moment somebody edits the
/// recipe, because a collapsing save repoints `branch_versions` at a fresh
/// Version and the Attempt is still pinned to the one it replaced.
///
/// A recipe that forgets it was cooked because its picture changed is exactly
/// the systematic wrongness ADR 0010 refuses, so the boundary is the household
/// — everyone sharing a Kitchen that holds this recipe — and membership does
/// not move when a Version does.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_collapsing_edit_never_makes_a_recipe_forget_it_was_cooked() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");

    cook_it(&app, &key, &branch_id, json!({ "rating": "again" }));
    cook_it(&app, &key, &branch_id, json!({}));

    let cooked_before = {
        let (_, read) = app.post_op(
            "get_recipe",
            Some(&key),
            &json!({ "branch_id": branch_id }).to_string(),
        );
        read["result"]["cooked"].clone()
    };
    assert_eq!(cooked_before["count"], json!(2));

    // An ordinary edit, inside the collapse window, so it folds into the very
    // Version both cookings are pinned to.
    let (status, saved) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": branch_id,
            "title": "Katsu Curry",
            "ingredients": [{ "kind": "ingredient", "text": "2 escalopes de poulet" }],
            "steps": [{ "kind": "step", "text": "Paner les escalopes" }],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{saved}");
    assert_eq!(
        saved["result"]["collapsed"],
        json!(true),
        "this test needs the collapse it is about: {saved}"
    );

    // The state this test exists for, asserted rather than assumed: the
    // Version both cookings are pinned to is no longer carried by the Branch
    // at all. Any implementation that reaches Attempts through
    // `branch_versions` now finds none — which is precisely the failure.
    let orphaned: i64 = app
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM attempts \
                  WHERE attempts.lineage_id = (SELECT lineage_id FROM branches WHERE id = ?1) \
                    AND attempts.version_id NOT IN \
                        (SELECT version_id FROM branch_versions WHERE branch_id = ?1)",
                rusqlite::params![branch_id],
                |r| r.get(0),
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .unwrap();
    assert_eq!(
        orphaned, 2,
        "the collapse must have orphaned both cookings, or this test proves nothing"
    );

    let (_, after) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(
        after["result"]["cooked"], cooked_before,
        "editing a recipe must not change how many times it has been cooked"
    );
}

/// The Share Link guarantee, held at the Catalogue rather than at a page that
/// does not exist yet (#65 builds it).
///
/// A share renders a **Version** (ADR 0013, ADR 0026). So the property that
/// keeps a private cooking note off a page a stranger can open is that what a
/// Version *is* — `recipe_content_schema`, the whole of what a fingerprint
/// names — carries no rating, no note of anybody's cooking and no Attempt
/// Photograph, and that no path leads from a Version back to an Attempt.
///
/// Asserting it here means #65 inherits the guarantee instead of re-deciding
/// it, and a future field added to a Version that would carry cooking data
/// fails this test rather than a review.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_version_carries_nothing_of_a_cooking() {
    let catalogue = kamosu::catalogue::declarations();
    let recipe = catalogue
        .as_array()
        .expect("the Catalogue is a list of declarations")
        .iter()
        .find(|op| op["name"] == json!("get_recipe"))
        .expect("get_recipe is declared");

    let version_content = &recipe["output_schema"]["properties"]["versions"]["items"]["properties"]
        ["content"]["properties"];
    let fields: Vec<&String> = version_content.as_object().unwrap().keys().collect();
    for forbidden in ["rating", "cooked", "attempt", "attempts", "photographs"] {
        assert!(
            !fields.iter().any(|field| field.as_str() == forbidden),
            "a Version declares '{forbidden}': a share would carry a private cooking. \
             Fields: {fields:?}"
        );
    }

    // And the cooking record sits beside the Versions rather than inside one,
    // which is what makes the sentence above true by construction.
    assert!(
        recipe["output_schema"]["properties"]["cooked"].is_object(),
        "the cooking record belongs to the recipe read, not to a Version"
    );
}

/// Promoting is an ordinary save, so it inherits the collapse window — and a
/// save that names nothing writes a Version's name away. Promoting a picture
/// into a Version somebody had named must not quietly un-name it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn promoting_a_picture_keeps_the_version_its_name() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");

    let (_, named) = app.post_op(
        "rename_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "sequence": 1, "name": "Sunday version" }).to_string(),
    );
    assert_eq!(named["result"]["name"], json!("Sunday version"));

    let picture = upload_a_picture(&app, &key, 123);
    let attempt_id = cook_it(&app, &key, &branch_id, json!({ "photographs": [picture] }));

    let (status, promoted) = app.post_op(
        "promote_attempt_photograph",
        Some(&key),
        &json!({
            "attempt_id": attempt_id,
            "photograph_id": picture,
            "branch_id": branch_id,
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{promoted}");
    assert_eq!(
        promoted["result"]["collapsed"],
        json!(true),
        "this test is about what a collapse does: {promoted}"
    );
    assert_eq!(
        promoted["result"]["copied"],
        json!(false),
        "promoting inside your own Kitchen is an edit, never a Copy"
    );
    assert_eq!(
        promoted["result"]["branch_id"],
        json!(branch_id),
        "an edit stays on the Branch it was asked for"
    );

    let (_, read) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let head = read["result"]["versions"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    assert_eq!(
        head["name"],
        json!("Sunday version"),
        "promoting a picture must not take a Version's name away"
    );
    assert_eq!(head["content"]["main_photo"], json!(picture));
}

// --- Units and conversion (issue #49, ADR 0016) ------------------------------

/// A recipe with one Ingredient Line per case, and a Reading laid over each —
/// the shortest route to a real Version with real Readings on a real database.
/// `lines` is (written line, amount, unit, target).
fn recipe_with_readings(
    app: &support::TestApp,
    key: &str,
    kitchen_id: &str,
    title: &str,
    lines: &[(&str, &str, &str, &str)],
) -> String {
    let ingredients: Vec<Value> = lines
        .iter()
        .map(|(text, _, _, _)| json!({ "kind": "ingredient", "text": text }))
        .collect();
    let (_, created) = app.post_op(
        "create_recipe",
        Some(key),
        &json!({ "kitchen_id": kitchen_id, "title": title, "ingredients": ingredients })
            .to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();
    for (index, (_, amount, unit, target)) in lines.iter().enumerate() {
        let (status, read) = app.post_op(
            "set_reading",
            Some(key),
            &json!({
                "branch_id": branch_id,
                "line_index": index,
                "amount": amount,
                "unit": unit,
                "target": target,
            })
            .to_string(),
        );
        assert_eq!(status, 200, "{read}");
    }
    branch_id
}

/// The one subordinate line under each Ingredient Line of a Branch's head
/// Version, as this Credential's Person reads it.
fn measured_ingredients(app: &support::TestApp, key: &str, branch_id: &str) -> Value {
    let (status, read) = app.post_op(
        "get_recipe",
        Some(key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 200, "{read}");
    read["result"]["versions"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()["measured"]["ingredients"]
        .clone()
}

/// Set how a Person measures. The Reading Language rides along because the two
/// are one preference at the Operation.
fn reads_in(app: &support::TestApp, key: &str, language: &str, measures: &str) {
    let (status, set) = app.post_op(
        "set_reading_preferences",
        Some(key),
        &json!({ "reading_language": language, "reading_measures": measures }).to_string(),
    );
    assert_eq!(status, 200, "{set}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn any_written_word_is_a_unit_and_only_the_closed_set_ever_converts() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    reads_in(&app, &key, "en", "metric");

    let branch_id = recipe_with_readings(
        &app,
        &key,
        &kitchen_id,
        "Tofu",
        &[
            // Whatever the cook wrote is a Unit. A poignée is as real as a
            // gram and simply never converts — no error, no mark, no nag.
            ("2 poignées de farine", "2", "poignée", "farine"),
            ("2 tbsp soy sauce", "2", "tbsp", "soy sauce"),
        ],
    );

    assert_eq!(
        measured_ingredients(&app, &key, &branch_id),
        json!([null, "about 30 ml"]),
        "an unknown Unit offers nothing and errors at nothing; a known one converts"
    );

    // `g`, `gr`, `gramme` and `grams` are one Unit, so spelling is not a
    // data-entry problem. Every one of them reads the same to an American.
    reads_in(&app, &key, "en", "us");
    for spelling in [
        "g", "gr", "gm", "gram", "grams", "gramme", "grammes", "gramos",
    ] {
        let branch_id = recipe_with_readings(
            &app,
            &key,
            &kitchen_id,
            &format!("Flour in {spelling}"),
            &[("250 g flour", "250", spelling, "flour")],
        );
        assert_eq!(
            measured_ingredients(&app, &key, &branch_id),
            json!(["about 8¾ oz"]),
            "'{spelling}' is the same Unit as every other spelling of the gram"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_convertible_set_knows_three_languages_and_the_named_regional_spoons() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    reads_in(&app, &key, "en", "metric");

    // The same spoon in three Languages is one Unit — and ADR 0016's own
    // worked example of the cost it accepts: three tablespoons of soy sauce
    // read as 45 ml where an Australian author meant 60.
    for spelling in [
        "tbsp",
        "tablespoons",
        "cuillères à soupe",
        "cuilleres a soupe",
        "c. à s.",
        "cucharadas",
        "cda",
    ] {
        let branch_id = recipe_with_readings(
            &app,
            &key,
            &kitchen_id,
            &format!("Soy in {spelling}"),
            &[("3 tbsp soy sauce", "3", spelling, "soy sauce")],
        );
        assert_eq!(
            measured_ingredients(&app, &key, &branch_id),
            json!(["about 45 ml"]),
            "'{spelling}' is the tablespoon"
        );
    }

    // The regional variants are ordinary members of the set, so a recipe that
    // names one gets it right without anybody being asked anything.
    let named = recipe_with_readings(
        &app,
        &key,
        &kitchen_id,
        "Soy, named precisely",
        &[
            (
                "3 Australian tablespoons soy sauce",
                "3",
                "australian tablespoon",
                "soy sauce",
            ),
            (
                "3 imperial tablespoons soy sauce",
                "3",
                "imperial tablespoon",
                "soy sauce",
            ),
        ],
    );
    assert_eq!(
        measured_ingredients(&app, &key, &named),
        json!(["about 60 ml", "about 55 ml"]),
        "an author who named the spoon gets the spoon they named"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rounding_happens_last_and_once_and_a_cup_of_a_staple_becomes_a_weight() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    reads_in(&app, &key, "en", "metric");

    let branch_id = recipe_with_readings(
        &app,
        &key,
        &kitchen_id,
        "Three cups of things",
        &[
            // ADR 0016's own arithmetic: three cups is 710 ml, not 720. It is
            // only 720 if the cup was rounded to 240 first — which is exactly
            // the double-rounding "round last, once" forbids.
            ("3 cups sliced mushrooms", "3", "cups", "sliced mushrooms"),
            // Cross-family, off the shipped Cup Weight: 125 g a cup.
            ("3 cups flour", "3", "cups", "flour"),
            // ADR 0016 states this one outright: 2 tbsp butter is about 28 g
            // from the single figure 227 — one Cup Weight answering a spoon.
            ("2 tbsp butter", "2", "tbsp", "butter"),
        ],
    );

    assert_eq!(
        measured_ingredients(&app, &key, &branch_id),
        json!(["about 710 ml", "about 375 g", "about 28 g"]),
    );

    // The written lines are untouched by every one of those conversions.
    let (_, read) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let lines: Vec<&str> = read["result"]["versions"][0]["content"]["ingredients"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| line["text"].as_str().unwrap())
        .collect();
    assert_eq!(
        lines,
        vec!["3 cups sliced mushrooms", "3 cups flour", "2 tbsp butter"],
        "a conversion is never a rewrite (ADR 0002)"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_blank_cup_weight_offers_millilitres_and_an_override_beats_the_shipped_figure() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    reads_in(&app, &key, "en", "metric");

    let branch_id = recipe_with_readings(
        &app,
        &key,
        &kitchen_id,
        "Mushrooms and flour",
        &[
            ("1 cup sliced mushrooms", "1", "cup", "sliced mushrooms"),
            ("1 cup flour", "1", "cup", "flour"),
        ],
    );

    // A Food nobody has weighed is not a gap to close. It is a line that
    // offers millilitres and says nothing at all about grams.
    assert_eq!(
        measured_ingredients(&app, &key, &branch_id),
        json!(["about 240 ml", "about 125 g"]),
    );

    let (_, foods) = app.post_op("list_foods", Some(&key), "{}");
    let id_of = |word: &str| -> String {
        foods["result"]["foods"]
            .as_array()
            .unwrap()
            .iter()
            .find(|food| food["name"] == json!(word))
            .unwrap_or_else(|| panic!("no Food named {word} in {foods}"))["id"]
            .as_str()
            .unwrap()
            .to_string()
    };

    // Anyone may set one, and an override always beats the shipped figure.
    for (word, grams) in [("sliced mushrooms", 70.0), ("flour", 150.0)] {
        let (status, set) = app.post_op(
            "set_food_cup_weight",
            Some(&key),
            &json!({ "food_id": id_of(word), "cup_weight_grams": grams }).to_string(),
        );
        assert_eq!(status, 200, "{set}");
    }
    assert_eq!(
        measured_ingredients(&app, &key, &branch_id),
        json!(["about 70 g", "about 150 g"]),
        "a figure somebody set wins over the one Kamosu ships"
    );

    // Clearing the override takes flour back to the shipped staple figure and
    // the mushrooms back to millilitres — nobody is ever asked to fill in a
    // blank, and the failure mode is silence.
    for word in ["sliced mushrooms", "flour"] {
        app.post_op(
            "set_food_cup_weight",
            Some(&key),
            &json!({ "food_id": id_of(word), "cup_weight_grams": null }).to_string(),
        );
    }
    assert_eq!(
        measured_ingredients(&app, &key, &branch_id),
        json!(["about 240 ml", "about 125 g"]),
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fractions_are_used_where_the_measure_is_fractional_and_metric_stays_whole() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    // An American reads a metric recipe. The cups in her drawer are marked in
    // fractions, so that is how the line is written.
    let branch_id = recipe_with_readings(
        &app,
        &key,
        &kitchen_id,
        "A metric recipe read in America",
        &[
            ("500 ml lait", "500", "ml", "lait"),
            ("1 l bouillon", "1", "l", "bouillon"),
            ("240 ml crème", "240", "ml", "crème"),
            ("250 g farine", "250", "g", "farine"),
        ],
    );
    assert_eq!(
        measured_ingredients(&app, &key, &branch_id),
        json!([
            "about 2⅛ cups",
            "about 4¼ cups",
            // Singular, because one cup is one cup.
            "about 1 cup",
            "about 8¾ oz",
        ]),
    );

    // The same recipe to a metric reader says nothing at all: it is already in
    // her measures at the Yield as written, so a line would only repeat what
    // is above it.
    reads_in(&app, &key, "fr", "metric");
    assert_eq!(
        measured_ingredients(&app, &key, &branch_id),
        json!([null, null, null, null]),
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_converted_amount_says_about_in_the_readers_own_language() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    let branch_id = recipe_with_readings(
        &app,
        &key,
        &kitchen_id,
        "One cup of flour",
        &[("2 cups flour", "2", "cups", "flour")],
    );

    // "About" is a fact about the number, not an apology for the food: Kamosu
    // rounded, so it is true every time and is therefore said every time.
    for (language, expected) in [
        ("en", "about 250 g"),
        ("fr", "environ 250 g"),
        ("es", "aprox. 250 g"),
    ] {
        reads_in(&app, &key, language, "metric");
        assert_eq!(
            measured_ingredients(&app, &key, &branch_id),
            json!([expected]),
            "the line is read in the reader's own Reading Language"
        );
    }

    // As written asks for no conversion at all, and gets none.
    reads_in(&app, &key, "en", "as_written");
    assert_eq!(measured_ingredients(&app, &key, &branch_id), json!([null]));

    // Correcting a Reading answers with the line it now produces, because the
    // moment somebody tells Kamosu it misread a line is the moment they want
    // to see what the corrected line says.
    reads_in(&app, &key, "en", "metric");
    let (status, corrected) = app.post_op(
        "set_reading",
        Some(&key),
        &json!({
            "branch_id": branch_id,
            "line_index": 0,
            "amount": "3",
            "unit": "cups",
            "target": "flour",
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{corrected}");
    assert_eq!(corrected["result"]["measured"], json!("about 375 g"));

    // Clearing it takes the line beneath away with it.
    let (_, cleared) = app.post_op(
        "set_reading",
        Some(&key),
        &json!({ "branch_id": branch_id, "line_index": 0 }).to_string(),
    );
    assert_eq!(cleared["result"]["measured"], json!(null));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn temperatures_convert_on_the_conventional_oven_ladder_never_arithmetically() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    reads_in(&app, &key, "en", "metric");

    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "An oven and some distractions",
            "steps": [
                // 350°F is 176.67°C by arithmetic and 180°C on the dial.
                { "kind": "step", "text": "Preheat the oven to 350°F." },
                // 400°F would be 205°C by arithmetic — a number no oven has.
                { "kind": "step", "text": "Raise it to 400 degrees F." },
                // 20 of the 39 real steps carrying a temperature print both.
                { "kind": "step", "text": "Preheat oven to 180C/350F" },
                { "kind": "step", "text": "350°F (175°C), fan off." },
                // The real corpus also contains these, and neither is a
                // temperature.
                { "kind": "step", "text": "Turn the tray 90 degrees." },
                { "kind": "step", "text": "Cook on gas 5 for 20 minutes." },
                { "kind": "section", "text": "To finish" },
            ],
        })
        .to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    let (_, read) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(
        read["result"]["versions"][0]["measured"]["steps"],
        json!(["about 180 °C", "about 200 °C", null, null, null, null, null,]),
        "the ladder, not the arithmetic — and nothing where the step said both"
    );

    // The step's own text is untouched: the conversion is an addition beside
    // the sentence and is never written into it.
    assert_eq!(
        read["result"]["versions"][0]["content"]["steps"][0]["text"],
        json!("Preheat the oven to 350°F."),
    );

    // The ladder runs both ways: an American reading a French recipe.
    reads_in(&app, &key, "en", "us");
    let (_, celsius) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Un four français",
            "steps": [
                { "kind": "step", "text": "Préchauffer le four à 180 °C." },
                { "kind": "step", "text": "Ajouter 1 c. à s. d'huile, puis enfourner à 200°C." },
            ],
        })
        .to_string(),
    );
    assert_eq!(
        celsius["result"]["versions"][0]["measured"]["steps"],
        json!(["about 350 °F", "about 400 °F"]),
        "a spoonful written `1 c.` is not a one-degree oven"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_recipe_already_in_your_measures_is_untouched_within_its_own_system() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    // Kamosu converts BETWEEN systems and never re-expresses within one. An
    // American told her `4 tsp` is `about 1⅓ tbsp` has been handed a second way
    // to say a thing she already owns the spoons for, and ADR 0016 wants that
    // line absent: "a recipe already in your measures at its written Yield is
    // untouched".
    let branch_id = recipe_with_readings(
        &app,
        &key,
        &kitchen_id,
        "Already American",
        &[
            ("4 tsp baking powder", "4", "tsp", "baking powder"),
            ("8 tbsp butter", "8", "tbsp", "butter"),
            ("24 oz flour", "24", "oz", "flour"),
        ],
    );
    assert_eq!(
        measured_ingredients(&app, &key, &branch_id),
        json!([null, null, null]),
    );

    // The same rule the other way round: a metric cook is not told that her
    // 1500 g is 1.5 kg, and her 250 ml is not restated in centilitres.
    reads_in(&app, &key, "fr", "metric");
    let metric = recipe_with_readings(
        &app,
        &key,
        &kitchen_id,
        "Déjà métrique",
        &[
            ("1500 g de farine", "1500", "g", "farine"),
            ("250 ml de lait", "250", "ml", "lait"),
            ("20 cl de crème", "20", "cl", "crème"),
        ],
    );
    assert_eq!(
        measured_ingredients(&app, &key, &metric),
        json!([null, null, null]),
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_metric_dial_setting_off_the_american_ladder_still_answers() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    reads_in(&app, &key, "en", "us");

    // 170 °C and 210 °C are ordinary settings on an oven sold in France and
    // appear nowhere in the American ladder. Reading that ladder backwards
    // leaves a hole exactly where this library lives, so the metric direction
    // has a table of its own — and every answer is still a real dial position
    // rather than the 338 °F or 410 °F the arithmetic would hand back.
    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Un four français",
            "steps": [
                { "kind": "step", "text": "Préchauffer le four à 170 °C." },
                { "kind": "step", "text": "Monter à 210 °C pour finir." },
                { "kind": "step", "text": "Sécher les meringues à 90 °C." },
                // Off the ladder entirely: silence beats a number nobody can set.
                { "kind": "step", "text": "Un four à pizza monte à 450 °C." },
            ],
        })
        .to_string(),
    );
    assert_eq!(
        created["result"]["versions"][0]["measured"]["steps"],
        json!(["about 350 °F", "about 400 °F", "about 200 °F", null]),
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_one_line_scales_to_the_yield_being_cooked_and_still_never_two() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    reads_in(&app, &key, "en", "metric");

    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Shortbread",
            "yield": { "amount": "4", "noun": "servings" },
            "ingredients": [
                { "kind": "ingredient", "text": "2 cups flour" },
                { "kind": "ingredient", "text": "250 g butter" },
                { "kind": "ingredient", "text": "2 poignées de sucre" },
                { "kind": "ingredient", "text": "Some cooking oil (for deep frying)" },
            ],
        })
        .to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();
    for (index, amount, unit, target) in [
        (0, "2", "cups", "flour"),
        (1, "250", "g", "butter"),
        (2, "2", "poignée", "sucre"),
    ] {
        app.post_op(
            "set_reading",
            Some(&key),
            &json!({
                "branch_id": branch_id, "line_index": index,
                "amount": amount, "unit": unit, "target": target,
            })
            .to_string(),
        );
    }
    // And the fourth is left with no Reading at all — #44's case, and the one
    // ADR 0002 says any code path assuming a quantity exists will get wrong.
    app.post_op(
        "set_reading",
        Some(&key),
        &json!({ "branch_id": branch_id, "line_index": 3 }).to_string(),
    );

    // Read, not cooked: the Yield is the one written, so only the cups convert.
    assert_eq!(
        measured_ingredients(&app, &key, &branch_id),
        json!(["about 250 g", null, null, null]),
    );

    // Now she is cooking it, for eight instead of four.
    let (_, attempt) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let (status, advanced) = app.post_op(
        "advance_attempt",
        Some(&key),
        &json!({
            "attempt_id": attempt["result"]["id"],
            "cooking_yield": { "amount": "8", "noun": "servings" },
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{advanced}");

    assert_eq!(
        measured_ingredients(&app, &key, &branch_id),
        json!([
            // Scaled AND converted, in one line: 4 cups of flour at 125 g.
            "about 500 g",
            // Scaled but not converted — she already measures in grams. Same
            // one slot, and nothing on screen says which of the two happened.
            "about 500 g",
            // An unrecognised Unit still scales: the quantity multiplies and
            // the cook's own word is untouched (ADR 0016).
            "about 4 poignée",
            // **And a line with no Reading scales unchanged** (#44): there is
            // no number here to double, so the line is simply the line.
            null,
        ]),
    );

    // Scaling is a fact about this cooking, not about the recipe: it makes no
    // Version, and it is invisible to anybody else in the Kitchen.
    let (_, read) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(read["result"]["versions"].as_array().unwrap().len(), 1);
    assert_eq!(
        read["result"]["versions"][0]["content"]["ingredients"][0]["text"],
        json!("2 cups flour"),
        "the written line is never rewritten by scaling either",
    );

    let cook = app.core.create_person("Camille").expect("person");
    let cook_key = app
        .core
        .mint_access_key(&cook, "browser", false)
        .unwrap()
        .secret;
    let (_, invite) = app.post_op(
        "invite_to_kitchen",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id }).to_string(),
    );
    app.post_op(
        "accept_kitchen_invite",
        Some(&cook_key),
        &json!({ "secret": invite["result"]["secret"].as_str().unwrap() }).to_string(),
    );
    reads_in(&app, &cook_key, "en", "metric");
    assert_eq!(
        measured_ingredients(&app, &cook_key, &branch_id),
        json!(["about 250 g", null, null, null]),
        "somebody else's cooking never scales your reading of the recipe",
    );

    // Finishing puts it back: the Yield being cooked was true while it was
    // being cooked and is not a deviation the recipe keeps.
    app.post_op(
        "finish_attempt",
        Some(&key),
        &json!({ "attempt_id": attempt["result"]["id"] }).to_string(),
    );
    assert_eq!(
        measured_ingredients(&app, &key, &branch_id),
        json!(["about 250 g", null, null, null]),
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reading_measures_live_on_the_account_and_default_to_american() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    // The default is American, stated rather than guessed at from anything
    // about the person asking.
    let (status, preferences) = app.post_op("get_reading_preferences", Some(&key), "{}");
    assert_eq!(status, 200, "{preferences}");
    assert_eq!(
        preferences["result"],
        json!({ "reading_language": "en", "reading_measures": "us" }),
    );

    reads_in(&app, &key, "fr", "metric");
    let (_, changed) = app.post_op("get_reading_preferences", Some(&key), "{}");
    assert_eq!(
        changed["result"],
        json!({ "reading_language": "fr", "reading_measures": "metric" }),
    );

    // Two people standing in one Kitchen read one recipe differently, because
    // measures are a fact about a person and never about a recipe. Setting one
    // stores nothing on the recipe and makes no Version.
    let branch_id = recipe_with_readings(
        &app,
        &key,
        &kitchen_id,
        "One recipe, two readers",
        &[("2 cups flour", "2", "cups", "flour")],
    );
    let versions_before = app
        .post_op(
            "get_recipe",
            Some(&key),
            &json!({ "branch_id": branch_id }).to_string(),
        )
        .1["result"]["versions"]
        .as_array()
        .unwrap()
        .len();

    let cook = app.core.create_person("Camille").expect("person");
    let cook_key = app
        .core
        .mint_access_key(&cook, "browser", false)
        .unwrap()
        .secret;
    let (_, invite) = app.post_op(
        "invite_to_kitchen",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id }).to_string(),
    );
    app.post_op(
        "accept_kitchen_invite",
        Some(&cook_key),
        &json!({ "secret": invite["result"]["secret"].as_str().unwrap() }).to_string(),
    );

    assert_eq!(
        measured_ingredients(&app, &key, &branch_id),
        json!(["environ 250 g"]),
        "the metric reader is offered grams",
    );
    assert_eq!(
        measured_ingredients(&app, &cook_key, &branch_id),
        json!([null]),
        "the American, who wrote it in cups, is offered nothing",
    );
    assert_eq!(
        app.post_op(
            "get_recipe",
            Some(&key),
            &json!({ "branch_id": branch_id }).to_string(),
        )
        .1["result"]["versions"]
            .as_array()
            .unwrap()
            .len(),
        versions_before,
        "reading a recipe in another system makes no Version",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_converted_line_reaches_an_agent_through_the_mcp_door_too() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    reads_in(&app, &key, "en", "metric");
    let branch_id = recipe_with_readings(
        &app,
        &key,
        &kitchen_id,
        "How much flour in grams",
        &[("2 cups flour", "2", "cups", "flour")],
    );

    // The conversion is computed in the Core, beneath both Doors (ADR 0001),
    // so an agent asked *how much flour in grams* answers correctly for free.
    let (status, answered) = app.post_mcp(
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "get_recipe",
                "arguments": { "branch_id": branch_id },
            },
        })
        .to_string(),
        Some(&key),
    );
    assert_eq!(status, 200, "{answered}");
    let structured = &answered["result"]["structuredContent"];
    assert_eq!(
        structured["versions"][0]["measured"]["ingredients"],
        json!(["about 250 g"]),
        "the MCP door grew the same answer the web door did: {answered}"
    );
}

// --- The cooking diary (issue #60) -------------------------------------------
//
// The Cooked tab: your Attempts sorted by date rather than by recipe, so *what
// did I cook that week* has an answer. `list_attempts` is the one Operation
// behind it, and everything below is asked through a real Door.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_diary_lists_the_callers_own_cookings_newest_first_naming_the_recipe_of_each() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let soup = recipe_in(&app, &key, &kitchen_id, "Miso Soup");
    let curry = recipe_in(&app, &key, &kitchen_id, "Katsu Curry");

    let long_ago = cook_it(&app, &key, &soup, json!({ "rating": "again" }));
    backdate_attempt_created(&app, &long_ago, 9);
    let yesterday = cook_it(
        &app,
        &key,
        &curry,
        json!({ "rating": "tweak", "note": "Trop de sel." }),
    );
    backdate_attempt_created(&app, &yesterday, 1);
    let today = cook_it(&app, &key, &soup, json!({}));

    let (status, listed) = app.post_op("list_attempts", Some(&key), "{}");
    assert_eq!(status, 200, "{listed}");
    let entries = listed["result"]["attempts"].as_array().expect("attempts");
    assert_eq!(entries.len(), 3, "{listed}");

    assert_eq!(
        entries
            .iter()
            .map(|entry| entry["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![today.as_str(), yesterday.as_str(), long_ago.as_str()],
        "a diary is read newest first — this week before last: {listed}"
    );

    // Each entry names the recipe it was cooked from, and opens it.
    assert_eq!(entries[0]["recipe"]["title"], json!("Miso Soup"));
    assert_eq!(entries[0]["recipe"]["branch_id"], json!(soup));
    assert_eq!(entries[1]["recipe"]["title"], json!("Katsu Curry"));
    assert_eq!(entries[1]["recipe"]["branch_id"], json!(curry));

    // And carries the judgement written at the stove: the date, the rating
    // where one was given, the note where one was written.
    assert!(entries[1]["created_at"].is_string());
    assert_eq!(entries[1]["rating"], json!("tweak"));
    assert_eq!(entries[1]["note"], json!("Trop de sel."));
    assert_eq!(entries[0]["rating"], json!(null), "a rating is optional");
    assert_eq!(entries[0]["note"], json!(null), "so is a note");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_diary_holds_cookings_nobody_ever_finished_beside_the_finished_ones() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let soup = recipe_in(&app, &key, &kitchen_id, "Miso Soup");
    let curry = recipe_in(&app, &key, &kitchen_id, "Katsu Curry");
    let bread = recipe_in(&app, &key, &kitchen_id, "Pain");

    let finished = cook_it(&app, &key, &soup, json!({ "rating": "again" }));

    // Still at the stove: In Progress, and still worth offering to resume.
    let (_, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": curry }).to_string(),
    );
    let in_progress = started["result"]["id"].as_str().unwrap().to_string();

    // Walked away from a week ago: never finished, past the resume window —
    // and still a cooking that happened (ADR 0010).
    let (_, abandoned) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": bread }).to_string(),
    );
    let walked_away = abandoned["result"]["id"].as_str().unwrap().to_string();
    backdate_attempt_action(&app, &walked_away, 7);
    backdate_attempt_created(&app, &walked_away, 7);

    let (status, listed) = app.post_op("list_attempts", Some(&key), "{}");
    assert_eq!(status, 200, "{listed}");
    let entries = listed["result"]["attempts"].as_array().expect("attempts");
    assert_eq!(
        entries.len(),
        3,
        "starting is what makes a cooking real, not finishing: {listed}"
    );

    let of = |id: &str| {
        entries
            .iter()
            .find(|entry| entry["id"] == json!(id))
            .unwrap_or_else(|| panic!("{id} is in the diary: {listed}"))
            .clone()
    };

    assert!(of(&finished)["finished_at"].is_string());
    assert_eq!(of(&finished)["resumable"], json!(false));

    // Both of these are In Progress — `finished_at IS NULL` is the whole of
    // that state, and `start_attempt` hands either one straight back.
    assert_eq!(of(&in_progress)["finished_at"], json!(null));
    assert_eq!(of(&walked_away)["finished_at"], json!(null));

    // `resumable` differs between them, and says only whether resuming is
    // still *offered*. The interval decides that and never whether the cooking
    // happened (ADR 0010), which is why the diary marks both the same way.
    assert_eq!(of(&in_progress)["resumable"], json!(true));
    assert_eq!(
        of(&walked_away)["resumable"],
        json!(false),
        "the offer to resume ages out; the cooking it belongs to does not"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_diary_is_the_callers_own_and_never_anybody_elses() {
    let app = support::spawn_app();
    let (aurelien, aurelien_key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let soup = recipe_in(&app, &aurelien_key, &kitchen_id, "Miso Soup");

    let (_marie, marie_key, _marie_kitchen) = person_with_kitchen(&app, "Marie");
    let invite = app
        .core
        .invite_to_kitchen(&aurelien, &kitchen_id)
        .unwrap()
        .1;
    let (status, joined) = app.post_op(
        "accept_kitchen_invite",
        Some(&marie_key),
        &json!({ "secret": invite }).to_string(),
    );
    assert_eq!(status, 200, "{joined}");

    let his = cook_it(&app, &aurelien_key, &soup, json!({ "rating": "again" }));
    let hers = cook_it(&app, &marie_key, &soup, json!({ "rating": "no" }));

    let (_, his_diary) = app.post_op("list_attempts", Some(&aurelien_key), "{}");
    let (_, her_diary) = app.post_op("list_attempts", Some(&marie_key), "{}");

    let ids = |listed: &Value| {
        listed["result"]["attempts"]
            .as_array()
            .expect("attempts")
            .iter()
            .map(|entry| entry["id"].as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        ids(&his_diary),
        vec![his.clone()],
        "a diary is one person's: {his_diary}"
    );
    assert_eq!(
        ids(&her_diary),
        vec![hers],
        "the same recipe, the same Kitchen, and still only her own: {her_diary}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_attempt_against_a_recipe_that_left_the_shelf_keeps_the_name_it_was_known_by() {
    let app = support::spawn_app();
    let (aurelien, aurelien_key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let soup = recipe_in(&app, &aurelien_key, &kitchen_id, "Miso Soup");

    let (marie, marie_key, _marie_kitchen) = person_with_kitchen(&app, "Marie");
    let invite = app
        .core
        .invite_to_kitchen(&aurelien, &kitchen_id)
        .unwrap()
        .1;
    app.core.accept_kitchen_invite(&marie, &invite).unwrap();

    let hers = cook_it(&app, &marie_key, &soup, json!({ "rating": "again" }));

    // Marie leaves Aurélien's Kitchen. Leaving is not a deletion: the recipe
    // stays his, and her Attempts follow her.
    let (status, left) = app.post_op(
        "remove_kitchen_member",
        Some(&marie_key),
        &json!({ "kitchen_id": kitchen_id, "person_id": marie }).to_string(),
    );
    assert_eq!(status, 200, "{left}");

    let (status, listed) = app.post_op("list_attempts", Some(&marie_key), "{}");
    assert_eq!(status, 200, "{listed}");
    let entries = listed["result"]["attempts"].as_array().expect("attempts");
    assert_eq!(entries.len(), 1, "the cooking still happened: {listed}");
    assert_eq!(entries[0]["id"], json!(hers));
    assert_eq!(
        entries[0]["recipe"]["title"],
        json!("Miso Soup"),
        "the name it was known by, remembered: {listed}"
    );
    assert_eq!(
        entries[0]["recipe"]["branch_id"],
        json!(null),
        "text is better than a pointer that opens nothing (#52's rule)"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_diary_follows_a_rename_while_the_recipe_is_still_on_the_shelf() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let soup = recipe_in(&app, &key, &kitchen_id, "Soupe");
    cook_it(&app, &key, &soup, json!({}));

    backdate_branch_head(&app, &soup);
    let (status, saved) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": soup,
            "title": "Soupe miso de Marcella",
            "steps": [{ "kind": "step", "text": "Cuire." }],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{saved}");

    let (_, listed) = app.post_op("list_attempts", Some(&key), "{}");
    assert_eq!(
        listed["result"]["attempts"][0]["recipe"]["title"],
        json!("Soupe miso de Marcella"),
        "a recipe still on the shelf is named as the shelf names it now: {listed}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_attempt_is_corrected_and_put_away_from_the_diary() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let soup = recipe_in(&app, &key, &kitchen_id, "Miso Soup");
    let curry = recipe_in(&app, &key, &kitchen_id, "Katsu Curry");

    let kept = cook_it(&app, &key, &soup, json!({ "rating": "no" }));
    let put_away = cook_it(&app, &key, &curry, json!({}));

    // Freely editable, unlike the recipe it was cooked from (ADR 0005).
    let (status, edited) = app.post_op(
        "edit_attempt",
        Some(&key),
        &json!({ "attempt_id": kept, "rating": "again", "note": "Mieux au dashi." }).to_string(),
    );
    assert_eq!(status, 200, "{edited}");

    let (status, deleted) = app.post_op(
        "delete_attempt",
        Some(&key),
        &json!({ "attempt_id": put_away }).to_string(),
    );
    assert_eq!(status, 200, "{deleted}");

    let (_, listed) = app.post_op("list_attempts", Some(&key), "{}");
    let entries = listed["result"]["attempts"].as_array().expect("attempts");
    assert_eq!(entries.len(), 1, "{listed}");
    assert_eq!(entries[0]["id"], json!(kept));
    assert_eq!(entries[0]["rating"], json!("again"));
    assert_eq!(entries[0]["note"], json!("Mieux au dashi."));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_diary_needs_a_credential_naming_a_person_and_takes_no_input() {
    let app = support::spawn_app();
    let (_person, key, _kitchen) = person_with_kitchen(&app, "Aurélien");

    assert_eq!(app.post_op("list_attempts", None, "{}").0, 401);

    let (status, refused) = app.post_op(
        "list_attempts",
        Some(&key),
        &json!({ "person_id": "p_somebody_else" }).to_string(),
    );
    assert_eq!(
        status, 400,
        "a diary is the caller's own — there is no whose to ask: {refused}"
    );
}

// --- The cooking screen (issue #61, ADR 0011) --------------------------------

/// A Recipe whose Steps really do use some of its Ingredient Lines and not
/// others, with the Readings that join the two already set — the ordinary
/// state of a recipe somebody has read. `(key, branch_id)`.
///
/// It is deliberately not tidy: a Section sits in each list, one line is
/// never named by any Step, one Step names nothing at all, and one line is
/// left unread so the panel has to cope with a Reading that is simply absent.
fn recipe_read_and_ready_to_cook(app: &support::TestApp, cook_name: &str) -> (String, String) {
    let (_person, key, kitchen_id) = person_with_kitchen(app, cook_name);
    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Chicken Katsu Curry",
            "yield": { "amount": "4", "noun": "servings" },
            "ingredients": [
                { "kind": "section", "text": "For the cutlets" },
                { "kind": "ingredient", "text": "2 chicken breasts" },
                { "kind": "ingredient", "text": "1 cup panko" },
                { "kind": "ingredient", "text": "800 ml water" },
                { "kind": "ingredient", "text": "a pinch of salt" },
                // A whole sentence where an ingredient should be — the real
                // corpus keeps several — which Kamosu declines to read rather
                // than mint a Food out of a paragraph.
                { "kind": "ingredient", "text": "I use single cream instead of double cream? Yes it also works very well" },
            ],
            "steps": [
                { "kind": "section", "text": "Assemble" },
                { "kind": "step", "text": "Coat the chicken breast in panko." },
                { "kind": "step", "text": "Pour in the water and simmer for about 7 minutes." },
                { "kind": "step", "text": "Cover and leave it alone." },
            ],
        })
        .to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    for (line_index, amount, unit, target) in [
        (1, "2", Value::Null, json!("chicken")),
        (2, "1", json!("cup"), json!("panko")),
        (3, "800", json!("ml"), json!("water")),
    ] {
        let (status, read) = app.post_op(
            "set_reading",
            Some(&key),
            &json!({
                "branch_id": branch_id,
                "line_index": line_index,
                "amount": amount,
                "unit": unit,
                "target": target,
            })
            .to_string(),
        );
        assert_eq!(status, 200, "{read}");
    }
    // The salt is left unread on purpose: 28% of the real corpus's lines carry
    // no quantity, and such a line still has to be a working line (ADR 0002).
    (key, branch_id)
}

/// The whole of ADR 0011's mechanism: which Ingredients a Step uses is worked
/// out from their Readings on every read, and is stored nowhere.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_uses_the_ingredients_its_readings_name_and_nothing_is_stored() {
    let app = support::spawn_app();
    let (key, branch_id) = recipe_read_and_ready_to_cook(&app, "Aurélien");

    let (status, fetched) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 200, "{fetched}");
    let cooking = &fetched["result"]["versions"][0]["cooking"];

    assert_eq!(
        cooking["steps"][0],
        json!(null),
        "a Section is neither a Step nor somewhere a cook stands"
    );
    assert_eq!(
        cooking["steps"][1]["uses"],
        json!([1, 2]),
        "`chicken breast` names the `2 chicken breasts` line although the Reading \
         says `chicken` and the step says `breast`: a whole word, with a trailing s \
         forgiven"
    );
    assert_eq!(
        cooking["steps"][2]["uses"],
        json!([3]),
        "the water step uses the water and nothing else"
    );
    assert_eq!(
        cooking["steps"][3]["uses"],
        json!([]),
        "a step that adds nothing new says so — nothing to add, just the pot"
    );

    // No Step names the salt, and the last line was never read at all, so
    // neither can reach a step-scoped panel. That is ADR 0002 working: both
    // lines are still on the page, both still shop, and neither is marked.
    for slot in cooking["steps"].as_array().unwrap() {
        if let Some(uses) = slot["uses"].as_array() {
            assert!(
                !uses.contains(&json!(4)) && !uses.contains(&json!(5)),
                "a line no Step names cannot be derived onto one, read or not"
            );
        }
    }

    // Nothing here is stored: it is worked out from `readings` and `content`,
    // which is what makes it survive the reader itself being replaced.
    assert_eq!(
        fetched["result"]["versions"][0]["readings"][5],
        json!(null),
        "a line Kamosu cannot read stays unread, and is a working line anyway"
    );
}

/// A Step names a Food as a whole word or not at all — and the one latitude
/// is a trailing `s`, so a line read as *egg* reaches the step that says
/// *eggs*.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_names_a_food_as_a_whole_word_and_never_as_a_fragment() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Whole words only",
            "ingredients": [
                { "kind": "ingredient", "text": "2 eggs" },
                { "kind": "ingredient", "text": "200 g rice" },
                { "kind": "ingredient", "text": "1 gousse d'ail" },
            ],
            "steps": [
                { "kind": "step", "text": "Beat the eggs." },
                { "kind": "step", "text": "Check the price of the eggshell substitute." },
                { "kind": "step", "text": "Faire revenir l'ail." },
            ],
        })
        .to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();
    for (line_index, target) in [(0, "egg"), (1, "rice"), (2, "ail")] {
        let (status, read) = app.post_op(
            "set_reading",
            Some(&key),
            &json!({ "branch_id": branch_id, "line_index": line_index, "target": target })
                .to_string(),
        );
        assert_eq!(status, 200, "{read}");
    }

    let (_, fetched) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let steps = &fetched["result"]["versions"][0]["cooking"]["steps"];
    assert_eq!(
        steps[0]["uses"],
        json!([0]),
        "`eggs` uses the line read as `egg` — one trailing s is forgiven"
    );
    assert_eq!(
        steps[1]["uses"],
        json!([]),
        "`price` does not contain `rice` and `eggshell` does not contain `egg`: \
         a fragment is not a mention"
    );
    assert_eq!(
        steps[2]["uses"],
        json!([2]),
        "`l'ail` names the garlic — a word France would otherwise find inside \
         half its own language"
    );
}

/// A duration is read out of the Step's own text and stored nowhere — and a
/// Step with no duration offers no timer, which is three Steps in four.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_duration_in_a_steps_text_is_offered_as_a_timer_and_nothing_else_is() {
    let app = support::spawn_app();
    let (key, branch_id) = recipe_read_and_ready_to_cook(&app, "Aurélien");

    let (_, fetched) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let cooking = &fetched["result"]["versions"][0]["cooking"];

    assert_eq!(
        cooking["steps"][2]["timer_seconds"],
        json!(420),
        "`simmer for about 7 minutes` offers seven minutes"
    );
    assert_eq!(cooking["steps"][1]["timer_seconds"], json!(null));
    assert_eq!(
        cooking["steps"][3]["timer_seconds"],
        json!(null),
        "`Cover and leave it alone` names no duration, so it offers no timer"
    );

    // The Step's text is untouched: a timer is read out of it, never written
    // into it (CONTEXT.md, "Step").
    assert_eq!(
        fetched["result"]["versions"][0]["content"]["steps"][2]["text"],
        json!("Pour in the water and simmer for about 7 minutes.")
    );
}

/// Every shape of duration the real corpus actually writes, driven through the
/// real Operation rather than a unit test — the same rule #49 holds units to.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_range_offers_its_lower_end_and_a_number_that_is_not_a_duration_offers_nothing() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let written = [
        ("Knead the dough for 3–4 minutes.", json!(180)),
        ("Microwave until golden, 1 to 3 minutes.", json!(60)),
        ("Marinate in the fridge for 20-30 min.", json!(1200)),
        ("Cover and refrigerate 8 hours or overnight.", json!(28800)),
        ("Laisser reposer 1 heure.", json!(3600)),
        ("Cocinar 30 segundos.", json!(30)),
        ("Rest for 1.5 hours.", json!(5400)),
        ("Preheat oven to 350 F (175 C).", json!(null)),
        ("Turn the pan 90 degrees.", json!(null)),
        ("Add 2 tbsp of soy sauce and 500 g of rice.", json!(null)),
        ("Bake at 450F/230C for 20-30 min.", json!(1200)),
        // Minutes written after an hour and given no unit of their own — how
        // French writes an hour and a half. Absent from the 86-recipe export,
        // and handled from the language rather than from the corpus.
        ("Laisser lever 1 h 30.", json!(5400)),
        ("Rest for 1 hour 30 minutes.", json!(5400)),
        // The same shape where the trailing number is somebody else's: an hour
        // count and a weight, never an hour and two hundred minutes.
        ("Simmer 2 hours, then add 200 g of rice.", json!(7200)),
        // Past a week it was never a duration anybody meant to time.
        ("Age it for 9000 hours.", json!(null)),
    ];
    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Every duration the corpus writes",
            "steps": written
                .iter()
                .map(|(text, _)| json!({ "kind": "step", "text": text }))
                .collect::<Vec<Value>>(),
        })
        .to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap();

    let (_, fetched) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let steps = fetched["result"]["versions"][0]["cooking"]["steps"]
        .as_array()
        .unwrap()
        .clone();
    for (index, (text, expected)) in written.iter().enumerate() {
        assert_eq!(
            steps[index]["timer_seconds"], *expected,
            "the timer read out of {text:?}"
        );
    }
}

/// The panel is scaled and converted by the same one slot the recipe page uses
/// (#49, ADR 0016) — the cooking screen learns no arithmetic of its own, and a
/// quantity Kamosu could not read is left whole rather than guessed at.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_amounts_a_step_uses_carry_the_one_subordinate_line_already_worked_out() {
    let app = support::spawn_app();
    let (key, branch_id) = recipe_read_and_ready_to_cook(&app, "Aurélien");
    let (status, set) = app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );
    assert_eq!(status, 200, "{set}");

    // Cooking to eight servings where the recipe is written for four: the
    // Attempt's Yield is what scales the panel, and it is a fact about this
    // afternoon rather than a deviation (ADR 0010).
    let (_, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let attempt_id = started["result"]["id"].as_str().unwrap().to_string();
    let (status, advanced) = app.post_op(
        "advance_attempt",
        Some(&key),
        &json!({
            "attempt_id": attempt_id,
            "cooking_yield": { "amount": "8", "noun": "servings" },
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{advanced}");

    let (_, fetched) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let version = &fetched["result"]["versions"][0];
    let panko = version["cooking"]["steps"][1]["uses"][1].as_u64().unwrap() as usize;
    assert_eq!(
        version["content"]["ingredients"][panko]["text"],
        json!("1 cup panko"),
        "the panel is drawn from the written line, at full size (ADR 0002)"
    );
    assert_eq!(
        version["measured"]["ingredients"][panko],
        json!("about 470 ml"),
        "two cups of panko for a doubled cook, in this reader's measures — one \
         slot doing scaling and conversion together (#49). Millilitres rather \
         than grams because panko has no Cup Weight, which ADR 0016 calls a line \
         that offers millilitres instead rather than a gap to close"
    );
    assert_eq!(
        version["measured"]["ingredients"][4],
        json!(null),
        "`a pinch of salt` was never read, so nothing is guessed beneath it"
    );
}

// --- Meaning Search (#63, ADR 0029) ------------------------------------------
//
// Everything here is about the instance that ships: no model, Meaning Search
// off, and complete. The half that needs 220 MB of weights and a real library
// is `tests/meaning_corpus.rs`, which is `#[ignore]`d for exactly that reason.

/// A second Person who is not the Operator, with an Access Key of their own.
fn a_person_who_is_not_the_operator(app: &support::TestApp) -> String {
    let person = app.core.create_person("Camille").expect("person");
    app.core
        .mint_access_key(&person, "camille's agent", false)
        .unwrap()
        .secret
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_fresh_instance_searches_by_words_and_says_so() {
    let app = support::spawn_app();
    let (key, kitchen_id) = operator_with_kitchen(&app);

    let (status, held) = app.post_op("meaning_search_status", Some(&key), "{}");
    assert_eq!(status, 200, "{held}");
    let held = &held["result"];
    // Kamosu ships no model, so this is where every instance begins — and where
    // most of them stay, complete (ADR 0029).
    assert_eq!(held["state"], json!("unasked"));
    assert_eq!(held["on"], json!(false));
    assert_eq!(held["model_present"], json!(false));
    assert_eq!(held["accepted_by"], json!(null));
    // The offer is live, and it names what would be agreed to before anybody
    // agrees to it.
    assert_eq!(held["offer"], json!(true));
    assert_eq!(
        held["terms_url"],
        json!("https://ai.google.dev/gemma/terms")
    );
    assert!(held["model"].as_str().is_some_and(|m| m.contains("Gemma")));

    // And searching is the same one Operation it always was, answering the same
    // shape. The Catalogue cannot change per instance, so nothing here is
    // conditional on there being a model.
    let (_, made) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Miso Soup" }).to_string(),
    );
    assert_eq!(made["ok"], json!(true), "{made}");
    let (status, found) = app.post_op(
        "search_recipes",
        Some(&key),
        &json!({ "query": "miso" }).to_string(),
    );
    assert_eq!(status, 200, "{found}");
    assert_eq!(found["result"]["recipes"][0]["title"], json!("Miso Soup"));
    assert_eq!(
        found["result"]["recipes"][0]["matched"]["by"],
        json!("words")
    );
    // With no model there is no nearest neighbour, and Kamosu does not invent
    // one: nothing found is nothing found.
    let (_, nothing) = app.post_op(
        "search_recipes",
        Some(&key),
        &json!({ "query": "osso buco alla milanese" }).to_string(),
    );
    assert_eq!(nothing["result"]["recipes"], json!([]));
    assert_eq!(nothing["result"]["closest"], json!(false));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_offer_is_only_made_to_somebody_who_could_act_on_it() {
    let app = support::spawn_app();
    let (_operator_key, _) = operator_with_kitchen(&app);
    let camille = a_person_who_is_not_the_operator(&app);

    // Camille can read the status — she will meet *nothing found* like anybody
    // else — but is never offered a thing only an Operator can do. An offer
    // somebody cannot act on is worse than no offer at all.
    let (status, held) = app.post_op("meaning_search_status", Some(&camille), "{}");
    assert_eq!(status, 200, "{held}");
    assert_eq!(held["result"]["state"], json!("unasked"));
    assert_eq!(held["result"]["offer"], json!(false));

    // And the Operations themselves refuse her, in the Core rather than in a
    // Door: the same refusal arrives at both.
    for name in [
        "accept_meaning_search_terms",
        "decline_meaning_search",
        "download_meaning_model",
        "build_meaning_index",
        "turn_off_meaning_search",
    ] {
        let (status, refused) = app.post_op(name, Some(&camille), "{}");
        assert_eq!(status, 401, "{name} must require an Operator: {refused}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn accepting_the_terms_keeps_the_hand_that_accepted_and_how_it_arrived() {
    let app = support::spawn_app();
    let (key, _) = operator_with_kitchen(&app);

    // Accepted through an Access Key — deliberately allowed. An agent acts *as*
    // its Person, and minting it a Key was already the act of authorising that;
    // a web-door-only carve-out would be the first hole in Parity and would
    // stop nothing anyway (ADR 0029).
    let (status, accepted) = app.post_op("accept_meaning_search_terms", Some(&key), "{}");
    assert_eq!(status, 200, "{accepted}");
    assert_eq!(accepted["result"]["state"], json!("accepted"));

    let (_, held) = app.post_op("meaning_search_status", Some(&key), "{}");
    let held = &held["result"];
    assert_eq!(held["state"], json!("accepted"));
    assert!(held["accepted_by"].is_string(), "the Hand is kept: {held}");
    assert_eq!(held["accepted_via_access_key"], json!(true));
    assert!(held["accepted_at"].is_string());
    // Which issue of the terms was agreed to, so a later revision is a new
    // question rather than a silent assumption.
    assert_eq!(held["terms_version"], json!("2026-04-01"));
    // Nothing has been downloaded by agreeing to anything.
    assert_eq!(held["model_present"], json!(false));
    assert_eq!(held["on"], json!(false));
    // The offer is spent: it was answered.
    assert_eq!(held["offer"], json!(false));

    // Accepting again is not an error and does not rewrite the first
    // acceptance — the record is of who agreed, and that already happened.
    let first = held["accepted_at"].clone();
    let (status, again) = app.post_op("accept_meaning_search_terms", Some(&key), "{}");
    assert_eq!(status, 200, "{again}");
    let (_, still) = app.post_op("meaning_search_status", Some(&key), "{}");
    assert_eq!(still["result"]["accepted_at"], first);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_read_only_access_key_cannot_accept_the_terms() {
    let app = support::spawn_app();
    let first = json!({
        "name": "Aurélien",
        "password": "a password only its person knows",
        "session_name": "test browser",
    });
    let (status, created) = app.post_auth("/auth/first-person", &first.to_string());
    assert_eq!(status, 200, "{created}");
    let operator_id = created["result"]["person"]["id"].as_str().unwrap();
    let read_only = app
        .core
        .mint_access_key(operator_id, "a watcher", true)
        .unwrap()
        .secret;

    // No new guard was needed for this: accepting is a write, and a read-only
    // Key is refused every write in the Catalogue (ADR 0029, ADR 0031).
    let (status, refused) = app.post_op("accept_meaning_search_terms", Some(&read_only), "{}");
    assert_eq!(status, 401, "{refused}");
    let (_, held) = app.post_op("meaning_search_status", Some(&read_only), "{}");
    assert_eq!(held["result"]["state"], json!("unasked"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn declining_means_the_offer_is_never_made_again() {
    let app = support::spawn_app();
    let (key, _) = operator_with_kitchen(&app);

    let (status, declined) = app.post_op("decline_meaning_search", Some(&key), "{}");
    assert_eq!(status, 200, "{declined}");
    assert_eq!(declined["result"]["state"], json!("declined"));

    let (_, held) = app.post_op("meaning_search_status", Some(&key), "{}");
    let held = &held["result"];
    assert_eq!(held["state"], json!("declined"));
    assert!(held["declined_at"].is_string());
    // The whole of what declining buys: a question already answered is never
    // asked again.
    assert_eq!(held["offer"], json!(false));
    assert_eq!(held["on"], json!(false));

    // And the instance is complete without it: word search is untouched.
    let (status, found) = app.post_op(
        "search_recipes",
        Some(&key),
        &json!({ "query": "anything" }).to_string(),
    );
    assert_eq!(status, 200, "{found}");
    assert_eq!(found["result"]["closest"], json!(false));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn nothing_downloads_or_indexes_before_the_terms_are_accepted() {
    let app = support::spawn_app();
    let (key, _) = operator_with_kitchen(&app);

    // The one thing Kamosu will not do without being asked: fetch somebody
    // else's weights under somebody else's licence. Asked before accepting, the
    // Job runs and fails saying exactly that — a Job's failure is read back the
    // way every Job's outcome is (ADR 0032).
    let (status, asked) = app.post_op("download_meaning_model", Some(&key), "{}");
    assert_eq!(status, 200, "{asked}");
    let job_id = asked["result"]["job_id"].as_str().unwrap().to_string();
    let failure = support::wait_terminal(&app, Some(&key), &job_id);
    assert_eq!(failure["status"], json!("failed"), "{failure}");
    assert!(
        failure["error"]
            .as_str()
            .is_some_and(|why| why.contains("terms")),
        "the reason names the terms: {failure}"
    );

    // And nothing can be indexed against a model that is not there.
    let (_, asked) = app.post_op("build_meaning_index", Some(&key), "{}");
    let job_id = asked["result"]["job_id"].as_str().unwrap().to_string();
    let failure = support::wait_terminal(&app, Some(&key), &job_id);
    assert_eq!(failure["status"], json!("failed"), "{failure}");
    assert!(
        failure["error"]
            .as_str()
            .is_some_and(|why| why.contains("downloaded")),
        "the reason names the missing model: {failure}"
    );

    let (_, held) = app.post_op("meaning_search_status", Some(&key), "{}");
    assert_eq!(held["result"]["state"], json!("unasked"));
    assert_eq!(held["result"]["on"], json!(false));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_meaning_search_operations_are_the_same_six_at_the_mcp_door() {
    let app = support::spawn_app();
    let (key, _) = operator_with_kitchen(&app);

    // Parity is a fact of assembly rather than a rule anybody remembers
    // (ADR 0001), and `tests/parity.rs` proves it for the whole Catalogue. What
    // this adds is that accepting a licence — the one place a web-only carve-out
    // would have been tempting — really is available to an agent (ADR 0029).
    let call = |name: &str| {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": name, "arguments": {} },
        });
        let (status, answered) = app.post_mcp(&payload.to_string(), Some(&key));
        assert_eq!(status, 200, "{name}: {answered}");
        answered["result"]["structuredContent"].clone()
    };

    let accepted = call("accept_meaning_search_terms");
    assert_eq!(accepted["state"], json!("accepted"), "{accepted}");

    let held = call("meaning_search_status");
    assert_eq!(held["state"], json!("accepted"));
    assert_eq!(held["accepted_via_access_key"], json!(true));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn turning_off_what_was_never_on_invents_no_acceptance() {
    let app = support::spawn_app();
    let (key, _) = operator_with_kitchen(&app);

    // Turning off an instance nobody has answered for must not quietly record
    // an acceptance with nobody's Hand on it — and, because the offer is live
    // exactly while nobody has answered, must not spend the offer either.
    let (status, off) = app.post_op("turn_off_meaning_search", Some(&key), "{}");
    assert_eq!(status, 200, "{off}");
    assert_eq!(off["result"]["state"], json!("unasked"), "{off}");

    let (_, held) = app.post_op("meaning_search_status", Some(&key), "{}");
    assert_eq!(held["result"]["state"], json!("unasked"));
    assert_eq!(held["result"]["accepted_by"], json!(null));
    assert_eq!(
        held["result"]["offer"],
        json!(true),
        "the offer has not been answered, so it is still there"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn declining_is_an_answer_to_the_offer_and_nothing_else() {
    let app = support::spawn_app();
    let (key, _) = operator_with_kitchen(&app);

    let (_, accepted) = app.post_op("accept_meaning_search_terms", Some(&key), "{}");
    assert_eq!(accepted["result"]["state"], json!("accepted"));

    // Declining terms already accepted is not a decline. It would be turning
    // Meaning Search off while claiming otherwise — and there is an Operation
    // that actually does that, so this one says no rather than pretending.
    let (status, refused) = app.post_op("decline_meaning_search", Some(&key), "{}");
    assert_eq!(status, 400, "{refused}");

    let (_, held) = app.post_op("meaning_search_status", Some(&key), "{}");
    assert_eq!(held["result"]["state"], json!("accepted"));
    assert_eq!(held["result"]["declined_at"], json!(null));
}

// --- Home: the computed shelves (issue #64) ----------------------------------
//
// Home answers *show me something* rather than handing back a search box, so
// everything on it is worked out from recipes and Attempts that already exist —
// except *recently opened*, which reads the one fact this screen stores. All of
// it is asked through a real Door.

/// Every shelf on this Person's Home, keyed by name, with each shelf's recipe
/// titles in the order they arrived. What the screen actually reads, minus the
/// card's other fields.
fn home_titles(
    app: &support::TestApp,
    key: &str,
) -> std::collections::HashMap<String, Vec<String>> {
    let (status, home) = app.post_op("home_shelves", Some(key), "{}");
    assert_eq!(status, 200, "{home}");
    home["result"]["shelves"]
        .as_array()
        .expect("shelves")
        .iter()
        .map(|shelf| {
            (
                shelf["name"].as_str().expect("a shelf name").to_string(),
                shelf["recipes"]
                    .as_array()
                    .expect("recipes")
                    .iter()
                    .map(|card| card["title"].as_str().unwrap().to_string())
                    .collect(),
            )
        })
        .collect()
}

/// A recipe with times on it, which is what *quick tonight* is computed from.
fn timed_recipe_in(
    app: &support::TestApp,
    key: &str,
    kitchen_id: &str,
    title: &str,
    prep: Option<i64>,
    cook: Option<i64>,
) -> String {
    let (status, created) = app.post_op(
        "create_recipe",
        Some(key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": title,
            "prep_time_minutes": prep,
            "cook_time_minutes": cook,
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{created}");
    created["result"]["branch_id"].as_str().unwrap().to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn home_shows_the_four_computed_shelves_the_spec_names() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    // Cooked twice and quick: it stands on two shelves at once, because a
    // suggestion is not a filing system and nothing here is exclusive.
    let ramen = timed_recipe_in(&app, &key, &kitchen_id, "Ramen", Some(5), Some(10));
    // Cooked once, and far too long to be a Tuesday.
    let cassoulet = timed_recipe_in(&app, &key, &kitchen_id, "Cassoulet", Some(60), Some(180));
    // Never cooked, and quick.
    let omelette = timed_recipe_in(&app, &key, &kitchen_id, "Omelette", None, Some(8));

    cook_it(&app, &key, &ramen, json!({}));
    cook_it(&app, &key, &ramen, json!({}));
    cook_it(&app, &key, &cassoulet, json!({}));

    app.post_op(
        "note_recipe_opened",
        Some(&key),
        &json!({ "branch_id": omelette }).to_string(),
    );

    let shelves = home_titles(&app, &key);

    assert_eq!(
        shelves.get("cooked_most"),
        Some(&vec!["Ramen".to_string(), "Cassoulet".to_string()]),
        "most-cooked first: {shelves:?}"
    );
    assert_eq!(
        shelves.get("quick_tonight"),
        Some(&vec!["Omelette".to_string(), "Ramen".to_string()]),
        "soonest first, and the three-hour cassoulet is not on it: {shelves:?}"
    );
    assert_eq!(
        shelves.get("never_cooked"),
        Some(&vec!["Omelette".to_string()]),
        "only the one nobody has made: {shelves:?}"
    );
    assert_eq!(
        shelves.get("recently_opened"),
        Some(&vec!["Omelette".to_string()]),
        "the one that was opened: {shelves:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn home_counts_unfinished_cookings_because_starting_is_what_makes_a_cooking_real() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let soup = recipe_in(&app, &key, &kitchen_id, "Miso Soup");
    recipe_in(&app, &key, &kitchen_id, "Pain");

    // Started and never finished. It is still a cooking (ADR 0010), which is
    // the same rule the diary shows it under — so it counts here and takes the
    // recipe off *never cooked*.
    let (status, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": soup }).to_string(),
    );
    assert_eq!(status, 200, "{started}");

    let shelves = home_titles(&app, &key);
    assert_eq!(
        shelves.get("cooked_most"),
        Some(&vec!["Miso Soup".to_string()]),
        "an unfinished cooking is a cooking: {shelves:?}"
    );
    assert_eq!(
        shelves.get("never_cooked"),
        Some(&vec!["Pain".to_string()]),
        "and the recipe it was started on has left *never cooked*: {shelves:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_recipe_kamosu_knows_no_time_for_is_never_called_quick() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    // No time at all — the ordinary state of 36 of the 86 real recipes. Calling
    // it quick would be a guess, which is the one thing Kamosu will not do with
    // a number it does not have.
    recipe_in(&app, &key, &kitchen_id, "Grand-mère's stew");
    // One of the two times is enough to answer with.
    timed_recipe_in(&app, &key, &kitchen_id, "Toast", None, Some(3));
    // Exactly on the line is under it.
    timed_recipe_in(&app, &key, &kitchen_id, "Risotto", Some(10), Some(20));
    // One minute past is past.
    timed_recipe_in(&app, &key, &kitchen_id, "Daube", Some(11), Some(20));

    let shelves = home_titles(&app, &key);
    assert_eq!(
        shelves.get("quick_tonight"),
        Some(&vec!["Toast".to_string(), "Risotto".to_string()]),
        "no time means not quick, and 31 minutes is not 30: {shelves:?}"
    );

    // The line itself is sent rather than written on the screen, so the heading
    // a reader sees cannot drift from the number that chose what is under it.
    let (_, home) = app.post_op("home_shelves", Some(&key), "{}");
    assert_eq!(home["result"]["quick_tonight_minutes"], json!(30), "{home}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_empty_shelf_is_left_out_and_an_empty_library_answers_with_no_shelves_at_all() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    // A Kitchen holding nothing. Four empty rows would be four ways of saying
    // the same nothing, so the answer carries none and the screen says it once.
    let (status, empty) = app.post_op("home_shelves", Some(&key), "{}");
    assert_eq!(status, 200, "{empty}");
    assert_eq!(empty["result"]["shelves"], json!([]), "{empty}");

    // One recipe, never cooked, no time on it, never opened: exactly one shelf
    // has anything to say, and it is the only one that appears.
    recipe_in(&app, &key, &kitchen_id, "Miso Soup");
    let shelves = home_titles(&app, &key);
    assert_eq!(
        shelves.keys().collect::<Vec<_>>(),
        vec!["never_cooked"],
        "a shelf with nothing on it is not sent: {shelves:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn recently_opened_is_one_fact_per_person_per_lineage_and_reaches_nobody_else() {
    let app = support::spawn_app();
    let (_aurelien, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_marc, marc_key, marc_kitchen) = person_with_kitchen(&app, "Marc");

    let soup = recipe_in(&app, &key, &kitchen_id, "Miso Soup");
    let curry = recipe_in(&app, &key, &kitchen_id, "Katsu Curry");
    recipe_in(&app, &marc_key, &marc_kitchen, "Cassoulet");

    let open = |branch: &str| {
        let (status, noted) = app.post_op(
            "note_recipe_opened",
            Some(&key),
            &json!({ "branch_id": branch }).to_string(),
        );
        assert_eq!(status, 200, "{noted}");
        noted["result"].clone()
    };

    let first = open(&soup);
    // The opening is recorded against the Lineage, never the Branch: opening a
    // recipe's French rendering and its English one is opening one recipe.
    assert!(first["lineage_id"].is_string(), "{first}");
    assert_ne!(first["lineage_id"], json!(soup), "{first}");
    open(&curry);

    let shelves = home_titles(&app, &key);
    assert_eq!(
        shelves.get("recently_opened"),
        Some(&vec!["Katsu Curry".to_string(), "Miso Soup".to_string()]),
        "latest first: {shelves:?}"
    );

    // Opening it again moves it to the front rather than adding a second row.
    open(&soup);
    let shelves = home_titles(&app, &key);
    assert_eq!(
        shelves.get("recently_opened"),
        Some(&vec!["Miso Soup".to_string(), "Katsu Curry".to_string()]),
        "one fact per Lineage, moved rather than doubled: {shelves:?}"
    );

    // Marc's Home knows nothing about what Aurélien opened, and Aurélien's
    // Home has never heard of Marc's Kitchen.
    let marc_home = home_titles(&app, &marc_key);
    assert_eq!(marc_home.get("recently_opened"), None, "{marc_home:?}");
    assert_eq!(
        marc_home.get("never_cooked"),
        Some(&vec!["Cassoulet".to_string()]),
        "{marc_home:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_recipe_in_a_kitchen_you_do_not_cook_in_can_be_neither_opened_nor_shelved() {
    let app = support::spawn_app();
    let (_aurelien, key, _kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_marc, marc_key, marc_kitchen) = person_with_kitchen(&app, "Marc");
    let cassoulet = recipe_in(&app, &marc_key, &marc_kitchen, "Cassoulet");

    let (status, refused) = app.post_op(
        "note_recipe_opened",
        Some(&key),
        &json!({ "branch_id": cassoulet }).to_string(),
    );
    assert_eq!(status, 404, "{refused}");
    assert_eq!(refused["error"]["kind"], json!("not_found"), "{refused}");
    assert_eq!(
        refused["error"]["message"],
        json!("no such Branch"),
        "{refused}"
    );

    // And nothing of Marc's reaches Aurélien's Home, which is the same
    // Kitchen boundary the library's shelf draws (ADR 0026).
    let shelves = home_titles(&app, &key);
    assert!(shelves.is_empty(), "{shelves:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn home_cards_are_the_same_cards_the_library_shelf_serves() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let soup = recipe_in(&app, &key, &kitchen_id, "Miso Soup");

    let (_, home) = app.post_op("home_shelves", Some(&key), "{}");
    let on_home = home["result"]["shelves"][0]["recipes"][0].clone();
    let (_, shelf) = app.post_op("search_recipes", Some(&key), "{}");
    let in_library = shelf["result"]["recipes"][0].clone();

    assert_eq!(
        on_home, in_library,
        "one card, one shape, both screens: {home} / {shelf}"
    );
    assert_eq!(on_home["branch_id"], json!(soup));
    assert_eq!(
        on_home["matched"],
        json!(null),
        "nothing was searched for, so nothing matched"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn home_files_a_lineage_once_under_the_reading_language() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (status, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Soupe miso",
            "language": "fr",
            "prep_time_minutes": 5,
            "cook_time_minutes": 5,
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{created}");
    let soupe = created["result"]["branch_id"].as_str().unwrap().to_string();

    // The same recipe again in English: a Translation is a Branch of the same
    // Lineage (ADR 0006), so Home must show one card, not two.
    let (status, translated) = app.post_op(
        "start_translation",
        Some(&key),
        &json!({
            "branch_id": soupe,
            "language": "en",
            "title": "Miso Soup",
            "prep_time_minutes": 5,
            "cook_time_minutes": 5,
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{translated}");

    let reading = |language: &str| {
        let (status, set) = app.post_op(
            "set_reading_preferences",
            Some(&key),
            &json!({ "reading_language": language, "reading_measures": "metric" }).to_string(),
        );
        assert_eq!(status, 200, "{set}");
    };

    reading("fr");
    let shelves = home_titles(&app, &key);
    assert_eq!(
        shelves.get("quick_tonight"),
        Some(&vec!["Soupe miso".to_string()]),
        "one card per Lineage, in the reader's own Language: {shelves:?}"
    );

    // Reading in English moves that same one card to the English Branch rather
    // than adding a second — the two Branches are one recipe (ADR 0006).
    reading("en");
    let shelves = home_titles(&app, &key);
    assert_eq!(
        shelves.get("quick_tonight"),
        Some(&vec!["Miso Soup".to_string()]),
        "still one card, now in English: {shelves:?}"
    );
    assert_eq!(
        shelves.get("never_cooked"),
        Some(&vec!["Miso Soup".to_string()]),
        "and one card on every shelf it stands on: {shelves:?}"
    );
}

// ── Share Links (#65, ADR 0026, ADR 0018) ────────────────────────────────────

/// Share a recipe and answer (branch_id, token, url).
fn share(app: &support::TestApp, key: &str, branch_id: &str) -> (String, String) {
    let (status, shared) = app.post_op(
        "share_recipe",
        Some(key),
        &json!({ "branch_id": branch_id, "public_address": "https://kamosu.example" }).to_string(),
    );
    assert_eq!(status, 200, "{shared}");
    let url = shared["result"]["url"]
        .as_str()
        .expect("a link")
        .to_string();
    let token = url.rsplit('/').next().expect("a token").to_string();
    (token, url)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_share_link_is_one_permanent_address_and_asking_twice_does_not_mint_a_second() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_status, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Tarte aux pommes" }).to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    let (token, url) = share(&app, &key, &branch_id);
    assert!(url.starts_with("https://kamosu.example/s/"));
    // 256 bits of Secret, as hex. Enumeration is answered by the size of the
    // Secret and by nothing else (ADR 0031, ADR 0032).
    assert_eq!(token.len(), 64, "a Share Link token is 256 bits");
    assert!(token.chars().all(|c| c.is_ascii_hexdigit()));

    // Asking again is one intention, not two: the same link comes back, and a
    // second live link is never minted.
    let (status, again) = app.post_op(
        "share_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 200, "{again}");
    assert_eq!(again["result"]["shared"], json!(true));
    // The Secret is answered once, at minting, because only its hash is kept.
    assert_eq!(again["result"]["url"], Value::Null);

    // And the link still opens the recipe.
    let (status, _type, page) = app.get(&format!("/s/{token}"));
    assert_eq!(status, 200, "{page}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ending_a_share_link_is_permanent_and_re_enabling_mints_a_new_one() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_status, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Coq au vin" }).to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    let (first, _url) = share(&app, &key, &branch_id);
    let (status, ended) = app.post_op(
        "end_share_link",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 200, "{ended}");
    assert_eq!(ended["result"]["shared"], json!(false));

    // The old link is dead — and says so, rather than reading as a mistake to
    // retry. It is still a 200: this is an answer, not a failure.
    let (status, _type, page) = app.get(&format!("/s/{first}"));
    assert_eq!(status, 200);
    assert!(page.contains("This link was ended"), "{page}");
    // And it carries none of the recipe.
    assert!(!page.contains("Coq au vin"), "an ended link shows nothing");

    // Turning sharing back on mints a NEW link, and the withdrawn one stays dead.
    let (second, _url) = share(&app, &key, &branch_id);
    assert_ne!(second, first, "re-enabling mints a new Secret");
    let (status, _type, page) = app.get(&format!("/s/{second}"));
    assert_eq!(status, 200);
    assert!(page.contains("Coq au vin"), "the new link opens the recipe");
    let (_status, _type, dead) = app.get(&format!("/s/{first}"));
    assert!(
        dead.contains("This link was ended"),
        "a withdrawn link stays dead"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_share_link_page_never_shows_an_attempt() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let plated = upload_a_picture(&app, &key, 12);
    let (_status, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Katsu Curry",
            "main_photo": plated,
            "steps": [{ "kind": "step", "text": "Fry the cutlet.", "photo": null }],
        })
        .to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    // Cook it, photograph it, and say something private about the cooking.
    let picture = upload_a_picture(&app, &key, 77);
    let (status, attempt) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 200, "{attempt}");
    let attempt_id = attempt["result"]["id"].as_str().unwrap().to_string();
    let (status, finished) = app.post_op(
        "finish_attempt",
        Some(&key),
        &json!({
            "attempt_id": attempt_id,
            "rating": "again",
            "note": "Cut the sugar, Marie hated it",
            "photographs": [picture],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{finished}");
    assert_eq!(finished["result"]["photographs"], json!([picture]));

    let (token, _url) = share(&app, &key, &branch_id);
    let (status, _type, page) = app.get(&format!("/s/{token}"));
    assert_eq!(status, 200);
    assert!(page.contains("Katsu Curry"), "the recipe is there");
    // The private place is the Attempt note, and it never leaves the instance.
    assert!(
        !page.contains("Marie"),
        "an Attempt note must never reach a Share Link page"
    );
    assert!(
        !page.contains("again"),
        "a rating must never reach a Share Link page"
    );
    assert!(
        !page.contains(&picture),
        "an Attempt photograph must never reach a Share Link page"
    );
    // The token opens the recipe's own pictures, and a cooking's is not one.
    let (status, _type, _body) = app.get(&format!("/s/{token}/photo/{plated}"));
    assert_eq!(status, 200, "the recipe's own photograph is served");
    let (status, _type, _body) = app.get(&format!("/s/{token}/photo/{picture}"));
    assert_ne!(
        status, 200,
        "a Share Link must not serve an Attempt photograph"
    );

    // Nor through the Operation the page consumes.
    let (status, read) = app.post_op(
        "read_shared_recipe",
        None,
        &json!({ "token": token }).to_string(),
    );
    assert_eq!(status, 200, "{read}");
    let text = read.to_string();
    assert!(!text.contains("Marie"), "no Attempt in the answer: {text}");
    assert!(
        !text.contains(&picture),
        "no Attempt photograph in the answer: {text}"
    );
    assert!(
        !text.contains("attempts"),
        "no Attempts field exists at all"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_share_carries_the_whole_chain_back_to_the_first_version() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_status, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Shortbread" }).to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    // A Translation closes the collapse window on the first Version, so the
    // next save appends rather than replacing it (ADR 0006).
    let (status, translated) = app.post_op(
        "start_translation",
        Some(&key),
        &json!({ "branch_id": branch_id, "language": "fr", "title": "Sablés" }).to_string(),
    );
    assert_eq!(status, 200, "{translated}");

    let (status, saved) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": branch_id,
            "title": "Shortbread",
            "ingredients": [{ "kind": "ingredient", "text": "250 g butter" }],
            "name": "More butter",
            "change_note": "Took the butter up, which is the whole of it.",
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{saved}");

    let (token, _url) = share(&app, &key, &branch_id);
    let (status, read) = app.post_op(
        "read_shared_recipe",
        None,
        &json!({ "token": token }).to_string(),
    );
    assert_eq!(status, 200, "{read}");
    let thread = read["result"]["thread"].as_array().expect("a Thread");
    assert_eq!(
        thread.len(),
        2,
        "the chain reaches the first Version: {thread:?}"
    );
    assert_eq!(thread[0]["sequence"], json!(1));
    assert_eq!(thread[1]["name"], json!("More butter"));
    // The *what changed* line and the Hand travel: that is what makes credit
    // travel with the recipe (ADR 0018, ADR 0015).
    assert_eq!(
        thread[1]["change_note"],
        json!("Took the butter up, which is the whole of it.")
    );
    assert_eq!(thread[1]["hand"], json!("Aurélien"), "a Person, by name");

    // Its Translation travels beside it (ADR 0006).
    let translations = read["result"]["translations"].as_array().expect("a list");
    assert_eq!(translations.len(), 1);
    assert_eq!(translations[0]["language"], json!("fr"));

    // The page shows the Thread rather than hiding it: the sharer opening
    // their own link must see what they actually published.
    let (status, _type, page) = app.get(&format!("/s/{token}"));
    assert_eq!(status, 200);
    assert!(page.contains("More butter"), "{page}");
    assert!(page.contains("Took the butter up"));
    assert!(page.contains("Aurélien"));
    // And the standing line is on it, in the same words every time.
    assert!(
        page.contains("it cannot reach a copy already sent"),
        "the standing line is on the page"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_share_link_page_closes_the_ingredients_with_the_nutrition_figure() {
    // #84: the figure is one of the recipe's own words, so it travels with the
    // recipe and a stranger holding a link reads it. Aurélien chose the foot of
    // the Ingredients on 21 September 2026, against four treatments drawn on
    // this page and on the app's. It always says what it counts — 308 says
    // nothing on its own — and most recipes carry none at all.
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_status, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Katsu Curry" }).to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    // A recipe with no figure, which is the ordinary case: nothing is said.
    let (token, _url) = share(&app, &key, &branch_id);
    let (status, _type, page) = app.get(&format!("/s/{token}"));
    assert_eq!(status, 200, "{page}");
    assert!(
        !page.contains("kcal"),
        "a recipe with no figure shows no placeholder: {page}"
    );

    // Per serving.
    let (status, saved) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": branch_id,
            "title": "Katsu Curry",
            "ingredients": [{ "kind": "ingredient", "text": "2 chicken thighs" }],
            "nutrition": { "calories": 308, "basis": "per_serving" },
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{saved}");
    let (status, _type, page) = app.get(&format!("/s/{token}"));
    assert_eq!(status, 200);
    assert!(page.contains("308 kcal a serving"), "{page}");
    // At the foot of the list: after the last Ingredient Line, before Method.
    let figure = page.find("308 kcal a serving").expect("the figure");
    assert!(page.find("2 chicken thighs").expect("the line") < figure);
    assert!(figure < page.find(">Method<").expect("the heading"));

    // Per 100 g says so instead, in the same place.
    let (status, saved) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": branch_id,
            "title": "Katsu Curry",
            "ingredients": [{ "kind": "ingredient", "text": "2 chicken thighs" }],
            "nutrition": { "calories": 154, "basis": "per_100g" },
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{saved}");
    let (status, _type, page) = app.get(&format!("/s/{token}"));
    assert_eq!(status, 200);
    assert!(page.contains("154 kcal per 100 g"), "{page}");
    assert!(!page.contains("kcal a serving"), "the basis is not guessed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_share_link_is_not_a_key_to_the_instance() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_status, shared_recipe) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Shared" }).to_string(),
    );
    let shared_branch = shared_recipe["result"]["branch_id"]
        .as_str()
        .unwrap()
        .to_string();
    let (_status, private_recipe) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Kept to myself" }).to_string(),
    );
    let private_branch = private_recipe["result"]["branch_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (token, _url) = share(&app, &key, &shared_branch);

    // The other recipe is not reachable by holding this token — the whole of
    // the second visibility level is *this* recipe (ADR 0026).
    let (status, _type, page) = app.get(&format!("/s/{token}"));
    assert_eq!(status, 200);
    assert!(!page.contains("Kept to myself"), "{page}");

    // An unshared recipe has no page at all, and a token nobody minted is not
    // found rather than being coy about it.
    let (status, _type, _page) = app.get(&format!("/s/{private_branch}"));
    assert_eq!(status, 404);
    let (status, _type, _page) =
        app.get("/s/0000000000000000000000000000000000000000000000000000000000000000");
    assert_eq!(status, 404);

    // A stranger still cannot read the recipe as an Operation.
    let (status, refused) = app.post_op(
        "get_recipe",
        None,
        &json!({ "branch_id": shared_branch }).to_string(),
    );
    assert_eq!(status, 401, "{refused}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn only_the_kitchen_holding_a_recipe_may_share_it() {
    let app = support::spawn_app();
    let (_mine, my_key, my_kitchen) = person_with_kitchen(&app, "Aurélien");
    let (_theirs, their_key, _their_kitchen) = person_with_kitchen(&app, "Marc");

    let (_status, created) = app.post_op(
        "create_recipe",
        Some(&my_key),
        &json!({ "kitchen_id": my_kitchen, "title": "Mine" }).to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    // Neither refusal says the recipe is here: the Kitchen was worked out from
    // the Branch id, so a non-member gets the plain not-found (ADR 0040).
    let (status, refused) = app.post_op(
        "share_recipe",
        Some(&their_key),
        &json!({ "branch_id": branch_id, "public_address": "https://kamosu.example" }).to_string(),
    );
    assert_eq!(status, 404, "{refused}");
    let (status, refused) = app.post_op(
        "end_share_link",
        Some(&their_key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 404, "{refused}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_share_link_is_a_token_so_moving_the_instance_does_not_break_it() {
    let app = support::spawn_app();
    // Moving the instance is the Operator's act, so this one needs the
    // Operator rather than any Person.
    let (key, kitchen_id) = operator_with_kitchen(&app);
    let (_status, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Moved house" }).to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();
    let (token, url) = share(&app, &key, &branch_id);
    assert!(url.starts_with("https://kamosu.example/s/"));

    // The Operator moves the instance. What was stored is a token, so the link
    // already minted renders against the new address rather than the old one.
    let (status, moved) = app.post_op(
        "set_public_address",
        Some(&key),
        &json!({ "public_address": "https://cuisine.example/" }).to_string(),
    );
    assert_eq!(status, 200, "{moved}");
    // A trailing slash is one spelling, not two.
    assert_eq!(
        moved["result"]["public_address"],
        json!("https://cuisine.example")
    );

    let (status, _type, page) = app.get(&format!("/s/{token}"));
    assert_eq!(status, 200);
    assert!(
        page.contains(&format!("https://cuisine.example/s/{token}/card")),
        "the card and the canonical URL follow the address: {page}"
    );

    // An address that is not one is refused rather than stored.
    let (status, refused) = app.post_op(
        "set_public_address",
        Some(&key),
        &json!({ "public_address": "cuisine.example" }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_card_a_messaging_app_fetches_is_a_real_picture() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_status, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Braised Chicken in Red Wine" }).to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();
    let (token, _url) = share(&app, &key, &branch_id);

    let (status, content_type, bytes) = app.get_bytes(&format!("/s/{token}/card"), None);
    assert_eq!(status, 200);
    assert_eq!(content_type, "image/png");
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "a real PNG");
    assert!(bytes.len() > 5_000, "a drawn card, not an empty canvas");

    // The page points a messaging app at it, and names the person rather than
    // the recipe in og:title — the recipe's name is on the picture (#65).
    let (_status, _type, page) = app.get(&format!("/s/{token}"));
    assert!(page.contains(&format!("/s/{token}/card")), "{page}");
    assert!(page.contains(r#"og:title" content="Shared by Aurélien"#));
    assert!(!page.contains(r#"og:title" content="Braised Chicken"#));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_shared_recipes_translations_are_readable_under_the_same_token() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_status, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Shortbread",
            "ingredients": [{ "kind": "ingredient", "text": "250 g butter" }],
        })
        .to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();
    let (status, translated) = app.post_op(
        "start_translation",
        Some(&key),
        &json!({ "branch_id": branch_id, "language": "fr", "title": "Sablés" }).to_string(),
    );
    assert_eq!(status, 200, "{translated}");
    let french = translated["result"]["branch_id"]
        .as_str()
        .unwrap()
        .to_string();
    let (status, saved) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": french,
            "title": "Sablés",
            "ingredients": [{ "kind": "ingredient", "text": "250 g de beurre" }],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{saved}");

    let (token, _url) = share(&app, &key, &branch_id);

    // The shared Branch itself, in English, offering its Translation.
    let (status, _type, page) = app.get(&format!("/s/{token}"));
    assert_eq!(status, 200);
    assert!(page.contains("Shortbread"), "{page}");
    assert!(page.contains(r#"lang="en""#), "served in its own Language");
    assert!(
        page.contains(&format!("/s/{token}/in/fr")),
        "the Translation is a link, not just a name: {page}"
    );

    // And the Translation is genuinely readable — under the same token, since
    // a token of its own would be a second link to end.
    let (status, _type, french_page) = app.get(&format!("/s/{token}/in/fr"));
    assert_eq!(status, 200);
    assert!(french_page.contains("Sablés"), "{french_page}");
    assert!(french_page.contains("250 g de beurre"));
    assert!(french_page.contains(r#"lang="fr""#), "and in French");
    // Its own words, not the English ones.
    assert!(french_page.contains("Ingrédients"));
    assert!(!french_page.contains(">Ingredients<"));

    // A Language nobody wrote this in falls back to the recipe rather than
    // failing: a hand-edited URL is a mistype, not an attack.
    let (status, _type, fallback) = app.get(&format!("/s/{token}/in/de"));
    assert_eq!(status, 200);
    assert!(fallback.contains("Shortbread"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_card_is_drawn_once_and_kept() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_status, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Drawn once" }).to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();
    let (token, _url) = share(&app, &key, &branch_id);

    // A stranger may cause work, never work that scales with them (ADR 0032):
    // asking twice draws once and serves the same bytes.
    let (first_status, _type, first) = app.get_bytes(&format!("/s/{token}/card"), None);
    let (second_status, _type, second) = app.get_bytes(&format!("/s/{token}/card"), None);
    assert_eq!(first_status, 200);
    assert_eq!(second_status, 200);
    assert_eq!(first, second, "the same card, byte for byte");

    // And it really is kept, rather than redrawn identically each time.
    let version_id = created["result"]["head_version_id"].as_str().unwrap();
    let kept = app
        .data_dir()
        .expect("this test owns its data directory")
        .join("cards")
        .join(format!("{version_id}.png"));
    assert!(kept.exists(), "the card was not kept at {kept:?}");
}

/// **A stranger holding a Share Link takes the recipe file** (#65, #66, ADR
/// 0020). No account and no Credential: the token is the whole of the
/// permission, so the file comes from an address of its own rather than from
/// `GET /api/bundles/<branch_id>`, which a stranger cannot reach.
///
/// It is the same Bundle the app hands a member, Passengers and Translations
/// included, it is built once and kept (ADR 0032), and an ended link hands
/// over nothing at all.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stranger_with_a_share_link_takes_the_recipe_file() {
    let app = support::spawn_app();
    let (key, _kitchen, pizza, _lineage, _french, _dough, photos) = a_pizza_worth_sending(&app);
    let (token, _url) = share(&app, &key, &pizza);

    // The page offers it, as a link: this page runs no script.
    let (status, _, page) = app.get(&format!("/s/{token}"));
    assert_eq!(status, 200);
    assert!(
        page.contains(&format!("href=\"/s/{token}/bundle\"")),
        "the page offers the recipe file: {page}"
    );

    let asked = kamosu::http_min::get(app.addr, &format!("/s/{token}/bundle")).expect("a reply");
    assert_eq!(asked.status, 200);
    let header = |name: &str| {
        asked
            .headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.clone())
            .unwrap_or_default()
    };
    assert_eq!(header("content-type"), "application/zip");
    assert!(
        header("content-disposition").contains("attachment"),
        "it is a download: {:?}",
        header("content-disposition")
    );
    assert!(
        header("content-disposition").contains("Pizza"),
        "named after the recipe: {:?}",
        header("content-disposition")
    );

    // The same Bundle a member gets: one file, two ways in. Compared by what
    // is inside rather than by the zip's own bytes, which carry the moment
    // each was written.
    let mine = bundle_of(&app, &key, &pizza);
    assert_eq!(
        unzip(&asked.body),
        unzip(&mine),
        "a stranger's file holds exactly what a member's does, name for name and byte for byte"
    );
    let files = unzip(&asked.body);
    assert!(
        files.contains_key("Neapolitan Pizza Dough.md"),
        "the Passenger travels: {:?}",
        files.keys().collect::<Vec<_>>()
    );
    assert!(
        files
            .keys()
            .any(|name| name.ends_with(".md") && name.contains("Pizza Margherita")),
        "the recipe itself travels: {:?}",
        files.keys().collect::<Vec<_>>()
    );
    assert!(
        files.keys().any(|name| name.starts_with("photographs/")),
        "the Photographs travel: {:?}",
        files.keys().collect::<Vec<_>>()
    );
    assert_eq!(photos.len(), 2, "the fixture has two photographs");

    // Built once and kept (ADR 0032): a link is held by however many people it
    // was passed to, and zipping every Photograph per reader is work that
    // scales with strangers.
    let again = kamosu::http_min::get(app.addr, &format!("/s/{token}/bundle")).expect("a reply");
    assert_eq!(again.body, asked.body, "the same file, byte for byte");
    let kept: Vec<_> = std::fs::read_dir(
        app.data_dir()
            .expect("this test owns its data directory")
            .join("bundles"),
    )
    .expect("the Bundles directory")
    .flatten()
    .collect();
    assert_eq!(kept.len(), 1, "built once, kept once");

    // Ending the link ends the file with it. Withdrawing a link stops anyone
    // new arriving, and taking the recipe away is arriving (ADR 0018).
    let (status, ended) = app.post_op(
        "end_share_link",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    assert_eq!(status, 200, "{ended}");
    let refused = kamosu::http_min::get(app.addr, &format!("/s/{token}/bundle")).expect("a reply");
    assert_eq!(refused.status, 404);
    assert!(
        !refused.headers.iter().any(|(name, value)| name
            .eq_ignore_ascii_case("content-type")
            && value.contains("zip")),
        "the ended link hands over no file"
    );
    assert!(
        !String::from_utf8_lossy(&refused.body).contains("Pizza Margherita"),
        "and leaks nothing of the recipe"
    );
}

// --- Nutrition (issue #72) ---------------------------------------------------
//
// v1's whole of nutrition: one figure the cook types onto the recipe, with a
// flag saying what it counts — a serving, or 100 g. Kamosu never works it out
// from the Ingredient Lines, because a plausible-but-wrong calorie figure is
// worse than no figure at all (#12 defers CIQUAL and the compute button past
// v1). The figure is one of the recipe's own words, so it rides in the
// fingerprint exactly as the Yield and the Note do (ADR 0021).

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_recipe_carries_a_typed_nutrition_figure_with_the_basis_it_counts() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    // Typed at creation, per serving.
    let (status, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Chocolate chunk cookies",
            "nutrition": { "calories": 308, "basis": "per_serving" },
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{created}");
    assert_eq!(
        created["result"]["versions"][0]["content"]["nutrition"],
        json!({ "calories": 308.0, "basis": "per_serving" })
    );

    // Retyped as a per-100 g figure, which is the other thing a packet says.
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();
    backdate_branch_head(&app, &branch_id);
    let (status, saved) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": branch_id,
            "title": "Chocolate chunk cookies",
            "nutrition": { "calories": 481.5, "basis": "per_100g" },
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{saved}");

    let (_, read_back) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(
        read_back["result"]["versions"][1]["content"]["nutrition"],
        json!({ "calories": 481.5, "basis": "per_100g" })
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn changing_only_the_nutrition_figure_mints_a_version() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Soupe",
            "nutrition": { "calories": 120, "basis": "per_serving" },
        })
        .to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();
    let first_version = created["result"]["head_version_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Nothing but the figure moves. It is still a new Version: the figure is
    // one of the recipe's own words, so the fingerprint covers it exactly as
    // it covers the Note (ADR 0021).
    backdate_branch_head(&app, &branch_id);
    let (_, saved) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": branch_id,
            "title": "Soupe",
            "nutrition": { "calories": 121, "basis": "per_serving" },
        })
        .to_string(),
    );
    assert_ne!(saved["result"]["version_id"], json!(first_version));

    // And the basis alone is enough on its own: 120 per serving and 120 per
    // 100 g are two different claims about the same dish.
    backdate_branch_head(&app, &branch_id);
    let second_version = saved["result"]["version_id"].as_str().unwrap().to_string();
    let (_, again) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": branch_id,
            "title": "Soupe",
            "nutrition": { "calories": 121, "basis": "per_100g" },
        })
        .to_string(),
    );
    assert_ne!(again["result"]["version_id"], json!(second_version));

    // Clearing it is a change too, and leaves an ordinary recipe behind
    // rather than an error: a recipe with no nutrition figure is the state
    // every recipe starts in.
    backdate_branch_head(&app, &branch_id);
    let (status, cleared) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": branch_id, "title": "Soupe" }).to_string(),
    );
    assert_eq!(status, 200, "{cleared}");
    assert_ne!(
        cleared["result"]["version_id"],
        again["result"]["version_id"]
    );

    let (_, read_back) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(
        read_back["result"]["versions"][3]["content"]["nutrition"],
        Value::Null
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nothing_computes_nutrition_from_the_ingredient_lines_or_the_foods_they_name() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    // Ingredient Lines that read cleanly — quantity, Unit and Food all found,
    // which is the state a computing importer would have everything it needed
    // for. v1 still stores no figure: CIQUAL and the compute button are
    // deferred past v1 (#12), and a figure invented here would be wrong in a
    // way nobody could see.
    let (status, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Plain Loaf",
            "ingredients": [
                { "kind": "ingredient", "text": "500 g strong white flour" },
                { "kind": "ingredient", "text": "300 ml water" },
                { "kind": "ingredient", "text": "10 g salt" },
            ],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{created}");
    let content = &created["result"]["versions"][0]["content"];
    assert_eq!(
        content["nutrition"],
        Value::Null,
        "three lines Kamosu read perfectly still produce no nutrition figure"
    );

    // The Readings really did land — so the emptiness above is a decision,
    // not an unread recipe quietly failing to give a computation its input.
    let readings = created["result"]["versions"][0]["readings"]
        .as_array()
        .expect("a Reading slot per Ingredient Line");
    assert_eq!(readings.len(), 3, "{created}");
    assert!(
        readings.iter().all(|reading| !reading.is_null()),
        "every line here carries a Reading: {readings:?}"
    );

    // And the Foods those Readings made carry the slot the deferred CIQUAL
    // binding will one day fill, empty (CONTEXT.md, "Food").
    let (status, foods) = app.post_op("list_foods", Some(&key), "{}");
    assert_eq!(status, 200, "{foods}");
    let listed = foods["result"]["foods"].as_array().expect("foods");
    assert!(!listed.is_empty(), "{foods}");
    for food in listed {
        assert_eq!(
            food["nutrition"],
            Value::Null,
            "a Food's nutrition is never computed either: {food}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_nutrition_figure_that_does_not_say_what_it_counts_is_refused() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Soupe" }).to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    // A number with no basis is not a fact about the dish, so it is refused
    // rather than defaulted: guessing "per serving" would silently mislabel
    // every packet figure somebody typed off the side of a box.
    for bad in [
        json!({ "calories": 308 }),
        json!({ "calories": 308, "basis": "per_portion" }),
        json!({ "basis": "per_serving" }),
        json!({ "calories": -1, "basis": "per_serving" }),
        json!({ "calories": "308", "basis": "per_serving" }),
        json!(308),
    ] {
        let (status, refused) = app.post_op(
            "save_recipe_version",
            Some(&key),
            &json!({ "branch_id": branch_id, "title": "Soupe", "nutrition": bad }).to_string(),
        );
        assert_eq!(status, 400, "{bad} was accepted: {refused}");
    }
}

/// The other half of "a Food's nutrition never travels" (#72, spec item 161),
/// held at the Catalogue. The real Bundle (#66) is proved separately, by
/// `a_bundle_carries_the_recipe_its_translation_its_passenger_and_its_photographs`.
///
/// **What #66 decided, explicitly.** A Bundle does carry Foods, as ADR 0021
/// and spec item 159 require: where a Reading resolved to a Food, the sidecar
/// puts that Food's names, in every Language it has one in, beside the Reading.
/// It carries no Food id, no Cup Weight and no Food nutrition. It is gathered
/// by `bundle_readings` in `src/core.rs` from the `readings` table, not from
/// any shape this test reads, so nothing here moved: the Catalogue's Reading
/// still names no Food.
///
/// A Bundle is one recipe's worth of Vault (ADR 0020), so what it can carry is
/// bounded by what a recipe declares: its Versions' content, and the Readings
/// beside them. The property that keeps this instance's Food knowledge out of
/// a file somebody else opens is that **neither of those two shapes reaches a
/// Food at all** — a Version's content carries the recipe's own words, and a
/// Reading carries the three words it read off one line and no pointer to the
/// Food they resolve to (ADR 0019: nothing is stapled to a line).
///
/// Asserting it here means #66 inherits the guarantee instead of re-deciding
/// it, and a field added later that would carry a Food's Cup Weight or its
/// nutrition into a Bundle fails this test rather than a review. The recipe's
/// own Nutrition figure is deliberately not covered: that one is a word of the
/// recipe and travels with it, which is exactly the distinction CONTEXT.md
/// draws between the two things called nutrition.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn what_a_bundle_could_carry_never_reaches_a_food() {
    let catalogue = kamosu::catalogue::declarations();
    let recipe = catalogue
        .as_array()
        .expect("the Catalogue is a list of declarations")
        .iter()
        .find(|op| op["name"] == json!("get_recipe"))
        .expect("get_recipe is declared");
    let version = &recipe["output_schema"]["properties"]["versions"]["items"]["properties"];

    // A Reading names no Food. This is the only edge that could lead from a
    // recipe to one, and it does not exist.
    //
    // It names a **Lineage**, and that is not the same hole. ADR 0021 says so
    // in as many words: a Food travels as its names and no id, because two
    // instances mint their *farine* separately; a Lineage is global by
    // construction (ADR 0004), so it means the same thing everywhere and
    // carries nothing this instance learned about anything.
    let reading: Vec<&String> = version["readings"]["items"]["properties"]
        .as_object()
        .expect("a Reading declares its fields")
        .keys()
        .collect();
    assert_eq!(
        reading,
        ["amount", "lineage_id", "target", "unit"],
        "a Reading declares something beyond what it read off the line and the \
         Recipe it may name: {reading:?}"
    );

    // And no shape a Bundle could carry declares anything a Food holds — the
    // Component's own content and Readings included, since unfolding puts a
    // second recipe's shapes inside the first one's answer (#50, ADR 0008).
    let component = &version["components"]["items"]["properties"];
    for (where_, properties) in [
        ("a Version's content", &version["content"]["properties"]),
        ("a Reading", &version["readings"]["items"]["properties"]),
        ("a Component's content", &component["content"]["properties"]),
        (
            "a Component's Readings",
            &component["readings"]["items"]["properties"],
        ),
        ("a Component", component),
    ] {
        let fields: Vec<&String> = properties
            .as_object()
            .unwrap_or_else(|| panic!("{where_} declares its fields"))
            .keys()
            .collect();
        for forbidden in ["food", "food_id", "foods", "cup_weight_grams"] {
            assert!(
                !fields.iter().any(|field| field.as_str() == forbidden),
                "{where_} declares '{forbidden}': a Bundle would carry what this \
                 instance learned about a Food. Fields: {fields:?}"
            );
        }
    }

    // The Food's own nutrition slot is reachable only through the Operations
    // that read this instance's own Foods — and it is empty there too, since
    // nothing in v1 writes it (the CIQUAL binding is deferred past v1, #12).
    let get_food = catalogue
        .as_array()
        .unwrap()
        .iter()
        .find(|op| op["name"] == json!("get_food"))
        .expect("get_food is declared");
    assert_eq!(
        get_food["output_schema"]["properties"]["nutrition"],
        json!({ "type": "null" }),
        "a Food's nutrition is declared as the empty slot it is in v1"
    );
}

/// A recipe written before a field existed still answers the shape the
/// Catalogue declares (#72).
///
/// A Version is immutable and named by the fingerprint of its own bytes
/// (ADR 0004, ADR 0021), so `nutrition` — the first field ever added to a
/// Recipe's content — could not be written into the rows already stored: that
/// would re-fingerprint the library. The rows therefore stay as they were
/// written, and the field is supplied on the way out. Without that, every
/// recipe written before this change answers content missing a field the
/// Catalogue says is required, and the generated client is handed a shape its
/// own declaration forbids.
///
/// This test writes the pre-change row directly, because that is the only way
/// to have one: nothing in the code path can produce a Version without the
/// field any more.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_version_written_before_a_field_existed_still_answers_the_declared_shape() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Written Before Nutrition" }).to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();
    let version_id = created["result"]["head_version_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Age the stored row back to what it would have held before #72: the same
    // content with the key simply absent. The Version keeps its id, exactly as
    // a real pre-change row does — the fingerprint is not recomputed.
    app.core
        .db()
        .with_conn(|conn| {
            let stored: String = conn
                .query_row(
                    "SELECT content FROM versions WHERE id = ?1",
                    rusqlite::params![version_id],
                    |row| row.get(0),
                )
                .unwrap();
            let mut content: Value = serde_json::from_str(&stored).unwrap();
            content.as_object_mut().unwrap().remove("nutrition");
            conn.execute(
                "UPDATE versions SET content = ?1 WHERE id = ?2",
                rusqlite::params![content.to_string(), version_id],
            )
            .unwrap();
            Ok(())
        })
        .expect("aged the row");

    let (status, read_back) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 200, "{read_back}");
    let content = &read_back["result"]["versions"][0]["content"];
    assert!(
        content.get("nutrition").is_some(),
        "a Version written before the field existed answers without it: {content}"
    );
    assert_eq!(content["nutrition"], Value::Null);

    // Every other declared field is answered too, and the Version keeps the id
    // it was fingerprinted under.
    for declared in [
        "title",
        "yield",
        "prep_time_minutes",
        "cook_time_minutes",
        "note",
        "main_photo",
        "source",
        "nutrition",
        "ingredients",
        "steps",
    ] {
        assert!(
            content.get(declared).is_some(),
            "content is missing declared field '{declared}': {content}"
        );
    }
    assert_eq!(
        read_back["result"]["versions"][0]["version_id"],
        json!(version_id),
        "supplying the field on the way out must not re-fingerprint the Version"
    );
}

// --- The Shopping List (issue #73, ADR 0024) ---------------------------------

/// Two real recipes from Aurélien's Crouton corpus, cut to the lines that make
/// a Shopping List interesting, and chosen at once.
///
/// Every line below is verbatim from `samples/crouton/`. Between them they
/// produce all four states one row can be in: an amount that added and says
/// *about*, two amounts that will not add and ride side by side, a line
/// carrying no quantity that rides as *some*, and a line with no Food to merge
/// under at all.
fn two_real_recipes(app: &support::TestApp, key: &str, kitchen_id: &str) -> (String, String) {
    let (_, chicken) = app.post_op(
        "create_recipe",
        Some(key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Korean Fried Chicken",
            "yield": { "amount": "4", "noun": "servings" },
            "ingredients": [
                { "kind": "ingredient", "text": "2 tbsp minced garlic" },
                { "kind": "ingredient", "text": "2 tbsp soy sauce" },
                { "kind": "ingredient", "text": "Some cooking oil (for deep frying)" },
            ],
        })
        .to_string(),
    );
    let (_, coq) = app.post_op(
        "create_recipe",
        Some(key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Braised Chicken in Red Wine (Coq au Vin)",
            "yield": { "amount": "4", "noun": "servings" },
            "ingredients": [
                { "kind": "section", "text": "For the braise" },
                { "kind": "ingredient", "text": "4 cloves minced garlic" },
                { "kind": "ingredient", "text": "1 tbsp soy sauce" },
                { "kind": "ingredient", "text": "olive oil" },
            ],
        })
        .to_string(),
    );
    (
        chicken["result"]["branch_id"].as_str().unwrap().to_string(),
        coq["result"]["branch_id"].as_str().unwrap().to_string(),
    )
}

/// One row off a list, by the name it carries.
fn row<'a>(list: &'a Value, name: &str) -> &'a Value {
    list["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .find(|row| row["name"] == json!(name))
        .unwrap_or_else(|| panic!("no row named {name} in {list}"))
}

/// What one row's amounts say, in order.
fn amounts(row: &Value) -> Vec<String> {
    row["parts"]
        .as_array()
        .expect("parts")
        .iter()
        .map(|part| part["text"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_list_leaves_as_text_under_a_header_line_and_kamosu_lets_go_of_it() {
    // ADR 0024's other end. Nothing is ticked inside Kamosu *because* the list
    // leaves and something else holds the ticks, so a list with no way out
    // would have made the missing tick a refusal rather than a boundary.
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );
    let (chicken, coq) = two_real_recipes(&app, &key, &kitchen_id);
    for branch_id in [&chicken, &coq] {
        app.post_op(
            "add_to_shopping_list",
            Some(&key),
            &json!({ "branch_id": branch_id }).to_string(),
        );
    }
    app.post_op(
        "add_loose_item",
        Some(&key),
        &json!({ "text": "bin bags" }).to_string(),
    );

    let (status, sent) = app.post_op("shopping_list_as_text", Some(&key), "{}");
    assert_eq!(status, 200, "{sent}");
    let text = sent["result"]["text"].as_str().expect("text").to_string();
    let mut lines = text.lines();

    // The header line is a divider before it is a label: the note accumulates,
    // and three trips appended with no divider are a wall.
    let header = lines.next().expect("a header line");
    let today: String = header.split(' ').next().unwrap().to_string();
    assert_eq!(
        today.len(),
        10,
        "the header opens with a date, in {header:?}"
    );
    assert!(
        header.contains("Korean Fried Chicken")
            && header.contains("Braised Chicken in Red Wine (Coq au Vin)"),
        "the header names the recipes it was built from, in {header:?}"
    );

    // Every line under the header is one thing to buy, as a Markdown checklist
    // line (#74): a Shortcut hands it to Notes, where each line becomes a
    // checkbox, so a line that was not a thing to buy would be a box to tick.
    let checklist: Vec<&str> = lines
        .map(|line| {
            line.strip_prefix("- [ ] ")
                .unwrap_or_else(|| panic!("{line:?} is not a checklist line, in {text:?}"))
        })
        .collect();

    // A row that added says one amount; the row that could not says both, on
    // its one line, each naming the dish that wanted it — which is the whole
    // answer to *which dish goes short* travelling out of Kamosu with the list.
    assert!(
        checklist.contains(&"soy sauce — about 45 ml"),
        "an added row carries its one amount, in {text:?}"
    );
    let garlic = checklist
        .iter()
        .find(|line| line.starts_with("minced garlic — "))
        .unwrap_or_else(|| panic!("the garlic row stays on one line, in {text:?}"));
    assert!(
        garlic.contains("about 30 ml for Korean Fried Chicken")
            && garlic.contains("; 4 cloves for "),
        "each amount is labelled with the dish that wanted it, in {garlic:?}"
    );

    // A line typed by hand goes over exactly as typed, with no amount slot,
    // in the same alphabetical list as everything else — no block of its own.
    assert!(
        checklist.contains(&"bin bags"),
        "a Loose Item travels whole and alone, in {text:?}"
    );
    let names: Vec<String> = checklist
        .iter()
        .map(|line| line.split(" — ").next().unwrap().to_lowercase())
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "one flat alphabetical list, in {text:?}");

    // Reading it changed nothing: Kamosu offers to empty and does not act.
    // Declining the offer is simply not asking, so the list is still whole —
    // both recipes and the typed line — and sending again sends the same.
    let (_, after) = app.post_op("get_shopping_list", Some(&key), "{}");
    assert_eq!(
        after["result"]["chosen"].as_array().unwrap().len(),
        2,
        "sending the list does not empty it"
    );
    assert!(
        after["result"]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["name"] == "bin bags"),
        "and it keeps the typed line too"
    );
    let (_, again) = app.post_op("shopping_list_as_text", Some(&key), "{}");
    assert_eq!(
        again["result"]["text"], sent["result"]["text"],
        "a list that was declined sends the same the second time"
    );

    let (status, emptied) = app.post_op("empty_shopping_list", Some(&key), "{}");
    assert_eq!(status, 200, "{emptied}");
    assert_eq!(
        emptied["result"],
        json!({ "chosen": [], "rows": [] }),
        "and the offer, once taken, takes the choosing and the typed lines together"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn choosing_a_recipe_already_on_the_list_moves_its_yield_rather_than_ignoring_it() {
    // The Catalogue promises "choose a recipe to shop for, at a Yield", and
    // that choosing one already on the list is not an error. Silently keeping
    // the old figure would shop for four while saying nothing.
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );
    let (chicken, _coq) = two_real_recipes(&app, &key, &kitchen_id);

    app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": chicken }).to_string(),
    );
    let (_, as_written) = app.post_op("get_shopping_list", Some(&key), "{}");
    assert_eq!(
        amounts(row(&as_written["result"], "soy sauce")),
        vec!["about 30 ml"],
        "the recipe as written, for four"
    );

    // Add it again, for eight. One entry, and the amounts move.
    let (status, doubled) = app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": chicken, "shopping_yield": { "amount": "8", "noun": "servings" } })
            .to_string(),
    );
    assert_eq!(status, 200, "{doubled}");
    assert_eq!(
        doubled["result"]["chosen"].as_array().unwrap().len(),
        1,
        "choosing it twice makes no second entry"
    );
    assert_eq!(
        doubled["result"]["chosen"][0]["shopping_yield"],
        json!({ "amount": "8", "noun": "servings" }),
        "and the Yield asked for is the Yield stored"
    );
    assert_eq!(
        amounts(row(&doubled["result"], "soy sauce")),
        vec!["about 60 ml"],
        "every amount it contributes moves with it"
    );

    // And adding it once more with no Yield puts it back to the recipe as
    // written, which is what asking for it with no Yield means.
    let (_, plain) = app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": chicken }).to_string(),
    );
    assert_eq!(plain["result"]["chosen"][0]["shopping_yield"], json!(null));
    assert_eq!(
        amounts(row(&plain["result"], "soy sauce")),
        vec!["about 30 ml"]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_shopping_row_merges_every_mention_of_one_food_and_says_what_it_cannot_add() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );
    let (chicken, coq) = two_real_recipes(&app, &key, &kitchen_id);

    // The list starts empty and is always there: nobody creates one.
    let (status, empty) = app.post_op("get_shopping_list", Some(&key), "{}");
    assert_eq!(status, 200, "{empty}");
    assert_eq!(empty["result"], json!({ "chosen": [], "rows": [] }));

    app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": chicken }).to_string(),
    );
    let (status, list) = app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": coq }).to_string(),
    );
    assert_eq!(status, 200, "{list}");
    let list = &list["result"];

    // **Amounts add where the Units honestly convert, saying about.** Two
    // tablespoons of soy sauce and one more make three, in this reader's
    // measures — one row, one number, out of two recipes.
    assert_eq!(amounts(row(list, "soy sauce")), vec!["about 45 ml"]);
    assert_eq!(
        row(list, "soy sauce")["parts"][0]["sources"],
        json!([
            "Korean Fried Chicken",
            "Braised Chicken in Red Wine (Coq au Vin)"
        ]),
        "a row says which recipes fed each of its amounts"
    );

    // **Where they do not convert, the row carries both** — two true amounts
    // beat one wrong one. Nothing here turns cloves into millilitres.
    assert_eq!(
        amounts(row(list, "minced garlic")),
        vec!["about 30 ml", "4 cloves"]
    );

    // **A line with no amount contributes some rather than being dropped** —
    // 28% of real Ingredient Lines carry no quantity (#5).
    let oil = row(list, "olive oil");
    assert_eq!(amounts(oil), vec!["some"]);
    assert_eq!(oil["parts"][0]["kind"], json!("no_amount"));

    // **The written lines are always one tap away** (ADR 0002, ADR 0019),
    // whole and unrewritten, each under the recipe it came from.
    assert_eq!(
        row(list, "minced garlic")["lines"],
        json!([
            {
                "branch_id": chicken,
                "recipe": "Korean Fried Chicken",
                "text": "2 tbsp minced garlic",
            },
            {
                "branch_id": coq,
                "recipe": "Braised Chicken in Red Wine (Coq au Vin)",
                "text": "4 cloves minced garlic",
            },
        ])
    );

    // A Section heads a list of ingredients; it is not a thing to buy.
    assert!(
        !list["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["name"] == json!("For the braise")),
        "a Section is not a Shopping Row"
    );

    // **Nothing is ticked inside Kamosu** (ADR 0024): a computed row has no
    // name to staple a tick to, so no answer here carries one.
    assert!(
        !list.to_string().contains("ticked"),
        "a Shopping Row carries nothing to tick"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_rows_are_computed_every_time_so_editing_a_recipe_changes_the_list_at_once() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );
    let (chicken, _coq) = two_real_recipes(&app, &key, &kitchen_id);
    app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": chicken }).to_string(),
    );

    // Correcting a Reading makes no Version and enters no history (ADR 0021),
    // and the list simply says something else next time it is read — which a
    // stored row could not do without going stale first.
    app.post_op(
        "set_reading",
        Some(&key),
        &json!({
            "branch_id": chicken,
            "line_index": 1,
            "amount": "500",
            "unit": "ml",
            "target": "soy sauce",
        })
        .to_string(),
    );
    let (_, list) = app.post_op("get_shopping_list", Some(&key), "{}");
    assert_eq!(
        amounts(row(&list["result"], "soy sauce")),
        vec!["about 500 ml"]
    );

    // A new Version of the recipe reaches the list too: the choosing holds a
    // Branch at its LATEST Version, never a pinned one (ADR 0024), so a recipe
    // edited between the planning and the shopping is right in the shop.
    app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": chicken,
            "title": "Korean Fried Chicken",
            "yield": { "amount": "4", "noun": "servings" },
            "ingredients": [
                { "kind": "ingredient", "text": "2 tbsp minced garlic" },
                { "kind": "ingredient", "text": "2 tbsp soy sauce" },
                { "kind": "ingredient", "text": "Some cooking oil (for deep frying)" },
                { "kind": "ingredient", "text": "300 g potato starch" },
            ],
        })
        .to_string(),
    );
    let (_, list) = app.post_op("get_shopping_list", Some(&key), "{}");
    assert_eq!(
        amounts(row(&list["result"], "potato starch")),
        vec!["about 300 g"]
    );
    assert_eq!(
        amounts(row(&list["result"], "soy sauce")),
        vec!["about 500 ml"],
        "a Reading is carried onto each new Version, so a correction is not undone by an edit"
    );
}

/// A value outside a declared `enum` is refused at both Doors, the same way an
/// undeclared field is (#85).
///
/// It lives here rather than in `tests/parity.rs` for one reason: shape is
/// checked *after* authorisation, so an Operation declaring an `enum` — every
/// one of them needs a Person — answers 401 before it ever reaches the check.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_value_outside_a_declared_enum_is_refused_at_both_doors() {
    let app = support::spawn_app();
    let (_person, key, _kitchen_id) = person_with_kitchen(&app, "Aurélien");

    let outside = json!({ "reading_language": "de", "reading_measures": "us" });

    let (status, refusal) =
        app.post_op("set_reading_preferences", Some(&key), &outside.to_string());
    assert_eq!(status, 400, "the web Door accepted it: {refusal}");
    assert_eq!(refusal["error"]["kind"], "bad_request");
    assert!(
        refusal["error"]["message"]
            .as_str()
            .expect("a message")
            .contains("reading_language"),
        "the refusal does not name the field: {refusal}"
    );

    let call = |arguments: Value| {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": "set_reading_preferences", "arguments": arguments },
        });
        app.post_mcp(&payload.to_string(), Some(&key)).1
    };

    let refused = call(outside);
    assert_eq!(
        refused["result"]["isError"],
        json!(true),
        "the MCP Door accepted it: {refused}"
    );
    assert!(
        refused["result"]["content"][0]["text"]
            .as_str()
            .expect("a message")
            .contains("reading_language"),
        "the MCP refusal does not name the field: {refused}"
    );

    // A Language the Catalogue does declare still lands, at both Doors.
    let (status, body) = app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "fr", "reading_measures": "metric" }).to_string(),
    );
    assert_eq!(status, 200, "a declared value was refused: {body}");
    let accepted = call(json!({ "reading_language": "es", "reading_measures": "us" }));
    assert_eq!(
        accepted["result"]["isError"],
        json!(false),
        "a declared value was refused at the MCP door: {accepted}"
    );
}

/// The failure #85 was opened for: a misspelt Yield used to erase the Yield and
/// report success.
///
/// `shopping_yield` is optional, and its absence legitimately means *back to
/// the recipe as written* — so before the Catalogue's declaration was enforced,
/// a typo and a deliberate reset were the same request. An assistant asked to
/// shop for eight got a cheerful OK and a list for four. That is ADR 0024's own
/// warning from the other end: a thing that quietly disappears from a shopping
/// list is a thing that does not get bought.
///
/// A misspelt *required* field never had this problem — the handler's own check
/// for the missing field caught it by accident. The danger was only ever an
/// optional field whose absence carries meaning.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_misspelt_yield_is_refused_rather_than_erasing_the_one_stored() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (chicken, _coq) = two_real_recipes(&app, &key, &kitchen_id);

    app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": chicken, "shopping_yield": { "amount": "8", "noun": "servings" } })
            .to_string(),
    );

    // The exact call from the ticket: `shopping_yield` misspelt back to what
    // the field used to be called.
    let (status, refusal) = app.post_op(
        "set_shopping_yield",
        Some(&key),
        &json!({ "branch_id": chicken, "yield": { "amount": "4", "noun": "servings" } })
            .to_string(),
    );
    assert_eq!(status, 400, "the misspelling was accepted: {refusal}");
    assert_eq!(refusal["error"]["kind"], "bad_request");
    assert!(
        refusal["error"]["message"]
            .as_str()
            .expect("a message")
            .contains("yield"),
        "the refusal does not name the offending field: {refusal}"
    );

    // And — the half that matters — the stored Yield is untouched.
    let (_, list) = app.post_op("get_shopping_list", Some(&key), "{}");
    assert_eq!(
        list["result"]["chosen"][0]["shopping_yield"],
        json!({ "amount": "8", "noun": "servings" }),
        "the Yield was erased by a call that was refused"
    );

    // Spelt as the Catalogue declares it, the same reset still works — the
    // point is that absence must be *asked for*, not arrived at by typo.
    let (status, list) = app.post_op(
        "set_shopping_yield",
        Some(&key),
        &json!({ "branch_id": chicken, "shopping_yield": Value::Null }).to_string(),
    );
    assert_eq!(status, 200, "{list}");
    assert_eq!(list["result"]["chosen"][0]["shopping_yield"], Value::Null);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_yield_being_shopped_for_moves_every_amount_with_it() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );
    let (chicken, _coq) = two_real_recipes(&app, &key, &kitchen_id);

    app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": chicken, "shopping_yield": { "amount": "8", "noun": "servings" } })
            .to_string(),
    );
    let (_, list) = app.post_op("get_shopping_list", Some(&key), "{}");
    assert_eq!(
        amounts(row(&list["result"], "soy sauce")),
        vec!["about 60 ml"]
    );
    assert_eq!(
        list["result"]["chosen"][0],
        json!({
            "branch_id": chicken,
            "title": "Korean Fried Chicken",
            "gone": false,
            "shopping_yield": { "amount": "8", "noun": "servings" },
            "written_yield": { "amount": "4", "noun": "servings" },
        })
    );

    // Back to the recipe as written.
    let (status, list) = app.post_op(
        "set_shopping_yield",
        Some(&key),
        &json!({ "branch_id": chicken, "shopping_yield": Value::Null }).to_string(),
    );
    assert_eq!(status, 200, "{list}");
    assert_eq!(
        amounts(row(&list["result"], "soy sauce")),
        vec!["about 30 ml"]
    );
    assert_eq!(list["result"]["chosen"][0]["shopping_yield"], Value::Null);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_loose_item_is_kept_exactly_as_typed_and_merges_with_nothing() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (chicken, _coq) = two_real_recipes(&app, &key, &kitchen_id);
    app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": chicken }).to_string(),
    );

    let (status, list) = app.post_op(
        "add_loose_item",
        Some(&key),
        &json!({ "text": "500g soy sauce" }).to_string(),
    );
    assert_eq!(status, 200, "{list}");

    // Never interpreted: the text is a row's whole name, it carries no amount,
    // and it did NOT join the soy sauce row that a recipe already put there.
    // Typing flour beside a recipe that wants flour gives two lines, and that
    // is the accepted cost of never guessing at a number about to be shopped
    // by (ADR 0024).
    let typed = row(&list["result"], "500g soy sauce");
    assert_eq!(typed["kind"], json!("loose"));
    assert_eq!(typed["parts"], json!([]));
    assert_eq!(typed["lines"], json!([]), "it was made from nothing");
    assert_eq!(
        amounts(row(&list["result"], "soy sauce")),
        vec!["about 2 tbsp"],
        "the recipe's own row is untouched by a Loose Item that reads like it"
    );

    let item_id = typed["id"].as_str().unwrap().to_string();
    let (status, list) = app.post_op(
        "remove_loose_item",
        Some(&key),
        &json!({ "item_id": item_id }).to_string(),
    );
    assert_eq!(status, 200, "{list}");
    assert!(
        !list["result"]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["name"] == json!("500g soy sauce"))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_recipe_that_can_no_longer_be_read_stays_on_the_list_and_says_so() {
    let app = support::spawn_app();
    let (aurelien, key, _kitchen) = person_with_kitchen(&app, "Aurélien");
    let (_marc, marc_key, marc_kitchen) = person_with_kitchen(&app, "Marc");

    // Aurélien joins Marc's Kitchen and chooses a recipe of Marc's to shop for.
    let (_, invite) = app.post_op(
        "invite_to_kitchen",
        Some(&marc_key),
        &json!({ "kitchen_id": marc_kitchen }).to_string(),
    );
    let secret = invite["result"]["secret"].as_str().unwrap().to_string();
    app.post_op(
        "accept_kitchen_invite",
        Some(&key),
        &json!({ "secret": secret }).to_string(),
    );
    let (_, made) = app.post_op(
        "create_recipe",
        Some(&marc_key),
        &json!({
            "kitchen_id": marc_kitchen,
            "title": "Ratatouille aux anchois",
            "ingredients": [{ "kind": "ingredient", "text": "2 tbsp soy sauce" }],
        })
        .to_string(),
    );
    let branch_id = made["result"]["branch_id"].as_str().unwrap().to_string();
    app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let (_, list) = app.post_op("get_shopping_list", Some(&key), "{}");
    assert_eq!(list["result"]["chosen"][0]["gone"], json!(false));
    assert_eq!(list["result"]["rows"].as_array().unwrap().len(), 1);

    // Marc withdraws the sharing.
    app.post_op(
        "remove_kitchen_member",
        Some(&marc_key),
        &json!({ "kitchen_id": marc_kitchen, "person_id": aurelien }).to_string(),
    );

    // **The entry stays, keeps the name it was known by, contributes nothing,
    // and says it can no longer be read** (ADR 0024). A thing that quietly
    // disappears from a shopping list is a thing that does not get bought.
    let (_, list) = app.post_op("get_shopping_list", Some(&key), "{}");
    assert_eq!(
        list["result"]["chosen"][0],
        json!({
            "branch_id": branch_id,
            "title": "Ratatouille aux anchois",
            "gone": true,
            "shopping_yield": Value::Null,
            "written_yield": Value::Null,
        })
    );
    assert_eq!(
        list["result"]["rows"],
        json!([]),
        "a recipe that cannot be read contributes no rows"
    );

    // And it can still be taken off — which is exactly the entry somebody most
    // wants gone.
    let (status, list) = app.post_op(
        "remove_from_shopping_list",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 200, "{list}");
    assert_eq!(list["result"]["chosen"], json!([]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn there_is_exactly_one_list_per_person_and_it_is_nobody_elses() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_marc, marc_key, _marc_kitchen) = person_with_kitchen(&app, "Marc");
    let (chicken, _coq) = two_real_recipes(&app, &key, &kitchen_id);

    // Choosing the same recipe twice makes no second entry: a list is a set.
    app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": chicken }).to_string(),
    );
    let (_, list) = app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": chicken }).to_string(),
    );
    assert_eq!(list["result"]["chosen"].as_array().unwrap().len(), 1);

    // A Shopping List is one **Person's** (ADR 0024) — not a Kitchen's, and
    // not shared with anyone who cooks in the same one.
    let (_, marcs) = app.post_op("get_shopping_list", Some(&marc_key), "{}");
    assert_eq!(marcs["result"], json!({ "chosen": [], "rows": [] }));

    // Nor may anyone choose a recipe they cannot see.
    let (status, refused) = app.post_op(
        "add_to_shopping_list",
        Some(&marc_key),
        &json!({ "branch_id": chicken }).to_string(),
    );
    assert_eq!(status, 404, "{refused}");
    assert_eq!(refused["error"]["kind"], json!("not_found"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_row_names_its_food_in_the_readers_own_language_and_measures() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "fr", "reading_measures": "metric" }).to_string(),
    );

    // Two recipes, one English and one French, both wanting flour. The Food
    // learns a name in each Language, which is what makes them one row
    // (ADR 0024, and the whole reason a per-Language Food name exists).
    let (_, english) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "language": "en",
            "title": "Yogurt Flatbread",
            "ingredients": [{ "kind": "ingredient", "text": "2 cups flour" }],
        })
        .to_string(),
    );
    let (_, french) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "language": "fr",
            "title": "Pâte à pizza",
            "ingredients": [{ "kind": "ingredient", "text": "500 g farine" }],
        })
        .to_string(),
    );
    // One Food, learning both names, is what a merged row rests on.
    app.core
        .db()
        .with_conn(|conn| {
            let food: String = conn
                .query_row(
                    "SELECT food_id FROM food_names WHERE language = 'en' AND name = 'flour'",
                    [],
                    |row| row.get(0),
                )
                .expect("the English Food");
            conn.execute(
                "INSERT OR REPLACE INTO food_names (food_id, language, name, name_folded) \
                 VALUES (?1, 'fr', 'farine', 'farine')",
                rusqlite::params![food],
            )
            .expect("name it in French");
            conn.execute(
                "UPDATE readings SET food_id = ?1 WHERE target = 'farine'",
                rusqlite::params![food],
            )
            .expect("point the French Reading at it");
            Ok(())
        })
        .expect("one Food, two names");

    for branch in [&english, &french] {
        app.post_op(
            "add_to_shopping_list",
            Some(&key),
            &json!({ "branch_id": branch["result"]["branch_id"] }).to_string(),
        );
    }
    let (_, list) = app.post_op("get_shopping_list", Some(&key), "{}");
    let list = &list["result"];

    // **One row, named in this reader's Reading Language** — every mention of
    // flour across every recipe on the list became one thing to buy.
    let flour = row(list, "farine");
    assert_eq!(flour["name_language"], json!("fr"));
    assert_eq!(flour["lines"].as_array().unwrap().len(), 2);

    // And added in her measures, across a Cup Weight: two cups of flour is
    // 250 g, plus the 500 g the other recipe weighed out (ADR 0016).
    assert_eq!(amounts(flour), vec!["environ 750 g"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_line_with_no_food_to_merge_under_is_kept_exactly_as_written() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    // Real corpus text: the Crouton export keeps whole paragraphs inside
    // ingredient entries, and no reading of one is honest. There is no Food to
    // merge it under, so it stands exactly as written — losing nothing,
    // because whatever amount it holds is in the line (ADR 0002, ADR 0024).
    let (_, made) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({
            "kitchen_id": kitchen_id,
            "title": "Gochujang And Halloumi Orzo Pasta",
            "ingredients": [
                { "kind": "ingredient", "text": "Can I substitute the wine as I don’t drink alcohol? Yes you can just use water instead" },
            ],
        })
        .to_string(),
    );
    app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": made["result"]["branch_id"] }).to_string(),
    );

    let (_, list) = app.post_op("get_shopping_list", Some(&key), "{}");
    let kept = &list["result"]["rows"][0];
    assert_eq!(kept["kind"], json!("line"));
    assert_eq!(
        kept["name"],
        json!(
            "Can I substitute the wine as I don’t drink alcohol? Yes you can just use water instead"
        )
    );
    assert_eq!(kept["parts"], json!([]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_line_carrying_no_quantity_says_some_and_no_yield_ever_moves_it() {
    // **#44's last acceptance criterion, on the list #73 built.** A line
    // carrying no quantity at all is 28% of the real corpus (#5), so this is
    // not an edge: dropping such a line means coming home short, and scaling
    // one means inventing an amount nobody wrote (ADR 0002).
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );
    let (_chicken, coq) = two_real_recipes(&app, &key, &kitchen_id);
    let (status, list) = app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": coq }).to_string(),
    );
    assert_eq!(status, 200, "{list}");

    // `olive oil` is written with no quantity beside it, so the row it makes
    // says *some* rather than nothing.
    assert_eq!(amounts(row(&list["result"], "olive oil")), vec!["some"]);

    // **Now shop for twice the recipe.** The soy sauce is here as the
    // contrast, not the subject: something has to move for *unmoved* to mean
    // anything.
    let (status, list) = app.post_op(
        "set_shopping_yield",
        Some(&key),
        &json!({ "branch_id": coq, "shopping_yield": { "amount": "8", "noun": "servings" } })
            .to_string(),
    );
    assert_eq!(status, 200, "{list}");
    let doubled = &list["result"];
    assert_eq!(amounts(row(doubled, "soy sauce")), vec!["about 30 ml"]);

    // Twice as much of the recipe is still *some* olive oil. Doubling an
    // amount nobody stated would be Kamosu writing the recipe, which is the
    // one thing ADR 0002 does not let it do.
    assert_eq!(amounts(row(doubled, "olive oil")), vec!["some"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_line_with_no_reading_at_all_still_reaches_the_list_and_the_yield_leaves_it_alone() {
    // The other half of ADR 0024's sentence — "a verbatim line — an
    // **Ingredient Line** with no **Reading**, or a **Loose Item** — merges
    // with nothing". Its sibling
    // `a_line_with_no_food_to_merge_under_is_kept_exactly_as_written` covers a
    // line Kamosu *did* read and found no Food in; this one covers the slot
    // being empty outright, which is a different branch of the same `else`.
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );
    let (chicken, _coq) = two_real_recipes(&app, &key, &kitchen_id);

    // Kamosu read `Some cooking oil (for deep frying)` and found a Food in it.
    // She disagrees, and clears the Reading — an ordinary correction, which
    // mints no Version and enters no history (ADR 0021). That is what leaves a
    // line with no Reading at all, and it is the honest way to reach the state:
    // every line in the real corpus carries a word, and a word is a target.
    let (status, cleared) = app.post_op(
        "set_reading",
        Some(&key),
        &json!({ "branch_id": chicken, "line_index": 2 }).to_string(),
    );
    assert_eq!(status, 200, "{cleared}");
    assert_eq!(cleared["result"]["reading"], Value::Null);

    let (_, read_back) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": chicken }).to_string(),
    );
    assert_eq!(
        read_back["result"]["versions"][0]["readings"][2],
        Value::Null,
        "the slot is empty outright, and not a Reading that merely found no Food"
    );

    // It is still a thing to buy. The line stands exactly as written, losing
    // nothing, because whatever it holds is in the line.
    let (_, list) = app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": chicken, "shopping_yield": { "amount": "8", "noun": "servings" } })
            .to_string(),
    );
    let kept = row(&list["result"], "Some cooking oil (for deep frying)");
    assert_eq!(kept["kind"], json!("line"));
    assert_eq!(kept["parts"], json!([]));

    // And shopping for twice the recipe leaves it exactly as written: there is
    // no number here to double.
    assert_eq!(
        amounts(row(&list["result"], "minced garlic")),
        vec!["about 60 ml"],
        "the amounts Kamosu did read moved, so the unread line's stillness means something"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn one_list_in_one_order_whatever_a_row_was_made_from() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (chicken, coq) = two_real_recipes(&app, &key, &kitchen_id);
    for branch in [&chicken, &coq] {
        app.post_op(
            "add_to_shopping_list",
            Some(&key),
            &json!({ "branch_id": branch }).to_string(),
        );
    }
    app.post_op(
        "add_loose_item",
        Some(&key),
        &json!({ "text": "bin bags" }).to_string(),
    );

    // A Loose Item sorts among the Foods rather than into a block of its own:
    // a heading over the rows that merged would teach that the others are
    // somehow less true, which is ADR 0015's trap wearing a heading.
    let (_, list) = app.post_op("get_shopping_list", Some(&key), "{}");
    let names: Vec<&str> = list["result"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        vec![
            "bin bags",
            "minced garlic",
            "olive oil",
            "Some cooking oil",
            "soy sauce",
        ]
    );
}

// ── Components (#50, ADR 0008) ───────────────────────────────────────────────
//
// A recipe used inside another is not a new kind of thing. It is an ordinary
// Ingredient whose Reading names a Recipe instead of a Food, and every test
// below drives that through the real Operations.

/// Write a recipe with a Yield and a list, and answer (branch_id, lineage_id).
fn recipe_with(
    app: &support::TestApp,
    key: &str,
    kitchen_id: &str,
    title: &str,
    made: Option<(&str, &str)>,
    ingredients: Value,
    steps: Value,
) -> (String, String) {
    let (status, created) = app.post_op(
        "create_recipe",
        Some(key),
        &json!({ "kitchen_id": kitchen_id, "title": title }).to_string(),
    );
    assert_eq!(status, 200, "{created}");
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();
    let lineage_id = created["result"]["lineage_id"]
        .as_str()
        .unwrap()
        .to_string();

    let mut save = json!({
        "branch_id": branch_id,
        "title": title,
        "ingredients": ingredients,
        "steps": steps,
    });
    if let Some((amount, noun)) = made {
        save["yield"] = json!({ "amount": amount, "noun": noun });
    }
    let (status, saved) = app.post_op("save_recipe_version", Some(key), &save.to_string());
    assert_eq!(status, 200, "{saved}");
    (branch_id, lineage_id)
}

/// Make one line of a recipe a Component: its Reading names a Lineage.
fn make_component(
    app: &support::TestApp,
    key: &str,
    branch_id: &str,
    line_index: i64,
    amount: Option<&str>,
    unit: Option<&str>,
    lineage_id: &str,
) {
    let (status, set) = app.post_op(
        "set_reading",
        Some(key),
        &json!({
            "branch_id": branch_id,
            "line_index": line_index,
            "amount": amount,
            "unit": unit,
            "lineage_id": lineage_id,
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{set}");
    assert_eq!(
        set["result"]["reading"]["lineage_id"],
        json!(lineage_id),
        "the Reading answers the Recipe it now names"
    );
    assert_eq!(
        set["result"]["reading"]["target"],
        Value::Null,
        "a Reading's target is either a Food or a Lineage, never both"
    );
}

/// The Components of a recipe's current Version, as `get_recipe` unfolds them.
fn components_of(app: &support::TestApp, key: &str, branch_id: &str) -> Vec<Value> {
    let (status, read) = app.post_op(
        "get_recipe",
        Some(key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 200, "{read}");
    read["result"]["versions"]
        .as_array()
        .expect("versions")
        .last()
        .expect("a Version")["components"]
        .as_array()
        .expect("components")
        .clone()
}

/// A pizza whose dough is a Component: the pointer resolves, the multiplier is
/// worked out from the dough's own Yield, and the dough's lines arrive scaled.
///
/// `500 g` of a dough that yields `1 kg` is half of it — ADR 0008's own worked
/// example — and the factor is computed at display time and stored nowhere.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_component_is_an_ingredient_whose_reading_names_a_recipe_and_it_arrives_scaled() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    // A metric kitchen, so the halved amounts read as ADR 0008's own example
    // writes them. A Component's lines are worded by exactly the code every
    // other Ingredient Line's slot uses, so they convert for the reader too.
    let (status, _) = app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );
    assert_eq!(status, 200);

    let (_dough, dough_lineage) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Neapolitan Pizza Dough",
        Some(("1", "kg")),
        json!([
            { "kind": "ingredient", "text": "600 g tipo 00 flour" },
            { "kind": "ingredient", "text": "390 ml cold water" },
        ]),
        json!([
            { "kind": "step", "text": "Dissolve the salt in the water." },
            { "kind": "step", "text": "Knead for ten minutes." },
        ]),
    );
    let (pizza, _) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Pizza Margherita",
        Some(("2", "pizzas")),
        json!([
            { "kind": "ingredient", "text": "Dough for 2 pizzas" },
            { "kind": "ingredient", "text": "250 g mozzarella" },
        ]),
        json!([{ "kind": "step", "text": "Stretch, top and bake." }]),
    );
    make_component(
        &app,
        &key,
        &pizza,
        0,
        Some("500"),
        Some("g"),
        &dough_lineage,
    );

    let components = components_of(&app, &key, &pizza);
    assert_eq!(
        components.len(),
        1,
        "one line names a recipe: {components:?}"
    );
    let dough = &components[0];
    assert_eq!(dough["path"], json!([0]), "it is the pizza's first line");
    assert_eq!(dough["held"], json!(true));
    assert_eq!(dough["stopped"], json!(false));
    assert_eq!(dough["title"], json!("Neapolitan Pizza Dough"));
    // 500 g of a kilo. Computed from the Reading and the dough's own Yield.
    assert_eq!(dough["share"], json!(0.5));
    assert_eq!(
        dough["said"],
        json!("Neapolitan Pizza Dough · ½ of the recipe"),
        "the one line beneath the written line, worded once in the Core"
    );

    // The dough's own lines, already halved. The written lines are untouched —
    // the amounts arrive in the subordinate slot every Ingredient Line has.
    let written: Vec<&str> = dough["content"]["ingredients"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| line["text"].as_str().unwrap())
        .collect();
    assert_eq!(written, ["600 g tipo 00 flour", "390 ml cold water"]);
    assert_eq!(
        dough["measured"]["ingredients"],
        json!(["about 300 g", "about 200 ml"]),
        "the dough arrives halved, worded by the same code as every other line"
    );

    // **Stored nowhere.** The multiplier is not in the Reading, which carries
    // the quantity somebody wrote and the Recipe it names, and nothing else.
    let (_, read) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    let reading = &read["result"]["versions"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()["readings"][0];
    assert_eq!(
        reading,
        &json!({ "amount": "500", "unit": "g", "target": null, "lineage_id": dough_lineage }),
        "the Reading holds the pointer and the quantity, never the factor"
    );

    // And the pizza's own Method never grew the dough's Steps: composition
    // says WHAT and never WHEN (ADR 0008).
    let steps: Vec<&str> = read["result"]["versions"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()["content"]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| line["text"].as_str().unwrap())
        .collect();
    assert_eq!(steps, ["Stretch, top and bake."]);
    assert_eq!(
        dough["content"]["steps"].as_array().unwrap().len(),
        2,
        "the dough's own Steps travel with the dough, separately"
    );
}

/// **A Component survives a save of the recipe it sits in** (#87).
///
/// A Reading travels with its Version and is carried onto the next one wherever
/// the line still reads exactly as it did (ADR 0021). The pointer that makes the
/// line a Component is part of that Reading, and it was being left behind: the
/// carry copied the amount, the Unit and the Food and not the Lineage. So
/// editing anything at all — a title, a step, a line somewhere else entirely —
/// turned every dough inside every pizza back into an ordinary ingredient, on
/// lines nobody had touched.
///
/// Nothing noticed until #87, because until #87 a Component could only be made
/// by an agent at the MCP door and never by somebody who then went on editing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_save_carries_a_component_forward_onto_the_version_it_writes() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_dough_branch, dough_lineage) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Pizza Dough",
        Some(("1", "kg")),
        json!([{ "kind": "ingredient", "text": "600 g flour" }]),
        json!([{ "kind": "step", "text": "Knead." }]),
    );
    let (pizza_branch, _) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Pizza Margherita",
        Some(("2", "pizzas")),
        json!([
            { "kind": "ingredient", "text": "500 g pizza dough" },
            { "kind": "ingredient", "text": "250 g mozzarella" },
        ]),
        json!([{ "kind": "step", "text": "Stretch and bake." }]),
    );
    make_component(
        &app,
        &key,
        &pizza_branch,
        0,
        Some("500"),
        Some("g"),
        &dough_lineage,
    );
    assert_eq!(
        components_of(&app, &key, &pizza_branch).len(),
        1,
        "the dough is a Component before anything is saved"
    );

    // A save that does not touch the Component's own line. The second
    // ingredient is rewritten, which is the ordinary thing somebody does on a
    // recipe — and the line that must not lose its pointer is the other one.
    let (status, saved) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": pizza_branch,
            "title": "Pizza Margherita",
            "yield": { "amount": "2", "noun": "pizzas" },
            "ingredients": [
                { "kind": "ingredient", "text": "500 g pizza dough" },
                { "kind": "ingredient", "text": "300 g mozzarella" },
            ],
            "steps": [{ "kind": "step", "text": "Stretch and bake." }],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{saved}");

    let after = components_of(&app, &key, &pizza_branch);
    assert_eq!(
        after.len(),
        1,
        "the line nobody edited is still a Component after the save"
    );
    assert_eq!(
        after[0]["title"],
        json!("Pizza Dough"),
        "and it still names the same Recipe"
    );
    assert_eq!(after[0]["path"], json!([0]), "on the line it was always on");
    // The whole Reading came across, not only the pointer: a Component with no
    // quantity is the whole of the inner recipe (ADR 0008), so an amount lost
    // here would double the dough rather than merely look untidy.
    assert!(
        after[0]["said"]
            .as_str()
            .expect("a Component says what it is")
            .contains("Pizza Dough"),
        "the Component's own line still names the dough: {:?}",
        after[0]["said"]
    );
}

/// **Making a Component by hand, and un-making it** (#87) — the act the
/// interface performs, driven here through the real Operations.
///
/// Two things this holds that the screen tests cannot. A line is made a
/// Component on a Version that has ALREADY been saved and read, which is the
/// order the writing screen works in: the save lands, the Core reads the line
/// into an amount and a Food, and only then is the pointer attached over the
/// top. And un-making returns the line to an ordinary one that still carries
/// its quantity, so `500 g` is not lost along with the pointer.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_component_is_made_over_a_read_line_and_un_made_without_losing_the_amount() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_dough_branch, dough_lineage) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Pizza Dough",
        Some(("1", "kg")),
        json!([{ "kind": "ingredient", "text": "600 g flour" }]),
        json!([{ "kind": "step", "text": "Knead." }]),
    );
    let (pizza_branch, _) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Pizza Margherita",
        Some(("2", "pizzas")),
        json!([{ "kind": "ingredient", "text": "500 g pizza dough" }]),
        json!([{ "kind": "step", "text": "Stretch and bake." }]),
    );

    // The save already read the line: an amount, a Unit and a Food of its own
    // (#71). This is the state the interface finds the line in.
    let (_, read) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": pizza_branch }).to_string(),
    );
    let was = read["result"]["versions"].as_array().expect("versions")[0]["readings"][0].clone();
    assert_eq!(was["amount"], json!("500"), "the Core read the quantity");
    assert_eq!(
        was["lineage_id"],
        Value::Null,
        "and read no Recipe, because composition is never guessed (ADR 0008)"
    );

    // Making it: the pointer goes on over what was read, carrying the amount
    // and the Unit with it. Sending the pointer alone would clear them, and a
    // Component with no quantity is the WHOLE of the inner recipe.
    make_component(
        &app,
        &key,
        &pizza_branch,
        0,
        was["amount"].as_str(),
        was["unit"].as_str(),
        &dough_lineage,
    );
    let made = components_of(&app, &key, &pizza_branch);
    assert_eq!(made.len(), 1, "the line names the dough now");
    assert_eq!(
        made[0]["share"],
        json!(0.5),
        "500 g of a dough that yields 1 kg is half of it — ADR 0008's own example"
    );

    // Un-making it: the pointer goes, the quantity stays. The line is an
    // ordinary Ingredient Line again, reading exactly as it always did.
    let (status, unmade) = app.post_op(
        "set_reading",
        Some(&key),
        &json!({
            "branch_id": pizza_branch,
            "line_index": 0,
            "amount": "500",
            "unit": "g",
            "target": "pizza dough",
            "lineage_id": null,
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{unmade}");
    assert_eq!(unmade["result"]["reading"]["lineage_id"], Value::Null);
    assert_eq!(
        unmade["result"]["reading"]["amount"],
        json!("500"),
        "un-making a Component does not take the quantity with it"
    );
    assert!(
        components_of(&app, &key, &pizza_branch).is_empty(),
        "and the recipe composes nothing again"
    );

    // The written line was never touched by any of it: the Reading is Kamosu's
    // reading and never the recipe (ADR 0002, ADR 0021).
    let (_, still) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": pizza_branch }).to_string(),
    );
    let content = &still["result"]["versions"].as_array().expect("versions")[0]["content"];
    assert_eq!(
        content["ingredients"][0]["text"],
        json!("500 g pizza dough")
    );
}

/// **A missing Component leaves a sentence rather than a hole** (ADR 0008).
///
/// Deleted, never received, and held by nobody here are one case on purpose:
/// no cascade, no warning, no "3 recipes use this". The row goes on reading
/// correctly because the written line was always the truth (ADR 0002), and it
/// is a Lineage that is missing rather than a Version, so this is what a Bundle
/// arriving without its passenger looks like too.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_component_whose_recipe_is_not_here_still_reads_and_says_so() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (pizza, _) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Pizza Margherita",
        Some(("2", "pizzas")),
        json!([{ "kind": "ingredient", "text": "Dough for 2 pizzas" }]),
        json!([{ "kind": "step", "text": "Stretch, top and bake." }]),
    );

    // A Lineage this instance has never held. It is accepted without ceremony:
    // refusing it would refuse the very pointer ADR 0008 exists to keep.
    make_component(
        &app,
        &key,
        &pizza,
        0,
        Some("500"),
        Some("g"),
        "l_never_received_here",
    );

    let components = components_of(&app, &key, &pizza);
    assert_eq!(components.len(), 1);
    let dough = &components[0];
    assert_eq!(dough["held"], json!(false));
    assert_eq!(dough["title"], Value::Null);
    assert_eq!(dough["content"], Value::Null);
    assert_eq!(dough["share"], Value::Null);
    assert_eq!(dough["said"], json!("Kamosu does not have this recipe."));

    // And the recipe itself is untouched and entirely usable.
    let (status, read) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    assert_eq!(status, 200, "{read}");
    let version = read["result"]["versions"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    assert_eq!(
        version["content"]["ingredients"][0]["text"],
        json!("Dough for 2 pizzas"),
        "the written line still reads correctly with nothing behind it"
    );
    assert_eq!(
        version["readings"][0]["lineage_id"],
        json!("l_never_received_here"),
        "and the pointer is kept, so the dough arriving later needs nothing done"
    );
}

/// **An inner recipe with no Yield gives no factor, and Kamosu says so** rather
/// than guessing (ADR 0008). The recipe is shown as written; a silently
/// mis-scaled dough would be worse than an unscaled one.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_component_whose_recipe_has_no_yield_is_shown_as_written_and_admits_it() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_dough, dough_lineage) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Versatile Pizza Dough",
        None, // no Yield: nothing to divide by
        json!([{ "kind": "ingredient", "text": "600 g AP flour" }]),
        json!([{ "kind": "step", "text": "Mix on low three minutes." }]),
    );
    let (pizza, _) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Pizza Margherita",
        Some(("2", "pizzas")),
        json!([{ "kind": "ingredient", "text": "Dough for 2 pizzas" }]),
        json!([{ "kind": "step", "text": "Stretch, top and bake." }]),
    );
    make_component(
        &app,
        &key,
        &pizza,
        0,
        Some("500"),
        Some("g"),
        &dough_lineage,
    );

    let components = components_of(&app, &key, &pizza);
    let dough = &components[0];
    assert_eq!(
        dough["held"],
        json!(true),
        "the recipe is here; the Yield is not"
    );
    assert_eq!(dough["share"], Value::Null, "no factor was invented");
    assert_eq!(
        dough["said"],
        json!(
            "Versatile Pizza Dough — Kamosu could not work out how much, so this is the recipe as written."
        )
    );
    assert_eq!(
        dough["content"]["ingredients"][0]["text"],
        json!("600 g AP flour"),
        "and it unfolds anyway, as written"
    );

    // A Unit measured against a noun it cannot be compared with is the same
    // answer: `2 handfuls` of a dough that yields `1 kg` is not a ratio.
    let (_starter, starter_lineage) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Starter",
        Some(("1", "kg")),
        json!([{ "kind": "ingredient", "text": "500 g flour" }]),
        json!([]),
    );
    let (bread, _) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Bread",
        Some(("1", "loaf")),
        json!([{ "kind": "ingredient", "text": "2 handfuls of starter" }]),
        json!([]),
    );
    make_component(
        &app,
        &key,
        &bread,
        0,
        Some("2"),
        Some("handfuls"),
        &starter_lineage,
    );
    assert_eq!(
        components_of(&app, &key, &bread)[0]["share"],
        Value::Null,
        "handfuls against kilograms is not a ratio, and none is invented"
    );
}

/// **A cycle is never refused; unfolding stops at the first repeat and says
/// so** (ADR 0008).
///
/// A loop can be assembled from two innocent halves on two servers and arrive
/// already formed, so a save-time check could not hold the line and would only
/// give false confidence. Both saves below are accepted without complaint; the
/// guard is at display time, where it belongs.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cycle_is_never_refused_and_unfolding_stops_at_the_repeat() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (noodles, noodles_lineage) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Dan Dan Noodles",
        Some(("4", "servings")),
        json!([{ "kind": "ingredient", "text": "3 tbsp chilli oil" }]),
        json!([{ "kind": "step", "text": "Assemble." }]),
    );
    let (oil, oil_lineage) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Chilli Oil",
        Some(("250", "ml")),
        json!([{ "kind": "ingredient", "text": "2 tbsp Dan Dan sauce, for the colour" }]),
        json!([{ "kind": "step", "text": "Warm the oil." }]),
    );
    // Each half is accepted on its own, which is the point: neither save can
    // see the loop, and on two servers neither ever would.
    make_component(
        &app,
        &key,
        &noodles,
        0,
        Some("3"),
        Some("tbsp"),
        &oil_lineage,
    );
    make_component(
        &app,
        &key,
        &oil,
        0,
        Some("2"),
        Some("tbsp"),
        &noodles_lineage,
    );

    let components = components_of(&app, &key, &noodles);
    assert_eq!(
        components.len(),
        2,
        "the oil unfolded, and the line inside it that loops back is reported: {components:?}"
    );

    let oil_entry = &components[0];
    assert_eq!(oil_entry["path"], json!([0]));
    assert_eq!(oil_entry["title"], json!("Chilli Oil"));
    // 3 tbsp of 250 ml — the tablespoons and the millilitres are compared
    // through the same base values everything else converts on.
    assert_eq!(oil_entry["stopped"], json!(false));
    assert!(
        (oil_entry["share"].as_f64().unwrap() - 0.1774).abs() < 0.001,
        "3 tbsp of 250 ml: {}",
        oil_entry["share"]
    );

    let repeat = &components[1];
    assert_eq!(
        repeat["path"],
        json!([0, 0]),
        "inside the oil, on its first line"
    );
    assert_eq!(repeat["stopped"], json!(true));
    assert_eq!(repeat["content"], Value::Null, "it goes no deeper");
    assert_eq!(
        repeat["said"],
        json!("Dan Dan Noodles is already open above — Kamosu stops here.")
    );

    // Standing in the oil instead, the same loop stops the other way round —
    // there is nothing special about where you entered it.
    let from_the_oil = components_of(&app, &key, &oil);
    assert_eq!(from_the_oil[0]["title"], json!("Dan Dan Noodles"));
    assert_eq!(from_the_oil[0]["stopped"], json!(false));
    assert_eq!(from_the_oil[1]["stopped"], json!(true));
    assert_eq!(
        from_the_oil[1]["said"],
        json!("Chilli Oil is already open above — Kamosu stops here.")
    );
}

/// **A Reading points at a Food or at a Recipe, never both.** Refused rather
/// than silently preferring one: a line claiming to be both a flour and a dough
/// is a caller's mistake, and quietly dropping half of what they sent hides it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_readings_target_is_a_food_or_a_lineage_and_never_both() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (pizza, lineage) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Pizza Margherita",
        Some(("2", "pizzas")),
        json!([{ "kind": "ingredient", "text": "Dough for 2 pizzas" }]),
        json!([]),
    );

    let (status, refused) = app.post_op(
        "set_reading",
        Some(&key),
        &json!({
            "branch_id": pizza,
            "line_index": 0,
            "amount": "500",
            "unit": "g",
            "target": "flour",
            "lineage_id": lineage,
        })
        .to_string(),
    );
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["error"]["kind"], "bad_request");

    // Clearing a Component clears the pointer with it: sending the whole
    // Reading is how `set_reading` has always worked, so the line goes back to
    // being an ordinary unread line rather than a Component with no quantity.
    make_component(&app, &key, &pizza, 0, Some("500"), Some("g"), "l_elsewhere");
    let (status, cleared) = app.post_op(
        "set_reading",
        Some(&key),
        &json!({ "branch_id": pizza, "line_index": 0 }).to_string(),
    );
    assert_eq!(status, 200, "{cleared}");
    assert_eq!(cleared["result"]["reading"], Value::Null);
    assert!(
        components_of(&app, &key, &pizza).is_empty(),
        "the line is no longer a Component"
    );
}

/// **A Component travels with its parent as a Passenger, and its own Visibility
/// is unchanged by that** (ADR 0008).
///
/// Sharing the pizza carries the dough, because a recipe that cannot tell you
/// how to make its own dough is incomplete. The dough gets no page, no Share
/// Link of its own and cannot be found: it is read *through* the pizza. Nothing
/// here sets a Visibility, because in Kamosu nothing sets a Visibility except a
/// person deciding to.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_component_travels_as_a_passenger_without_its_own_visibility_changing() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (dough, dough_lineage) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Neapolitan Pizza Dough",
        Some(("1", "kg")),
        json!([{ "kind": "ingredient", "text": "600 g tipo 00 flour" }]),
        json!([{ "kind": "step", "text": "Knead for ten minutes." }]),
    );
    let (pizza, _) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Pizza Margherita",
        Some(("2", "pizzas")),
        json!([{ "kind": "ingredient", "text": "Dough for 2 pizzas" }]),
        json!([{ "kind": "step", "text": "Stretch, top and bake." }]),
    );
    make_component(
        &app,
        &key,
        &pizza,
        0,
        Some("500"),
        Some("g"),
        &dough_lineage,
    );

    let (token, _url) = share(&app, &key, &pizza);

    // A stranger, holding the token and nothing else.
    let (status, read) = app.post_op(
        "read_shared_recipe",
        None,
        &json!({ "token": token }).to_string(),
    );
    assert_eq!(status, 200, "{read}");
    let carried = read["result"]["recipe"]["components"]
        .as_array()
        .expect("the Passengers travel with the recipe");
    assert_eq!(carried.len(), 1, "the dough came along: {carried:?}");
    assert_eq!(carried[0]["title"], json!("Neapolitan Pizza Dough"));
    assert_eq!(carried[0]["held"], json!(true));
    assert_eq!(carried[0]["share"], json!(0.5));
    assert_eq!(
        carried[0]["content"]["steps"][0]["text"],
        json!("Knead for ten minutes."),
        "a recipe that cannot tell you how to make its own dough is incomplete"
    );
    // A Share Link's rows carry no subordinate line at all — Kamosu converts to
    // a kitchen and a stranger has none — so a Passenger's slot is empty, and
    // declared empty rather than absent, the way a Food's nutrition is.
    assert_eq!(
        carried[0]["measured"],
        Value::Null,
        "a Passenger carries no measured line: {carried:?}"
    );

    // **The dough's own Visibility did not change.** It has no Share Link, so
    // it has no page and cannot be found — it is readable only through the
    // pizza's link.
    let (status, dough_link) = app.post_op(
        "get_share_link",
        Some(&key),
        &json!({ "branch_id": dough }).to_string(),
    );
    assert_eq!(status, 200, "{dough_link}");
    assert_eq!(
        dough_link["result"]["shared"],
        json!(false),
        "sharing the pizza minted no link for the dough"
    );

    // And the page a stranger is served renders it: the dough's lines under the
    // row that names them, its Steps at the foot under their own heading (#50).
    let (status, _kind, page) = app.get(&format!("/s/{token}"));
    assert_eq!(status, 200);
    assert!(
        page.contains("Neapolitan Pizza Dough · ½ of the recipe"),
        "the Component's own line is on the page"
    );
    assert!(
        page.contains("600 g tipo 00 flour"),
        "its Ingredient Lines unfold in place"
    );
    assert!(
        page.contains("its own method") && page.contains("Knead for ten minutes."),
        "and its Steps are set at the foot, under a heading of their own"
    );
}

/// **Scaling the outer recipe rescales the Reading, and the factor follows for
/// free** (ADR 0008).
///
/// Cooking a pizza at double wants the whole of a dough it otherwise wants half
/// of. The sentence beneath the line and the amounts printed under it are one
/// answer to one question, so the factor carries the Yield being cooked rather
/// than being applied on top of it — otherwise the row reads "half the recipe"
/// over a full dough's worth of flour.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scaling_the_outer_recipe_carries_into_how_much_of_the_component_is_wanted() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (status, _) = app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );
    assert_eq!(status, 200);

    let (_dough, dough_lineage) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Neapolitan Pizza Dough",
        Some(("1", "kg")),
        json!([{ "kind": "ingredient", "text": "600 g tipo 00 flour" }]),
        json!([{ "kind": "step", "text": "Knead for ten minutes." }]),
    );
    let (pizza, _) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Pizza Margherita",
        Some(("2", "pizzas")),
        json!([{ "kind": "ingredient", "text": "Dough for 2 pizzas" }]),
        json!([{ "kind": "step", "text": "Stretch, top and bake." }]),
    );
    make_component(
        &app,
        &key,
        &pizza,
        0,
        Some("500"),
        Some("g"),
        &dough_lineage,
    );

    // Read as written: half a kilo of a kilo.
    let at_rest = components_of(&app, &key, &pizza);
    assert_eq!(at_rest[0]["share"], json!(0.5));
    assert_eq!(
        at_rest[0]["said"],
        json!("Neapolitan Pizza Dough · ½ of the recipe")
    );
    assert_eq!(
        at_rest[0]["measured"]["ingredients"],
        json!(["about 300 g"])
    );

    // Now cook four pizzas instead of two. An In Progress Attempt at another
    // Yield is what `get_recipe` reads its scale from (#61).
    let (status, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    assert_eq!(status, 200, "{started}");
    let attempt_id = started["result"]["id"].as_str().unwrap().to_string();
    let (status, advanced) = app.post_op(
        "advance_attempt",
        Some(&key),
        &json!({
            "attempt_id": attempt_id,
            "cooking_yield": { "amount": "4", "noun": "pizzas" },
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{advanced}");

    let doubled = components_of(&app, &key, &pizza);
    assert_eq!(
        doubled[0]["share"],
        json!(1.0),
        "two pizzas' worth of dough doubled is the whole dough"
    );
    assert_eq!(
        doubled[0]["said"],
        json!("Neapolitan Pizza Dough · the whole recipe"),
        "and the sentence says so, rather than contradicting the amounts below it"
    );
    // And the dough's own lines fall SILENT, which is the same rule every other
    // Ingredient Line obeys: at the whole recipe, in this reader's own measures,
    // a subordinate line would only repeat `600 g tipo 00 flour` back at her
    // (#49, ADR 0016). The line above is already the answer.
    assert_eq!(
        doubled[0]["measured"]["ingredients"],
        json!([null]),
        "nothing to say beneath a line that is already what it should be"
    );
}

// --- The As Cooked, and Promotion (#58, ADR 0005) ----------------------------
//
// An Attempt that deviated holds a COMPLETE RECIPE STATE, not a record of
// differences. Everything below is that sentence in various lights: the words
// stored are ordinary Ingredient Lines and ordinary Step text, the whole recipe
// is there whether or not a line changed, and Promotion is an ordinary save
// rather than a mechanism of its own.

/// The Katsu Curry of `recipe_ready_to_cook`, exactly as written. Whatever a
/// test does to a clone of this is the deviation, and nothing else is.
fn katsu_as_written() -> Value {
    json!({
        "title": "Katsu Curry",
        "ingredients": [
            { "kind": "ingredient", "text": "2 escalopes de poulet" },
            { "kind": "ingredient", "text": "200 g de riz" },
        ],
        "steps": [
            { "kind": "step", "text": "Paner les escalopes" },
            { "kind": "step", "text": "Frire jusqu'à dorer" },
            { "kind": "step", "text": "Servir avec le riz" },
        ],
    })
}

/// Start cooking the Katsu Curry and hand back the Attempt's id.
fn cooking_katsu(app: &support::TestApp, key: &str, branch_id: &str) -> String {
    let (_, started) = app.post_op(
        "start_attempt",
        Some(key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    started["result"]["id"].as_str().unwrap().to_string()
}

/// Age a Branch's head out of the one-hour collapse window, so the next save
/// appends a Version instead of folding into the one being shaped.
///
/// A promotion in real life happens hours after the cooking, or days; a test
/// does the whole thing in milliseconds, and would otherwise be measuring the
/// collapse window rather than Promotion. The same trick as
/// `backdate_attempt_action` next door, on the other clock that matters.
fn age_branch_head(app: &support::TestApp, branch_id: &str) {
    app.core
        .db()
        .with_conn(|conn| {
            conn.execute(
                "UPDATE branch_versions \
                    SET created_at = strftime('%Y-%m-%dT%H:%M:%fZ','now','-2 hours') \
                  WHERE branch_id = ?1",
                rusqlite::params![branch_id],
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            Ok(())
        })
        .expect("age the Branch head");
}

/// How many `versions` rows no Branch's chain names — an As Cooked is exactly
/// such a row, so this counts them without knowing how one is stored.
fn versions_on_no_branch(app: &support::TestApp) -> i64 {
    app.core
        .db()
        .with_conn(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM versions WHERE id NOT IN \
                 (SELECT version_id FROM branch_versions)",
                [],
                |row| row.get(0),
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .unwrap()
}

// ── A Shopping List unfolds Components to the bottom (#86, ADR 0008) ─────────
//
// ADR 0008's last consequence, and the one #73 shipped before it was possible:
// a list unfolds every Component to the bottom under the same repeat guard,
// applies the factor at each level, and then merges Foods — which is a Food
// question rather than a link question. Choosing the pizza has to buy the
// dough's flour.

/// A pizza, its dough, and the Component that joins them — the worked example
/// ADR 0008 itself uses. Answers the pizza's Branch, the dough's Branch and
/// the dough's Lineage, which is what a Component actually names.
fn a_pizza_on_a_dough(
    app: &support::TestApp,
    key: &str,
    kitchen_id: &str,
) -> (String, String, String) {
    let (dough, dough_lineage) = recipe_with(
        app,
        key,
        kitchen_id,
        "Neapolitan Pizza Dough",
        Some(("1", "kg")),
        json!([
            { "kind": "ingredient", "text": "600 g tipo 00 flour" },
            { "kind": "ingredient", "text": "390 ml cold water" },
        ]),
        json!([{ "kind": "step", "text": "Knead for ten minutes." }]),
    );
    let (pizza, _) = recipe_with(
        app,
        key,
        kitchen_id,
        "Pizza Margherita",
        Some(("2", "pizzas")),
        json!([
            { "kind": "ingredient", "text": "Dough for 2 pizzas" },
            { "kind": "ingredient", "text": "100 g tipo 00 flour" },
            { "kind": "ingredient", "text": "250 g mozzarella" },
        ]),
        json!([{ "kind": "step", "text": "Stretch, top and bake." }]),
    );
    // 500 g of a dough that yields a kilo: half of it.
    make_component(app, key, &pizza, 0, Some("500"), Some("g"), &dough_lineage);
    (pizza, dough, dough_lineage)
}

/// Every row of a list, by name, so a test can say what the whole list holds.
fn row_names(list: &Value) -> Vec<String> {
    list["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|row| row["name"].as_str().unwrap_or_default().to_string())
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn choosing_a_recipe_buys_the_ingredients_of_the_recipes_inside_it() {
    // The failure #86 names: choosing the pizza used to put *Dough for 2
    // pizzas* on the list as an uninterpreted line and none of the dough's own
    // flour and water at all. Now the dough contributes its Foods and no line
    // of its own, and its flour merges with the pizza's.
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );
    let (pizza, _dough, _lineage) = a_pizza_on_a_dough(&app, &key, &kitchen_id);

    let (status, list) = app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    assert_eq!(status, 200, "{list}");
    let list = &list["result"];

    // 300 g of the dough's flour — half of 600 — and the pizza's own 100 g,
    // in one row, because two mentions of one Food are one thing to buy.
    assert_eq!(
        amounts(row(list, "tipo 00 flour")),
        vec!["about 400 g"],
        "the dough's half and the pizza's own, merged"
    );
    assert_eq!(
        amounts(row(list, "cold water")),
        vec!["about 200 ml"],
        "the dough's water, halved and then rounded for the jug in the drawer"
    );

    // **A Component contributes Foods, not a line of its own.** The written
    // line that used to sit here uninterpreted is gone, because what it stood
    // for is now on the list.
    assert!(
        !row_names(list)
            .iter()
            .any(|name| name == "Dough for 2 pizzas"),
        "the Component's own line comes off once it has opened: {:?}",
        row_names(list)
    );

    // The flour row breaks open to both written lines, each naming the recipe
    // it is really from — the dough by name, not the pizza (ADR 0002).
    let lines = row(list, "tipo 00 flour")["lines"].as_array().unwrap();
    let said = lines
        .iter()
        .map(|line| {
            (
                line["recipe"].as_str().unwrap().to_string(),
                line["text"].as_str().unwrap().to_string(),
            )
        })
        .collect::<Vec<_>>();
    assert!(
        said.contains(&(
            "Neapolitan Pizza Dough".to_string(),
            "600 g tipo 00 flour".to_string()
        )),
        "the dough's line says it is the dough's: {said:?}"
    );
    assert!(
        said.contains(&(
            "Pizza Margherita".to_string(),
            "100 g tipo 00 flour".to_string()
        )),
        "and the pizza's says it is the pizza's: {said:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_factor_compounds_through_nested_components_and_the_shopping_yield_scales_the_chain() {
    // ADR 0008: half of a dough that is itself half a starter is a quarter of
    // the starter. The Shopping Yield is the outer scale on top of all of it,
    // exactly as the cooking Yield is on the recipe page.
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );
    let (_starter, starter_lineage) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Levain",
        Some(("400", "g")),
        json!([{ "kind": "ingredient", "text": "200 g rye flour" }]),
        json!([{ "kind": "step", "text": "Feed it." }]),
    );
    let (pizza, dough, _lineage) = a_pizza_on_a_dough(&app, &key, &kitchen_id);
    // 200 g of a starter that makes 400 g: half of it, inside a dough the
    // pizza takes half of. A quarter of the starter reaches the pizza.
    let (status, saved) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": dough,
            "title": "Neapolitan Pizza Dough",
            "yield": { "amount": "1", "noun": "kg" },
            "ingredients": [
                { "kind": "ingredient", "text": "600 g tipo 00 flour" },
                { "kind": "ingredient", "text": "390 ml cold water" },
                { "kind": "ingredient", "text": "200 g levain" },
            ],
            "steps": [{ "kind": "step", "text": "Knead for ten minutes." }],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{saved}");
    make_component(
        &app,
        &key,
        &dough,
        2,
        Some("200"),
        Some("g"),
        &starter_lineage,
    );

    let (status, list) = app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    assert_eq!(status, 200, "{list}");
    assert_eq!(
        amounts(row(&list["result"], "rye flour")),
        vec!["about 50 g"],
        "200 g of rye, halved into the dough and halved again into the pizza"
    );

    // Four pizzas rather than two: the whole chain doubles, the nested level
    // included.
    let (status, doubled) = app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({
            "branch_id": pizza,
            "shopping_yield": { "amount": "4", "noun": "pizzas" },
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{doubled}");
    let doubled = &doubled["result"];
    assert_eq!(
        amounts(row(doubled, "rye flour")),
        vec!["about 100 g"],
        "the Shopping Yield is the outer scale over the compounded factor"
    );
    assert_eq!(
        amounts(row(doubled, "cold water")),
        vec!["about 390 ml"],
        "and it reaches the first level down just the same"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_component_kamosu_cannot_measure_puts_its_foods_on_the_list_unmeasured() {
    // Aurélien's call on #86. `2 poignées de pâte` cannot be compared with the
    // dough's Yield, so there is no honest factor — and rather than dropping
    // the dough's flour or inventing a figure for it, the list carries it in
    // the *some* bucket ADR 0024 already gives a line written with no quantity.
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );
    let (pizza, _dough, dough_lineage) = a_pizza_on_a_dough(&app, &key, &kitchen_id);
    // A quantity in nobody's units, against a Yield in kilos.
    make_component(
        &app,
        &key,
        &pizza,
        0,
        Some("2"),
        Some("poignées"),
        &dough_lineage,
    );

    let (status, list) = app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    assert_eq!(status, 200, "{list}");
    let list = &list["result"];

    // The dough's water is on the list and carries no number.
    assert_eq!(
        amounts(row(list, "cold water")),
        vec!["some"],
        "on the list, and honest about not being a quantity"
    );
    // And where it meets a measured amount, the row says both rather than one.
    assert_eq!(
        amounts(row(list, "tipo 00 flour")),
        vec!["about 100 g", "some"],
        "the pizza's own 100 g, and the dough's flour it could not work out"
    );
    // **The list says which it did**: the unmeasured part names the recipe it
    // came from, so the shopper can see it is the dough that is unaccounted.
    let unmeasured = row(list, "tipo 00 flour")["parts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|part| part["kind"] == json!("no_amount"))
        .expect("the part with no amount");
    assert_eq!(
        unmeasured["sources"],
        json!(["Neapolitan Pizza Dough"]),
        "and says which recipe it could not measure"
    );

    // Scaling the list cannot rescue a factor that was never worked out.
    let (_, doubled) = app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({
            "branch_id": pizza,
            "shopping_yield": { "amount": "4", "noun": "pizzas" },
        })
        .to_string(),
    );
    assert_eq!(
        amounts(row(&doubled["result"], "cold water")),
        vec!["some"],
        "twice nothing is still nothing"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn once_a_components_share_is_lost_everything_under_it_is_unmeasured_too() {
    // The other half of Aurélien's call on #86. A starter inside a dough the
    // pizza could not measure has its own perfectly good arithmetic — 200 g of
    // a 400 g levain is half of it — but there is nothing honest to multiply
    // that half by, because how much dough the pizza wants was never worked
    // out. Picking the inner chain's arithmetic back up would put a figure on
    // the list that rests on a guess Kamosu declined to make.
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );
    let (_starter, starter_lineage) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Levain",
        Some(("400", "g")),
        json!([{ "kind": "ingredient", "text": "200 g rye flour" }]),
        json!([{ "kind": "step", "text": "Feed it." }]),
    );
    let (pizza, dough, dough_lineage) = a_pizza_on_a_dough(&app, &key, &kitchen_id);
    let (status, saved) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": dough,
            "title": "Neapolitan Pizza Dough",
            "yield": { "amount": "1", "noun": "kg" },
            "ingredients": [
                { "kind": "ingredient", "text": "600 g tipo 00 flour" },
                { "kind": "ingredient", "text": "390 ml cold water" },
                { "kind": "ingredient", "text": "200 g levain" },
            ],
            "steps": [{ "kind": "step", "text": "Knead for ten minutes." }],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{saved}");
    // The starter inside the dough: measurable on its own terms.
    make_component(
        &app,
        &key,
        &dough,
        2,
        Some("200"),
        Some("g"),
        &starter_lineage,
    );
    // The dough inside the pizza: not measurable at all.
    make_component(
        &app,
        &key,
        &pizza,
        0,
        Some("2"),
        Some("poignées"),
        &dough_lineage,
    );

    let (status, list) = app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    assert_eq!(status, 200, "{list}");
    let list = &list["result"];

    // The rye is on the list — it does not vanish — and it carries no number,
    // rather than the 100 g its own half-of-a-half would have given.
    assert_eq!(
        amounts(row(list, "rye flour")),
        vec!["some"],
        "the level below an unmeasured Component is unmeasured too"
    );
    assert_eq!(
        row(list, "rye flour")["lines"][0]["recipe"],
        json!("Levain"),
        "and still says which recipe wants it"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_component_that_unfolds_to_nothing_keeps_its_written_line() {
    // The trap in taking a Component's line off once it has opened: a recipe
    // with no Ingredients at all opens perfectly well and contributes nothing,
    // so the line would come off with nothing to replace it and the pizza
    // would say nothing about its dough. ADR 0024's rule does not care how the
    // disappearing happened.
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );
    let (_empty, empty_lineage) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Sourdough Starter",
        Some(("1", "kg")),
        json!([{ "kind": "section", "text": "Nothing to buy" }]),
        json!([{ "kind": "step", "text": "Keep feeding what you already have." }]),
    );
    let (pizza, _dough, _lineage) = a_pizza_on_a_dough(&app, &key, &kitchen_id);
    make_component(
        &app,
        &key,
        &pizza,
        0,
        Some("500"),
        Some("g"),
        &empty_lineage,
    );

    let (status, list) = app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    assert_eq!(status, 200, "{list}");
    let names = row_names(&list["result"]);
    assert!(
        names.iter().any(|name| name == "Dough for 2 pizzas"),
        "nothing replaced it, so the written line stays: {names:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_component_whose_recipe_is_missing_keeps_its_written_line_on_the_list() {
    // ADR 0024: a thing that quietly disappears from a shopping list is a
    // thing that does not get bought. A Component that cannot be opened is the
    // one case where the written line is all there is, so it stays — quoted
    // whole, contributing nothing, exactly as it did before #86.
    //
    // A Lineage nothing in view holds is how this arrives in practice: a
    // Bundle received without its dough, or a sharing withdrawn. ADR 0008
    // needs no cascade and no ceremony for it, and neither does a list.
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );
    let (pizza, _dough, _lineage) = a_pizza_on_a_dough(&app, &key, &kitchen_id);
    make_component(
        &app,
        &key,
        &pizza,
        0,
        Some("500"),
        Some("g"),
        "l_never_received_here",
    );
    app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );

    let (status, list) = app.post_op("get_shopping_list", Some(&key), "{}");
    assert_eq!(status, 200, "{list}");
    let list = &list["result"];
    let names = row_names(list);
    assert!(
        names.iter().any(|name| name == "Dough for 2 pizzas"),
        "the written line is what is left, and it stays: {names:?}"
    );
    assert_eq!(
        amounts(row(list, "Dough for 2 pizzas")),
        Vec::<String>::new(),
        "quoted whole and contributing nothing"
    );
    // **And it says why it contributes nothing** (ADR 0008). Without this the
    // row reads exactly like a line Kamosu could not interpret, and nothing on
    // the list tells the shopper that the dough's flour is their own problem.
    assert_eq!(
        row(list, "Dough for 2 pizzas")["said"],
        json!("Kamosu does not have this recipe."),
        "worded once in the Core, so the recipe page says it the same way"
    );
    // A row Kamosu did read has nothing to say about itself.
    assert_eq!(row(list, "tipo 00 flour")["said"], Value::Null);
    assert!(
        !names.iter().any(|name| name == "cold water"),
        "and nothing of the recipe nobody holds is invented: {names:?}"
    );
    // The pizza's own flour is untouched by any of it.
    assert_eq!(amounts(row(list, "tipo 00 flour")), vec!["about 100 g"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_loop_of_components_stops_at_the_first_repeat_and_says_nothing_alarming() {
    // ADR 0008: cycles are never refused, unfolding stops. A loop can arrive
    // already formed from two halves on two servers, so the guard is here
    // rather than at the door — and a shopping list meeting one simply stops
    // rather than refusing to draw or warning about a thing nobody did wrong.
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );
    let (pizza, dough, _lineage) = a_pizza_on_a_dough(&app, &key, &kitchen_id);
    let pizza_lineage = app
        .post_op(
            "get_recipe",
            Some(&key),
            &json!({ "branch_id": pizza }).to_string(),
        )
        .1["result"]["lineage_id"]
        .as_str()
        .unwrap()
        .to_string();
    // The dough now names the pizza back.
    let (status, saved) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": dough,
            "title": "Neapolitan Pizza Dough",
            "yield": { "amount": "1", "noun": "kg" },
            "ingredients": [
                { "kind": "ingredient", "text": "600 g tipo 00 flour" },
                { "kind": "ingredient", "text": "390 ml cold water" },
                { "kind": "ingredient", "text": "1 pizza, torn up" },
            ],
            "steps": [{ "kind": "step", "text": "Knead for ten minutes." }],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{saved}");
    make_component(&app, &key, &dough, 2, Some("1"), None, &pizza_lineage);

    let (status, list) = app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    assert_eq!(status, 200, "the loop is drawn, not refused: {list}");
    let list = &list["result"];

    // It went round once and stopped: the dough's own lines are there, and the
    // pizza's did not arrive a second time.
    assert_eq!(amounts(row(list, "cold water")), vec!["about 200 ml"]);
    assert_eq!(
        amounts(row(list, "tipo 00 flour")),
        vec!["about 400 g"],
        "the pizza's 100 g and the dough's halved 600 g, counted once each"
    );
    // The line that closed the loop is left as the words somebody wrote —
    // which is the whole of what the list says about it.
    let names = row_names(list);
    assert!(
        names.iter().any(|name| name == "1 pizza, torn up"),
        "the repeated line stays as written: {names:?}"
    );
    assert_eq!(
        amounts(row(list, "1 pizza, torn up")),
        Vec::<String>::new(),
        "no amount, and no refusal"
    );
    // It says where it stopped, calmly. ADR 0008 asks unfolding to stop at a
    // repeat *saying so*, on a list as much as on a page — and a plain
    // sentence is not the alarm #86 asked it not to raise.
    assert_eq!(
        row(list, "1 pizza, torn up")["said"],
        json!("Pizza Margherita is already open above — Kamosu stops here.")
    );
}

/// **The invariant, asked of a real database after every path that writes a
/// Version** (#89, ADR 0038).
///
/// A Version's id is the fingerprint of its content, and everything that asks
/// whether two people hold the same recipe asks it by comparing ids. That
/// question is only answerable by comparison for as long as the answer is true
/// of every row — so this drives each way a Version gets written and then puts
/// #89's own reproduction recipe to the database in one line.
///
/// It catches what a unit test cannot: a path that stores a Version under an
/// id it did not compute from that Version's content. There is no such path
/// today, and the day somebody adds one — receiving a Bundle from another
/// instance is the obvious candidate — this is what says so.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_version_a_door_writes_fingerprints_to_its_own_id() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, kitchen_id) = recipe_ready_to_cook(&app, "Aurélien");

    // An ordinary edit, appending a Version to the Branch.
    age_branch_head(&app, &branch_id);
    let mut edited = katsu_as_written();
    edited["branch_id"] = json!(branch_id);
    edited["steps"][1]["text"] = json!("Frire à 170 °C jusqu'à dorer");
    let (status, saved) = app.post_op("save_recipe_version", Some(&key), &edited.to_string());
    assert_eq!(status, 200, "{saved}");

    // The same recipe edited by a second Kitchen, which starts a Copy.
    let (_, other_key, other_kitchen) = person_with_kitchen(&app, "Marc");
    let (_, invited) = app.post_op(
        "invite_to_kitchen",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id }).to_string(),
    );
    let invite = invited["result"]["secret"].as_str().unwrap().to_string();
    app.post_op(
        "accept_kitchen_invite",
        Some(&other_key),
        &json!({ "secret": invite }).to_string(),
    );
    let mut theirs = katsu_as_written();
    theirs["branch_id"] = json!(branch_id);
    theirs["kitchen_id"] = json!(other_kitchen);
    theirs["steps"][2]["text"] = json!("Servir avec le riz et du chou");
    let (status, copied) =
        app.post_op("save_recipe_version", Some(&other_key), &theirs.to_string());
    assert_eq!(status, 200, "{copied}");

    // A cooking that deviated, which stores a Version joined to no Branch.
    let attempt_id = cooking_katsu(&app, &key, &branch_id);
    let mut cooked = katsu_as_written();
    cooked["ingredients"][1]["text"] = json!("300 g de riz");
    let (status, deviated) = app.post_op(
        "set_as_cooked",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "as_cooked": cooked }).to_string(),
    );
    assert_eq!(status, 200, "{deviated}");
    assert_ne!(
        deviated["result"]["as_cooked"],
        json!(null),
        "the rice was changed, so this must have stored a Version"
    );

    // And that cooking promoted into the recipe.
    age_branch_head(&app, &branch_id);
    let (status, promoted) = app.post_op(
        "promote_as_cooked",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 200, "{promoted}");

    // A recipe arriving from somewhere else entirely.
    let arrived = import_and_wait(
        &app,
        &key,
        &json!({
            "source_kind": "crouton",
            "candidates": [{ "foreign_id": "crouton-uuid-1", "title": "Ratatouille" }],
        }),
    );
    assert_eq!(
        arrived["arrived"][0]["status"],
        json!("created"),
        "{arrived}"
    );

    // And a Bundle from another instance: the first path where a Version id
    // arrives from outside rather than being computed here (#67).
    let there = support::spawn_app();
    let (their_key, _kitchen, pizza, _lineage, _french, _dough, _photos) =
        a_pizza_worth_sending(&there);
    let report = receive(&app, &key, &bundle_of(&there, &their_key, &pizza));
    assert_eq!(
        arrived_row(&report, &pizza)["status"],
        json!("created"),
        "{report}"
    );

    app.core
        .db()
        .with_conn(|conn| {
            let (total, wrong): (i64, i64) = conn
                .query_row(
                    "SELECT COUNT(*), COUNT(*) FILTER (WHERE id <> version_fingerprint(content)) \
                     FROM versions",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            assert!(
                total >= 5,
                "the paths above must actually have written Versions, found {total}"
            );
            assert_eq!(
                wrong, 0,
                "every Version's id must be the fingerprint of its own content"
            );
            Ok(())
        })
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cooking_as_written_stores_no_as_cooked_at_all() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let attempt_id = cooking_katsu(&app, &key, &branch_id);

    // A screen that helpfully posts the whole recipe back unchanged must not
    // create anything. The fingerprint says this is the Version already cooked.
    let (status, answered) = app.post_op(
        "set_as_cooked",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "as_cooked": katsu_as_written() }).to_string(),
    );
    assert_eq!(status, 200, "{answered}");
    assert_eq!(
        answered["result"]["as_cooked"],
        json!(null),
        "cooked as written is not a deviation, however it was sent"
    );
    assert_eq!(
        versions_on_no_branch(&app),
        0,
        "the common case stores no state whatever"
    );

    // And explicitly saying so is the same answer.
    let (_, cleared) = app.post_op(
        "set_as_cooked",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "as_cooked": null }).to_string(),
    );
    assert_eq!(cleared["result"]["as_cooked"], json!(null));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_as_cooked_is_a_whole_recipe_of_words_rather_than_a_diff() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let attempt_id = cooking_katsu(&app, &key, &branch_id);

    // One amount rewritten, one step reworded. Everything else untouched.
    let mut cooked = katsu_as_written();
    cooked["ingredients"][1]["text"] = json!("150 g de riz");
    cooked["steps"][1]["text"] = json!("Frire jusqu'à dorer, cinq minutes de plus");

    let (status, answered) = app.post_op(
        "set_as_cooked",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "as_cooked": cooked }).to_string(),
    );
    assert_eq!(status, 200, "{answered}");
    let as_cooked = &answered["result"]["as_cooked"];

    // The deviations are ordinary written lines, in the recipe's own shape —
    // there is no second, structured vocabulary for "what changed" (ADR 0002).
    let content = &as_cooked["content"];
    assert_eq!(content["ingredients"][1]["text"], json!("150 g de riz"));
    assert_eq!(
        content["steps"][1]["text"],
        json!("Frire jusqu'à dorer, cinq minutes de plus")
    );

    // And the lines nobody touched are there too, which is the whole point:
    // this is a recipe you could stand in, not a list of differences.
    assert_eq!(
        content["ingredients"][0]["text"],
        json!("2 escalopes de poulet")
    );
    assert_eq!(content["steps"][0]["text"], json!("Paner les escalopes"));
    assert_eq!(content["steps"][2]["text"], json!("Servir avec le riz"));
    assert_eq!(content["title"], json!("Katsu Curry"));

    // Structurally identical to a Version, and named the same way — but on no
    // Branch, which is the only difference there is (ADR 0005).
    assert!(as_cooked["version_id"].as_str().unwrap().starts_with("v_"));
    assert_eq!(
        versions_on_no_branch(&app),
        1,
        "an As Cooked is a Version that never joined a Branch"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_as_cooked_can_add_a_line_drop_one_and_grow_a_step() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let attempt_id = cooking_katsu(&app, &key, &branch_id);

    // Threw in a bay leaf, skipped the rice, and the method grew a stage.
    // None of these is expressible as "this line, this new amount", which is
    // exactly why the As Cooked holds a whole recipe.
    let cooked = json!({
        "title": "Katsu Curry",
        "ingredients": [
            { "kind": "ingredient", "text": "2 escalopes de poulet" },
            { "kind": "ingredient", "text": "1 feuille de laurier" },
        ],
        "steps": [
            { "kind": "step", "text": "Paner les escalopes" },
            { "kind": "step", "text": "Laisser reposer la panure vingt minutes" },
            { "kind": "step", "text": "Frire jusqu'à dorer" },
            { "kind": "step", "text": "Servir avec le riz" },
        ],
    });

    let (status, answered) = app.post_op(
        "set_as_cooked",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "as_cooked": cooked }).to_string(),
    );
    assert_eq!(status, 200, "{answered}");
    let content = &answered["result"]["as_cooked"]["content"];

    assert_eq!(
        content["ingredients"].as_array().unwrap().len(),
        2,
        "the rice was dropped and a bay leaf added"
    );
    assert_eq!(
        content["ingredients"][1]["text"],
        json!("1 feuille de laurier")
    );
    assert_eq!(
        content["steps"].as_array().unwrap().len(),
        4,
        "a step was inserted before the frying"
    );
    assert_eq!(
        content["steps"][1]["text"],
        json!("Laisser reposer la panure vingt minutes")
    );

    // The recipe itself has not moved an inch. Recording a cooking never
    // changes what the author wrote.
    let (_, recipe) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let head = recipe["result"]["versions"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    assert_eq!(head["content"]["ingredients"].as_array().unwrap().len(), 2);
    assert_eq!(
        head["content"]["ingredients"][1]["text"],
        json!("200 g de riz")
    );
    assert_eq!(head["content"]["steps"].as_array().unwrap().len(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn promotion_makes_an_ordinary_version_and_mints_no_new_identity() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let attempt_id = cooking_katsu(&app, &key, &branch_id);

    let mut cooked = katsu_as_written();
    cooked["ingredients"][1]["text"] = json!("150 g de riz");
    let (_, deviated) = app.post_op(
        "set_as_cooked",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "as_cooked": cooked }).to_string(),
    );
    let as_cooked_version_id = deviated["result"]["as_cooked"]["version_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Hours later, at the kitchen table.
    age_branch_head(&app, &branch_id);

    let (status, promoted) = app.post_op(
        "promote_as_cooked",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 200, "{promoted}");

    // Nothing was minted: the As Cooked's fingerprint IS the new Version's id.
    assert_eq!(
        promoted["result"]["version_id"],
        json!(as_cooked_version_id),
        "promotion mints no new identity — the fingerprint already named this"
    );
    assert_eq!(promoted["result"]["branch_id"], json!(branch_id));
    assert_eq!(
        promoted["result"]["copied"],
        json!(false),
        "promoting into your own Kitchen's Branch is an ordinary save"
    );

    // The recipe now reads as it was cooked, with no retyping anywhere.
    let (_, recipe) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(
        recipe["result"]["head_version_id"],
        json!(as_cooked_version_id)
    );
    let head = recipe["result"]["versions"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    assert_eq!(
        head["content"]["ingredients"][1]["text"],
        json!("150 g de riz")
    );

    // The Version joined the Branch — which is the whole of how "already
    // promoted" is answered, with no flag anywhere to disagree with the chain.
    assert_eq!(versions_on_no_branch(&app), 0);
    assert_eq!(
        recipe["result"]["versions"].as_array().unwrap().len(),
        2,
        "an appended Version, not a rewritten one"
    );

    // And the cooking is untouched: it still says which Version it cooked.
    let (_, after) = app.post_op("list_attempts", Some(&key), &json!({}).to_string());
    let entry = &after["result"]["attempts"][0];
    assert_eq!(entry["id"], json!(attempt_id));
    assert_ne!(
        entry["version_id"],
        json!(as_cooked_version_id),
        "the Attempt goes on pinning to the Version that was on screen at the time"
    );
    assert_eq!(
        entry["as_cooked"]["version_id"],
        json!(as_cooked_version_id),
        "promoting is not moving: the cooking keeps its own record"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn promoting_a_cooking_of_an_older_version_appends_rather_than_merging() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let attempt_id = cooking_katsu(&app, &key, &branch_id);

    // The cook deviates on Tuesday's text...
    let mut cooked = katsu_as_written();
    cooked["steps"][2]["text"] = json!("Servir avec le riz et du chou râpé");
    let (_, deviated) = app.post_op(
        "set_as_cooked",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "as_cooked": cooked }).to_string(),
    );
    let as_cooked_version_id = deviated["result"]["as_cooked"]["version_id"]
        .as_str()
        .unwrap()
        .to_string();

    // ...and meanwhile the recipe itself moves on.
    age_branch_head(&app, &branch_id);
    let mut edited = katsu_as_written();
    edited["branch_id"] = json!(branch_id);
    edited["title"] = json!("Katsu Curry maison");
    let (status, saved) = app.post_op("save_recipe_version", Some(&key), &edited.to_string());
    assert_eq!(status, 200, "{saved}");
    let moved_on = saved["result"]["version_id"].as_str().unwrap().to_string();

    age_branch_head(&app, &branch_id);
    let (status, promoted) = app.post_op(
        "promote_as_cooked",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 200, "{promoted}");

    // It is a Version, appended onto where the Branch stands now — not a merge.
    assert_eq!(
        promoted["result"]["parent_version_id"],
        json!(moved_on),
        "promotion appends onto the current head, whatever Version was cooked"
    );
    assert_eq!(
        promoted["result"]["version_id"],
        json!(as_cooked_version_id)
    );

    // Nothing was combined: the promoted text is the cook's, whole, and the
    // title the recipe gained meanwhile is simply gone — which is what "a
    // Version, not a merge" means (ADR 0004).
    let (_, recipe) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let head = recipe["result"]["versions"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    assert_eq!(
        head["content"]["steps"][2]["text"],
        json!("Servir avec le riz et du chou râpé")
    );
    assert_eq!(
        head["content"]["title"],
        json!("Katsu Curry"),
        "the whole state the cook cooked is what lands — nothing is reconciled"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn promoting_a_cooking_that_deviated_from_nothing_is_refused() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let attempt_id = cooking_katsu(&app, &key, &branch_id);

    let (status, refused) = app.post_op(
        "promote_as_cooked",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["ok"], json!(false));
    assert!(
        refused["error"]["message"]
            .as_str()
            .unwrap()
            .contains("cooked as written"),
        "{refused}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_read_only_access_key_can_neither_write_an_as_cooked_nor_promote_one() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let attempt_id = cooking_katsu(&app, &key, &branch_id);

    let person = app
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row(
                "SELECT person_id FROM attempts WHERE id = ?1",
                rusqlite::params![attempt_id],
                |r| r.get::<_, String>(0),
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .unwrap();
    let read_only = app
        .core
        .mint_access_key(&person, "read-only agent", true)
        .unwrap()
        .secret;

    // The refusal comes from the Catalogue's `write: true`, in the Core, before
    // either Operation runs — so it reads the same as every other write a
    // read-only Key is refused, and neither of these had to remember it.
    let (status, refused) = app.post_op(
        "set_as_cooked",
        Some(&read_only),
        &json!({ "attempt_id": attempt_id, "as_cooked": katsu_as_written() }).to_string(),
    );
    assert_eq!(status, 401, "{refused}");
    assert!(
        refused["error"]["message"]
            .as_str()
            .unwrap()
            .contains("read-only Access Key"),
        "{refused}"
    );

    let (status, refused) = app.post_op(
        "promote_as_cooked",
        Some(&read_only),
        &json!({ "attempt_id": attempt_id, "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 401, "{refused}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_as_cooked_is_read_against_the_version_cooked_by_the_one_pairing() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let attempt_id = cooking_katsu(&app, &key, &branch_id);

    // One line rewritten, one dropped, one added, and a step inserted.
    let cooked = json!({
        "title": "Katsu Curry",
        "ingredients": [
            { "kind": "ingredient", "text": "3 escalopes de poulet" },
            { "kind": "ingredient", "text": "1 feuille de laurier" },
        ],
        "steps": [
            { "kind": "step", "text": "Paner les escalopes" },
            { "kind": "step", "text": "Laisser reposer la panure vingt minutes" },
            { "kind": "step", "text": "Frire jusqu'à dorer" },
            { "kind": "step", "text": "Servir avec le riz" },
        ],
    });
    let (status, answered) = app.post_op(
        "set_as_cooked",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "as_cooked": cooked }).to_string(),
    );
    assert_eq!(status, 200, "{answered}");

    // The reading is the Core's, by the same Pairing two Branches use — no
    // screen anywhere works out which line became which (ADR 0019).
    let against = &answered["result"]["as_cooked"]["against"];
    let states = |rows: &Value| -> Vec<String> {
        rows.as_array()
            .unwrap()
            .iter()
            .map(|row| row["state"].as_str().unwrap().to_string())
            .collect()
    };

    // The added line sits where the cook put it — anchored after the escalopes
    // it follows — and the dropped one is a Ghost in the position it held in
    // the recipe. That ordering is the Pairing's, not this ticket's.
    assert_eq!(
        states(&against["ingredients"]),
        vec!["changed", "only-theirs", "only-mine"],
        "the escalopes were rewritten, the laurier added, the rice dropped"
    );
    assert_eq!(
        states(&against["steps"]),
        vec!["same", "only-theirs", "same", "same"],
        "one step inserted, the other three left exactly alone"
    );

    // A dropped line is still readable, and says what it said.
    let dropped = &against["ingredients"][2];
    assert_eq!(dropped["mine"]["text"], json!("200 g de riz"));
    assert_eq!(dropped["theirs"], json!(null));

    // A cooking that followed the recipe has nothing to read at all.
    let (_, cleared) = app.post_op(
        "set_as_cooked",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "as_cooked": null }).to_string(),
    );
    assert_eq!(cleared["result"]["as_cooked"], json!(null));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn promotion_keeps_the_name_a_version_was_given_and_never_un_names_it() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let attempt_id = cooking_katsu(&app, &key, &branch_id);

    // A Version somebody named. Promoting inside the collapse window folds into
    // this very Version, and a save naming nothing writes the name away — the
    // trap `promote_attempt_photograph` records next door.
    let mut named = katsu_as_written();
    named["branch_id"] = json!(branch_id);
    named["name"] = json!("Sunday version");
    // Different words, or the save is the identical state the fingerprint
    // already names and there is nothing to write a name onto.
    named["steps"][2]["text"] = json!("Servir avec le riz, bien chaud");
    let (status, saved) = app.post_op("save_recipe_version", Some(&key), &named.to_string());
    assert_eq!(status, 200, "{saved}");
    let (_, before) = app.post_op(
        "get_thread",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(
        before["result"]["versions"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()["name"],
        json!("Sunday version"),
        "the Version is named before anything is promoted: {before}"
    );

    let mut cooked = katsu_as_written();
    cooked["ingredients"][1]["text"] = json!("150 g de riz");
    let (_, deviated) = app.post_op(
        "set_as_cooked",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "as_cooked": cooked }).to_string(),
    );
    assert_ne!(deviated["result"]["as_cooked"], json!(null), "{deviated}");

    let (status, promoted) = app.post_op(
        "promote_as_cooked",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 200, "{promoted}");
    assert_eq!(
        promoted["result"]["collapsed"],
        json!(true),
        "this promotion is inside the collapse window, which is what makes the name losable"
    );

    let (_, thread) = app.post_op(
        "get_thread",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let head = thread["result"]["versions"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    assert_eq!(
        head["name"],
        json!("Sunday version"),
        "promoting a cooking must not quietly un-name the Version it folds into"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn promoting_into_another_kitchens_branch_takes_a_copy() {
    let app = support::spawn_app();
    let (key, branch_id, lineage_id, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let attempt_id = cooking_katsu(&app, &key, &branch_id);

    let mut cooked = katsu_as_written();
    cooked["steps"][0]["text"] = json!("Paner les escalopes deux fois");
    let (_, deviated) = app.post_op(
        "set_as_cooked",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "as_cooked": cooked }).to_string(),
    );
    let as_cooked_version_id = deviated["result"]["as_cooked"]["version_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Camille cooks Aurélien's recipe in her own Kitchen. An Attempt belongs to
    // the Lineage, so hers is legitimate — and promoting it must not write on
    // his Branch. `save_recipe_version` already knows this; promotion inherits
    // the whole of an ordinary edit, Copy included.
    let (camille, camille_key, camille_kitchen) = person_with_kitchen(&app, "Camille");
    let _ = camille;
    let (_, copied) = app.post_op(
        "save_recipe_version",
        Some(&camille_key),
        &json!({
            "branch_id": branch_id,
            "kitchen_id": camille_kitchen,
            "title": "Katsu Curry",
            "ingredients": [
                { "kind": "ingredient", "text": "2 escalopes de poulet" },
                { "kind": "ingredient", "text": "200 g de riz" },
                { "kind": "ingredient", "text": "1 c. à s. de sauce tonkatsu" },
            ],
            "steps": [
                { "kind": "step", "text": "Paner les escalopes" },
                { "kind": "step", "text": "Frire jusqu'à dorer" },
                { "kind": "step", "text": "Servir avec le riz" },
            ],
        })
        .to_string(),
    );
    let hers = copied["result"]["branch_id"].as_str().unwrap().to_string();
    assert_eq!(copied["result"]["copied"], json!(true), "{copied}");
    assert_ne!(hers, branch_id);

    // Aurélien promotes his own cooking into HER Branch. He does not cook in her
    // Kitchen, so this starts a Branch of his rather than writing on hers.
    age_branch_head(&app, &hers);
    let (status, promoted) = app.post_op(
        "promote_as_cooked",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "branch_id": hers }).to_string(),
    );
    assert_eq!(status, 200, "{promoted}");
    assert_eq!(
        promoted["result"]["copied"],
        json!(true),
        "promoting into a Kitchen you do not cook in is a Copy, like any other edit"
    );
    assert_ne!(promoted["result"]["branch_id"], json!(hers));
    assert_eq!(
        promoted["result"]["version_id"],
        json!(as_cooked_version_id)
    );

    // Her Branch is untouched, which is the whole point of a Copy.
    let (_, thread) = app.post_op(
        "get_thread",
        Some(&camille_key),
        &json!({ "branch_id": hers }).to_string(),
    );
    let still = thread["result"]["branches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|branch| branch["branch_id"] == json!(hers))
        .expect("her Branch");
    assert_ne!(still["head_version_id"], json!(as_cooked_version_id));
    let _ = lineage_id;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn another_cooks_as_cooked_never_leaves_the_core() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, kitchen_id) = recipe_ready_to_cook(&app, "Aurélien");

    // Camille joins the Kitchen and cooks the dish her own way, and does not
    // finish — an In Progress Attempt is "visible to its cook alone" (ADR 0010).
    let (camille, camille_key, _her_kitchen) = person_with_kitchen(&app, "Camille");
    let _ = camille;
    let (_, minted) = app.post_op(
        "invite_to_kitchen",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id }).to_string(),
    );
    let secret = minted["result"]["secret"].as_str().unwrap().to_string();
    let (status, joined) = app.post_op(
        "accept_kitchen_invite",
        Some(&camille_key),
        &json!({ "secret": secret }).to_string(),
    );
    assert_eq!(status, 200, "{joined}");
    let hers = cooking_katsu(&app, &camille_key, &branch_id);
    let mut cooked = katsu_as_written();
    cooked["ingredients"][0]["text"] = json!("4 escalopes de poulet");
    let (status, deviated) = app.post_op(
        "set_as_cooked",
        Some(&camille_key),
        &json!({ "attempt_id": hers, "as_cooked": cooked }).to_string(),
    );
    assert_eq!(status, 200, "{deviated}");

    // Aurélien reads the Thread. Her cooking is on it — an Attempt names its
    // cook and inherits the recipe's visibility — but the words she wrote at
    // her own stove are not.
    let (_, thread) = app.post_op(
        "get_thread",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let attempts = thread["result"]["attempts"].as_array().unwrap();
    let seen = attempts
        .iter()
        .find(|attempt| attempt["id"] == json!(hers))
        .expect("her cooking is on the Thread");
    assert_eq!(
        seen["as_cooked"],
        json!(null),
        "the count and the date travel; the recipe she wrote does not"
    );

    // And she still has it, whole.
    let (_, mine) = app.post_op(
        "get_thread",
        Some(&camille_key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let ownself = mine["result"]["attempts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|attempt| attempt["id"] == json!(hers))
        .expect("her own cooking");
    assert_eq!(
        ownself["as_cooked"]["content"]["ingredients"][0]["text"],
        json!("4 escalopes de poulet")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_promotion_declined_stays_declined_and_keeps_what_was_cooked() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let attempt_id = cooking_katsu(&app, &key, &branch_id);

    let mut cooked = katsu_as_written();
    cooked["ingredients"][1]["text"] = json!("150 g de riz");
    let (_, deviated) = app.post_op(
        "set_as_cooked",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "as_cooked": cooked }).to_string(),
    );
    assert_eq!(
        deviated["result"]["as_cooked"]["promotion_declined"],
        json!(false),
        "nobody has been asked yet"
    );

    let (status, declined) = app.post_op(
        "decline_promotion",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "declined": true }).to_string(),
    );
    assert_eq!(status, 200, "{declined}");
    assert_eq!(
        declined["result"]["as_cooked"]["promotion_declined"],
        json!(true)
    );

    // Answering the offer says nothing about what was cooked, which is the
    // point: the diary keeps every word of it.
    assert_eq!(
        declined["result"]["as_cooked"]["content"]["ingredients"][1]["text"],
        json!("150 g de riz")
    );
    // And it survives being read back — a question already answered, asked
    // twice, is a nag.
    let (_, read) = app.post_op(
        "get_thread",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let entry = &read["result"]["attempts"][0];
    assert_eq!(entry["as_cooked"]["promotion_declined"], json!(true));

    // Changing your mind is an ordinary edit of your own Attempt (ADR 0005),
    // and promoting after that works exactly as it would have.
    let (_, undone) = app.post_op(
        "decline_promotion",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "declined": false }).to_string(),
    );
    assert_eq!(
        undone["result"]["as_cooked"]["promotion_declined"],
        json!(false)
    );
    age_branch_head(&app, &branch_id);
    let (status, promoted) = app.post_op(
        "promote_as_cooked",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 200, "{promoted}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cook_who_inserted_a_step_can_stand_on_every_one_of_them() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let attempt_id = cooking_katsu(&app, &key, &branch_id);

    // The recipe has three steps, so index 3 is off the end of it.
    let (status, refused) = app.post_op(
        "advance_attempt",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "current_step_index": 3 }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");

    // A method that grew a stage. `current_step_index` indexes what this
    // cooking is walking through, which is the As Cooked once there is one —
    // otherwise the last step a cook inserted is one they cannot stand on, and
    // the screen would put the refusal back without saying anything.
    let mut cooked = katsu_as_written();
    cooked["steps"] = json!([
        { "kind": "step", "text": "Paner les escalopes" },
        { "kind": "step", "text": "Laisser reposer la panure vingt minutes" },
        { "kind": "step", "text": "Frire jusqu'à dorer" },
        { "kind": "step", "text": "Servir avec le riz" },
    ]);
    let (status, deviated) = app.post_op(
        "set_as_cooked",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "as_cooked": cooked }).to_string(),
    );
    assert_eq!(status, 200, "{deviated}");

    let (status, advanced) = app.post_op(
        "advance_attempt",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "current_step_index": 3 }).to_string(),
    );
    assert_eq!(status, 200, "{advanced}");
    assert_eq!(advanced["result"]["current_step_index"], json!(3));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cooking_as_written_stores_nothing_even_where_the_version_id_has_drifted() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let attempt_id = cooking_katsu(&app, &key, &branch_id);

    // A Version's id is the fingerprint of its content AT THE MOMENT IT WAS
    // SAVED. A migration that adds a field to the content shape moves the
    // content and leaves the id where it was — which is not hypothetical: on
    // the dev instance 27 of 40 Versions are already in exactly this state,
    // found by cooking a real recipe as written and watching it store one.
    //
    // Here that history is forced: the stored content is rewritten behind the
    // id, the way a migration would.
    let version_id = app
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row(
                "SELECT version_id FROM attempts WHERE id = ?1",
                rusqlite::params![attempt_id],
                |row| row.get::<_, String>(0),
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .unwrap();
    app.core
        .db()
        .with_conn(|conn| {
            let stored: String = conn
                .query_row(
                    "SELECT content FROM versions WHERE id = ?1",
                    rusqlite::params![version_id],
                    |row| row.get(0),
                )
                .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            let mut content: Value = serde_json::from_str(&stored).unwrap();
            content["a_field_a_later_migration_added"] = json!(null);
            conn.execute(
                "UPDATE versions SET content = ?2 WHERE id = ?1",
                rusqlite::params![version_id, content.to_string()],
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            Ok(())
        })
        .expect("rewrite the stored content behind its id");

    // The cook cooks it exactly as written and the screen posts the recipe
    // back. Nothing about this afternoon was a deviation, and nothing is
    // stored — which is what makes ordinary cooking free (ADR 0005).
    let (status, answered) = app.post_op(
        "set_as_cooked",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "as_cooked": katsu_as_written() }).to_string(),
    );
    assert_eq!(status, 200, "{answered}");
    assert_eq!(
        answered["result"]["as_cooked"],
        json!(null),
        "a Version whose id no longer fingerprints its own content must not turn \
         every cooking of it into a deviation"
    );
}

// ---------------------------------------------------------------------------
// Backups (#78, ADR 0039): one archive of the whole instance, three kept at
// three distances, never sent anywhere.
// ---------------------------------------------------------------------------

/// Take a Backup through the Web Door and answer the Job's result. Copying a
/// whole library is a Job, so asking answers a job id and the archive is
/// finished by the time `get_job` says so.
fn take_a_backup(app: &support::TestApp, key: &str) -> Value {
    let (status, asked) = app.post_op("take_backup", Some(key), "{}");
    assert_eq!(status, 200, "{asked}");
    let job_id = asked["result"]["job_id"].as_str().expect("a job id");
    let finished = support::wait_terminal(app, Some(key), job_id);
    assert_eq!(finished["status"], json!("completed"), "{finished}");
    finished["result"].clone()
}

/// Every entry name inside one archive, read off the disk as an Operator
/// restoring an instance would read it.
fn entries_in(path: &std::path::Path) -> Vec<String> {
    let file = std::fs::File::open(path).expect("open the Backup");
    let mut archive = zip::ZipArchive::new(file).expect("a readable zip");
    (0..archive.len())
        .map(|at| archive.by_index(at).expect("an entry").name().to_string())
        .collect()
}

/// Move one archive back in time by renaming it, which is the only clock a
/// test can wind: what is owed is read off the names, never a table.
fn backdate_archive(app: &support::TestApp, name: &str, days: i64) {
    let dir = app.core.data_dir().join("backups");
    let stamp: String = app
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row(
                "SELECT strftime('%Y%m%dT%H%M%SZ','now',?1)",
                rusqlite::params![format!("-{days} days")],
                |row| row.get(0),
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .expect("a stamp");
    let (slot, _) = name
        .strip_prefix("kamosu-backup-")
        .and_then(|rest| rest.strip_suffix(".zip"))
        .and_then(|rest| rest.split_once('-'))
        .expect("a Backup's name");
    let moved = format!("kamosu-backup-{slot}-{stamp}.zip");
    std::fs::rename(dir.join(name), dir.join(&moved)).expect("wind the clock back");
}

/// The one archive holding `slot`, by name.
fn archive_named(app: &support::TestApp, key: &str, slot: &str) -> String {
    let held = names_held(app, key);
    let mut matching = held
        .iter()
        .filter(|name| name.contains(&format!("-{slot}-")));
    let found = matching
        .next()
        .unwrap_or_else(|| panic!("no {slot} archive in {held:?}"));
    assert!(
        matching.next().is_none(),
        "more than one {slot} archive: {held:?}"
    );
    found.clone()
}

fn names_held(app: &support::TestApp, key: &str) -> Vec<String> {
    let (status, listed) = app.post_op("list_backups", Some(key), "{}");
    assert_eq!(status, 200, "{listed}");
    listed["result"]["backups"]
        .as_array()
        .expect("a list of Backups")
        .iter()
        .map(|backup| backup["name"].as_str().expect("a name").to_string())
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_backup_is_one_archive_of_the_database_and_the_photographs() {
    let app = support::spawn_app();
    let (key, kitchen_id) = operator_with_kitchen(&app);

    // A recipe with a picture, so there is something in both halves of the
    // archive — and a Display Copy drawn, so the test can prove it stayed out.
    let photograph = upload_a_picture(&app, &key, 7);
    let (status, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Katsu", "main_photo": photograph })
            .to_string(),
    );
    assert_eq!(status, 200, "{created}");
    let (card_status, _, _) =
        app.get_bytes(&format!("/api/photographs/{photograph}/card"), Some(&key));
    assert_eq!(card_status, 200);

    take_a_backup(&app, &key);
    let name = archive_named(&app, &key, "daily");
    let path = app.core.data_dir().join("backups").join(&name);

    // Written under `/data`, beside the database rather than anywhere else.
    assert!(
        path.starts_with(app.data_dir().expect("the helper owns a temp dir")),
        "a Backup must live under the one data directory: {}",
        path.display()
    );
    assert!(path.is_file(), "the Backup named was not written");

    let entries = entries_in(&path);
    assert!(
        entries.contains(&"kamosu.db".to_string()),
        "an archive without the database restores nothing: {entries:?}"
    );
    assert!(
        entries.contains(&format!("photographs/{photograph}.webp")),
        "the Photograph is missing from the archive: {entries:?}"
    );
    // Display Copies and Covers are rebuildable, so carrying them would roughly
    // double what an Operator stores for nothing (ADR 0017, ADR 0039).
    assert!(
        !entries
            .iter()
            .any(|entry| entry.starts_with("display/") || entry.starts_with("cards/")),
        "Display Copies must not be backed up: {entries:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_database_inside_a_backup_is_a_working_kamosu_database() {
    let app = support::spawn_app();
    let (key, kitchen_id) = operator_with_kitchen(&app);
    let (status, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Tonkatsu" }).to_string(),
    );
    assert_eq!(status, 200, "{created}");

    take_a_backup(&app, &key);
    let restored = restore(&app, &archive_named(&app, &key, "daily"));

    // The recipe is there, under a Kamosu that opened the file for itself.
    let titles: Vec<String> = restored
        .with_conn(|conn| {
            let mut statement = conn
                .prepare("SELECT json_extract(content, '$.title') FROM versions")
                .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            let rows = statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            Ok(rows.filter_map(Result::ok).collect())
        })
        .expect("read the restored library");
    assert!(
        titles.contains(&"Tonkatsu".to_string()),
        "the restored database does not hold the recipe: {titles:?}"
    );
}

/// Unzip one Backup into a directory of its own and open the database in it,
/// which is exactly what restoring an instance is.
fn restore(app: &support::TestApp, name: &str) -> kamosu::db::Db {
    let path = app.core.data_dir().join("backups").join(name);
    let into = app
        .data_dir()
        .expect("the helper owns a temp dir")
        .join(format!("restored-{name}"));
    std::fs::create_dir_all(&into).expect("somewhere to restore into");
    let file = std::fs::File::open(&path).expect("open the Backup");
    let mut archive = zip::ZipArchive::new(file).expect("a readable zip");
    archive.extract(&into).expect("unzip over a fresh /data");
    kamosu::db::Db::open(&into).expect("the restored database opens")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_backup_taken_during_writes_restores_to_a_consistent_database() {
    let app = support::spawn_app();
    let (key, kitchen_id) = operator_with_kitchen(&app);
    let (status, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Under the knife" }).to_string(),
    );
    assert_eq!(status, 200, "{created}");
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    // Real saves, through a real Door, for as long as the Backup takes. The
    // archive reads the database on a second connection, so these are not
    // politely waiting their turn — which is the whole point of the test.
    let writing = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let stop = writing.clone();
    let addr = app.addr;
    let writer_key = key.clone();
    let writer = std::thread::spawn(move || {
        let mut written = 0;
        while stop.load(std::sync::atomic::Ordering::Relaxed) {
            written += 1;
            let body = json!({
                "branch_id": branch_id,
                "title": format!("Under the knife {written}"),
                "steps": [{ "kind": "step", "text": format!("Save number {written}") }],
            })
            .to_string();
            let _ = kamosu::http_min::post_json(
                addr,
                "/api/op/save_recipe_version",
                Some(&writer_key),
                &body,
            );
        }
        written
    });

    take_a_backup(&app, &key);
    writing.store(false, std::sync::atomic::Ordering::Relaxed);
    let written = writer.join().expect("the writer finished");
    assert!(
        written > 1,
        "the Backup finished before a second save even started, so nothing was \
         written during it"
    );

    let restored = restore(&app, &archive_named(&app, &key, "daily"));

    restored
        .with_conn(|conn| {
            // Not a torn file: SQLite checks its own pages.
            let verdict: String = conn
                .query_row("PRAGMA integrity_check", [], |row| row.get(0))
                .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            assert_eq!(verdict, "ok", "the restored database is not intact");

            // And not a half-written save: every Version in it still
            // fingerprints its own content, which a torn copy would break.
            let wrong: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM versions WHERE id <> version_fingerprint(content)",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            assert_eq!(
                wrong, 0,
                "the Backup caught a save half-written: {wrong} Versions in the \
                 restored database no longer fingerprint their own content"
            );
            Ok(())
        })
        .expect("read the restored database");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn three_archives_are_kept_at_three_distances_and_older_ones_are_pruned() {
    let app = support::spawn_app();
    let (key, _kitchen_id) = operator_with_kitchen(&app);
    let data_dir = app.core.data_dir();

    // A fresh instance holds nothing until something is owed.
    assert!(names_held(&app, &key).is_empty());

    // The first Backup fills all three slots at once: an instance with no past
    // has nothing further back to reach for, so the three start together and
    // pull apart as each comes due (ADR 0039).
    let first = take_a_backup(&app, &key);
    assert_eq!(
        first["taken"].as_array().expect("names").len(),
        3,
        "{first}"
    );
    let held = names_held(&app, &key);
    assert_eq!(held.len(), 3, "three archives and no more: {held:?}");
    for slot in ["daily", "weekly", "monthly"] {
        archive_named(&app, &key, slot);
    }

    // Nothing is owed a moment later, so a scheduled run takes nothing.
    let idle = kamosu::backups::run(&data_dir, kamosu::backups::Ask::WhenDue).expect("a quiet run");
    assert_eq!(idle["taken"], json!([]), "{idle}");
    assert_eq!(names_held(&app, &key).len(), 3);

    // Eight days on, the daily and the weekly have both aged out. The monthly
    // has not, and leaving it alone is the whole point: it is the only archive
    // reaching further back than a week.
    for name in names_held(&app, &key) {
        backdate_archive(&app, &name, 8);
    }
    let aged_daily = archive_named(&app, &key, "daily");
    let aged_weekly = archive_named(&app, &key, "weekly");
    let aged_monthly = archive_named(&app, &key, "monthly");
    let week_on =
        kamosu::backups::run(&data_dir, kamosu::backups::Ask::WhenDue).expect("a scheduled run");
    assert_eq!(
        week_on["taken"].as_array().expect("names").len(),
        2,
        "the daily and the weekly are owed after eight days, the monthly is not: {week_on}"
    );
    assert_eq!(
        archive_named(&app, &key, "monthly"),
        aged_monthly,
        "an eight-day-old monthly must be left where it is"
    );
    assert_eq!(names_held(&app, &key).len(), 3, "still three, never more");
    for pruned in [&aged_daily, &aged_weekly] {
        assert!(
            !data_dir.join("backups").join(pruned).exists(),
            "the archive a fresh one replaced must be pruned, not kept: {pruned}"
        );
    }

    // Two days on, only the daily is owed.
    for name in names_held(&app, &key) {
        backdate_archive(&app, &name, 2);
    }
    let two_days_on =
        kamosu::backups::run(&data_dir, kamosu::backups::Ask::WhenDue).expect("a scheduled run");
    let taken = two_days_on["taken"].as_array().expect("names");
    assert_eq!(
        taken.len(),
        1,
        "only the daily is owed after two days: {two_days_on}"
    );
    assert!(
        taken[0].as_str().expect("a name").contains("-daily-"),
        "{two_days_on}"
    );
    assert_eq!(names_held(&app, &key).len(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_backup_is_listed_at_both_doors_and_fetched_out_of_band() {
    let app = support::spawn_app();
    let (key, _kitchen_id) = operator_with_kitchen(&app);
    take_a_backup(&app, &key);
    let name = archive_named(&app, &key, "daily");

    // The Web Door lists it, because `list_backups` is an ordinary Operation.
    assert!(names_held(&app, &key).contains(&name));

    // So does the MCP door, for the same reason and with no code of its own.
    let (status, answered) = app.post_mcp(
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": "list_backups", "arguments": {} },
        })
        .to_string(),
        Some(&key),
    );
    assert_eq!(status, 200, "{answered}");
    let at_the_mcp_door: Vec<String> = answered["result"]["structuredContent"]["backups"]
        .as_array()
        .expect("a list of Backups")
        .iter()
        .map(|backup| backup["name"].as_str().expect("a name").to_string())
        .collect();
    assert_eq!(at_the_mcp_door, names_held(&app, &key), "{answered}");

    // And the bytes travel out of band under the same Credential (ADR 0001):
    // this Access Key is an agent's, so an agent asked to carry a Backup off
    // the machine can, without a browser and without a JSON envelope.
    let (fetch_status, content_type, bytes) =
        app.get_bytes(&format!("/api/backups/{name}"), Some(&key));
    assert_eq!(fetch_status, 200);
    assert_eq!(content_type, "application/zip");
    let on_disk = std::fs::read(app.core.data_dir().join("backups").join(&name)).expect("read");
    assert_eq!(bytes, on_disk, "the archive fetched is the archive written");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fetching_a_backup_is_an_operator_power_and_nobody_elses() {
    let app = support::spawn_app();
    let (operator_key, _kitchen_id) = operator_with_kitchen(&app);
    take_a_backup(&app, &operator_key);
    let name = archive_named(&app, &operator_key, "daily");

    // An ordinary Person may not read another Person's recipes, and an archive
    // is every Person's recipes in one file (CONTEXT.md, "Operator").
    let (_person, key, _kitchen) = person_with_kitchen(&app, "Someone else");
    assert_eq!(app.post_op("list_backups", Some(&key), "{}").0, 401);
    assert_eq!(app.post_op("take_backup", Some(&key), "{}").0, 401);
    let (status, _, _) = app.get_bytes(&format!("/api/backups/{name}"), Some(&key));
    assert_eq!(status, 401, "an ordinary Person must not fetch a Backup");

    // Nor may a stranger presenting nothing at all.
    let (stranger, _, _) = app.get_bytes(&format!("/api/backups/{name}"), None);
    assert_eq!(stranger, 401);

    // A name nobody wrote is not a path into `/data`.
    for wrong in [
        "kamosu.db",
        "kamosu-backup-daily-not-a-moment.zip",
        "kamosu-backup-hourly-20260908T141500Z.zip",
    ] {
        let (status, _, _) = app.get_bytes(&format!("/api/backups/{wrong}"), Some(&operator_key));
        assert_eq!(status, 404, "`{wrong}` must name no Backup");
    }
}

/// **Kamosu never sends a Backup anywhere** (CONTEXT.md, "Backup"). Said here
/// as a fact about the Catalogue, which is the whole of what Kamosu can be
/// asked to do (ADR 0001): neither Operation takes any input at all, so there
/// is nowhere for a caller to name a destination.
#[test]
fn nothing_in_the_catalogue_can_be_told_where_to_send_a_backup() {
    for name in ["take_backup", "list_backups"] {
        let op = kamosu::catalogue::find(name).expect("the Operation is declared");
        assert_eq!(
            op.input_schema["properties"],
            json!({}),
            "`{name}` takes input, and the one thing a Backup Operation must never \
             accept is somewhere to send the archive"
        );
        assert_eq!(op.input_schema["additionalProperties"], json!(false));
    }
}

// --- Bundles (#66, ADR 0020) --------------------------------------------------

/// Every file in a Bundle, by name, read whole.
fn unzip(bytes: &[u8]) -> std::collections::BTreeMap<String, Vec<u8>> {
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("a Bundle is a plain zip");
    (0..archive.len())
        .map(|index| {
            let mut entry = archive.by_index(index).expect("an entry");
            let mut held = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut held).expect("reads");
            (entry.name().to_string(), held)
        })
        .collect()
}

/// A pizza with Sections, a photograph on it and on a Step, a named second
/// Version, a Component, a Food with names in two Languages and a Cup Weight,
/// a Tag, and a French Translation growing from it. Returns
/// `(key, kitchen, pizza, pizza lineage, french, dough, [main photo, step photo])`.
fn a_pizza_worth_sending(
    app: &support::TestApp,
) -> (String, String, String, String, String, String, [String; 2]) {
    let (_person, key, kitchen_id) = person_with_kitchen(app, "Aurélien");
    let (dough, dough_lineage) = recipe_with(
        app,
        &key,
        &kitchen_id,
        "Neapolitan Pizza Dough",
        Some(("1", "kg")),
        json!([{ "kind": "ingredient", "text": "600 g tipo 00 flour" }]),
        json!([{ "kind": "step", "text": "Knead for ten minutes." }]),
    );
    let main_photo = upload_a_picture(app, &key, 1);
    let step_photo = upload_a_picture(app, &key, 2);

    let (status, created) = app.post_op(
        "create_recipe",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id, "title": "Pizza Margherita" }).to_string(),
    );
    assert_eq!(status, 200, "{created}");
    let pizza = created["result"]["branch_id"].as_str().unwrap().to_string();
    let lineage = created["result"]["lineage_id"]
        .as_str()
        .unwrap()
        .to_string();
    let written = |mozzarella: &str| {
        json!({
            "branch_id": pizza,
            "title": "Pizza Margherita",
            "yield": { "amount": "2", "noun": "pizzas" },
            "prep_time_minutes": 20,
            "cook_time_minutes": 8,
            "main_photo": main_photo,
            "note": "The dough wants making the day before.",
            "nutrition": { "calories": 800, "basis": "per_serving" },
            "ingredients": [
                { "kind": "section", "text": "For the base" },
                { "kind": "ingredient", "text": "500 g Neapolitan pizza dough" },
                { "kind": "section", "text": "To finish" },
                { "kind": "ingredient", "text": mozzarella },
            ],
            "steps": [
                { "kind": "section", "text": "Bake" },
                { "kind": "step", "text": "Bake 6 to 8 minutes.", "photo": step_photo },
            ],
        })
    };
    let (status, saved) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &written("250 g mozzarella").to_string(),
    );
    assert_eq!(status, 200, "{saved}");
    backdate_branch_head(app, &pizza);
    let mut second = written("125 g mozzarella");
    second["name"] = json!("Less cheese");
    second["change_note"] = json!("Half the mozzarella; it was drowning the tomato.");
    let (status, saved) = app.post_op("save_recipe_version", Some(&key), &second.to_string());
    assert_eq!(status, 200, "{saved}");

    make_component(app, &key, &pizza, 1, Some("500"), Some("g"), &dough_lineage);
    let (status, read) = app.post_op(
        "set_reading",
        Some(&key),
        &json!({
            "branch_id": pizza, "line_index": 3,
            "amount": "125", "unit": "g", "target": "mozzarella",
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{read}");
    let food = food_named(app, &key, "en", "mozzarella");
    let (status, named) = app.post_op(
        "set_food_name",
        Some(&key),
        &json!({ "food_id": food, "language": "fr", "name": "mozzarella di bufala" }).to_string(),
    );
    assert_eq!(status, 200, "{named}");
    let (status, weighed) = app.post_op(
        "set_food_cup_weight",
        Some(&key),
        &json!({ "food_id": food, "cup_weight_grams": 113.0 }).to_string(),
    );
    assert_eq!(status, 200, "{weighed}");

    let tag = tag_in(app, &key, &kitchen_id, "en", "Weekend");
    file_under(app, &key, &pizza, &tag, true);

    let mut french = written("125 g de mozzarella");
    french["branch_id"] = json!(pizza);
    french["language"] = json!("fr");
    french["name"] = json!("Traduite");
    let (status, translated) = app.post_op("start_translation", Some(&key), &french.to_string());
    assert_eq!(status, 200, "{translated}");
    let french = translated["result"]["branch_id"]
        .as_str()
        .unwrap()
        .to_string();

    (
        key,
        kitchen_id,
        pizza,
        lineage,
        french,
        dough,
        [main_photo, step_photo],
    )
}

/// **A Bundle is one recipe's worth of Vault** (#66, ADR 0020): a plain zip of
/// readable notes, the Photographs byte for byte, and a sidecar carrying every
/// Version whole with its Readings and ids — the pizza and its French
/// Translation as what it is about, the dough as a Passenger.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bundle_carries_the_recipe_its_translation_its_passenger_and_its_photographs() {
    let app = support::spawn_app();
    let (key, _kitchen, pizza, lineage, french, dough, photos) = a_pizza_worth_sending(&app);

    let (status, exported) = app.post_op(
        "export_bundle",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    assert_eq!(status, 200, "{exported}");
    let exported = &exported["result"];
    assert_eq!(exported["file_name"], json!("Pizza Margherita.zip"));
    assert_eq!(exported["fetch_at"], json!(format!("/api/bundles/{pizza}")));
    assert_eq!(
        exported["subjects"],
        json!([{ "lineage_id": lineage, "title": "Pizza Margherita" }]),
        "one recipe is what this Bundle is about, however many Branches carry it"
    );
    assert_eq!(
        exported["passengers"][0]["title"],
        json!("Neapolitan Pizza Dough")
    );
    assert_eq!(exported["photographs"], json!(2));
    assert_eq!(exported["missing_photographs"], json!([]));

    let (status, content_type, bytes) = app.get_bytes(&format!("/api/bundles/{pizza}"), Some(&key));
    assert_eq!(status, 200);
    assert_eq!(content_type, "application/zip");
    assert_eq!(
        exported["notes"],
        json!([
            "Pizza Margherita.md",
            "Pizza Margherita (Français).md",
            "Neapolitan Pizza Dough.md"
        ]),
        "what export_bundle describes is what the bytes hold"
    );
    let files = unzip(&bytes);
    let names: Vec<&str> = files.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        [
            ".kamosu/bundle.json",
            "Neapolitan Pizza Dough.md",
            "Pizza Margherita (Français).md",
            "Pizza Margherita.md",
            "photographs/Pizza Margherita, step 1.webp",
            "photographs/Pizza Margherita.webp",
        ],
        "notes at the top, pictures beside them, machinery in the dot-directory"
    );

    // Photographs travel as their remade bytes, unmodified (ADR 0017).
    for (name, hash) in [
        ("photographs/Pizza Margherita.webp", &photos[0]),
        ("photographs/Pizza Margherita, step 1.webp", &photos[1]),
    ] {
        let (status, _, stored) = app.get_bytes(&format!("/api/photographs/{hash}"), Some(&key));
        assert_eq!(status, 200);
        assert_eq!(
            files[name], stored,
            "{name} is byte-identical to the Photograph"
        );
    }

    let sidecar: Value = serde_json::from_slice(&files[kamosu::bundles::SIDECAR]).expect("JSON");
    assert_eq!(sidecar["subjects"], json!([lineage]));
    let branches = sidecar["branches"].as_array().expect("the Branches");
    let branch = |id: &str| {
        branches
            .iter()
            .find(|b| b["branch_id"] == json!(id))
            .unwrap_or_else(|| panic!("{id} travels: {sidecar}"))
    };
    assert_eq!(branch(&pizza)["subject"], json!(true));
    assert_eq!(branch(&french)["subject"], json!(true));
    assert_eq!(
        branch(&dough)["subject"],
        json!(false),
        "the dough is a Passenger: present because the pizza needs it"
    );
    assert_eq!(branch(&pizza)["note"], json!("Pizza Margherita.md"));
    assert_eq!(
        branch(&pizza)["tags"],
        json!([{ "names": { "en": "Weekend" } }])
    );

    // Complete states, never deltas: every Version, each one's content
    // fingerprinting to its own id, chained by parent back to the first.
    let versions = branch(&pizza)["versions"].as_array().unwrap();
    assert_eq!(
        versions.len(),
        2,
        "the whole chain, back to the first Version"
    );
    assert_eq!(versions[0]["parent_version_id"], Value::Null);
    assert_eq!(versions[1]["parent_version_id"], versions[0]["version_id"]);
    assert_eq!(versions[1]["name"], json!("Less cheese"));
    for record in branches {
        assert!(
            record.as_object().unwrap().contains_key("origin_address"),
            "each Branch carries an origin address slot"
        );
        for version in record["versions"].as_array().unwrap() {
            assert_eq!(
                json!(kamosu::fingerprint::fingerprint_content(
                    &version["content"]
                )),
                version["version_id"],
                "a Version's content is what its id says it is"
            );
            assert!(
                version.as_object().unwrap().contains_key("signature"),
                "each Version carries a signature slot"
            );
            assert_eq!(version["signature"], Value::Null, "and v1 signs nothing");
        }
    }
    let french_first = &branch(&french)["versions"][0];
    assert_eq!(
        french_first["translates_version_id"], versions[1]["version_id"],
        "the Translation says which Version it renders"
    );

    // The recipe's own nutrition figure is part of its words, so it travels.
    assert_eq!(
        versions[1]["content"]["nutrition"],
        json!({ "calories": 800.0, "basis": "per_serving" })
    );

    // A Reading travels beside its Version: the Component as a Lineage, the
    // Food as its names in every Language and nothing else.
    let readings = &versions[1]["readings"];
    assert_eq!(readings[0], Value::Null, "a Section has no Reading");
    assert_eq!(readings[1]["lineage_id"], branch(&dough)["lineage_id"]);
    assert_eq!(
        readings[3],
        json!({
            "amount": "125", "unit": "g", "target": "mozzarella", "lineage_id": null,
            "food": { "names": { "en": "mozzarella", "fr": "mozzarella di bufala" } },
        }),
        "a Food travels as its names and no id"
    );
    let raw = String::from_utf8(files[kamosu::bundles::SIDECAR].clone()).unwrap();
    let food = food_named(&app, &key, "en", "mozzarella");
    assert!(!raw.contains(&food), "no Food id travels");

    // A Cup Weight belongs to this instance and never travels in a share
    // (ADR 0016). The mozzarella under test was given one, so there is
    // something here to catch. This asks the parsed sidecar, not its text.
    // The figure is three digits, and the file is full of 64-character hex
    // ids and millisecond timestamps that spell one by chance, which failed
    // this test about one run in twenty (#96).
    fn no_cup_weight_anywhere(value: &Value, path: &str) {
        match value {
            Value::Object(fields) => {
                for (key, child) in fields {
                    assert!(
                        !key.contains("cup_weight"),
                        "{path}.{key} carries a Cup Weight"
                    );
                    no_cup_weight_anywhere(child, &format!("{path}.{key}"));
                }
            }
            Value::Array(items) => {
                for (index, child) in items.iter().enumerate() {
                    no_cup_weight_anywhere(child, &format!("{path}[{index}]"));
                }
            }
            _ => {}
        }
    }
    no_cup_weight_anywhere(&sidecar, "bundle");
    // A broader net over the same question. The walk above reads keys; this
    // also catches the name inside a value. Unlike the figure, a key name is
    // long enough that no id spells it by accident.
    assert!(
        !raw.contains("cup_weight"),
        "the sidecar names a Cup Weight somewhere"
    );

    // The Access Key that wrote each Version stays here (ADR 0015).
    let key_ids: Vec<String> = app
        .core
        .db()
        .with_conn(|conn| {
            let mut statement = conn
                .prepare("SELECT DISTINCT access_key_id FROM branch_versions WHERE access_key_id IS NOT NULL")
                .unwrap();
            let ids = statement
                .query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<Vec<String>, _>>()
                .unwrap();
            Ok(ids)
        })
        .unwrap();
    assert!(
        !key_ids.is_empty(),
        "these Versions were written by an Access Key"
    );
    for (name, held) in &files {
        let text = String::from_utf8_lossy(held);
        for id in &key_ids {
            assert!(
                !text.contains(id.as_str()),
                "{name} carries the Access Key {id}"
            );
        }
        assert!(!text.contains("access_key"), "{name} names an Access Key");
    }

    // The note: the recipe as it reads today, with its Thread beneath it.
    let note = String::from_utf8(files["Pizza Margherita.md"].clone()).unwrap();
    for expected in [
        "# Pizza Margherita\n",
        "![Pizza Margherita](<photographs/Pizza Margherita.webp>)",
        "2 pizzas · 20 min prep · 8 min cook",
        "**For the base**",
        "- 500 g Neapolitan pizza dough\n  - *Neapolitan Pizza Dough · ½ of the recipe*, see [Neapolitan Pizza Dough](<Neapolitan Pizza Dough.md>)",
        "- 125 g mozzarella",
        "**Bake**",
        "1. Bake 6 to 8 minutes.\n   ![](<photographs/Pizza Margherita, step 1.webp>)",
        "## Note\n\nThe dough wants making the day before.",
        "Tags: Weekend",
        "---\n\n## Everything this recipe has been",
        "- **Less cheese** · Aurélien · ",
        "  Half the mozzarella; it was drowning the tomato.",
        "  - *Branched here:* **Pizza Margherita (Français)**, see [Pizza Margherita (Français)](<Pizza Margherita (Français).md>)",
        "    - **Traduite** · Aurélien · ",
        "- **Written down** · Aurélien · ",
    ] {
        assert!(
            note.contains(expected),
            "the note has {expected:?}:\n{note}"
        );
    }
    assert!(
        note.find("**Less cheese**") < note.find("**Written down**"),
        "the Thread reads newest first, like the Share Link page"
    );
    let translation = String::from_utf8(files["Pizza Margherita (Français).md"].clone()).unwrap();
    assert!(
        translation.contains("## Ingrédients"),
        "in its own Language:\n{translation}"
    );
    assert!(
        translation.contains(
            "*Traduite de* **Less cheese**, voir [Pizza Margherita](<Pizza Margherita.md>)"
        ),
        "a Translation says what it renders:\n{translation}"
    );

    // Exporting changed nothing about the dough's own Visibility (ADR 0008).
    let (status, dough_link) = app.post_op(
        "get_share_link",
        Some(&key),
        &json!({ "branch_id": dough }).to_string(),
    );
    assert_eq!(status, 200, "{dough_link}");
    assert_eq!(dough_link["result"]["shared"], json!(false));
}

/// **Generating a Bundle is available at both Doors**, and its bytes travel out
/// of band under the same Credential — to the Kitchen that holds the recipe
/// and nobody else (#66).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bundle_is_made_at_both_doors_for_the_kitchen_that_holds_it_and_nobody_else() {
    let app = support::spawn_app();
    let (key, _kitchen, pizza, _lineage, _french, _dough, _photos) = a_pizza_worth_sending(&app);

    let (status, answered) = app.post_mcp(
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": "export_bundle", "arguments": { "branch_id": pizza } },
        })
        .to_string(),
        Some(&key),
    );
    assert_eq!(status, 200, "{answered}");
    let at_mcp = &answered["result"]["structuredContent"];
    assert_eq!(
        at_mcp["file_name"],
        json!("Pizza Margherita.zip"),
        "{answered}"
    );
    let (_, at_web) = app.post_op(
        "export_bundle",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    assert_eq!(
        at_mcp["notes"], at_web["result"]["notes"],
        "both Doors describe the same Bundle"
    );

    // An agent's read-only Access Key may still take a copy: exporting writes
    // nothing.
    let person = app
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row("SELECT id FROM people WHERE name = 'Aurélien'", [], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .unwrap();
    let read_only = app
        .core
        .mint_access_key(&person, "reader", true)
        .unwrap()
        .secret;
    let (status, _, _) = app.get_bytes(&format!("/api/bundles/{pizza}"), Some(&read_only));
    assert_eq!(status, 200, "a read-only key may export");

    let (status, _, _) = app.get_bytes(&format!("/api/bundles/{pizza}"), None);
    assert_eq!(status, 401, "no Credential, no Bundle");

    let (_stranger, stranger_key, _) = person_with_kitchen(&app, "Nadia");
    let (status, _, _) = app.get_bytes(&format!("/api/bundles/{pizza}"), Some(&stranger_key));
    assert_ne!(
        status, 200,
        "a Person outside the Kitchen cannot take its recipe"
    );
    let (status, refused) = app.post_op(
        "export_bundle",
        Some(&stranger_key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    assert_ne!(status, 200, "{refused}");

    let (status, _, _) = app.get_bytes("/api/bundles/b_nothing", Some(&key));
    assert_eq!(status, 404);
}

/// **An origin address belongs to a Branch and is carried as it stands**, so a
/// reshare does not launder where a recipe came from (ADR 0020). This instance
/// writes none of its own; a Branch that arrived with one keeps it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bundle_carries_each_branchs_origin_as_it_stands_and_invents_none() {
    let app = support::spawn_app();
    let (key, _kitchen, pizza, _lineage, _french, dough, _photos) = a_pizza_worth_sending(&app);
    app.core
        .db()
        .with_conn(|conn| {
            conn.execute(
                "UPDATE branches SET origin_address = 'https://marc.example' WHERE id = ?1",
                rusqlite::params![pizza],
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .unwrap();

    let (_, _, bytes) = app.get_bytes(&format!("/api/bundles/{pizza}"), Some(&key));
    let files = unzip(&bytes);
    let sidecar: Value = serde_json::from_slice(&files[".kamosu/bundle.json"]).unwrap();
    let origin = |id: &str| {
        sidecar["branches"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["branch_id"] == json!(id))
            .unwrap()["origin_address"]
            .clone()
    };
    assert_eq!(origin(&pizza), json!("https://marc.example"));
    assert_eq!(origin(&dough), Value::Null, "nothing in v1 writes one");
    assert!(
        sidecar.get("origin_address").is_none(),
        "an origin belongs to a Branch, never to the file"
    );
}

/// **A Copy's Bundle carries the whole chain it grew from** (ADR 0018, ADR
/// 0020): Marc changed Aurélien's pizza, so his Branch forks at Aurélien's
/// Version, and his Bundle begins at the beginning — Aurélien's Versions under
/// Aurélien's Hand, Marc's on top — with each still what its id says it is.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_copys_bundle_carries_the_chain_it_forked_from_under_each_hand() {
    let app = support::spawn_app();
    let (_key, _kitchen, pizza, lineage, _french, _dough, _photos) = a_pizza_worth_sending(&app);
    let (_marc, marc_key, marc_kitchen) = person_with_kitchen(&app, "Marc");
    let (status, copied) = app.post_op(
        "save_recipe_version",
        Some(&marc_key),
        &json!({
            "branch_id": pizza,
            "kitchen_id": marc_kitchen,
            "title": "Pizza Margherita",
            "ingredients": [{ "kind": "ingredient", "text": "A lot more basil" }],
            "name": "Marc's",
            "change_note": "Basil, and nothing else matters.",
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{copied}");
    assert_eq!(copied["result"]["copied"], json!(true));
    let copy = copied["result"]["branch_id"].as_str().unwrap().to_string();

    let (status, _, bytes) = app.get_bytes(&format!("/api/bundles/{copy}"), Some(&marc_key));
    assert_eq!(status, 200);
    let files = unzip(&bytes);
    let sidecar: Value = serde_json::from_slice(&files[".kamosu/bundle.json"]).unwrap();
    assert_eq!(
        sidecar["subjects"],
        json!([lineage]),
        "the same recipe, forked"
    );
    let record = &sidecar["branches"][0];
    assert_eq!(record["branch_id"], json!(copy));
    let versions = record["versions"].as_array().unwrap();
    let hands: Vec<&str> = versions
        .iter()
        .map(|v| v["hand"]["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        hands,
        ["Aurélien", "Aurélien", "Marc"],
        "the whole chain, each under its Hand"
    );
    for window in versions.windows(2) {
        assert_eq!(window[1]["parent_version_id"], window[0]["version_id"]);
    }
    for version in versions {
        assert_eq!(
            json!(kamosu::fingerprint::fingerprint_content(
                &version["content"]
            )),
            version["version_id"]
        );
    }
    let note = String::from_utf8(files["Pizza Margherita.md"].clone()).unwrap();
    let marcs = note
        .find("- **Marc's** · Marc · ")
        .expect("Marc's Version, newest");
    let aureliens = note
        .find("- **Less cheese** · Aurélien · ")
        .expect("the Version it forked at");
    assert!(marcs < aureliens, "{note}");
}

/// **A Bundle short of a Photograph says so** rather than being quietly
/// incomplete: the recipe still travels, and the sidecar and `export_bundle`
/// both name what is missing (#66).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bundle_short_of_a_photograph_names_what_is_missing() {
    let app = support::spawn_app();
    let (key, _kitchen, pizza, _lineage, _french, _dough, photos) = a_pizza_worth_sending(&app);
    std::fs::remove_file(kamosu::photographs::photograph_path(
        &app.core.data_dir(),
        &photos[1],
    ))
    .expect("the step's picture is on disk");

    let (status, exported) = app.post_op(
        "export_bundle",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    assert_eq!(status, 200, "{exported}");
    assert_eq!(
        exported["result"]["missing_photographs"],
        json!([photos[1]])
    );
    assert_eq!(exported["result"]["photographs"], json!(1));

    let (status, _, bytes) = app.get_bytes(&format!("/api/bundles/{pizza}"), Some(&key));
    assert_eq!(status, 200, "the recipe still travels");
    let files = unzip(&bytes);
    let sidecar: Value = serde_json::from_slice(&files[".kamosu/bundle.json"]).unwrap();
    assert_eq!(sidecar["missing_photographs"], json!([photos[1]]));
    assert!(files.contains_key("photographs/Pizza Margherita.webp"));
    assert!(!files.keys().any(|name| name.contains("step 1")));
}

// --- Bundle import (#67, ADR 0020) ---------------------------------------------

/// A Bundle's bytes from the instance that wrote it.
fn bundle_of(app: &support::TestApp, key: &str, branch_id: &str) -> Vec<u8> {
    let (status, _, bytes) = app.get_bytes(&format!("/api/bundles/{branch_id}"), Some(key));
    assert_eq!(status, 200, "the Bundle is fetched");
    bytes
}

/// Ask `import_bundle` however the input says, and answer how the Job ended.
///
/// Every way of receiving a Bundle goes through here, because the asking and
/// the waiting are the same either way (ADR 0032) — only the input differs.
fn importing(app: &support::TestApp, key: &str, input: Value) -> Value {
    let (status, ask) = app.post_op("import_bundle", Some(key), &input.to_string());
    assert_eq!(status, 200, "{ask}");
    let job_id = ask["result"]["job_id"].as_str().expect("a job id");
    support::wait_terminal(app, Some(key), job_id)
}

/// The Import Report of a Bundle that was received, insisting it completed.
fn imported(app: &support::TestApp, key: &str, input: Value) -> Value {
    let finished = importing(app, key, input);
    assert_eq!(finished["status"], json!("completed"), "{finished}");
    finished["result"].clone()
}

/// Receive a Bundle base64-encoded inside the body: the way a Door that can
/// send nothing but JSON does it.
fn receive(app: &support::TestApp, key: &str, bytes: &[u8]) -> Value {
    use base64::Engine;
    let data = base64::engine::general_purpose::STANDARD.encode(bytes);
    imported(app, key, json!({ "data": data }))
}

/// Stage a Bundle's bytes at `POST /api/uploads` and answer the id (#93).
fn stage(app: &support::TestApp, key: &str, bytes: &[u8]) -> String {
    let (status, staged) = app.post_bytes("/api/uploads", Some(key), "application/zip", bytes);
    assert_eq!(status, 200, "{staged}");
    staged["result"]["upload_id"]
        .as_str()
        .expect("an upload id")
        .to_string()
}

/// Receive a Bundle the way a browser sends one: its own bytes first, then the
/// id naming them. `receive` is the other way in, base64 inside the body.
fn receive_staged(app: &support::TestApp, key: &str, bytes: &[u8]) -> Value {
    let upload_id = stage(app, key, bytes);
    imported(app, key, json!({ "upload_id": upload_id }))
}

/// A Credential on a fresh instance, to receive a Bundle with.
fn a_receiver(app: &support::TestApp) -> String {
    let person = app.core.create_person("Nadia").expect("person");
    app.core
        .mint_access_key(&person, "browser", false)
        .unwrap()
        .secret
}

/// The files of a Bundle zipped back up, after a test has had its way with
/// them — the sender's file mangled in transit.
fn rezip(files: &std::collections::BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, bytes) in files {
        archive
            .start_file(name.as_str(), zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut archive, bytes).unwrap();
    }
    archive.finish().unwrap().into_inner()
}

/// Rewrite a Bundle's sidecar, leaving everything else as it was.
fn with_sidecar(bytes: &[u8], change: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut files = unzip(bytes);
    let mut sidecar: Value = serde_json::from_slice(&files[".kamosu/bundle.json"]).unwrap();
    change(&mut sidecar);
    files.insert(
        ".kamosu/bundle.json".to_string(),
        serde_json::to_vec(&sidecar).unwrap(),
    );
    rezip(&files)
}

/// The Report's line for one Branch the Bundle carried, by the id it carried.
fn arrived_row<'a>(report: &'a Value, foreign_id: &str) -> &'a Value {
    report["arrived"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["foreign_id"] == json!(foreign_id))
        .unwrap_or_else(|| panic!("{foreign_id} is in what arrived: {report}"))
}

/// The **local** Branch id a carried one landed under (#90). A Branch's
/// travelling id is the sender's and unique per Kitchen; the row holding it
/// here has an id of this instance's own, and that is the one every Operation
/// and every URL takes.
fn landed_as(report: &Value, foreign_id: &str) -> String {
    arrived_row(report, foreign_id)["branch_id"]
        .as_str()
        .unwrap_or_else(|| panic!("{foreign_id} landed under a local id: {report}"))
        .to_string()
}

/// One Version's Readings as the database holds them: `(line, amount, unit,
/// target, lineage)` in line order.
type HeldReading = (
    i64,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);
fn readings_held(app: &support::TestApp, version_id: &str) -> Vec<HeldReading> {
    app.core
        .db()
        .with_conn(|conn| {
            let mut statement = conn
                .prepare(
                    "SELECT line_index, amount, unit, target, lineage_id FROM readings \
                      WHERE version_id = ?1 ORDER BY line_index",
                )
                .unwrap();
            Ok(statement
                .query_map(rusqlite::params![version_id], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap())
        })
        .unwrap()
}

/// Every Branch this instance holds of one Lineage.
fn branches_of_lineage(app: &support::TestApp, lineage_id: &str) -> Vec<String> {
    app.core
        .db()
        .with_conn(|conn| {
            let mut statement = conn
                .prepare("SELECT id FROM branches WHERE lineage_id = ?1 ORDER BY id")
                .unwrap();
            Ok(statement
                .query_map(rusqlite::params![lineage_id], |row| row.get(0))
                .unwrap()
                .collect::<Result<Vec<String>, _>>()
                .unwrap())
        })
        .unwrap()
}

/// **Receiving a Bundle places the sender's Branch here under their ids**
/// (ADR 0020): the Branch id, the Lineage id, every Version id, the Hand on the
/// Branch and on each Version, the names, the *what changed* lines and the
/// dates — the recipe as it was written there, now held by a Kitchen here.
/// The Photographs arrive as the bytes that left, the Readings as they were
/// sent, the Foods by their names, the Tags into this Kitchen's own list, and
/// the origin address as a hint.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bundle_arrives_whole_under_the_senders_ids_and_hands() {
    let there = support::spawn_app();
    let (key, _kitchen, pizza, lineage, french, dough, photos) = a_pizza_worth_sending(&there);
    // A Reading no parser would produce, and a line whose Reading was cleared:
    // if either arrives any other way, the receiver recomputed them.
    let (status, set) = there.post_op(
        "set_reading",
        Some(&key),
        &json!({
            "branch_id": pizza, "line_index": 3,
            "amount": "125", "unit": "g", "target": "fior di latte",
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{set}");
    there
        .core
        .db()
        .with_conn(|conn| {
            conn.execute(
                "UPDATE branches SET origin_address = 'https://aurelien.example' WHERE id = ?1",
                rusqlite::params![pizza],
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .unwrap();
    let bytes = bundle_of(&there, &key, &pizza);
    let (_, sent) = there.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    let sent = &sent["result"];

    let here = support::spawn_app();
    let nadia = here.core.create_person("Nadia").expect("person");
    let nadia_key = here
        .core
        .mint_access_key(&nadia, "browser", false)
        .unwrap()
        .secret;
    let nadia_kitchen = home_kitchen_of(&here, &nadia);

    let report = receive(&here, &nadia_key, &bytes);
    assert_eq!(report["source_kind"], json!("bundle"), "{report}");
    assert_eq!(report["kitchen_id"], json!(nadia_kitchen));
    assert_eq!(report["unreadable"], json!([]), "{report}");
    for (branch, subject) in [(&pizza, true), (&french, true), (&dough, false)] {
        let row = arrived_row(&report, branch);
        assert_eq!(row["status"], json!("created"), "{row}");
        assert_ne!(
            row["branch_id"],
            json!(branch),
            "held under a local id of this instance's own (#90)"
        );
        assert_eq!(row["subject"], json!(subject), "{row}");
    }
    assert_eq!(arrived_row(&report, &pizza)["lineage_id"], json!(lineage));
    // The sender's ids are what the Bundle carried; the ids here are this
    // instance's own, and the Report gives both.
    let pizza_here = landed_as(&report, &pizza);

    let (status, held) = here.post_op(
        "get_recipe",
        Some(&nadia_key),
        &json!({ "branch_id": pizza_here }).to_string(),
    );
    assert_eq!(status, 200, "{held}");
    let held = &held["result"];
    assert_eq!(
        held["kitchen_id"],
        json!(nadia_kitchen),
        "held by a Kitchen here"
    );
    assert_eq!(held["lineage_id"], json!(lineage));
    assert_eq!(
        held["hand_id"], sent["hand_id"],
        "under the sender's Kitchen's Hand"
    );
    assert_eq!(held["head_version_id"], sent["head_version_id"]);
    assert_eq!(held["origin_address"], json!("https://aurelien.example"));
    let sent_versions = sent["versions"].as_array().unwrap();
    let held_versions = held["versions"].as_array().unwrap();
    assert_eq!(held_versions.len(), sent_versions.len());
    for (held, sent) in held_versions.iter().zip(sent_versions) {
        for field in [
            "sequence",
            "version_id",
            "parent_version_id",
            "hand_id",
            "name",
            "change_note",
            "created_at",
            "content",
        ] {
            assert_eq!(
                held[field], sent[field],
                "{field} arrives as it was written"
            );
        }
    }

    // Byte for byte: the receiver stores what arrived and remakes nothing.
    for hash in &photos {
        let (_, _, left) = there.get_bytes(&format!("/api/photographs/{hash}"), Some(&key));
        let (status, _, stored) =
            here.get_bytes(&format!("/api/photographs/{hash}"), Some(&nadia_key));
        assert_eq!(status, 200);
        assert_eq!(stored, left, "{hash} is the bytes that left");
    }

    // Carried, never recomputed.
    let head = held["head_version_id"].as_str().unwrap();
    let readings = readings_held(&here, head);
    assert_eq!(
        readings,
        readings_held(&there, head),
        "the Readings are the sender's, line for line"
    );
    assert!(
        readings
            .iter()
            .any(|r| r.3.as_deref() == Some("fior di latte")),
        "{readings:?}"
    );

    // The Food arrives by its names alone, and nothing about it that was
    // learned there — the Cup Weight stays behind.
    let food = food_named(&here, &nadia_key, "en", "fior di latte");
    let (_, food) = here.post_op(
        "get_food",
        Some(&nadia_key),
        &json!({ "food_id": food }).to_string(),
    );
    assert_eq!(food["result"]["cup_weight_grams"], Value::Null, "{food}");

    // The Tag lands in the receiving Kitchen's own list, and files the recipe.
    let (_, tags) = here.post_op(
        "list_tags",
        Some(&nadia_key),
        &json!({ "kitchen_id": nadia_kitchen }).to_string(),
    );
    let weekend = tags["result"]["tags"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tag| tag["name"] == json!("Weekend"))
        .unwrap_or_else(|| panic!("the Tag arrived: {tags}"));
    assert_eq!(weekend["kitchen_id"], json!(nadia_kitchen));

    // The Hands' names travel on: resharing names Aurélien, not Nadia. So does
    // the Branch id — a reshare carries the sender's, never the local row id,
    // or the next instance would see a third recipe rather than Aurélien's
    // (#90).
    let reshared = bundle_of(&here, &nadia_key, &pizza_here);
    let files = unzip(&reshared);
    let sidecar: Value = serde_json::from_slice(&files[".kamosu/bundle.json"]).unwrap();
    let record = sidecar["branches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["branch_id"] == json!(pizza))
        .unwrap();
    assert_eq!(record["hand"]["name"], json!("Aurélien's Kitchen"));
    assert_eq!(record["versions"][0]["hand"]["name"], json!("Aurélien"));
    assert_eq!(record["origin_address"], json!("https://aurelien.example"));
}

/// **Receipt is not a Copy** (ADR 0020, #54): the Bundle puts the sender's
/// Branch in your Kitchen and makes nothing of your own. Changing it does —
/// your Branch, forking at the Version you changed, carrying their chain behind
/// it — and the sender's Branch is left exactly as it arrived.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn receiving_makes_no_branch_of_your_own_and_changing_it_does() {
    let there = support::spawn_app();
    let (key, _kitchen, pizza, lineage, french, _dough, _photos) = a_pizza_worth_sending(&there);
    let bytes = bundle_of(&there, &key, &pizza);

    let here = support::spawn_app();
    let nadia = here.core.create_person("Nadia").expect("person");
    let nadia_key = here
        .core
        .mint_access_key(&nadia, "browser", false)
        .unwrap()
        .secret;
    let nadia_kitchen = home_kitchen_of(&here, &nadia);
    let report = receive(&here, &nadia_key, &bytes);
    // Nadia's own row ids for what arrived; both still travel under
    // Aurélien's (#90).
    let pizza_here = landed_as(&report, &pizza);
    let french_here = landed_as(&report, &french);

    let mut expected = vec![pizza_here.clone(), french_here.clone()];
    expected.sort();
    assert_eq!(
        branches_of_lineage(&here, &lineage),
        expected,
        "receipt alone makes no Branch of your own"
    );

    let (_, before) = here.post_op(
        "get_recipe",
        Some(&nadia_key),
        &json!({ "branch_id": pizza_here }).to_string(),
    );
    let (status, saved) = here.post_op(
        "save_recipe_version",
        Some(&nadia_key),
        &json!({
            "branch_id": pizza_here,
            "title": "Pizza Margherita",
            "ingredients": [{ "kind": "ingredient", "text": "A lot more basil" }],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{saved}");
    assert_eq!(saved["result"]["copied"], json!(true), "{saved}");
    let mine = saved["result"]["branch_id"].as_str().unwrap().to_string();
    assert_ne!(mine, pizza_here);
    assert_eq!(
        saved["result"]["parent_version_id"], before["result"]["head_version_id"],
        "forking at the Version changed"
    );

    let (_, theirs) = here.post_op(
        "get_recipe",
        Some(&nadia_key),
        &json!({ "branch_id": pizza_here }).to_string(),
    );
    assert_eq!(
        theirs["result"], before["result"],
        "the sender's Branch is untouched"
    );
    let (_, copy) = here.post_op(
        "get_recipe",
        Some(&nadia_key),
        &json!({ "branch_id": mine }).to_string(),
    );
    assert_eq!(
        copy["result"]["hand_id"],
        json!(nadia_kitchen),
        "yours from then on"
    );
    assert_eq!(copy["result"]["kitchen_id"], json!(nadia_kitchen));
    assert_eq!(
        copy["result"]["versions"].as_array().unwrap().len(),
        before["result"]["versions"].as_array().unwrap().len() + 1,
        "their whole chain behind it"
    );

    // Saying what Language it is in is a change too, and changes only yours.
    let (status, relabelled) = here.post_op(
        "set_recipe_language",
        Some(&nadia_key),
        &json!({ "branch_id": french_here, "language": "es" }).to_string(),
    );
    assert_eq!(status, 200, "{relabelled}");
    let relabelled_branch = relabelled["result"]["branch_id"].as_str().unwrap();
    assert_ne!(
        relabelled_branch, french_here,
        "a Copy, not a line in their history"
    );
    let (_, french_after) = here.post_op(
        "get_recipe",
        Some(&nadia_key),
        &json!({ "branch_id": french_here }).to_string(),
    );
    assert_eq!(french_after["result"]["language"], json!("fr"));
}

/// **A second Bundle continues the Branch it continues** (ADR 0020): the same
/// friend's pizza, three Versions on, extends the Branch already held rather
/// than lining up beside it. A Reading corrected here on a Version already
/// held stays as corrected (ADR 0021).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_second_bundle_from_the_same_sender_extends_the_branch_already_held() {
    let there = support::spawn_app();
    let (key, _kitchen, pizza, lineage, french, _dough, _photos) = a_pizza_worth_sending(&there);
    let first = bundle_of(&there, &key, &pizza);

    let here = support::spawn_app();
    let nadia = here.core.create_person("Nadia").expect("person");
    let nadia_key = here
        .core
        .mint_access_key(&nadia, "browser", false)
        .unwrap()
        .secret;
    let received = receive(&here, &nadia_key, &first);
    // Nadia's own row ids for the two Branches; they travel under Aurélien's.
    let pizza_here = landed_as(&received, &pizza);
    let french_here = landed_as(&received, &french);
    let (_, held) = here.post_op(
        "get_recipe",
        Some(&nadia_key),
        &json!({ "branch_id": pizza_here }).to_string(),
    );
    let held_head = held["result"]["head_version_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Nadia corrects a Reading on the Version she holds.
    let (status, corrected) = here.post_op(
        "set_reading",
        Some(&nadia_key),
        &json!({
            "branch_id": pizza_here, "line_index": 3,
            "amount": "125", "unit": "g", "target": "burrata",
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{corrected}");

    // Aurélien keeps writing.
    backdate_branch_head(&there, &pizza);
    let (_, current) = there.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    let mut next = current["result"]["versions"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()["content"]
        .clone();
    next["branch_id"] = json!(pizza);
    next["note"] = json!("Make the dough two days before if you can.");
    next["name"] = json!("Two days");
    let (status, saved) = there.post_op("save_recipe_version", Some(&key), &next.to_string());
    assert_eq!(status, 200, "{saved}");
    let second = bundle_of(&there, &key, &pizza);

    let report = receive(&here, &nadia_key, &second);
    assert_eq!(report["unreadable"], json!([]), "{report}");
    assert_eq!(
        arrived_row(&report, &pizza)["status"],
        json!("extended"),
        "{report}"
    );
    assert_eq!(
        arrived_row(&report, &pizza)["branch_id"],
        json!(pizza_here),
        "the Branch she already held, not a new one"
    );
    assert_eq!(arrived_row(&report, &french)["status"], json!("unchanged"));

    let mut expected = vec![pizza_here.clone(), french_here.clone()];
    expected.sort();
    assert_eq!(
        branches_of_lineage(&here, &lineage),
        expected,
        "no third Branch"
    );
    let (_, after) = here.post_op(
        "get_recipe",
        Some(&nadia_key),
        &json!({ "branch_id": pizza_here }).to_string(),
    );
    let versions = after["result"]["versions"].as_array().unwrap();
    assert_eq!(versions.len(), 3);
    assert_eq!(versions[2]["name"], json!("Two days"));
    assert_eq!(versions[2]["parent_version_id"], json!(held_head));
    assert_eq!(
        after["result"]["head_version_id"],
        saved["result"]["version_id"]
    );

    assert!(
        readings_held(&here, &held_head)
            .iter()
            .any(|r| r.0 == 3 && r.3.as_deref() == Some("burrata")),
        "yours stay"
    );

    // A Branch has one Kitchen writing it: the same Branch id under somebody
    // else's Hand is not its next chapter, however well its chain lines up.
    backdate_branch_head(&there, &pizza);
    next["note"] = json!("Three days, even.");
    next["name"] = json!("Three days");
    let (status, saved) = there.post_op("save_recipe_version", Some(&key), &next.to_string());
    assert_eq!(status, 200, "{saved}");
    let forged = with_sidecar(&bundle_of(&there, &key, &pizza), |sidecar| {
        for branch in sidecar["branches"].as_array_mut().unwrap() {
            branch["hand"]["id"] = json!("k_somebody_else");
        }
    });
    let report = receive(&here, &nadia_key, &forged);
    assert!(
        report["unreadable"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["foreign_id"] == json!(pizza)),
        "{report}"
    );
    let (_, still) = here.post_op(
        "get_recipe",
        Some(&nadia_key),
        &json!({ "branch_id": pizza_here }).to_string(),
    );
    assert_eq!(still["result"]["versions"].as_array().unwrap().len(), 3);
}

/// **Two households on one instance each receive the same friend's recipe**
/// (#90). Marc sends his pizza to Aurélien and to Nadia, who have accounts on
/// the same Kamosu and cook in separate Kitchens. Each of them gets it, each
/// under an id of this instance's own, and each still travelling under Marc's
/// — so Marc's next Bundle extends the copy of whoever receives it and no
/// other, and a reshare from either one continues Marc's Branch rather than
/// starting a third.
///
/// What this replaces: the second one to import used to get nothing at all,
/// and a sentence telling her another household here already had it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_kitchens_here_each_receive_the_same_senders_bundle_and_keep_it_apart() {
    let there = support::spawn_app();
    let (marc_key, _marc_kitchen, pizza, lineage, french, dough, _photos) =
        a_pizza_worth_sending(&there);
    let first = bundle_of(&there, &marc_key, &pizza);

    let here = support::spawn_app();
    let aurelien = here.core.create_person("Aurélien").expect("person");
    let aurelien_key = here
        .core
        .mint_access_key(&aurelien, "browser", false)
        .unwrap()
        .secret;
    let aurelien_kitchen = home_kitchen_of(&here, &aurelien);
    let nadia = here.core.create_person("Nadia").expect("person");
    let nadia_key = here
        .core
        .mint_access_key(&nadia, "browser", false)
        .unwrap()
        .secret;
    let nadia_kitchen = home_kitchen_of(&here, &nadia);
    assert_ne!(aurelien_kitchen, nadia_kitchen, "separate households");

    // Aurélien first, then Nadia — the very same file.
    let to_aurelien = receive(&here, &aurelien_key, &first);
    let to_nadia = receive(&here, &nadia_key, &first);

    for report in [&to_aurelien, &to_nadia] {
        assert_eq!(report["unreadable"], json!([]), "{report}");
        for carried in [&pizza, &french, &dough] {
            assert_eq!(
                arrived_row(report, carried)["status"],
                json!("created"),
                "{report}"
            );
        }
    }
    // Nothing either report says mentions the other household, or that a
    // Kitchen the reader does not cook in exists at all.
    let said = to_nadia.to_string();
    assert!(!said.contains(&aurelien_kitchen), "{to_nadia}");
    assert!(
        !said.contains("Kitchen you do not cook in"),
        "the refusal that leaked the other household is gone: {to_nadia}"
    );

    // Two rows, one per Kitchen, each with its own id here.
    let aurelien_pizza = landed_as(&to_aurelien, &pizza);
    let nadia_pizza = landed_as(&to_nadia, &pizza);
    assert_ne!(aurelien_pizza, nadia_pizza, "a copy each");
    for (key, branch, kitchen) in [
        (&aurelien_key, &aurelien_pizza, &aurelien_kitchen),
        (&nadia_key, &nadia_pizza, &nadia_kitchen),
    ] {
        let (status, held) = here.post_op(
            "get_recipe",
            Some(key),
            &json!({ "branch_id": branch }).to_string(),
        );
        assert_eq!(status, 200, "{held}");
        assert_eq!(held["result"]["kitchen_id"], json!(kitchen), "{held}");
        assert_eq!(held["result"]["lineage_id"], json!(lineage));
    }

    // Both copies travel under Marc's Branch id, so a reshare from either one
    // continues his Branch at the next instance.
    for (key, branch) in [(&aurelien_key, &aurelien_pizza), (&nadia_key, &nadia_pizza)] {
        let files = unzip(&bundle_of(&here, key, branch));
        let sidecar: Value = serde_json::from_slice(&files[".kamosu/bundle.json"]).unwrap();
        assert!(
            sidecar["branches"]
                .as_array()
                .unwrap()
                .iter()
                .any(|record| record["branch_id"] == json!(pizza)),
            "the reshare carries Marc's Branch id: {sidecar}"
        );
    }

    // Marc writes again, and sends the same second Bundle to both.
    backdate_branch_head(&there, &pizza);
    let (_, current) = there.post_op(
        "get_recipe",
        Some(&marc_key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    let mut next = current["result"]["versions"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()["content"]
        .clone();
    next["branch_id"] = json!(pizza);
    next["name"] = json!("Two days");
    next["note"] = json!("Make the dough two days before if you can.");
    let (status, saved) = there.post_op("save_recipe_version", Some(&marc_key), &next.to_string());
    assert_eq!(status, 200, "{saved}");
    let second = bundle_of(&there, &marc_key, &pizza);

    // Aurélien takes it; Nadia leaves it on her desk for now.
    let again = receive(&here, &aurelien_key, &second);
    assert_eq!(
        arrived_row(&again, &pizza)["status"],
        json!("extended"),
        "{again}"
    );
    assert_eq!(
        arrived_row(&again, &pizza)["branch_id"],
        json!(aurelien_pizza),
        "his own copy, not hers"
    );
    let versions_of = |key: &str, branch: &str| {
        let (_, held) = here.post_op(
            "get_recipe",
            Some(key),
            &json!({ "branch_id": branch }).to_string(),
        );
        held["result"]["versions"].as_array().unwrap().len()
    };
    assert_eq!(versions_of(&aurelien_key, &aurelien_pizza), 3);
    assert_eq!(
        versions_of(&nadia_key, &nadia_pizza),
        2,
        "hers is untouched by his import"
    );

    // And when Nadia does take it, it extends hers.
    let hers = receive(&here, &nadia_key, &second);
    assert_eq!(
        arrived_row(&hers, &pizza)["status"],
        json!("extended"),
        "{hers}"
    );
    assert_eq!(arrived_row(&hers, &pizza)["branch_id"], json!(nadia_pizza));
    assert_eq!(versions_of(&nadia_key, &nadia_pizza), 3);

    // Six Branches of the pizza's Lineage here: the recipe and its Translation,
    // twice over, plus Marc's own two are on the other instance.
    let mut held = branches_of_lineage(&here, &lineage);
    held.sort();
    let mut expect = vec![
        aurelien_pizza,
        nadia_pizza,
        landed_as(&to_aurelien, &french),
        landed_as(&to_nadia, &french),
    ];
    expect.sort();
    assert_eq!(held, expect);
}

/// **A Bundle that left here and came back can only extend, never conflict**
/// (ADR 0020). Marc received Aurélien's pizza and forked it; while he did,
/// Aurélien moved on. Marc's Bundle carries Aurélien's own Branch as it stood
/// when it left, which changes nothing here, and Marc's fork arrives beside it
/// as the second Branch of the same recipe — sharing Aurélien's Versions, so
/// where the two diverged is a fact rather than a guess.
///
/// The pizza is written in Aurélien's **Home** Kitchen, because that is the one
/// a Bundle he receives lands in: since #90 the question a receiving import
/// asks is about that Kitchen's own copy, so a Bundle coming back to the
/// Kitchen it left is the case this is about.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bundle_that_left_here_and_came_back_extends_and_never_conflicts() {
    let here = support::spawn_app();
    let aurelien = here.core.create_person("Aurélien").expect("person");
    let key = here
        .core
        .mint_access_key(&aurelien, "browser", false)
        .unwrap()
        .secret;
    let kitchen = home_kitchen_of(&here, &aurelien);
    let (pizza, lineage) = recipe_with(
        &here,
        &key,
        &kitchen,
        "Pizza Margherita",
        Some(("2", "pizzas")),
        json!([{ "kind": "ingredient", "text": "250 g mozzarella" }]),
        json!([{ "kind": "step", "text": "Bake 6 to 8 minutes." }]),
    );
    let outbound = bundle_of(&here, &key, &pizza);

    let there = support::spawn_app();
    let marc = there.core.create_person("Marc").expect("person");
    let marc_key = there
        .core
        .mint_access_key(&marc, "browser", false)
        .unwrap()
        .secret;
    let received = receive(&there, &marc_key, &outbound);
    // Aurélien's Branch, as Marc's instance holds it: his own row id for it,
    // still travelling under Aurélien's (#90).
    let pizza_at_marcs = landed_as(&received, &pizza);
    let (status, forked) = there.post_op(
        "save_recipe_version",
        Some(&marc_key),
        &json!({
            "branch_id": pizza_at_marcs,
            "title": "Pizza Margherita",
            "ingredients": [{ "kind": "ingredient", "text": "A lot more basil" }],
            "name": "Marc's",
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{forked}");
    let marcs = forked["result"]["branch_id"].as_str().unwrap().to_string();

    // Meanwhile, here, Aurélien moves on.
    backdate_branch_head(&here, &pizza);
    let (status, moved) = here.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({ "branch_id": pizza, "title": "Pizza Margherita", "note": "Hotter oven." })
            .to_string(),
    );
    assert_eq!(status, 200, "{moved}");
    let (_, before) = here.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );

    // Aurélien's own Branch comes back as it left.
    let returning = bundle_of(&there, &marc_key, &pizza_at_marcs);
    let report = receive(&here, &key, &returning);
    assert_eq!(report["unreadable"], json!([]), "{report}");
    assert_eq!(
        arrived_row(&report, &pizza)["status"],
        json!("unchanged"),
        "{report}"
    );
    let (_, after) = here.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    assert_eq!(
        after["result"], before["result"],
        "nothing written here is undone"
    );

    // And Marc's fork arrives beside it, sharing Aurélien's Versions.
    let report = receive(&here, &key, &bundle_of(&there, &marc_key, &marcs));
    assert_eq!(
        arrived_row(&report, &marcs)["status"],
        json!("created"),
        "{report}"
    );
    let marcs_here = landed_as(&report, &marcs);
    assert!(branches_of_lineage(&here, &lineage).contains(&marcs_here));
    let (status, point) = here.post_op(
        "branch_point",
        Some(&key),
        &json!({ "branch_a_id": pizza, "branch_b_id": marcs_here }).to_string(),
    );
    assert_eq!(status, 200, "{point}");
    assert_eq!(
        point["result"]["version_id"], forked["result"]["parent_version_id"],
        "{point}"
    );
}

/// **A Bundle is received into your Home Kitchen, and that is which Kitchen
/// the question is about** (#90). Aurélien cooks in two: he exports a recipe
/// from the second one and imports the file back. Since a Branch is held per
/// Kitchen, his Home Kitchen genuinely did not have that recipe, so it arrives
/// there as its own copy rather than finding the original — which stays exactly
/// as it was, in the Kitchen that holds it.
///
/// Pinned because it is the one place the per-Kitchen rule is visible to a
/// person with more than one Kitchen, and because before #90 the same import
/// answered "unchanged" and placed nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bundle_from_another_of_your_kitchens_arrives_in_your_home_one() {
    let here = support::spawn_app();
    let (_person, key, second_kitchen) = person_with_kitchen(&here, "Aurélien");
    let person = here
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row("SELECT id FROM people WHERE name = 'Aurélien'", [], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .unwrap();
    let home_kitchen = home_kitchen_of(&here, &person);
    assert_ne!(home_kitchen, second_kitchen, "he cooks in two");

    let (pizza, lineage) = recipe_with(
        &here,
        &key,
        &second_kitchen,
        "Pizza Margherita",
        Some(("2", "pizzas")),
        json!([{ "kind": "ingredient", "text": "250 g mozzarella" }]),
        json!([{ "kind": "step", "text": "Bake 6 to 8 minutes." }]),
    );
    let (_, before) = here.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );

    let report = receive(&here, &key, &bundle_of(&here, &key, &pizza));
    assert_eq!(report["kitchen_id"], json!(home_kitchen), "{report}");
    assert_eq!(
        arrived_row(&report, &pizza)["status"],
        json!("created"),
        "his Home Kitchen did not hold it: {report}"
    );
    let at_home = landed_as(&report, &pizza);
    assert_ne!(at_home, pizza, "its own copy, with its own Local id");

    let (_, landed) = here.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": at_home }).to_string(),
    );
    assert_eq!(landed["result"]["kitchen_id"], json!(home_kitchen));
    assert_eq!(landed["result"]["lineage_id"], json!(lineage));

    let (_, after) = here.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    assert_eq!(
        after["result"], before["result"],
        "the Kitchen that holds the original is untouched"
    );

    // And his Home Kitchen's copy travels under the same Travelling id, so
    // sending both to a friend is still one Branch rather than two.
    let files = unzip(&bundle_of(&here, &key, &at_home));
    let sidecar: Value = serde_json::from_slice(&files[".kamosu/bundle.json"]).unwrap();
    assert!(
        sidecar["branches"]
            .as_array()
            .unwrap()
            .iter()
            .any(|record| record["branch_id"] == json!(pizza)),
        "{sidecar}"
    );
}

/// **A damaged Bundle keeps the dinner and loses the provenance** (ADR 0020):
/// a chain that does not reach the first Version, or a Version whose words do
/// not match its own name, has its history refused. The words arrive as a new
/// recipe of the receiver's own, with no Lineage id of the sender's — and the
/// Report says all three things.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_damaged_bundle_keeps_the_words_as_a_new_recipe_and_says_so() {
    let there = support::spawn_app();
    let (key, _kitchen, pizza, lineage, _french, dough, _photos) = a_pizza_worth_sending(&there);
    let bytes = bundle_of(&there, &key, &pizza);

    let find = |sidecar: &mut Value, branch: &str| -> usize {
        sidecar["branches"]
            .as_array()
            .unwrap()
            .iter()
            .position(|b| b["branch_id"] == json!(branch))
            .unwrap()
    };
    let short = with_sidecar(&bytes, |sidecar| {
        let at = find(sidecar, &pizza);
        sidecar["branches"][at]["versions"]
            .as_array_mut()
            .unwrap()
            .remove(0);
    });
    let forged = with_sidecar(&bytes, |sidecar| {
        let at = find(sidecar, &pizza);
        sidecar["branches"][at]["versions"][0]["content"]["title"] = json!("Pizza Bianca");
    });

    for (damage, mangled) in [
        ("a short chain", short),
        ("a Version's words changed", forged),
    ] {
        let here = support::spawn_app();
        let nadia = here.core.create_person("Nadia").expect("person");
        let nadia_key = here
            .core
            .mint_access_key(&nadia, "browser", false)
            .unwrap()
            .secret;
        let nadia_kitchen = home_kitchen_of(&here, &nadia);

        let report = receive(&here, &nadia_key, &mangled);
        assert!(
            !report["arrived"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["foreign_id"] == json!(pizza)),
            "{damage}: one row, not two: {report}"
        );
        let refused = report["unreadable"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["foreign_id"] == json!(pizza))
            .unwrap_or_else(|| panic!("{damage}: the damage is named: {report}"));
        let reason = refused["reason"].as_str().unwrap();
        for said in ["history", "new recipe", "no link"] {
            assert!(reason.contains(said), "{damage}: says {said:?}: {reason}");
        }
        let row = &refused["kept_as"];
        assert_ne!(
            row["branch_id"],
            json!(pizza),
            "{damage}: not under the sender's Branch id"
        );
        assert_ne!(row["lineage_id"], json!(lineage), "{damage}: no Lineage id");
        assert_eq!(
            row["title"],
            json!("Pizza Margherita"),
            "{damage}: the words as they read"
        );

        assert!(
            !branches_of_lineage(&here, &lineage).contains(&pizza),
            "{damage}: the damaged Branch is not placed; its sound Translation is"
        );
        let rescued = row["branch_id"].as_str().unwrap();
        let (_, recipe) = here.post_op(
            "get_recipe",
            Some(&nadia_key),
            &json!({ "branch_id": rescued }).to_string(),
        );
        let recipe = &recipe["result"];
        let versions = recipe["versions"].as_array().unwrap();
        assert_eq!(versions.len(), 1, "{damage}: no history");
        assert_eq!(
            versions[0]["hand_id"],
            json!(nadia),
            "{damage}: a recipe of your own"
        );
        assert_eq!(recipe["hand_id"], json!(nadia_kitchen));
        assert_eq!(
            versions[0]["content"]["ingredients"][3]["text"],
            json!("125 g mozzarella")
        );

        // One damaged Branch costs only itself.
        assert_eq!(
            arrived_row(&report, &dough)["status"],
            json!("created"),
            "{damage}"
        );
        let dough_here = landed_as(&report, &dough);
        let (status, sound) = here.post_op(
            "get_recipe",
            Some(&nadia_key),
            &json!({ "branch_id": dough_here }).to_string(),
        );
        assert_eq!(status, 200, "{damage}: and it opens: {sound}");
    }
}

/// **A photograph that is not what its name says is left out**, and said, and
/// costs the recipe nothing (ADR 0017): its words are still what their names
/// say, so the history is kept.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_photograph_mangled_in_transit_is_left_out_and_named() {
    let there = support::spawn_app();
    let (key, _kitchen, pizza, _lineage, _french, _dough, photos) = a_pizza_worth_sending(&there);
    let mut files = unzip(&bundle_of(&there, &key, &pizza));
    files.insert(
        "photographs/Pizza Margherita.webp".to_string(),
        b"not the picture".to_vec(),
    );

    let here = support::spawn_app();
    let nadia = here.core.create_person("Nadia").expect("person");
    let nadia_key = here
        .core
        .mint_access_key(&nadia, "browser", false)
        .unwrap()
        .secret;
    let report = receive(&here, &nadia_key, &rezip(&files));
    assert_eq!(arrived_row(&report, &pizza)["status"], json!("created"));
    let (status, kept) = here.post_op(
        "get_recipe",
        Some(&nadia_key),
        &json!({ "branch_id": landed_as(&report, &pizza) }).to_string(),
    );
    assert_eq!(status, 200, "the recipe is here and readable: {kept}");
    let unreadable = report["unreadable"].as_array().unwrap();
    assert_eq!(unreadable.len(), 1, "{report}");
    assert_eq!(
        unreadable[0]["foreign_id"],
        json!("photographs/Pizza Margherita.webp")
    );
    let (status, _, _) =
        here.get_bytes(&format!("/api/photographs/{}", photos[0]), Some(&nadia_key));
    assert_eq!(
        status, 404,
        "nothing stored under a name its bytes do not have"
    );
    let (status, _, _) =
        here.get_bytes(&format!("/api/photographs/{}", photos[1]), Some(&nadia_key));
    assert_eq!(status, 200, "the other picture arrived");
}

/// **Something that is not a Bundle at all** is named in the Report with why,
/// rather than failing the Job: the Report is a ledger, not an error.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_file_that_is_not_a_bundle_is_named_with_its_reason() {
    let here = support::spawn_app();
    let nadia = here.core.create_person("Nadia").expect("person");
    let nadia_key = here
        .core
        .mint_access_key(&nadia, "browser", false)
        .unwrap()
        .secret;

    let not_a_zip = receive(&here, &nadia_key, b"a shopping list, not a recipe");
    let mut files = std::collections::BTreeMap::new();
    files.insert(
        "notes.txt".to_string(),
        b"nothing here is a recipe".to_vec(),
    );
    let nothing_readable = receive(&here, &nadia_key, &rezip(&files));
    for report in [not_a_zip, nothing_readable] {
        assert_eq!(report["arrived"], json!([]), "{report}");
        let unreadable = report["unreadable"].as_array().unwrap();
        assert_eq!(unreadable.len(), 1, "{report}");
        assert_eq!(unreadable[0]["foreign_id"], Value::Null);
        assert!(!unreadable[0]["reason"].as_str().unwrap().is_empty());
    }
}

/// **A Bundle that lost its machine half still keeps the dinner** (ADR 0020,
/// ADR 0003): with no `.kamosu/` to trust, each note is read back into a new
/// recipe of your own, and the Report says what was lost.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bundle_without_its_sidecar_is_read_back_from_its_notes() {
    let there = support::spawn_app();
    let (key, _kitchen, pizza, lineage, _french, _dough, _photos) = a_pizza_worth_sending(&there);
    let mut files = unzip(&bundle_of(&there, &key, &pizza));
    files.remove(".kamosu/bundle.json");

    let here = support::spawn_app();
    let nadia = here.core.create_person("Nadia").expect("person");
    let nadia_key = here
        .core
        .mint_access_key(&nadia, "browser", false)
        .unwrap()
        .secret;
    let report = receive(&here, &nadia_key, &rezip(&files));
    assert_eq!(report["arrived"], json!([]), "{report}");
    let kept = report["unreadable"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["foreign_id"] == json!("Pizza Margherita.md"))
        .unwrap_or_else(|| panic!("the pizza's note is named: {report}"));
    for said in ["history", "new recipe", "no link"] {
        assert!(kept["reason"].as_str().unwrap().contains(said), "{kept}");
    }
    assert_ne!(kept["kept_as"]["lineage_id"], json!(lineage));
    let (_, recipe) = here.post_op(
        "get_recipe",
        Some(&nadia_key),
        &json!({ "branch_id": kept["kept_as"]["branch_id"] }).to_string(),
    );
    let content = &recipe["result"]["versions"][0]["content"];
    assert_eq!(content["title"], json!("Pizza Margherita"));
    assert_eq!(
        content["ingredients"],
        json!([
            { "kind": "section", "text": "For the base" },
            { "kind": "ingredient", "text": "500 g Neapolitan pizza dough" },
            { "kind": "section", "text": "To finish" },
            { "kind": "ingredient", "text": "125 g mozzarella" },
        ]),
        "{content}"
    );
    assert_eq!(content["steps"][1]["text"], json!("Bake 6 to 8 minutes."));
    assert_eq!(
        content["note"],
        json!("The dough wants making the day before.")
    );
}

/// **A Food arriving known by two names that hit two Foods here makes a
/// third** (ADR 0022, #47): somebody on another server put *farine* and
/// *flour* in one Food, which is testimony, and testimony is kept as a Food and
/// a suggestion — never acted on by welding the two here together.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_food_whose_names_hit_two_foods_here_arrives_as_a_third_and_a_suggestion() {
    let there = support::spawn_app();
    let (_marc, marc_key, marc_kitchen) = person_with_kitchen(&there, "Marc");
    let bread = read_a_word(&there, &marc_key, &marc_kitchen, "fr", "farine");
    let farine = food_named(&there, &marc_key, "fr", "farine");
    let (status, named) = there.post_op(
        "set_food_name",
        Some(&marc_key),
        &json!({ "food_id": farine, "language": "en", "name": "flour" }).to_string(),
    );
    assert_eq!(status, 200, "{named}");
    let bytes = bundle_of(&there, &marc_key, &bread);

    let here = support::spawn_app();
    let (key, kitchen_id) = operator_with_kitchen(&here);
    read_a_word(&here, &key, &kitchen_id, "fr", "farine");
    read_a_word(&here, &key, &kitchen_id, "en", "flour");
    let ours_fr = food_named(&here, &key, "fr", "farine");
    let ours_en = food_named(&here, &key, "en", "flour");

    let report = receive(&here, &key, &bytes);
    assert_eq!(
        arrived_row(&report, &bread)["status"],
        json!("created"),
        "{report}"
    );

    let (_, listed) = here.post_op("list_foods", Some(&key), "{}");
    let third: Vec<&Value> = listed["result"]["foods"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|food| food["names"].as_array().unwrap().len() == 2)
        .collect();
    assert_eq!(third.len(), 1, "a third Food carrying both names: {listed}");
    let third_id = third[0]["id"].as_str().unwrap();
    assert_ne!(third_id, ours_fr, "nothing merged");
    assert_ne!(third_id, ours_en, "nothing merged");

    let (_, suggestions) = here.post_op("list_merge_suggestions", Some(&key), "{}");
    assert!(
        !suggestions["result"]["suggestions"]
            .as_array()
            .unwrap()
            .is_empty(),
        "the collision is kept as a suggestion: {suggestions}"
    );
    let (_, held) = here.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": landed_as(&report, &bread) }).to_string(),
    );
    let head = held["result"]["head_version_id"].as_str().unwrap();
    let food_of_line: Option<String> = here
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row(
                "SELECT food_id FROM readings WHERE version_id = ?1 AND line_index = 0",
                rusqlite::params![head],
                |row| row.get(0),
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .unwrap();
    assert_eq!(
        food_of_line.as_deref(),
        Some(third_id),
        "the Reading points at the third"
    );
}

/// A Bundle sent as its own bytes rather than base64 inside the body (#93).
///
/// This is the way a browser sends one: `import_crouton` already took the pair
/// because a Crouton library is 114 MB, and a single recipe carrying
/// photographs is megabytes for the same reason. Both ways have to reach the
/// same reader and land the same recipe, or the Catalogue would be declaring
/// one Operation that behaves as two.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bundle_arrives_the_same_whether_it_was_staged_or_base64ed() {
    let there = support::spawn_app();
    let (key, _kitchen, pizza, _lineage, _french, _dough, _photos) = a_pizza_worth_sending(&there);
    let bytes = bundle_of(&there, &key, &pizza);

    // The same file, into two fresh instances, one way each.
    let staged_at = support::spawn_app();
    let staged_key = a_receiver(&staged_at);
    let inline_at = support::spawn_app();
    let inline_key = a_receiver(&inline_at);

    let staged = receive_staged(&staged_at, &staged_key, &bytes);
    let inline = receive(&inline_at, &inline_key, &bytes);

    // The travelling ids are the sender's, so they are what the two Reports can
    // be compared on: the local ids are each instance's own and must differ.
    let carried = |report: &Value| -> Vec<(String, String, String)> {
        let mut rows: Vec<(String, String, String)> = report["arrived"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                (
                    row["foreign_id"].as_str().unwrap().to_string(),
                    row["title"].as_str().unwrap().to_string(),
                    row["status"].as_str().unwrap().to_string(),
                )
            })
            .collect();
        rows.sort();
        rows
    };
    assert_eq!(
        carried(&staged),
        carried(&inline),
        "the same Bundle lands the same recipes either way: {staged} vs {inline}"
    );
    assert!(
        !carried(&staged).is_empty(),
        "the pizza actually arrived: {staged}"
    );

    // And the staged file is used once: an upload named a second time is gone,
    // so a retry sends the file again rather than importing a stale copy.
    let upload_id = stage(&staged_at, &staged_key, &bytes);
    let named = json!({ "upload_id": upload_id });
    importing(&staged_at, &staged_key, named.clone());
    let again = importing(&staged_at, &staged_key, named);
    assert_eq!(
        again["status"],
        json!("failed"),
        "the upload was spent by the first import: {again}"
    );
}

/// Neither way in, or both at once, is a refusal — not a guess about which was
/// meant. The same rule `import_crouton` holds.
///
/// Where the refusal *lands* is worth being explicit about, because the two
/// halves of the input check sit either side of the Job boundary. Shape — the
/// declared fields and their types, and nothing else being present — is the
/// Core's, checked at dispatch before the Job is ever asked for (#85), so it is
/// a 400 at the door. *Exactly one of these two* is not shape, so it is the
/// handler's, and the handler runs inside the Job: the ask is accepted and the
/// Job fails saying what it takes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn import_bundle_wants_exactly_one_of_upload_id_and_data() {
    let app = support::spawn_app();
    let key = a_receiver(&app);

    for input in [json!({}), json!({ "upload_id": "u_1", "data": "" })] {
        let finished = importing(&app, &key, input.clone());
        assert_eq!(finished["status"], json!("failed"), "{input}: {finished}");
        let why = finished["error"].as_str().unwrap_or_default();
        assert!(
            why.contains("upload_id") && why.contains("data"),
            "{input}: it says what it takes: {finished}"
        );
    }

    // A field the declaration does not name never gets that far: the shape
    // check refuses it at dispatch, so both Doors inherit the refusal (#85).
    let (status, refused) = app.post_op(
        "import_bundle",
        Some(&key),
        &json!({ "upload_id": "u_1", "kitchen_id": "k_1" }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");
    assert!(
        refused["error"]["message"]
            .as_str()
            .unwrap()
            .contains("kitchen_id"),
        "it names the field nobody declared: {refused}"
    );

    // And a `data` that is not base64 is the handler's to refuse too, for the
    // same reason: what the string *is* was never shape.
    let finished = importing(&app, &key, json!({ "data": "not base64 at all !!" }));
    assert_eq!(finished["status"], json!("failed"), "{finished}");
}

// ---------------------------------------------------------------------------
// The Crouton importer (#69, ADR 0025): a whole library in one Job, through a
// file staged out of band, with a Report that says what arrived, what waits
// for a tap and what could not be read.
// ---------------------------------------------------------------------------

mod crouton {
    use super::*;
    use std::io::Write;

    /// A Crouton export as Crouton writes one: a flat zip of `.crumb` files.
    fn an_export(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buffer);
            for (name, bytes) in files {
                zip.start_file(*name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(bytes).unwrap();
            }
            zip.finish().unwrap();
        }
        buffer.into_inner()
    }

    fn crumb(value: Value) -> Vec<u8> {
        value.to_string().into_bytes()
    }

    /// The library the tests share: a dish with two photos and a site icon,
    /// a second go at it saved by hand, a link-only stub, and a damaged file.
    fn a_library() -> Vec<u8> {
        an_export(&[
            (
                "Korean Fried Chicken.crumb",
                crumb(json!({
                    "uuid": "KFC-1", "name": "Korean Fried Chicken", "serves": 4,
                    "sourceName": "mykoreankitchen.com",
                    "webLink": "https://mykoreankitchen.com/korean-fried-chicken/",
                    "sourceImage": a_picture(9),
                    "neutritionalInfo": "Calories: 610 kcal",
                    "images": [a_picture(1), a_picture(2)],
                    "ingredients": [
                        { "order": 0, "ingredient": { "name": "Chicken" }, "quantity": { "quantityType": "SECTION" } },
                        { "order": 1, "ingredient": { "name": "chicken wings" }, "quantity": { "amount": 1, "quantityType": "KGS" } },
                        { "order": 2, "ingredient": { "name": "cloves minced garlic" }, "quantity": { "amount": 2, "secondaryAmount": 3, "quantityType": "ITEM" } },
                        { "order": 3, "ingredient": { "name": "salt" } },
                    ],
                    "steps": [
                        { "order": 0, "isSection": true, "step": "Noodles &amp; choi sum:" },
                        { "order": 1, "isSection": false, "step": "Fry twice." },
                    ],
                })),
            ),
            (
                "Korean Fried Chicken-1.crumb",
                crumb(json!({
                    "uuid": "KFC-2", "name": "Korean Fried Chicken",
                    "webLink": "https://mykoreankitchen.com/korean-fried-chicken/",
                    "ingredients": [
                        { "order": 0, "ingredient": { "name": "soy sauce" }, "quantity": { "amount": 0.25, "quantityType": "CUP" } },
                    ],
                    "steps": [],
                })),
            ),
            (
                "Gochujang Pasta.crumb",
                crumb(json!({
                    "uuid": "GOCHU", "name": "Gochujang Pasta",
                    "webLink": "https://youtube.com/shorts/E9omFgkaCTA",
                    "ingredients": [], "steps": [],
                })),
            ),
            (
                "Îles Flottantes.crumb",
                "{\"uuid\": \"ILES\", \"name\": \"Îles".as_bytes().to_vec(),
            ),
        ])
    }

    fn a_person(app: &support::TestApp) -> (String, String) {
        let person = app.core.create_person("Aurélien").expect("person");
        let key = app
            .core
            .mint_access_key(&person, "importer", false)
            .unwrap()
            .secret;
        (person, key)
    }

    fn upload(app: &support::TestApp, key: &str, bytes: &[u8]) -> String {
        let (status, staged) = app.post_bytes("/api/uploads", Some(key), "application/zip", bytes);
        assert_eq!(status, 200, "{staged}");
        staged["result"]["upload_id"]
            .as_str()
            .expect("an upload id")
            .to_string()
    }

    fn import_crouton(app: &support::TestApp, key: &str, bytes: &[u8]) -> Value {
        let upload_id = upload(app, key, bytes);
        let (status, ask) = app.post_op(
            "import_crouton",
            Some(key),
            &json!({ "upload_id": upload_id }).to_string(),
        );
        assert_eq!(status, 200, "{ask}");
        let job_id = ask["result"]["job_id"].as_str().expect("a job id");
        let finished = support::wait_terminal(app, Some(key), job_id);
        assert_eq!(finished["status"], json!("completed"), "{finished}");
        assert_eq!(finished["progress"]["done"], json!(4), "{finished}");
        finished["result"].clone()
    }

    fn recipe(app: &support::TestApp, key: &str, branch_id: &str) -> Value {
        let (status, recipe) = app.post_op(
            "get_recipe",
            Some(key),
            &json!({ "branch_id": branch_id }).to_string(),
        );
        assert_eq!(status, 200, "{recipe}");
        recipe["result"].clone()
    }

    fn arrived_titled<'a>(report: &'a Value, foreign_id: &str) -> &'a Value {
        report["arrived"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["foreign_id"] == json!(foreign_id))
            .unwrap_or_else(|| panic!("{foreign_id} did not arrive: {report}"))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_whole_library_arrives_in_one_job_and_the_report_says_what_happened() {
        let app = support::spawn_app();
        let (_, key) = a_person(&app);
        let report = import_crouton(&app, &key, &a_library());

        assert_eq!(report["source_kind"], json!("crouton"));
        assert_eq!(report["arrived"].as_array().unwrap().len(), 3, "{report}");

        // What could not be read is named by its file, with what to do.
        let unreadable = report["unreadable"].as_array().unwrap();
        assert_eq!(unreadable.len(), 1, "{report}");
        assert_eq!(unreadable[0]["name"], json!("Îles Flottantes.crumb"));
        assert_eq!(unreadable[0]["foreign_id"], Value::Null);
        assert!(
            unreadable[0]["reason"]
                .as_str()
                .unwrap()
                .contains("export it from Crouton again"),
            "{report}"
        );

        // The stub is a real recipe that arrived bare, not a failure.
        assert_eq!(arrived_titled(&report, "GOCHU")["bare"], json!(true));
        assert_eq!(arrived_titled(&report, "KFC-1")["bare"], json!(false));

        // The two hand-made versions are offered to tick, never joined.
        let pairs = report["related_candidates"].as_array().unwrap();
        assert_eq!(pairs.len(), 1, "{report}");
        assert_eq!(pairs[0]["shared"], json!(["name", "page"]));
        let lineages: Vec<&Value> = pairs[0]["recipes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|side| &side["lineage_id"])
            .collect();
        assert_ne!(
            lineages[0], lineages[1],
            "importing never joins two Lineages"
        );

        // The favicon and the second photo are left out and said so.
        let left_out: Vec<(&str, u64)> = report["left_out"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                (
                    row["what"].as_str().unwrap(),
                    row["count"].as_u64().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            left_out,
            vec![("site_icon", 1), ("extra_photos", 1)],
            "{report}"
        );

        // The icon rides in the Report so the Report can show it — and only there.
        let icon = report["left_out"][0]["icon"].as_str().expect("the icon");
        assert!(icon.starts_with("data:image/jpeg;base64,"), "{icon}");

        // The recipe itself: ordinary content, lines rebuilt with no mark.
        let chicken = arrived_titled(&report, "KFC-1");
        let branch_id = chicken["branch_id"].as_str().unwrap();
        let content = &recipe(&app, &key, branch_id)["versions"][0]["content"];
        assert_eq!(
            content["ingredients"],
            json!([
                { "kind": "section", "text": "Chicken" },
                { "kind": "ingredient", "text": "1 kg chicken wings" },
                { "kind": "ingredient", "text": "2-3 cloves minced garlic" },
                { "kind": "ingredient", "text": "salt" },
            ])
        );
        assert_eq!(content["steps"][0]["text"], json!("Noodles & choi sum:"));
        assert_eq!(
            content["yield"],
            json!({ "amount": "4", "noun": "servings" })
        );
        assert_eq!(
            content["nutrition"],
            Value::Null,
            "Crouton's nutrition text is dropped"
        );
        let photo = content["main_photo"].as_str().expect("a Main Photo");
        assert_eq!(chicken["main_photo"], json!(photo));

        // Only the first picture was stored: no favicon, no second photo.
        let stored: i64 = app
            .core
            .db()
            .with_conn(|conn| {
                Ok(conn
                    .query_row("SELECT COUNT(*) FROM photographs", [], |row| row.get(0))
                    .unwrap())
            })
            .unwrap();
        assert_eq!(stored, 1);

        // The staged file was used once and is gone.
        let uploads = app.data_dir().unwrap().join("uploads");
        let left: usize = std::fs::read_dir(&uploads)
            .unwrap()
            .flatten()
            .map(|person| std::fs::read_dir(person.path()).unwrap().count())
            .sum();
        assert_eq!(left, 0, "the staged upload outlived its import");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn importing_the_same_library_again_matches_through_the_ledger() {
        let app = support::spawn_app();
        let (_, key) = a_person(&app);
        let first = import_crouton(&app, &key, &a_library());
        let second = import_crouton(&app, &key, &a_library());

        let statuses: Vec<&Value> = second["arrived"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| &row["status"])
            .collect();
        assert_eq!(statuses, vec![&json!("unchanged"); 3], "{second}");
        assert_eq!(first["import_id"], second["import_id"]);
        for (a, b) in first["arrived"]
            .as_array()
            .unwrap()
            .iter()
            .zip(second["arrived"].as_array().unwrap())
        {
            assert_eq!(a["branch_id"], b["branch_id"], "the library doubled");
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_pair_once_related_is_not_offered_again() {
        let app = support::spawn_app();
        let (_, key) = a_person(&app);
        let report = import_crouton(&app, &key, &a_library());
        let pair = &report["related_candidates"][0]["recipes"];
        let (status, related) = app.post_op(
            "set_related_recipe",
            Some(&key),
            &json!({
                "branch_id": pair[0]["branch_id"],
                "related_branch_id": pair[1]["branch_id"],
                "related": true,
            })
            .to_string(),
        );
        assert_eq!(status, 200, "{related}");

        let again = import_crouton(&app, &key, &a_library());
        assert_eq!(again["related_candidates"], json!([]), "{again}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_forgotten_ledger_leaves_every_recipe_and_matches_nothing_after() {
        let app = support::spawn_app();
        let (_, key) = a_person(&app);
        let first = import_crouton(&app, &key, &a_library());

        let (status, forgotten) = app.post_op(
            "forget_import",
            Some(&key),
            &json!({ "import_id": first["import_id"] }).to_string(),
        );
        assert_eq!(status, 200, "{forgotten}");
        assert_eq!(forgotten["result"]["forgotten"], json!(3));

        // Every recipe it made is still on the shelf, untouched.
        for row in first["arrived"].as_array().unwrap() {
            recipe(&app, &key, row["branch_id"].as_str().unwrap());
        }

        // And the memory is really gone: the same file now arrives as new.
        let again = import_crouton(&app, &key, &a_library());
        assert_ne!(again["import_id"], first["import_id"]);
        assert!(
            again["arrived"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| row["status"] == json!("created")),
            "{again}"
        );

        let (status, missing) = app.post_op(
            "forget_import",
            Some(&key),
            &json!({ "import_id": first["import_id"] }).to_string(),
        );
        assert_eq!(status, 404, "{missing}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn another_persons_ledger_cannot_be_forgotten() {
        let app = support::spawn_app();
        let (_, key) = a_person(&app);
        let report = import_crouton(&app, &key, &a_library());
        let stranger = app.core.create_person("Marc").expect("person");
        let stranger_key = app
            .core
            .mint_access_key(&stranger, "marc", false)
            .unwrap()
            .secret;
        let (status, refused) = app.post_op(
            "forget_import",
            Some(&stranger_key),
            &json!({ "import_id": report["import_id"] }).to_string(),
        );
        assert_ne!(status, 200, "{refused}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn one_crumb_sent_as_base64_at_the_mcp_door_lands_the_same_way() {
        use base64::Engine;
        let app = support::spawn_app();
        let (_, key) = a_person(&app);
        let data = base64::engine::general_purpose::STANDARD.encode(crumb(json!({
            "uuid": "ONE", "name": "Avocado Rice",
            "ingredients": [{ "order": 0, "ingredient": { "name": "rice" }, "quantity": { "amount": 1.5, "quantityType": "CUP" } }],
        })));
        let payload = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {
                "name": "import_crouton",
                "arguments": { "data": data },
                "_meta": tasks_meta()["_meta"],
            },
        });
        let (status, answer) = app.post_mcp(&payload.to_string(), Some(&key));
        assert_eq!(status, 200, "{answer}");
        let job_id = answer["result"]["taskId"]
            .as_str()
            .unwrap_or_else(|| panic!("{answer}"));
        let finished = support::wait_terminal(&app, Some(&key), job_id);
        assert_eq!(finished["status"], json!("completed"), "{finished}");
        let branch_id = finished["result"]["arrived"][0]["branch_id"]
            .as_str()
            .unwrap();
        assert_eq!(
            recipe(&app, &key, branch_id)["versions"][0]["content"]["ingredients"][0]["text"],
            json!("1½ cups rice")
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_upload_is_its_senders_alone_and_a_read_only_key_cannot_send_one() {
        let app = support::spawn_app();
        let (person, key) = a_person(&app);
        let upload_id = upload(&app, &key, &a_library());

        let marc = app.core.create_person("Marc").expect("person");
        let marc_key = app
            .core
            .mint_access_key(&marc, "marc", false)
            .unwrap()
            .secret;
        let (status, asked) = app.post_op(
            "import_crouton",
            Some(&marc_key),
            &json!({ "upload_id": upload_id }).to_string(),
        );
        assert_eq!(status, 200, "{asked}");
        let finished = support::wait_terminal(
            &app,
            Some(&marc_key),
            asked["result"]["job_id"].as_str().unwrap(),
        );
        assert_eq!(finished["status"], json!("failed"), "{finished}");
        assert!(
            finished["error"]
                .as_str()
                .unwrap()
                .contains("no such upload"),
            "{finished}"
        );

        let (status, bad) = app.post_op(
            "import_crouton",
            Some(&key),
            &json!({ "upload_id": "../../kamosu.db" }).to_string(),
        );
        assert_eq!(status, 200, "{bad}");
        let finished =
            support::wait_terminal(&app, Some(&key), bad["result"]["job_id"].as_str().unwrap());
        assert_eq!(finished["status"], json!("failed"), "{finished}");

        let reader = app
            .core
            .mint_access_key(&person, "reader", true)
            .unwrap()
            .secret;
        let (status, refused) =
            app.post_bytes("/api/uploads", Some(&reader), "application/zip", b"PK");
        assert_eq!(status, 401, "{refused}");
        let (status, refused) = app.post_bytes("/api/uploads", None, "application/zip", b"PK");
        assert_eq!(status, 401, "{refused}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_file_that_is_not_a_crouton_export_fails_the_job_and_says_why() {
        let app = support::spawn_app();
        let (_, key) = a_person(&app);
        let upload_id = upload(&app, &key, &an_export(&[("photo.jpg", vec![1, 2, 3])]));
        let (_, asked) = app.post_op(
            "import_crouton",
            Some(&key),
            &json!({ "upload_id": upload_id }).to_string(),
        );
        let finished = support::wait_terminal(
            &app,
            Some(&key),
            asked["result"]["job_id"].as_str().unwrap(),
        );
        assert_eq!(finished["status"], json!("failed"), "{finished}");
        assert!(
            finished["error"]
                .as_str()
                .unwrap()
                .contains("not a Crouton export"),
            "{finished}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_upload_nobody_used_is_swept_after_a_day() {
        let app = support::spawn_app();
        let (person, key) = a_person(&app);
        let stale = upload(&app, &key, b"PK stale");
        let fresh = upload(&app, &key, b"PK fresh");
        let dir = app.data_dir().unwrap().join("uploads").join(&person);
        let two_days_ago = std::time::SystemTime::now() - Duration::from_secs(2 * 24 * 60 * 60);
        std::fs::File::options()
            .write(true)
            .open(dir.join(&stale))
            .unwrap()
            .set_modified(two_days_ago)
            .unwrap();

        assert_eq!(app.core.sweep_uploads().unwrap(), 1);
        assert!(!dir.join(&stale).exists(), "the stale upload stayed");
        assert!(dir.join(&fresh).exists(), "a fresh upload was taken");
    }
}

// ── Sheets (#75, ADR 0023) ───────────────────────────────────────────────────

/// Ask for a Sheet, wait for its Job, and fetch the PDF it left: the result
/// the Job answered, the PDF's content type, and its text with every run of
/// whitespace made one space — so a sentence the page wrapped still reads as
/// one sentence.
fn a_sheet(
    app: &support::TestApp,
    operation: &str,
    bearer: Option<&str>,
    input: Value,
) -> (Value, String, String) {
    let (status, asked) = app.post_op(operation, bearer, &input.to_string());
    assert_eq!(status, 200, "{asked}");
    let job_id = asked["result"]["job_id"]
        .as_str()
        .expect("a job id")
        .to_string();
    let job = support::wait_terminal(app, bearer, &job_id);
    assert_eq!(job["status"], json!("completed"), "{job}");
    let result = job["result"].clone();
    assert_eq!(result["fetch_at"], json!(format!("/api/sheets/{job_id}")));
    let (status, content_type, bytes) = app.get_bytes(&format!("/api/sheets/{job_id}"), bearer);
    assert_eq!(status, 200);
    (result, content_type, pdf_text(&bytes))
}

fn pdf_text(bytes: &[u8]) -> String {
    assert!(bytes.starts_with(b"%PDF-"), "a Sheet is a PDF");
    pdf_extract::extract_text_from_mem(bytes)
        .expect("a Sheet's text can be read back")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// A Sheet's text with no whitespace at all: for what the page may wrap, and
/// for lines set one under the other, which a PDF reader can run together.
fn compact(text: &str) -> String {
    text.split_whitespace().collect()
}

fn read_in(app: &support::TestApp, key: &str, measures: &str) {
    let (status, set) = app.post_op(
        "set_reading_preferences",
        Some(key),
        &json!({ "reading_language": "en", "reading_measures": measures }).to_string(),
    );
    assert_eq!(status, 200, "{set}");
}

/// **A Sheet carries the recipe, not the library** (ADR 0023). Everything that
/// is a fact about the dish is on the page — title, Yield, times, Sections,
/// Ingredient Lines exactly as written, Steps, Note, the Component unfolded
/// after it — and nothing about how Kamosu files it: no Tag, no past Version,
/// no *what changed*, no Attempt, no Reading.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_sheet_carries_the_recipe_and_leaves_the_library_behind() {
    let app = support::spawn_app();
    let (key, _kitchen, pizza, _lineage, _french, _dough, _photos) = a_pizza_worth_sending(&app);
    read_in(&app, &key, "metric");

    // A cooking of it, finished, with a verdict and a note: the Sheet must not
    // know about any of it.
    let (status, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    assert_eq!(status, 200, "{started}");
    let (status, finished) = app.post_op(
        "finish_attempt",
        Some(&key),
        &json!({
            "attempt_id": started["result"]["id"],
            "note": "Too salty this time",
            "rating": "tweak",
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{finished}");

    let (result, content_type, text) = a_sheet(
        &app,
        "make_sheet",
        Some(&key),
        json!({ "branch_id": pizza }),
    );
    assert_eq!(content_type, "application/pdf");
    assert_eq!(result["file_name"], json!("Pizza Margherita.pdf"));

    // Every page carries its footer, and the page ends saying which Version it
    // was set from, by fingerprint.
    let (status, recipe) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    assert_eq!(status, 200, "{recipe}");
    let fingerprint = recipe["result"]["head_version_id"].as_str().unwrap();
    assert!(compact(&text).contains(fingerprint), "{text}");
    let pages = result["pages"].as_u64().unwrap();
    assert!(text.contains(&format!("page {pages} of {pages}")), "{text}");
    assert!(text.contains("printed "), "{text}");

    // The same page asked for again is the Sheet already set, not a second
    // one (ADR 0032).
    let (again, _, _) = a_sheet(
        &app,
        "make_sheet",
        Some(&key),
        json!({ "branch_id": pizza }),
    );
    assert_eq!(again["kept_as"], result["kept_as"]);
    assert_eq!(
        result["paper"],
        json!("a4"),
        "metric Reading Measures print A4"
    );
    assert!(result["pages"].as_u64().unwrap() >= 1);

    for on_the_page in [
        "Pizza Margherita",
        "Makes 2 pizzas",
        "Prep 20 min",
        "Cook 8 min",
        "For the base",
        "500 g Neapolitan pizza dough",
        "To finish",
        "125 g mozzarella",
        "Bake 6 to 8 minutes.",
        "The dough wants making the day before.",
        // The Component, unfolded after the parent and already scaled: half
        // of a dough that makes a kilo.
        "Neapolitan Pizza Dough",
        "½ of the recipe",
        "600 g tipo 00 flour",
        "about 300 g",
        "Knead for ten minutes.",
        // Where it came from: the Version's name, the bare Hand.
        "Less cheese, written",
        "Aurélien",
    ] {
        assert!(
            text.contains(on_the_page),
            "{on_the_page:?} is on the Sheet: {text}"
        );
    }
    for left_behind in [
        "Weekend",             // a Tag is how this Kitchen files it
        "250 g mozzarella",    // a past Version
        "drowning the tomato", // what changed, which belongs to the Thread
        "Too salty",           // an Attempt
        "di bufala",           // what a Reading points at, never printed
        "about 125",           // no Reading beneath a line the recipe is not scaled on
    ] {
        assert!(
            !text.contains(left_behind),
            "{left_behind:?} is not on the Sheet: {text}"
        );
    }
}

/// **The Sheet is the Branch on screen, printed as it stands** (ADR 0023): a
/// cooking that has scaled the recipe scales the page, the one place a Reading
/// reaches paper is the amount beneath a scaled line, and a Reading that would
/// print badly never does — the written line is printed, not what Kamosu read.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_scaled_sheet_prints_the_written_line_and_the_scaled_amount_beneath_it() {
    let app = support::spawn_app();
    let (key, _kitchen, pizza, _lineage, _french, _dough, _photos) = a_pizza_worth_sending(&app);
    read_in(&app, &key, "metric");

    // A Reading gone wrong: the mozzarella line read as anchovy paste.
    let (status, read) = app.post_op(
        "set_reading",
        Some(&key),
        &json!({
            "branch_id": pizza, "line_index": 3,
            "amount": "125", "unit": "g", "target": "anchovy paste",
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{read}");

    let (status, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    assert_eq!(status, 200, "{started}");
    let (status, advanced) = app.post_op(
        "advance_attempt",
        Some(&key),
        &json!({
            "attempt_id": started["result"]["id"],
            "cooking_yield": { "amount": "4", "noun": "pizzas" },
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{advanced}");

    let (_result, _type, text) = a_sheet(
        &app,
        "make_sheet",
        Some(&key),
        json!({ "branch_id": pizza }),
    );
    assert!(
        text.contains("Scaled to 4 pizzas — as written, makes 2"),
        "{text}"
    );
    assert!(
        compact(&text).contains("125gmozzarellaabout250g"),
        "the written line, then the scaled amount beneath it: {text}"
    );
    assert!(
        !text.contains("anchovy"),
        "a Reading is never printed: {text}"
    );
    // Twice the pizza wants the whole kilo of dough, so the dough is printed
    // as written and gains nothing beneath its lines.
    assert!(text.contains("the whole recipe"), "{text}");
    assert!(text.contains("600 g tipo 00 flour"), "{text}");
    assert!(!text.contains("about 600"), "{text}");
}

/// **A stranger holding a Share Link is offered a Sheet too** (ADR 0023): no
/// account, no Credential, the page size decided by their locale, the dough
/// carried as a Passenger and printed at the amount the pizza asks for, and
/// the link itself in the provenance block.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stranger_with_a_share_link_gets_a_sheet_without_an_account() {
    let app = support::spawn_app();
    let (key, _kitchen, pizza, _lineage, _french, _dough, _photos) = a_pizza_worth_sending(&app);
    let (token, url) = share(&app, &key, &pizza);

    let (result, content_type, text) = a_sheet(
        &app,
        "make_shared_sheet",
        None,
        json!({ "token": token, "locale": "en-US,en;q=0.9" }),
    );
    assert_eq!(content_type, "application/pdf");
    assert_eq!(
        result["paper"],
        json!("us-letter"),
        "a US locale prints Letter"
    );
    for on_the_page in [
        "Pizza Margherita",
        "125 g mozzarella",
        "Neapolitan Pizza Dough",
        "½ of the recipe",
        "about 300 g",
    ] {
        assert!(
            text.contains(on_the_page),
            "{on_the_page:?} is on the Sheet: {text}"
        );
    }
    // The link itself, which the page is free to wrap.
    assert!(
        compact(&text).contains(&url),
        "the Share Link is on the Sheet: {text}"
    );
    assert!(!text.contains("Weekend"), "{text}");

    // Any other locale, or none, prints A4.
    let (result, _, _) = a_sheet(&app, "make_shared_sheet", None, json!({ "token": token }));
    assert_eq!(result["paper"], json!("a4"));

    // One of its Translations, in its own words.
    let (_, _, french) = a_sheet(
        &app,
        "make_shared_sheet",
        None,
        json!({ "token": token, "language": "fr" }),
    );
    assert!(french.contains("125 g de mozzarella"), "{french}");
    assert!(
        french.contains("Pour 2 pizzas"),
        "the page speaks the recipe's Language: {french}"
    );
    assert!(french.contains("imprimée le"), "{french}");
}

/// The Share Link page runs no script, so its Sheet is a link: it starts the
/// Job and sends the reader to the Job's own address, which is the PDF once it
/// is set.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_share_link_pages_sheet_is_a_link_that_ends_in_the_pdf() {
    let app = support::spawn_app();
    let (key, _kitchen, pizza, _lineage, _french, _dough, _photos) = a_pizza_worth_sending(&app);
    let (token, _url) = share(&app, &key, &pizza);

    let (status, _, page) = app.get(&format!("/s/{token}"));
    assert_eq!(status, 200);
    assert!(
        page.contains(&format!("href=\"/s/{token}/sheet\"")),
        "the page offers a Sheet"
    );

    let asked = kamosu::http_min::get(app.addr, &format!("/s/{token}/sheet")).expect("a reply");
    assert_eq!(asked.status, 303);
    let at = asked
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("location"))
        .map(|(_, value)| value.clone())
        .expect("sent on to the Sheet's own address");
    assert!(at.starts_with(&format!("/s/{token}/sheet/j_")), "{at}");

    let mut fetched = None;
    for _ in 0..400 {
        let (status, content_type, bytes) = app.get_bytes(&at, None);
        assert_eq!(status, 200);
        if content_type == "application/pdf" {
            fetched = Some(bytes);
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let text = pdf_text(&fetched.expect("the address becomes the PDF"));
    assert!(text.contains("Pizza Margherita"), "{text}");

    // Ending the link ends the Sheet with it — one already set included,
    // since fetching it through the link is arriving (ADR 0018).
    let (status, ended) = app.post_op(
        "end_share_link",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    assert_eq!(status, 200, "{ended}");
    let (_, content_type, _) = app.get_bytes(&at, None);
    assert_ne!(
        content_type, "application/pdf",
        "the ended link hands over nothing"
    );
    let job_id = at.rsplit('/').next().unwrap();
    let (status, _, _) = app.get_bytes(&format!("/api/sheets/{job_id}"), None);
    assert_eq!(status, 404);
    let (status, asked) = app.post_op(
        "make_shared_sheet",
        None,
        &json!({ "token": token }).to_string(),
    );
    assert_eq!(status, 200, "{asked}");
    let job = support::wait_terminal(&app, None, asked["result"]["job_id"].as_str().unwrap());
    assert_eq!(job["status"], json!("failed"), "{job}");
}

/// A Person's Sheet is theirs: fetched under the Credential that asked for it,
/// and by nobody else. The paper follows their Reading Measures.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_persons_sheet_is_fetched_only_under_their_credential() {
    let app = support::spawn_app();
    let (key, _kitchen, pizza, _lineage, _french, _dough, _photos) = a_pizza_worth_sending(&app);
    // US measures are the stated default, and they print Letter.
    let (result, _, _) = a_sheet(
        &app,
        "make_sheet",
        Some(&key),
        json!({ "branch_id": pizza }),
    );
    assert_eq!(result["paper"], json!("us-letter"));
    let fetch_at = result["fetch_at"].as_str().unwrap().to_string();

    let (status, _, _) = app.get_bytes(&fetch_at, None);
    assert_ne!(status, 200, "a stranger cannot fetch a Person's Sheet");
    let (_other, other_key, _) = person_with_kitchen(&app, "Someone else");
    let (status, _, _) = app.get_bytes(&fetch_at, Some(&other_key));
    assert_ne!(status, 200, "nor can another Person");

    // And nobody outside the Kitchen can ask for one of this recipe at all.
    let (status, asked) = app.post_op(
        "make_sheet",
        Some(&other_key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    assert_eq!(status, 200, "{asked}");
    let job = support::wait_terminal(
        &app,
        Some(&other_key),
        asked["result"]["job_id"].as_str().unwrap(),
    );
    assert_eq!(job["status"], json!("failed"), "{job}");
}

// --- Cooking and shopping with no network (issue #77, ADR 0013) --------------
//
// A phone with no signal keeps the writes on the Attempt's side of Promotion,
// and the Shopping List's, and sends them when it can. These are those writes
// arriving late, through the same Doors as any other.

/// A time `minutes` before now, as a phone writes one.
fn minutes_ago(app: &support::TestApp, minutes: i64) -> String {
    app.core
        .db()
        .with_conn(|conn| {
            conn.query_row(
                "SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now', ?1)",
                rusqlite::params![format!("-{minutes} minutes")],
                |row| row.get(0),
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .expect("a time")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cooking_started_offline_lands_under_the_id_the_phone_gave_it_and_on_the_day_it_happened()
{
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let saturday = minutes_ago(&app, 2 * 24 * 60);

    let start = json!({
        "branch_id": branch_id,
        "attempt_id": "at_0123456789abcdef",
        "started_at": saturday,
    })
    .to_string();
    let (status, started) = app.post_op("start_attempt", Some(&key), &start);
    assert_eq!(status, 200, "{started}");
    assert_eq!(started["result"]["id"], json!("at_0123456789abcdef"));
    assert_eq!(
        started["result"]["created_at"],
        json!(saturday),
        "a cooking at your parents' on Saturday is dated Saturday, whenever it arrives"
    );

    // A phone that sent it and never heard back sends it again.
    let (status, again) = app.post_op("start_attempt", Some(&key), &start);
    assert_eq!(status, 200, "{again}");
    assert_eq!(again["result"]["id"], json!("at_0123456789abcdef"));
    let count: i64 = app
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row("SELECT COUNT(*) FROM attempts", [], |r| r.get(0))
                .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .unwrap();
    assert_eq!(count, 1, "the same start sent twice is one cooking");

    // An id in any other shape is refused rather than stored.
    let (status, refused) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id, "attempt_id": "mine" }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");
    let (status, refused) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id, "started_at": "last saturday" }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cooking_started_offline_follows_the_one_another_device_already_began() {
    // One Attempt per person per Lineage (ADR 0010). The iPad started this
    // cooking while the phone had no signal and started it too; when the
    // phone's start arrives it is handed the iPad's, and nobody is asked
    // which cooking they meant.
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let (_, on_the_ipad) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let (status, from_the_phone) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({
            "branch_id": branch_id,
            "attempt_id": "at_00000000000000aa",
            "started_at": minutes_ago(&app, 30),
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{from_the_phone}");
    assert_eq!(from_the_phone["result"]["id"], on_the_ipad["result"]["id"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_move_that_arrives_late_never_puts_the_cook_back_a_step() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let (_, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let attempt_id = started["result"]["id"].as_str().unwrap().to_string();

    // Only opening the screen on the iPad is not moving on: a move the phone
    // made before that, sent afterwards, still lands.
    let (status, landed) = app.post_op(
        "advance_attempt",
        Some(&key),
        &json!({
            "attempt_id": attempt_id,
            "current_step_index": 1,
            "ticked_ingredients": [0],
            "written_at": minutes_ago(&app, 20),
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{landed}");
    assert_eq!(landed["result"]["current_step_index"], json!(1));

    // The iPad moves on to the last step, now.
    app.post_op(
        "advance_attempt",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "current_step_index": 2 }).to_string(),
    );

    // The phone's older move, held while it had no signal, arrives after it.
    let (status, late) = app.post_op(
        "advance_attempt",
        Some(&key),
        &json!({
            "attempt_id": attempt_id,
            "current_step_index": 0,
            "ticked_ingredients": [],
            "written_at": minutes_ago(&app, 10),
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{late}");
    assert_eq!(
        late["result"]["current_step_index"],
        json!(2),
        "the last one moved on is where the cook is (ADR 0010), by when it moved"
    );
    assert_eq!(late["result"]["ticked_ingredients"], json!([0]));

    // A clock running fast cannot win every argument: a time after now is now.
    let (status, fast) = app.post_op(
        "advance_attempt",
        Some(&key),
        &json!({
            "attempt_id": attempt_id,
            "current_step_index": 1,
            "written_at": "2099-01-01T00:00:00.000Z",
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{fast}");
    let (_, next) = app.post_op(
        "advance_attempt",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "current_step_index": 2 }).to_string(),
    );
    assert_eq!(
        next["result"]["current_step_index"],
        json!(2),
        "a move from a phone whose clock is in 2099 does not outrank every move after it"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cooking_recorded_offline_lands_unchanged_against_a_recipe_edited_meanwhile() {
    // The ugly case in most offline systems, closed here by decisions made for
    // other reasons (ADR 0013): the Version cooked still exists, so the
    // cooking lands exactly as it was recorded and says it cooked that one.
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let (_, before) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let cooked_version = before["result"]["head_version_id"]
        .as_str()
        .unwrap()
        .to_string();

    // While the phone is out, somebody edits the recipe at home.
    backdate_branch_head(&app, &branch_id);
    let (status, edited) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": branch_id,
            "title": "Katsu Curry",
            "ingredients": [
                { "kind": "ingredient", "text": "3 escalopes de poulet" },
                { "kind": "ingredient", "text": "200 g de riz" },
            ],
            "steps": [{ "kind": "step", "text": "Tout frire" }],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{edited}");
    assert_ne!(edited["result"]["version_id"], json!(cooked_version));

    // The phone comes home and sends what it did, in order, each saying when.
    let photograph = upload_a_picture(&app, &key, 77);
    let attempt_id = "at_77777777777777aa";
    let finished_at = minutes_ago(&app, 60);
    let sent = [
        (
            "start_attempt",
            json!({ "branch_id": branch_id, "version_id": cooked_version, "attempt_id": attempt_id, "started_at": minutes_ago(&app, 90) }),
        ),
        (
            "advance_attempt",
            json!({ "attempt_id": attempt_id, "current_step_index": 2, "ticked_ingredients": [0, 1], "written_at": minutes_ago(&app, 80) }),
        ),
        (
            "set_as_cooked",
            json!({ "attempt_id": attempt_id, "written_at": minutes_ago(&app, 70), "as_cooked": {
            "title": "Katsu Curry",
            "ingredients": [
                { "kind": "ingredient", "text": "2 escalopes de poulet" },
                { "kind": "ingredient", "text": "150 g de riz" },
            ],
            "steps": [
                { "kind": "step", "text": "Paner les escalopes" },
                { "kind": "step", "text": "Frire jusqu'à dorer" },
                { "kind": "step", "text": "Servir avec le riz" },
            ],
        } }),
        ),
        (
            "edit_attempt",
            json!({ "attempt_id": attempt_id, "add_photographs": [photograph], "written_at": minutes_ago(&app, 65) }),
        ),
        (
            "finish_attempt",
            json!({ "attempt_id": attempt_id, "written_at": finished_at }),
        ),
        (
            "edit_attempt",
            json!({ "attempt_id": attempt_id, "rating": "again", "note": "Chez mes parents", "written_at": minutes_ago(&app, 50) }),
        ),
    ];
    let mut last = Value::Null;
    for (operation, input) in sent {
        let (status, answered) = app.post_op(operation, Some(&key), &input.to_string());
        assert_eq!(status, 200, "{operation}: {answered}");
        last = answered;
    }

    let attempt = &last["result"];
    assert_eq!(attempt["id"], json!(attempt_id));
    assert_eq!(
        attempt["version_id"],
        json!(cooked_version),
        "the cooking still says which Version it cooked, not the one written since"
    );
    assert_eq!(attempt["current_step_index"], json!(2));
    assert_eq!(attempt["ticked_ingredients"], json!([0, 1]));
    assert_eq!(attempt["photographs"], json!([photograph]));
    assert_eq!(attempt["rating"], json!("again"));
    assert_eq!(attempt["note"], json!("Chez mes parents"));
    assert_eq!(
        attempt["as_cooked"]["content"]["ingredients"][1]["text"],
        json!("150 g de riz")
    );
    assert_eq!(
        attempt["finished_at"],
        json!(finished_at),
        "it finished when the cook finished, not when the phone got home"
    );

    // And the recipe is exactly what the edit at home made it: the cooking's
    // words never reached it.
    let (_, after) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(
        after["result"]["head_version_id"],
        edited["result"]["version_id"]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_picture_taken_on_one_device_never_erases_one_taken_on_another() {
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let (_, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let attempt_id = started["result"]["id"].as_str().unwrap().to_string();
    let on_the_phone = upload_a_picture(&app, &key, 11);
    let on_the_ipad = upload_a_picture(&app, &key, 22);

    for picture in [&on_the_phone, &on_the_ipad, &on_the_phone] {
        let (status, added) = app.post_op(
            "edit_attempt",
            Some(&key),
            &json!({ "attempt_id": attempt_id, "add_photographs": [picture] }).to_string(),
        );
        assert_eq!(status, 200, "{added}");
    }
    let (_, current) = app.post_op(
        "edit_attempt",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "add_photographs": [] }).to_string(),
    );
    assert_eq!(
        current["result"]["photographs"],
        json!([on_the_phone, on_the_ipad]),
        "added beside each other, once each, in the order taken"
    );

    let (status, refused) = app.post_op(
        "edit_attempt",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "add_photographs": ["not-a-photograph"] }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_last_device_to_write_a_shopping_list_wins() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (chicken, coq) = two_real_recipes(&app, &key, &kitchen_id);

    // Yesterday, with no signal, the phone chose the chicken and typed a line.
    let yesterday = minutes_ago(&app, 24 * 60);
    // This morning, online, the iPad chose the coq au vin.
    let (_, ipad) = app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": coq }).to_string(),
    );
    assert_eq!(ipad["result"]["chosen"].as_array().unwrap().len(), 1);

    // The phone's writes arrive now, older than the iPad's.
    for (operation, input) in [
        (
            "add_to_shopping_list",
            json!({ "branch_id": chicken, "written_at": yesterday }),
        ),
        (
            "add_loose_item",
            json!({ "text": "bin bags", "item_id": "i_00000000000000bb", "written_at": yesterday }),
        ),
        (
            "remove_from_shopping_list",
            json!({ "branch_id": coq, "written_at": yesterday }),
        ),
        ("empty_shopping_list", json!({ "written_at": yesterday })),
    ] {
        let (status, answered) = app.post_op(operation, Some(&key), &input.to_string());
        assert_eq!(status, 200, "{operation}: {answered}");
        assert_eq!(
            answered["result"], ipad["result"],
            "{operation} written before the iPad's list changes nothing"
        );
    }

    // A write the phone made after the iPad's does land, a line typed offline
    // keeps the id the phone gave it, and the same line sent twice is one.
    let later = json!({ "text": "coffee", "item_id": "i_00000000000000cc" }).to_string();
    app.post_op("add_loose_item", Some(&key), &later);
    let (_, twice) = app.post_op("add_loose_item", Some(&key), &later);
    let loose = twice["result"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["kind"] == json!("loose"))
        .collect::<Vec<_>>();
    assert_eq!(loose.len(), 1, "{twice}");
    assert_eq!(loose[0]["id"], json!("i_00000000000000cc"));
    let (_, removed) = app.post_op(
        "remove_loose_item",
        Some(&key),
        &json!({ "item_id": "i_00000000000000cc" }).to_string(),
    );
    assert!(
        removed["result"]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["kind"] != json!("loose"))
    );

    let (status, refused) = app.post_op(
        "add_loose_item",
        Some(&key),
        &json!({ "text": "tea", "item_id": "not-mine" }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_recipes_shopping_basis_is_what_its_rows_are_added_up_from() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    app.post_op(
        "set_reading_preferences",
        Some(&key),
        &json!({ "reading_language": "en", "reading_measures": "metric" }).to_string(),
    );
    let (_chicken, coq) = two_real_recipes(&app, &key, &kitchen_id);

    let (status, basis) = app.post_op(
        "shopping_basis",
        Some(&key),
        &json!({ "branch_id": coq }).to_string(),
    );
    assert_eq!(status, 200, "{basis}");
    let basis = &basis["result"];
    assert_eq!(basis["branch_id"], json!(coq));
    assert_eq!(
        basis["written_yield"],
        json!({ "amount": "4", "noun": "servings" })
    );
    let lines = basis["lines"].as_array().unwrap();
    assert!(
        lines
            .iter()
            .all(|line| line["text"] != json!("For the braise")),
        "a Section is not a thing to buy"
    );
    let soy = lines
        .iter()
        .find(|line| line["text"] == json!("1 tbsp soy sauce"))
        .expect("the soy sauce line");
    assert_eq!(
        soy["path"],
        json!([2]),
        "named by where it sits in the recipe"
    );
    assert_eq!(
        soy["from"],
        Value::Null,
        "and by nothing else: it is the chosen recipe's own line, not a Component's"
    );
    assert_eq!(soy["food"]["name"], json!("soy sauce"));
    assert_eq!(soy["food"]["amount"], json!(1.0));
    assert_eq!(soy["food"]["unit_id"], json!("tablespoon"));

    // Somebody who cannot see the recipe cannot read what it would buy.
    let (_other, stranger, _kitchen) = person_with_kitchen(&app, "Marie");
    let (status, refused) = app.post_op(
        "shopping_basis",
        Some(&stranger),
        &json!({ "branch_id": coq }).to_string(),
    );
    assert_ne!(status, 200, "{refused}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_finished_cooking_is_final_whatever_arrives_late() {
    // Finished on the iPad; the phone, with no network, went on moving through
    // it and finished it too. What the phone sends later changes nothing and
    // fails nothing: one cooking, finished once (#77).
    let app = support::spawn_app();
    let (key, branch_id, _lineage, _kitchen) = recipe_ready_to_cook(&app, "Aurélien");
    let (_, started) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let attempt_id = started["result"]["id"].as_str().unwrap().to_string();
    let (_, finished) = app.post_op(
        "finish_attempt",
        Some(&key),
        &json!({ "attempt_id": attempt_id }).to_string(),
    );
    let finished_at = finished["result"]["finished_at"].clone();

    for (operation, input) in [
        (
            "advance_attempt",
            json!({ "attempt_id": attempt_id, "current_step_index": 2, "written_at": minutes_ago(&app, 5) }),
        ),
        (
            "finish_attempt",
            json!({ "attempt_id": attempt_id, "rating": "again", "note": "Chez mes parents", "written_at": minutes_ago(&app, 5) }),
        ),
    ] {
        let (status, answered) = app.post_op(operation, Some(&key), &input.to_string());
        assert_eq!(status, 200, "{operation}: {answered}");
        assert_eq!(
            answered["result"]["finished_at"], finished_at,
            "{operation}"
        );
        assert_eq!(
            answered["result"]["current_step_index"],
            json!(0),
            "{operation}"
        );
    }

    // What the cook said with the late finish still lands: only the finish
    // itself is the first one's.
    let (_, diary) = app.post_op("list_attempts", Some(&key), "{}");
    let kept = &diary["result"]["attempts"][0];
    assert_eq!(kept["rating"], json!("again"));
    assert_eq!(kept["note"], json!("Chez mes parents"));

    // Asked live, with no time of its own, it is still a mistake worth saying.
    let (status, refused) = app.post_op(
        "advance_attempt",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "current_step_index": 1 }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");
}

// ── Deleting a recipe (#120) ─────────────────────────────────────────────────
//
// A recipe could arrive by three routes and leave by none, which is how a dev
// instance came to hold 295 recipes for a library of 86. `delete_recipe` takes
// one Branch off the shelf for good. What it must NOT take with it is most of
// what these tests are about.

/// Ask the database a question these tests cannot ask through a Door, because
/// no Operation counts rows nobody is meant to think about.
fn count_of(app: &support::TestApp, sql: &str) -> i64 {
    app.core
        .db()
        .with_conn(|conn| {
            conn.query_row(sql, [], |row| row.get(0))
                .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .expect("a count")
}

fn delete_recipe(app: &support::TestApp, key: &str, branch_id: &str) -> (u16, Value) {
    app.post_op(
        "delete_recipe",
        Some(key),
        &json!({ "branch_id": branch_id }).to_string(),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_deleted_recipe_leaves_the_shelf_the_search_and_home_for_everyone_in_its_kitchen() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let (_housemate, housemate_key, _) = person_with_kitchen(&app, "Marc");

    // Marc cooks here too, so the delete has to reach his shelf as well as
    // hers: a Branch belongs to the Kitchen, never to whoever typed it.
    let (_, invite) = app.post_op(
        "invite_to_kitchen",
        Some(&key),
        &json!({ "kitchen_id": kitchen_id }).to_string(),
    );
    let secret = invite["result"]["secret"].as_str().unwrap().to_string();
    app.post_op(
        "accept_kitchen_invite",
        Some(&housemate_key),
        &json!({ "secret": secret }).to_string(),
    );

    let doomed = shelve(&app, &key, &kitchen_id, "Soba with walnut miso");
    let kept = shelve(&app, &key, &kitchen_id, "Tarte aux pommes");

    // Opened, so it stands on Home's *lately* shelf before it goes.
    app.post_op(
        "note_recipe_opened",
        Some(&key),
        &json!({ "branch_id": doomed }).to_string(),
    );

    // A Meaning Search row of the kind `build_meaning_index` writes, put here
    // directly because no model runs in these tests (ADR 0029) and a vacuous
    // assertion would prove nothing. The index is derived and never truth, so
    // a deleted recipe leaves it at once rather than waiting for a rebuild.
    app.core
        .db()
        .with_conn(|conn| {
            conn.execute(
                "INSERT INTO meaning_vectors \
                   (id, role, lineage_id, branch_id, embedding_space, embedded_text, vector) \
                 VALUES ('mv_120', 'block', \
                   (SELECT lineage_id FROM branches WHERE id = ?1), ?1, \
                   'test-space', 'soba with walnut miso', x'00')",
                rusqlite::params![doomed],
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            Ok(())
        })
        .expect("a Meaning Search row");

    let (status, answered) = delete_recipe(&app, &key, &doomed);
    assert_eq!(status, 200, "{answered}");
    assert_eq!(answered["result"], json!({ "deleted": true }));

    assert_eq!(
        count_of(&app, "SELECT COUNT(*) FROM meaning_vectors"),
        0,
        "the deleted recipe is still in Meaning Search"
    );

    // Off the shelf, for both of them.
    for (whose, their_key) in [("hers", &key), ("his", &housemate_key)] {
        let standing = shelf(&app, their_key, json!({}));
        assert_eq!(
            titles(&standing),
            vec!["Tarte aux pommes"],
            "the deleted recipe is still on {whose} shelf"
        );
        let searched = shelf(&app, their_key, json!({ "query": "soba" }));
        assert_eq!(
            titles(&searched),
            Vec::<&str>::new(),
            "the deleted recipe still answers {whose} search"
        );
        let (status, refused) = app.post_op(
            "get_recipe",
            Some(their_key),
            &json!({ "branch_id": doomed }).to_string(),
        );
        assert_eq!(status, 404, "{refused}");
        assert_eq!(refused["error"]["message"], json!("no such Branch"));
    }

    // And off Home. The `recipe_opens` row naming its Lineage is left where it
    // is — it is keyed on the Lineage, which a sibling Branch may still stand
    // on — and it is inert: Home reads those only as an ordering over recipes
    // already on the shelf, so one that is not there cannot be ranked onto it.
    let (_, home) = app.post_op("home_shelves", Some(&key), "{}");
    let shown = serde_json::to_string(&home["result"]).unwrap();
    assert!(
        !shown.contains("Soba with walnut miso"),
        "Home still carries the deleted recipe: {shown}"
    );

    // The recipe left; the recipe beside it did not.
    let (status, still_there) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": kept }).to_string(),
    );
    assert_eq!(status, 200, "{still_there}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deleting_one_branch_of_a_lineage_leaves_its_translation_whole() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    let english = shelve_recipe(
        &app,
        &key,
        &kitchen_id,
        json!({
            "title": "Chocolate mousse",
            "ingredients": [{ "kind": "ingredient", "text": "200 g dark chocolate" }],
            "steps": [{ "kind": "step", "text": "Melt the chocolate." }],
        }),
    );
    let (status, translated) = app.post_op(
        "start_translation",
        Some(&key),
        &json!({
            "branch_id": english,
            "language": "fr",
            "title": "Mousse au chocolat",
            "ingredients": [{ "kind": "ingredient", "text": "200 g de chocolat noir" }],
            "steps": [{ "kind": "step", "text": "Faire fondre le chocolat." }],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{translated}");
    let french = translated["result"]["branch_id"]
        .as_str()
        .unwrap()
        .to_string();

    // A translation is an ordinary Branch (ADR 0006). Deleting the English one
    // is deleting one Branch, not the recipe in every language it was written.
    let (status, answered) = delete_recipe(&app, &key, &english);
    assert_eq!(status, 200, "{answered}");

    let (status, gone) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": english }).to_string(),
    );
    assert_eq!(status, 404, "{gone}");

    // The French one opens, reads and cooks exactly as before.
    let (status, whole) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": french }).to_string(),
    );
    assert_eq!(status, 200, "{whole}");
    let content = &whole["result"]["versions"][0]["content"];
    assert_eq!(content["title"], json!("Mousse au chocolat"), "{whole}");
    assert_eq!(
        content["ingredients"][0]["text"],
        json!("200 g de chocolat noir")
    );
    let (status, cooking) = app.post_op(
        "start_attempt",
        Some(&key),
        &json!({ "branch_id": french }).to_string(),
    );
    assert_eq!(status, 200, "{cooking}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_deleted_recipe_keeps_every_cooking_it_was_ever_made_for() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let branch_id = shelve(&app, &key, &kitchen_id, "Soba with walnut miso");

    let picture = upload_a_picture(&app, &key, 7);
    let attempt_id = cook_it(
        &app,
        &key,
        &branch_id,
        json!({ "rating": "again", "note": "Chez mes parents", "photographs": [picture] }),
    );

    let (status, answered) = delete_recipe(&app, &key, &branch_id);
    assert_eq!(status, 200, "{answered}");

    // **The whole of the second choice on #120.** Cooked eleven times over two
    // years is the record a person would least expect a delete to take, and an
    // Attempt is a private diary entry deleted on its own terms (ADR 0010).
    let (status, diary) = app.post_op("list_attempts", Some(&key), "{}");
    assert_eq!(status, 200, "{diary}");
    let entries = diary["result"]["attempts"].as_array().expect("attempts");
    assert_eq!(entries.len(), 1, "{diary}");
    let kept = &entries[0];
    assert_eq!(kept["id"], json!(attempt_id));
    assert_eq!(kept["rating"], json!("again"));
    assert_eq!(kept["note"], json!("Chez mes parents"));
    assert_eq!(
        kept["recipe"],
        json!({ "branch_id": Value::Null, "title": "Soba with walnut miso" }),
        "the diary must keep the name the recipe was known by and offer no way \
         into a recipe that is not there: {kept}"
    );
    assert_eq!(
        kept["photographs"].as_array().map(Vec::len),
        Some(1),
        "the cooking lost its Photograph: {kept}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_deleted_recipe_stays_on_the_shopping_list_and_says_it_cannot_be_read() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    let branch_id = shelve_recipe(
        &app,
        &key,
        &kitchen_id,
        json!({
            "title": "Ratatouille aux anchois",
            "ingredients": [{ "kind": "ingredient", "text": "2 tbsp soy sauce" }],
        }),
    );
    app.post_op(
        "add_to_shopping_list",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    let (_, before) = app.post_op("get_shopping_list", Some(&key), "{}");
    assert_eq!(before["result"]["chosen"][0]["gone"], json!(false));
    assert_eq!(before["result"]["rows"].as_array().unwrap().len(), 1);

    let (status, answered) = delete_recipe(&app, &key, &branch_id);
    assert_eq!(status, 200, "{answered}");

    // **The path written against a deletion that could not happen, reached at
    // last.** A thing that quietly disappears from a shopping list is a thing
    // that does not get bought (ADR 0024), so the entry stays, keeps the name
    // it was known by, contributes nothing, and says so. Migration 33 is what
    // lets it: the foreign key here would have refused the delete outright.
    let (_, list) = app.post_op("get_shopping_list", Some(&key), "{}");
    assert_eq!(
        list["result"]["chosen"][0],
        json!({
            "branch_id": branch_id,
            "title": "Ratatouille aux anchois",
            "gone": true,
            "shopping_yield": Value::Null,
            "written_yield": Value::Null,
        })
    );
    assert_eq!(
        list["result"]["rows"],
        json!([]),
        "a recipe that cannot be read contributes no rows"
    );

    // And it can still be taken off, which is exactly the entry somebody most
    // wants gone.
    let (status, emptied) = app.post_op(
        "remove_from_shopping_list",
        Some(&key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 200, "{emptied}");
    assert_eq!(emptied["result"]["chosen"], json!([]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deleting_a_recipe_deletes_no_version_and_no_other_branchs_reading() {
    let app = support::spawn_app();
    let (person, key, kitchen_a) = person_with_kitchen(&app, "Aurélien");
    let kitchen_b = home_kitchen_of(&app, &person);

    let mine = shelve_recipe(
        &app,
        &key,
        &kitchen_a,
        json!({
            "title": "Korean fried chicken",
            "ingredients": [{ "kind": "ingredient", "text": "2 tbsp soy sauce" }],
        }),
    );

    // A Copy: saving into a *different* Kitchen the same Person cooks in
    // starts a second Branch holding the very same Version. That shared row is
    // the trap this test exists for.
    backdate_branch_head(&app, &mine);
    let (status, copied) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": mine,
            "kitchen_id": kitchen_b,
            "title": "Korean fried chicken, baked",
            "ingredients": [{ "kind": "ingredient", "text": "2 tbsp soy sauce" }],
        })
        .to_string(),
    );
    assert_eq!(status, 200, "{copied}");
    let theirs = copied["result"]["branch_id"].as_str().unwrap().to_string();
    assert_eq!(
        copied["result"]["copied"],
        json!(true),
        "that was not a Copy: {copied}"
    );
    assert_ne!(theirs, mine, "that was not a Copy: {copied}");

    let shared_version = count_of(
        &app,
        "SELECT COUNT(DISTINCT version_id) FROM branch_versions \
          GROUP BY version_id HAVING COUNT(DISTINCT branch_id) > 1 LIMIT 1",
    );
    assert_eq!(shared_version, 1, "the two Branches share no Version");

    // A Reading correction, which travels beside the Version and belongs to
    // every Branch holding it (ADR 0021).
    let (status, read) = app.post_op(
        "set_reading",
        Some(&key),
        &json!({ "branch_id": mine, "line_index": 0, "unit": "tbsp", "amount": "2" }).to_string(),
    );
    assert_eq!(status, 200, "{read}");

    let versions_before = count_of(&app, "SELECT COUNT(*) FROM versions");
    let readings_before = count_of(&app, "SELECT COUNT(*) FROM readings");
    assert!(readings_before > 0, "the Reading was never written");

    let (status, answered) = delete_recipe(&app, &key, &mine);
    assert_eq!(status, 200, "{answered}");

    // **No Version is ever deleted, by this or by anything else** (ADR 0004,
    // spec item 59). A Version is global: the row this Branch held is the row
    // the other one holds.
    assert_eq!(
        count_of(&app, "SELECT COUNT(*) FROM versions"),
        versions_before,
        "a Version was deleted"
    );
    assert_eq!(
        count_of(
            &app,
            "SELECT COUNT(*) FROM versions WHERE id <> version_fingerprint(content)"
        ),
        0,
        "a Version no longer fingerprints to its own id"
    );

    // And no Reading. They are keyed on `version_id`, not on a Branch, so
    // sweeping this Branch's would take the correction off the other copy.
    assert_eq!(
        count_of(&app, "SELECT COUNT(*) FROM readings"),
        readings_before,
        "deleting a Branch took a Reading off a Version another Branch holds"
    );
    let (status, other) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": theirs }).to_string(),
    );
    assert_eq!(status, 200, "{other}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_deleted_recipes_share_link_stops_resolving() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");
    let branch_id = shelve(&app, &key, &kitchen_id, "Soba with walnut miso");

    let (status, shared) = app.post_op(
        "share_recipe",
        Some(&key),
        &json!({ "branch_id": branch_id, "public_address": "https://kamosu.example" }).to_string(),
    );
    assert_eq!(status, 200, "{shared}");
    let url = shared["result"]["url"]
        .as_str()
        .expect("a link")
        .to_string();
    let token = url.rsplit('/').next().expect("a token").to_string();

    let (status, _type, body) = app.get_bytes(&format!("/s/{token}"), None);
    assert_eq!(status, 200, "{}", String::from_utf8_lossy(&body));

    let (status, answered) = delete_recipe(&app, &key, &branch_id);
    assert_eq!(status, 200, "{answered}");

    // The row went with the Branch, so the page answers as it does for a token
    // that was never minted. This is a consequence of deleting, not a
    // withdrawal, so ADR 0018's "sharing ended" is not what it says.
    let (status, _type, gone) = app.get_bytes(&format!("/s/{token}"), None);
    let never = "tk_ffffffffffffffffffffffffffffffff";
    let (never_status, _type, never_body) = app.get_bytes(&format!("/s/{never}"), None);
    assert_eq!(
        (status, String::from_utf8_lossy(&gone).to_string()),
        (
            never_status,
            String::from_utf8_lossy(&never_body).to_string()
        ),
        "a deleted recipe's link does not answer as a token nobody minted"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deleting_a_recipe_lets_the_importer_bring_it_back_as_new() {
    let app = support::spawn_app();
    // The importer lands a recipe in the Home Kitchen of whoever asked, so the
    // Kitchen made above is not named here.
    let (_person, key, _kitchen_id) = person_with_kitchen(&app, "Aurélien");

    let bring_in = || {
        import_and_wait(
            &app,
            &key,
            &json!({
                "source_kind": "crouton",
                "candidates": [{
                    "foreign_id": "crouton-uuid-120",
                    "title": "Soba with walnut miso",
                    "ingredients": [{ "kind": "ingredient", "text": "200 g soba" }],
                    "steps": [{ "kind": "step", "text": "Boil the noodles.", "photo": null }],
                }],
            }),
        )
    };

    let first = bring_in();
    assert_eq!(first["arrived"][0]["status"], json!("created"), "{first}");
    let branch_id = first["arrived"][0]["branch_id"]
        .as_str()
        .expect("a Branch")
        .to_string();

    // Run again and the ledger recognises it: nothing arrives twice.
    let again = bring_in();
    assert_eq!(again["arrived"][0]["status"], json!("unchanged"), "{again}");

    let (status, answered) = delete_recipe(&app, &key, &branch_id);
    assert_eq!(status, 200, "{answered}");

    // **The ledger belongs to the Import, not to the recipe** (ADR 0025). Its
    // row went with the Branch, so the foreign id matches nothing and the
    // recipe arrives afresh rather than being recognised as the deleted one.
    let afresh = bring_in();
    assert_eq!(afresh["arrived"][0]["status"], json!("created"), "{afresh}");
    assert_ne!(
        afresh["arrived"][0]["branch_id"].as_str(),
        Some(branch_id.as_str()),
        "the importer matched the deleted recipe instead of creating one"
    );
    assert_eq!(
        titles(&shelf(&app, &key, json!({}))),
        vec!["Soba with walnut miso"]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_component_line_naming_a_deleted_recipe_still_reads_and_says_what_happened() {
    let app = support::spawn_app();
    let (_person, key, kitchen_id) = person_with_kitchen(&app, "Aurélien");

    let dough = shelve(&app, &key, &kitchen_id, "Pizza dough");
    let (_, read_dough) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": dough }).to_string(),
    );
    let dough_lineage = read_dough["result"]["lineage_id"]
        .as_str()
        .expect("a Lineage")
        .to_string();

    let (pizza, _) = recipe_with(
        &app,
        &key,
        &kitchen_id,
        "Pizza Margherita",
        Some(("2", "pizzas")),
        json!([{ "kind": "ingredient", "text": "Dough for 2 pizzas" }]),
        json!([{ "kind": "step", "text": "Stretch, top and bake." }]),
    );
    make_component(
        &app,
        &key,
        &pizza,
        0,
        Some("500"),
        Some("g"),
        &dough_lineage,
    );
    assert_eq!(components_of(&app, &key, &pizza)[0]["held"], json!(true));

    // Spec item 50, in as many words: delete the dough, hold no copy, and the
    // line still reads and the recipe is still correct.
    let (status, deleted) = delete_recipe(&app, &key, &dough);
    assert_eq!(status, 200, "{deleted}");

    let components = components_of(&app, &key, &pizza);
    assert_eq!(components.len(), 1, "the Component pointer was swept away");
    let carried = &components[0];
    assert_eq!(carried["held"], json!(false));
    assert_eq!(carried["content"], Value::Null);
    assert_eq!(
        carried["said"],
        json!("Kamosu does not have this recipe."),
        "a Component whose recipe left must say what happened: {carried}"
    );

    // And the line it hangs off is untouched. The written words were never the
    // Component (ADR 0008) — the pointer travels beside them — so a recipe
    // whose dough left still tells you it wants dough for two pizzas.
    let (status, read) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": pizza }).to_string(),
    );
    assert_eq!(status, 200, "{read}");
    let version = read["result"]["versions"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    assert_eq!(
        version["content"]["ingredients"][0]["text"],
        json!("Dough for 2 pizzas"),
        "the written line must still read as a sentence"
    );
    assert_eq!(
        version["readings"][0]["lineage_id"],
        json!(dough_lineage),
        "the pointer is kept, so the dough arriving again later needs nothing done"
    );
}
