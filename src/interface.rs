//! The interface, compiled into the binary (ADR 0028).
//!
//! Kamosu is one executable and one data directory. The Svelte app built from
//! `ui/` is therefore embedded rather than copied beside the binary: the
//! artefact *is* the version, and a half-upgraded install — new binary, last
//! month's asset directory — is unrepresentable.
//!
//! In debug builds `rust-embed` reads `ui/build` from disk instead, so a
//! frontend change recompiles no Rust and `npm run dev` (5174) and
//! `cargo run` (5266) stay two processes side by side.
//!
//! None of this is an Operation. Nothing here enters the Catalogue and Parity
//! is untouched — these routes carry no Credential, only pixels.

use axum::Router;
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use rust_embed::Embed;

/// The output of `npm run build` in `ui/`: SvelteKit on `adapter-static` with
/// `fallback: index.html`, so the whole app is one shell plus its chunks.
#[derive(Embed)]
#[folder = "ui/build/"]
struct Built;

/// The shell every screen is served from. `adapter-static`'s fallback: the
/// router runs in the browser, so `/recipes` and `/settings` are the same
/// document arriving at different addresses.
const SHELL: &str = "index.html";

/// The interface's routes: the compiled app, and the fallback that hands any
/// unclaimed path back to the shell.
///
/// Merged **last**, after both Doors and the design tokens, because it owns the
/// fallback — a path is a screen only once nothing else has claimed it.
pub fn router() -> Router {
    Router::new().fallback(serve)
}

async fn serve(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    if path.is_empty() {
        return shell();
    }

    if let Some(file) = Built::get(path) {
        let content_type = content_type_for(path);
        return (
            [
                (header::CONTENT_TYPE, content_type),
                (header::CACHE_CONTROL, cache_control_for(path)),
            ],
            file.data.into_owned(),
        )
            .into_response();
    }

    // A path with a dot asked for a file that does not exist; answering the
    // shell there would hand a browser HTML where it expected a script, which
    // fails later and less clearly than a 404 does.
    if path.contains('.') {
        return StatusCode::NOT_FOUND.into_response();
    }

    shell()
}

fn shell() -> Response {
    match Built::get(SHELL) {
        Some(file) => (
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                // The shell names hashed chunks, so it must never be the stale
                // half of a pair. Only the chunks it names are cacheable.
                (header::CACHE_CONTROL, "no-cache"),
            ],
            file.data.into_owned(),
        )
            .into_response(),
        // Only reachable in a debug build run before `just ui-build`: the
        // release build fails in build.rs rather than shipping an empty app.
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            "The interface has not been built. Run `just ui-build`, or use the \
             vite dev server on 5174.",
        )
            .into_response(),
    }
}

/// Content types for what SvelteKit actually emits. A short explicit list beats
/// a mime-guessing dependency: anything absent from it is a file this build
/// does not produce.
fn content_type_for(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, ext)| ext) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        // A web manifest has its own registered media type; Safari is the
        // stricter reader of it, and iOS is what this app is built for.
        Some("webmanifest") => "application/manifest+json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("woff2") => "font/woff2",
        Some("txt") => "text/plain; charset=utf-8",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}

/// SvelteKit puts everything content-hashed under `_app/immutable/`; its name
/// changes when its bytes do, so it can be cached for as long as a browser
/// likes. Everything else is a plain name that may be replaced in place.
fn cache_control_for(path: &str) -> &'static str {
    if path.starts_with("_app/immutable/") {
        "public, max-age=31536000, immutable"
    } else {
        "public, max-age=3600"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashed_chunks_are_cached_forever_and_nothing_else_is() {
        assert_eq!(
            cache_control_for("_app/immutable/chunks/abc123.js"),
            "public, max-age=31536000, immutable"
        );
        assert_eq!(
            cache_control_for("manifest.webmanifest"),
            "public, max-age=3600"
        );
    }

    #[test]
    fn content_types_cover_what_the_build_emits() {
        assert_eq!(
            content_type_for("_app/immutable/entry/start.js"),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(
            content_type_for("manifest.webmanifest"),
            "application/manifest+json; charset=utf-8"
        );
        assert_eq!(content_type_for("robots.txt"), "text/plain; charset=utf-8");
        assert_eq!(content_type_for("no-extension"), "application/octet-stream");
    }
}
