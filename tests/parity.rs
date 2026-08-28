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
        "set_reading_preferences" => json!({ "reading_language": "en", "reading_measures": "us" }),
        "rename_person" => json!({ "name": "Parity" }),
        "revoke_session" => json!({ "session_id": "s_parity" }),
        "mint_access_key" => json!({ "name": "parity key" }),
        "revoke_access_key" => json!({ "access_key_id": "ak_parity" }),
        "create_kitchen" => json!({ "name": "Parity Kitchen" }),
        "rename_kitchen" => json!({ "kitchen_id": "k_parity", "name": "Parity Kitchen" }),
        "set_kitchen_nickname" => json!({ "kitchen_id": "k_parity", "nickname": "Parity" }),
        "invite_to_kitchen" => json!({ "kitchen_id": "k_parity" }),
        "accept_kitchen_invite" => json!({ "secret": "parity-no-such-invite" }),
        "remove_kitchen_member" => json!({ "kitchen_id": "k_parity", "person_id": "p_parity" }),
        "delete_kitchen" => json!({ "kitchen_id": "k_parity" }),
        "disable_account" | "delete_account" | "mint_recovery_link" => json!({ "name": "Parity" }),
        "create_recipe" => json!({ "kitchen_id": "k_parity", "title": "Parity Recipe" }),
        "save_recipe_version" => json!({ "branch_id": "b_parity", "title": "Parity Recipe" }),
        "rename_version" => json!({ "branch_id": "b_parity", "sequence": 1, "name": "Parity" }),
        "get_recipe" => json!({ "branch_id": "b_parity" }),
        "set_reading" => json!({ "branch_id": "b_parity", "line_index": 0 }),
        "create_tag" => json!({ "kitchen_id": "k_parity", "language": "en", "name": "parity" }),
        "list_tags" => json!({ "kitchen_id": "k_parity" }),
        "rename_tag" => json!({ "tag_id": "t_parity", "language": "en", "name": "parity" }),
        "delete_tag" => json!({ "tag_id": "t_parity" }),
        "merge_tags" => {
            json!({ "keep_tag_id": "t_parity_keep", "merge_tag_id": "t_parity_merge" })
        }
        "set_recipe_tag" => {
            json!({ "branch_id": "b_parity", "tag_id": "t_parity", "carried": true })
        }
        "set_related_recipe" => {
            json!({ "branch_id": "b_parity", "related_branch_id": "b_related_parity", "related": true })
        }
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
            "list_jobs"
            | "list_sessions"
            | "set_reading_preferences"
            | "rename_person"
            | "revoke_session"
            | "mint_access_key"
            | "list_access_keys"
            | "revoke_access_key"
            | "create_kitchen"
            | "list_kitchens"
            | "rename_kitchen"
            | "set_kitchen_nickname"
            | "invite_to_kitchen"
            | "accept_kitchen_invite"
            | "remove_kitchen_member"
            | "delete_kitchen"
            | "mint_invite"
            | "disable_account"
            | "delete_account"
            | "mint_recovery_link"
            | "create_recipe"
            | "save_recipe_version"
            | "rename_version"
            | "get_recipe"
            | "set_reading"
            | "create_tag"
            | "list_tags"
            | "rename_tag"
            | "delete_tag"
            | "merge_tags"
            | "set_recipe_tag"
            | "set_related_recipe"
            | "upload_photograph" => ("{}", 401),
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
        } else if matches!(
            name,
            "get_job"
                | "list_jobs"
                | "list_sessions"
                | "cancel_job"
                | "set_reading_preferences"
                | "rename_person"
                | "revoke_session"
                | "mint_access_key"
                | "list_access_keys"
                | "revoke_access_key"
                | "create_kitchen"
                | "list_kitchens"
                | "rename_kitchen"
                | "set_kitchen_nickname"
                | "invite_to_kitchen"
                | "accept_kitchen_invite"
                | "remove_kitchen_member"
                | "delete_kitchen"
                | "mint_invite"
                | "disable_account"
                | "delete_account"
                | "mint_recovery_link"
                | "create_recipe"
                | "save_recipe_version"
                | "rename_version"
                | "get_recipe"
                | "set_reading"
                | "create_tag"
                | "list_tags"
                | "rename_tag"
                | "delete_tag"
                | "merge_tags"
                | "set_recipe_tag"
                | "set_related_recipe"
                | "upload_photograph"
        ) {
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
async fn a_read_only_access_keys_mcp_tool_list_carries_exactly_the_reads() {
    let app = support::spawn_app();
    let person = app.core.create_person("Aurélien").expect("person");
    let read_only_key = app
        .core
        .mint_access_key(&person, "read-only agent", true)
        .unwrap()
        .secret;
    let full_key = app
        .core
        .mint_access_key(&person, "full agent", false)
        .unwrap()
        .secret;

    let (_, listing) = app.post_mcp(
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
        Some(&read_only_key),
    );
    let mut served: Vec<&str> = listing["result"]["tools"]
        .as_array()
        .expect("tools list")
        .iter()
        .map(|t| t["name"].as_str().expect("tool name"))
        .collect();
    served.sort_unstable();

    let mut expected_reads: Vec<&str> = catalogue::OPERATIONS
        .iter()
        .filter(|op| !op.write)
        .map(|op| op.name)
        .collect();
    expected_reads.sort_unstable();

    assert_eq!(
        served, expected_reads,
        "a read-only Access Key's MCP tool list must be exactly the Catalogue's \
         non-writing Operations — nothing more, nothing less"
    );

    // A full-power Key's list is untouched: every Operation, writes included.
    let (_, full_listing) = app.post_mcp(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        Some(&full_key),
    );
    let mut full_served: Vec<&str> = full_listing["result"]["tools"]
        .as_array()
        .expect("tools list")
        .iter()
        .map(|t| t["name"].as_str().expect("tool name"))
        .collect();
    full_served.sort_unstable();
    let mut every_operation: Vec<&str> = catalogue::OPERATIONS.iter().map(|op| op.name).collect();
    every_operation.sort_unstable();
    assert_eq!(full_served, every_operation);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_route_exists_outside_the_catalogue() {
    let app = support::spawn_app();

    // Also documents ADR 0004: there is no Operation that merges two
    // Lineages, in v1 or ever.
    let (status, _) = app.post_op("merge_lineages", None, "{}");
    assert_eq!(status, 404, "an Operation outside the Catalogue answered");

    let (status, _) = app.post_op("../etc/passwd", None, "{}");
    assert_eq!(status, 404, "a non-Operation path answered");
}
