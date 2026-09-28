//! What names a Version: the fingerprint of the recipe it holds.
//!
//! A Version's id is a hash of its own content and nothing else (ADR 0004,
//! ADR 0021), which is what lets two people who reached the identical state
//! hold the identical id without ever having communicated. Everything in this
//! module exists to keep that sentence true across the years a recipe outlives
//! any one release of Kamosu.
//!
//! **A field holding nothing is no part of the fingerprint** (ADR 0038, which
//! carries the reasoning and what adopting it cost). The practical shape of it
//! is that a field added to a recipe later starts out empty on every recipe
//! already written, drops straight back out here, and moves no existing id.

use rusqlite::Connection;
use rusqlite::functions::FunctionFlags;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// The canonical serialisation a Version's fingerprint is taken over: object
/// keys sorted recursively, independent of `serde_json`'s own default Map
/// ordering, so a field added later stays deterministic.
pub fn canonical_json(value: &Value) -> String {
    fn canonicalise(value: &Value) -> Value {
        match value {
            Value::Object(map) => {
                let sorted: std::collections::BTreeMap<String, Value> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), canonicalise(v)))
                    .collect();
                json!(sorted)
            }
            Value::Array(items) => Value::Array(items.iter().map(canonicalise).collect()),
            other => other.clone(),
        }
    }
    canonicalise(value).to_string()
}

/// The same content with every field holding nothing removed — `null`, the
/// empty list, the empty object — at every depth, so a field added inside a
/// Step or an Ingredient Line is as free as one added beside the title.
///
/// Only **object keys** go. An array is positional, so an entry that happens
/// to be empty is a line in a place, and removing it would silently renumber
/// everything after it.
///
/// The empty string stays. `note: ""` is somebody who opened the note box and
/// left it blank, which the recipe parser keeps distinct from never having
/// opened it; the rule here is about a slot that was never filled in, not
/// about text that happens to be short.
fn without_empty_fields(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(_, v)| !is_empty(v))
                .map(|(k, v)| (k.clone(), without_empty_fields(v)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(without_empty_fields).collect()),
        other => other.clone(),
    }
}

fn is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Array(items) => items.is_empty(),
        Value::Object(map) => map.is_empty(),
        _ => false,
    }
}

/// A Version's id: the fingerprint of its content alone (ADR 0004, ADR 0021,
/// ADR 0038).
pub fn fingerprint_content(content: &Value) -> String {
    format!(
        "v_{}",
        hex::encode(Sha256::digest(
            canonical_json(&without_empty_fields(content)).as_bytes()
        ))
    )
}

/// A Version ready to store: its id and the text that goes in the row, from
/// the one content.
///
/// They are always wanted together and must always describe each other —
/// everything that asks whether two people hold the same recipe asks it by
/// comparing ids, and that question is only answerable by comparison while
/// every row's id really is the fingerprint of the text beside it. Handing the
/// two out as a pair is what stops a caller computing one from the content and
/// the other from something else.
pub fn stored_version(content: &Value) -> (String, String) {
    (fingerprint_content(content), canonical_json(content))
}

/// Teach one SQLite connection to compute a Version's fingerprint, as
/// `version_fingerprint(content)` over the stored JSON text.
///
/// It is here for two callers and no third. The migration that put #89 right
/// needed to re-fingerprint the library in SQL, and **a test needs to assert
/// the invariant on a real database** — `SELECT COUNT(*) FROM versions WHERE
/// id <> version_fingerprint(content)` is #89's own reproduction recipe,
/// answered in one line, and its answer must be zero forever.
///
/// It is emphatically not an invitation to re-fingerprint again. Doing that
/// rewrites append-only history and every reference to it (ADR 0004), and
/// ADR 0038 exists so that no future field addition ever needs to.
///
/// Unreadable content answers NULL rather than failing: a migration comparing
/// against it must treat that as "leave this row alone", never as a match.
pub fn register(conn: &Connection) -> rusqlite::Result<()> {
    conn.create_scalar_function(
        "version_fingerprint",
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            let text = ctx.get::<String>(0)?;
            Ok(serde_json::from_str::<Value>(&text)
                .ok()
                .map(|content| fingerprint_content(&content)))
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rule itself: a field holding nothing is invisible.
    #[test]
    fn an_empty_field_moves_no_fingerprint() {
        let bare = json!({ "title": "Soup", "steps": [{ "text": "Boil" }] });
        let padded = json!({
            "title": "Soup",
            "yield": Value::Null,
            "note": Value::Null,
            "ingredients": [],
            "steps": [{ "text": "Boil", "photo": Value::Null }],
        });
        assert_eq!(fingerprint_content(&bare), fingerprint_content(&padded));
    }

    /// The thing #89 was actually about: a recipe written before `nutrition`
    /// existed and the same recipe written after it fingerprint identically.
    #[test]
    fn nutrition_added_later_moves_no_fingerprint() {
        let before = json!({ "title": "Coq au Vin", "note": "Best hot." });
        let after = json!({ "title": "Coq au Vin", "note": "Best hot.", "nutrition": Value::Null });
        assert_eq!(fingerprint_content(&before), fingerprint_content(&after));
    }

    /// A field holding something is very much part of it.
    #[test]
    fn a_filled_field_moves_the_fingerprint() {
        let empty = json!({ "title": "Soup", "nutrition": Value::Null });
        let filled = json!({ "title": "Soup", "nutrition": { "calories": 300 } });
        assert_ne!(fingerprint_content(&empty), fingerprint_content(&filled));
    }

    /// An empty *string* is somebody who wrote nothing, not a slot never
    /// filled — the two are different recipes and stay different ids.
    #[test]
    fn an_empty_string_is_not_an_empty_field() {
        let absent = json!({ "title": "Soup" });
        let blank = json!({ "title": "Soup", "note": "" });
        assert_ne!(fingerprint_content(&absent), fingerprint_content(&blank));
    }

    /// Stripping never renumbers a list: an empty entry holds its place.
    #[test]
    fn an_empty_list_entry_keeps_its_place() {
        let content = json!({ "steps": [{ "text": "One" }, {}, { "text": "Three" }] });
        let stripped = without_empty_fields(&content);
        assert_eq!(stripped["steps"].as_array().map(Vec::len), Some(3));
    }

    /// Key order in the input cannot move an id.
    #[test]
    fn key_order_moves_no_fingerprint() {
        let one: Value = serde_json::from_str(r#"{"title":"Soup","note":"Hot"}"#).unwrap();
        let other: Value = serde_json::from_str(r#"{"note":"Hot","title":"Soup"}"#).unwrap();
        assert_eq!(fingerprint_content(&one), fingerprint_content(&other));
    }

    /// SQL answers exactly what Rust answers, which is what lets the migration
    /// and the invariant test below it be written in SQL at all.
    #[test]
    fn sql_answers_what_rust_answers() {
        let conn = Connection::open_in_memory().unwrap();
        register(&conn).unwrap();
        let content = r#"{"title":"Soup","nutrition":null,"steps":[]}"#;
        let from_sql: String = conn
            .query_row("SELECT version_fingerprint(?1)", [content], |row| {
                row.get(0)
            })
            .unwrap();
        let parsed: Value = serde_json::from_str(content).unwrap();
        assert_eq!(from_sql, fingerprint_content(&parsed));
    }

    #[test]
    fn unreadable_content_answers_null_rather_than_failing() {
        let conn = Connection::open_in_memory().unwrap();
        register(&conn).unwrap();
        let answer: Option<String> = conn
            .query_row("SELECT version_fingerprint('not json')", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(answer, None);
    }
}
