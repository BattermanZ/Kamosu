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

/// Poll `get_job` through the web door until the Job reaches an end state.
fn wait_terminal(app: &support::TestApp, bearer: Option<&str>, job_id: &str) -> Value {
    let body = json!({ "job_id": job_id }).to_string();
    for _ in 0..400 {
        let (_, body) = app.post_op("get_job", bearer, &body);
        let result = body["result"].clone();
        let status = result["status"].as_str().unwrap_or("");
        if ["completed", "failed", "cancelled"].contains(&status) {
            return result;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("job {job_id} never reached an end state");
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
    let finished = wait_terminal(&app, Some(&key), &job_id);
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
    let secret = logged_in
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("set-cookie"))
        .and_then(|(_, value)| value.split(';').next())
        .and_then(|pair| pair.strip_prefix("kamosu_session="))
        .expect("HttpOnly session cookie");
    assert_eq!(app.post_op("list_jobs", Some(secret), "{}").0, 200);
    let (status, revoked) = app.post_op(
        "revoke_session",
        Some(secret),
        &json!({ "session_id": session_id }).to_string(),
    );
    assert_eq!(status, 200, "{revoked}");
    assert_eq!(app.post_op("list_jobs", Some(secret), "{}").0, 401);
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

/// Push a Branch's current head further into the past, so the next save
/// falls outside the collapse window instead of being read as a rapid
/// re-save. The one place these tests reach past Operations into the Core's
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

    let (status, _) = app.post_op(
        "get_recipe",
        Some(&stranger_key),
        &json!({ "branch_id": branch_id }).to_string(),
    );
    assert_eq!(status, 401);
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
    let copier_kitchen: String = app
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row(
                "SELECT home_kitchen_id FROM people WHERE id = ?1",
                rusqlite::params![copier],
                |row| row.get(0),
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .unwrap();

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
    // not (ADR 0002).
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
            ],
        })
        .to_string(),
    );
    let branch_id = created["result"]["branch_id"].as_str().unwrap().to_string();

    // Freshly created: nothing has been read yet, so every line — including
    // the one with an obvious quantity — carries no Reading. An unread line
    // is not an error; it is a working line.
    assert_eq!(
        created["result"]["versions"][0]["readings"],
        json!([null, null, null, null]),
        "no parser runs yet (ADR 0002); every line starts unread"
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
            "reading": { "amount": "2", "unit": "tbsp", "target": "soy sauce" },
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
            { "amount": "2", "unit": "tbsp", "target": "soy sauce" },
            null,
            null,
        ]),
        "pinch of salt and Za'tar stay unread — no quantity is not an error"
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
    assert_eq!(
        fetched_again["result"]["versions"][0]["readings"],
        json!([null, null, null, null])
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
    assert_eq!(status, 401);
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
            { "amount": "2", "unit": "tbsp", "target": "soy sauce" },
            null,
        ]),
        "the soy sauce Reading survived the save; the rewritten stock line lost its own"
    );
    // The earlier Version keeps exactly what it always had.
    assert_eq!(
        versions[0]["readings"],
        json!([
            { "amount": "2", "unit": "tbsp", "target": "soy sauce" },
            { "amount": "1", "unit": "litre", "target": "stock" },
        ])
    );
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
    assert_eq!(status, 400, "{refused}");

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
    let session_secret = created
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("set-cookie"))
        .and_then(|(_, value)| value.split(';').next())
        .and_then(|pair| pair.strip_prefix("kamosu_session="))
        .expect("HttpOnly session cookie")
        .to_string();

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
    assert!(
        body.contains("--text-step: 33px"),
        "the Step is the largest type (ADR 0011)"
    );
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

    let finished = wait_terminal(&app, None, &job_id);
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

    wait_terminal(&app, Some(&his_key), &job_id);

    // Marie asking about his Job learns nothing: it is not hers to read.
    let get = json!({ "job_id": job_id }).to_string();
    let (status, body) = app.post_op("get_job", Some(&her_key), &get);
    assert_eq!(status, 401, "{body}");

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
    wait_terminal(&app, None, &stranger_id);
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
    let operator_secret = operator
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("set-cookie"))
        .and_then(|(_, value)| value.split(';').next())
        .and_then(|pair| pair.strip_prefix("kamosu_session="))
        .expect("operator Session cookie");

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
    let operator_secret = operator
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("set-cookie"))
        .and_then(|(_, value)| value.split(';').next())
        .and_then(|pair| pair.strip_prefix("kamosu_session="))
        .expect("operator Session cookie");

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
        &json!({ "attempt_id": attempt_id, "note": "Un peu trop cuit", "rating": 4 }).to_string(),
    );
    assert_eq!(status, 200, "{edited}");
    assert_eq!(edited["result"]["note"], json!("Un peu trop cuit"));
    assert_eq!(edited["result"]["rating"], json!(4));

    // A rating outside 1..=5 is refused.
    let (status, bad_rating) = app.post_op(
        "edit_attempt",
        Some(&key),
        &json!({ "attempt_id": attempt_id, "rating": 6 }).to_string(),
    );
    assert_eq!(status, 400, "{bad_rating}");

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

    let (status, _) = app.post_op(
        "advance_attempt",
        Some(&intruder_key),
        &json!({ "attempt_id": attempt_id, "current_step_index": 1 }).to_string(),
    );
    assert_eq!(status, 401);
    let (status, _) = app.post_op(
        "finish_attempt",
        Some(&intruder_key),
        &json!({ "attempt_id": attempt_id }).to_string(),
    );
    assert_eq!(status, 401);
    let (status, _) = app.post_op(
        "edit_attempt",
        Some(&intruder_key),
        &json!({ "attempt_id": attempt_id, "note": "not mine to say" }).to_string(),
    );
    assert_eq!(status, 401);
    let (status, _) = app.post_op(
        "delete_attempt",
        Some(&intruder_key),
        &json!({ "attempt_id": attempt_id }).to_string(),
    );
    assert_eq!(status, 401);
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
    assert_eq!(status, 401, "{refused}");
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
    let kitchen_b: String = app
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row(
                "SELECT home_kitchen_id FROM people WHERE id = ?1",
                rusqlite::params![person],
                |row| row.get(0),
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .unwrap();

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
    assert_eq!(status, 401);
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
    let finished = wait_terminal(app, Some(key), job_id);
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
    let home_kitchen_id = app
        .core
        .db()
        .with_conn(|conn| {
            conn.query_row(
                "SELECT home_kitchen_id FROM people WHERE id = ?1",
                rusqlite::params![person],
                |row| row.get::<_, String>(0),
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))
        })
        .expect("home kitchen");

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
        let finished = wait_terminal(app, Some(key), job_id);
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
        let finished = wait_terminal(&app, Some(&key), job_id);
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
    assert_eq!(status, 401, "{refused}");
    assert_eq!(refused["error"]["kind"], json!("unauthorized"));
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
    let (_, copied) = app.post_op(
        "save_recipe_version",
        Some(&key),
        &json!({
            "branch_id": english,
            "kitchen_id": other,
            "title": "Mousse au chocolat",
            "ingredients": [{ "kind": "ingredient", "text": "200 g de chocolat noir" }],
        })
        .to_string(),
    );
    let french_branch = copied["result"]["branch_id"].as_str().unwrap().to_string();
    app.core
        .db()
        .with_conn(|conn| {
            conn.execute(
                "UPDATE branches SET language = 'fr' WHERE id = ?1",
                rusqlite::params![french_branch],
            )
            .map_err(|e| kamosu::core::OpError::internal(e.to_string()))?;
            Ok(())
        })
        .unwrap();

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
