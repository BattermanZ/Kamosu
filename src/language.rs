//! The Language a Branch is written in, and reading it out of the recipe's own
//! text (ADR 0006, #56).
//!
//! Kamosu detects a Language and never asserts one: detection **sets** the
//! Language when a Branch has none to disagree with, and **offers** it when it
//! disagrees with one already there. Nothing here ever writes over a Language a
//! cook has stated — that is [`crate::core::Core::set_recipe_language`]'s job,
//! and it makes a Version.
//!
//! **Unknown** is a real Language here, not the absence of one. A recipe
//! honestly written in two — a French ingredient heading under an English
//! title, which the real 86-recipe corpus contains — carries it permanently,
//! shows to every reader whatever they read in, and is never offered anything.
//! No detector can be right about such a recipe, so Kamosu does not pretend to
//! be: it offers the cook a place to say so instead.
//!
//! The detector is `whatlang`, restricted to the three Languages Kamosu is
//! written in and gated on its own `is_reliable`. Restricting it matters: an
//! unrestricted detector asked about "Sukiyaki Udon" reaches for Japanese,
//! which is not one of the answers Kamosu has. The gate matters just as much —
//! a recipe that is nothing but a title and a link (the corpus holds three) is
//! far too little text to read a Language out of, and the honest answer there
//! is none.

use serde_json::Value;

/// The Languages Kamosu is written in, and the only ones detection can answer.
pub const LANGUAGES: [&str; 3] = ["en", "fr", "es"];

/// The Language of a recipe that is honestly more than one — permanent,
/// unremarkable, and never the result of detection (ADR 0006).
pub const UNKNOWN: &str = "unknown";

/// Every Language a Branch may carry: the three, plus Unknown.
pub const BRANCH_LANGUAGES: [&str; 4] = ["en", "fr", "es", UNKNOWN];

/// Whether a Branch in this Language takes part in detection at all. Unknown
/// does not: no prompt, no badge, no nag.
pub fn is_stated_unknown(language: &str) -> bool {
    language == UNKNOWN
}

/// The recipe's own text, in reading order: its title, its Ingredient Lines and
/// Section headings, its Steps, and its Note. Everything a person would read
/// off the page and nothing they would not — no ids, no Photograph hashes, no
/// Source link, whose words belong to a website rather than to the cook.
pub fn recipe_text(content: &Value) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if let Some(title) = content["title"].as_str() {
        parts.push(title);
    }
    for field in ["ingredients", "steps"] {
        if let Some(lines) = content[field].as_array() {
            parts.extend(lines.iter().filter_map(|line| line["text"].as_str()));
        }
    }
    if let Some(note) = content["note"].as_str() {
        parts.push(note);
    }
    parts.join("\n")
}

/// The Language this text reads as, or `None` when there is not enough of it
/// to say. Never answers [`UNKNOWN`]: Unknown is something a cook states, not
/// something a detector concludes.
pub fn detect(text: &str) -> Option<&'static str> {
    let detector = whatlang::Detector::with_allowlist(vec![
        whatlang::Lang::Eng,
        whatlang::Lang::Fra,
        whatlang::Lang::Spa,
    ]);
    let info = detector.detect(text)?;
    if !info.is_reliable() {
        return None;
    }
    match info.lang() {
        whatlang::Lang::Eng => Some("en"),
        whatlang::Lang::Fra => Some("fr"),
        whatlang::Lang::Spa => Some("es"),
        // Unreachable through the allowlist above, and answered as "could not
        // tell" rather than by panicking if the allowlist ever grows.
        _ => None,
    }
}

/// The Language of a recipe's content, read off the text as written.
pub fn detect_content(content: &Value) -> Option<&'static str> {
    detect(&recipe_text(content))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn recipe(title: &str, lines: &[&str]) -> Value {
        json!({
            "title": title,
            "note": Value::Null,
            "source": Value::Null,
            "ingredients": lines.iter().map(|text| json!({ "kind": "ingredient", "text": text })).collect::<Vec<_>>(),
            "steps": [],
        })
    }

    #[test]
    fn a_recipe_written_in_french_reads_as_french() {
        let content = recipe(
            "Purée de Pommes de Terre",
            &[
                "1 kg de pommes de terre à chair farineuse",
                "200 g de beurre doux coupé en dés",
                "20 cl de lait entier bien chaud",
                "Sel fin et poivre du moulin selon votre goût",
            ],
        );
        assert_eq!(detect_content(&content), Some("fr"));
    }

    #[test]
    fn a_recipe_written_in_english_reads_as_english() {
        let content = recipe(
            "Chocolate Chip Cookies",
            &[
                "2 cups of all purpose flour, sifted",
                "1 cup of soft unsalted butter at room temperature",
                "2 large eggs, beaten lightly with a fork",
                "A generous pinch of flaky sea salt over the top",
            ],
        );
        assert_eq!(detect_content(&content), Some("en"));
    }

    #[test]
    fn a_recipe_that_is_only_a_title_reads_as_nothing() {
        // Three of the real 86 are exactly this: a name, a link, and a promise
        // to fill it in later. There is no Language to be had from it, and
        // guessing one would be the silent change ADR 0006 forbids.
        assert_eq!(detect_content(&recipe("Dan Dan Noodles", &[])), None);
    }

    #[test]
    fn the_source_is_not_read_as_the_recipes_own_words() {
        let mut content = recipe("Recette", &[]);
        content["source"] = json!({
            "text": "Adapted from a long English write-up on somebody else's blog, \
                     quoted here in full because the citation happens to be wordy",
            "link": "https://example.com/recipes/1",
        });
        // A citation names where the recipe came from; it is not the recipe.
        // Left in, this alone would be enough English text to overrule a French
        // recipe that is nothing but its title.
        assert!(!recipe_text(&content).contains("blog"));
    }
}
