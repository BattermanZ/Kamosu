//! The MCP door: built by walking the Catalogue, like the web door.
//!
//! Speaks MCP revision `2026-07-28` only — stateless, no handshake. A legacy
//! `initialize` receives a courteous error naming the version this door speaks.

use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::catalogue;
use crate::core::Core;
use crate::{MCP_PROTOCOL_VERSION, web_door};

/// Build the MCP door from the Catalogue: one stateless route whose listing is
/// the Catalogue itself.
pub fn router(core: Arc<Core>) -> Router {
    Router::new().route("/mcp", post(handle)).with_state(core)
}

async fn handle(
    State(core): State<Arc<Core>>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Response {
    let request = match body {
        Some(Json(value)) => value,
        None => {
            return json_rpc_error_response(
                None,
                json_rpc_error(-32700, "Parse error: a JSON-RPC request body is required"),
            );
        }
    };

    let id = request.get("id").cloned();
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    let result = match method.as_str() {
        // The handshake is retired in this revision: requests are self-contained.
        "initialize" => Err(courteous_initialize_refusal()),
        "tools/list" => Ok(tools_list()),
        "tools/call" => tools_call(&core, &headers, request.get("params")),
        "" => Err(json_rpc_error(
            -32600,
            "Invalid Request: 'method' is missing",
        )),
        other => Err(json_rpc_error(
            -32601,
            format!("no method '{other}' at the MCP door"),
        )),
    };

    match result {
        Ok(result) => json_rpc_success(id, result),
        Err(err) => json_rpc_error_response(id, err),
    }
}

fn courteous_initialize_refusal() -> Value {
    json_rpc_error(
        -32000,
        format!(
            "Kamosu speaks MCP revision {MCP_PROTOCOL_VERSION} only: stateless, with no \
             handshake. Send each request on its own — no initialize is needed or spoken."
        ),
    )
}

fn tools_list() -> Value {
    let tools: Vec<Value> = catalogue::OPERATIONS
        .iter()
        .map(|op| {
            json!({
                "name": op.name,
                "description": op.summary,
                "inputSchema": op.input_schema,
            })
        })
        .collect();
    json!({ "tools": tools })
}

fn tools_call(core: &Core, headers: &HeaderMap, params: Option<&Value>) -> Result<Value, Value> {
    let params = params.cloned().unwrap_or(Value::Null);
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| json_rpc_error(-32602, "params.name is required to call a tool"))?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

    if catalogue::find(name).is_none() {
        return Err(json_rpc_error(-32602, format!("no tool named '{name}'")));
    }

    let secret = web_door::bearer_from_headers(headers);

    match core.execute(secret.as_deref(), name, arguments) {
        Ok(result) => Ok(json!({
            "content": [{ "type": "text", "text": serde_json::to_string_pretty(&result).expect("serialisable result") }],
            "structuredContent": result,
            "isError": false,
        })),
        Err(err) => Ok(json!({
            "content": [{ "type": "text", "text": err.to_sentence() }],
            "isError": true,
        })),
    }
}

// --- JSON-RPC plumbing -------------------------------------------------------

fn json_rpc_error(code: i64, message: impl Into<String>) -> Value {
    json!({
        "error": { "code": code, "message": message.into() },
    })
}

fn json_rpc_success(id: Option<Value>, payload: Value) -> Response {
    let envelope = json!({ "jsonrpc": "2.0", "id": id.unwrap_or(Value::Null), "result": payload });
    send(envelope)
}

fn json_rpc_error_response(id: Option<Value>, payload: Value) -> Response {
    let envelope = json!({ "jsonrpc": "2.0", "id": id.unwrap_or(Value::Null), "error": payload["error"].clone() });
    send(envelope)
}

fn send(envelope: Value) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        "application/json".parse().unwrap(),
    );
    headers.insert(
        "MCP-Protocol-Version",
        MCP_PROTOCOL_VERSION.parse().unwrap(),
    );
    (StatusCode::OK, headers, Json(envelope)).into_response()
}
