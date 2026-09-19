//! The Sheet (#75, ADR 0023): one recipe rendered for paper.
//!
//! The Core gathers what the page carries — the Branch as it stands, scaled as
//! it stands on screen, its Components unfolded — into a [`Gathered`]. This
//! module words it in the recipe's own Language ([`compose`]) and sets it in
//! Typst, embedded as a library ([`typeset`]): no browser, no hand-rolled PDF
//! writer, and nothing fetched while it runs. The template and the typeface
//! are compiled into the binary, so a Sheet is set the same on every instance.
//!
//! **What goes on the page is decided by one rule** — a Sheet carries the
//! recipe, not the library — and this module is where the rule is applied, not
//! where it is argued; ADR 0023 is. **What the page looks like** is
//! `sheet.typ`'s business, and was chosen by Aurélien on #75.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use image::{DynamicImage, ImageFormat, imageops::FilterType};
use serde_json::{Value, json};
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_layout::PagedDocument;

use crate::core::OpError;
use crate::units;

/// The template. Layout only: every word arrives already worded.
const TEMPLATE: &str = include_str!("sheet.typ");

/// Zen Old Mincho, the display face, at the two weights the page sets: the
/// same Latin and Latin-ext subsets the browser loads, decompressed from their
/// woff2 (see `assets/fonts/README.md`). Typst reads TrueType, not woff2.
const FONTS: [&[u8]; 4] = [
    include_bytes!("../assets/fonts/zen-old-mincho-400-latin.ttf"),
    include_bytes!("../assets/fonts/zen-old-mincho-400-latin-ext.ttf"),
    include_bytes!("../assets/fonts/zen-old-mincho-600-latin.ttf"),
    include_bytes!("../assets/fonts/zen-old-mincho-600-latin-ext.ttf"),
];

/// Where kept Sheets live. A Sheet is a rendering and is set again from the
/// recipe in a moment, so this directory holds nothing that is not disposable
/// (ADR 0032); [`prune`] empties it of anything older than a day.
pub fn sheets_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("sheets")
}

/// How long a kept Sheet is kept. A day, because the date printed on it moves
/// on after one and no later ask would ever find it again.
const KEPT_FOR: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);

/// Throw away kept Sheets older than a day. Called whenever a Sheet is asked
/// for, which bounds the directory by how many different pages a day asks for.
pub fn prune(data_dir: &Path) {
    let Ok(entries) = std::fs::read_dir(sheets_dir(data_dir)) else {
        return;
    };
    for entry in entries.flatten() {
        let stale = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > KEPT_FOR);
        if stale {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// **What a page is**, as a key: the composed document and the Photograph it
/// prints. The same key sets to the same bytes, which is what lets a Sheet be
/// set once and kept (ADR 0032).
pub fn key_of(document: &Value, main_photo: Option<&str>) -> String {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(document.to_string().as_bytes());
    hash.update([0]);
    hash.update(main_photo.unwrap_or_default().as_bytes());
    format!("s_{}", hex::encode(hash.finalize()))
}

/// A kept Sheet: the file it is kept in, and how many pages it came to. The
/// page count rides in the name, so asking again needs no second file.
pub struct Kept {
    pub name: String,
    pub pages: usize,
}

/// The kept Sheet for a key, if one was set today.
pub fn kept(data_dir: &Path, key: &str) -> Option<Kept> {
    std::fs::read_dir(sheets_dir(data_dir))
        .ok()?
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .find_map(|name| {
            let pages = name
                .strip_prefix(key)?
                .strip_prefix('-')?
                .strip_suffix(".pdf")?
                .parse()
                .ok()?;
            Some(Kept { name, pages })
        })
}

/// Keep a set Sheet under its key.
pub fn keep(data_dir: &Path, key: &str, set: &Typeset) -> Result<Kept, OpError> {
    let name = format!("{key}-{}.pdf", set.pages);
    std::fs::create_dir_all(sheets_dir(data_dir))
        .map_err(|e| OpError::internal(format!("cannot create the Sheets directory: {e}")))?;
    std::fs::write(sheets_dir(data_dir).join(&name), &set.pdf)
        .map_err(|e| OpError::internal(format!("cannot keep the Sheet: {e}")))?;
    Ok(Kept {
        name,
        pages: set.pages,
    })
}

/// A kept Sheet's bytes, by the name a Job recorded. A name that is not one
/// this module could have written reads nothing, so a Job's result cannot be
/// made to point outside the directory.
pub fn read_kept(data_dir: &Path, name: &str) -> Option<Vec<u8>> {
    let plausible = name.starts_with("s_")
        && name.ends_with(".pdf")
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'));
    if !plausible {
        return None;
    }
    std::fs::read(sheets_dir(data_dir).join(name)).ok()
}

/// Whether a factor scales anything at all. A share or a Yield a hair from
/// one is the recipe as written, and saying *about* beside the same number
/// would only repeat the line.
pub fn scales(factor: f64) -> bool {
    (factor - 1.0).abs() >= SAME
}

/// How close two factors are before they are the same amount on paper.
const SAME: f64 = 0.005;

// ── Paper ────────────────────────────────────────────────────────────────────

/// The two sizes a Sheet comes in. Never asked (ADR 0023): the page follows
/// the reader, and a wrong guess reflows rather than losing anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paper {
    A4,
    Letter,
}

impl Paper {
    /// A Person's Reading Measures: *US measures* prints Letter, anything else
    /// prints A4.
    pub fn for_measures(stored: &str) -> Self {
        match units::Measures::from_stored(stored) {
            units::Measures::Us => Paper::Letter,
            _ => Paper::A4,
        }
    }

    /// A stranger has no Reading Measures, so their browser's locale decides
    /// the same way: a US or Canadian locale prints Letter, anything else —
    /// including no locale at all — prints A4. This is a guess at a page size
    /// and never authority for anything (ADR 0033).
    pub fn for_locale(locale: Option<&str>) -> Self {
        let region = locale
            .and_then(|tag| tag.split(',').next())
            .and_then(|tag| tag.split(';').next())
            .and_then(|tag| tag.trim().split(['-', '_']).nth(1))
            .map(str::to_ascii_uppercase);
        match region.as_deref() {
            Some("US" | "CA") => Paper::Letter,
            _ => Paper::A4,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Paper::A4 => "a4",
            Paper::Letter => "us-letter",
        }
    }
}

// ── What the Core gathers ────────────────────────────────────────────────────

/// Everything a Sheet carries, as the Core found it and before any of it is
/// worded. Built by `Core::gather_sheet`.
pub struct Gathered {
    /// The Branch's own Language. The page's words follow it, as the Share
    /// Link page's do: a Sheet is the recipe, and the recipe is in one.
    pub language: String,
    /// The head Version's content.
    pub content: Value,
    /// One slot per Ingredient Line: the scaled amount, worded, where the
    /// recipe is scaled and the line has one. Empty on a recipe at the Yield
    /// as written — ADR 0023 prints no Reading but a scaled one.
    pub about: Vec<Option<String>>,
    /// The Yield being cooked and the Yield as written, where they differ.
    pub scaled: Option<(Value, Value)>,
    /// Every Component, flat and depth first, as `unfold_components` answers.
    pub components: Vec<Value>,
    pub version_name: Option<String>,
    /// When the head Version was written, as stored (`2026-09-03T19:07:…Z`).
    pub written_at: String,
    /// The Hand that wrote it, by name and nothing else (ADR 0015).
    pub hand: String,
    /// The head Version's id.
    pub fingerprint: String,
    /// The Share Link this Sheet was reached through, where there was one.
    pub share_url: Option<String>,
    pub paper: Paper,
    /// Today, as `YYYY-MM-DD`.
    pub today: String,
}

// ── The words ────────────────────────────────────────────────────────────────

/// A Sheet's own words, in the three Languages Kamosu speaks. Written here, as
/// the Share Link page's are, because this page is set by the server.
struct Words {
    ingredients: &'static str,
    method: &'static str,
    makes: &'static str,
    prep: &'static str,
    cook: &'static str,
    scaled: &'static str,
    for_recipe: &'static str,
    whole: &'static str,
    part: &'static str,
    unscaled: &'static str,
    as_written_makes: &'static str,
    written: &'static str,
    printed: &'static str,
    page: &'static str,
    of: &'static str,
    page_ref: &'static str,
    written_in: &'static str,
    /// English, French and Spanish, named in this Language.
    languages: [&'static str; 3],
    months: [&'static str; 12],
    /// How a date is set: `{day}`, `{month}`, `{year}`.
    date: &'static str,
}

const EN: Words = Words {
    ingredients: "Ingredients",
    method: "Method",
    makes: "Makes {yield}",
    prep: "Prep {time}",
    cook: "Cook {time}",
    scaled: "Scaled to {wanted} — as written, makes {written}",
    for_recipe: "for {title}",
    whole: "the whole recipe",
    part: "{share} of the recipe",
    unscaled: "Kamosu could not work out how much, so this is the recipe as written",
    as_written_makes: "as written, makes {yield}",
    written: "written {date}",
    printed: "printed {date}",
    page: "page",
    of: "of",
    page_ref: "p.",
    written_in: "written in {language}",
    languages: ["English", "French", "Spanish"],
    months: [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ],
    date: "{day} {month} {year}",
};

const FR: Words = Words {
    ingredients: "Ingrédients",
    method: "Préparation",
    makes: "Pour {yield}",
    prep: "Prép. {time}",
    cook: "Cuisson {time}",
    scaled: "Ajustée à {wanted} — telle qu'écrite, pour {written}",
    for_recipe: "pour {title}",
    whole: "la recette entière",
    part: "{share} de la recette",
    unscaled: "Kamosu n'a pas pu établir la quantité : voici la recette telle qu'elle est écrite",
    as_written_makes: "telle qu'écrite, pour {yield}",
    written: "écrite le {date}",
    printed: "imprimée le {date}",
    page: "page",
    of: "sur",
    page_ref: "p.",
    written_in: "écrite en {language}",
    languages: ["anglais", "français", "espagnol"],
    months: [
        "janvier",
        "février",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "août",
        "septembre",
        "octobre",
        "novembre",
        "décembre",
    ],
    date: "{day} {month} {year}",
};

const ES: Words = Words {
    ingredients: "Ingredientes",
    method: "Preparación",
    makes: "Rinde {yield}",
    prep: "Prep. {time}",
    cook: "Cocción {time}",
    scaled: "Ajustada a {wanted} — tal como está escrita, rinde {written}",
    for_recipe: "para {title}",
    whole: "la receta entera",
    part: "{share} de la receta",
    unscaled: "Kamosu no pudo calcular la cantidad: esta es la receta tal como está escrita",
    as_written_makes: "tal como está escrita, rinde {yield}",
    written: "escrita el {date}",
    printed: "impresa el {date}",
    page: "página",
    of: "de",
    page_ref: "p.",
    written_in: "escrita en {language}",
    languages: ["inglés", "francés", "español"],
    months: [
        "enero",
        "febrero",
        "marzo",
        "abril",
        "mayo",
        "junio",
        "julio",
        "agosto",
        "septiembre",
        "octubre",
        "noviembre",
        "diciembre",
    ],
    date: "{day} de {month} de {year}",
};

/// The Language the page is set in: the recipe's own, where Kamosu speaks it,
/// and English otherwise.
fn words_for(language: &str) -> (&'static Words, &'static str) {
    match language {
        "fr" => (&FR, "fr"),
        "es" => (&ES, "es"),
        _ => (&EN, "en"),
    }
}

/// A Language's name in the page's own words, or its code where Kamosu has no
/// name for it.
fn language_name<'a>(code: &'a str, words: &'static Words) -> &'a str {
    match code {
        "en" => words.languages[0],
        "fr" => words.languages[1],
        "es" => words.languages[2],
        other => other,
    }
}

/// `19 September 2026` from `2026-09-19` or a full timestamp. Anything that
/// does not read as a date is printed as it came, which beats printing nothing.
fn long_date(stored: &str, words: &Words) -> String {
    let date = stored.get(..10).unwrap_or(stored);
    let mut parts = date.split('-');
    let (Some(year), Some(month), Some(day)) = (parts.next(), parts.next(), parts.next()) else {
        return stored.to_string();
    };
    let (Ok(month), Ok(day)) = (month.parse::<usize>(), day.parse::<u32>()) else {
        return stored.to_string();
    };
    let Some(month) = month.checked_sub(1).and_then(|m| words.months.get(m)) else {
        return stored.to_string();
    };
    words
        .date
        .replace("{day}", &day.to_string())
        .replace("{month}", month)
        .replace("{year}", year)
}

/// `20 min`, `1 h 30`, `2 h` — the way a time is written on a card.
fn duration(minutes: i64) -> String {
    let (hours, rest) = (minutes / 60, minutes % 60);
    match (hours, rest) {
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m:02}"),
    }
}

/// `3 pizzas` from a Yield.
fn yield_text(value: &Value) -> Option<String> {
    let amount = value["amount"].as_str().unwrap_or("").trim();
    let noun = value["noun"].as_str().unwrap_or("").trim();
    let joined = format!("{amount} {noun}").trim().to_string();
    (!joined.is_empty()).then_some(joined)
}

/// The one meta line under a title: what it makes, and how long.
fn meta_line(content: &Value, words: &Words) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(made) = yield_text(&content["yield"]) {
        parts.push(words.makes.replace("{yield}", &made));
    }
    if let Some(minutes) = content["prep_time_minutes"].as_i64().filter(|m| *m > 0) {
        parts.push(words.prep.replace("{time}", &duration(minutes)));
    }
    if let Some(minutes) = content["cook_time_minutes"].as_i64().filter(|m| *m > 0) {
        parts.push(words.cook.replace("{time}", &duration(minutes)));
    }
    (!parts.is_empty()).then(|| parts.join("  ·  "))
}

// ── Composing the page ───────────────────────────────────────────────────────

/// The path of line indexes a Component sits at.
fn path_of(component: &Value) -> Vec<i64> {
    component["path"].as_array().map_or(Vec::new(), |path| {
        path.iter().filter_map(Value::as_i64).collect()
    })
}

/// A Component gets a block of its own when there is something to put in it:
/// a recipe this instance holds, or a repeat Kamosu stopped at, which the page
/// says so about where the block would have been (ADR 0023). A Component
/// nobody here holds gets no block — its written line already says what it is.
fn has_block(component: &Value) -> bool {
    component["held"].as_bool().unwrap_or(false)
}

/// Whether two Components print as one block: the same recipe, at the same
/// amount, and neither a repeat Kamosu stopped at.
fn same_block(a: &Value, b: &Value) -> bool {
    a["lineage_id"] == b["lineage_id"]
        && a["stopped"] == b["stopped"]
        && match (a["share"].as_f64(), b["share"].as_f64()) {
            (Some(x), Some(y)) => (x - y).abs() < SAME,
            (None, None) => true,
            _ => false,
        }
}

/// Word everything the Core gathered into what `sheet.typ` reads.
pub fn compose(gathered: &Gathered) -> Value {
    let (words, language) = words_for(&gathered.language);
    let title = gathered.content["title"].as_str().unwrap_or("").to_string();

    // Blocks are numbered in the order the page meets them, which is the
    // depth-first order the Core already unfolded them in: parent first, each
    // Component in order of first mention (ADR 0023). A recipe named twice at
    // the same amount is printed once, and both lines point at it.
    let mut blocked: Vec<&Value> = Vec::new();
    for component in gathered.components.iter().filter(|c| has_block(c)) {
        if !blocked.iter().any(|seen| same_block(seen, component)) {
            blocked.push(component);
        }
    }
    let block_at = |path: &[i64]| -> Option<usize> {
        let component = gathered
            .components
            .iter()
            .find(|c| has_block(c) && path_of(c) == path)?;
        blocked.iter().position(|seen| same_block(seen, component))
    };

    let lines = |content: &Value, here: &[i64], about: &dyn Fn(usize) -> Option<String>| {
        let empty = Vec::new();
        content["ingredients"]
            .as_array()
            .unwrap_or(&empty)
            .iter()
            .enumerate()
            .map(|(index, line)| {
                let text = line["text"].as_str().unwrap_or("");
                if line["kind"] == "section" {
                    return json!({ "section": true, "text": text });
                }
                let mut path = here.to_vec();
                path.push(index as i64);
                let block = block_at(&path);
                let names = block
                    .and_then(|b| blocked[b]["title"].as_str())
                    .unwrap_or("");
                json!({
                    "section": false,
                    "text": text,
                    // The written line is printed and a Reading is not; the one
                    // exception is a scaled amount beneath it (ADR 0023).
                    "about": about(index),
                    // The other exception: where a Component's block begins.
                    "block": block,
                    "names": names,
                })
            })
            .collect::<Vec<_>>()
    };
    let steps = |content: &Value| {
        let empty = Vec::new();
        content["steps"]
            .as_array()
            .unwrap_or(&empty)
            .iter()
            .map(|step| {
                json!({
                    "section": step["kind"] == "section",
                    "text": step["text"].as_str().unwrap_or(""),
                })
            })
            .collect::<Vec<_>>()
    };
    let note = |content: &Value| {
        content["note"]
            .as_str()
            .filter(|n| !n.trim().is_empty())
            .map_or(Value::Null, Value::from)
    };

    let root_about = |index: usize| gathered.about.get(index).cloned().flatten();
    let recipe = json!({
        "title": title,
        "photo": gathered.content["main_photo"].is_string(),
        "meta": meta_line(&gathered.content, words),
        "scaled": gathered.scaled.as_ref().and_then(|(wanted, written)| {
            Some(
                words
                    .scaled
                    .replace("{wanted}", &yield_text(wanted)?)
                    .replace("{written}", written["amount"].as_str()?.trim()),
            )
        }),
        "ingredients": lines(&gathered.content, &[], &root_about),
        "steps": steps(&gathered.content),
        "note": note(&gathered.content),
    });

    let blocks: Vec<Value> = blocked
        .iter()
        .map(|component| {
            let path = path_of(component);
            let parent_title = gathered
                .components
                .iter()
                .find(|c| path_of(c) == path[..path.len() - 1])
                .and_then(|c| c["title"].as_str())
                .unwrap_or(&title);
            let own_title = component["title"].as_str().unwrap_or("");

            if component["stopped"].as_bool().unwrap_or(false) {
                return json!({
                    "title": own_title,
                    "lead": units::component_line(true, true, Some(own_title), None, language),
                    "meta": Value::Null,
                    "recipe": Value::Null,
                });
            }

            let content = &component["content"];
            let share = component["share"].as_f64();
            let how_much = match share {
                None => words.unscaled.to_string(),
                Some(share) if !scales(share) => words.whole.to_string(),
                Some(share) => words.part.replace("{share}", &units::as_fraction(share)),
            };
            let mut lead = vec![words.for_recipe.replace("{title}", parent_title), how_much];
            if share.is_some()
                && let Some(made) = yield_text(&content["yield"])
            {
                lead.push(words.as_written_makes.replace("{yield}", &made));
            }
            // A Component names a Lineage, not a Branch, so it prints in
            // whatever Language this instance holds it in — marked, where that
            // is not the page's own (ADR 0023).
            if let Some(its) = component["language"].as_str()
                && its != gathered.language
            {
                lead.push(
                    words
                        .written_in
                        .replace("{language}", language_name(its, words)),
                );
            }
            // A Component is printed already scaled, so its lines carry the
            // amount wanted — unless Kamosu could not say how much, when it is
            // handed over as written and says so above rather than guessing.
            let scaled = share.is_some_and(scales);
            let measured = component["measured"]["ingredients"].clone();
            let about = move |index: usize| {
                if !scaled {
                    return None;
                }
                measured[index].as_str().map(str::to_string)
            };
            json!({
                "title": own_title,
                "lead": lead.join("  ·  "),
                "meta": meta_line(content, words),
                "recipe": {
                    "ingredients": lines(content, &path, &about),
                    "steps": steps(content),
                    "note": note(content),
                },
            })
        })
        .collect();

    // A small block saying where this page came from, so a Sheet found in a
    // drawer can still say what it is (ADR 0023). The Hand is its name, bare:
    // no "by", nothing Kamosu could not stand behind (ADR 0015).
    let mut provenance = Vec::new();
    let written = words
        .written
        .replace("{date}", &long_date(&gathered.written_at, words));
    provenance.push(match &gathered.version_name {
        Some(name) if !name.trim().is_empty() => format!("{name}, {written}"),
        _ => written,
    });
    provenance.push(gathered.hand.clone());
    if let Some(source) = gathered.content["source"].as_object() {
        let text = source.get("text").and_then(Value::as_str).unwrap_or("");
        let link = source.get("link").and_then(Value::as_str).unwrap_or("");
        let said = match (text.trim(), link.trim()) {
            ("", "") => String::new(),
            (text, "") => text.to_string(),
            ("", link) => link.to_string(),
            (text, link) => format!("{text}, {link}"),
        };
        if !said.is_empty() {
            provenance.push(said);
        }
    }
    if let Some(url) = &gathered.share_url {
        provenance.push(url.clone());
    }

    json!({
        "paper": gathered.paper.as_str(),
        "language": language,
        "words": {
            "ingredients": words.ingredients,
            "method": words.method,
            "page": words.page,
            "of": words.of,
            "page_ref": words.page_ref,
        },
        "printed": words.printed.replace("{date}", &long_date(&gathered.today, words)),
        "recipe": recipe,
        "blocks": blocks,
        "provenance": provenance,
        "fingerprint": gathered.fingerprint,
    })
}

// ── The photograph ───────────────────────────────────────────────────────────

/// The Main Photo as the page prints it: a strip the width of the text, cut
/// from the middle of the print-sized Display Copy and encoded as JPEG.
///
/// JPEG because Typst hands a JPEG to the PDF as it is, where any other
/// format is written out as raw pixels — a 100 KB WebP became a 2 MB Sheet in
/// the prototype. The strip is cut here rather than by the template so the
/// PDF carries only the part of the picture that is printed.
pub fn print_photo(display_copy: &[u8]) -> Result<Vec<u8>, OpError> {
    /// The strip's shape on the page: the text width over 32 mm.
    const ASPECT: f64 = 176.0 / 32.0;
    /// About 250 dpi across the text width, which is more than a kitchen
    /// printer resolves.
    const WIDTH: u32 = 1800;

    let image = image::load_from_memory_with_format(display_copy, ImageFormat::WebP)
        .map_err(|e| OpError::internal(format!("cannot read the Display Copy: {e}")))?;
    let (width, height) = (image.width(), image.height());
    let (crop_w, crop_h) = if f64::from(width) / f64::from(height) > ASPECT {
        ((f64::from(height) * ASPECT) as u32, height)
    } else {
        (width, (f64::from(width) / ASPECT) as u32)
    };
    let strip = image.crop_imm((width - crop_w) / 2, (height - crop_h) / 2, crop_w, crop_h);
    let strip = if crop_w > WIDTH {
        strip.resize(WIDTH, u32::MAX, FilterType::Lanczos3)
    } else {
        strip
    };
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 82)
        .encode_image(&DynamicImage::ImageRgb8(strip.to_rgb8()))
        .map_err(|e| OpError::internal(format!("cannot encode the Sheet's photograph: {e}")))?;
    Ok(jpeg)
}

// ── Typst ────────────────────────────────────────────────────────────────────

/// The parts of a Typst world that never change between Sheets, built once.
struct Shared {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
}

static SHARED: LazyLock<Shared> = LazyLock::new(|| {
    let fonts: Vec<Font> = FONTS
        .iter()
        .flat_map(|data| Font::iter(Bytes::new(*data)))
        .collect();
    Shared {
        library: LazyHash::new(Library::default()),
        book: LazyHash::new(FontBook::from_fonts(&fonts)),
        fonts,
    }
});

fn file_id(name: &str) -> FileId {
    RootedPath::new(
        VirtualRoot::Project,
        VirtualPath::new(name).expect("a fixed name is a valid path"),
    )
    .intern()
}

/// One Sheet's world: the template, its data, and its photograph. Nothing else
/// exists in it — no file system, no packages, no clock — so a recipe's own
/// text cannot reach anything by being set.
struct SheetWorld {
    main: Source,
    data: Bytes,
    photo: Option<Bytes>,
}

impl World for SheetWorld {
    fn library(&self) -> &LazyHash<Library> {
        &SHARED.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &SHARED.book
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            Ok(self.main.clone())
        } else {
            Err(FileError::AccessDenied)
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == file_id("data.json") {
            return Ok(self.data.clone());
        }
        if id == file_id("photo.jpg")
            && let Some(photo) = &self.photo
        {
            return Ok(photo.clone());
        }
        if id == self.main.id() {
            return Ok(Bytes::from_string(self.main.clone()));
        }
        Err(FileError::AccessDenied)
    }

    fn font(&self, index: usize) -> Option<Font> {
        SHARED.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<typst::foundations::Duration>) -> Option<Datetime> {
        // The date printed arrives worded in the data; the page never asks.
        None
    }
}

/// A set Sheet: the PDF, and how many pages it came to.
pub struct Typeset {
    pub pdf: Vec<u8>,
    pub pages: usize,
}

/// Set one composed Sheet as a PDF.
pub fn typeset(document: &Value, photo: Option<Vec<u8>>) -> Result<Typeset, OpError> {
    let world = SheetWorld {
        main: Source::new(file_id("sheet.typ"), TEMPLATE.to_string()),
        data: Bytes::new(document.to_string().into_bytes()),
        photo: photo.map(Bytes::new),
    };
    let compiled: PagedDocument = typst::compile(&world).output.map_err(|errors| {
        let said: Vec<String> = errors.iter().map(|e| e.message.to_string()).collect();
        OpError::internal(format!("the Sheet could not be set: {}", said.join("; ")))
    })?;
    let pdf = typst_pdf::pdf(&compiled, &typst_pdf::PdfOptions::default()).map_err(|errors| {
        let said: Vec<String> = errors.iter().map(|e| e.message.to_string()).collect();
        OpError::internal(format!(
            "the Sheet could not be written: {}",
            said.join("; ")
        ))
    })?;
    Ok(Typeset {
        pdf,
        pages: compiled.pages().len(),
    })
}

/// A file name for the Sheet a browser saves: the recipe's title.
pub fn file_name(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|ch| {
            if matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                ' '
            } else {
                ch
            }
        })
        .collect();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.is_empty() {
        "Recipe.pdf".to_string()
    } else {
        format!("{cleaned}.pdf")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gathered(content: Value, components: Vec<Value>) -> Gathered {
        Gathered {
            language: "en".into(),
            content,
            about: Vec::new(),
            scaled: None,
            components,
            version_name: None,
            written_at: "2026-09-03T19:07:04.759Z".into(),
            hand: "Aurélien".into(),
            fingerprint: "v_abc".into(),
            share_url: None,
            paper: Paper::A4,
            today: "2026-09-19".into(),
        }
    }

    fn pizza() -> Value {
        json!({
            "title": "Pizza",
            "yield": { "amount": "2", "noun": "pizzas" },
            "prep_time_minutes": 20, "cook_time_minutes": 90,
            "note": null, "main_photo": null, "source": null,
            "ingredients": [
                { "kind": "ingredient", "text": "Dough for 2 pizzas" },
                { "kind": "ingredient", "text": "Sauce, see below" },
            ],
            "steps": [{ "kind": "step", "text": "Bake." }],
        })
    }

    #[test]
    fn the_paper_follows_the_reader_and_is_never_asked() {
        assert_eq!(Paper::for_measures("us"), Paper::Letter);
        assert_eq!(Paper::for_measures("metric"), Paper::A4);
        assert_eq!(Paper::for_measures("as_written"), Paper::A4);
        assert_eq!(Paper::for_locale(Some("en-US,en;q=0.9")), Paper::Letter);
        assert_eq!(Paper::for_locale(Some("fr-CA")), Paper::Letter);
        assert_eq!(Paper::for_locale(Some("en-GB")), Paper::A4);
        assert_eq!(Paper::for_locale(Some("en")), Paper::A4);
        assert_eq!(Paper::for_locale(None), Paper::A4);
    }

    #[test]
    fn dates_and_times_are_written_the_way_a_card_writes_them() {
        assert_eq!(long_date("2026-09-19", &EN), "19 September 2026");
        assert_eq!(long_date("2026-08-01T10:00:00Z", &FR), "1 août 2026");
        assert_eq!(long_date("2026-09-19", &ES), "19 de septiembre de 2026");
        assert_eq!(long_date("someday", &EN), "someday");
        assert_eq!(duration(20), "20 min");
        assert_eq!(duration(90), "1 h 30");
        assert_eq!(duration(120), "2 h");
    }

    /// A Component nobody here holds has no block, and its line reads as
    /// written; one Kamosu stopped at gets a block that says so; one it holds
    /// gets the recipe, and its line says which page to turn to.
    #[test]
    fn a_component_gets_a_block_only_where_there_is_something_to_say() {
        let composed = compose(&gathered(
            pizza(),
            vec![
                json!({
                    "path": [0], "held": true, "stopped": false, "title": "Dough",
                    "share": 0.5,
                    "content": {
                        "title": "Dough", "yield": { "amount": "1", "noun": "kg" },
                        "prep_time_minutes": null, "cook_time_minutes": null,
                        "note": null, "main_photo": null, "source": null,
                        "ingredients": [{ "kind": "ingredient", "text": "600 g flour" }],
                        "steps": [],
                    },
                    "measured": { "ingredients": ["about 300 g"], "steps": [] },
                }),
                json!({
                    "path": [1], "held": false, "stopped": false, "title": null,
                    "share": null, "content": null, "measured": null,
                }),
            ],
        ));
        let lines = &composed["recipe"]["ingredients"];
        assert_eq!(lines[0]["block"], json!(0));
        assert_eq!(lines[0]["names"], json!("Dough"));
        assert_eq!(
            lines[1]["block"],
            json!(null),
            "no block for a recipe not held"
        );
        let blocks = composed["blocks"].as_array().unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(
            blocks[0]["lead"],
            json!("for Pizza  ·  ½ of the recipe  ·  as written, makes 1 kg")
        );
        assert_eq!(
            blocks[0]["recipe"]["ingredients"][0]["about"],
            json!("about 300 g")
        );
        assert_eq!(
            composed["recipe"]["meta"],
            json!("Makes 2 pizzas  ·  Prep 20 min  ·  Cook 1 h 30")
        );
        assert_eq!(composed["recipe"]["scaled"], json!(null));
        assert_eq!(composed["printed"], json!("printed 19 September 2026"));
        assert_eq!(
            composed["provenance"],
            json!(["written 3 September 2026", "Aurélien"])
        );
    }

    #[test]
    fn a_component_kamosu_stopped_at_says_so_where_its_block_would_be() {
        let composed = compose(&gathered(
            pizza(),
            vec![json!({
                "path": [0], "held": true, "stopped": true, "title": "Pizza",
                "share": null, "content": null, "measured": null,
            })],
        ));
        let block = &composed["blocks"][0];
        assert_eq!(block["recipe"], json!(null));
        assert!(
            block["lead"]
                .as_str()
                .unwrap()
                .contains("Kamosu stops here")
        );
        assert_eq!(composed["recipe"]["ingredients"][0]["block"], json!(0));
        assert!(typeset(&composed, None).is_ok(), "the plain line sets");
    }

    fn dough(path: i64, share: f64, language: &str) -> Value {
        json!({
            "path": [path], "lineage_id": "l_dough", "held": true, "stopped": false,
            "title": "Dough", "share": share, "language": language,
            "content": {
                "title": "Dough", "yield": { "amount": "1", "noun": "kg" },
                "prep_time_minutes": null, "cook_time_minutes": null,
                "note": null, "main_photo": null, "source": null,
                "ingredients": [{ "kind": "ingredient", "text": "600 g flour" }],
                "steps": [],
            },
            "measured": { "ingredients": [null], "steps": [] },
        })
    }

    /// Components are appended in order of first mention (ADR 0023): a recipe
    /// two lines ask for at the same amount is printed once, and both lines
    /// turn to it. At two different amounts it is two different blocks, since
    /// one scaled block could not be right for both.
    #[test]
    fn a_component_named_twice_is_printed_once_at_one_amount() {
        let composed = compose(&gathered(
            pizza(),
            vec![dough(0, 0.5, "en"), dough(1, 0.5, "en")],
        ));
        assert_eq!(composed["blocks"].as_array().unwrap().len(), 1);
        assert_eq!(composed["recipe"]["ingredients"][0]["block"], json!(0));
        assert_eq!(composed["recipe"]["ingredients"][1]["block"], json!(0));

        let composed = compose(&gathered(
            pizza(),
            vec![dough(0, 0.5, "en"), dough(1, 0.25, "en")],
        ));
        assert_eq!(composed["blocks"].as_array().unwrap().len(), 2);
        assert_eq!(composed["recipe"]["ingredients"][1]["block"], json!(1));
    }

    /// A Component prints in whatever Language it is held in, marked where
    /// that is not the page's (ADR 0023).
    #[test]
    fn a_component_in_another_language_is_marked() {
        let composed = compose(&gathered(pizza(), vec![dough(0, 0.5, "fr")]));
        let lead = composed["blocks"][0]["lead"].as_str().unwrap().to_string();
        assert!(lead.ends_with("written in French"), "{lead}");
        let composed = compose(&gathered(pizza(), vec![dough(0, 0.5, "en")]));
        let lead = composed["blocks"][0]["lead"].as_str().unwrap().to_string();
        assert!(!lead.contains("written in"), "{lead}");
    }

    /// The template and the fonts are in the binary; setting a page needs
    /// nothing else, and a recipe's text is set as text, never evaluated.
    #[test]
    fn a_sheet_is_set_from_the_binary_alone() {
        let mut content = pizza();
        content["title"] = json!("#panic(\"x\") [*not markup*] $x$");
        let set = typeset(&compose(&gathered(content, Vec::new())), None).expect("set");
        assert!(set.pdf.starts_with(b"%PDF-"));
        assert_eq!(set.pages, 1);
    }
}
