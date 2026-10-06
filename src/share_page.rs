//! The public Share Link page (#65, ADR 0026, ADR 0018).
//!
//! **This page is not an Operation.** It *consumes* them — `read_shared_recipe`
//! to show the recipe and `make_shared_sheet` to print it (#75), both Public
//! because holding the token is the whole of the permission — and renders what
//! comes back as plain HTML. Parity is therefore untouched: both Doors still
//! materialise exactly the Catalogue, and this is a third thing built on top of
//! it, the way the interface and the design tokens are.
//!
//! What it hands over as **bytes** is not an Operation either, because no
//! Operation answers bytes: the card (`Core::shared_card`) and the recipe file
//! (`Core::shared_bundle`, #66) are Core methods reached from this router
//! alone. That is the same shape as the photographs and the Sheet's PDF, and
//! the rule holds throughout — whether a stranger may be handed any of them is
//! answered by the Core, never decided here.
//!
//! It is rendered by the server rather than by the Svelte app because the
//! reader has no account, arrives from a messaging app, and must get a recipe
//! rather than a loading spinner (ADR 0012) — and because a card in a chat is
//! drawn from tags in the HTML a crawler is served, which a client-rendered
//! page has none of.
//!
//! What it shows was chosen by Aurélien on 30 August 2026 against three full
//! directions drawn on real recipes, recorded on #65 rather than repeated here.
//! What that settled:
//!
//!   · the page IS the card the message carried, grown to full size: one
//!     bordered card on paper-white over the recessed ground, with a line above
//!     it saying who sent it, and the Thread as a second card beneath —
//!     separated by air rather than a rule, because it is a different thing you
//!     were also given;
//!   · the hero is the app's own (#81): the photograph under the indigo wash
//!     with the title standing on it, and the Source above it on the wash. With
//!     no photograph the Cover leads and carries the title bare — its dye was
//!     chosen and needs no wash — so only the small Source line drops to paper;
//!   · the picture a messaging app fetches is **that same hero**, so there is
//!     no second piece of art to keep true;
//!   · a share names the **Person** who shared it and never the Kitchen. A
//!     Kitchen's Nickname is private to the member who set it and could not
//!     appear here in any case (GLOSSARY.md, "Kitchen").
//!
//! **No Attempt appears here in any form** (ADR 0005, ADR 0026). That is not a
//! filter this file applies; `read_shared_recipe` has no field for one.

use std::sync::Arc;

use axum::Router;
use axum::extract::Path;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde_json::Value;

use crate::core::{Core, OpError, SheetState};
use crate::cover::cover_for;
use crate::photographs::DisplaySize;

/// The routes the Share Link page owns. Merged beside the Doors at startup;
/// none of them is an Operation and none enters the Catalogue.
pub fn router(core: Arc<Core>) -> Router {
    let page_core = core.clone();
    let photo_core = core.clone();
    let card_core = core.clone();
    let translation_core = core.clone();
    let sheet_core = core.clone();
    let translated_sheet_core = core.clone();
    let waiting_core = core.clone();
    let bundle_core = core.clone();
    Router::new()
        .route(
            "/s/{token}",
            get(move |Path(token): Path<String>| {
                let core = page_core.clone();
                async move { page(&core, &token) }
            }),
        )
        // The photograph, reachable with the token and nothing else. A stranger
        // has no Credential, so the ordinary `/api/photographs/…` route cannot
        // serve them — and this one asks the Core whether the picture actually
        // belongs to the shared recipe, so a live token is not a key to every
        // photograph on the instance.
        .route(
            "/s/{token}/photo/{hash}",
            get(move |Path((token, hash)): Path<(String, String)>| {
                let core = photo_core.clone();
                async move { photo(&core, &token, &hash) }
            }),
        )
        // One of the shared Branch's Translations, under the same token (ADR
        // 0006, ADR 0018). A Translation is a Branch of its own, so it gets a
        // page of its own — but not a token of its own, which would be a
        // second link to end.
        .route(
            "/s/{token}/in/{language}",
            get(move |Path((token, language)): Path<(String, String)>| {
                let core = translation_core.clone();
                async move { translated(&core, &token, &language) }
            }),
        )
        // The Sheet (#75, ADR 0023). A stranger's page runs no script, so
        // asking for one is a link: it starts the Job and sends the browser to
        // the Job's own address, which is a short page that refreshes itself
        // until the PDF is ready and then is the PDF.
        .route(
            "/s/{token}/sheet",
            get(move |Path(token): Path<String>, headers: HeaderMap| {
                let core = sheet_core.clone();
                async move { ask_sheet(&core, &token, None, &headers) }
            }),
        )
        .route(
            "/s/{token}/in/{language}/sheet",
            get(
                move |Path((token, language)): Path<(String, String)>, headers: HeaderMap| {
                    let core = translated_sheet_core.clone();
                    async move { ask_sheet(&core, &token, Some(&language), &headers) }
                },
            ),
        )
        .route(
            "/s/{token}/sheet/{job_id}",
            get(move |Path((token, job_id)): Path<(String, String)>| {
                let core = waiting_core.clone();
                async move { sheet(&core, &token, &job_id) }
            }),
        )
        // The recipe file (#65, #66, ADR 0020). A link and not a button, for
        // the same reason the Sheet is one: this page runs no script. The
        // Credential-bearing route the app uses cannot serve a stranger, so the
        // token answers for itself here.
        .route(
            "/s/{token}/bundle",
            get(move |Path(token): Path<String>| {
                let core = bundle_core.clone();
                bundle(core, token)
            }),
        )
        .route(
            "/s/{token}/card",
            get(move |Path(token): Path<String>| {
                let core = card_core.clone();
                async move { card(&core, &token) }
            }),
        )
}

// ── The words ────────────────────────────────────────────────────────────────

/// The page's own words, in the three Languages Kamosu speaks.
///
/// Written here rather than compiled by Paraglide because this page is Rust and
/// Paraglide compiles to TypeScript. They are the only prose the server
/// renders; anything else belongs in the app, where Paraglide already checks
/// that all three Languages carry every phrase.
///
/// The Language chosen is the **recipe's**, not the reader's — a stranger
/// arriving from a link has told Kamosu nothing, and the one thing known about
/// what they are about to read is what it is written in.
struct Words {
    sent_you: &'static str,
    ingredients: &'static str,
    method: &'static str,
    from_source: &'static str,
    /// Said to a screen reader after the Source's name, where the line links
    /// to the recipe's original page (#152).
    source_opens: &'static str,
    also_in: &'static str,
    history: &'static str,
    /// The one way into a reader's own library (#170), for every reader alike.
    import: &'static str,
    bundle: &'static str,
    /// One line under the recipe file, saying what it is good for. A stranger
    /// can see the zip is a download; what they cannot see is that another
    /// Kamosu takes the same file in whole (ADR 0020).
    file_note: &'static str,
    sheet: &'static str,
    /// The page shown while a Sheet is being set (#75): a heading and a line.
    sheet_making: &'static str,
    sheet_making_body: &'static str,
    /// The page shown when a Sheet could not be made; the reason follows it.
    sheet_failed: &'static str,
    /// The page shown when the recipe file could not be handed over, which is
    /// nearly always an ended link; the reason follows it.
    file_failed: &'static str,
    written_down: &'static str,
    /// The standing line, in the same words every time (ADR 0018).
    standing: &'static str,
    /// "Shared by «name»" — the chat card's own text line, which is a phrase
    /// and therefore belongs here rather than being built in English at the
    /// point of use.
    shared_by: &'static str,
    ended_title: &'static str,
    ended_body: &'static str,
    min_prep: &'static str,
    min_cook: &'static str,
    /// The Nutrition figure at the foot of the Ingredients (#84), in its two
    /// bases. `{n}` is the number. A phrase rather than a label, because the
    /// figure has to say what it counts wherever it appears: 308 on its own
    /// says nothing, and a serving and 100 g do not convert into each other
    /// without a weight the recipe does not carry.
    kcal_serving: &'static str,
    kcal_100g: &'static str,
    /// The heading over a Component's own Steps (#50), beside `ingredients` and
    /// `method` because it is the same kind of thing: a label this page writes.
    ///
    /// A Component's own LINE is not here. That one is prose computed from the
    /// recipe's data — which recipe, and how much of it — so
    /// `read_shared_recipe` hands it over already worded, by
    /// `units::component_line`, in the recipe's own Language. Same rule as
    /// `measured` (#49): what is computed is worded once in the Core, what is a
    /// label follows each rendering's own convention.
    component_method: &'static str,
    /// "Its method is at the foot of the page ↓" — the row's link to the annexe.
    component_method_below: &'static str,
}

const EN: Words = Words {
    sent_you: "sent you this recipe",
    ingredients: "Ingredients",
    method: "Method",
    from_source: "From",
    source_opens: "opens the original page",
    also_in: "Also written in",
    history: "History",
    import: "Import this recipe",
    bundle: "Recipe file",
    file_note: "A zip of readable notes and photographs that opens anywhere. Another Kamosu \
                takes it in whole, with every version behind it.",
    sheet: "Print a sheet",
    sheet_making: "Setting your sheet",
    sheet_making_body: "This takes a moment. The sheet opens here by itself when it is ready.",
    sheet_failed: "The sheet could not be made",
    file_failed: "The recipe file could not be made",
    written_down: "Written down",
    standing: "This link carries every version of this recipe, back to the first. Ending it \
                stops new visitors, not copies already sent.",
    shared_by: "Shared by {name}",
    ended_title: "This link was ended",
    ended_body: "The person who shared this recipe turned its link off. Ending a link cannot \
                 reach a copy already sent, so somebody may still have this recipe — but this \
                 address no longer opens it.",
    min_prep: "min prep",
    min_cook: "min cook",
    kcal_serving: "{n} kcal a serving",
    kcal_100g: "{n} kcal per 100 g",
    component_method: "its own method",
    component_method_below: "Its method is at the foot of the page ↓",
};

const FR: Words = Words {
    sent_you: "vous a envoyé cette recette",
    ingredients: "Ingrédients",
    method: "Préparation",
    from_source: "D'après",
    source_opens: "ouvre la page d'origine",
    also_in: "Également écrite en",
    history: "Historique",
    import: "Importer cette recette",
    bundle: "Fichier de recette",
    file_note: "Un zip de notes lisibles et de photographies, qui s'ouvre partout. Un autre \
                Kamosu le reprend en entier, avec toutes les versions derrière lui.",
    sheet: "Imprimer une fiche",
    sheet_making: "Mise en page de votre fiche",
    sheet_making_body: "Cela prend un instant. La fiche s'ouvre ici d'elle-même dès qu'elle est prête.",
    sheet_failed: "La fiche n'a pas pu être faite",
    file_failed: "Le fichier de recette n'a pas pu être fait",
    written_down: "Écrite",
    standing: "Ce lien emporte chaque version de cette recette, jusqu'à la première. Y mettre \
                fin arrête les nouveaux visiteurs, pas les copies déjà envoyées.",
    shared_by: "Partagée par {name}",
    ended_title: "Ce lien a pris fin",
    ended_body: "La personne qui a partagé cette recette a mis fin à son lien. Mettre fin à un \
                 lien n'atteint aucune copie déjà envoyée : quelqu'un a peut-être encore cette \
                 recette, mais cette adresse ne l'ouvre plus.",
    min_prep: "min prép.",
    min_cook: "min cuisson",
    kcal_serving: "{n} kcal par portion",
    kcal_100g: "{n} kcal pour 100 g",
    component_method: "sa propre préparation",
    component_method_below: "Sa préparation est en bas de page ↓",
};

const ES: Words = Words {
    sent_you: "te ha enviado esta receta",
    ingredients: "Ingredientes",
    method: "Preparación",
    from_source: "De",
    source_opens: "abre la página original",
    also_in: "También escrita en",
    history: "Historial",
    import: "Importar esta receta",
    bundle: "Archivo de receta",
    file_note: "Un zip de notas legibles y fotografías que se abre en cualquier parte. Otro \
                Kamosu lo recibe entero, con todas las versiones detrás.",
    sheet: "Imprimir una hoja",
    sheet_making: "Preparando tu hoja",
    sheet_making_body: "Tarda un momento. La hoja se abre aquí sola cuando esté lista.",
    sheet_failed: "No se pudo hacer la hoja",
    file_failed: "No se pudo hacer el archivo de receta",
    written_down: "Escrita",
    standing: "Este enlace lleva cada versión de esta receta, hasta la primera. Terminarlo \
                detiene a los visitantes nuevos, no las copias ya enviadas.",
    shared_by: "Compartida por {name}",
    ended_title: "Este enlace ha terminado",
    ended_body: "La persona que compartió esta receta terminó su enlace. Terminar un enlace no \
                 alcanza ninguna copia ya enviada: puede que alguien todavía tenga esta receta, \
                 pero esta dirección ya no la abre.",
    min_prep: "min prep.",
    min_cook: "min cocción",
    kcal_serving: "{n} kcal por ración",
    kcal_100g: "{n} kcal por 100 g",
    component_method: "su propia preparación",
    component_method_below: "Su preparación está al pie de la página ↓",
};

fn words(language: &str) -> &'static Words {
    match language {
        "fr" => &FR,
        "es" => &ES,
        _ => &EN,
    }
}

/// The name of a Language, written in itself.
fn language_name(language: &str) -> &'static str {
    match language {
        "fr" => "Français",
        "es" => "Español",
        "en" => "English",
        _ => "—",
    }
}

// ── Handlers ─────────────────────────────────────────────────────────────────

fn page(core: &Core, token: &str) -> Response {
    match core.read_shared_recipe(token) {
        Ok(shared) => html(StatusCode::OK, render(token, &shared, None)),
        // A token nobody minted gets the page's own not-found rather than a
        // bare status line: somebody mistyped a link a friend sent them, and
        // the thing to say is that this address opens nothing.
        Err(err) => html(status_for(&err), render_missing(&err)),
    }
}

fn translated(core: &Core, token: &str, language: &str) -> Response {
    match core.read_shared_recipe(token) {
        Ok(shared) => html(StatusCode::OK, render(token, &shared, Some(language))),
        Err(err) => html(status_for(&err), render_missing(&err)),
    }
}

fn photo(core: &Core, token: &str, hash: &str) -> Response {
    match core.read_shared_photograph(token, hash, DisplaySize::Page) {
        Ok(bytes) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "image/webp"),
                // A Photograph is known by its contents (ADR 0017), so this
                // URL's bytes can never change and may be cached hard.
                (header::CACHE_CONTROL, "public, max-age=86400"),
            ],
            bytes,
        )
            .into_response(),
        Err(err) => status_for(&err).into_response(),
    }
}

fn card(core: &Core, token: &str) -> Response {
    match core.shared_card(token) {
        Ok(bytes) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "image/png"),
                // The card is derived from the recipe as it stands, so a
                // recipe edited after being shared gets a new card. An hour is
                // long enough for the burst a link's first sending causes and
                // short enough that a fixed title is not stuck in a chat for
                // ever.
                (header::CACHE_CONTROL, "public, max-age=3600"),
            ],
            bytes,
        )
            .into_response(),
        Err(err) => status_for(&err).into_response(),
    }
}

/// Start a Sheet for whoever followed the link, and send them to it.
///
/// The browser's `Accept-Language` is passed on as the reader's locale, and it
/// decides one thing: A4 or Letter (ADR 0023). It is a guess at a page size,
/// never authority for anything (ADR 0033), and a wrong one costs margins.
fn ask_sheet(core: &Core, token: &str, language: Option<&str>, headers: &HeaderMap) -> Response {
    let locale = headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|value| value.to_str().ok());
    let mut input = serde_json::json!({ "token": token });
    if let Some(language) = language {
        input["language"] = language.into();
    }
    if let Some(locale) = locale {
        input["locale"] = locale.into();
    }
    // Asked exactly as any stranger asks at either Door: no Credential, so the
    // work lands in the stranger lane and a full line refuses (ADR 0032).
    match core.execute(None, "make_shared_sheet", input) {
        Ok(asked) => {
            let job_id = asked["job_id"].as_str().unwrap_or_default();
            (
                StatusCode::SEE_OTHER,
                [(header::LOCATION, format!("/s/{token}/sheet/{job_id}"))],
            )
                .into_response()
        }
        Err(err) => sheet_failed(core, token, &err),
    }
}

/// A Sheet's own address: the PDF once it is set, and until then a page that
/// says so and looks again in a second. Whether this link may still hand it
/// over is the Core's to say (`Core::shared_sheet`), not this page's.
fn sheet(core: &Core, token: &str, job_id: &str) -> Response {
    match core.shared_sheet(token, job_id) {
        Ok(SheetState::Ready { name, bytes }) => crate::web_door::pdf_response(&name, bytes),
        Ok(SheetState::Failed(reason)) => sheet_failed(core, token, &OpError::internal(reason)),
        Ok(SheetState::Setting) => {
            let language = language_of_share(core, token);
            let words = words(&language);
            let page = plain_page(&language, words.sheet_making, words.sheet_making_body).replacen(
                "</head>",
                "<meta http-equiv=\"refresh\" content=\"1\">\n</head>",
                1,
            );
            html(StatusCode::OK, page)
        }
        Err(err) => sheet_failed(core, token, &err),
    }
}

/// The Language a Share Link's own pages speak: its recipe's, or English when
/// there is no recipe to read one from.
fn language_of_share(core: &Core, token: &str) -> String {
    core.read_shared_recipe(token)
        .ok()
        .and_then(|shared| text_at(&shared["recipe"], "language").map(str::to_string))
        .unwrap_or_else(|| "en".to_string())
}

/// **The recipe file**, for whoever followed the link (#65, ADR 0020).
///
/// Built on the blocking pool: a Bundle reads every Photograph off disk and
/// deflates every note, which is file work the runtime answering every other
/// request should not be made to wait behind. Whether this link may hand one
/// over at all is the Core's to say, and so is keeping what it built.
async fn bundle(core: Arc<Core>, token: String) -> Response {
    let built = tokio::task::spawn_blocking({
        let core = core.clone();
        let token = token.clone();
        move || core.shared_bundle(&token)
    })
    .await;
    match built {
        Ok(Ok(written)) => crate::web_door::zip_response(&written.file_name, written.bytes),
        Ok(Err(err)) => bundle_failed(&core, &token, &err),
        Err(e) => bundle_failed(
            &core,
            &token,
            &OpError::internal(format!("the recipe file could not be made: {e}")),
        ),
    }
}

/// An ended link, or a Bundle that could not be built, as a page rather than a
/// raw error: whoever clicked was reading a recipe a moment ago.
fn bundle_failed(core: &Core, token: &str, err: &OpError) -> Response {
    let language = language_of_share(core, token);
    html(
        status_for(err),
        plain_page(&language, words(&language).file_failed, &err.to_sentence()),
    )
}

fn sheet_failed(core: &Core, token: &str, err: &OpError) -> Response {
    let language = language_of_share(core, token);
    html(
        status_for(err),
        plain_page(&language, words(&language).sheet_failed, &err.to_sentence()),
    )
}

fn status_for(err: &OpError) -> StatusCode {
    match err.kind {
        crate::core::ErrorKind::NotFound => StatusCode::NOT_FOUND,
        crate::core::ErrorKind::BadRequest => StatusCode::BAD_REQUEST,
        // Only asking for a Sheet can meet this: the stranger lane was full.
        crate::core::ErrorKind::Busy => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn html(status: StatusCode, body: String) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        body,
    )
        .into_response()
}

// ── Rendering ────────────────────────────────────────────────────────────────

/// HTML-escape. Everything below goes through this: a recipe's words are
/// somebody's typing, and a title containing `<` is an ordinary thing to write.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

/// What every page a Share Link serves says about itself, the refusals
/// included. It is how an importer on any Kamosu tells an ended link from a
/// stranger's site whose `/s/…/bundle` merely failed (#169,
/// `web_import::fetch_shared_recipe`).
pub const SHARE_PAGE_MARK: &str = r#"<meta name="generator" content="Kamosu">"#;

/// The name of the tag that says who shared the recipe on its page (#170).
pub const SHARED_BY_META: &str = "kamosu:shared-by";

fn shell(language: &str, title: &str, head: String, body: String) -> String {
    let mark = SHARE_PAGE_MARK;
    format!(
        r#"<!doctype html>
<html lang="{language}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
{mark}
<title>{title}</title>
<link rel="stylesheet" href="/assets/app.css">
<link rel="icon" href="/favicon.svg">
{head}
</head>
<body class="bg-ground-2">
{body}
</body>
</html>"#
    )
}

/// A page that carries a sentence instead of a recipe: a token nobody minted,
/// or a link that was ended. One shape for both, because they differ in what
/// they say and in nothing else.
fn plain_page(language: &str, heading: &str, body: &str) -> String {
    shell(
        language,
        &format!("{} · Kamosu", escape(heading)),
        String::new(),
        format!(
            r#"<div class="mx-auto max-w-[34rem] px-4 py-12">
  <div class="rounded-sm border border-rule bg-card p-6">
    <h1 class="font-display text-title font-semibold">{}</h1>
    <p class="mt-3 text-body text-ink-2">{}</p>
  </div>
</div>"#,
            escape(heading),
            escape(body)
        ),
    )
}

/// A token that resolves to nothing. English, because nothing is known about
/// what the reader was expecting to find — there is no recipe to take a
/// Language from.
fn render_missing(err: &OpError) -> String {
    if err.kind == crate::core::ErrorKind::NotFound {
        plain_page("en", "This address opens nothing", &err.to_sentence())
    } else {
        plain_page("en", EN.ended_title, EN.ended_body)
    }
}

/// The whole page.
///
/// `showing` names one of the shared Branch's Translations to read instead of
/// the Branch itself. Every Translation is carried whole by the Operation, so
/// this chooses between renderings rather than fetching anything more.
fn render(token: &str, shared: &Value, showing: Option<&str>) -> String {
    let ended = shared["ended"].as_bool().unwrap_or(false);
    let shared_recipe = &shared["recipe"];
    // A Language nobody wrote this recipe in falls back to the recipe itself
    // rather than to an error: a hand-edited URL is a mistyped one, not an
    // attack, and the recipe is what the reader came for.
    let recipe = showing
        .and_then(|language| {
            shared["translations"]
                .as_array()?
                .iter()
                .find(|t| text_at(t, "language") == Some(language))
        })
        .unwrap_or(shared_recipe);
    if ended || recipe.is_null() {
        // An ended link carries no recipe, so there is no Language to read off
        // one. It says so in English, which is the same bind `render_missing`
        // is in and for the same reason.
        return plain_page("en", EN.ended_title, EN.ended_body);
    }

    // A Translation is on screen only when the Language asked for is one this
    // share carries; the Sheet then prints that one, as the page shows it.
    let reading_a_translation = !std::ptr::eq(recipe, shared_recipe);
    let language = text_at(recipe, "language").unwrap_or("en");
    let words = words(language);
    let content = &recipe["content"];
    let title = text_at(content, "title").unwrap_or("");
    let sharer = text_at(shared, "shared_by").unwrap_or("");

    // The Passengers of the rendering being read (#50, ADR 0008) — the
    // Translation's own where a Translation is on screen, since each Branch
    // composes what it composes.
    let no_passengers = Vec::new();
    let carried = recipe["components"]
        .as_array()
        .unwrap_or(&no_passengers)
        .as_slice();

    // Who shared it, for a machine rather than a person: another Kamosu about
    // to import this recipe says so on its confirm screen (#170). The card's
    // own "Shared by" is worded in the recipe's Language, and is no field.
    let head = format!(
        "{}\n<meta name=\"{SHARED_BY_META}\" content=\"{}\">",
        open_graph(token, shared, title, sharer, language),
        escape(sharer)
    );
    let translations = translations_of(shared, token, language, words);
    let thread = thread_of(shared, words);

    let body = format!(
        r#"<div class="mx-auto max-w-[34rem] px-4 py-6 pb-12">
  <p class="flex items-center gap-2 px-2 pb-3 text-read text-ink-2">
    <span class="h-4 w-4 shrink-0 rounded-sm bg-accent" aria-hidden="true"></span>
    <span>{sharer_line}</span>
  </p>
  <div class="overflow-hidden rounded-sm border border-rule bg-card">
    {hero}
    <div class="px-6 pt-6 pb-8">
      {source_on_paper}
      {meta}
      <h2 class="mt-8 mb-2 font-display text-label font-semibold text-accent uppercase">{ingredients_heading}</h2>
      <ul>{ingredients}</ul>
      {nutrition}
      <h2 class="mt-8 mb-2 font-display text-label font-semibold text-accent uppercase">{method_heading}</h2>
      <ol>{steps}</ol>
      {annexes}
      {note}
      {translations}
      <!--
        What a reader can take away, both real and both links, because this page
        runs no script: the Sheet (#75) and the recipe file (#66, ADR 0020). The
        file is offered under the token rather than through the app's
        Credential-bearing route, since a stranger holding a link has no
        Credential at all.

        The line under it says the file can be brought into a Kamosu, which is
        the half of ADR 0020 a stranger cannot guess: the zip opens as plain
        notes anywhere, and is also exactly what another instance takes in.

        Importing is the third (#170): the recipe file received into the
        reader's own Cookbook, exactly as a Bundle is. It is a link into the app
        rather than anything done here: the page it opens is where the reader
        signs in, or names their own Kamosu, and is told what importing would
        do before it happens. One button for every reader, because this page
        never asks who is looking (Aurélien's choices 2 and 3).
      -->
      <div class="mt-8 border-t border-rule pt-4">
        <a href="{sheet_href}" class="block rounded-sm bg-accent p-3 text-center font-display text-read text-on-accent">{sheet}</a>
        <a href="/s/{bundle_token}/bundle" class="mt-2 block rounded-sm border border-rule p-3 text-center font-display text-read text-accent">{bundle}</a>
        <p class="mt-2 text-read text-ink-2">{file_note}</p>
        <a href="/import?link=%2Fs%2F{bundle_token}" class="mt-4 block rounded-sm bg-accent p-3 text-center font-display text-read text-on-accent">{import}</a>
      </div>
    </div>
  </div>
  {thread}
  <p class="px-2 pt-6 text-read text-ink-2">{standing}</p>
</div>"#,
        sharer_line = format_args!("{} {}", escape(sharer), escape(words.sent_you)),
        hero = hero(token, recipe, content, words),
        source_on_paper = source_on_paper(content, words),
        meta = meta(content, words),
        ingredients_heading = escape(words.ingredients),
        ingredients = ingredients(content, carried, words),
        nutrition = nutrition(content, words),
        method_heading = escape(words.method),
        steps = steps(content),
        annexes = annexes(carried, words),
        note = note(content),
        translations = translations,
        import = escape(words.import),
        bundle = escape(words.bundle),
        bundle_token = escape(token),
        file_note = escape(words.file_note),
        sheet = escape(words.sheet),
        sheet_href = match showing.filter(|_| reading_a_translation) {
            Some(language) => format!("/s/{}/in/{}/sheet", escape(token), escape(language)),
            None => format!("/s/{}/sheet", escape(token)),
        },
        thread = thread,
        standing = escape(words.standing),
    );

    shell(language, &format!("{} · Kamosu", escape(title)), head, body)
}

/// The tags a messaging app reads to draw its card.
///
/// The title is on the **picture** and is deliberately not `og:title`: every
/// chat client prints `og:title` as text under the image, so putting the
/// recipe's name in both is the one thing that reads twice. `og:title`
/// therefore says who shared it, and `og:description` the servings and the
/// time — three lines, three different facts (#65).
fn open_graph(token: &str, shared: &Value, title: &str, sharer: &str, language: &str) -> String {
    let address = text_at(shared, "public_address").unwrap_or("");
    let words = words(language);
    let content = &shared["recipe"]["content"];

    let mut facts: Vec<String> = Vec::new();
    if let Some(y) = content["yield"].as_object() {
        let amount = y.get("amount").and_then(Value::as_str).unwrap_or("");
        let noun = y.get("noun").and_then(Value::as_str).unwrap_or("");
        if !amount.is_empty() {
            facts.push(format!("{amount} {noun}").trim().to_string());
        }
    }
    let minutes = content["prep_time_minutes"].as_i64().unwrap_or(0)
        + content["cook_time_minutes"].as_i64().unwrap_or(0);
    if minutes > 0 {
        facts.push(format!("{minutes} min"));
    }
    if let Some(source) = content["source"].as_object()
        && let Some(text) = source.get("text").and_then(Value::as_str)
    {
        facts.push(format!("{} {text}", words.from_source.to_lowercase()));
    }

    format!(
        r#"<meta property="og:type" content="article">
<meta property="og:site_name" content="Kamosu">
<meta property="og:title" content="{shared_by}">
<meta property="og:description" content="{description}">
<meta property="og:image" content="{address}/s/{token}/card">
<meta property="og:image:alt" content="{alt}">
<meta property="og:url" content="{address}/s/{token}">
<meta name="twitter:card" content="summary_large_image">
<meta name="description" content="{description}">"#,
        shared_by = escape(&words.shared_by.replace("{name}", sharer)),
        description = escape(&facts.join(" · ")),
        address = escape(address),
        token = escape(token),
        alt = escape(title),
    )
}

/// The hero: the app's own (#81). A photograph wears the wash and carries the
/// title on it; a Cover carries the title bare.
fn hero(token: &str, recipe: &Value, content: &Value, words: &Words) -> String {
    let title = text_at(content, "title").unwrap_or("");
    let source = source_line(content, words);

    match text_at(content, "main_photo") {
        Some(hash) => format!(
            r#"<div class="relative overflow-hidden">
  <img src="/s/{token}/photo/{hash}" alt="" class="block h-[260px] w-full object-cover">
  <div class="wash pointer-events-none absolute inset-x-0 bottom-0"></div>
  <div class="absolute inset-x-0 bottom-0 px-6 pt-8 pb-4">
    {source}
    <h1 class="mt-1 font-display text-title font-semibold text-on-accent">{title}</h1>
  </div>
</div>"#,
            token = escape(token),
            hash = escape(hash),
            source = source.map_or(String::new(), |line| format!(
                r#"<p class="text-label text-on-accent uppercase">{line}</p>"#
            )),
            title = escape(title),
        ),
        None => {
            // No wash over a Cover: its dye was chosen (#46, #81).
            // The Lineage id and nothing else (#46): a Branch id here would
            // give a Translation a different face from its source, which is
            // the one thing `coverFor` promises never happens.
            let lineage = text_at(recipe, "lineage_id").unwrap_or("");
            format!(
                r#"<div class="relative overflow-hidden">
  {cover}
  <div class="absolute inset-x-0 bottom-0 px-6 pt-8 pb-4">
    <h1 class="font-display text-title font-semibold text-on-accent">{title}</h1>
  </div>
</div>"#,
                cover = cover_for(lineage).html("260px"),
                title = escape(title),
            )
        }
    }
}

/// On a Cover the hero carries the title alone: kinari over a dyed ground
/// clears the title and not the 10.5px Source line, so the Source is set here
/// instead, on paper (#81). The title is 25px since #88 and still large text at
/// weight 600, so the 3:1 it is judged against did not move.
fn source_on_paper(content: &Value, words: &Words) -> String {
    if content["main_photo"].as_str().is_some() {
        return String::new();
    }
    source_line(content, words).map_or(String::new(), |line| {
        format!(r#"<p class="mb-4 text-label text-ink-2 uppercase">{line}</p>"#)
    })
}

/// What the Source line says, wherever it is set, escaped and ready to place.
/// Where the Source has a web link the line itself is the link, with an
/// underline and a ↗ so it does not lean on colour alone (#152, Aurélien's
/// choice A, the recipe page's own). It opens outside Kamosu. A link that is
/// not an `http:` or `https:` address is not offered: `parse_source` stores
/// any string and this page is public, and the CSP blocking a `javascript:`
/// link is not something to depend on alone.
fn source_line(content: &Value, words: &Words) -> Option<String> {
    let source = content["source"].as_object()?;
    let said = format!(
        "{} {}",
        escape(words.from_source),
        escape(source.get("text").and_then(Value::as_str)?)
    );
    Some(
        match source
            .get("link")
            .and_then(Value::as_str)
            .and_then(web_link)
        {
            Some(link) => format!(
                r#"<a href="{href}" target="_blank" rel="noopener noreferrer" class="underline underline-offset-2">{said}<span aria-hidden="true">&nbsp;↗</span><span class="sr-only">, {opens}</span></a>"#,
                href = escape(link),
                opens = escape(words.source_opens),
            ),
            None => said,
        },
    )
}

/// A link as it can be offered to tap: an `http:` or `https:` address, or
/// nothing. The recipe page asks the same question in `ui/src/lib/source.ts`.
fn web_link(link: &str) -> Option<&str> {
    let link = link.trim();
    let (scheme, rest) = link.split_once("://")?;
    ((scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
        && !rest.is_empty())
    .then_some(link)
}

/// The meta: one strip of up to three cells, hairlines between.
fn meta(content: &Value, words: &Words) -> String {
    let mut cells: Vec<String> = Vec::new();
    let mut cell = |figure: String, label: String| {
        cells.push(format!(
            r#"<div class="flex-1 border-l border-rule px-2 py-3 text-center first:border-l-0">
  <b class="block font-display text-panel-figure font-semibold">{figure}</b>
  <span class="mt-1 block text-label text-ink-2 uppercase">{label}</span>
</div>"#
        ));
    };
    if let Some(minutes) = content["prep_time_minutes"].as_i64() {
        cell(minutes.to_string(), escape(words.min_prep));
    }
    if let Some(minutes) = content["cook_time_minutes"].as_i64() {
        cell(minutes.to_string(), escape(words.min_cook));
    }
    if let Some(y) = content["yield"].as_object() {
        cell(
            escape(y.get("amount").and_then(Value::as_str).unwrap_or("")),
            escape(y.get("noun").and_then(Value::as_str).unwrap_or("")),
        );
    }
    if cells.is_empty() {
        return String::new();
    }
    format!(
        r#"<div class="flex border-y border-rule">{}</div>"#,
        cells.join("")
    )
}

/// **The Nutrition figure, closing the Ingredients** (#84) — the treatment
/// Aurélien chose on 21 September 2026 against four drawn on both surfaces: a
/// fourth cell in the meta strip, a line directly under the strip, this, and a
/// place beside the Source. The list is where what goes into the dish is
/// already the subject, and the strip keeps the three cells #65 gave it.
///
/// It always says what it counts. 308 on its own says nothing, and a serving
/// and 100 g do not convert into each other without a weight the recipe does
/// not carry (GLOSSARY.md, "Nutrition").
///
/// Empty for the great majority of recipes, which carry no figure — and then
/// there is nothing here at all, not a dash and not a zero. Zero would be a
/// claim about the dish rather than a silence about it.
fn nutrition(content: &Value, words: &Words) -> String {
    let Some(figure) = content["nutrition"].as_object() else {
        return String::new();
    };
    let Some(calories) = figure.get("calories").and_then(Value::as_f64) else {
        return String::new();
    };
    // Two bases, two phrases, and no arm that falls through to a default: the
    // Catalogue declares exactly these two and the Core checks input against it
    // (#85), so anything else is not a figure this build knows how to word.
    // Saying nothing beats saying the wrong basis.
    let phrase = match figure.get("basis").and_then(Value::as_str) {
        Some("per_serving") => words.kcal_serving,
        Some("per_100g") => words.kcal_100g,
        _ => return String::new(),
    }
    // `{}` on an f64 prints 308 for 308.0 and 154.5 for 154.5, which is what a
    // source page stated either way. Nothing here rounds a figure.
    .replace("{n}", &calories.to_string());
    format!(
        r#"<p class="mt-3 text-read text-ink-2">{}</p>"#,
        escape(&phrase)
    )
}

/// One Ingredient Line: the written line at full size, led by a small indigo
/// square — and no mark anywhere saying whether Kamosu read it (ADR 0002).
///
/// **No subordinate line here, and that is not an omission.** In the app that
/// slot holds the amount converted for the reader, falling back to the echo of
/// what Kamosu read where there is nothing to convert (#49, ADR 0016). Kamosu
/// converts to the *kitchen*, and a stranger holding a link has no kitchen —
/// so there is never a conversion to make, and what is left is the echo, which
/// on a page like this restates the line directly above it. Live acceptance
/// showed exactly that: every row wearing a grey copy of itself.
///
/// So the written line is all a stranger is shown, which is also what ADR 0002
/// says it is: the truth. The Reading still travels in `read_shared_recipe`
/// for an agent reading the same share at the MCP door, where it is structure
/// rather than noise.
fn ingredients(content: &Value, carried: &[Value], words: &Words) -> String {
    lines(content, &[], carried, words)
}

/// The Ingredients list, with any **Component** on it unfolded in place (#50,
/// ADR 0008): the inner recipe's own lines, indented under the row that names
/// them behind a matcha rule, so the list stays a list you can shop from. Its
/// Steps are not here — they are set at the foot of the page by [`annexes`],
/// which is the treatment Aurélien chose on 3 September 2026.
///
/// `here` is the path of line indexes this list sits at: empty for the recipe
/// itself, `[0]` inside the Component on its first line. `carried` is every
/// Passenger of the whole page, flat, each carrying its own path — so finding
/// this list's Components is a matter of matching the prefix.
fn lines(content: &Value, here: &[i64], carried: &[Value], words: &Words) -> String {
    let empty = Vec::new();
    content["ingredients"]
        .as_array()
        .unwrap_or(&empty)
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let text = text_at(item, "text").unwrap_or("");
            if text_at(item, "kind") == Some("section") {
                return section_row(text);
            }
            match component_at(carried, here, index as i64) {
                Some(component) => component_row(component, text, carried, words),
                None => format!(
                    r#"<li class="flex gap-3 border-b border-rule py-3">
  <span class="ingredient-marker shrink-0 bg-accent" aria-hidden="true"></span>
  <span class="min-w-0 flex-1 text-line">{}</span>
</li>"#,
                    escape(text)
                ),
            }
        })
        .collect()
}

/// The Passenger sitting at one line of one list, or nothing — which is the
/// answer for every line of nearly every recipe.
fn component_at<'a>(carried: &'a [Value], here: &[i64], index: i64) -> Option<&'a Value> {
    carried.iter().find(|component| {
        component["path"].as_array().is_some_and(|path| {
            path.len() == here.len() + 1
                && path
                    .iter()
                    .take(here.len())
                    .map(Value::as_i64)
                    .eq(here.iter().copied().map(Some))
                && path.last().and_then(Value::as_i64) == Some(index)
        })
    })
}

/// **A Component's row**: the written line exactly as any other, led by a
/// matcha square rather than an indigo one, with one quiet line beneath saying
/// which recipe it names and how much of it — and, where it is held, the inner
/// recipe's own Ingredient Lines indented beneath that.
///
/// The written line is never rewritten and never replaced (ADR 0002), which is
/// what makes all three failures survivable: a Component whose recipe is
/// missing, one whose Yield gave no factor, and one that would loop each leave
/// a sentence where the unfolding would have been.
fn component_row(component: &Value, written: &str, carried: &[Value], words: &Words) -> String {
    let said = escape(text_at(component, "said").unwrap_or(""));

    // **Closed by default** (ADR 0008), and with no script: `<details>` is the
    // disclosure HTML already has, so a stranger's page opens a Component the
    // same way the app does without this page growing a single line of
    // JavaScript (ADR 0012's instinct, one layer down).
    //
    // A Component with nothing behind it is not a door: a missing recipe and a
    // repeat that stopped are sentences, and a `<summary>` promising to open
    // something empty would be the one control on this page that lies.
    let Some(content) = component["content"].as_object() else {
        return format!(
            r#"<li class="flex gap-3 border-b border-rule py-3">
  <span class="ingredient-marker shrink-0 bg-support-2" aria-hidden="true"></span>
  <div class="min-w-0 flex-1">
    <span class="block text-line">{written}</span>
    <span class="block text-read text-support-2">{said}</span>
  </div>
</li>"#,
            written = escape(written),
            said = said,
        );
    };

    let here: Vec<i64> = component["path"].as_array().map_or(Vec::new(), |path| {
        path.iter().filter_map(Value::as_i64).collect()
    });
    let has_steps = content
        .get("steps")
        .and_then(Value::as_array)
        .is_some_and(|steps| !steps.is_empty());
    // Where the method went. Under treatment B a Component is in two places and
    // the second is a long way down the page, so the row says where — and on a
    // page with no script the saying is an ordinary anchor.
    let to_the_foot = if has_steps {
        format!(
            r##"<a href="#annexe-{anchor}" class="mt-2 block text-read text-support-2 underline underline-offset-2">{words}</a>"##,
            anchor = anchor_of(&here),
            words = escape(words.component_method_below),
        )
    } else {
        String::new()
    };

    format!(
        r#"<li class="flex gap-3 border-b border-rule py-3">
  <span class="ingredient-marker shrink-0 bg-support-2" aria-hidden="true"></span>
  <div class="min-w-0 flex-1">
    <span class="block text-line">{written}</span>
    <details>
      <summary class="cursor-pointer text-read text-support-2">{said}</summary>
      <ul class="mt-3 border-l-2 border-support-2 bg-ground-2 py-1 pl-3">{within}</ul>
      {to_the_foot}
    </details>
  </div>
</li>"#,
        written = escape(written),
        said = said,
        within = lines(&component["content"], &here, carried, words),
        to_the_foot = to_the_foot,
    )
}

/// One Component's anchor, from the path of line indexes that reaches it — the
/// same key the app builds, so the two pages link the same way.
fn anchor_of(path: &[i64]) -> String {
    path.iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(".")
}

/// A Section, in either list: the quiet uppercase heading over a rule (#81).
fn section_row(text: &str) -> String {
    format!(
        r#"<li class="border-b border-rule py-4 pb-1"><h3 class="font-display text-label text-ink-2 uppercase">{}</h3></li>"#,
        escape(text)
    )
}

/// A Step: a number in a narrow column, in indigo, and the step at body size.
///
/// No subordinate line here either, and for the same reason as an Ingredient
/// Line: the app's slot holds the oven temperature in *this reader's* measures
/// (ADR 0016), and a stranger has none to convert to.
fn steps(content: &Value) -> String {
    let empty = Vec::new();
    let mut number = 0;
    content["steps"]
        .as_array()
        .unwrap_or(&empty)
        .iter()
        .map(|item| {
            let text = text_at(item, "text").unwrap_or("");
            if text_at(item, "kind") == Some("section") {
                return section_row(text);
            }
            number += 1;
            format!(
                r#"<li class="flex gap-3 border-b border-rule py-3">
  <span class="w-6 shrink-0 font-display text-line font-semibold text-accent">{number}</span>
  <p class="min-w-0 flex-1 text-body">{text}</p>
</li>"#,
                number = number,
                text = escape(text),
            )
        })
        .collect()
}

/// **The annexe** (#50, ADR 0008): a Component's own Steps, set at the foot of
/// the page after the recipe's Method, under a heading of their own — the
/// treatment Aurélien chose on 3 September 2026.
///
/// Composition says *what*, never *when*, so the dough's Steps are never
/// spliced into the pizza's method: Kamosu does not know the dough is made the
/// day before, and where the timing matters the cook writes a Step saying so.
/// They are set in the order the page meets them, which is the order the Core
/// already unfolded them in.
///
/// A Component with no Steps, one this instance does not hold, and one that
/// stopped at a repeat all contribute nothing here — there is no method to set.
fn annexes(carried: &[Value], words: &Words) -> String {
    carried
        .iter()
        .filter(|component| {
            component["content"]["steps"]
                .as_array()
                .is_some_and(|steps| !steps.is_empty())
        })
        .map(|component| {
            format!(
                r#"<div id="annexe-{anchor}" class="mt-8">
  <h2 class="mb-1 font-display text-label font-semibold text-support-2 uppercase">{title} · {method}</h2>
  <p class="mb-2 text-read text-ink-2">{said}</p>
  <ol class="border-l-2 border-support-2 bg-ground-2 py-1 pl-3">{steps}</ol>
</div>"#,
                anchor = anchor_of(
                    &component["path"]
                        .as_array()
                        .map_or(Vec::new(), |path| path
                            .iter()
                            .filter_map(Value::as_i64)
                            .collect::<Vec<_>>()),
                ),
                title = escape(text_at(component, "title").unwrap_or("")),
                method = escape(words.component_method),
                // How much of that recipe is wanted, repeated at the foot — the
                // annexe is a long way from the row that named it, and the
                // screen says it here too.
                said = escape(text_at(component, "said").unwrap_or("")),
                steps = steps(&component["content"]),
            )
        })
        .collect()
}

fn note(content: &Value) -> String {
    text_at(content, "note").map_or(String::new(), |note| {
        format!(
            r#"<div class="mt-6 border-l-2 border-accent py-1 pl-4 text-body whitespace-pre-wrap">{}</div>"#,
            escape(note)
        )
    })
}

/// Its Translations (ADR 0006, ADR 0018) — **readable**, not merely named.
///
/// Every Language this share carries, each a link to its own page under the
/// same token. The one being read is set as plain text: a link to the page you
/// are on says nothing, and the list is how a reader sees at a glance that the
/// recipe exists in their language at all.
fn translations_of(shared: &Value, token: &str, reading: &str, words: &Words) -> String {
    let empty = Vec::new();
    let others = shared["translations"].as_array().unwrap_or(&empty);
    if others.is_empty() {
        return String::new();
    }
    let original = text_at(&shared["recipe"], "language").unwrap_or("en");
    let mut languages: Vec<&str> = vec![original];
    languages.extend(others.iter().filter_map(|t| text_at(t, "language")));

    let links: Vec<String> = languages
        .iter()
        .map(|language| {
            let name = escape(language_name(language));
            if *language == reading {
                format!(r#"<span class="text-ink">{name}</span>"#)
            } else if *language == original {
                format!(
                    r#"<a class="text-accent" href="/s/{}">{name}</a>"#,
                    escape(token)
                )
            } else {
                format!(
                    r#"<a class="text-accent" href="/s/{}/in/{}">{name}</a>"#,
                    escape(token),
                    escape(language)
                )
            }
        })
        .collect();
    format!(
        r#"<p class="mt-6 text-read text-ink-2">{} {}</p>"#,
        escape(words.also_in),
        links.join(" · ")
    )
}

/// The Thread, as a second card beneath the recipe: every Version in order with
/// its name, its *what changed* line and its author, complete back to the first
/// (ADR 0018).
///
/// **Collapsed by default**, as ADR 0018 says in as many words. That is not in
/// tension with the same ADR's reason for putting it here at all — "a page you
/// can look at beats a warning you have to believe" — because the fold is
/// labelled with what it holds and how many Versions are in it. The sharer
/// opening their own link sees that their history travelled; they are one tap
/// from reading it, and the recipe is not buried under it.
fn thread_of(shared: &Value, words: &Words) -> String {
    let empty = Vec::new();
    let rows = shared["thread"].as_array().unwrap_or(&empty);
    if rows.is_empty() {
        return String::new();
    }
    let items: String = rows
        .iter()
        .rev()
        .map(|row| {
            let name = text_at(row, "name").map(str::to_string).unwrap_or_else(|| {
                match row["sequence"].as_i64() {
                    Some(1) => words.written_down.to_string(),
                    Some(n) => format!("{} {n}", words.written_down),
                    None => words.written_down.to_string(),
                }
            });
            let note = text_at(row, "change_note").map_or(String::new(), |line| {
                format!(
                    r#"<span class="mt-1 block text-read text-ink-2">{}</span>"#,
                    escape(line)
                )
            });
            format!(
                r#"<li class="flex gap-3 border-b border-rule py-3 last:border-b-0">
  <span class="ingredient-marker shrink-0 bg-accent" aria-hidden="true"></span>
  <div class="min-w-0 flex-1">
    <span class="block text-line">{name}</span>
    {note}
    <span class="mt-1 block text-label text-ink-2 uppercase">{hand} · {when}</span>
  </div>
</li>"#,
                name = escape(&name),
                note = note,
                hand = escape(text_at(row, "hand").unwrap_or("")),
                when = escape(
                    text_at(row, "created_at")
                        .unwrap_or("")
                        .get(..10)
                        .unwrap_or("")
                ),
            )
        })
        .collect();
    format!(
        r#"<details class="mt-4 rounded-sm border border-rule bg-card px-6 py-6">
  <summary class="cursor-pointer font-display text-label font-semibold text-accent uppercase">{summary}</summary>
  <ul class="mt-2">{items}</ul>
</details>"#,
        summary = format_args!("{} · {}", escape(words.history), rows.len()),
        items = items
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every class this page writes must exist in the stylesheet the binary
    /// ships.
    ///
    /// This page is the one place in Kamosu where markup is written as Rust
    /// strings, so nothing else catches a class that does not exist: there is
    /// no `svelte-check` here and no compiler that knows what a utility is.
    /// The trap is real and specific — Tailwind's palette, type scale, spacing
    /// and radii are REPLACED wholesale in `ui/src/app.css`, so a perfectly
    /// ordinary-looking `mt-0.5` silently styles nothing, the spacing scale
    /// being seven steps and a gutter. That exact class is what this test
    /// caught when it was first written.
    #[test]
    fn every_class_this_page_writes_is_in_the_stylesheet() {
        let css = crate::design_tokens::STYLESHEET;
        let source = include_str!("share_page.rs");
        let mut checked = 0;
        for attribute in source.split("class=\"").skip(1) {
            let Some(value) = attribute.split('"').next() else {
                continue;
            };
            for class in value.split_whitespace() {
                // A format placeholder, not a class.
                if class.contains('{') || class.contains('}') {
                    continue;
                }
                let escaped: String = class
                    .chars()
                    .flat_map(|ch| {
                        let needs = matches!(ch, '[' | ']' | '.' | ':' | '/' | '%');
                        needs.then_some('\\').into_iter().chain(std::iter::once(ch))
                    })
                    .collect();
                assert!(
                    css.contains(&format!(".{escaped}")),
                    "the Share Link page writes `{class}`, which ui/src/app.css does not emit — \
                     either it is not a class this design has (the scales are replaced, not \
                     extended) or `just css` has not been run since it was added"
                );
                checked += 1;
            }
        }
        assert!(checked > 40, "the class scan found almost nothing to check");
    }

    /// The standing line is the same words every time, on every page, with no
    /// first-time special case and nothing to dismiss (ADR 0018). It is a
    /// constant per Language and this pins that it stays one.
    #[test]
    fn the_standing_line_is_fixed_wording_in_every_language() {
        for words in [&EN, &FR, &ES] {
            assert!(
                words.standing.split(". ").count() == 2,
                "the standing line says both halves"
            );
            assert!(!words.standing.is_empty());
        }
        // And it is not built from anything: the same string every time.
        assert_eq!(words("en").standing, EN.standing);
        assert_eq!(words("fr").standing, FR.standing);
        assert_eq!(words("es").standing, ES.standing);
        // An unknown Language falls back rather than showing nothing.
        assert_eq!(words("de").standing, EN.standing);
    }

    /// A Cover never wears the wash (#46, #81): its dye was chosen.
    #[test]
    fn a_cover_hero_carries_no_wash() {
        let recipe = serde_json::json!({
            "lineage_id": "l_1848cb7653cb9d93",
            "content": { "title": "Coq au Vin", "main_photo": null, "source": null },
        });
        let html = hero("t", &recipe, &recipe["content"], &EN);
        assert!(!html.contains("wash"), "a Cover must not wear the wash");
        assert!(html.contains("Coq au Vin"), "the title stands on it");
    }

    #[test]
    fn a_photograph_hero_wears_the_wash_and_carries_the_title() {
        let recipe = serde_json::json!({
            "lineage_id": "l_1848cb7653cb9d93",
            "content": {
                "title": "Korean Fried Chicken",
                "main_photo": "p_abc",
                "source": { "text": "mykoreankitchen.com", "link": null },
            },
        });
        let html = hero("tok", &recipe, &recipe["content"], &EN);
        assert!(
            html.contains(r#"class="wash"#),
            "a photograph wears the wash"
        );
        assert!(html.contains("Korean Fried Chicken"));
        assert!(html.contains("mykoreankitchen.com"));
        // The photograph is reached through the token, never through the
        // Credential-bearing route a stranger cannot use.
        assert!(html.contains("/s/tok/photo/p_abc"));
        assert!(!html.contains("/api/photographs"));
    }

    /// A recipe's words are somebody's typing, and a title with a `<` in it is
    /// an ordinary thing to write.
    #[test]
    fn a_title_cannot_carry_markup_onto_the_page() {
        let recipe = serde_json::json!({
            "lineage_id": "l_1848cb7653cb9d93",
            "content": { "title": "<script>alert(1)</script>", "main_photo": null, "source": null },
        });
        let html = hero("t", &recipe, &recipe["content"], &EN);
        assert!(!html.contains("<script>"), "a title is escaped");
        assert!(html.contains("&lt;script&gt;"));
    }

    /// The card's three text lines each say a different thing (#65): the
    /// picture carries the recipe's name, so `og:title` must not.
    #[test]
    fn the_cards_lines_do_not_repeat_each_other() {
        let shared = serde_json::json!({
            "shared_by": "Aurélien",
            "public_address": "https://kamosu.example",
            "recipe": { "content": {
                "title": "Korean Fried Chicken",
                "yield": { "amount": "4", "noun": "servings" },
                "prep_time_minutes": 10,
                "cook_time_minutes": 30,
                "source": { "text": "mykoreankitchen.com", "link": null },
            }},
        });
        let tags = open_graph("tok", &shared, "Korean Fried Chicken", "Aurélien", "en");
        assert!(
            tags.contains(r#"og:title" content="Shared by Aurélien"#),
            "og:title names the person, never the recipe: {tags}"
        );
        assert!(
            !tags.contains(r#"og:title" content="Korean Fried Chicken"#),
            "the recipe's name is on the picture and must not be printed twice"
        );
        assert!(tags.contains("4 servings · 40 min · from mykoreankitchen.com"));
        assert!(tags.contains("https://kamosu.example/s/tok/card"));
    }

    /// The Source line as the page draws it on either layout (#152): on the
    /// photograph's hero, or on paper under a Cover.
    fn source_as_drawn(source: Value, photo: bool) -> String {
        let recipe = serde_json::json!({
            "lineage_id": "l_1848cb7653cb9d93",
            "content": {
                "title": "Best Steak Marinade",
                "main_photo": if photo { Value::from("p_abc") } else { Value::Null },
                "source": source,
            },
        });
        if photo {
            let on_paper = source_on_paper(&recipe["content"], &EN);
            assert_eq!(on_paper, "", "with a photograph the Source is on the hero");
            hero("tok", &recipe, &recipe["content"], &EN)
        } else {
            source_on_paper(&recipe["content"], &EN)
        }
    }

    #[test]
    fn a_source_with_a_web_link_is_the_link_on_both_layouts() {
        for photo in [true, false] {
            let html = source_as_drawn(
                serde_json::json!({
                    "text": "Allrecipes",
                    "link": "https://www.allrecipes.com/recipe/143809/?a=1&b=2",
                }),
                photo,
            );
            assert!(
                html.contains(
                    r#"<a href="https://www.allrecipes.com/recipe/143809/?a=1&amp;b=2" target="_blank" rel="noopener noreferrer" class="underline underline-offset-2">From Allrecipes<span aria-hidden="true">&nbsp;↗</span><span class="sr-only">, opens the original page</span></a>"#
                ),
                "photo {photo}: {html}"
            );
        }
    }

    #[test]
    fn a_source_with_no_link_is_the_plain_line_on_both_layouts() {
        for photo in [true, false] {
            let html = source_as_drawn(
                serde_json::json!({ "text": "Allrecipes", "link": null }),
                photo,
            );
            assert!(
                html.contains("From Allrecipes</p>"),
                "photo {photo}: {html}"
            );
            assert!(!html.contains("<a "), "photo {photo}: {html}");
        }
    }

    #[test]
    fn a_link_that_is_not_a_web_address_is_never_offered() {
        for link in [
            "javascript:alert(1)",
            "data:text/html,hi",
            "allrecipes",
            "https://",
        ] {
            for photo in [true, false] {
                let html = source_as_drawn(
                    serde_json::json!({ "text": "Allrecipes", "link": link }),
                    photo,
                );
                assert!(
                    html.contains("From Allrecipes</p>"),
                    "{link}, photo {photo}: {html}"
                );
                assert!(!html.contains("<a "), "{link}, photo {photo}: {html}");
            }
        }
    }

    /// The link and the Source's words are somebody's typing on a public page.
    #[test]
    fn a_sources_words_and_link_cannot_carry_markup_onto_the_page() {
        for photo in [true, false] {
            let html = source_as_drawn(
                serde_json::json!({
                    "text": "<b>Allrecipes</b>",
                    "link": "https://example.com/\"><script>alert(1)</script>",
                }),
                photo,
            );
            assert!(!html.contains("<script>"), "photo {photo}: {html}");
            assert!(!html.contains("<b>"), "photo {photo}: {html}");
            assert!(
                html.contains(r#"href="https://example.com/&quot;&gt;&lt;script&gt;"#),
                "photo {photo}: {html}"
            );
            assert!(
                html.contains("From &lt;b&gt;Allrecipes&lt;/b&gt;"),
                "photo {photo}: {html}"
            );
        }
    }
}
