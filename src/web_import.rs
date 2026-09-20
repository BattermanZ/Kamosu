//! The web link importer (#70): a thin, Rust-native schema.org JSON-LD reader,
//! written against the seventeen real-world shapes catalogued in
//! `docs/research/web-link-recipe-ingestion.md`, plus the outbound fetch that
//! feeds it. `recipe-scrapers` is not adopted (45% of its 637 site files are
//! empty stubs, ADR 0025); there is no LLM fallback in v1.
//!
//! Two concerns live here on purpose, and only one of them touches the
//! network: [`extract_recipe`] is a pure function over already-fetched HTML,
//! tested directly against inline fixtures for all seventeen shapes with no
//! server involved. [`fetch_page`] and [`fetch_photo`] are the guarded
//! fetches (ADR 0033) — every outbound byte Kamosu reads for this feature
//! passes through them, and through no other HTTP client.

use std::time::Duration;

use reqwest_ssrf_guard::Acl;
use scraper::{Html, Selector};
use serde_json::Value;
use url::Url;

use crate::core::OpError;
use crate::entities::decode_entities;

/// Kamosu never pretends to be a browser (ADR 0033, ADR 0015).
const USER_AGENT: &str = "Kamosu/1.0";
/// A size a server declares is a claim, not a fact — this is what is read,
/// whatever `Content-Length` says.
const MAX_HTML_BYTES: usize = 5 * 1024 * 1024;
/// The picture cap lives with the pictures: `photographs` owns it, and a
/// picture fetched from a URL is held to the same number as one handed over
/// directly (ADR 0033).
use crate::photographs::MAX_PICTURE_BYTES as MAX_IMAGE_BYTES;
const FETCH_TIMEOUT_SECS: u64 = 10;
const MAX_REDIRECTS: usize = 5;

/// One dish read off a page: everything [`extract_recipe`] could tell, mapped
/// as closely as schema.org's own field names allow. Kamosu's Recipe content
/// schema fields it lands on live in `src/core.rs::parse_recipe_content` and
/// `src/catalogue.rs::recipe_content_properties` — this type mirrors only
/// what an importer needs to decide, not the stored shape itself.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedRecipe {
    pub title: String,
    /// Whether a Recipe node was actually found, as opposed to the
    /// no-structured-data fallback (title and Source alone, ADR 0025). Read
    /// by no production caller — `import_web_link` treats both cases the
    /// same, landing whatever was found — but kept so the fallback path
    /// itself is directly assertable in tests, rather than inferred from
    /// every other field happening to be empty.
    pub found_recipe: bool,
    pub yield_amount_noun: Option<(String, String)>,
    pub prep_time_minutes: Option<i64>,
    pub cook_time_minutes: Option<i64>,
    pub ingredients: Vec<String>,
    pub steps: Vec<StepEntry>,
    /// The attribution text for Source — an author's name, a publisher's
    /// name, or the page's own hostname, whichever the page actually gave.
    pub source_text: String,
    /// A candidate photo URL, already screened against looking like a
    /// favicon (ADR 0025) — never itself fetched by this function.
    pub image_url: Option<String>,
    /// Present only where the page's own structured data gave a number
    /// (ADR 0025). `import_web_link` lands it on the recipe as its Nutrition
    /// figure, per serving — schema.org defines `NutritionInformation` that
    /// way — and lands nothing at all where this is `None` (#72).
    pub nutrition_calories: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StepEntry {
    Section(String),
    Step(String),
}

/// Read a Recipe out of an already-fetched page. Never fails: a page with no
/// usable structured data still yields a title (the page's own `<title>`,
/// falling back to the URL itself) and a Source link, per ADR 0025 — an
/// importer reports what it found, never an error for a bare-name recipe.
pub fn extract_recipe(html: &str, page_url: &str) -> ParsedRecipe {
    let document = Html::parse_document(html);
    let hostname = Url::parse(page_url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string))
        .unwrap_or_else(|| page_url.to_string());

    match find_recipe_node(&document) {
        Some(node) => from_recipe_node(&node, &hostname),
        None => ParsedRecipe {
            title: page_title(&document).unwrap_or_else(|| page_url.to_string()),
            found_recipe: false,
            yield_amount_noun: None,
            prep_time_minutes: None,
            cook_time_minutes: None,
            ingredients: Vec::new(),
            steps: Vec::new(),
            source_text: hostname,
            image_url: None,
            nutrition_calories: None,
        },
    }
}

/// A URL that names a favicon rather than a photograph — a site's tiny icon
/// mistakenly surfaced as a recipe's `image`. Dropped rather than fetched
/// (ADR 0025): seven identical site icons must never enter the library.
fn looks_like_favicon(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.contains("favicon") || lower.ends_with(".ico")
}

fn page_title(document: &Html) -> Option<String> {
    let selector = Selector::parse("title").ok()?;
    let text: String = document
        .select(&selector)
        .next()?
        .text()
        .collect::<String>()
        .trim()
        .to_string();
    if text.is_empty() { None } else { Some(text) }
}

/// Walk every `<script type="application/ld+json">` block looking for the
/// first node whose `@type` names `Recipe` — a bare object, a member of an
/// `@type` array, or nested inside `@graph` (Yoast's shape). One malformed
/// block never stops the search: real pages carry several ld+json blocks
/// from unrelated plugins, and only one needs to parse.
fn find_recipe_node(document: &Html) -> Option<Value> {
    let selector = Selector::parse(r#"script[type="application/ld+json"]"#).ok()?;
    for script in document.select(&selector) {
        let text: String = script.text().collect();
        let Ok(parsed) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        if let Some(node) = search_for_recipe(&parsed) {
            return Some(node);
        }
    }
    None
}

fn search_for_recipe(value: &Value) -> Option<Value> {
    match value {
        Value::Array(items) => items.iter().find_map(search_for_recipe),
        Value::Object(map) => {
            if is_recipe_type(map.get("@type")) {
                return Some(value.clone());
            }
            if let Some(graph) = map.get("@graph") {
                return search_for_recipe(graph);
            }
            None
        }
        _ => None,
    }
}

fn is_recipe_type(type_field: Option<&Value>) -> bool {
    match type_field {
        Some(Value::String(name)) => name.eq_ignore_ascii_case("recipe"),
        Some(Value::Array(names)) => names
            .iter()
            .any(|n| n.as_str().is_some_and(|n| n.eq_ignore_ascii_case("recipe"))),
        _ => false,
    }
}

fn from_recipe_node(node: &Value, hostname: &str) -> ParsedRecipe {
    let title = text_field(node, "name").unwrap_or_else(|| hostname.to_string());
    let ingredients = string_array(node.get("recipeIngredient"));
    let steps = extract_steps(node.get("recipeInstructions"));
    let image_url = first_string_or_field(node.get("image"), &["url", "contentUrl"])
        .filter(|u| !looks_like_favicon(u));
    let source_text = first_string_or_field(node.get("author"), &["name"])
        .or_else(|| first_string_or_field(node.get("publisher"), &["name"]))
        .unwrap_or_else(|| hostname.to_string());

    // Decoded once, here, at the one place every text field this function
    // reads converges — never on a URL, which is never HTML-entity-encoded
    // in schema.org's own use of it. A templating system that ran a field
    // through an HTML-escaping function before writing the JSON-LD (a real
    // failure mode on some WordPress recipe plugins) leaves `&amp;` sitting
    // inside an otherwise-valid JSON string; this undoes exactly that.
    ParsedRecipe {
        title: decode_entities(&title),
        found_recipe: true,
        yield_amount_noun: extract_yield(node.get("recipeYield"))
            .map(|(amount, noun)| (amount, decode_entities(&noun))),
        prep_time_minutes: duration_field(node, "prepTime"),
        cook_time_minutes: duration_field(node, "cookTime"),
        ingredients: ingredients.iter().map(|s| decode_entities(s)).collect(),
        steps: steps.into_iter().map(decode_step_entities).collect(),
        source_text: decode_entities(&source_text),
        image_url,
        nutrition_calories: extract_calories(node.get("nutrition")),
    }
}

fn decode_step_entities(step: StepEntry) -> StepEntry {
    match step {
        StepEntry::Section(text) => StepEntry::Section(decode_entities(&text)),
        StepEntry::Step(text) => StepEntry::Step(decode_entities(&text)),
    }
}

fn text_field(node: &Value, field: &str) -> Option<String> {
    node.get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
        Some(Value::String(text)) => vec![text.trim().to_string()],
        _ => Vec::new(),
    }
}

/// `recipeInstructions` arrives as a single string, a flat array of strings
/// or `HowToStep` objects, or a mix of `HowToStep` and `HowToSection` — the
/// nastiest real-world shape, because an unhandled `HowToSection` looks like
/// a recipe with zero steps rather than an error. Flattened here into
/// Kamosu's own section-or-step step list.
fn extract_steps(value: Option<&Value>) -> Vec<StepEntry> {
    match value {
        Some(Value::String(text)) => text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| StepEntry::Step(line.to_string()))
            .collect(),
        Some(Value::Array(items)) => items.iter().flat_map(extract_step_item).collect(),
        _ => Vec::new(),
    }
}

fn extract_step_item(item: &Value) -> Vec<StepEntry> {
    match item {
        Value::String(text) => vec![StepEntry::Step(text.trim().to_string())],
        Value::Object(_) => {
            let is_section = is_type_named(item, "HowToSection");
            if is_section {
                let mut out = Vec::new();
                if let Some(name) = text_field(item, "name") {
                    out.push(StepEntry::Section(name));
                }
                if let Some(Value::Array(children)) = item.get("itemListElement") {
                    out.extend(children.iter().flat_map(extract_step_item));
                }
                out
            } else {
                // A HowToStep's own text lives in `text`, falling back to
                // `name` for the rare page that only gave that.
                text_field(item, "text")
                    .or_else(|| text_field(item, "name"))
                    .map(StepEntry::Step)
                    .into_iter()
                    .collect()
            }
        }
        _ => Vec::new(),
    }
}

fn is_type_named(value: &Value, name: &str) -> bool {
    match value.get("@type") {
        Some(Value::String(t)) => t.eq_ignore_ascii_case(name),
        Some(Value::Array(ts)) => ts
            .iter()
            .any(|t| t.as_str().is_some_and(|t| t.eq_ignore_ascii_case(name))),
        _ => false,
    }
}

/// A value that arrives as a bare string, an object carrying one of `fields`,
/// or an array of either — take the first usable one. One shape covers
/// `image` (`url`/`contentUrl`), `author` and `publisher` (`name`), since
/// schema.org lets an author and a publisher take exactly the same shapes.
fn first_string_or_field(value: Option<&Value>, fields: &[&str]) -> Option<String> {
    match value? {
        Value::String(text) => Some(text.trim().to_string()).filter(|s| !s.is_empty()),
        Value::Object(_) => fields.iter().find_map(|field| text_field(value?, field)),
        Value::Array(items) => items
            .iter()
            .find_map(|item| first_string_or_field(Some(item), fields)),
        _ => None,
    }
}

/// `recipeYield` arrives as free text ("4 servings"), a bare number, or an
/// array of either — take the first. A page that gave no noun word at all
/// (a bare "4") is assumed to mean servings, the near-universal convention;
/// anything the page did write is kept exactly as written (ADR 0002's habit,
/// applied to Yield).
fn extract_yield(value: Option<&Value>) -> Option<(String, String)> {
    let raw = match value? {
        Value::String(text) => text.trim().to_string(),
        Value::Number(n) => n.to_string(),
        Value::Array(items) => return items.iter().find_map(|item| extract_yield(Some(item))),
        Value::Object(_) => {
            let amount = value?.get("value").and_then(|v| {
                v.as_str()
                    .map(str::to_string)
                    .or_else(|| v.as_i64().map(|n| n.to_string()))
            })?;
            let noun = text_field(value?, "unitText").unwrap_or_else(|| "servings".to_string());
            return Some((amount, noun));
        }
        _ => return None,
    };
    if raw.is_empty() {
        return None;
    }
    let split_at = raw.find(|c: char| !c.is_ascii_digit() && c != '.' && c != ',');
    match split_at {
        Some(0) => Some((raw.clone(), "servings".to_string())),
        Some(i) => {
            let (amount, noun) = raw.split_at(i);
            let noun = noun.trim();
            if noun.is_empty() {
                Some((amount.to_string(), "servings".to_string()))
            } else {
                Some((amount.to_string(), noun.to_string()))
            }
        }
        None => Some((raw, "servings".to_string())),
    }
}

/// `prepTime`/`cookTime` are ISO 8601 durations (`PT12M`) in practice, not
/// just in the schema.org spec (confirmed against a live bbcgoodfood.com
/// fetch, `docs/research/web-link-recipe-ingestion.md`). A duration naming
/// years or months has no fixed length and is silently skipped rather than
/// guessed at — never invented, matching how nutrition is handled.
fn duration_field(node: &Value, field: &str) -> Option<i64> {
    let text = text_field(node, field)?;
    let duration: iso8601_duration::Duration = text.parse().ok()?;
    let std_duration = duration.to_std()?;
    Some((std_duration.as_secs() / 60) as i64)
}

/// `nutrition.calories` arrives as a string with the unit fused in
/// (`"308 calories"`, matching Mealie's own all-string nutrition fields) —
/// pull the leading number and nothing else. Absent entirely when the page
/// gave no number, never computed or guessed (ADR 0025).
fn extract_calories(value: Option<&Value>) -> Option<f64> {
    let calories = text_field(value?, "calories")?;
    leading_number(&calories)
}

fn leading_number(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    let end = trimmed
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit() && *c != '.')
        .map_or(trimmed.len(), |(i, _)| i);
    if end == 0 {
        return None;
    }
    trimmed[..end].parse().ok()
}

/// Test-only escape hatch, compiled only under the `test-jobs` feature
/// (`just test`'s flag; `just docker-build` never sets it, so a release
/// binary carries neither the flag nor this code at all): lets the
/// behaviour suite's own in-process HTTP server stand in for "the internet"
/// so the fetch-and-extract pipeline can be exercised through real
/// Operations. Not a general "trust the LAN" switch — ADR 0033 rejected
/// exactly that — only the loopback address itself is ever exempted, and
/// only once a test explicitly asks; every other private address, including
/// the one used to prove a redirect is refused, stays denied regardless.
#[cfg(feature = "test-jobs")]
static TEST_ALLOW_LOOPBACK: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(feature = "test-jobs")]
pub fn allow_loopback_fetches_for_tests() {
    TEST_ALLOW_LOOPBACK.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// The one Access Control List every outbound fetch for this feature shares:
/// public addresses only, at the address actually dialled and at every
/// redirect (ADR 0033). A fresh `Acl` is cheap to build (it is a small
/// composition of rules, not a lookup), so each fetch builds its own rather
/// than reaching for shared mutable state.
fn guarded_acl() -> Acl {
    let acl = Acl::new().deny_local_network();
    #[cfg(feature = "test-jobs")]
    let acl = if TEST_ALLOW_LOOPBACK.load(std::sync::atomic::Ordering::SeqCst) {
        acl.allow_ip_when(|ip| ip.is_loopback())
    } else {
        acl
    };
    acl
}

fn guarded_client(acl: &Acl) -> Result<reqwest::Client, OpError> {
    let for_redirects = acl.clone();
    let redirect_policy = reqwest::redirect::Policy::custom(move |attempt| {
        // `previous()` names the hops already taken before this one is
        // considered, so refusing at `> MAX_REDIRECTS` — not `>=` — is what
        // lets exactly `MAX_REDIRECTS` redirects through and gives up on the
        // next: confirmed empirically against a real redirect chain, since
        // the crate's own docs don't spell out where the boundary falls.
        if attempt.previous().len() > MAX_REDIRECTS {
            return attempt.error("too many redirects");
        }
        match for_redirects.validate_url(attempt.url()) {
            Ok(()) => attempt.follow(),
            Err(err) => attempt.error(err.to_string()),
        }
    });
    acl.configure(reqwest::Client::builder())
        .redirect(redirect_policy)
        .timeout(Duration::from_secs(FETCH_TIMEOUT_SECS))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| OpError::internal(format!("cannot build the guarded HTTP client: {e}")))
}

fn parse_fetchable_url(url: &str) -> Result<Url, OpError> {
    let parsed = Url::parse(url).map_err(|_| OpError::bad_request("that is not a valid URL"))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(OpError::bad_request(
            "only http:// and https:// links can be imported",
        ));
    }
    Ok(parsed)
}

async fn read_capped(mut response: reqwest::Response, cap: usize) -> Result<Vec<u8>, OpError> {
    let mut buf = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| OpError::bad_request(format!("cannot read that response: {e}")))?
    {
        buf.extend_from_slice(&chunk);
        if buf.len() > cap {
            return Err(OpError::bad_request(
                "that page's response was larger than Kamosu will read",
            ));
        }
    }
    Ok(buf)
}

/// The one guarded connect-and-answer shared by [`fetch_page`] and
/// [`fetch_photo`]: validate the address, build the client, send the GET,
/// and cap what is read — `noun` names what a failure sentence calls the
/// thing being fetched ("that page", "that photo").
///
/// Blocks the current thread on the async client via `Handle::current()`,
/// which only works from inside `tokio::task::spawn_blocking` — exactly
/// where a Job handler always runs (`jobs.rs::carry`). Both callers here are
/// Job handlers reached only through `import_web_link` (`Kind::Job` in the
/// Catalogue); an `Immediate` Operation calling either would panic, which is
/// why neither is wired as one.
fn guarded_get(url: &str, noun: &str, cap: usize) -> Result<(Vec<u8>, String), OpError> {
    let parsed = parse_fetchable_url(url)?;
    let acl = guarded_acl();
    acl.validate_url(&parsed)
        .map_err(|e| OpError::bad_request(format!("that address cannot be fetched: {e}")))?;
    let client = guarded_client(&acl)?;

    tokio::runtime::Handle::current().block_on(async move {
        let response = client
            .get(parsed)
            .send()
            .await
            .map_err(|e| OpError::bad_request(format!("cannot reach {noun}: {e}")))?;
        if !response.status().is_success() {
            return Err(OpError::bad_request(format!(
                "{noun} answered with {}",
                response.status()
            )));
        }
        let effective_url = response.url().to_string();
        let bytes = read_capped(response, cap).await?;
        Ok((bytes, effective_url))
    })
}

/// Fetch a page's HTML, bound to public addresses at the dialled address and
/// at every redirect (ADR 0033). Answers the HTML and the address it actually
/// ended up at once redirects are followed — the effective URL, used as the
/// import ledger's foreign id so a re-import of the same page matches (#68).
pub fn fetch_page(url: &str) -> Result<(String, String), OpError> {
    let (bytes, effective_url) = guarded_get(url, "that page", MAX_HTML_BYTES)?;
    Ok((String::from_utf8_lossy(&bytes).into_owned(), effective_url))
}

/// Fetch a candidate photo's bytes, under the same guard as the page itself.
pub fn fetch_photo(url: &str) -> Result<Vec<u8>, OpError> {
    let (bytes, _effective_url) = guarded_get(url, "that photo", MAX_IMAGE_BYTES)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ld(json: &str) -> String {
        format!(
            "<html><head><script type=\"application/ld+json\">{json}</script></head><body></body></html>"
        )
    }

    // 1. `@graph` nesting (Yoast).
    #[test]
    fn graph_nesting() {
        let html = ld(r#"{"@context":"https://schema.org","@graph":[
            {"@type":"WebSite","name":"Not a recipe"},
            {"@type":"Recipe","name":"Yoast Cookies","recipeIngredient":["1 cup flour"]}
        ]}"#);
        let parsed = extract_recipe(&html, "https://example.com/r");
        assert!(parsed.found_recipe);
        assert_eq!(parsed.title, "Yoast Cookies");
        assert_eq!(parsed.ingredients, vec!["1 cup flour"]);
    }

    // 2. `@type` as an array.
    #[test]
    fn type_as_array() {
        let html = ld(r#"{"@type":["Recipe","NewsArticle"],"name":"Array Typed"}"#);
        let parsed = extract_recipe(&html, "https://example.com/r");
        assert!(parsed.found_recipe);
        assert_eq!(parsed.title, "Array Typed");
    }

    // 3. Top-level array of nodes.
    #[test]
    fn top_level_array() {
        let html = ld(r#"[{"@type":"WebSite"},{"@type":"Recipe","name":"From Array"}]"#);
        let parsed = extract_recipe(&html, "https://example.com/r");
        assert!(parsed.found_recipe);
        assert_eq!(parsed.title, "From Array");
    }

    // 4. Several ld+json blocks, one malformed.
    #[test]
    fn skips_malformed_block_keeps_looking() {
        let html = "<html><head>\
             <script type=\"application/ld+json\">{ this is not json </script>\
             <script type=\"application/ld+json\">{\"@type\":\"Recipe\",\"name\":\"Survivor\"}</script>\
             </head><body></body></html>";
        let parsed = extract_recipe(html, "https://example.com/r");
        assert!(parsed.found_recipe);
        assert_eq!(parsed.title, "Survivor");
    }

    // 5. No Recipe on the page at all — the ADR 0025 fallback.
    #[test]
    fn no_recipe_falls_back_to_title_and_source() {
        let html = "<html><head><title>Just A Blog Post</title>\
                     <script type=\"application/ld+json\">{\"@type\":\"WebSite\"}</script>\
                     </head><body></body></html>";
        let parsed = extract_recipe(html, "https://example.com/blog/post");
        assert!(!parsed.found_recipe);
        assert_eq!(parsed.title, "Just A Blog Post");
        assert_eq!(parsed.source_text, "example.com");
        assert!(parsed.ingredients.is_empty());
        assert!(parsed.steps.is_empty());
    }

    // 6. `recipeInstructions` as `HowToStep[]`.
    #[test]
    fn instructions_as_howto_steps() {
        let html = ld(r#"{"@type":"Recipe","name":"Steps","recipeInstructions":[
                {"@type":"HowToStep","text":"Mix"},
                {"@type":"HowToStep","text":"Bake"}
            ]}"#);
        let parsed = extract_recipe(&html, "https://example.com/r");
        assert_eq!(
            parsed.steps,
            vec![
                StepEntry::Step("Mix".into()),
                StepEntry::Step("Bake".into())
            ]
        );
    }

    // 7. `recipeInstructions` as a single string.
    #[test]
    fn instructions_as_single_string() {
        let html = ld(
            r#"{"@type":"Recipe","name":"One Line","recipeInstructions":"Mix everything and bake."}"#,
        );
        let parsed = extract_recipe(&html, "https://example.com/r");
        assert_eq!(
            parsed.steps,
            vec![StepEntry::Step("Mix everything and bake.".into())]
        );
    }

    // 8. `recipeInstructions` with `HowToSection` — the nastiest shape.
    #[test]
    fn instructions_with_sections_are_flattened() {
        let html = ld(
            r#"{"@type":"Recipe","name":"Sectioned","recipeInstructions":[
                {"@type":"HowToSection","name":"The dough","itemListElement":[
                    {"@type":"HowToStep","text":"Knead"}
                ]},
                {"@type":"HowToSection","name":"The filling","itemListElement":[
                    {"@type":"HowToStep","text":"Chop"},
                    {"@type":"HowToStep","text":"Mix"}
                ]}
            ]}"#,
        );
        let parsed = extract_recipe(&html, "https://example.com/r");
        assert_eq!(
            parsed.steps,
            vec![
                StepEntry::Section("The dough".into()),
                StepEntry::Step("Knead".into()),
                StepEntry::Section("The filling".into()),
                StepEntry::Step("Chop".into()),
                StepEntry::Step("Mix".into()),
            ]
        );
    }

    // 9. `image` as an object with `url`.
    #[test]
    fn image_as_object_with_url() {
        let html = ld(
            r#"{"@type":"Recipe","name":"Pic","image":{"@type":"ImageObject","url":"https://example.com/dish.jpg"}}"#,
        );
        let parsed = extract_recipe(&html, "https://example.com/r");
        assert_eq!(
            parsed.image_url.as_deref(),
            Some("https://example.com/dish.jpg")
        );
    }

    // 10. `image` as a bare string.
    #[test]
    fn image_as_bare_string() {
        let html = ld(r#"{"@type":"Recipe","name":"Pic","image":"https://example.com/dish.jpg"}"#);
        let parsed = extract_recipe(&html, "https://example.com/r");
        assert_eq!(
            parsed.image_url.as_deref(),
            Some("https://example.com/dish.jpg")
        );
    }

    // 11. `image` as an array.
    #[test]
    fn image_as_array_takes_first() {
        let html = ld(
            r#"{"@type":"Recipe","name":"Pic","image":["https://example.com/one.jpg","https://example.com/two.jpg"]}"#,
        );
        let parsed = extract_recipe(&html, "https://example.com/r");
        assert_eq!(
            parsed.image_url.as_deref(),
            Some("https://example.com/one.jpg")
        );
    }

    // The favicon-drop rule for `image`.
    #[test]
    fn favicon_image_is_dropped() {
        let html = ld(
            r#"{"@type":"Recipe","name":"No Real Photo","image":"https://example.com/favicon.ico"}"#,
        );
        let parsed = extract_recipe(&html, "https://example.com/r");
        assert_eq!(parsed.image_url, None);
    }

    // 12. `recipeYield` as an array.
    #[test]
    fn yield_as_array() {
        let html = ld(r#"{"@type":"Recipe","name":"Y","recipeYield":["4 servings","4"]}"#);
        let parsed = extract_recipe(&html, "https://example.com/r");
        assert_eq!(
            parsed.yield_amount_noun,
            Some(("4".to_string(), "servings".to_string()))
        );
    }

    // 13. `recipeYield` as a bare number.
    #[test]
    fn yield_as_number() {
        let html = ld(r#"{"@type":"Recipe","name":"Y","recipeYield":6}"#);
        let parsed = extract_recipe(&html, "https://example.com/r");
        assert_eq!(
            parsed.yield_amount_noun,
            Some(("6".to_string(), "servings".to_string()))
        );
    }

    // recipeYield as free text with its own noun, kept as written.
    #[test]
    fn yield_as_text_with_noun() {
        let html = ld(r#"{"@type":"Recipe","name":"Y","recipeYield":"24 cookies"}"#);
        let parsed = extract_recipe(&html, "https://example.com/r");
        assert_eq!(
            parsed.yield_amount_noun,
            Some(("24".to_string(), "cookies".to_string()))
        );
    }

    // 14. `recipeCategory` as an array — must not crash the extractor, even
    // though Kamosu has no field to land it on in v1.
    #[test]
    fn category_as_array_does_not_crash() {
        let html = ld(r#"{"@type":"Recipe","name":"Cat","recipeCategory":["Dessert","Snack"]}"#);
        let parsed = extract_recipe(&html, "https://example.com/r");
        assert_eq!(parsed.title, "Cat");
    }

    // 15. `author` as a single object.
    #[test]
    fn author_as_object() {
        let html =
            ld(r#"{"@type":"Recipe","name":"A","author":{"@type":"Person","name":"Jane Cook"}}"#);
        let parsed = extract_recipe(&html, "https://example.com/r");
        assert_eq!(parsed.source_text, "Jane Cook");
    }

    // 16. ISO-8601 durations.
    #[test]
    fn iso8601_durations() {
        let html = ld(r#"{"@type":"Recipe","name":"D","prepTime":"PT20M","cookTime":"PT12M"}"#);
        let parsed = extract_recipe(&html, "https://example.com/r");
        assert_eq!(parsed.prep_time_minutes, Some(20));
        assert_eq!(parsed.cook_time_minutes, Some(12));
    }

    // A genuine (non-entity) special character in ingredient text survives
    // JSON-LD's own string escaping intact, untouched by entity decoding.
    #[test]
    fn ingredient_text_is_preserved_verbatim() {
        let html =
            ld(r#"{"@type":"Recipe","name":"E","recipeIngredient":["¼ tsp ground nutmeg"]}"#);
        let parsed = extract_recipe(&html, "https://example.com/r");
        assert_eq!(parsed.ingredients, vec!["\u{bc} tsp ground nutmeg"]);
    }

    // 17. HTML entities in ingredient text — a real failure mode where a
    // templating system ran a field through an HTML-escaping function before
    // writing the JSON-LD, leaving `&amp;` sitting inside an otherwise-valid
    // JSON string (`<script>` content is raw text under the HTML5 spec, so
    // this is never undone by ordinary HTML parsing — it must be decoded).
    #[test]
    fn html_entities_in_ingredient_text_are_decoded() {
        let html = ld(
            r#"{"@type":"Recipe","name":"Salt &amp; Pepper","recipeIngredient":["salt &amp; pepper","2 cups flour &#39;00&#39;"]}"#,
        );
        let parsed = extract_recipe(&html, "https://example.com/r");
        assert_eq!(parsed.title, "Salt & Pepper");
        assert_eq!(
            parsed.ingredients,
            vec!["salt & pepper", "2 cups flour '00'"]
        );
    }

    #[test]
    fn a_lone_ampersand_with_no_matching_entity_is_kept_literally() {
        let html = ld(r#"{"@type":"Recipe","name":"R&D Kitchen","recipeIngredient":["A & B"]}"#);
        let parsed = extract_recipe(&html, "https://example.com/r");
        assert_eq!(parsed.title, "R&D Kitchen");
        assert_eq!(parsed.ingredients, vec!["A & B"]);
    }

    // Nutrition: taken only where the source gave a number.
    #[test]
    fn nutrition_calories_parsed_when_present() {
        let html = ld(
            r#"{"@type":"Recipe","name":"N","nutrition":{"@type":"NutritionInformation","calories":"308 calories"}}"#,
        );
        let parsed = extract_recipe(&html, "https://example.com/r");
        assert_eq!(parsed.nutrition_calories, Some(308.0));
    }

    #[test]
    fn nutrition_absent_stays_absent() {
        let html = ld(r#"{"@type":"Recipe","name":"N"}"#);
        let parsed = extract_recipe(&html, "https://example.com/r");
        assert_eq!(parsed.nutrition_calories, None);
    }

    // A real fetch of bbcgoodfood.com's own JSON-LD shape (captured in the
    // research doc) — the fixture the recommendation itself was built from.
    #[test]
    fn bbcgoodfood_shape() {
        let html = ld(r#"{"@type":"Recipe","name":"Chocolate chunk cookies",
            "recipeIngredient":["120g butter softened","75g light brown sugar"],
            "cookTime":"PT12M","prepTime":"PT20M",
            "nutrition":{"@type":"NutritionInformation","calories":"308 calories"}}"#);
        let parsed = extract_recipe(
            &html,
            "https://www.bbcgoodfood.com/recipes/chocolate-chunk-cookies",
        );
        assert_eq!(parsed.title, "Chocolate chunk cookies");
        assert_eq!(parsed.ingredients.len(), 2);
        assert_eq!(parsed.prep_time_minutes, Some(20));
        assert_eq!(parsed.cook_time_minutes, Some(12));
        assert_eq!(parsed.nutrition_calories, Some(308.0));
    }

    #[test]
    fn favicon_url_detection() {
        assert!(looks_like_favicon("https://example.com/favicon.ico"));
        assert!(looks_like_favicon(
            "https://example.com/assets/favicon-32.png"
        ));
        assert!(!looks_like_favicon("https://example.com/photos/dish.jpg"));
    }

    /// The guard itself, checked directly against known public and private
    /// addresses (ADR 0033) — no server needed for this part.
    #[test]
    fn the_guard_denies_private_addresses_and_allows_public_ones() {
        let acl = guarded_acl();
        for bad in [
            "http://127.0.0.1/",
            "http://10.0.0.1/",
            "http://192.168.1.1/",
            "http://169.254.169.254/", // cloud metadata endpoint
            "http://[::1]/",
        ] {
            let url = Url::parse(bad).unwrap();
            assert!(
                acl.validate_url(&url).is_err(),
                "{bad} should have been denied"
            );
        }
        let good = Url::parse("http://8.8.8.8/").unwrap();
        assert!(
            acl.validate_url(&good).is_ok(),
            "a public address should be allowed"
        );
    }

    /// A real HTTP round trip against a local server, proving the redirect
    /// *policy* itself — not just address classification — actually rejects
    /// a hop into a private address. The server's own loopback address is
    /// allow-listed purely so the request can be dialled at all; the redirect
    /// target is a different, still-denied private address, so this exercises
    /// exactly the gap ADR 0033 warns about: installing the resolver without
    /// the redirect policy (or vice versa) leaves a hole.
    #[tokio::test]
    async fn a_redirect_to_a_private_address_is_refused() {
        let router = axum::Router::new().route(
            "/start",
            axum::routing::get(|| async {
                axum::response::Redirect::temporary("http://10.0.0.1/private")
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        let acl = Acl::new()
            .deny_local_network()
            .allow_ip_when(|ip| ip.is_loopback());
        let client = guarded_client(&acl).unwrap();
        let url = format!("http://127.0.0.1:{}/start", addr.port());
        let result = client.get(&url).send().await;
        assert!(
            result.is_err(),
            "a redirect to a private address must be refused"
        );
    }

    /// Locks in the exact boundary ADR 0033 names ("5 redirects"): a chain of
    /// five hops must complete, and a sixth must be refused. Written after an
    /// off-by-one (`>=` instead of `>`) let only four hops through in an
    /// earlier version of this code — a real HTTP round trip, since the
    /// crate's own docs don't spell out whether `previous().len()` counts the
    /// hop about to be taken or the ones already behind it.
    #[tokio::test]
    async fn exactly_five_redirects_are_allowed_and_a_sixth_is_refused() {
        use axum::response::IntoResponse;

        async fn chain_of(hops: u32) -> String {
            let router = axum::Router::new().route(
                "/hop/{n}",
                axum::routing::get(
                    |axum::extract::Path(n): axum::extract::Path<u32>| async move {
                        if n == 0 {
                            axum::response::Html("done").into_response()
                        } else {
                            axum::response::Redirect::temporary(&format!("/hop/{}", n - 1))
                                .into_response()
                        }
                    },
                ),
            );
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            tokio::spawn(async move {
                axum::serve(listener, router).await.unwrap();
            });
            format!("http://127.0.0.1:{}/hop/{hops}", addr.port())
        }

        let acl = Acl::new()
            .deny_local_network()
            .allow_ip_when(|ip| ip.is_loopback());
        let client = guarded_client(&acl).unwrap();

        let five = client.get(chain_of(5).await).send().await;
        assert!(five.is_ok(), "5 redirects must be followed: {five:?}");

        let six = client.get(chain_of(6).await).send().await;
        assert!(six.is_err(), "a 6th redirect must be refused");
    }
}
