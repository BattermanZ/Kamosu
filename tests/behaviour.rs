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
        .unwrap();

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
        .expect("key");

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
async fn the_tokens_page_reads_every_value_from_the_real_stylesheet() {
    let app = support::spawn_app();

    let (status, content_type, body) = app.get("/tokens");
    assert_eq!(status, 200, "the tokens page serves without a Credential");
    assert!(content_type.starts_with("text/html"), "{content_type}");

    // Values are parsed out of the embedded stylesheet, not maintained by hand:
    // the page shows the accent's hex exactly as /assets/app.css declares it.
    assert!(body.contains("--color-accent"), "{body:200}");
    let stylesheet = std::str::from_utf8(include_bytes!("../assets/app.css")).expect("utf-8");
    let accent_value = stylesheet
        .split("--color-accent: ")
        .nth(1)
        .and_then(|rest| rest.split(';').next())
        .expect("accent declared");
    assert!(body.contains(accent_value), "page shows {accent_value}");
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
    let key = app.core.mint_access_key(&person, "member", false).unwrap();

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
        .unwrap();
    let marie = app.core.create_person("Marie").expect("person");
    let her_key = app
        .core
        .mint_access_key(&marie, "her agent", false)
        .unwrap();

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
    let key = app.core.mint_access_key(&person, "browser", false).unwrap();

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
