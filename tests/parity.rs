//! The parity check: walks the Catalogue and asserts both Doors materialise every
//! Operation. This is ADR 0001's guarantee made visible in one place — hand-writing
//! a route, or dropping an Operation from a Door's walk, breaks the build.

mod support;

use kamosu::catalogue;
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_web_door_materialises_every_operation_in_the_catalogue() {
    let app = support::spawn_app();

    for op in catalogue::OPERATIONS.iter() {
        let (status, body) = app.post_op(op.name, None, "{}");
        assert_eq!(
            status, 200,
            "web door does not materialise Operation '{}' ({body})",
            op.name
        );
        assert_eq!(
            body["ok"],
            json!(true),
            "Operation '{}' failed at the web door",
            op.name
        );
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

    // And each one answers when called.
    for name in expected {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": name, "arguments": {} },
        });
        let (status, body) = app.post_mcp(&payload.to_string(), None);
        assert_eq!(status, 200, "tool '{name}' did not answer");
        assert_eq!(
            body["error"],
            serde_json::Value::Null,
            "tool '{name}' errored at the MCP door"
        );
        assert_eq!(
            body["result"]["isError"],
            json!(false),
            "tool '{name}' reported isError"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_route_exists_outside_the_catalogue() {
    let app = support::spawn_app();

    let (status, _) = app.post_op("create_recipe", None, "{}");
    assert_eq!(status, 404, "an Operation outside the Catalogue answered");

    let (status, _) = app.post_op("../etc/passwd", None, "{}");
    assert_eq!(status, 404, "a non-Operation path answered");
}
