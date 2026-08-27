//! The web door: an Axum router built by walking the Catalogue at startup.
//!
//! The API is deliberately RPC-shaped (`POST /api/op/<name>`), not REST-shaped.
//! "Fixing" that would destroy the parity guarantee (ADR 0001) — there is no
//! hand-written route here, only the Catalogue walked once.

use std::sync::Arc;

use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, post};
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::catalogue;
use crate::core::{Core, ErrorKind, OpError};

/// Build the whole web door from the Catalogue. One route per Operation; nothing
/// else. A permission check here would be a bug — authorisation lives in the Core.
pub fn router(core: Arc<Core>) -> Router {
    // The unknown-Operation answer is scoped to `/api/op/…`, not to the whole
    // router: every other path belongs to the interface, which is merged after
    // the Doors and owns the fallback. `/api/op/anything-else` is still a
    // Catalogue question, and still answered as one.
    let first_core = core.clone();
    let login_core = core.clone();
    let invite_core = core.clone();
    let recovery_core = core.clone();
    let mut router = Router::new()
        .route("/api/op/{*name}", any(unknown_operation))
        // Credential minting is deliberately outside the Catalogue: it precedes
        // every Operation, then the cookie becomes the Credential the Door carries.
        .route(
            "/auth/first-person",
            post(move |body: Option<Json<Value>>| async move {
                respond(authenticate_first_person(&first_core, body))
            }),
        )
        .route(
            "/auth/login",
            post(move |body: Option<Json<Value>>| async move {
                respond(authenticate_login(&login_core, body))
            }),
        )
        .route(
            "/auth/invite",
            post(move |body: Option<Json<Value>>| async move {
                respond(authenticate_invite(&invite_core, body))
            }),
        )
        .route(
            "/auth/recover",
            post(move |body: Option<Json<Value>>| async move {
                respond(authenticate_recovery(&recovery_core, body))
            }),
        );
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

/// Reached only when no exact route claimed the path, so the name is one the
/// Catalogue does not declare. Naming it back is the whole of the answer.
async fn unknown_operation(axum::extract::Path(name): axum::extract::Path<String>) -> Response {
    respond(Err(OpError::unknown_operation(&name)))
}

fn authenticate_first_person(core: &Core, body: Option<Json<Value>>) -> Result<Value, OpError> {
    let input = body.map(|Json(value)| value).unwrap_or_default();
    let name = input.get("name").and_then(Value::as_str).ok_or_else(|| {
        OpError::bad_request("account creation takes { name, password, session_name }")
    })?;
    let password = input
        .get("password")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request("account creation takes { name, password, session_name }")
        })?;
    let session_name = input
        .get("session_name")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request("account creation takes { name, password, session_name }")
        })?;
    core.create_first_person(name, password, session_name)
}

fn authenticate_login(core: &Core, body: Option<Json<Value>>) -> Result<Value, OpError> {
    let input = body.map(|Json(value)| value).unwrap_or_default();
    let name = input
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("login takes { name, password, session_name }"))?;
    let password = input
        .get("password")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("login takes { name, password, session_name }"))?;
    let session_name = input
        .get("session_name")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("login takes { name, password, session_name }"))?;
    core.log_in(name, password, session_name)
}

fn authenticate_invite(core: &Core, body: Option<Json<Value>>) -> Result<Value, OpError> {
    let input = body.map(|Json(value)| value).unwrap_or_default();
    let link = input.get("link").and_then(Value::as_str).ok_or_else(|| {
        OpError::bad_request("Invite redemption takes { link, name, password, session_name }")
    })?;
    let name = input.get("name").and_then(Value::as_str).ok_or_else(|| {
        OpError::bad_request("Invite redemption takes { link, name, password, session_name }")
    })?;
    let password = input
        .get("password")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request("Invite redemption takes { link, name, password, session_name }")
        })?;
    let session_name = input
        .get("session_name")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request("Invite redemption takes { link, name, password, session_name }")
        })?;
    core.redeem_invite(link, name, password, session_name)
}

fn authenticate_recovery(core: &Core, body: Option<Json<Value>>) -> Result<Value, OpError> {
    let input = body.map(|Json(value)| value).unwrap_or_default();
    let link = input
        .get("link")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("recovery takes { link, password, session_name }"))?;
    let password = input
        .get("password")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("recovery takes { link, password, session_name }"))?;
    let session_name = input
        .get("session_name")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("recovery takes { link, password, session_name }"))?;
    core.redeem_recovery(link, password, session_name)
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
        Ok(mut value) => {
            // A browser Session is delivered in an HttpOnly Secure cookie, never
            // exposed to the app's JavaScript. The private field is Core plumbing,
            // removed before the declared Operation result crosses this Door.
            let session_secret = value
                .as_object_mut()
                .and_then(|object| object.remove("_session_secret"))
                .and_then(|secret| secret.as_str().map(str::to_owned));
            let mut response =
                (StatusCode::OK, Json(json!({ "ok": true, "result": value }))).into_response();
            if let Some(secret) = session_secret {
                let cookie =
                    format!("kamosu_session={secret}; Path=/; HttpOnly; Secure; SameSite=Lax");
                response.headers_mut().insert(
                    header::SET_COOKIE,
                    HeaderValue::from_str(&cookie).expect("safe session cookie"),
                );
            }
            response
        }
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
    if let Some(value) = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        && let Some(rest) = value
            .strip_prefix("Bearer ")
            .or_else(|| value.strip_prefix("bearer "))
    {
        return Some(rest.trim().to_string());
    }
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())?
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix("kamosu_session=").map(str::to_owned))
}
