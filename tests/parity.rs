//! The parity check: walks the Catalogue and asserts both Doors materialise every
//! Operation. This is ADR 0001's guarantee made visible in one place — hand-writing
//! a route, or dropping an Operation from a Door's walk, breaks the build.

mod support;

use kamosu::catalogue;
use serde_json::{Value, json};

/// Per-request capability declaration for the long-running-task extension, so
/// Job Operations answer CreateTaskResult rather than refusing with -32003.
fn tasks_capability() -> Value {
    // The identifiers are fixed by the extension spec; MCP_TASKS_EXTENSION
    // carries the same string.
    assert_eq!(kamosu::MCP_TASKS_EXTENSION, "io.modelcontextprotocol/tasks");
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

/// Arguments that satisfy each Operation's declared input. Most take `{}`;
/// the ones needing more say so here, and their expectation changes with them.
fn arguments_for(name: &str) -> Value {
    match name {
        // A Job id that names nothing is the honest probe for routing.
        "get_job" | "cancel_job" => json!({ "job_id": "parity-no-such-job" }),
        _ => json!({}),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_web_door_materialises_every_operation_in_the_catalogue() {
    let app = support::spawn_app();

    for op in catalogue::OPERATIONS.iter() {
        let (body_text, expected_status) = match op.name {
            "get_job" | "cancel_job" => (r#"{"job_id":"parity-no-such-job"}"#, 404),
            // Person-only Operations answer the stranger with a refusal.
            "list_jobs" => ("{}", 401),
            _ => ("{}", 200),
        };
        let (status, body) = app.post_op(op.name, None, body_text);
        assert_eq!(
            status, expected_status,
            "web door does not materialise Operation '{}' ({body})",
            op.name
        );
        if expected_status == 200 {
            assert_eq!(
                body["ok"],
                json!(true),
                "Operation '{}' failed at the web door",
                op.name
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_mcp_door_materialises_every_operation_in_the_catalogue() {
    let app = support::spawn_app();

    let (_, listing) = app.post_mcp(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#, None);
    let tools = listing["result"]["tools"].as_array().expect("tools list");

    let mut served: Vec<&str> = tools
        .iter()
        .map(|t| t["name"].as_str().expect("tool name"))
        .collect();
    served.sort_unstable();

    let mut expected: Vec<&str> = catalogue::OPERATIONS.iter().map(|op| op.name).collect();
    expected.sort_unstable();

    assert_eq!(
        served, expected,
        "the MCP door's tool list is not exactly the Catalogue"
    );

    // And each one answers when called. Job Operations answer a CreateTaskResult,
    // which is followed through tasks/get to its end before being called good.
    for name in expected {
        let arguments = arguments_for(name);
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": name,
                "arguments": arguments,
                "_meta": tasks_capability()["_meta"],
            },
        });
        let (status, body) = app.post_mcp(&payload.to_string(), None);
        assert_eq!(status, 200, "tool '{name}' did not answer");
        assert_eq!(
            body["error"],
            serde_json::Value::Null,
            "tool '{name}' errored at the MCP door"
        );
        let result = &body["result"];
        if result["resultType"] == "task" {
            follow_task(&app, name, result["taskId"].clone());
        } else if matches!(name, "get_job" | "list_jobs" | "cancel_job") {
            // Asked without what they need — a real id or a Credential — they
            // refuse as errors rather than pretending success.
            assert_eq!(
                result["isError"],
                json!(true),
                "'{name}' should refuse this call"
            );
        } else {
            assert_eq!(
                result["isError"],
                json!(false),
                "tool '{name}' reported isError"
            );
        }
    }
}

/// Poll one task to its end and require it completed — the generic proof that
/// every Job Operation in the Catalogue is readable back through tasks/get.
fn follow_task(app: &support::TestApp, name: &str, task_id: serde_json::Value) {
    for _ in 0..200 {
        let poll = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tasks/get",
            "params": {
                "taskId": task_id,
                "_meta": tasks_capability()["_meta"],
            },
        });
        let (_, body) = app.post_mcp(&poll.to_string(), None);
        let task = &body["result"];
        if task["status"] == "completed"
            || task["status"] == "failed"
            || task["status"] == "cancelled"
        {
            assert_eq!(
                task["status"],
                json!("completed"),
                "task for tool '{name}' ended other than completed"
            );
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("task for tool '{name}' never reached an end state");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_route_exists_outside_the_catalogue() {
    let app = support::spawn_app();

    let (status, _) = app.post_op("create_recipe", None, "{}");
    assert_eq!(status, 404, "an Operation outside the Catalogue answered");

    let (status, _) = app.post_op("../etc/passwd", None, "{}");
    assert_eq!(status, 404, "a non-Operation path answered");
}
