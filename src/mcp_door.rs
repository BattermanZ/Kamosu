//! The MCP door: built by walking the Catalogue, like the web door.
//!
//! Speaks MCP revision `2026-07-28` only — stateless, no handshake. A legacy
//! `initialize` receives a courteous error naming the version this door speaks.
//!
//! It carries the standard long-running-task extension
//! (`io.modelcontextprotocol/tasks`): asking for an Operation declared `Kind::Job`
//! answers a `CreateTaskResult`, whose taskId is Kamosu's job id, and the client
//! polls `tasks/get` until the work ends. Underneath, that is decoration over
//! ordinary Operations — every poll reads through `get_job`, exactly what the
//! web door serves, so watching slow work is never a feature of one Door alone.

use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::catalogue;
use crate::core::{Core, ErrorKind};
use crate::jobs;
use crate::{MCP_PROTOCOL_VERSION, MCP_TASKS_EXTENSION, web_door};

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
        "tools/list" => Ok(tools_list(&core, &headers)),
        "tools/call" => tools_call(&core, &headers, request.get("params")),
        "tasks/get" => tasks_get(&core, &headers, request.get("params")),
        "tasks/cancel" => tasks_cancel(&core, &headers, request.get("params")),
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

/// The listing itself is not an Operation and carries no permission check, but
/// a read-only Access Key's writes are still absent from it (ADR 0031): the
/// filter walks the Catalogue exactly as `Core::execute` refuses them, so the
/// two can never disagree about which Operations that count as.
fn tools_list(core: &Core, headers: &HeaderMap) -> Value {
    let secret = web_door::bearer_from_headers(headers);
    let read_only = secret
        .as_deref()
        .is_some_and(|secret| core.is_read_only_credential(secret));
    let tools: Vec<Value> = catalogue::OPERATIONS
        .iter()
        .filter(|op| !(read_only && op.write))
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

    let op = catalogue::find(name)
        .ok_or_else(|| json_rpc_error(-32602, format!("no tool named '{name}'")))?;

    // A Job answers through the long-running-task extension, and only to clients
    // that declared it. Refusing before any work is recorded is the point: a
    // client that cannot poll must not be able to cause work it can never see.
    if op.kind == catalogue::Kind::Job && !declares_tasks_capability(&params) {
        return Err(missing_tasks_capability_error());
    }

    let secret = web_door::bearer_from_headers(headers);

    match core.execute(secret.as_deref(), name, arguments) {
        Ok(result) => {
            if op.kind == catalogue::Kind::Job {
                let job_id = result["job_id"]
                    .as_str()
                    .ok_or_else(|| json_rpc_error(-32603, "a Job answered without an id"))?;
                // The row was durably written inside execute(), so the first
                // tasks/get for this taskId already resolves.
                let record = core
                    .job(job_id)
                    .map_err(|e| json_rpc_error(-32603, e.to_sentence()))?
                    .ok_or_else(|| json_rpc_error(-32603, "the Job vanished as it was created"))?;
                Ok(create_task_result(&record))
            } else {
                Ok(json!({
                    "content": [{ "type": "text", "text": serde_json::to_string_pretty(&result).expect("serialisable result") }],
                    "structuredContent": result,
                    "isError": false,
                }))
            }
        }
        Err(err) => Ok(json!({
            "content": [{ "type": "text", "text": err.to_sentence() }],
            "isError": true,
        })),
    }
}

// --- The long-running-task extension (io.modelcontextprotocol/tasks) ---------

/// Whether this request's `_meta` declares the tasks extension capability.
fn declares_tasks_capability(params: &Value) -> bool {
    params
        .pointer("/_meta/io.modelcontextprotocol~1clientCapabilities/extensions")
        .and_then(|extensions| extensions.get(MCP_TASKS_EXTENSION))
        .is_some()
}

fn missing_tasks_capability_error() -> Value {
    // Same envelope as json_rpc_error: the handle() plumbing unwraps ["error"].
    json!({
        "error": {
            "code": -32003,
            "message": "Missing required client capability",
            "data": {
                "requiredCapabilities": {
                    "extensions": {
                        MCP_TASKS_EXTENSION: {},
                    },
                },
            },
        },
    })
}

/// The `CreateTaskResult`: the seed state of the task, sent only after the row
/// behind it exists — so the first `tasks/get` for this taskId resolves.
fn create_task_result(record: &jobs::JobRecord) -> Value {
    let mut result = json!({
        "resultType": "task",
        "taskId": record.id,
        // A just-created Job is working, whatever the lane still has queued.
        "status": "working",
        "createdAt": record.created_at,
        "lastUpdatedAt": record.updated_at,
        "ttlMs": null,
        "pollIntervalMs": jobs::POLL_INTERVAL_MS,
    });
    if let Some(message) = &record.progress_message {
        result["statusMessage"] = json!(message);
    }
    result
}

/// `tasks/get`: poll one task. Reads through the ordinary `get_job` Operation —
/// the same ownership rules, the same truth, at both Doors alike.
fn tasks_get(core: &Core, headers: &HeaderMap, params: Option<&Value>) -> Result<Value, Value> {
    let params = params.cloned().unwrap_or(Value::Null);
    if !declares_tasks_capability(&params) {
        return Err(missing_tasks_capability_error());
    }
    let task_id = params
        .get("taskId")
        .and_then(Value::as_str)
        .ok_or_else(|| json_rpc_error(-32602, "params.taskId is required"))?;

    let job = read_job_for_task(core, headers, task_id)?;
    Ok(detailed_task(&job))
}

/// `tasks/cancel`: signal intent to cancel. Acknowledged either way; honoured
/// while the work still waits in line, cooperative once a worker carries it.
/// Decoration over the ordinary `cancel_job` Operation — the Catalogue owns
/// cancellation, this only translates it.
fn tasks_cancel(core: &Core, headers: &HeaderMap, params: Option<&Value>) -> Result<Value, Value> {
    let params = params.cloned().unwrap_or(Value::Null);
    if !declares_tasks_capability(&params) {
        return Err(missing_tasks_capability_error());
    }
    let task_id = params
        .get("taskId")
        .and_then(Value::as_str)
        .ok_or_else(|| json_rpc_error(-32602, "params.taskId is required"))?;

    let secret = web_door::bearer_from_headers(headers);
    match core.execute(
        secret.as_deref(),
        "cancel_job",
        json!({ "job_id": task_id }),
    ) {
        Ok(_) => Ok(json!({ "resultType": "complete" })),
        Err(err) => match err.kind {
            ErrorKind::BadRequest | ErrorKind::NotFound | ErrorKind::Unauthorized => {
                Err(json_rpc_error(
                    -32602,
                    format!("Failed to cancel task: no such Job '{task_id}'"),
                ))
            }
            _ => Err(json_rpc_error(-32603, err.to_sentence())),
        },
    }
}

/// Resolve one taskId to a readable Job, or say it names nothing — a forbidden
/// Job and an absent one are indistinguishable here, so asking tells you nothing
/// about other People's work.
fn read_job_for_task(core: &Core, headers: &HeaderMap, task_id: &str) -> Result<Value, Value> {
    let secret = web_door::bearer_from_headers(headers);
    match core.execute(secret.as_deref(), "get_job", json!({ "job_id": task_id })) {
        Ok(job) => Ok(job),
        Err(err) => match err.kind {
            ErrorKind::BadRequest | ErrorKind::NotFound | ErrorKind::Unauthorized => {
                Err(json_rpc_error(
                    -32602,
                    format!("Failed to retrieve task: no such Job '{task_id}'"),
                ))
            }
            _ => Err(json_rpc_error(-32603, err.to_sentence())),
        },
    }
}

/// One `tasks/get` response: the full DetailedTask for the current status.
fn detailed_task(job: &Value) -> Value {
    let status = match job["status"].as_str().unwrap_or("") {
        "completed" => "completed",
        "failed" => "failed",
        "cancelled" => "cancelled",
        // Both queued and running are, to a polling client, work in progress.
        _ => "working",
    };

    let mut task = json!({
        "resultType": "complete",
        "taskId": job["id"],
        "status": status,
        "createdAt": job["created_at"],
        "lastUpdatedAt": job["updated_at"],
        "ttlMs": null,
        "pollIntervalMs": jobs::POLL_INTERVAL_MS,
    });
    if let Some(message) = job.pointer("/progress/message").filter(|m| !m.is_null()) {
        task["statusMessage"] = message.clone();
    }
    match status {
        "completed" => {
            // The final result matches the original request's shape: a
            // CallToolResult carrying what the Operation produced.
            task["result"] = call_tool_result(job["result"].clone());
        }
        "failed" => {
            task["error"] = json!({
                "code": job.get("errorCode").and_then(Value::as_i64).unwrap_or(-32603),
                "message": job["error"],
            });
        }
        _ => {}
    }
    task
}

fn call_tool_result(structured: Value) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&structured).unwrap_or_default(),
        }],
        "structuredContent": structured,
        "isError": false,
    })
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
