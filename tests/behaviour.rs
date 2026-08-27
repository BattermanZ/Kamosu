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
    assert_eq!(logged_in.status, 200, "{}", logged_in.body);
    let session_id: Value = serde_json::from_str(&logged_in.body).expect("auth envelope");
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
async fn only_a_kitchen_member_may_touch_its_branches() {
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

    let (status, _) = app.post_op(
        "save_recipe_version",
        Some(&stranger_key),
        &json!({ "branch_id": branch_id, "title": "Soupe froide" }).to_string(),
    );
    assert_eq!(status, 401);
}

// --- Access Keys and Sessions (issue #41) ------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn minting_an_access_key_shows_the_secret_once_afterwards_known_by_name_and_last_use() {
    let app = support::spawn_app();
    let create = json!({ "name": "Aurélien", "password": "the right password", "session_name": "first browser" });
    let created = app.post_auth_response("/auth/first-person", &create.to_string());
    assert_eq!(created.status, 200, "{}", created.body);
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
    assert_eq!(operator.status, 200, "{}", operator.body);
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
    assert_eq!(recovery_session.status, 200, "{}", recovery_session.body);
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
