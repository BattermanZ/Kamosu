//! The web door: an Axum router built by walking the Catalogue at startup.
//!
//! The API is deliberately RPC-shaped (`POST /api/op/<name>`), not REST-shaped.
//! "Fixing" that would destroy the parity guarantee (ADR 0001) — there is no
//! hand-written route here, only the Catalogue walked once.

use std::sync::Arc;

use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::catalogue;
use crate::core::{Core, ErrorKind, OpError};

/// Build the whole web door from the Catalogue. One route per Operation; nothing
/// else. A permission check here would be a bug — authorisation lives in the Core.
pub fn router(core: Arc<Core>) -> Router {
    let mut router = Router::new().fallback(unknown_operation_fallback);
    for op in catalogue::OPERATIONS.iter() {
        let core = core.clone();
        router = router.route(
            &format!("/api/op/{}", op.name),
            post(
                move |headers: HeaderMap, body: Option<Json<Value>>| async move {
                    respond(call_operation(&core, op.name, headers, body))
                },
            ),
        );
    }
    router.with_state(())
}

async fn unknown_operation_fallback() -> Response {
    let err = OpError::unknown_operation("(the path does not name an Operation in the Catalogue)");
    respond(Err(err))
}

fn call_operation(
    core: &Core,
    operation_name: &str,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Result<Value, OpError> {
    // Transport concern only: carry the Secret to the Core. Deciding anything
    // about it here would be a permission check inside a Door.
    let secret = bearer_from_headers(&headers);

    // An absent body is an empty envelope; the Core still checks the input against
    // what the Catalogue declared.
    let input = match body {
        Some(Json(value)) => value,
        None => Value::Object(serde_json::Map::new()),
    };

    core.execute(secret.as_deref(), operation_name, input)
}

fn respond(result: Result<Value, OpError>) -> Response {
    match result {
        Ok(value) => (StatusCode::OK, Json(json!({ "ok": true, "result": value }))).into_response(),
        Err(err) => (status_for(err.kind), Json(error_body(&err))).into_response(),
    }
}

fn error_body(err: &OpError) -> Value {
    json!({
        "ok": false,
        "error": {
            "kind": kind_name(err.kind),
            "message": err.to_sentence(),
        },
    })
}

fn status_for(kind: ErrorKind) -> StatusCode {
    match kind {
        ErrorKind::Unauthorized => StatusCode::UNAUTHORIZED,
        ErrorKind::UnknownOperation => StatusCode::NOT_FOUND,
        ErrorKind::NotFound => StatusCode::NOT_FOUND,
        // Refused, not failed: a full lane answers busy rather than growing
        // (ADR 0032). The caller is invited back in a moment.
        ErrorKind::Busy => StatusCode::SERVICE_UNAVAILABLE,
        ErrorKind::BadRequest => StatusCode::BAD_REQUEST,
        ErrorKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn kind_name(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::Unauthorized => "unauthorized",
        ErrorKind::UnknownOperation => "unknown_operation",
        ErrorKind::NotFound => "not_found",
        ErrorKind::Busy => "busy",
        ErrorKind::BadRequest => "bad_request",
        ErrorKind::Internal => "internal",
    }
}

/// Pull `Authorization: Bearer <secret>` off a request. Shared by both Doors as a
/// transport detail; what the Secret means is decided in the Core alone.
pub fn bearer_from_headers(headers: &HeaderMap) -> Option<String> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    let rest = value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))?;
    Some(rest.trim().to_string())
}
