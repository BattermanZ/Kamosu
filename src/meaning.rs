//! Meaning Search: the half of searching that matches on meaning rather than on
//! words, so *that thing with aubergines* finds a recipe that never says
//! aubergine (ADR 0027).
//!
//! **Kamosu ships no model** (ADR 0029). The weights are EmbeddingGemma-300M,
//! whose licence forbids redistributing them, so nothing here is in the image:
//! an Operator accepts Google's terms, Kamosu downloads the model into
//! `/data/model/`, and an index is built from the recipes. An instance whose
//! Operator never does that searches by words and is complete.
//!
//! Three things live in three places, deliberately:
//!
//! - **The acceptance** is a row in the database, because it names the **Hand**
//!   that accepted and how that Hand arrived — a fact about people, which is
//!   what the database is the truth about (ADR 0003).
//! - **The weights** live under `/data/model/`, beside the database rather than
//!   inside it. They are somebody else's 220 MB, re-downloadable at any time,
//!   and they must stay out of a Backup for exactly that reason (ADR 0029).
//! - **The index** is rows in the database again, and is *derived*: every vector
//!   in it can be rebuilt from the recipes, which is what makes turning Meaning
//!   Search off a thing that discards nothing.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::core::OpError;

/// The model, named as a person would name it.
pub const MODEL_NAME: &str = "EmbeddingGemma-300M (4-bit)";
/// The Hugging Face repository the weights come from. The `onnx-community`
/// conversion rather than Google's own upload: it is the one FastEmbed knows
/// how to load, and it carries the same Gemma terms.
pub const MODEL_REPOSITORY: &str = "onnx-community/embeddinggemma-300m-ONNX";
/// Pinned to one commit, not to `main`. A model that changed under a built
/// index would leave vectors from two different embedding spaces in one table,
/// and nothing would look wrong until a search answered nonsense.
pub const MODEL_REVISION: &str = "5090578d9565bb06545b4552f76e6bc2c93e4a66";
/// The terms the accepting Operator is agreeing to, and the policy beside them.
pub const TERMS_URL: &str = "https://ai.google.dev/gemma/terms";
pub const PROHIBITED_USE_POLICY_URL: &str = "https://ai.google.dev/gemma/prohibited_use_policy";
/// Which issue of the terms this build asks about. Stored with each acceptance,
/// so what somebody agreed to can always be told apart from what Kamosu asks
/// today — which is what a later revision of Gemma's terms would need.
pub const TERMS_VERSION: &str = "2026-04-01";

/// EmbeddingGemma's output width, and the input it truncates at.
pub const DIMENSIONS: usize = 768;
const MAX_LENGTH: usize = 2048;

/// The artefacts FastEmbed needs to load `EmbeddingGemma300MQ4`. Downloaded
/// through the pinned revision below, because FastEmbed itself only ever asks
/// for `main`.
const ARTEFACTS: &[&str] = &[
    "onnx/model_q4.onnx",
    "onnx/model_q4.onnx_data",
    "config.json",
    "special_tokens_map.json",
    "tokenizer.json",
    "tokenizer_config.json",
];

/// The file recording what was downloaded and what each artefact hashed to.
/// Written after a load succeeds; checked before one is attempted.
const MANIFEST: &str = "manifest.json";

/// Where the weights live: one directory beside the database, inside the one
/// mount, so moving `/data` moves the model with it (ADR 0028).
pub fn model_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("model")
}

// ── The embedder ──────────────────────────────────────────────────────────────

/// A loaded EmbeddingGemma, and the two templates it was trained with.
///
/// The document and query sides are *not* symmetric: Gemma's retrieval training
/// wraps a document as `title: … | text: …` and a query as
/// `task: search result | query: …`, and embedding either side with the other's
/// template measurably costs recall. Both templates live here, next to the one
/// model that needs them, rather than at the call sites.
pub struct Embedder {
    // FastEmbed's `embed` takes `&mut self`, and this is shared behind an Arc.
    // Inference is serialised through a Mutex rather than leaking `&mut` into
    // every caller — Kamosu embeds a whole library rarely and a query often,
    // and a query is one short string.
    model: Mutex<TextEmbedding>,
}

impl Embedder {
    /// Load the model out of `/data/model/`. Downloads nothing: reaching this
    /// with no weights on disk is a failure, not an invitation to fetch 220 MB
    /// inside somebody's search.
    pub fn load(data_dir: &Path) -> Result<Embedder, String> {
        let model = TextEmbedding::try_new(
            TextInitOptions::new(EmbeddingModel::EmbeddingGemma300MQ4)
                .with_max_length(MAX_LENGTH)
                .with_cache_dir(model_dir(data_dir))
                .with_show_download_progress(false),
        )
        .map_err(|error| format!("cannot load the Meaning Search model: {error}"))?;
        Ok(Embedder {
            model: Mutex::new(model),
        })
    }

    /// Embed documents — one vector per input, in order.
    pub fn embed_documents(&self, documents: &[String]) -> Result<Vec<Vec<f32>>, String> {
        self.embed(documents)
    }

    /// Embed one query, wrapped in the template Gemma expects on the asking side.
    pub fn embed_query(&self, query: &str) -> Result<Vec<f32>, String> {
        let asked = format!("task: search result | query: {query}");
        Ok(self
            .embed(std::slice::from_ref(&asked))?
            .pop()
            .expect("one input, one vector"))
    }

    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        self.model
            .lock()
            .map_err(|_| "the Meaning Search model is in a broken state".to_string())?
            .embed(texts, None)
            .map_err(|error| format!("cannot read meaning: {error}"))
    }
}

/// The document side of Gemma's retrieval template: the recipe's title in front
/// of the block, which is what stops a mid-recipe block embedding as an
/// anonymous fragment (ADR 0027).
pub fn document_text(title: &str, section: Option<&str>, body: &str) -> String {
    let title = if title.trim().is_empty() {
        "none"
    } else {
        title
    };
    let text = match section {
        Some(section) if !section.trim().is_empty() => format!("Section: {section}\n\n{body}"),
        _ => body.to_string(),
    };
    format!("title: {title} | text: {text}")
}

// ── The slot the running instance holds it in ────────────────────────────────

/// Meaning Search as the running instance holds it: a directory it may load a
/// model from, and the model once loaded.
///
/// The slot may legitimately be empty — on an instance whose Operator never
/// turned it on, which is the default and the majority. Nothing anywhere else
/// branches on that: callers ask [`MeaningSearch::ready`], and every screen and
/// every Operation behaves the same either way.
pub struct MeaningSearch {
    data_dir: PathBuf,
    loaded: RwLock<Option<Arc<Embedder>>>,
    /// Held for the whole of an index build, so exactly one runs at a time.
    ///
    /// Two would be worse than slow: what is owed is read before anything is
    /// written, so two builds would find the same recipes owing, embed both
    /// copies, and leave the index holding every one of them twice. The tick
    /// that keeps the index level skips when this is held; an index build asked
    /// for by name waits, and then finds nothing left to do.
    building: Mutex<()>,
}

impl MeaningSearch {
    pub fn new(data_dir: PathBuf) -> MeaningSearch {
        MeaningSearch {
            data_dir,
            loaded: RwLock::new(None),
            building: Mutex::new(()),
        }
    }

    /// Claim the right to build the index, waiting for whoever holds it.
    pub fn building(&self) -> std::sync::MutexGuard<'_, ()> {
        self.building
            .lock()
            .unwrap_or_else(|held| held.into_inner())
    }

    /// Claim it only if nobody else is building. `None` means somebody is, and
    /// the caller should simply not — the work is already being done.
    pub fn building_now(&self) -> Option<std::sync::MutexGuard<'_, ()>> {
        match self.building.try_lock() {
            Ok(held) => Some(held),
            Err(std::sync::TryLockError::Poisoned(held)) => Some(held.into_inner()),
            Err(std::sync::TryLockError::WouldBlock) => None,
        }
    }

    /// The loaded model, if one is loaded.
    pub fn embedder(&self) -> Option<Arc<Embedder>> {
        self.loaded
            .read()
            .expect("the Meaning Search slot is never poisoned")
            .clone()
    }

    pub fn ready(&self) -> bool {
        self.embedder().is_some()
    }

    /// Load the model into the slot, or fail saying why. Idempotent: loading an
    /// already-loaded model replaces it with an equivalent one rather than
    /// erroring, so a rebuild after a re-download needs no special case.
    pub fn load(&self) -> Result<Arc<Embedder>, String> {
        let embedder = Arc::new(Embedder::load(&self.data_dir)?);
        *self
            .loaded
            .write()
            .expect("the Meaning Search slot is never poisoned") = Some(embedder.clone());
        Ok(embedder)
    }

    /// Empty the slot. The weights stay on disk — what this ends is Kamosu
    /// *using* them.
    pub fn unload(&self) {
        *self
            .loaded
            .write()
            .expect("the Meaning Search slot is never poisoned") = None;
    }

    /// Whether a complete set of weights is sitting on disk.
    ///
    /// Deliberately cheap — it asks whether every file the manifest names is
    /// still there, and does not read them. This is asked on every screen that
    /// shows Meaning Search's state, and hashing 220 MB to answer it would put
    /// a second of disk into opening the shelf. What the digests are for is
    /// resuming an interrupted download safely; a model that is present but
    /// damaged fails to load, loudly, which is the honest place to find out.
    pub fn weights_are_present(&self) -> bool {
        matches!(self.present(false), Ok(true))
    }

    /// Download every artefact FastEmbed needs, at the pinned revision, then
    /// point FastEmbed's ordinary `main` lookup at that immutable snapshot —
    /// FastEmbed exposes no revision option of its own.
    ///
    /// `report` is called with (this file, of how many, bytes so far, bytes
    /// expected) — which is what a Job turns into progress a person can watch.
    ///
    /// Per file rather than one overall percentage, because the artefacts are
    /// 662 bytes and 197 MB and everything between: weighting them equally
    /// would produce a bar that sits at 17% for four minutes and then finishes
    /// in a second. Saying which file, and how far into it, is true.
    pub fn download(
        &self,
        report: &(dyn Fn(usize, usize, u64, u64) + Send + Sync),
    ) -> Result<(), String> {
        use hf_hub::api::sync::ApiBuilder;
        use hf_hub::{Cache, Repo, RepoType};

        let cache_dir = model_dir(&self.data_dir);
        std::fs::create_dir_all(&cache_dir)
            .map_err(|error| format!("cannot make the model directory: {error}"))?;

        // Anything left from an interrupted or tampered-with attempt goes before
        // a byte is written, so a resumed download can never be half of one
        // revision and half of another.
        // Here — and only here — the digests are actually read: what is on disk
        // must be one whole revision before a resumed download builds on it.
        if !matches!(self.present(true), Ok(true)) {
            discard(&cache_dir)?;
        }

        let api = ApiBuilder::new()
            .with_cache_dir(cache_dir.clone())
            .with_progress(false)
            .build()
            .map_err(|error| format!("cannot reach Hugging Face: {error}"))?;
        let pinned = Repo::with_revision(
            MODEL_REPOSITORY.to_string(),
            RepoType::Model,
            MODEL_REVISION.to_string(),
        );
        let cached = Cache::new(cache_dir.clone()).repo(pinned.clone());
        let repository = api.repo(pinned);

        for (index, artefact) in ARTEFACTS.iter().enumerate() {
            if cached.get(artefact).is_some() {
                continue;
            }
            let of_this_file =
                |done: u64, total: u64| report(index + 1, ARTEFACTS.len(), done, total);
            repository
                .download_with_progress(artefact, Reporting::new(&of_this_file))
                .map_err(|error| format!("cannot download {artefact}: {error}"))?;
        }

        // FastEmbed asks for revision `main`. Its cache honours a local ref, so
        // `main` is pointed at the pinned commit — and only once every artefact
        // has actually landed, so a half-download is never mistaken for a model.
        Cache::new(cache_dir.clone())
            .repo(Repo::model(MODEL_REPOSITORY.to_string()))
            .create_ref(MODEL_REVISION)
            .map_err(|error| format!("cannot activate the downloaded model: {error}"))?;

        record(&cache_dir)
    }

    /// Whether every artefact the manifest names is on disk — and, with
    /// `digests`, whether each still hashes to what it did when it arrived. A
    /// missing manifest means nothing has been downloaded yet; a mismatch means
    /// what is on disk is not what came down the wire.
    fn present(&self, digests: bool) -> Result<bool, String> {
        let dir = model_dir(&self.data_dir);
        let Ok(bytes) = std::fs::read(dir.join(MANIFEST)) else {
            return Ok(false);
        };
        let manifest: Value = serde_json::from_slice(&bytes)
            .map_err(|error| format!("the model manifest is unreadable: {error}"))?;
        let Some(artefacts) = manifest["artefacts"].as_array().filter(|a| !a.is_empty()) else {
            return Ok(false);
        };
        for artefact in artefacts {
            let (Some(path), Some(digest)) =
                (artefact["path"].as_str(), artefact["sha256"].as_str())
            else {
                return Ok(false);
            };
            let path = dir.join(path);
            if !path.is_file() {
                return Ok(false);
            }
            if digests && sha256_of(&path)? != digest {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// Record what is on disk, so the next startup can tell an intact model from a
/// half-written one without downloading anything to compare against.
fn record(dir: &Path) -> Result<(), String> {
    let mut artefacts: Vec<Value> = Vec::new();
    for path in files_under(dir)? {
        if path.file_name().and_then(|name| name.to_str()) == Some(MANIFEST) {
            continue;
        }
        let relative = path
            .strip_prefix(dir)
            .map_err(|error| format!("cannot record the model: {error}"))?
            .to_string_lossy()
            .into_owned();
        artefacts.push(serde_json::json!({
            "path": relative,
            "sha256": sha256_of(&path)?,
        }));
    }
    if artefacts.is_empty() {
        return Err("the model download finished with no files".to_string());
    }
    artefacts.sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));

    let manifest = serde_json::json!({
        "repository": MODEL_REPOSITORY,
        "revision": MODEL_REVISION,
        "artefacts": artefacts,
    });
    let text = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| format!("cannot write the model manifest: {error}"))?;
    let temporary = dir.join("manifest.json.writing");
    std::fs::write(&temporary, text)
        .map_err(|error| format!("cannot write the model manifest: {error}"))?;
    std::fs::rename(&temporary, dir.join(MANIFEST))
        .map_err(|error| format!("cannot write the model manifest: {error}"))
}

/// Empty the model directory, manifest included.
fn discard(dir: &Path) -> Result<(), String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };
    for entry in entries {
        let path = entry
            .map_err(|error| format!("cannot clear the model directory: {error}"))?
            .path();
        let removed = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        removed.map_err(|error| format!("cannot clear the model directory: {error}"))?;
    }
    Ok(())
}

/// Every regular file under `dir`, at any depth. Small enough to hold in memory:
/// the model is six artefacts inside Hugging Face's cache layout.
fn files_under(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        let entries = std::fs::read_dir(&next)
            .map_err(|error| format!("cannot read {}: {error}", next.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| format!("cannot read the model: {error}"))?;
            let path = entry.path();
            // Hugging Face's cache links a snapshot's files at their blobs. The
            // link is what a load opens, so hashing the link is hashing the
            // artefact; following it twice would only record the same bytes
            // under two names.
            let kind = entry
                .file_type()
                .map_err(|error| format!("cannot read the model: {error}"))?;
            if kind.is_dir() {
                pending.push(path);
            } else {
                found.push(path);
            }
        }
    }
    Ok(found)
}

fn sha256_of(path: &Path) -> Result<String, String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Hugging Face's progress callback, forwarded to whatever the Job gave us.
struct Reporting<'a> {
    report: &'a (dyn Fn(u64, u64) + Send + Sync),
    done: u64,
    total: u64,
}

impl<'a> Reporting<'a> {
    fn new(report: &'a (dyn Fn(u64, u64) + Send + Sync)) -> Self {
        Reporting {
            report,
            done: 0,
            total: 0,
        }
    }
}

impl hf_hub::api::Progress for Reporting<'_> {
    fn init(&mut self, size: usize, _filename: &str) {
        self.total = size as u64;
        self.done = 0;
        (self.report)(0, self.total);
    }

    fn update(&mut self, size: usize) {
        self.done = self.done.saturating_add(size as u64);
        (self.report)(self.done, self.total);
    }

    fn finish(&mut self) {
        (self.report)(self.total, self.total);
    }
}

// ── How a recipe is cut ──────────────────────────────────────────────────────

/// One piece of a recipe as the index holds it, and the lines it was made from.
///
/// **Blocks are what ranking sees; lines are what a result quotes.** Cutting per
/// line for retrieval scatters the signal across many weak vectors — a bare
/// `salt` gives a meaning-matcher almost nothing — so retrieval runs on the
/// recipe's own blocks. Pinpointing the best line inside a matched block never
/// enters the ranking, so it survives any change to how finely a recipe is cut
/// (ADR 0027).
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// What this block is, in the same words a result uses to say what matched.
    pub kind: BlockKind,
    /// The Section this block belongs to, where the recipe has Sections.
    pub section: Option<String>,
    /// The block's body, as it is embedded (before the title template).
    pub body: String,
    /// The lines it was made from, each one quotable on its own.
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// The recipe entire — title, ingredients, method, Note. The block that
    /// answers a query about the dish rather than about one of its parts.
    Recipe,
    Ingredients,
    Steps,
    Note,
}

/// One quotable line, carrying exactly what a shelf entry's `matched` needs.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// `title`, `ingredient`, `step`, `section`, `note` — the same vocabulary a
    /// word match answers in, because a reader cannot tell the two apart and
    /// should not have to.
    pub where_: String,
    pub text: String,
    /// Counted over the Steps alone, Sections taking no number, exactly as the
    /// recipe page counts them.
    pub step_number: Option<i64>,
}

/// Cut one Version's content into the blocks the index holds.
///
/// The cut is **the recipe's own structure**: its ingredients, its method, its
/// Note, split per Section where the recipe has Sections, plus the recipe as a
/// whole. It is a dial, not a promise — what is promised is that a result can
/// say what matched, and that survives any cut (ADR 0027).
pub fn blocks_of(content: &Value) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();

    let cut = [
        (BlockKind::Ingredients, "ingredients", "ingredient"),
        (BlockKind::Steps, "steps", "step"),
    ];
    for (kind, key, where_) in cut {
        for (section, lines) in sectioned(content, key, where_) {
            blocks.push(Block {
                kind,
                section,
                body: lines
                    .iter()
                    .map(|line| line.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n"),
                lines,
            });
        }
    }

    if let Some(note) = content["note"]
        .as_str()
        .map(str::trim)
        .filter(|n| !n.is_empty())
    {
        blocks.push(Block {
            kind: BlockKind::Note,
            section: None,
            body: note.to_string(),
            lines: vec![Line {
                where_: "note".to_string(),
                text: note.to_string(),
                step_number: None,
            }],
        });
    }

    // The recipe as a whole, last: it is the block that answers *what is this
    // dish*, which no part of it answers on its own.
    //
    // It carries **no lines of its own**, deliberately. Every line it is made
    // of is already quotable through the block it came from, and copying them
    // here would double the index and every minute of a build for nothing.
    // Where this block is the one that matched, the line to quote is chosen
    // across the whole recipe's lines — which is what "inside the block that
    // matched" means when the block is the recipe.
    let whole: Vec<String> = blocks.iter().map(|block| block.body.clone()).collect();
    blocks.push(Block {
        kind: BlockKind::Recipe,
        section: None,
        body: whole.join("\n\n"),
        lines: Vec::new(),
    });

    blocks
        .into_iter()
        .filter(|block| !block.body.trim().is_empty())
        .collect()
}

/// One list of the recipe's lines — its ingredients or its steps — split at its
/// Section headers. A recipe with no Sections comes back as one unnamed group,
/// which is the ordinary case and needs no branch anywhere downstream.
fn sectioned(content: &Value, key: &str, where_: &str) -> Vec<(Option<String>, Vec<Line>)> {
    let Some(entries) = content[key].as_array() else {
        return Vec::new();
    };
    let mut groups: Vec<(Option<String>, Vec<Line>)> = vec![(None, Vec::new())];
    let mut number = 0_i64;
    for entry in entries {
        let text = entry["text"].as_str().unwrap_or_default().trim();
        if text.is_empty() {
            continue;
        }
        if entry["kind"].as_str() == Some("section") {
            // A Section header opens a new block *and* is a quotable line of the
            // one it opens: somebody searching for *for the sauce* is searching
            // for the sauce.
            groups.push((
                Some(text.to_string()),
                vec![Line {
                    where_: "section".to_string(),
                    text: text.to_string(),
                    step_number: None,
                }],
            ));
            continue;
        }
        number += 1;
        let line = Line {
            where_: where_.to_string(),
            text: text.to_string(),
            step_number: (where_ == "step").then_some(number),
        };
        groups
            .last_mut()
            .expect("there is always an open group")
            .1
            .push(line);
    }
    groups
        .into_iter()
        .filter(|(_, lines)| lines.iter().any(|line| line.where_ != "section"))
        .collect()
}

// ── Vectors ──────────────────────────────────────────────────────────────────

/// Cosine similarity between two vectors of the same width.
///
/// FastEmbed already returns normalised vectors, so this is a dot product in
/// practice — the division is kept because a vector that arrived un-normalised
/// would otherwise score arbitrarily high rather than merely wrong.
pub fn cosine(left: &[f32], right: &[f32]) -> f32 {
    if left.len() != right.len() || left.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0_f32;
    let mut left_norm = 0.0_f32;
    let mut right_norm = 0.0_f32;
    for (a, b) in left.iter().zip(right) {
        dot += a * b;
        left_norm += a * a;
        right_norm += b * b;
    }
    let scale = (left_norm.sqrt() * right_norm.sqrt()).max(f32::MIN_POSITIVE);
    dot / scale
}

/// A vector as the database holds it: little-endian `f32`s in one BLOB.
///
/// Not a JSON array of numbers: 768 floats are 3 KB packed and about 10 KB as
/// text, and the index is read whole on every meaning search.
pub fn pack(vector: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vector.len() * 4);
    for value in vector {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

pub fn unpack(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

// ── The index ────────────────────────────────────────────────────────────────

/// How close a block has to be before Kamosu will call it a match at all.
///
/// **Measured against the real 86-recipe library, not assumed** (2026-08-30).
/// Over eleven queries the bands came out cleanly separated:
///
/// | what the query was                           | where its results sat |
/// |----------------------------------------------|-----------------------|
/// | plainly the right recipe                     | 0.44 – 0.65 |
/// | arguably related — anchovies for *shellfish* | 0.36 – 0.40 |
/// | a recipe that merely shares a word           | 0.32 – 0.40 |
/// | nothing to do with cooking at all            | below 0.24 |
///
/// So 0.40, which keeps every plainly-right result in the sample and admits
/// none of the noise. Dropping to 0.38 would have bought two more chocolate
/// recipes and also handed *something with shellfish* a bowl of cherry
/// tomatoes — and ADR 0027 refused, four separate times, to let a weak answer
/// pass as a good one.
///
/// Note the scale. EmbeddingGemma's similarities on short recipe text sit far
/// below the 0.7-and-up a reader might expect: this is the model's own
/// geometry, not a confidence percentage, and reading it as one is exactly how
/// a threshold gets set to a number that matches nothing.
pub const CLOSE_ENOUGH: f32 = 0.40;

/// The similarity at which a meaning match scores as high as it ever will.
///
/// From the same measurement: 0.55 and up was reached only where the query was
/// very nearly the recipe's own words — *a rich beef stew cooked in wine*
/// finding Beef Bourguignon at 0.58, *deep fried chicken* finding Korean Fried
/// Chicken at 0.58. Above that the model has nothing more to say, so neither
/// has the ranking.
pub const CONFIDENT: f32 = 0.55;

/// What one word-match rung is worth on the shelf's single ranking.
///
/// The rungs are the search's own, in its order — exact title, title, tag,
/// ingredient, step, Note, Attempt — and the numbers exist so that a meaning
/// match can be compared against them rather than merely queued behind them.
///
/// It lives here, beside the band below, because the two are one rule with one
/// invariant: **only a title outranks meaning.** Split across two files, that
/// invariant would be two numbers nobody was reading together.
///
/// - **An exact title scores 1.0**, above anything meaning can reach, so *most
///   searching is navigation* stays true whatever the model thinks.
/// - **A title that merely contains the words scores 0.95**, also above
///   meaning's ceiling. Everything below that — a tag, an ingredient, a step, a
///   Note, an Attempt — a confident meaning match may outrank.
pub fn word_score(rank: u8) -> f32 {
    match rank {
        0 => 1.00,
        1 => 0.95,
        2 => 0.90,
        3 => 0.85,
        4 => 0.80,
        5 => 0.75,
        _ => 0.70,
    }
}

/// The band a meaning match scores in, on that same scale. Its ceiling sits
/// just under [`word_score`]'s title rungs and just above every other one —
/// which is what *combined in one ranking* means. Concatenating meaning after
/// words would make the blend a label rather than a behaviour (ADR 0027).
const MEANING_FLOOR: f32 = 0.60;
const MEANING_CEILING: f32 = 0.94;

/// Where one meaning match scores, between [`MEANING_FLOOR`] and
/// [`MEANING_CEILING`]. `None` when the block was not close enough to be called
/// a match at all.
pub fn score(similarity: f32) -> Option<f32> {
    if similarity < CLOSE_ENOUGH {
        return None;
    }
    let reach = ((similarity - CLOSE_ENOUGH) / (CONFIDENT - CLOSE_ENOUGH)).clamp(0.0, 1.0);
    Some(MEANING_FLOOR + reach * (MEANING_CEILING - MEANING_FLOOR))
}

/// What Meaning Search is doing on this instance, as the one row holds it.
#[derive(Debug, Clone)]
pub struct State {
    /// `unasked`, `declined`, `accepted` or `on`.
    pub state: String,
    pub accepted_by: Option<String>,
    pub accepted_via_access_key: Option<bool>,
    pub accepted_at: Option<String>,
    pub declined_at: Option<String>,
    /// Which issue of the terms was actually agreed to. Not the constant this
    /// build carries: an acceptance is a record of what somebody said yes to,
    /// and saying it back as today's version would be putting words in their
    /// mouth.
    pub terms_version: Option<String>,
    pub indexed_at: Option<String>,
}

impl State {
    /// Answering searches: accepted, downloaded and indexed.
    pub fn is_on(&self) -> bool {
        self.state == "on"
    }

    /// The model's terms have been agreed to, whether or not the weights have
    /// arrived yet or the index has been built.
    pub fn is_accepted(&self) -> bool {
        self.state == "accepted" || self.state == "on"
    }

    /// Whether the offer to turn Meaning Search on should still be made.
    /// Never again once an Operator has declined, and pointless once accepted.
    pub fn still_worth_offering(&self) -> bool {
        self.state == "unasked"
    }
}

/// The stamp every vector carries: which weights, at which revision, produced
/// it. Vectors stamped with anything else belong to a different embedding space.
pub fn embedding_space() -> String {
    format!("{MODEL_REPOSITORY}@{MODEL_REVISION}/q4/{DIMENSIONS}")
}

/// One index row's id. Random rather than derived: the same block re-embedded
/// after an edit is a new row, and reusing an id would make the two look like
/// one thing that changed.
fn row_id() -> String {
    format!("mv_{}", hex::encode(crate::core::random_bytes(8)))
}

fn read(e: rusqlite::Error) -> OpError {
    OpError::internal(format!("cannot read Meaning Search: {e}"))
}

fn write(e: rusqlite::Error) -> OpError {
    OpError::internal(format!("cannot record Meaning Search: {e}"))
}

pub fn state(conn: &Connection) -> Result<State, OpError> {
    conn.query_row(
        "SELECT state, accepted_by, accepted_via_access_key, accepted_at, declined_at, \
                terms_version, indexed_at FROM meaning_search WHERE id = 1",
        [],
        |row| {
            Ok(State {
                state: row.get(0)?,
                accepted_by: row.get(1)?,
                accepted_via_access_key: row.get::<_, Option<i64>>(2)?.map(|v| v != 0),
                accepted_at: row.get(3)?,
                declined_at: row.get(4)?,
                terms_version: row.get(5)?,
                indexed_at: row.get(6)?,
            })
        },
    )
    .map_err(read)
}

/// Accept the model's terms, keeping the Hand that did it and how it arrived.
///
/// Accepting twice is not an error and does not rewrite the first acceptance:
/// the record is of who agreed, and that already happened.
pub fn accept(conn: &Connection, person_id: &str, via_access_key: bool) -> Result<(), OpError> {
    if state(conn)?.is_accepted() {
        return Ok(());
    }
    conn.execute(
        "UPDATE meaning_search SET state = 'accepted', accepted_by = ?1, \
                accepted_via_access_key = ?2, \
                accepted_at = strftime('%Y-%m-%dT%H:%M:%fZ','now'), \
                terms_version = ?3, terms_url = ?4, \
                model_repository = ?5, model_revision = ?6, \
                declined_by = NULL, declined_at = NULL \
          WHERE id = 1",
        params![
            person_id,
            i64::from(via_access_key),
            TERMS_VERSION,
            TERMS_URL,
            MODEL_REPOSITORY,
            MODEL_REVISION,
        ],
    )
    .map_err(write)?;
    Ok(())
}

/// Decline. The offer is never made again — an answered question asked twice is
/// a nag, and this one was asked where it was earned.
///
/// Only ever an answer to the offer, so only ever from `unasked`. Declining
/// terms already accepted is not a decline, it is turning Meaning Search off
/// while claiming otherwise, and `turn_off_meaning_search` is the Operation
/// that actually does that.
pub fn decline(conn: &Connection, person_id: &str) -> Result<(), OpError> {
    if !state(conn)?.still_worth_offering() {
        return Err(OpError::bad_request(
            "the model's terms have already been answered on this Kamosu",
        ));
    }
    conn.execute(
        "UPDATE meaning_search SET state = 'declined', declined_by = ?1, \
                declined_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = 1",
        params![person_id],
    )
    .map_err(write)?;
    Ok(())
}

/// Mark the index built and Meaning Search answering.
pub fn turn_on(conn: &Connection) -> Result<(), OpError> {
    conn.execute(
        "UPDATE meaning_search SET state = 'on', indexed_with = ?1, \
                indexed_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = 1",
        params![embedding_space()],
    )
    .map_err(write)?;
    Ok(())
}

/// Turn Meaning Search off and throw the index away.
///
/// Everything discarded here is derived from the recipes, so this loses nothing
/// that cannot be rebuilt — which is the whole of why it is safe to offer as a
/// plain button. The acceptance stays: somebody did agree to those terms, and
/// that is history rather than a setting. The weights stay on disk too, so
/// turning it back on is an index rebuild rather than another 220 MB.
///
/// **Only `on` falls back to `accepted`.** Turning off an instance that never
/// accepted anything must not invent an acceptance with nobody's Hand on it —
/// and, because the offer is live only while nobody has answered, doing so
/// would silently spend the offer as well.
pub fn turn_off(conn: &Connection) -> Result<(), OpError> {
    discard_index(conn)?;
    conn.execute(
        "UPDATE meaning_search SET state = 'accepted', indexed_with = NULL, \
                indexed_at = NULL WHERE id = 1 AND state = 'on'",
        [],
    )
    .map_err(write)?;
    Ok(())
}

/// Fall back to word search without touching the acceptance or the index.
///
/// What this records is that Kamosu is no longer *using* the model — because it
/// is gone from disk, or would not load. An instance can lose its model and be
/// fine: deleting `/data/model/` returns it to word search and nothing else
/// notices (ADR 0029). The index is left exactly where it is, since the same
/// weights coming back make it good again and a different revision is pruned by
/// the next build anyway.
pub fn fell_back(conn: &Connection) -> Result<(), OpError> {
    conn.execute(
        "UPDATE meaning_search SET state = 'accepted' WHERE id = 1 AND state = 'on'",
        [],
    )
    .map_err(write)?;
    Ok(())
}

pub fn discard_index(conn: &Connection) -> Result<(), OpError> {
    conn.execute("DELETE FROM meaning_vectors", [])
        .map_err(write)?;
    Ok(())
}

/// One thing the index does not yet hold, or holds a stale copy of.
struct Owed {
    lineage_id: String,
    branch_id: Option<String>,
    version_id: Option<String>,
    attempt_id: Option<String>,
    person_id: Option<String>,
    title: String,
    content: Value,
    note: Option<String>,
}

/// Everything the index owes the library right now: a Branch whose head Version
/// has no rows (or rows from an older Version, or from another embedding
/// space), and an Attempt whose note has none — or has rows made from a
/// sentence its writer has since corrected.
///
/// A Branch is caught by its head Version moving, because an edit always makes
/// one. An Attempt is not: correcting a cooking note leaves the Attempt's id
/// alone, so what is compared there is the text itself.
fn owed(conn: &Connection) -> Result<Vec<Owed>, OpError> {
    let space = embedding_space();
    let mut owed = Vec::new();

    {
        let mut statement = conn
            .prepare(
                "SELECT branches.id, branches.lineage_id, branches.head_version_id, versions.content \
                   FROM branches JOIN versions ON versions.id = branches.head_version_id \
                  WHERE NOT EXISTS ( \
                        SELECT 1 FROM meaning_vectors \
                         WHERE meaning_vectors.branch_id = branches.id \
                           AND meaning_vectors.version_id = branches.head_version_id \
                           AND meaning_vectors.embedding_space = ?1)",
            )
            .map_err(read)?;
        let rows: Vec<(String, String, String, String)> = statement
            .query_map(params![space], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(read)?
            .collect::<Result<_, _>>()
            .map_err(read)?;
        for (branch_id, lineage_id, version_id, content) in rows {
            let content: Value = serde_json::from_str(&content).unwrap_or(Value::Null);
            let title = content["title"].as_str().unwrap_or_default().to_string();
            owed.push(Owed {
                lineage_id,
                branch_id: Some(branch_id),
                version_id: Some(version_id),
                attempt_id: None,
                person_id: None,
                title,
                content,
                note: None,
            });
        }
    }

    {
        let mut statement = conn
            .prepare(
                "SELECT attempts.id, attempts.lineage_id, attempts.person_id, attempts.note \
                   FROM attempts \
                  WHERE attempts.note IS NOT NULL AND trim(attempts.note) <> '' \
                    AND NOT EXISTS ( \
                        SELECT 1 FROM meaning_vectors \
                         WHERE meaning_vectors.attempt_id = attempts.id \
                           AND meaning_vectors.embedding_space = ?1 \
                           AND meaning_vectors.embedded_text = attempts.note)",
            )
            .map_err(read)?;
        let rows: Vec<(String, String, String, String)> = statement
            .query_map(params![space], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(read)?
            .collect::<Result<_, _>>()
            .map_err(read)?;
        for (attempt_id, lineage_id, person_id, note) in rows {
            owed.push(Owed {
                lineage_id,
                branch_id: None,
                version_id: None,
                attempt_id: Some(attempt_id),
                person_id: Some(person_id),
                title: String::new(),
                content: Value::Null,
                note: Some(note),
            });
        }
    }

    Ok(owed)
}

/// Drop every row the library no longer justifies: a Branch or Attempt that is
/// gone, a Version that has been superseded, a cooking note that has since been
/// corrected, or a vector stamped with a different embedding space than the one
/// this build produces.
fn prune(conn: &Connection) -> Result<(), OpError> {
    conn.execute(
        "DELETE FROM meaning_vectors WHERE embedding_space <> ?1 \
            OR (branch_id IS NOT NULL AND NOT EXISTS ( \
                    SELECT 1 FROM branches \
                     WHERE branches.id = meaning_vectors.branch_id \
                       AND branches.head_version_id = meaning_vectors.version_id)) \
            OR (attempt_id IS NOT NULL AND NOT EXISTS ( \
                    SELECT 1 FROM attempts WHERE attempts.id = meaning_vectors.attempt_id \
                       AND attempts.note IS NOT NULL AND trim(attempts.note) <> '' \
                       AND attempts.note = meaning_vectors.embedded_text))",
        params![embedding_space()],
    )
    .map_err(write)?;
    Ok(())
}

/// How many recipes and Attempts the index is behind by. Cheap: one query when
/// nothing has changed, which is what it usually is.
pub fn behind(conn: &Connection) -> Result<usize, OpError> {
    Ok(owed(conn)?.len())
}

/// Bring the index up to date, embedding only what has actually changed.
///
/// Rebuilding the whole library is what happens the first time and after a
/// model change, because then every row is owed. Afterwards this is the cheap
/// path: the query above finds nothing and no embedding runs at all.
///
/// `progress` is called with (done, total) so a Job can be watched.
pub fn build(
    db: &crate::db::Db,
    embedder: &Embedder,
    progress: &(dyn Fn(usize, usize) + Send + Sync),
) -> Result<usize, OpError> {
    // The database lock is taken to read what is owed, released while the model
    // runs, and taken again to write one recipe's rows. Holding it across
    // inference would freeze every other Operation for as long as an index
    // build takes, which is minutes on a first run.
    let owed = db.with_conn(|conn| {
        prune(conn)?;
        owed(conn)
    })?;
    let total = owed.len();
    progress(0, total);
    if total == 0 {
        return Ok(0);
    }

    let space = embedding_space();
    for (done, one) in owed.into_iter().enumerate() {
        // One recipe (or one Attempt) at a time: a whole library in one batch
        // would hold every vector in memory before a single row landed, and a
        // Job that fails at 90% would have nothing to show for it. Per recipe,
        // an interrupted build simply resumes where it stopped, because what is
        // written is what the next `owed` no longer asks for.
        let blocks = match &one.note {
            Some(note) => vec![Block {
                kind: BlockKind::Note,
                section: None,
                body: note.clone(),
                lines: vec![Line {
                    where_: "attempt".to_string(),
                    text: note.clone(),
                    step_number: None,
                }],
            }],
            None => blocks_of(&one.content),
        };
        if blocks.is_empty() {
            // A recipe of nothing but a title has nothing to embed — which is a
            // perfectly ordinary recipe (#6), not a failure. Its title is still
            // found by words, which is how a title is looked for anyway.
            progress(done + 1, total);
            continue;
        }

        // Blocks and lines are embedded in **separate** batches, and that is a
        // measured decision rather than tidiness. The tokenizer pads a batch to
        // its longest member, so putting a whole recipe in with its own
        // one-line ingredients makes the model chew through a full recipe's
        // worth of padding for every `2 aubergines` in it. Splitting them keeps
        // short inputs batched with short ones.
        let block_inputs: Vec<String> = blocks
            .iter()
            .map(|block| document_text(&one.title, block.section.as_deref(), &block.body))
            .collect();
        // A line is embedded **without** the recipe's title, and a block with
        // it. That asymmetry is the difference between the two jobs. A block is
        // being retrieved, and the title is what stops it embedding as an
        // anonymous fragment; a line is only ever compared against the other
        // lines of a recipe already chosen, where the title is identical on
        // every one of them and therefore pure noise. Left in, it drowns short
        // lines: a search for *shellfish* inside Moules Marinières quoted a
        // step about whisking butter, because every line of that recipe scored
        // the same and one of them had to win.
        let mut line_index: Vec<(usize, usize)> = Vec::new();
        let mut line_inputs: Vec<String> = Vec::new();
        for (b, block) in blocks.iter().enumerate() {
            for (l, line) in block.lines.iter().enumerate() {
                line_index.push((b, l));
                line_inputs.push(document_text("", None, &line.text));
            }
        }

        let block_vectors = embedder
            .embed_documents(&block_inputs)
            .map_err(OpError::internal)?;
        let line_vectors = embedder
            .embed_documents(&line_inputs)
            .map_err(OpError::internal)?;

        db.with_conn(|conn| {
            let mut block_ids: Vec<String> = Vec::new();
            for (b, block) in blocks.iter().enumerate() {
                let id = row_id();
                block_ids.push(id.clone());
                insert(
                    conn,
                    &id,
                    "block",
                    None,
                    &one,
                    &space,
                    block.section.as_deref(),
                    &block.body,
                    None,
                    &block_vectors[b],
                )?;
            }
            for (offset, (b, l)) in line_index.iter().enumerate() {
                let line = &blocks[*b].lines[*l];
                insert(
                    conn,
                    &row_id(),
                    "line",
                    Some(&block_ids[*b]),
                    &one,
                    &space,
                    blocks[*b].section.as_deref(),
                    &line.text,
                    Some(line),
                    &line_vectors[offset],
                )?;
            }
            Ok(())
        })?;

        progress(done + 1, total);
    }

    Ok(total)
}

#[allow(clippy::too_many_arguments)]
fn insert(
    conn: &Connection,
    id: &str,
    role: &str,
    block_id: Option<&str>,
    one: &Owed,
    space: &str,
    section: Option<&str>,
    embedded: &str,
    line: Option<&Line>,
    vector: &[f32],
) -> Result<(), OpError> {
    conn.execute(
        "INSERT INTO meaning_vectors \
            (id, role, block_id, lineage_id, branch_id, version_id, attempt_id, person_id, \
             embedding_space, section, embedded_text, where_, step_number, vector) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            id,
            role,
            block_id,
            one.lineage_id,
            one.branch_id,
            one.version_id,
            one.attempt_id,
            one.person_id,
            space,
            section,
            embedded,
            line.map(|line| line.where_.as_str()),
            line.and_then(|line| line.step_number),
            pack(vector),
        ],
    )
    .map_err(write)?;
    Ok(())
}

// ── Asking it something ──────────────────────────────────────────────────────

/// One Lineage's best meaning match: how close it came, and the line to quote.
#[derive(Debug, Clone)]
pub struct Hit {
    pub similarity: f32,
    /// `{ "where": …, "line": …, "step_number": … }` — the same shape a word
    /// match answers in, because a reader cannot tell the two apart by looking.
    pub matched: Value,
}

/// The best block in every Lineage, against one query.
///
/// **`visible` is the shelf's own boundary, already resolved**, and it is the
/// whole of what keeps this honest. A Lineage can be held as one Branch in your
/// Kitchen and as another in somebody else's (ADR 0004, ADR 0014), so scanning
/// every block of every Branch would let a recipe you cannot see set your card's
/// rank and hand it a line to quote. The caller passes the Branches it already
/// worked out you may read — filters and all — rather than this asking the
/// permission question a second time in different words.
///
/// No Operation in today's Catalogue can put two Branches of one Lineage in two
/// Kitchens, so there is no behaviour test that catches this going wrong. Bundle
/// import (#67) and Share Links (#65) are the two that will, and the guard is
/// here first on purpose: a boundary added after the door opens is a boundary
/// that was missing for a release.
///
/// **Own-only Attempts are a fact of the rows**, not a check made here: an
/// Attempt's vectors carry the Person who wrote them, so a Kitchen-mate's
/// *burnt it again, honestly* is not in the scan at all (ADR 0027).
///
/// The whole index is scanned. That is not a shortcut around a real vector
/// index — it is the right shape for a library of this size, where every vector
/// fits in a couple of megabytes and the scan costs less than the one inference
/// that produced the query.
pub fn nearest(
    conn: &Connection,
    asked: &[f32],
    person_id: &str,
    visible: &HashSet<&str>,
) -> Result<HashMap<String, Hit>, OpError> {
    // Rung one: which block, in each Lineage, comes closest. Only 'block' rows
    // are ever compared here — a line's vector never enters a ranking.
    let mut best: HashMap<String, (f32, String, Option<String>, Option<String>)> = HashMap::new();
    {
        let mut statement = conn
            .prepare(
                "SELECT id, lineage_id, branch_id, attempt_id, vector FROM meaning_vectors \
                  WHERE role = 'block' AND (person_id IS NULL OR person_id = ?1)",
            )
            .map_err(read)?;
        let rows = statement
            .query_map(params![person_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                ))
            })
            .map_err(read)?;
        for row in rows {
            let (id, lineage_id, branch_id, attempt_id, vector) = row.map_err(read)?;
            // A recipe's block counts only where its Branch is one the caller
            // may read. An Attempt's has no Branch and needs none: the SQL
            // above already kept it to the Person who wrote it.
            if branch_id
                .as_deref()
                .is_some_and(|branch_id| !visible.contains(branch_id))
            {
                continue;
            }
            let similarity = cosine(asked, &unpack(&vector));
            match best.get(&lineage_id) {
                Some((held, _, _, _)) if *held >= similarity => {}
                _ => {
                    best.insert(lineage_id, (similarity, id, branch_id, attempt_id));
                }
            }
        }
    }

    // Rung two, and it is display only: which line inside that block to quote.
    // These comparisons never touch the ranking above, which is why the promise
    // that a result says what matched survives any change to how a recipe is cut.
    let mut hits: HashMap<String, Hit> = HashMap::new();
    let mut of_block = conn
        .prepare(
            "SELECT where_, embedded_text, step_number, vector FROM meaning_vectors \
              WHERE role = 'line' AND block_id = ?1",
        )
        .map_err(read)?;
    // The whole-recipe block keeps no lines of its own, so *inside the block
    // that matched* means across the recipe when the recipe is what matched.
    let mut of_recipe = conn
        .prepare(
            "SELECT where_, embedded_text, step_number, vector FROM meaning_vectors \
              WHERE role = 'line' AND (branch_id = ?1 OR attempt_id = ?2)",
        )
        .map_err(read)?;
    for (lineage_id, (similarity, block_id, branch_id, attempt_id)) in best {
        let has_own: i64 = conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM meaning_vectors \
                                 WHERE role = 'line' AND block_id = ?1)",
                params![block_id],
                |row| row.get(0),
            )
            .map_err(read)?;
        let statement = if has_own != 0 {
            &mut of_block
        } else {
            &mut of_recipe
        };
        let bound: Vec<&dyn rusqlite::ToSql> = if has_own != 0 {
            vec![&block_id]
        } else {
            vec![&branch_id, &attempt_id]
        };
        let rows = statement
            .query_map(bound.as_slice(), |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                ))
            })
            .map_err(read)?;
        let mut quoted: Option<(f32, Value)> = None;
        for row in rows {
            let (where_, text, step_number, vector) = row.map_err(read)?;
            let (Some(where_), Some(text)) = (where_, text) else {
                continue;
            };
            let closeness = cosine(asked, &unpack(&vector));
            if quoted.as_ref().is_none_or(|(held, _)| closeness > *held) {
                quoted = Some((
                    closeness,
                    json!({ "where": where_, "line": text, "step_number": step_number }),
                ));
            }
        }
        if let Some((_, matched)) = quoted {
            hits.insert(
                lineage_id,
                Hit {
                    similarity,
                    matched,
                },
            );
        }
    }

    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn recipe() -> Value {
        json!({
            "title": "Ratatouille",
            "ingredients": [
                { "kind": "ingredient", "text": "2 aubergines" },
                { "kind": "section", "text": "For the sauce" },
                { "kind": "ingredient", "text": "1 tin of tomatoes" },
            ],
            "steps": [
                { "kind": "step", "text": "Slice everything." },
                { "kind": "section", "text": "Then" },
                { "kind": "step", "text": "Bake for an hour." },
            ],
            "note": "Better the next day.",
        })
    }

    #[test]
    fn a_recipe_is_cut_at_its_own_sections() {
        let blocks = blocks_of(&recipe());
        let sections: Vec<Option<&str>> = blocks
            .iter()
            .map(|block| block.section.as_deref())
            .collect();
        assert_eq!(
            sections,
            vec![
                None,                  // the ingredients before any Section
                Some("For the sauce"), // and the ones after it
                None,                  // the first steps
                Some("Then"),
                None, // the Note
                None, // the recipe entire
            ]
        );
    }

    #[test]
    fn the_whole_recipe_is_a_block_of_its_own() {
        let blocks = blocks_of(&recipe());
        let whole = blocks.last().expect("a block");
        assert_eq!(whole.kind, BlockKind::Recipe);
        assert!(whole.body.contains("aubergines"));
        assert!(whole.body.contains("Bake for an hour."));
        assert!(whole.body.contains("Better the next day."));
    }

    #[test]
    fn steps_are_numbered_over_the_steps_alone() {
        let blocks = blocks_of(&recipe());
        let numbered: Vec<(String, Option<i64>)> = blocks
            .iter()
            .filter(|block| block.kind == BlockKind::Steps)
            .flat_map(|block| block.lines.iter())
            .map(|line| (line.text.clone(), line.step_number))
            .collect();
        assert_eq!(
            numbered,
            vec![
                ("Slice everything.".to_string(), Some(1)),
                ("Then".to_string(), None),
                ("Bake for an hour.".to_string(), Some(2)),
            ]
        );
    }

    #[test]
    fn a_recipe_with_only_a_title_still_makes_no_empty_blocks() {
        let blocks = blocks_of(&json!({ "title": "Toast" }));
        assert!(blocks.is_empty(), "nothing to embed is not one empty block");
    }

    #[test]
    fn the_document_template_puts_the_title_in_front() {
        assert_eq!(
            document_text("Ratatouille", Some("For the sauce"), "1 tin of tomatoes"),
            "title: Ratatouille | text: Section: For the sauce\n\n1 tin of tomatoes"
        );
        assert_eq!(
            document_text("Ratatouille", None, "2 aubergines"),
            "title: Ratatouille | text: 2 aubergines"
        );
    }

    #[test]
    fn a_vector_survives_the_round_trip_into_a_blob() {
        let vector = vec![0.5_f32, -0.25, 0.125];
        assert_eq!(unpack(&pack(&vector)), vector);
    }

    #[test]
    fn cosine_is_one_for_a_vector_against_itself() {
        let vector = vec![0.3_f32, 0.4, 0.5];
        assert!((cosine(&vector, &vector) - 1.0).abs() < 1e-6);
        assert_eq!(cosine(&vector, &[1.0, 2.0]), 0.0);
    }
}
