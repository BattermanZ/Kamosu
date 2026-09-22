//! The web door: an Axum router built by walking the Catalogue at startup.
//!
//! The API is deliberately RPC-shaped (`POST /api/op/<name>`), not REST-shaped.
//! "Fixing" that would destroy the parity guarantee (ADR 0001) — there is no
//! hand-written route here, only the Catalogue walked once.

use std::sync::Arc;

use axum::body::{Body, Bytes};
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
    // A file too large for any envelope — a whole Crouton library (#69) —
    // is staged the same way, streamed to disk rather than held in memory,
    // and named afterwards by the Operation that reads it. The Door's body
    // limit is lifted here only because the stream enforces its own.
    let staging_core = core.clone();
    router = router.route(
        "/api/uploads",
        post(move |headers: HeaderMap, body: Body| {
            let core = staging_core.clone();
            async move { stage_upload(&core, &headers, body).await }
        })
        .layer(axum::extract::DefaultBodyLimit::disable()),
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
    // A Bundle travels as its own bytes too (#66): `export_bundle` says what it
    // holds at both Doors, and this hands it over under the same Credential.
    let bundle_core = core.clone();
    router = router.route(
        "/api/bundles/{branch_id}",
        get(move |Path(branch_id): Path<String>, headers: HeaderMap| {
            let core = bundle_core.clone();
            async move { get_bundle(core, headers, branch_id).await }
        }),
    );
    // A Sheet is a PDF (#75), made as a Job and fetched here as its own bytes
    // under the same Credential the Job was asked with — or none, when a
    // stranger asked through a Share Link.
    let sheet_core = core.clone();
    router = router.route(
        "/api/sheets/{job_id}",
        get(move |Path(job_id): Path<String>, headers: HeaderMap| {
            let core = sheet_core.clone();
            async move { get_sheet(core, headers, job_id).await }
        }),
    );
    for op in catalogue::OPERATIONS.iter() {
        let core = core.clone();
        router = router.route(
            &format!("/api/op/{}", op.name),
            post(
                move |headers: HeaderMap, body: Option<Json<Value>>| async move {
                    let answered = call_operation(&core, op.name, &headers, body);
                    respond_to(&headers, answered)
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
    headers: &HeaderMap,
    body: Option<Json<Value>>,
) -> Result<Value, OpError> {
    // Transport concern only: carry the Secret to the Core. Deciding anything
    // about it here would be a permission check inside a Door.
    let secret = bearer_from_headers(headers);

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
        .and_then(|caller| core.upload_photograph(&caller, body))
    {
        Ok(result) => respond_to(headers, Ok(result)),
        Err(err) => respond_to(headers, Err(err)),
    }
}

/// The largest file `POST /api/uploads` stages. Aurélien's 86-recipe library
/// is 114 MB, almost all of it photographs; this leaves room for a library
/// many times that size without letting one request fill the disk.
const MAX_UPLOAD_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// `POST /api/uploads`: the raw bytes are the whole body, written to a staged
/// file under the sender and answered with its id — the envelope every
/// Operation answers, since the Operation that reads the file comes next.
async fn stage_upload(core: &Core, headers: &HeaderMap, body: Body) -> Response {
    let secret = bearer_from_headers(headers);
    let staged = match core
        .authenticate_for_write(secret.as_deref())
        .and_then(|caller| core.begin_upload(&caller.person_id))
    {
        Ok(staged) => staged,
        Err(err) => return respond_to(headers, Err(err)),
    };
    let (upload_id, path) = staged;
    match write_upload(body, &path).await {
        Ok(()) => respond_to(headers, Ok(json!({ "upload_id": upload_id }))),
        Err(err) => {
            let _ = tokio::fs::remove_file(&path).await;
            respond_to(headers, Err(err))
        }
    }
}

async fn write_upload(body: Body, path: &std::path::Path) -> Result<(), OpError> {
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    let mut file = tokio::fs::File::create(path)
        .await
        .map_err(|e| OpError::internal(format!("cannot stage the upload: {e}")))?;
    let mut stream = body.into_data_stream();
    let mut written: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|e| OpError::bad_request(format!("the upload stopped partway: {e}")))?;
        written += chunk.len() as u64;
        if written > MAX_UPLOAD_BYTES {
            return Err(OpError::bad_request("the file is larger than 2 GB"));
        }
        file.write_all(&chunk)
            .await
            .map_err(|e| OpError::internal(format!("cannot stage the upload: {e}")))?;
    }
    if written == 0 {
        return Err(OpError::bad_request("the upload is empty"));
    }
    file.flush()
        .await
        .map_err(|e| OpError::internal(format!("cannot stage the upload: {e}")))
}

/// `GET /api/photographs/{hash}`: the Photograph's own bytes, WebP.
///
/// The Caller goes through to the Core, which decides whether they may see
/// the picture (#99). Nothing here asks: a check in a Door is a bug (ADR 0001).
fn get_photograph(core: &Core, headers: &HeaderMap, hash: &str) -> Response {
    let secret = bearer_from_headers(headers);
    match core
        .authenticate(secret.as_deref())
        .and_then(|caller| core.read_photograph(&caller, hash))
    {
        Ok(bytes) => image_response(bytes),
        Err(err) => respond_to(headers, Err(err)),
    }
}

/// `GET /api/photographs/{hash}/{size}`: a Display Copy — `card`, `page` or
/// `print` (CONTEXT.md, "Display Copy") — generated and cached on first ask.
fn get_display_copy(core: &Core, headers: &HeaderMap, hash: &str, size: &str) -> Response {
    let secret = bearer_from_headers(headers);
    let Some(size) = DisplaySize::parse(size) else {
        return respond_to(
            headers,
            Err(OpError::bad_request(
                "a Display Copy is one of 'card', 'page' or 'print'",
            )),
        );
    };
    match core
        .authenticate(secret.as_deref())
        .and_then(|caller| core.read_display_copy(&caller, hash, size))
    {
        Ok(bytes) => image_response(bytes),
        Err(err) => respond_to(headers, Err(err)),
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
        Err(err) => return respond_to(headers, Err(err)),
    };
    let file = match tokio::fs::File::open(&path).await {
        Ok(file) => file,
        Err(_) => return respond_to(headers, Err(OpError::not_found("no such Backup"))),
    };
    // The length is declared rather than left to chunked framing, because the
    // thing on the other end is usually copying half a gigabyte and wants to
    // know how far along it is. An archive is never rewritten in place — a new
    // one gets a new name and the old one is unlinked — so the size read here
    // is the size that arrives.
    let length = match file.metadata().await {
        Ok(metadata) => metadata.len(),
        Err(_) => return respond_to(headers, Err(OpError::not_found("no such Backup"))),
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

/// `GET /api/bundles/{branch_id}`: one recipe's Bundle, zip (#66).
///
/// Built when asked, on the blocking pool: a Bundle carries its Photographs,
/// and reading and zipping them is file work the runtime answering every other
/// request should not wait behind. The Credential is checked in the Core, as
/// for any Operation — the same circle that may read the Branch may take it.
async fn get_bundle(core: Arc<Core>, headers: HeaderMap, branch_id: String) -> Response {
    let secret = bearer_from_headers(&headers);
    let built = tokio::task::spawn_blocking(move || {
        let caller = core.authenticate(secret.as_deref())?;
        core.bundle(&caller.person_id, &branch_id)
    })
    .await;
    let written = match built {
        Ok(Ok(written)) => written,
        Ok(Err(err)) => return respond_to(&headers, Err(err)),
        Err(e) => {
            return respond_to(
                &headers,
                Err(OpError::internal(format!(
                    "the Bundle could not be built: {e}"
                ))),
            );
        }
    };
    zip_response(&written.file_name, written.bytes)
}

async fn get_sheet(core: Arc<Core>, headers: HeaderMap, job_id: String) -> Response {
    let secret = bearer_from_headers(&headers);
    let read =
        tokio::task::spawn_blocking(move || core.read_sheet(secret.as_deref(), &job_id)).await;
    match read {
        Ok(Ok((name, bytes))) => pdf_response(&name, bytes),
        Ok(Err(err)) => respond_to(&headers, Err(err)),
        Err(e) => respond_to(
            &headers,
            Err(OpError::internal(format!(
                "the Sheet could not be read: {e}"
            ))),
        ),
    }
}

/// A Sheet as the browser should get it: shown rather than saved, so printing
/// is the browser's own Print and Kamosu has no dialog of its own (ADR 0023).
pub fn pdf_response(name: &str, bytes: Vec<u8>) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (header::CONTENT_LENGTH, bytes.len().to_string()),
            (header::CONTENT_DISPOSITION, disposition("inline", name)),
        ],
        bytes,
    )
        .into_response()
}

/// A `Content-Disposition` saying a download is to be saved, naming a file
/// that may not be ASCII.
fn attachment(name: &str) -> String {
    disposition("attachment", name)
}

/// One Bundle, as a download. Shared with the Share Link page, which hands the
/// same zip to a stranger holding the token (#65).
pub fn zip_response(name: &str, bytes: Vec<u8>) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/zip".to_string()),
            (header::CONTENT_LENGTH, bytes.len().to_string()),
            (header::CONTENT_DISPOSITION, attachment(name)),
        ],
        bytes,
    )
        .into_response()
}

/// A `Content-Disposition` naming a file that may not be ASCII.
///
/// `filename*` carries the real name, percent-encoded as UTF-8 (RFC 6266), and
/// `filename` an ASCII stand-in for a client too old to read it: a title like
/// `Bœuf bourguignon` is ordinary, and a raw non-ASCII header is not.
fn disposition(kind: &str, name: &str) -> String {
    let fallback: String = name
        .chars()
        .map(|ch| {
            if (ch.is_ascii_graphic() && ch != '"' && ch != '\\') || ch == ' ' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    let encoded: String = name
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect();
    format!("{kind}; filename=\"{fallback}\"; filename*=UTF-8''{encoded}")
}

fn image_response(bytes: Vec<u8>) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/webp")],
        bytes,
    )
        .into_response()
}

/// Everything about the cookie but its value, written once. Setting and
/// expiring share it because a browser keeps the original unless the expiring
/// one matches it on name and Path — two spellings that drifted apart would
/// leave the fault in place while looking fixed.
const SESSION_COOKIE_ATTRIBUTES: &str = "Path=/; HttpOnly; Secure; SameSite=Lax";

/// Answer a request that carries no Credential of this Door's making: the
/// `/auth/…` routes, which mint a Credential rather than present one, and the
/// reply to an Operation name the Catalogue does not declare. The empty headers
/// say what is true of both — there is no cookie here to take back.
fn respond(result: Result<Value, OpError>) -> Response {
    respond_to(&HeaderMap::new(), result)
}

/// Answer one request, taking back its Session cookie where the Core says that
/// cookie names nobody (#91).
fn respond_to(headers: &HeaderMap, result: Result<Value, OpError>) -> Response {
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
                let cookie = format!("kamosu_session={secret}; {SESSION_COOKIE_ATTRIBUTES}");
                response.headers_mut().insert(
                    header::SET_COOKIE,
                    HeaderValue::from_str(&cookie).expect("safe session cookie"),
                );
            }
            response
        }
        Err(err) => {
            let mut response = (status_for(err.kind), Json(error_body(&err))).into_response();
            // A Session that has ended leaves the browser still holding its
            // cookie, and holding it is what makes even a Public Operation come
            // back refused — so without this the browser waits on the sign-in
            // screen forever (#91). The Core has already decided the Secret
            // names nobody; the Door merely takes back a cookie it set, which is
            // housekeeping rather than a permission check.
            //
            // Only when that cookie is the Secret the Core was actually handed.
            // A bearer token wins over the cookie when both arrive, and a bearer
            // token is not this Door's to take back — so a refused one leaves
            // even a cookie sitting beside it exactly as it was.
            if err.credential_names_nobody && secret_came_from_cookie(headers) {
                let expired = format!("kamosu_session=; {SESSION_COOKIE_ATTRIBUTES}; Max-Age=0");
                response.headers_mut().insert(
                    header::SET_COOKIE,
                    HeaderValue::from_str(&expired).expect("safe expiring session cookie"),
                );
            }
            response
        }
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
    bearer_token(headers).or_else(|| session_cookie(headers))
}

/// Whether the Secret the Core was handed came from the cookie this Door set,
/// which is the only Secret a refusal may ever take back. It reads the same two
/// headers in the same order `bearer_from_headers` does, so the question "whose
/// Secret was refused" can never be answered about a different one.
fn secret_came_from_cookie(headers: &HeaderMap) -> bool {
    bearer_token(headers).is_none() && session_cookie(headers).is_some()
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())?;
    let rest = value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))?;
    Some(rest.trim().to_string())
}

/// The Secret this Door itself put in a `kamosu_session` cookie, if the request
/// carried one back. Kept apart from a bearer token because the two are the
/// Door's business to different degrees: this cookie is Kamosu's own to set and
/// to take back, while a bearer token belongs to whoever sent it and the Door
/// touches nothing of it.
fn session_cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())?
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix("kamosu_session=").map(str::to_owned))
}
