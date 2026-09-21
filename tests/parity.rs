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
        "read_pasted_recipe" => json!({ "text": "Parity Recipe\n\n1 tsp salt\nStir it in." }),
        "start_translation" => {
            json!({ "branch_id": "b_parity", "language": "fr", "title": "Recette Parité" })
        }
        "set_recipe_language" => json!({ "branch_id": "b_parity", "language": "fr" }),
        "rename_version" => json!({ "branch_id": "b_parity", "sequence": 1, "name": "Parity" }),
        "get_recipe" => json!({ "branch_id": "b_parity" }),
        "export_bundle" | "make_sheet" => json!({ "branch_id": "b_parity" }),
        "import_bundle" => json!({ "data": "" }),
        "import_crouton" => json!({ "upload_id": "u_parity" }),
        "forget_import" => json!({ "import_id": "i_parity" }),
        "set_reading" => json!({ "branch_id": "b_parity", "line_index": 0 }),
        "get_food" => json!({ "food_id": "f_parity" }),
        "set_food_name" => json!({ "food_id": "f_parity", "language": "en", "name": "parity" }),
        "remove_food_name" => json!({ "food_id": "f_parity", "language": "en" }),
        "set_food_cup_weight" => json!({ "food_id": "f_parity", "cup_weight_grams": 100 }),
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
        "share_recipe" | "end_share_link" | "get_share_link" => {
            json!({ "branch_id": "b_parity" })
        }
        "set_public_address" => json!({ "public_address": "https://parity.example" }),
        // A token nobody minted is the honest probe: the Operation is Public,
        // so what it must demonstrate is that it routes and refuses on the
        // token alone rather than on a Credential.
        "read_shared_recipe" => json!({ "token": "parity-no-such-token" }),
        // Public too, and a Job: asking is accepted on the token alone, and it
        // is the Job that then finds the token opens nothing.
        "make_shared_sheet" => json!({ "token": "parity-no-such-token" }),
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
            | "get_reading_preferences"
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
            | "sweep_photographs"
            | "take_backup"
            | "list_backups"
            | "create_recipe"
            | "save_recipe_version"
            | "read_pasted_recipe"
            | "start_translation"
            | "set_recipe_language"
            | "import"
            | "import_web_link"
            | "import_bundle"
            | "import_crouton"
            | "forget_import"
            | "rename_version"
            | "search_recipes"
            | "home_shelves"
            | "note_recipe_opened"
            | "get_recipe"
            | "export_bundle"
            | "make_sheet"
            | "get_thread"
            | "branch_point"
            | "divergence"
            | "set_reading"
            | "read_ingredient_lines"
            | "create_tag"
            | "list_tags"
            | "rename_tag"
            | "delete_tag"
            | "merge_tags"
            | "set_recipe_tag"
            | "set_related_recipe"
            | "upload_photograph"
            | "list_foods"
            | "get_food"
            | "set_food_name"
            | "remove_food_name"
            | "set_food_cup_weight"
            | "list_merge_suggestions"
            | "preview_food_merge"
            | "merge_food"
            | "delete_food"
            | "start_attempt"
            | "advance_attempt"
            | "finish_attempt"
            | "edit_attempt"
            | "delete_attempt"
            | "list_attempts"
            | "get_current_attempt"
            | "promote_attempt_photograph"
            | "set_as_cooked"
            | "promote_as_cooked"
            | "decline_promotion"
            | "meaning_search_status"
            | "accept_meaning_search_terms"
            | "decline_meaning_search"
            | "download_meaning_model"
            | "build_meaning_index"
            | "turn_off_meaning_search"
            | "share_recipe"
            | "end_share_link"
            | "get_share_link"
            | "set_public_address"
            | "get_shopping_list"
            | "shopping_basis"
            | "add_to_shopping_list"
            | "remove_from_shopping_list"
            | "set_shopping_yield"
            | "shopping_list_as_text"
            | "empty_shopping_list"
            | "add_loose_item"
            | "remove_loose_item" => ("{}", 401),
            // Public, and answered on the token alone: a token nobody minted
            // is not found, which is the routing this check is after.
            "read_shared_recipe" => (r#"{"token":"parity-no-such-token"}"#, 404),
            "make_shared_sheet" => (r#"{"token":"parity-no-such-token"}"#, 200),
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
            // A Sheet asked for with a token nobody minted is accepted, and then
            // fails on the token — that failure is the routing being proved.
            let ends = if name == "make_shared_sheet" {
                "failed"
            } else {
                "completed"
            };
            follow_task(&app, name, result["taskId"].clone(), ends);
        } else if matches!(
            name,
            "get_job"
                | "list_jobs"
                | "list_sessions"
                | "cancel_job"
                | "set_reading_preferences"
                | "get_reading_preferences"
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
                | "sweep_photographs"
                | "take_backup"
                | "list_backups"
                | "create_recipe"
                | "save_recipe_version"
                | "read_pasted_recipe"
                | "start_translation"
                | "set_recipe_language"
                | "import"
                | "import_web_link"
                | "import_bundle"
                | "import_crouton"
                | "forget_import"
                | "rename_version"
                | "search_recipes"
                | "home_shelves"
                | "note_recipe_opened"
                | "get_recipe"
                | "export_bundle"
                | "make_sheet"
                | "get_thread"
                | "branch_point"
                | "divergence"
                | "set_reading"
                | "read_ingredient_lines"
                | "create_tag"
                | "list_tags"
                | "rename_tag"
                | "delete_tag"
                | "merge_tags"
                | "set_recipe_tag"
                | "set_related_recipe"
                | "upload_photograph"
                | "list_foods"
                | "get_food"
                | "set_food_name"
                | "remove_food_name"
                | "set_food_cup_weight"
                | "list_merge_suggestions"
                | "preview_food_merge"
                | "merge_food"
                | "delete_food"
                | "start_attempt"
                | "advance_attempt"
                | "finish_attempt"
                | "edit_attempt"
                | "delete_attempt"
                | "list_attempts"
                | "get_current_attempt"
                | "promote_attempt_photograph"
                | "set_as_cooked"
                | "promote_as_cooked"
                | "decline_promotion"
                | "meaning_search_status"
                | "accept_meaning_search_terms"
                | "decline_meaning_search"
                | "download_meaning_model"
                | "build_meaning_index"
                | "turn_off_meaning_search"
                | "share_recipe"
                | "end_share_link"
                | "get_share_link"
                | "set_public_address"
                | "read_shared_recipe"
                | "get_shopping_list"
                | "shopping_basis"
                | "add_to_shopping_list"
                | "remove_from_shopping_list"
                | "set_shopping_yield"
                | "shopping_list_as_text"
                | "empty_shopping_list"
                | "add_loose_item"
                | "remove_loose_item"
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

/// Poll one task to its end and require the end expected — the generic proof
/// that every Job Operation in the Catalogue is readable back through tasks/get.
fn follow_task(app: &support::TestApp, name: &str, task_id: serde_json::Value, ends: &str) {
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
                json!(ends),
                "task for tool '{name}' ended other than {ends}"
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

/// Every Operation's declared input compiles into the thing that checks it (#85).
///
/// This is `src/schema.rs`'s guarantee held where `tests/parity.rs` holds ADR
/// 0001's: a declaration the validator cannot compile — a misspelt keyword, an
/// unknown type name, an `additionalProperties` that is not `false`, a
/// `required` naming a field nobody declared — breaks the build here rather
/// than quietly becoming a check that no longer runs.
///
/// It is deliberately not enough to let the Core panic at startup. A JSON
/// Schema validator is *required by its own specification* to ignore a keyword
/// it does not recognise, so the failure this guards against is silent
/// everywhere else: `"requird": ["branch_id"]` would validate nothing and say
/// nothing.
#[test]
fn every_declared_input_compiles_into_its_own_check() {
    for op in catalogue::OPERATIONS.iter() {
        if let Err(reason) = kamosu::schema::compile(&op.input_schema, "") {
            panic!("{}'s declared input does not compile: {reason}", op.name);
        }
    }
}

/// The Catalogue's declared input is what refuses a bad call, at **both** Doors.
///
/// The web Door answers a `bad_request`; the MCP Door answers a tool error. The
/// same undeclared field, refused the same way at each, is the whole of the
/// parity claim — neither Door checks anything itself.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_undeclared_field_is_refused_at_both_doors() {
    let app = support::spawn_app();

    // `instance_status` is Public, so this needs no Credential and the refusal
    // cannot be mistaken for an authorisation answer.
    let (status, body) = app.post_op("instance_status", None, r#"{"unexpected":1}"#);
    assert_eq!(
        status, 400,
        "the web Door accepted an undeclared field: {body}"
    );
    assert_eq!(body["error"]["kind"], "bad_request");
    assert!(
        body["error"]["message"]
            .as_str()
            .expect("a message")
            .contains("unexpected"),
        "the refusal does not name the offending field: {body}"
    );

    let call = |arguments: Value| {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": "instance_status", "arguments": arguments },
        });
        app.post_mcp(&payload.to_string(), None).1
    };

    let refused = call(json!({ "unexpected": 1 }));
    assert_eq!(
        refused["result"]["isError"],
        json!(true),
        "the MCP Door accepted an undeclared field: {refused}"
    );
    assert!(
        refused["result"]["content"][0]["text"]
            .as_str()
            .expect("a message")
            .contains("unexpected"),
        "the MCP refusal does not name the offending field: {refused}"
    );

    // Two more shape failures, refused the same way at the same two Doors.
    // `read_shared_recipe` is Public and takes one required string, so this
    // reaches the check without a Credential — validation runs *after*
    // authorisation, so an Operation needing one would answer 401 first.
    for (what, arguments) in [
        ("a missing required field", json!({})),
        ("a wrong type", json!({ "token": 7 })),
    ] {
        let (status, body) = app.post_op("read_shared_recipe", None, &arguments.to_string());
        assert_eq!(status, 400, "the web Door accepted {what}: {body}");
        assert_eq!(body["error"]["kind"], "bad_request", "for {what}");

        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": "read_shared_recipe", "arguments": arguments },
        });
        let refused = app.post_mcp(&payload.to_string(), None).1;
        assert_eq!(
            refused["result"]["isError"],
            json!(true),
            "the MCP Door accepted {what}: {refused}"
        );
    }

    // And the same Operation, called as declared, still answers at both Doors.
    let (status, body) = app.post_op("instance_status", None, "{}");
    assert_eq!(status, 200, "a valid call was refused: {body}");
    let accepted = call(json!({}));
    assert_eq!(
        accepted["result"]["isError"],
        json!(false),
        "a valid MCP call was refused: {accepted}"
    );
}
