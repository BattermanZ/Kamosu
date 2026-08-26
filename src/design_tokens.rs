//! The design tokens, served (issue #80).
//!
//! Everything Kamosu looks like is compiled, once, by the Tailwind build in
//! `ui/` into `assets/app.css` — one generated stylesheet. This module embeds
//! that stylesheet and its companion assets (fonts, the mark, icons) into the
//! binary and serves them; it renders `/tokens`, a development surface that
//! reads every value back out of the same embedded stylesheet, so the page
//! cannot drift from what Kamosu actually looks like — it is drawn from the
//! same code every screen is.
//!
//! None of this is an Operation: nothing here enters the Catalogue, and Parity
//! is untouched. A permission check here would be meaningless anyway — these
//! routes carry no Credential, only pixels.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use axum::Router;
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
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
        // Safari asks for these at the root by convention when no <link> tag
        // names them; the HTML pages that will name them arrive in #37.
        .route(
            "/favicon-32.png",
            get(|| async { icon_bytes("favicon-32.png") }),
        )
        .route(
            "/apple-touch-icon.png",
            get(|| async { icon_bytes("apple-touch-icon.png") }),
        )
        .route("/tokens", get(tokens_page))
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

async fn tokens_page() -> Html<String> {
    Html(render_tokens_page())
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

// ── Reading the tokens back out of the stylesheet ────────────────────────────

/// Every custom property declared in the generated stylesheet's theme layer,
/// as `--name → value`. This is the whole trick of the tokens page: the values
/// shown are parsed from the very stylesheet the app loads, never copied.
fn theme_variables() -> &'static BTreeMap<String, String> {
    static VARS: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    VARS.get_or_init(|| parse_theme_variables(STYLESHEET))
}

fn parse_theme_variables(css: &str) -> BTreeMap<String, String> {
    let mut vars = BTreeMap::new();
    // The generated file declares exactly one `:root, :host { … }` block, inside
    // @layer theme. Values contain commas and parens but never a semicolon, so
    // splitting declarations on ';' is safe.
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
            // not tokens worth showing.
            if !stripped.ends_with("*") && stripped != "default-font-family" {
                vars.insert(name.to_string(), value.trim().to_string());
            }
        }
    }
    vars
}

fn lookup(var: &str) -> String {
    theme_variables().get(var).cloned().unwrap_or_default()
}

// ── The page ─────────────────────────────────────────────────────────────────

/// Canonical narrative order for each token group. Names only — every value on
/// the page comes from `theme_variables()`.
const COLOUR_ORDER: &[&str] = &[
    "--color-ground",
    "--color-ground-2",
    "--color-card",
    "--color-ink",
    "--color-ink-2",
    "--color-rule",
    "--color-accent",
    "--color-on-accent",
    "--color-support",
    "--color-support-2",
    "--color-cook-ground",
    "--color-cook-ink",
    "--color-cook-ink-2",
    "--color-cook-rule",
    "--color-cook-panel",
    "--color-cook-accent",
    "--color-cook-on-accent",
];
const RADIUS_ORDER: &[&str] = &["--radius-sm", "--radius-md", "--radius-lg", "--radius-pill"];

fn render_tokens_page() -> String {
    // Type sizes: the base names (--text-step) with their optional sub-keys.
    let mut type_sizes: Vec<(String, Option<String>, Option<String>)> = Vec::new();
    for name in theme_variables().keys() {
        if !name.starts_with("--text-")
            || name.contains("--line-height")
            || name.contains("--letter-spacing")
        {
            continue;
        }
        type_sizes.push((
            name.clone(),
            theme_variables()
                .get(&format!("{name}--line-height"))
                .cloned(),
            theme_variables()
                .get(&format!("{name}--letter-spacing"))
                .cloned(),
        ));
    }
    type_sizes.sort();

    // Spacing steps, numerically, gutter last.
    let mut spacing: Vec<(String, String)> = theme_variables()
        .iter()
        .filter(|(k, _)| k.starts_with("--spacing-"))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    spacing.sort_by(|a, b| {
        let key = |(name, _): &(String, String)| {
            name.trim_start_matches("--spacing-")
                .parse::<f64>()
                .unwrap_or(f64::INFINITY)
        };
        key(a).total_cmp(&key(b))
    });

    let mut page = String::with_capacity(16 * 1024);
    page.push_str(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>Kamosu · tokens</title>\n<link rel=\"stylesheet\" href=\"/assets/app.css\">\n\
         <link rel=\"icon\" href=\"/favicon.svg\" type=\"image/svg+xml\">\n</head>\n\
         <body class=\"min-h-screen bg-ground text-ink font-sans text-body\">\n\
         <main class=\"max-w-3xl mx-auto p-gutter pb-12\">\n",
    );

    page.push_str(
        "<header class=\"flex items-center gap-3 mb-6\">\
         <img src=\"/assets/img/kamosu-mark.svg\" alt=\"Kamosu mark\" class=\"h-10 w-10 rounded-sm\">\
         <div><h1 class=\"font-display font-semibold text-title leading-none\">Kamosu tokens</h1>\
         <p class=\"text-read text-ink-2 mt-1\">Every value below is read from /assets/app.css — \
         the same stylesheet every screen loads. Nothing here is maintained by hand.</p></div></header>\n",
    );

    page.push_str("<section><h2 class=\"font-sans font-medium text-label uppercase tracking-label text-accent border-b border-rule pb-2 mb-3\">Colour</h2>");
    page.push_str("<div class=\"grid grid-cols-2 sm:grid-cols-3 gap-2 mb-6\">");
    for name in COLOUR_ORDER {
        page.push_str(&format!(
            "<div class=\"bg-card border border-rule rounded-sm overflow-hidden\">\
             <div class=\"h-12\" style=\"background-color:var({name})\"></div>\
             <div class=\"p-2\"><div class=\"text-read font-medium\">{}</div>\
             <div class=\"text-read text-ink-2\">{}</div></div></div>",
            escape(name),
            escape(&lookup(name))
        ));
    }
    page.push_str("</div></section>");

    page.push_str("<section><h2 class=\"font-sans font-medium text-label uppercase tracking-label text-accent border-b border-rule pb-2 mb-3\">Type scale</h2>");
    for (name, line_height, letter_spacing) in &type_sizes {
        let mut style = format!("font-size:var({name});");
        if let Some(lh) = line_height {
            style.push_str(&format!("line-height:var({lh});"));
        }
        if let Some(ls) = letter_spacing {
            style.push_str(&format!("letter-spacing:var({ls});"));
        }
        page.push_str(&format!(
            "<div class=\"mb-4\"><p style=\"{style}\">The quick brown fox · Le renard brun saute par-dessus le chien paresseux</p>\
             <p class=\"text-read text-ink-2\">{} · {}px{}{}</p></div>",
            escape(name),
            lookup(name).trim_end_matches("px"),
            line_height
                .as_ref()
                .map(|v| format!(" / {}", lookup(v)))
                .unwrap_or_default(),
            letter_spacing
                .as_ref()
                .map(|v| format!(" · {}", lookup(v)))
                .unwrap_or_default(),
        ));
    }

    page.push_str("<p class=\"font-display font-semibold mb-4\" style=\"font-family:var(--font-display)\">Zen Old Mincho sets what a person wrote — steps, titles, covers.</p>");
    page.push_str("<p class=\"mb-6\">Zen Kaku Gothic New sets the interface — 400 regular, 500 medium, 600 resolves to the real 700, 700 bold.</p></section>");

    page.push_str("<section><h2 class=\"font-sans font-medium text-label uppercase tracking-label text-accent border-b border-rule pb-2 mb-3\">Spacing</h2>");
    for (name, value) in &spacing {
        page.push_str(&format!(
            "<div class=\"flex items-center gap-3 mb-2\">\
             <span class=\"w-16 text-read text-ink-2\">{}</span>\
             <span class=\"h-4 bg-accent\" style=\"width:var({name})\"></span>\
             <span class=\"text-read text-ink-2\">{}</span></div>",
            escape(name),
            escape(value)
        ));
    }
    page.push_str("<p class=\"text-read text-ink-2 mt-2\">The screen gutter is --spacing-gutter. No other step exists to reach for.</p></section>");

    page.push_str("<section><h2 class=\"font-sans font-medium text-label uppercase tracking-label text-accent border-b border-rule pb-2 mb-3 mt-6\">Shape</h2><div class=\"flex items-end gap-4 mb-4\">");
    for name in RADIUS_ORDER {
        page.push_str(&format!(
            "<div class=\"text-center\"><div class=\"h-14 w-14 bg-card border-2 border-accent\" style=\"border-radius:var({name})\"></div>\
             <div class=\"text-read text-ink-2 mt-1\">{}<br>{}</div></div>",
            escape(name.trim_start_matches("--radius-")),
            escape(&lookup(name))
        ));
    }
    page.push_str("</div><p class=\"text-read text-ink-2\">Near-square is the direction: ");
    page.push_str(
        &RADIUS_ORDER
            .iter()
            .map(|r| lookup(r))
            .collect::<Vec<_>>()
            .join(", "),
    );
    page.push_str(" everywhere, including buttons and fields.</p>");

    page.push_str(&format!(
        "<div class=\"flex gap-4 mt-4\">\
         <div class=\"bg-card border border-rule rounded-sm p-3\"><div class=\"text-label uppercase tracking-label text-ink-2 mb-1\">Tile width</div>\
         <div class=\"bg-ground-2\" style=\"width:var(--tile-w);height:calc(var(--tile-w)*4/5)\"></div>\
         <div class=\"text-read text-ink-2 mt-1\">--tile-w · {}</div></div>\
         <div class=\"bg-card border border-rule rounded-sm p-3\"><div class=\"text-label uppercase tracking-label text-ink-2 mb-1\">Hero height</div>\
         <div class=\"bg-ground-2\" style=\"height:var(--hero-h);width:96px\"></div>\
         <div class=\"text-read text-ink-2 mt-1\">--hero-h · {}</div></div>\
         </div></section>",
        escape(&lookup("--tile-w")),
        escape(&lookup("--hero-h"))
    ));

    page.push_str("</main>\n</body>\n</html>\n");
    page
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;")
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(vars.get("--text-step").map(String::as_str), Some("33px"));
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
        let colours = COLOUR_ORDER
            .iter()
            .filter(|n| vars.contains_key(**n))
            .count();
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
            steps, 10,
            "step, title, line, list-title, shelf-heading, tile-title, panel-figure, body, read, label"
        );
    }
}
