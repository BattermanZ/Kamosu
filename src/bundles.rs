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
use std::path::{Path, PathBuf};

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
    /// The Branch's **Local id**, which is **not** what the record carries and
    /// never leaves this process (#90): the sidecar holds the Travelling id,
    /// and a Component found while gathering names the local one. This is what
    /// matches the two up.
    pub local_id: String,
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
    // Keyed on the Local id, because that is what a Component names. The
    // record's `branch_id` is the Travelling id, and the two differ on every
    // Branch that arrived here from somewhere else (#90).
    let by_branch: HashMap<&str, usize> = contents
        .branches
        .iter()
        .enumerate()
        .map(|(index, carried)| (carried.local_id.as_str(), index))
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

// ── Keeping one, for a stranger (#65, ADR 0032) ──────────────────────────────
//
// A Person asking for their own Bundle builds it: they are one Credential and
// they ask once. A Share Link is held by however many people it was passed to,
// and building a zip means reading every Photograph off disk and deflating
// every note, so a link on a busy day would be work that scales with the
// strangers holding it. The same recipe makes the same bytes, so it is built
// once and kept, exactly as a Sheet and a share card already are.

/// Where kept Bundles live. Everything here rebuilds from the recipes in a
/// moment, so nothing in this directory is truth (ADR 0032); [`prune`] empties
/// it of anything older than a day.
pub fn bundles_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("bundles")
}

/// How long a kept Bundle is kept. A day, which is long enough for a link
/// passed round a family on one evening and short enough that a withdrawn
/// recipe's bytes do not sit on disk.
const KEPT_FOR: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);

/// Throw away kept Bundles older than a day. Called whenever one is asked for,
/// which bounds the directory by how many different recipes a day asks for.
pub fn prune(data_dir: &Path) {
    let Ok(entries) = std::fs::read_dir(bundles_dir(data_dir)) else {
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

/// **What a Bundle is, as a key.** Every carried Branch exactly as the sidecar
/// will record it, and which Photographs travel with them.
///
/// Keying on the shared Branch's head Version alone would be wrong: a Bundle
/// carries its Translations and its Passengers too, and any of those may move
/// while the shared recipe stands still. Hashing every record catches all of
/// them, and hashing the Photographs' hashes rather than their bytes keeps this
/// cheap — a Photograph is known by its contents already (ADR 0017), so a
/// picture that changed has a different hash.
pub fn key_of(contents: &Contents, present: &[String], missing: &[String]) -> String {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    let mut feed = |bytes: &[u8]| {
        hash.update(bytes);
        hash.update([0]);
    };
    // The writer, not only what it is given. A kept zip outlives an upgrade by
    // up to a day, and the sidecar's shape is the one part of that which is
    // declared and can be checked for.
    feed(FORMAT.to_string().as_bytes());
    for subject in &contents.subjects {
        feed(subject.as_bytes());
    }
    for carried in &contents.branches {
        feed(crate::fingerprint::canonical_json(&carried.record).as_bytes());
        for component in &carried.components {
            feed(crate::fingerprint::canonical_json(component).as_bytes());
        }
    }
    for photograph in present {
        feed(photograph.as_bytes());
    }
    for photograph in missing {
        feed(b"missing");
        feed(photograph.as_bytes());
    }
    format!("b_{}", hex::encode(hash.finalize()))
}

/// The kept Bundle for a key, if one was written within the day.
pub fn kept(data_dir: &Path, key: &str) -> Option<Vec<u8>> {
    std::fs::read(bundles_dir(data_dir).join(format!("{key}.zip"))).ok()
}

/// Keep a written Bundle under its key.
///
/// **Written whole or not at all.** Unlike a Sheet, which one Job lane sets,
/// this cache's readers are concurrent strangers by design: two people opening
/// the same link at once both build, both write, and a third arriving mid-write
/// would read a truncated zip and be handed it as a Bundle. Writing beside it
/// and renaming makes the swap atomic, since a rename within one directory
/// either happened or did not.
///
/// Best effort otherwise, like a kept share card: the zip in hand is a complete
/// Bundle, and refusing to hand it over because a directory is unwritable would
/// turn a disk problem into a broken button on somebody's page.
pub fn keep(data_dir: &Path, key: &str, bytes: &[u8]) {
    let directory = bundles_dir(data_dir);
    if std::fs::create_dir_all(&directory).is_err() {
        return;
    }
    // Unique per writer: two strangers on one token are two threads of one
    // process, so the process id alone would have them writing the same file.
    static WRITER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let writer = WRITER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let beside = directory.join(format!(".{key}.{}.{writer}.part", std::process::id()));
    if std::fs::write(&beside, bytes).is_ok()
        && std::fs::rename(&beside, directory.join(format!("{key}.zip"))).is_err()
    {
        let _ = std::fs::remove_file(&beside);
    }
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
    history: "History",
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
    history: "Historique",
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
    history: "Historial",
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

// ── Opening one (#67) ────────────────────────────────────────────────────────

/// The most of any one file inside a Bundle that is ever read. A zip says how
/// big its files are and can lie about it, so this is the ceiling rather than
/// the header: a sidecar or a Photograph larger than this is not something any
/// Kamosu wrote, and inflating it would spend the instance's memory on a file
/// built to do exactly that.
const MOST_READ: u64 = 64 * 1024 * 1024;

/// A Bundle opened: its sidecar, and every Photograph the sidecar names.
pub struct Opened {
    pub sidecar: Value,
    pub photographs: Vec<CarriedPhotograph>,
}

/// A file that could not be opened as a Bundle: why, and the words of every
/// recipe note it still holds, read back out as best they can be.
pub struct Unopened {
    pub reason: String,
    /// `(note file name, recipe content)` for every note that yielded a title.
    pub notes: Vec<(String, Value)>,
}

/// One Photograph a Bundle names, as it turned out once opened.
pub enum CarriedPhotograph {
    /// The bytes are the picture the sidecar says: they hash to its id (ADR 0017).
    Sound { hash: String, bytes: Vec<u8> },
    /// Absent, or not that picture. `name` is the file inside the Bundle,
    /// which is what a person looking at it would recognise.
    Unusable { name: String, reason: String },
}

/// Open a Bundle's bytes.
///
/// A Bundle is read from its sidecar, since that is where the complete states
/// and the ids are (ADR 0020). One whose sidecar is missing or unreadable is
/// the folder ADR 0003 describes arriving with no `.kamosu/` — "imported
/// best-effort from the notes, and says so" — so its notes are read back
/// instead, and the caller keeps their words.
pub fn open(bytes: &[u8]) -> Result<Opened, Unopened> {
    let Ok(mut archive) = zip::ZipArchive::new(std::io::Cursor::new(bytes)) else {
        return Err(Unopened {
            reason: "this file is not a Bundle: it is not a zip".to_string(),
            notes: Vec::new(),
        });
    };
    let sidecar = match read_entry(&mut archive, SIDECAR) {
        None => Err(format!("it has no {SIDECAR}")),
        Some(text) => match serde_json::from_slice::<Value>(&text) {
            Ok(sidecar) if sidecar["branches"].is_array() => Ok(sidecar),
            _ => Err(format!("its {SIDECAR} cannot be read")),
        },
    };
    let sidecar = match sidecar {
        Ok(sidecar) => sidecar,
        Err(why) => {
            let notes = read_notes(&mut archive);
            let reason = if notes.is_empty() {
                format!("this zip is not a Bundle: {why}, and no recipe note in it could be read")
            } else {
                why
            };
            return Err(Unopened { reason, notes });
        }
    };

    let named: Vec<(String, String)> = sidecar["photographs"]
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(hash, name)| Some((hash.clone(), name.as_str()?.to_string())))
        .collect();
    let photographs = named
        .into_iter()
        .map(|(hash, name)| match read_entry(&mut archive, &name) {
            None => CarriedPhotograph::Unusable {
                name,
                reason: "this photograph is missing from the Bundle".to_string(),
            },
            Some(bytes) if crate::photographs::hash_bytes(&bytes) == hash => {
                CarriedPhotograph::Sound { hash, bytes }
            }
            Some(_) => CarriedPhotograph::Unusable {
                name,
                reason: "this photograph is not the picture the Bundle says it is, so it was \
                         left out; the recipe shows no picture there"
                    .to_string(),
            },
        })
        .collect();
    Ok(Opened {
        sidecar,
        photographs,
    })
}

/// Every top-level recipe note in the zip, read back into a recipe's words.
fn read_notes(archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>) -> Vec<(String, Value)> {
    let mut names: Vec<String> = archive
        .file_names()
        .filter(|name| name.ends_with(".md") && !name.contains('/'))
        .map(str::to_string)
        .collect();
    names.sort();
    names
        .into_iter()
        .filter_map(|name| {
            let text = String::from_utf8(read_entry(archive, &name)?).ok()?;
            Some((name, read_note(&text)?))
        })
        .collect()
}

/// A note read back into a recipe's words: the title, the Ingredients and the
/// Method with their Sections, and the Note — the parts of the layout `note`
/// writes that are the recipe as it reads today. Everything else in it is
/// Kamosu's own furniture or history, and a note with no machine half behind
/// it carries no history anyone can trust. `None` when there is no title,
/// since a recipe needs one and nothing less is a recipe (#43).
///
/// Best effort, as ADR 0003 promises and no more: a note somebody has edited
/// by hand reads as far as it still looks like one Kamosu wrote.
pub fn read_note(text: &str) -> Option<Value> {
    #[derive(PartialEq)]
    enum Part {
        Other,
        Ingredients,
        Method,
        Note,
    }
    let heading = |title: &str| {
        let title = title.trim();
        [&EN, &FR, &ES]
            .iter()
            .find_map(|words| {
                if title == words.ingredients {
                    Some(Part::Ingredients)
                } else if title == words.method {
                    Some(Part::Method)
                } else if title == words.note {
                    Some(Part::Note)
                } else {
                    None
                }
            })
            .unwrap_or(Part::Other)
    };

    let mut title: Option<String> = None;
    let mut part = Part::Other;
    let mut ingredients: Vec<Value> = Vec::new();
    let mut steps: Vec<Value> = Vec::new();
    let mut note: Vec<String> = Vec::new();
    // The list item a continuation line belongs to: the written line break
    // that `block` turned into a hard break comes back as a newline.
    let mut open = false;
    for line in text.lines() {
        if line.trim() == "---" {
            break;
        }
        if let Some(rest) = line.strip_prefix("# ") {
            title.get_or_insert_with(|| unescape(rest.trim()));
            continue;
        }
        if let Some(rest) = line.strip_prefix("## ") {
            part = heading(rest);
            open = false;
            continue;
        }
        let section = line
            .strip_prefix("**")
            .and_then(|rest| rest.strip_suffix("**"))
            .filter(|_| !line.starts_with("***"));
        match part {
            Part::Ingredients | Part::Method => {
                let list = if part == Part::Ingredients {
                    &mut ingredients
                } else {
                    &mut steps
                };
                if let Some(text) = section {
                    list.push(json!({ "kind": "section", "text": unescape(text) }));
                    open = false;
                    continue;
                }
                let item = if part == Part::Ingredients {
                    line.strip_prefix("- ")
                } else {
                    let digits = line.chars().take_while(char::is_ascii_digit).count();
                    (digits > 0)
                        .then(|| line[digits..].strip_prefix(". "))
                        .flatten()
                };
                if let Some(text) = item {
                    let kind = if part == Part::Ingredients {
                        "ingredient"
                    } else {
                        "step"
                    };
                    list.push(json!({ "kind": kind, "text": unescape(text.trim_end()) }));
                    open = true;
                    continue;
                }
                let indented = line.starts_with(' ');
                let trimmed = line.trim();
                // A Component's quiet line and a Step's picture are Kamosu's
                // furniture, not the recipe's words.
                let furniture = trimmed.starts_with("- ") || trimmed.starts_with("![");
                match list.last_mut() {
                    Some(item) if open && indented && !trimmed.is_empty() && !furniture => {
                        let held = item["text"].as_str().unwrap_or_default().to_string();
                        item["text"] = json!(format!("{held}\n{}", unescape(trimmed)));
                    }
                    _ if trimmed.is_empty() || indented => {}
                    _ => open = false,
                }
            }
            Part::Note => {
                // The Tags line follows the Note, and is filing, not words.
                let tags = [&EN, &FR, &ES]
                    .iter()
                    .any(|words| line.starts_with(&format!("{}: ", words.tags)));
                if tags {
                    continue;
                }
                if !line.trim().is_empty() {
                    note.push(unescape(line.trim()));
                } else if note.last().is_some_and(|held| !held.is_empty()) {
                    note.push(String::new());
                }
            }
            Part::Other => {}
        }
    }
    let title = title.filter(|title| !title.is_empty())?;
    let note = note.join("\n").trim().replace("\n\n\n", "\n\n");
    Some(json!({
        "title": title,
        "ingredients": ingredients,
        "steps": steps,
        "note": if note.is_empty() { Value::Null } else { json!(note) },
    }))
}

/// Undo `inline`: every backslash it put before a mark comes back out.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek().is_some_and(|next| next.is_ascii_punctuation()) {
            continue;
        }
        out.push(ch);
    }
    out
}

fn read_entry(
    archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
    name: &str,
) -> Option<Vec<u8>> {
    let entry = archive.by_name(name).ok()?;
    let mut held = Vec::new();
    std::io::Read::read_to_end(&mut std::io::Read::take(entry, MOST_READ + 1), &mut held).ok()?;
    (held.len() as u64 <= MOST_READ).then_some(held)
}

/// What is wrong with a carried Branch's history, in words, or `None` when it
/// is whole.
///
/// Whole means what ADR 0018 made it mean: the chain reaches the first
/// Version, each Version following the one before it, and each Version's
/// contents are what its id was taken from — the receiver recomputes every
/// fingerprint, so damage is arithmetic rather than trust (ADR 0020). The ids
/// that place the Branch, and the Hand on it and on each Version, have to be
/// there too, or there is nothing to place it under: a Version's Hand is a
/// column no row can be without. A missing date is not damage; the Version
/// takes the day it arrived.
///
/// Nothing else is checked. A field this writer does not know is not damage
/// (ADR 0020), and the Hand is never verified (ADR 0015).
pub fn damage(record: &Value) -> Option<String> {
    for field in ["branch_id", "lineage_id"] {
        if record[field].as_str().is_none_or(str::is_empty) {
            return Some(format!("it carries no {}", field.replace('_', " ")));
        }
    }
    if record["hand"]["id"].as_str().is_none_or(str::is_empty) {
        return Some("it does not say which Kitchen wrote it".to_string());
    }
    let versions = match record["versions"].as_array() {
        Some(versions) if !versions.is_empty() => versions,
        _ => return Some("it carries no Versions".to_string()),
    };
    let mut previous: Option<&str> = None;
    for (index, version) in versions.iter().enumerate() {
        let number = index + 1;
        let Some(id) = version["version_id"].as_str() else {
            return Some(format!("Version {number} has no id"));
        };
        if version["sequence"].as_u64() != Some(number as u64)
            || version["parent_version_id"].as_str() != previous
        {
            return Some(if index == 0 {
                "its history does not reach its first Version".to_string()
            } else {
                format!("its history has a gap before Version {number}")
            });
        }
        if !version["content"].is_object()
            || crate::fingerprint::fingerprint_content(&version["content"]) != id
        {
            return Some(format!(
                "the words of Version {number} do not match the id they were saved under"
            ));
        }
        if version["hand"]["id"].as_str().is_none_or(str::is_empty) {
            return Some(format!("Version {number} does not say who wrote it"));
        }
        previous = Some(id);
    }
    None
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

    /// A note is read back into the words it was written from — the escaping
    /// undone, a broken line still broken, the furniture left behind.
    #[test]
    fn a_note_reads_back_into_the_words_it_was_written_from() {
        let mut written = contents();
        written.branches[0].record["versions"][1]["content"] = json!({
            "title": "Bœuf *bourguignon*",
            "note": "Better the next day.\n\n# Not a heading",
            "ingredients": [
                { "kind": "section", "text": "Pour la sauce" },
                { "kind": "ingredient", "text": "1 bouteille de vin\nrouge, de Bourgogne" },
            ],
            "steps": [
                { "kind": "step", "text": "1. Mariner une nuit." },
                { "kind": "step", "text": "Cuire trois heures.", "photo": "p_x" },
            ],
        });
        let written = write(&written).expect("writes");
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(written.bytes)).expect("a zip");
        let name = archive
            .file_names()
            .find(|name| name.ends_with(".md"))
            .expect("a note")
            .to_string();
        let mut text = String::new();
        std::io::Read::read_to_string(&mut archive.by_name(&name).unwrap(), &mut text).unwrap();
        assert_eq!(
            read_note(&text).expect("a title"),
            json!({
                "title": "Bœuf *bourguignon*",
                "note": "Better the next day.\n\n# Not a heading",
                "ingredients": [
                    { "kind": "section", "text": "Pour la sauce" },
                    { "kind": "ingredient", "text": "1 bouteille de vin\nrouge, de Bourgogne" },
                ],
                "steps": [
                    { "kind": "step", "text": "1. Mariner une nuit." },
                    { "kind": "step", "text": "Cuire trois heures." },
                ],
            }),
            "{text}"
        );
        assert_eq!(read_note("no title here"), None);
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
                local_id: "b_1".into(),
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
        assert!(text.contains("## Historique"));
        assert!(
            !text.contains("## Ingrédients") && !text.contains("## Préparation"),
            "a recipe with nothing listed gets no heading over nothing:\n{text}"
        );
    }
}
