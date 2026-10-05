//! The design tokens, served (issue #80).
//!
//! Everything Kamosu looks like is compiled, once, by the Tailwind build in
//! `ui/` into `assets/app.css` — one generated stylesheet. This module embeds
//! that stylesheet and its companion assets (fonts, the mark, icons) into the
//! binary and serves them.
//!
//! The `/tokens` page this module once rendered now lives in the Svelte app as
//! an ordinary route (issue #37). It reads the same values from the same
//! stylesheet, which is the whole point of it: the page cannot drift from what
//! Kamosu actually looks like, because it is drawn from the same source every
//! screen is.
//!
//! None of this is an Operation: nothing here enters the Catalogue, and Parity
//! is untouched. A permission check here would be meaningless anyway — these
//! routes carry no Credential, only pixels.

use axum::Router;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;

/// The one generated stylesheet. Committed because a cargo build cannot run
/// npm; `just css` regenerates it and `just check` fails if it has drifted.
pub const STYLESHEET: &str = include_str!("../assets/app.css");

const MARK_SVG: &str = include_str!("../assets/img/kamosu-mark.svg");

/// Fonts self-hosted under the binary: an instance may never reach the
/// internet, so no external font request exists. Latin + Latin-ext subsets,
/// with their licences alongside them.
const FONTS: &[(&str, &[u8])] = &[
    (
        "zen-old-mincho-400-latin.woff2",
        include_bytes!("../assets/fonts/zen-old-mincho-400-latin.woff2"),
    ),
    (
        "zen-old-mincho-400-latin-ext.woff2",
        include_bytes!("../assets/fonts/zen-old-mincho-400-latin-ext.woff2"),
    ),
    (
        "zen-old-mincho-600-latin.woff2",
        include_bytes!("../assets/fonts/zen-old-mincho-600-latin.woff2"),
    ),
    (
        "zen-old-mincho-600-latin-ext.woff2",
        include_bytes!("../assets/fonts/zen-old-mincho-600-latin-ext.woff2"),
    ),
    (
        "zen-kaku-gothic-new-400-latin.woff2",
        include_bytes!("../assets/fonts/zen-kaku-gothic-new-400-latin.woff2"),
    ),
    (
        "zen-kaku-gothic-new-400-latin-ext.woff2",
        include_bytes!("../assets/fonts/zen-kaku-gothic-new-400-latin-ext.woff2"),
    ),
    (
        "zen-kaku-gothic-new-500-latin.woff2",
        include_bytes!("../assets/fonts/zen-kaku-gothic-new-500-latin.woff2"),
    ),
    (
        "zen-kaku-gothic-new-500-latin-ext.woff2",
        include_bytes!("../assets/fonts/zen-kaku-gothic-new-500-latin-ext.woff2"),
    ),
    (
        "zen-kaku-gothic-new-700-latin.woff2",
        include_bytes!("../assets/fonts/zen-kaku-gothic-new-700-latin.woff2"),
    ),
    (
        "zen-kaku-gothic-new-700-latin-ext.woff2",
        include_bytes!("../assets/fonts/zen-kaku-gothic-new-700-latin-ext.woff2"),
    ),
];

const ICONS: &[(&str, &[u8])] = &[
    (
        "icon-192.png",
        include_bytes!("../assets/icons/icon-192.png"),
    ),
    (
        "icon-512.png",
        include_bytes!("../assets/icons/icon-512.png"),
    ),
    (
        "apple-touch-icon.png",
        include_bytes!("../assets/icons/apple-touch-icon.png"),
    ),
    (
        "favicon-32.png",
        include_bytes!("../assets/icons/favicon-32.png"),
    ),
];

/// The routes for everything visual. Merged beside the Doors at startup —
/// they are not Operations and never enter the Catalogue.
pub fn router() -> Router {
    Router::new()
        .route("/assets/app.css", get(stylesheet))
        .route("/assets/fonts/{name}", get(font))
        .route("/assets/icons/{name}", get(icon))
        .route(
            "/assets/img/kamosu-mark.svg",
            get(|| async { svg(MARK_SVG) }),
        )
        .route("/favicon.svg", get(|| async { svg(MARK_SVG) }))
        // Safari asks for these at the root by convention even where a <link>
        // tag names them, so both spellings answer.
        .route(
            "/favicon-32.png",
            get(|| async { icon_bytes("favicon-32.png") }),
        )
        .route(
            "/apple-touch-icon.png",
            get(|| async { icon_bytes("apple-touch-icon.png") }),
        )
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn stylesheet() -> Response {
    css(STYLESHEET)
}

async fn font(axum::extract::Path(name): axum::extract::Path<String>) -> Response {
    match FONTS.iter().find(|(known, _)| *known == name) {
        Some((_, bytes)) => response(bytes.to_vec(), "font/woff2"),
        None => not_found(),
    }
}

async fn icon(axum::extract::Path(name): axum::extract::Path<String>) -> Response {
    match find_icon(&name) {
        Some(bytes) => response(bytes.to_vec(), "image/png"),
        None => not_found(),
    }
}

fn icon_bytes(name: &str) -> Response {
    match find_icon(name) {
        Some(bytes) => response(bytes.to_vec(), "image/png"),
        None => not_found(),
    }
}

fn find_icon(name: &str) -> Option<&'static [u8]> {
    ICONS
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, bytes)| *bytes)
}

// ── Responses ────────────────────────────────────────────────────────────────

fn response(body: Vec<u8>, content_type: &str) -> Response {
    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        body,
    )
        .into_response()
}

fn css(body: &'static str) -> Response {
    response(body.as_bytes().to_vec(), "text/css; charset=utf-8")
}

fn svg(body: &'static str) -> Response {
    response(body.as_bytes().to_vec(), "image/svg+xml")
}

fn not_found() -> Response {
    StatusCode::NOT_FOUND.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    /// Every custom property declared in the generated stylesheet's theme layer,
    /// as `--name -> value`. Nothing at runtime reads these — the interface does,
    /// in the browser, off the very stylesheet it loaded. Parsing them here is
    /// how the tests below hold the embedded copy to the identity it must carry.
    fn parse_theme_variables(css: &str) -> BTreeMap<String, String> {
        let mut vars = BTreeMap::new();
        // The generated file declares exactly one `:root, :host { ... }` block,
        // inside @layer theme. Values contain commas and parens but never a
        // semicolon, so splitting declarations on ';' is safe.
        let Some(start) = css.find(":root, :host {") else {
            return vars;
        };
        let body = &css[start..];
        let Some(end) = body.find('}') else {
            return vars;
        };
        for declaration in body[..end].split(';') {
            let declaration = declaration.trim();
            let Some((name, value)) = declaration.split_once(':') else {
                continue;
            };
            let name = name.trim();
            if let Some(stripped) = name.strip_prefix("--") {
                // Namespace resets (`--color-*: initial`) are build machinery,
                // not tokens worth holding to a value.
                if !stripped.ends_with("*") && stripped != "default-font-family" {
                    vars.insert(name.to_string(), value.trim().to_string());
                }
            }
        }
        vars
    }

    #[test]
    fn theme_variables_come_from_the_generated_stylesheet() {
        let vars = parse_theme_variables(STYLESHEET);
        // The identity's anchor colours are present with their chosen values.
        assert_eq!(
            vars.get("--color-ground").map(String::as_str),
            Some("#f4efe3")
        );
        assert_eq!(
            vars.get("--color-accent").map(String::as_str),
            Some("#1d2b4c")
        );
        // The Step is the LARGEST TYPE IN THE APP. ADR 0011 records that as spec
        // rather than preference, so the ranking is asserted and not just the
        // number: #88 moved the Step 33px -> 26px and the recipe title 27px ->
        // 25px together, re-fitting the cooking screen against the ~700px a
        // phone actually gives it once Safari's URL bar is counted. A future
        // change that shrinks the Step past the title breaks the ADR, and this
        // is what says so.
        let px = |name: &str| -> f32 {
            vars.get(name)
                .and_then(|value| value.strip_suffix("px"))
                .and_then(|value| value.parse().ok())
                .unwrap_or_else(|| panic!("{name} is missing or is not a px value"))
        };
        assert_eq!(vars.get("--text-step").map(String::as_str), Some("26px"));
        assert!(
            px("--text-step") > px("--text-title"),
            "ADR 0011: the Step is the largest type in the app"
        );
        // The wide layout sets the cooking screen larger (#197), and the same
        // ranking holds there: nothing the screen draws is larger than its Step.
        let far = px("--text-step-far");
        for other in [
            "--text-step",
            "--text-step-reading-far",
            "--text-panel-figure-far",
            "--text-body-far",
            "--text-line-far",
            "--text-read-far",
            "--text-label-far",
            "--text-foot-far",
            "--text-next-step",
        ] {
            assert!(
                far > px(other),
                "ADR 0011: the Step is the largest type in the app, and {other} is not smaller"
            );
        }
        assert_eq!(
            vars.get("--spacing-gutter").map(String::as_str),
            Some("20px")
        );
        assert_eq!(vars.get("--radius-pill").map(String::as_str), Some("2px"));
        assert_eq!(vars.get("--tile-w").map(String::as_str), Some("168px"));
    }

    #[test]
    fn tailwinds_default_palette_is_gone_wholesale() {
        let vars = parse_theme_variables(STYLESHEET);
        // Reaching for a Tailwind default colour must not be possible.
        assert!(vars.keys().all(|name| !name.starts_with("--color-red")
            && !name.starts_with("--color-slate")
            && !name.starts_with("--color-gray")));
        assert!(vars.contains_key("--color-ground"), "our own tokens remain");
    }

    #[test]
    fn every_declared_token_group_has_its_values() {
        let vars = parse_theme_variables(STYLESHEET);
        // Counted rather than listed: the interface holds the palette in the
        // order it shows them (ui/src/lib/tokens.ts), and a second hand-kept
        // list here would be one more thing to fall behind the stylesheet.
        let colours = vars.keys().filter(|k| k.starts_with("--color-")).count();
        assert_eq!(colours, 17, "the whole palette from #36 is present");
        let steps = vars
            .keys()
            .filter(|k| {
                k.starts_with("--text-")
                    && !k.contains("--line-height")
                    && !k.contains("--letter-spacing")
            })
            .count();
        assert_eq!(
            steps, 20,
            "step, title, line, list-title, shelf-heading, tile-title, panel-figure, body, \
             step-reading, read, label; and the cooking screen read from the counter (#197): \
             step-far, step-reading-far, panel-figure-far, body-far, line-far, read-far, label-far, \
             foot-far, next-step"
        );
    }
}
