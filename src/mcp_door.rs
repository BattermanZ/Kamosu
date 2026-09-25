//! The MCP door: built by walking the Catalogue, like the web door.
//!
//! Speaks MCP revision `2026-07-28` only — stateless, no handshake. A legacy
//! `initialize` receives a courteous error naming the version this door speaks,
//! and `server/discover`, which that revision requires of every server, says
//! the same thing to a modern client before it asks for anything else (#143).
//!
//! It carries the standard long-running-task extension
//! (`io.modelcontextprotocol/tasks`): asking for an Operation declared `Kind::Job`
//! answers a `CreateTaskResult`, whose taskId is Kamosu's job id, and the client
//! polls `tasks/get` until the work ends. Underneath, that is decoration over
//! ordinary Operations — every poll reads through `get_job`, exactly what the
//! web door serves, so watching slow work is never a feature of one Door alone.
//!
//! It answers no web page: a request carrying an `Origin` header is refused
//! before anything else about it is read (#145).

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
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
    Router::new()
        .route(
            "/mcp",
            post(handle).layer(middleware::from_fn(refuse_any_origin)),
        )
        .with_state(core)
}

/// The revision requires every server to validate `Origin` against DNS
/// rebinding, and the set this door accepts is empty. Nothing Kamosu serves
/// calls `/mcp`, and a rebound page's `Origin` always matches the request's
/// own `Host`, so no value is worth trusting. A layer rather than a check in
/// `handle`, because "all incoming connections" includes a body the handler's
/// extractor would refuse first. Transport validation, not authorisation: no
/// Credential is read, so no permission check lives here.
async fn refuse_any_origin(request: Request, next: Next) -> Response {
    if !request.headers().contains_key(axum::http::header::ORIGIN) {
        return next.run(request).await;
    }
    // Nothing of the request was read, so the error names no id.
    send_error(
        StatusCode::FORBIDDEN,
        None,
        json_rpc_error(
            -32600,
            "Invalid Request: this endpoint accepts no Origin header, since it serves no web page",
        ),
    )
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

    // A notification carries no id and wants no answer. None changes anything
    // at this door, so each is accepted and nothing is done.
    if id.is_none() && !method.is_empty() {
        return StatusCode::ACCEPTED.into_response();
    }
    // MCP narrows JSON-RPC here: a request's id is never null.
    if id.as_ref().is_some_and(Value::is_null) {
        return json_rpc_error_response(
            None,
            json_rpc_error(-32600, "Invalid Request: 'id' may not be null"),
        );
    }

    // The handshake is retired in this revision: requests are self-contained.
    // Answered before the metadata is read, since a legacy client sends none.
    if method == "initialize" {
        return json_rpc_error_response(id, courteous_initialize_refusal(request.get("params")));
    }
    if let Err(fault) = check_request_metadata(&headers, &request, &method) {
        return json_rpc_error_response(id, fault);
    }

    let result = match method.as_str() {
        // Needs no Credential: nothing it says depends on who is asking.
        "server/discover" => Ok(server_discover()),
        "tools/list" => Ok(tools_list(&core, &headers)),
        "tools/call" => tools_call(&core, &headers, request.get("params")),
        "tasks/get" => tasks_get(&core, &headers, request.get("params")),
        "tasks/update" => tasks_update(&core, &headers, request.get("params")),
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

/// An `initialize` is how a legacy client asks for its own revision, so it is
/// refused as an unsupported version: the one refusal a legacy client can show
/// its person, and one a modern client recognises and does not fall back from.
fn courteous_initialize_refusal(params: Option<&Value>) -> Value {
    let requested = params
        .and_then(|params| params.get("protocolVersion"))
        .and_then(Value::as_str)
        .unwrap_or("");
    unsupported_version(
        requested,
        format!(
            "Kamosu speaks MCP revision {MCP_PROTOCOL_VERSION} only: stateless, with no \
             handshake. Send each request on its own — no initialize is needed or spoken."
        ),
    )
}

fn unsupported_version(requested: &str, message: impl Into<String>) -> Value {
    json_rpc_error_with_data(
        UNSUPPORTED_PROTOCOL_VERSION,
        message,
        json!({ "supported": [MCP_PROTOCOL_VERSION], "requested": requested }),
    )
}

/// The revision a request names, in its `_meta` or its header, must be this
/// one, and every header mirroring the body must agree with it.
///
/// Only what is present is checked. The revision also makes the version, the
/// `Mcp-Method` and `Mcp-Name` headers and the `_meta` fields required, but
/// every caller written before #143 sends none of them, and refusing their
/// absence is its own decision.
fn check_request_metadata(headers: &HeaderMap, request: &Value, method: &str) -> Result<(), Value> {
    let header = |name: &str| -> Result<Option<&str>, Value> {
        headers
            .get(name)
            .map(|value| {
                value
                    .to_str()
                    .map_err(|_| header_mismatch(format!("the {name} header is not plain text")))
            })
            .transpose()
    };
    let params = request.get("params");

    let body_version = params
        .and_then(|params| params.pointer("/_meta/io.modelcontextprotocol~1protocolVersion"))
        .and_then(Value::as_str);
    let header_version = header("MCP-Protocol-Version")?;
    if let (Some(body), Some(header)) = (body_version, header_version)
        && body != header
    {
        return Err(header_mismatch(format!(
            "MCP-Protocol-Version header value '{header}' does not match body value '{body}'"
        )));
    }
    if let Some(requested) = body_version.or(header_version)
        && requested != MCP_PROTOCOL_VERSION
    {
        return Err(unsupported_version(
            requested,
            "Unsupported protocol version",
        ));
    }

    if let Some(named) = header("Mcp-Method")?
        && named != method
    {
        return Err(header_mismatch(format!(
            "Mcp-Method header value '{named}' does not match body value '{method}'"
        )));
    }

    if let Some(named) = header("Mcp-Name")? {
        let named = decode_header_value(named)
            .ok_or_else(|| header_mismatch("the Mcp-Name header's Base64 does not decode"))?;
        // A tool's name, a resource's uri, or a task's id (the tasks extension).
        let body = params.and_then(|params| {
            ["name", "uri", "taskId"]
                .iter()
                .find_map(|field| params.get(field).and_then(Value::as_str))
        });
        if body != Some(named.as_str()) {
            return Err(header_mismatch(format!(
                "Mcp-Name header value '{named}' does not match body value '{}'",
                body.unwrap_or("")
            )));
        }
    }
    Ok(())
}

/// A header value as sent, or decoded out of the `=?base64?…?=` sentinel a
/// client uses for anything that is not plain ASCII.
fn decode_header_value(value: &str) -> Option<String> {
    use base64::Engine;
    match value
        .strip_prefix("=?base64?")
        .and_then(|rest| rest.strip_suffix("?="))
    {
        Some(encoded) => base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok()),
        None => Some(value.to_string()),
    }
}

fn header_mismatch(message: impl Into<String>) -> Value {
    json_rpc_error(
        HEADER_MISMATCH,
        format!("Header mismatch: {}", message.into()),
    )
}

/// How long a client may keep the discovery answer and the tool listing before
/// asking again. Both change only when a new build ships, so five minutes costs
/// an upgrade nothing a person would notice.
const LISTING_TTL_MS: u64 = 5 * 60 * 1000;

/// `server/discover`: what this door serves, read off the Catalogue it walks
/// rather than kept by hand. The tasks extension is claimed only because some
/// Operation is a Job; without one the door would never answer with a task.
fn server_discover() -> Value {
    let mut capabilities = json!({ "tools": {} });
    if catalogue::OPERATIONS
        .iter()
        .any(|op| op.kind == catalogue::Kind::Job)
    {
        capabilities["extensions"] = json!({ MCP_TASKS_EXTENSION: {} });
    }
    json!({
        "resultType": "complete",
        "supportedVersions": [MCP_PROTOCOL_VERSION],
        "capabilities": capabilities,
        "ttlMs": LISTING_TTL_MS,
        // The same answer for every caller, Credential or none.
        "cacheScope": "public",
    })
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
    json!({
        "resultType": "complete",
        "tools": tools,
        "ttlMs": LISTING_TTL_MS,
        // What a read-only Access Key is shown differs from the rest.
        "cacheScope": "private",
    })
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
                Ok(call_tool_result(result))
            }
        }
        Err(err) => Ok(json!({
            "resultType": "complete",
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
    json_rpc_error_with_data(
        MISSING_REQUIRED_CLIENT_CAPABILITY,
        "Missing required client capability",
        json!({ "requiredCapabilities": { "extensions": { MCP_TASKS_EXTENSION: {} } } }),
    )
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

/// `tasks/update`: the client answering what a task asked of it. No Operation
/// asks anything mid-work, so there is never an answer to apply, but the
/// extension still has a known task acknowledged and an unknown one refused.
fn tasks_update(core: &Core, headers: &HeaderMap, params: Option<&Value>) -> Result<Value, Value> {
    let params = params.cloned().unwrap_or(Value::Null);
    if !declares_tasks_capability(&params) {
        return Err(missing_tasks_capability_error());
    }
    let task_id = params
        .get("taskId")
        .and_then(Value::as_str)
        .ok_or_else(|| json_rpc_error(-32602, "params.taskId is required"))?;

    read_job_for_task(core, headers, task_id)?;
    Ok(json!({ "resultType": "complete" }))
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
            // A Job that is not the caller's arrives as a not-found from the
            // Core (ADR 0040), so this Door does not collapse it. An
            // `Unauthorized` reaching here is about the Credential itself and
            // says so, rather than being dressed up as a missing task.
            ErrorKind::BadRequest | ErrorKind::NotFound => Err(json_rpc_error(
                -32602,
                format!("Failed to cancel task: no such Job '{task_id}'"),
            )),
            _ => Err(json_rpc_error(-32603, err.to_sentence())),
        },
    }
}

/// Resolve one taskId to a readable Job, or say it names nothing.
///
/// A forbidden Job and an absent one arrive here as the same refusal already:
/// the Core collapses them (ADR 0040), so this Door no longer does. It used to,
/// and that is exactly the bug ADR 0001 warns about — the collapse lived in one
/// Door, so `get_job` at the web Door never inherited it. An `Unauthorized`
/// reaching this point is now about the Credential itself, and is passed on
/// saying so rather than dressed up as a missing task.
fn read_job_for_task(core: &Core, headers: &HeaderMap, task_id: &str) -> Result<Value, Value> {
    let secret = web_door::bearer_from_headers(headers);
    match core.execute(secret.as_deref(), "get_job", json!({ "job_id": task_id })) {
        Ok(job) => Ok(job),
        Err(err) => match err.kind {
            ErrorKind::BadRequest | ErrorKind::NotFound => Err(json_rpc_error(
                -32602,
                format!("Failed to retrieve task: no such Job '{task_id}'"),
            )),
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
        "resultType": "complete",
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&structured).unwrap_or_default(),
        }],
        "structuredContent": structured,
        "isError": false,
    })
}

// --- JSON-RPC plumbing -------------------------------------------------------

/// The three codes the revision allocates itself. Each is answered 400, which
/// is how a client knows it reached a modern server that refused the request,
/// rather than a legacy one that did not understand it.
const HEADER_MISMATCH: i64 = -32020;
const MISSING_REQUIRED_CLIENT_CAPABILITY: i64 = -32021;
const UNSUPPORTED_PROTOCOL_VERSION: i64 = -32022;

fn json_rpc_error(code: i64, message: impl Into<String>) -> Value {
    json!({
        "error": { "code": code, "message": message.into() },
    })
}

fn json_rpc_error_with_data(code: i64, message: impl Into<String>, data: Value) -> Value {
    let mut error = json_rpc_error(code, message);
    error["error"]["data"] = data;
    error
}

fn json_rpc_success(id: Option<Value>, mut payload: Value) -> Response {
    // Every result names the server that gave it, since no handshake did.
    payload["_meta"]["io.modelcontextprotocol/serverInfo"] =
        json!({ "name": env!("CARGO_PKG_NAME"), "version": env!("CARGO_PKG_VERSION") });
    let envelope = json!({ "jsonrpc": "2.0", "id": id.unwrap_or(Value::Null), "result": payload });
    send(StatusCode::OK, envelope)
}

fn json_rpc_error_response(id: Option<Value>, payload: Value) -> Response {
    // The HTTP status is how a client tells a modern server from a legacy one
    // without reading further, so the revision names it for these codes.
    let status = match payload["error"]["code"].as_i64() {
        Some(-32601) => StatusCode::NOT_FOUND,
        Some(
            HEADER_MISMATCH | MISSING_REQUIRED_CLIENT_CAPABILITY | UNSUPPORTED_PROTOCOL_VERSION,
        ) => StatusCode::BAD_REQUEST,
        _ => StatusCode::OK,
    };
    send_error(status, id, payload)
}

fn send_error(status: StatusCode, id: Option<Value>, payload: Value) -> Response {
    let envelope =
        json!({ "jsonrpc": "2.0", "id": id.unwrap_or(Value::Null), "error": payload["error"] });
    send(status, envelope)
}

fn send(status: StatusCode, envelope: Value) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        "application/json".parse().unwrap(),
    );
    headers.insert(
        "MCP-Protocol-Version",
        MCP_PROTOCOL_VERSION.parse().unwrap(),
    );
    (status, headers, Json(envelope)).into_response()
}
