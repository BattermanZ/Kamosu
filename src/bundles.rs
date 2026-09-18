//! A **Bundle**: one recipe's worth of Vault (#66, ADR 0020).
//!
//! A plain zip with UTF-8 filenames. At the top level, one readable Markdown
//! note per recipe, the recipe as it reads today with its **Thread** beneath
//! it. Beside the notes, `photographs/`, holding every Photograph any carried
//! Version shows, byte for byte and under a proper name. Beneath them, the
//! hidden `.kamosu/bundle.json`, holding what a machine needs: every Branch's
//! complete chain of Versions (complete states, never deltas), the Readings
//! beside each one, the ids, and which Lineages the Bundle is **about**.
//!
//! This module writes and never reads the database. The Core gathers what
//! travels, under the Credential that asked, and hands it over as
//! [`Contents`]; everything here is about how that looks on disk. That split
//! is also why nothing in this file can widen what a Bundle carries: a field
//! reaches a Bundle only if the Core put it in `Contents`.
//!
//! **The note's layout is the one Aurélien chose on 18 September 2026** (layout
//! A on #66, "the plain page"): pure CommonMark and nothing any reader has to
//! understand. No front matter, no wiki links, no callouts. Sections are bold
//! lines, a Component is its written line with one quiet line under it linking
//! to the recipe it names, and the Thread follows a rule, newest first like the
//! Share Link page, with a branch nested under the Version it grew from.

use std::collections::{HashMap, HashSet};
use std::io::Write;

use serde_json::{Value, json};

use crate::core::OpError;

/// Where the sidecar sits inside the zip.
pub const SIDECAR: &str = ".kamosu/bundle.json";

/// Where the Photographs sit inside the zip, beside the notes that embed them.
pub const PHOTOGRAPHS: &str = "photographs";

/// Which revision of the sidecar's shape this writer produces. A reader ignores
/// fields it does not recognise rather than refusing them (ADR 0020), so this
/// moves only if a field's *meaning* changes.
pub const FORMAT: i64 = 1;

/// Everything one Bundle carries, gathered by the Core.
pub struct Contents {
    /// The Lineages the Bundle is about. Every carried Branch outside these is
    /// a Passenger.
    pub subjects: Vec<String>,
    /// Every Branch that travels, the shared one first.
    pub branches: Vec<Carried>,
    /// Every Photograph any carried Version shows, keyed by its hash, as its
    /// stored bytes. Never remade on the way out (ADR 0017).
    pub photographs: Vec<(String, Vec<u8>)>,
    /// Photographs a carried Version shows whose bytes this instance could not
    /// read. Named in the sidecar, so a Bundle that is short says so.
    pub missing_photographs: Vec<String>,
}

/// One Branch that travels.
pub struct Carried {
    /// The Branch as the sidecar records it: its ids, Language, Hand, origin
    /// address, Tags, and its complete chain of Versions under `versions`,
    /// oldest first, each with its content and its Readings.
    pub record: Value,
    /// The head Version's Components, as the Core unfolds them for a Passenger
    /// (ADR 0008). The note uses the top-level ones; the sidecar never sees
    /// this, since a Component is a Reading's `lineage_id` and nothing more.
    pub components: Vec<Value>,
}

/// The zip, and the name it goes out under.
pub struct Written {
    pub file_name: String,
    pub bytes: Vec<u8>,
}

/// Write one Bundle.
pub fn write(contents: &Contents) -> Result<Written, OpError> {
    let notes = note_names(&contents.branches);
    let photographs = photograph_names(contents);
    let by_branch: HashMap<&str, usize> = contents
        .branches
        .iter()
        .enumerate()
        .filter_map(|(index, carried)| carried.record["branch_id"].as_str().map(|id| (id, index)))
        .collect();

    let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let deflated = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    // A Photograph is already a compressed picture; deflating it again spends
    // CPU to save nothing (ADR 0020).
    let stored =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for index in 0..contents.branches.len() {
        let text = note(contents, index, &notes, &photographs, &by_branch);
        put(&mut archive, &notes[index], text.as_bytes(), deflated)?;
    }
    for (hash, bytes) in &contents.photographs {
        if let Some(name) = photographs.get(hash) {
            put(&mut archive, name, bytes, stored)?;
        }
    }
    let sidecar = sidecar(contents, &notes, &photographs);
    let sidecar = serde_json::to_vec_pretty(&sidecar)
        .map_err(|e| OpError::internal(format!("cannot write the Bundle's sidecar: {e}")))?;
    put(&mut archive, SIDECAR, &sidecar, deflated)?;

    let bytes = archive
        .finish()
        .map_err(|e| OpError::internal(format!("cannot finish the Bundle: {e}")))?
        .into_inner();
    Ok(Written {
        file_name: file_name(contents),
        bytes,
    })
}

/// The name a Bundle goes out under: the shared recipe's title, as a plain
/// `.zip` with no extension of Kamosu's own (ADR 0020).
pub fn file_name(contents: &Contents) -> String {
    format!(
        "{}.zip",
        file_stem(
            contents
                .branches
                .first()
                .map_or("", |c| title_of(&c.record))
        )
    )
}

fn put(
    archive: &mut zip::ZipWriter<std::io::Cursor<Vec<u8>>>,
    name: &str,
    bytes: &[u8],
    options: zip::write::SimpleFileOptions,
) -> Result<(), OpError> {
    // The zip crate sets the UTF-8 filename flag on any name that is not plain
    // ASCII, which ADR 0020 makes part of the format: legacy encoding mangles
    // `Bœuf bourguignon.md`. A test reads the flag back off the bytes.
    archive
        .start_file(name, options)
        .map_err(|e| OpError::internal(format!("cannot start {name} in the Bundle: {e}")))?;
    archive
        .write_all(bytes)
        .map_err(|e| OpError::internal(format!("cannot write {name} into the Bundle: {e}")))
}

/// The sidecar: the machine half of the Bundle.
///
/// What is **not** here is as deliberate as what is. No Attempt, since nothing
/// the Core gathers reads one. No Access Key: the Core never selects the
/// column. No Food id, Cup Weight or Food nutrition: a Reading's Food travels
/// as its names alone (ADR 0021).
fn sidecar(contents: &Contents, notes: &[String], photographs: &HashMap<String, String>) -> Value {
    let subjects: HashSet<&str> = contents.subjects.iter().map(String::as_str).collect();
    let branches: Vec<Value> = contents
        .branches
        .iter()
        .zip(notes)
        .map(|(carried, note)| {
            let mut record = carried.record.clone();
            let about = record["lineage_id"]
                .as_str()
                .is_some_and(|lineage| subjects.contains(lineage));
            record["subject"] = json!(about);
            record["note"] = json!(note);
            record
        })
        .collect();
    let mut named: Vec<(&String, &String)> = photographs.iter().collect();
    named.sort();
    let photographs: serde_json::Map<String, Value> = named
        .into_iter()
        .map(|(hash, name)| (hash.to_string(), json!(name)))
        .collect();
    json!({
        "kamosu_bundle": FORMAT,
        "subjects": contents.subjects,
        "branches": branches,
        "photographs": photographs,
        "missing_photographs": contents.missing_photographs,
    })
}

// ── Names ────────────────────────────────────────────────────────────────────

/// A name a recipe's title can safely become on every file system a Bundle
/// will be unzipped onto: no path separators, none of the characters Windows
/// refuses, no leading dot (which would hide the note), and not empty.
///
/// `<` and `>` go too, and not only for Windows: a note links to another as
/// `[Title](<Title.md>)`, and those two are what close that link.
pub fn file_stem(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            ch if ch.is_control() => ' ',
            ch => ch,
        })
        .collect();
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim_start_matches('.').trim();
    let short: String = trimmed.chars().take(120).collect();
    let short = short.trim_end_matches(['.', ' ']).to_string();
    if short.is_empty() {
        return "Recipe".to_string();
    }
    // Windows keeps a handful of names for devices, with or without an
    // extension: a recipe called "Con" would unzip to nothing there.
    let device = short
        .split('.')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_uppercase();
    let reserved = matches!(device.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (device.len() == 4
            && (device.starts_with("COM") || device.starts_with("LPT"))
            && device.as_bytes()[3].is_ascii_digit());
    if reserved {
        format!("{short} (recipe)")
    } else {
        short
    }
}

/// One note per carried Branch, named by its title. A Translation sharing its
/// title with the recipe it translates is named with its Language after it;
/// anything still colliding takes a number.
pub fn note_names(branches: &[Carried]) -> Vec<String> {
    let mut taken: HashSet<String> = HashSet::new();
    branches
        .iter()
        .map(|carried| {
            let stem = file_stem(title_of(&carried.record));
            let language = carried.record["language"].as_str().unwrap_or("");
            let candidates = [
                stem.clone(),
                format!("{stem} ({})", language_name(language)),
            ];
            let chosen = candidates
                .into_iter()
                .find(|name| !taken.contains(&name.to_lowercase()))
                .unwrap_or_else(|| numbered(&stem, &taken));
            taken.insert(chosen.to_lowercase());
            format!("{chosen}.md")
        })
        .collect()
}

fn numbered(stem: &str, taken: &HashSet<String>) -> String {
    (2..)
        .map(|n| format!("{stem} ({n})"))
        .find(|name| !taken.contains(&name.to_lowercase()))
        .expect("an unbounded range always finds a free name")
}

/// Every Photograph a proper name (ADR 0020, "every way out of Kamosu writes
/// proper names"), from the first place it is met: heads before older
/// Versions, so a picture on the recipe as it reads today is named for where
/// it sits today. A Main Photo is the recipe's title; a Step's picture is the
/// title and the Step's number.
fn photograph_names(contents: &Contents) -> HashMap<String, String> {
    let present: HashSet<&str> = contents
        .photographs
        .iter()
        .map(|(hash, _)| hash.as_str())
        .collect();
    let mut names: HashMap<String, String> = HashMap::new();
    let mut taken: HashSet<String> = HashSet::new();
    let mut name = |hash: &str, wanted: String, names: &mut HashMap<String, String>| {
        if !present.contains(hash) || names.contains_key(hash) {
            return;
        }
        let stem = if taken.contains(&wanted.to_lowercase()) {
            numbered(&wanted, &taken)
        } else {
            wanted
        };
        taken.insert(stem.to_lowercase());
        names.insert(hash.to_string(), format!("{PHOTOGRAPHS}/{stem}.webp"));
    };

    for carried in &contents.branches {
        let versions = carried.record["versions"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default();
        for version in versions.iter().rev() {
            let content = &version["content"];
            let title = file_stem(content["title"].as_str().unwrap_or(""));
            if let Some(hash) = content["main_photo"].as_str() {
                name(hash, title.clone(), &mut names);
            }
            for (number, step) in numbered_steps(content) {
                if let Some(hash) = step["photo"].as_str() {
                    name(hash, format!("{title}, step {number}"), &mut names);
                }
            }
        }
    }
    names
}

fn language_name(language: &str) -> &str {
    match language {
        "en" => "English",
        "fr" => "Français",
        "es" => "Español",
        other => other,
    }
}

// ── The note ─────────────────────────────────────────────────────────────────

/// The words a note writes of its own, in the recipe's Language. The same
/// words the Share Link page uses wherever the two say the same thing, since
/// ADR 0020 puts the Thread in the note "in the same words the Share Link page
/// uses".
struct Words {
    ingredients: &'static str,
    method: &'static str,
    note: &'static str,
    tags: &'static str,
    from_source: &'static str,
    history: &'static str,
    written_down: &'static str,
    min_prep: &'static str,
    min_cook: &'static str,
    see: &'static str,
    branched_here: &'static str,
    translated_from: &'static str,
}

const EN: Words = Words {
    ingredients: "Ingredients",
    method: "Method",
    note: "Note",
    tags: "Tags",
    from_source: "From",
    history: "Everything this recipe has been",
    written_down: "Written down",
    min_prep: "min prep",
    min_cook: "min cook",
    see: "see",
    branched_here: "Branched here",
    translated_from: "Translated from",
};

const FR: Words = Words {
    ingredients: "Ingrédients",
    method: "Préparation",
    note: "Note",
    tags: "Étiquettes",
    from_source: "D'après",
    history: "Tout ce que cette recette a été",
    written_down: "Écrite",
    min_prep: "min prép.",
    min_cook: "min cuisson",
    see: "voir",
    branched_here: "Partie d'ici",
    translated_from: "Traduite de",
};

const ES: Words = Words {
    ingredients: "Ingredientes",
    method: "Preparación",
    note: "Nota",
    tags: "Etiquetas",
    from_source: "De",
    history: "Todo lo que esta receta ha sido",
    written_down: "Escrita",
    min_prep: "min prep.",
    min_cook: "min cocción",
    see: "ver",
    branched_here: "Partió de aquí",
    translated_from: "Traducida de",
};

fn words(language: &str) -> &'static Words {
    match language {
        "fr" => &FR,
        "es" => &ES,
        _ => &EN,
    }
}

/// One Branch's note: the recipe as it reads today, then its Thread.
fn note(
    contents: &Contents,
    index: usize,
    notes: &[String],
    photographs: &HashMap<String, String>,
    by_branch: &HashMap<&str, usize>,
) -> String {
    let carried = &contents.branches[index];
    let record = &carried.record;
    let words = words(record["language"].as_str().unwrap_or(""));
    let empty = Value::Null;
    let content = head(record).map_or(&empty, |version| &version["content"]);
    let title = content["title"].as_str().unwrap_or("");

    let mut out = format!("# {}\n\n", inline(title));

    if let Some(name) = content["main_photo"]
        .as_str()
        .and_then(|hash| photographs.get(hash))
    {
        out.push_str(&format!("![{}](<{name}>)\n\n", inline(title)));
    }

    let mut meta = Vec::new();
    let recipe_yield = [
        content["yield"]["amount"].as_str().unwrap_or(""),
        content["yield"]["noun"].as_str().unwrap_or(""),
    ]
    .join(" ");
    if !recipe_yield.trim().is_empty() {
        meta.push(inline(recipe_yield.trim()));
    }
    if let Some(minutes) = content["prep_time_minutes"].as_i64() {
        meta.push(format!("{minutes} {}", words.min_prep));
    }
    if let Some(minutes) = content["cook_time_minutes"].as_i64() {
        meta.push(format!("{minutes} {}", words.min_cook));
    }
    if !meta.is_empty() {
        out.push_str(&meta.join(" · "));
        out.push_str("\n\n");
    }
    if let Some(text) = content["source"]["text"].as_str().filter(|t| !t.is_empty()) {
        let source = match content["source"]["link"].as_str().filter(|l| !l.is_empty()) {
            Some(link) => format!("[{}](<{}>)", inline(text), link.replace(['<', '>'], "")),
            None => inline(text),
        };
        out.push_str(&format!("{} {source}\n\n", words.from_source));
    }

    // A heading over nothing says the recipe lost something it never had: a
    // Translation started from a title alone is an ordinary thing to share.
    let listed = ingredients(content, &carried.components, notes, by_branch, words);
    if !listed.is_empty() {
        out.push_str(&format!("## {}\n\n{listed}", words.ingredients));
    }
    let method = steps(content, photographs);
    if !method.is_empty() {
        out.push_str(&format!("## {}\n\n{method}", words.method));
    }

    let tags = tag_names(record);
    if let Some(text) = content["note"].as_str().filter(|n| !n.trim().is_empty()) {
        out.push_str(&format!("## {}\n\n", words.note));
        for paragraph in text.trim().split("\n\n") {
            out.push_str(&block(paragraph, ""));
            out.push_str("\n\n");
        }
    }
    if !tags.is_empty() {
        out.push_str(&format!("{}: {}\n\n", words.tags, tags.join(", ")));
    }

    out.push_str("---\n\n");
    out.push_str(&format!("## {}\n\n", words.history));
    out.push_str(&thread(contents, index, notes, words, ""));
    out
}

/// The Ingredients, Sections as bold lines, each Component with one quiet line
/// under it naming the recipe it uses and linking to that recipe's own note.
fn ingredients(
    content: &Value,
    components: &[Value],
    notes: &[String],
    by_branch: &HashMap<&str, usize>,
    words: &Words,
) -> String {
    let mut out = String::new();
    let mut in_list = false;
    let items = content["ingredients"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    for (index, item) in items.iter().enumerate() {
        let text = item["text"].as_str().unwrap_or("");
        if item["kind"].as_str() == Some("section") {
            if in_list {
                out.push('\n');
                in_list = false;
            }
            out.push_str(&format!("**{}**\n\n", inline(text)));
            continue;
        }
        in_list = true;
        out.push_str(&format!("- {}\n", block(text, "  ")));
        let component = components.iter().find(|component| {
            component["path"]
                .as_array()
                .is_some_and(|path| path.len() == 1 && path[0].as_u64() == Some(index as u64))
        });
        // Only a Component the Bundle actually carries gets its line: one it
        // does not hold leaves the written line to say what it means, which is
        // all ADR 0002 ever promised it would.
        if let Some(component) = component
            && let Some(target) = component["branch_id"]
                .as_str()
                .and_then(|branch| by_branch.get(branch))
        {
            let said = component["said"].as_str().unwrap_or("");
            let title = component["title"].as_str().unwrap_or("");
            out.push_str(&format!(
                "  - *{}*, {} [{}](<{}>)\n",
                inline(said),
                words.see,
                inline(title),
                notes[*target]
            ));
        }
    }
    if in_list {
        out.push('\n');
    }
    out
}

/// The Method: Sections as bold lines, Steps numbered straight through them,
/// and a Step's picture embedded under it.
fn steps(content: &Value, photographs: &HashMap<String, String>) -> String {
    let mut out = String::new();
    let mut in_list = false;
    let mut number = 0;
    let items = content["steps"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    for item in items {
        let text = item["text"].as_str().unwrap_or("");
        if item["kind"].as_str() == Some("section") {
            if in_list {
                out.push('\n');
                in_list = false;
            }
            out.push_str(&format!("**{}**\n\n", inline(text)));
            continue;
        }
        number += 1;
        in_list = true;
        let marker = format!("{number}. ");
        let indent = " ".repeat(marker.len());
        out.push_str(&format!("{marker}{}\n", block(text, &indent)));
        if let Some(name) = item["photo"]
            .as_str()
            .and_then(|hash| photographs.get(hash))
        {
            out.push_str(&format!("{indent}![](<{name}>)\n"));
        }
    }
    if in_list {
        out.push('\n');
    }
    out
}

/// A Branch's Thread, newest first: each Version's name, its Hand and date, and
/// its *what changed* line. A carried Branch that grew from one of these
/// Versions is nested under it, with its own Thread beneath.
fn thread(
    contents: &Contents,
    index: usize,
    notes: &[String],
    words: &Words,
    indent: &str,
) -> String {
    let record = &contents.branches[index].record;
    let versions = record["versions"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let mut out = String::new();
    for version in versions.iter().rev() {
        out.push_str(&format!("{indent}- {}\n", version_line(version, words)));
        if let Some(said) = version["change_note"].as_str().filter(|s| !s.is_empty()) {
            // Two trailing spaces end the line above as a hard break, so the
            // *what changed* line sits under its Version rather than running
            // on after the date.
            out.pop();
            out.push_str(&format!(
                "  \n{indent}  {}\n",
                block(said, &format!("{indent}  "))
            ));
        }
        // Only at the top level: a nested Branch's own Thread is written out
        // in full in its own note, so going deeper here would print it twice.
        if indent.is_empty() {
            for (other, carried) in contents.branches.iter().enumerate() {
                if other == index || !grew_from(&carried.record, version) {
                    continue;
                }
                let language = carried.record["language"].as_str().unwrap_or("");
                let named = format!(
                    "{} ({})",
                    title_of(&carried.record),
                    language_name(language)
                );
                out.push_str(&format!(
                    "  - *{}:* **{}**, {} [{}](<{}>)\n",
                    words.branched_here,
                    inline(&named),
                    words.see,
                    inline(&named),
                    notes[other]
                ));
                out.push_str(&thread(contents, other, notes, words, "    "));
            }
        }
    }

    // A Translation says what it translates, where that travels too.
    if indent.is_empty()
        && let Some(source) = versions
            .first()
            .and_then(|first| first["translates_version_id"].as_str())
        && let Some((other, version)) = contents
            .branches
            .iter()
            .enumerate()
            .filter(|(other, _)| *other != index)
            .find_map(|(other, carried)| {
                carried.record["versions"]
                    .as_array()?
                    .iter()
                    .find(|v| v["version_id"].as_str() == Some(source))
                    .map(|version| (other, version))
            })
    {
        let title = title_of(&contents.branches[other].record);
        out.push_str(&format!(
            "- *{}* **{}**, {} [{}](<{}>)\n",
            words.translated_from,
            inline(&version_label(version, words)),
            words.see,
            inline(title),
            notes[other]
        ));
    }
    out
}

/// Whether a Branch's chain begins by growing out of this Version: a
/// Translation rendering it, or a fork whose first Version's parent it is.
fn grew_from(record: &Value, version: &Value) -> bool {
    let Some(id) = version["version_id"].as_str() else {
        return false;
    };
    let Some(first) = record["versions"].as_array().and_then(|v| v.first()) else {
        return false;
    };
    first["translates_version_id"].as_str() == Some(id)
        || first["parent_version_id"].as_str() == Some(id)
}

fn version_line(version: &Value, words: &Words) -> String {
    format!(
        "**{}** · {} · {}",
        inline(&version_label(version, words)),
        inline(version["hand"]["name"].as_str().unwrap_or("")),
        version["created_at"]
            .as_str()
            .unwrap_or("")
            .get(..10)
            .unwrap_or("")
    )
}

/// A Version's name, or what the Share Link page calls one with none.
fn version_label(version: &Value, words: &Words) -> String {
    if let Some(name) = version["name"].as_str().filter(|n| !n.is_empty()) {
        return name.to_string();
    }
    match version["sequence"].as_i64() {
        Some(1) | None => words.written_down.to_string(),
        Some(n) => format!("{} {n}", words.written_down),
    }
}

/// Each Tag in the Branch's own Language where it has a name in it, and
/// otherwise in whichever it has.
fn tag_names(record: &Value) -> Vec<String> {
    let language = record["language"].as_str().unwrap_or("");
    record["tags"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|tag| {
            let names = tag["names"].as_object()?;
            names
                .get(language)
                .or_else(|| names.values().next())
                .and_then(Value::as_str)
                .map(inline)
        })
        .collect()
}

/// The head Version of a carried Branch: the last in its chain.
pub fn head(record: &Value) -> Option<&Value> {
    record["versions"].as_array().and_then(|v| v.last())
}

/// A carried Branch's title as it reads today.
pub fn title_of(record: &Value) -> &str {
    head(record)
        .and_then(|version| version["content"]["title"].as_str())
        .unwrap_or("")
}

/// Each Step with its number, Sections skipped and not counted.
fn numbered_steps(content: &Value) -> Vec<(usize, &Value)> {
    content["steps"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter(|step| step["kind"].as_str() != Some("section"))
        .enumerate()
        .map(|(index, step)| (index + 1, step))
        .collect()
}

/// A piece of somebody's typing, made safe to sit inside a line of Markdown.
///
/// Only what would change the *shape* of the note is escaped: emphasis, code,
/// link brackets, a raw HTML tag, an entity, and a mark at the start of the
/// text that would turn the line into a heading, a quote or a list. Everything
/// else is left alone, because the note is read in plain text editors too, and
/// a backslash before every full stop would be the note failing at its job.
pub fn inline(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if matches!(ch, '\\' | '*' | '_' | '`' | '[' | ']' | '<' | '&') {
            out.push('\\');
        }
        out.push(ch);
    }
    guard_start(&out)
}

/// Text that may run over several lines, each continuation indented to stay
/// inside the list item or paragraph it belongs to, and joined by hard breaks
/// so a line the writer broke stays broken.
fn block(text: &str, indent: &str) -> String {
    text.lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(inline)
        .collect::<Vec<_>>()
        .join(&format!("  \n{indent}"))
}

/// Escape a mark at the very start of the text that would make a block of it.
fn guard_start(text: &str) -> String {
    let trimmed = text.trim_start();
    let lead = &text[..text.len() - trimmed.len()];
    if trimmed.starts_with(['#', '>', '-', '+', '=', '|']) {
        return format!("{lead}\\{trimmed}");
    }
    let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 && trimmed[digits..].starts_with(['.', ')']) {
        return format!("{lead}{}\\{}", &trimmed[..digits], &trimmed[digits..]);
    }
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_title_becomes_a_name_every_file_system_takes() {
        assert_eq!(file_stem("Bœuf bourguignon"), "Bœuf bourguignon");
        assert_eq!(file_stem("Salt / pepper: a study?"), "Salt pepper a study");
        assert_eq!(file_stem("..hidden"), "hidden");
        assert_eq!(file_stem("   "), "Recipe");
        assert_eq!(file_stem("<b>Bold</b>"), "b Bold b");
        assert_eq!(file_stem(&"x".repeat(300)).chars().count(), 120);
        assert_eq!(file_stem("Con"), "Con (recipe)");
        assert_eq!(file_stem("com1.txt"), "com1.txt (recipe)");
        assert_eq!(file_stem("Console"), "Console");
    }

    #[test]
    fn typing_cannot_change_the_shape_of_the_note() {
        assert_eq!(inline("# not a heading"), "\\# not a heading");
        assert_eq!(inline("1. not a list"), "1\\. not a list");
        assert_eq!(inline("- not a list"), "\\- not a list");
        assert_eq!(inline("<script>"), "\\<script>");
        assert_eq!(inline("salt & pepper"), "salt \\& pepper");
        assert_eq!(inline("*bold*"), "\\*bold\\*");
        assert_eq!(inline("2 tbsp (Note 1)"), "2 tbsp (Note 1)");
    }

    #[test]
    fn a_line_the_writer_broke_stays_broken_inside_its_item() {
        assert_eq!(block("one\ntwo", "   "), "one  \n   two");
    }

    fn version(sequence: i64, id: &str, parent: Option<&str>, name: Option<&str>) -> Value {
        json!({
            "sequence": sequence,
            "version_id": id,
            "parent_version_id": parent,
            "translates_version_id": null,
            "name": name,
            "change_note": null,
            "created_at": "2026-06-02T10:00:00.000Z",
            "hand": { "id": "p_1", "name": "Aurélien" },
            "signature": null,
            "content": { "title": "Bœuf bourguignon", "ingredients": [], "steps": [] },
            "readings": [],
        })
    }

    fn contents() -> Contents {
        Contents {
            subjects: vec!["l_1".into()],
            branches: vec![Carried {
                record: json!({
                    "branch_id": "b_1",
                    "lineage_id": "l_1",
                    "language": "fr",
                    "hand": { "id": "k_1", "name": "Maison" },
                    "origin_address": null,
                    "tags": [],
                    "versions": [
                        version(1, "v_1", None, None),
                        version(2, "v_2", Some("v_1"), Some("Moins de vin")),
                    ],
                }),
                components: vec![],
            }],
            photographs: vec![],
            missing_photographs: vec![],
        }
    }

    /// ADR 0020: the UTF-8 filename flag is spec, not an implementation detail.
    /// General-purpose bit 11 of every local header naming a non-ASCII file.
    #[test]
    fn a_name_that_is_not_ascii_carries_the_utf8_flag() {
        let written = write(&contents()).expect("writes");
        assert_eq!(written.file_name, "Bœuf bourguignon.zip");
        let name = "Bœuf bourguignon.md".as_bytes();
        let bytes = &written.bytes;
        let header = bytes
            .windows(4)
            .enumerate()
            .filter(|(_, w)| *w == [0x50, 0x4b, 0x03, 0x04])
            .map(|(at, _)| at)
            .find(|at| {
                let length = u16::from_le_bytes([bytes[at + 26], bytes[at + 27]]) as usize;
                bytes.get(at + 30..at + 30 + length) == Some(name)
            })
            .expect("the note has a local header");
        let flags = u16::from_le_bytes([bytes[header + 6], bytes[header + 7]]);
        assert_ne!(flags & (1 << 11), 0, "the UTF-8 flag is set");
    }

    #[test]
    fn the_thread_is_newest_first_in_the_recipes_own_words() {
        let written = write(&contents()).expect("writes");
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(written.bytes)).expect("a zip");
        let mut text = String::new();
        std::io::Read::read_to_string(
            &mut archive.by_name("Bœuf bourguignon.md").expect("the note"),
            &mut text,
        )
        .expect("reads");
        let newest = text.find("**Moins de vin**").expect("the named Version");
        let oldest = text.find("**Écrite**").expect("the unnamed first Version");
        assert!(newest < oldest, "newest first:\n{text}");
        assert!(text.contains("## Tout ce que cette recette a été"));
        assert!(
            !text.contains("## Ingrédients") && !text.contains("## Préparation"),
            "a recipe with nothing listed gets no heading over nothing:\n{text}"
        );
    }
}
