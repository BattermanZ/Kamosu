//! The web door: an Axum router built by walking the Catalogue at startup.
//!
//! The API is deliberately RPC-shaped (`POST /api/op/<name>`), not REST-shaped.
//! "Fixing" that would destroy the parity guarantee (ADR 0001) — there is no
//! hand-written route here, only the Catalogue walked once.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::Path;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get, post};
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::catalogue;
use crate::core::{Core, ErrorKind, OpError};
use crate::photographs::DisplaySize;

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
    // Photographs travel out of band, authenticated with the same Credential
    // as any Operation (ADR 0001) — raw bytes rather than a JSON envelope, so
    // they never pass through the Catalogue's dispatch. `upload_photograph`
    // is the base64 fallback for a Door that cannot carry raw bytes at all.
    let upload_core = core.clone();
    let photograph_core = core.clone();
    let display_core = core.clone();
    router = router
        .route(
            "/api/photographs",
            post(move |headers: HeaderMap, body: Bytes| {
                let core = upload_core.clone();
                async move { upload_photograph(&core, &headers, &body) }
            }),
        )
        .route(
            "/api/photographs/{hash}",
            get(move |Path(hash): Path<String>, headers: HeaderMap| {
                let core = photograph_core.clone();
                async move { get_photograph(&core, &headers, &hash) }
            }),
        )
        .route(
            "/api/photographs/{hash}/{size}",
            get(
                move |Path((hash, size)): Path<(String, String)>, headers: HeaderMap| {
                    let core = display_core.clone();
                    async move { get_display_copy(&core, &headers, &hash, &size) }
                },
            ),
        );
    // A Backup travels the same way and for the same reason (#78): it is one
    // archive of the whole instance, far past anything a JSON envelope should
    // carry, so it is fetched as its own bytes under the same Credential.
    // `list_backups` names what there is to fetch, and it is an ordinary
    // Operation, so both Doors list Backups even though only this one can hand
    // the bytes over.
    let backup_core = core.clone();
    router = router.route(
        "/api/backups/{name}",
        get(move |Path(name): Path<String>, headers: HeaderMap| {
            let core = backup_core.clone();
            async move { get_backup(&core, &headers, &name).await }
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
    router
        .layer(axum::extract::DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(())
}

/// Axum's own default body limit is 2 MB — plenty for a recipe, nowhere near
/// enough for a camera-original photograph carried as base64 (a third larger
/// again than its own bytes) or raw. 40 MB comfortably covers a modern phone's
/// photo either way, with room to spare.
const MAX_BODY_BYTES: usize = 40 * 1024 * 1024;

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

/// `POST /api/photographs`: the raw bytes are the whole body — one picture
/// per request, so no multipart envelope is needed. Answers the same
/// `{ "ok": true, "result": { "photograph_id": ... } }` envelope as an
/// Operation, since this is one in every way but how its bytes travel.
fn upload_photograph(core: &Core, headers: &HeaderMap, body: &[u8]) -> Response {
    let secret = bearer_from_headers(headers);
    match core
        .authenticate_for_write(secret.as_deref())
        .and_then(|_| core.store_photograph(body))
    {
        Ok(result) => respond(Ok(result)),
        Err(err) => respond(Err(err)),
    }
}

/// `GET /api/photographs/{hash}`: the Photograph's own bytes, WebP.
fn get_photograph(core: &Core, headers: &HeaderMap, hash: &str) -> Response {
    let secret = bearer_from_headers(headers);
    match core
        .authenticate(secret.as_deref())
        .and_then(|_| core.read_photograph(hash))
    {
        Ok(bytes) => image_response(bytes),
        Err(err) => respond(Err(err)),
    }
}

/// `GET /api/photographs/{hash}/{size}`: a Display Copy — `card`, `page` or
/// `print` (CONTEXT.md, "Display Copy") — generated and cached on first ask.
fn get_display_copy(core: &Core, headers: &HeaderMap, hash: &str, size: &str) -> Response {
    let secret = bearer_from_headers(headers);
    let Some(size) = DisplaySize::parse(size) else {
        return respond(Err(OpError::bad_request(
            "a Display Copy is one of 'card', 'page' or 'print'",
        )));
    };
    match core
        .authenticate(secret.as_deref())
        .and_then(|_| core.read_display_copy(hash, size))
    {
        Ok(bytes) => image_response(bytes),
        Err(err) => respond(Err(err)),
    }
}

/// `GET /api/backups/{name}`: one Backup's archive, zip.
///
/// Sent straight off the disk in chunks rather than read whole into memory
/// first: an archive of a real library is hundreds of megabytes, and buffering
/// one would make fetching a Backup the largest thing the instance ever does
/// to its own memory.
async fn get_backup(core: &Core, headers: &HeaderMap, name: &str) -> Response {
    let secret = bearer_from_headers(headers);
    let path = match core
        .authenticate_for_operator(secret.as_deref())
        .and_then(|_| core.backup_path(name))
    {
        Ok(path) => path,
        Err(err) => return respond(Err(err)),
    };
    let file = match tokio::fs::File::open(&path).await {
        Ok(file) => file,
        Err(_) => return respond(Err(OpError::not_found("no such Backup"))),
    };
    // The length is declared rather than left to chunked framing, because the
    // thing on the other end is usually copying half a gigabyte and wants to
    // know how far along it is. An archive is never rewritten in place — a new
    // one gets a new name and the old one is unlinked — so the size read here
    // is the size that arrives.
    let length = match file.metadata().await {
        Ok(metadata) => metadata.len(),
        Err(_) => return respond(Err(OpError::not_found("no such Backup"))),
    };
    let stream = tokio_util::io::ReaderStream::new(file);
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/zip".to_string()),
            (header::CONTENT_LENGTH, length.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{name}\""),
            ),
        ],
        axum::body::Body::from_stream(stream),
    )
        .into_response()
}

fn image_response(bytes: Vec<u8>) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/webp")],
        bytes,
    )
        .into_response()
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
