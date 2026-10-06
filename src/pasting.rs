//! Reading a whole recipe pasted as text (#94).
//!
//! **A pasted recipe is two blocks with one boundary between them, not a
//! jumble of lines to classify.** So the job is finding one split, and this
//! module's whole question is where it goes. That framing is #83's, measured
//! before it was chosen, and it is what keeps this a page of arithmetic
//! rather than a classifier.
//!
//! **Every line goes in exactly as pasted** (ADR 0002). What is decided here
//! is what a line *is* — an ingredient, a step, a heading — and never what it
//! *says*. No amount is lifted out of a line, no word is rewritten, no line is
//! reordered. Two repairs are made. [`crate::entities`]'s, which every
//! importer makes and none of them owns: a page that escaped its own text
//! leaves `Noodles &amp; choi sum:` behind, and that is not what anybody typed.
//! And the list marker in front of a line (`•`, `-`, `*`, `–`, #176): it marks
//! the line as a list item, which the list it lands in already says, and is
//! not a word of it. A marker removed is a mark, not a word, so ADR 0002 holds.
//!
//! **A paste is not only two blocks, though.** A page puts prose about the
//! recipe around its lists, under `Description`, `Notes` or `Suggestions de
//! service :`, and that has nowhere to go in either. It goes to the recipe's
//! own `note` instead, every line of it as pasted (#176), and is taken out
//! before the split is looked for, so it never votes on where the method
//! starts.
//!
//! **Nothing else is guessed.** No Yield, no times, no Source, and no
//! Component: ADR 0008 refuses matching a written line against a recipe on the
//! shelf in as many words, so a pasted line naming one is an ordinary
//! Ingredient Line until a person says otherwise.
//!
//! ## What it scores, and why those three things
//!
//! Measured over the 80 recipes in the real 86-recipe export that carry both a
//! list and a method (`tests/pasting_corpus.rs`):
//!
//! | | ingredient lines (855) | steps (599) |
//! |---|---|---|
//! | ends in `.`, `!` or `?` | **0.0%** | **80.5%** |
//! | opens with an amount | **71.8%** | **1.3%** |
//! | median length | **4 words** | **16 words** |
//!
//! Three signals that clean need no fourth. The amount and the Unit are read
//! by [`crate::reading::read_line`] rather than by a leading-digit test, which
//! is what makes this work outside English: `20 cl de crème fraîche` is a line
//! `reading.rs` reads in full, because [`crate::units`] holds the Unit
//! vocabulary in three Languages.
//!
//! **The boundary is found exactly in 81.2% of them (65 of 80) and within
//! one line in 95.0% (76 of 80)**, against the 92% within-one-line bar #94 set
//! from the throwaway scorer. Four recipes miss by more than a line. Measured
//! on 27 September 2026 after #176's headings, bullets and notes; before them
//! it was 77.5% and 95.0%, and #176 forbade either to fall.
//!
//! ## Which is why nothing here is applied silently
//!
//! That is the whole reason the screen shows what was made of the paste
//! *before* anything lands, names the counts, and offers
//! *move where the method starts*. A parser right four times in five and silent
//! the fifth is worse than one right four times in five that says so.
//!
//! The 18.8% it does not land exactly is mostly off by a single line. The four
//! that miss by more are the ones whose steps are short and imperative enough
//! to read like ingredients — *Korean Beef Noodles* has 5 ingredients and 9
//! clipped steps, and no weighting separates `Boil the noodles` from
//! `Spring onions` on the words alone.
//!
//! ## What a heading is, and what it deliberately is not
//!
//! **A heading is a line ending in a colon**, short, carrying no quantity.
//! That rule is narrow on purpose. Of the 20 step Sections in the real export
//! only 6 are written with the colon (`Dan Dan Sauce:`, `Noodles & choi sum:`)
//! and 14 are not (`Drain the Tofu`, `Prepare the Chili Garlic Sauce`) — so
//! this finds under a third of the real ones and leaves the rest as ordinary
//! steps, which is a heading somebody re-marks by hand on the screen they are
//! already standing on.
//!
//! The wider rule that would catch them — a short line in Title Case with no
//! final stop — is refused, because `Salt and pepper` and `Spring onions` fit
//! it exactly and turning either into a heading loses an ingredient. Missing a
//! heading costs one tap; inventing one silently deletes a line of the recipe.
//!
//! **The one exception is a fixed list of words** (#176): a line that is
//! exactly `Ingrédients`, `Préparation`, `Method`, `Instructions`,
//! `Preparación` and a few more in the three Languages is a heading with no
//! colon. That does not reopen the refusal above. A list of whole lines names
//! what a heading *is*, and `Salt and pepper` is on no list; a rule about the
//! shape of a line would guess. And a heading heads what follows it, so one
//! left at the foot of the ingredients by the split goes to the top of the
//! method.

use crate::entities::decode_entities;
use crate::reading;

/// What one pasted line turned out to be. A **Section** is a Section in
/// either block; everything else is an Ingredient Line above the boundary and
/// a Step below it, which is what lets the boundary move without re-reading a
/// line.
///
/// The word is GLOSSARY.md's, and `"section"` is what the Catalogue already
/// calls this in a recipe's own content — so a pasted row goes onto the page
/// as itself rather than being translated on the way in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// An ordinary line: an Ingredient Line, or a Step, according to which
    /// side of the boundary it falls.
    Line,
    /// A Section — a named part of whichever list it lands in.
    Section,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Line => "line",
            Kind::Section => "section",
        }
    }
}

/// One line of the paste, exactly as it was pasted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub kind: Kind,
}

/// What Kamosu made of a paste: a title it may or may not have found, every
/// line in the order they were pasted, and the one index the method starts at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paste {
    /// The first line where it stands alone above a blank line, or nothing.
    /// A guess, and corrected by typing in the title field like any other.
    pub title: Option<String>,
    /// Every line of the paste but the title and the blanks, in order.
    pub lines: Vec<Line>,
    /// Where the method starts: `lines[..boundary]` are the ingredients and
    /// `lines[boundary..]` are the steps. `0` and `lines.len()` are both real
    /// answers — a paste can be all method or all list.
    pub boundary: usize,
    /// What the paste said *about* the recipe rather than in it: a block
    /// under `Description`, `Notes` or `Suggestions de service :`, every line
    /// of it exactly as pasted. It lands in the recipe's own `note`, and
    /// nothing here is lost by leaving the two lists (#176).
    pub note: Option<String>,
}

/// Past this many **bytes** a paste is not a recipe somebody typed. The
/// arithmetic below is quadratic in nothing, but it is linear in this, so the
/// bound is on the text Kamosu agrees to hold rather than on how many letters
/// a recipe may have. A refusal says so rather than working for a minute.
pub const LONGEST_PASTE: usize = 64 * 1024;

/// **Read a whole pasted recipe into a title, a list and a method.**
///
/// Pure, and deliberately so: no database, no Core, no Person. It is testable
/// against corpus fixtures with no server, the same way `extract_recipe` is.
pub fn read_paste(text: &str) -> Paste {
    let (title, rest) = take_title(text);
    read_lines(title, rest)
}

/// **Read the text a PDF printed** (#176): [`read_paste`] after two things
/// only a printed page needs.
///
/// A PDF keeps where each printed line ended, not where the sentence did, so
/// a step that wrapped comes out as three lines and would be read as three
/// steps. [`join_wrapped`] puts them back together. And a printed page leaves
/// no blank line under its title, so the first line is the title here without
/// one. Neither happens to pasted text, whose line breaks are the ones the
/// person typed.
pub fn read_printed(text: &str) -> Paste {
    let rows: Vec<String> = text
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();
    let mut rows = join_wrapped(rows)
        .into_iter()
        .map(|line| clean(&line))
        .filter(|line| !line.is_empty());
    let title = rows.next();
    read_lines(title, rows.collect())
}

/// **One printed line that wrapped is one line.** A line that does not end
/// the way a sentence or a heading ends (`.` `!` `?` `:`), followed by one
/// that starts in lowercase, is the same line carried on.
///
/// With one guard the ticket's rule did not name: only a line that ran most
/// of the way across the page can have wrapped. `2 eggs` above `salt and
/// pepper` fits the rule and is two ingredients, and what tells them apart is
/// that `2 eggs` stopped far short of the margin. Half the longest line is the
/// bar, since a wrapped line always runs close to the longest.
fn join_wrapped(lines: Vec<String>) -> Vec<String> {
    let longest = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let mut joined: Vec<String> = Vec::with_capacity(lines.len());
    for line in lines {
        let carries_on = joined.last().is_some_and(|above: &String| {
            !above.ends_with(['.', '!', '?', ':'])
                && above.chars().count() * 2 >= longest
                && line.chars().next().is_some_and(char::is_lowercase)
        });
        match joined.last_mut() {
            Some(above) if carries_on => {
                above.push(' ');
                above.push_str(&line);
            }
            _ => joined.push(line),
        }
    }
    joined
}

/// What both ways in share once the title is settled: the notes taken out,
/// every remaining line looked at once, and the one split.
fn read_lines(title: Option<String>, rest: Vec<String>) -> Paste {
    let (rest, note) = take_notes(rest);
    let title = title.map(unbullet).filter(|title| !title.is_empty());
    // A marker on a line of its own is a line of nothing, and goes.
    let rest: Vec<String> = rest
        .into_iter()
        .filter(|line| unbulleted(line).is_none_or(|kept| !kept.trim().is_empty()))
        .collect();

    // Read once, here. Everything below works off what this pass found, so no
    // line is put through `reading.rs` twice however often the score is asked
    // for.
    //
    // **Each line is judged as it was pasted, marker and all**, and only the
    // text that comes back loses it. A bulleted list inside a method is its
    // sub-list of things to add (`• ½ tablespoon red miso paste`), and read
    // without the marker every one of them opens with an amount and drags the
    // split down to it: one real recipe was missed by nine lines that way.
    let looked: Vec<Looked> = rest.iter().map(|line| look(line)).collect();
    let boundary = split(&looked);
    let rest: Vec<String> = rest.into_iter().map(unbullet).collect();

    let lines = rest
        .into_iter()
        .zip(&looked)
        .map(|(text, looked)| Line {
            text,
            kind: looked.kind,
        })
        .collect();
    Paste {
        title,
        lines,
        boundary,
        note,
    }
}

/// **The title is the first line where it stands alone above a blank line.**
///
/// Only there. A paste whose lines run together from the first word gets no
/// title, and the title field stays empty rather than eating the first
/// ingredient — which is the failure nobody would notice until the recipe was
/// saved a line short.
///
/// Answers the title, and every remaining non-blank line in order.
fn take_title(text: &str) -> (Option<String>, Vec<String>) {
    let raw: Vec<&str> = text.lines().collect();
    let first = raw.iter().position(|line| !line.trim().is_empty());

    let Some(first) = first else {
        return (None, Vec::new());
    };
    let stands_alone = raw
        .get(first + 1)
        .is_some_and(|next| next.trim().is_empty())
        && raw[first + 2..].iter().any(|line| !line.trim().is_empty());

    // A title that cleans away to nothing is no title: `&nbsp;` on its own
    // above a blank line is not what anybody meant by one.
    let (title, body) = match stands_alone {
        true => (
            Some(clean(raw[first])).filter(|t| !t.is_empty()),
            &raw[first + 1..],
        ),
        false => (None, &raw[first..]),
    };

    let lines = body
        .iter()
        .map(|line| clean(line))
        .filter(|line| !line.is_empty())
        .collect();
    (title, lines)
}

/// The one repair, made on every importer's behalf in one place (#69): text
/// that was never HTML carrying HTML entities, plus the surrounding whitespace
/// a paste always brings, and the list marker in front of it (#176). Nothing
/// inside the line is touched (ADR 0002).
///
/// The marker comes off last, in [`read_lines`], because until the notes are
/// taken out it is a sign that the recipe has started again.
fn clean(line: &str) -> String {
    decode_entities(line.trim()).trim().to_string()
}

/// A line without its list marker.
fn unbullet(line: String) -> String {
    match unbulleted(&line) {
        Some(rest) => rest.trim().to_string(),
        None => line,
    }
}

/// The list markers a copied page or a printed one puts before an item.
const BULLETS: [char; 4] = ['•', '-', '*', '–'];

/// What follows this line's list marker, or nothing where it has none.
///
/// A marker is a bullet followed by a space, or `•` on its own, which nobody
/// types inside a word. `-5 °C` and `*optional*` keep what they start with,
/// because there the mark is part of what was written. A line that is nothing
/// but a marker cleans away to nothing and is dropped, which is how a PDF
/// that printed each bullet on a line of its own reads.
fn unbulleted(line: &str) -> Option<&str> {
    let rest = line.strip_prefix(BULLETS)?;
    (line.starts_with('•') || rest.is_empty() || rest.starts_with(char::is_whitespace))
        .then_some(rest)
}

// --- Headings known by name (#176) ------------------------------------------
//
// Two short, fixed lists, in the three Languages Kamosu reads. They are the
// only words recognised as headings without a colon, and only when the line
// is that word and nothing else: the module header's reason for refusing the
// wider Title Case rule still holds, and a fixed list of whole lines cannot
// swallow `Salt and pepper`.

/// A line that is exactly one of these is a Section, colon or none. The words
/// a recipe page puts over its two lists: `Préparation` has no colon, and was
/// read as a step.
const SECTION_WORDS: &[&str] = &[
    // English
    "ingredients",
    "method",
    "instructions",
    "directions",
    "preparation",
    "steps",
    // Français
    "ingrédients",
    "préparation",
    "étapes",
    "méthode",
    "instructions",
    "déroulé",
    // Español
    "ingredientes",
    "preparación",
    "elaboración",
    "instrucciones",
    "pasos",
    "modo de preparación",
];

/// A block under one of these is about the recipe rather than in it, and
/// goes to its `note`.
const NOTE_WORDS: &[&str] = &[
    // English
    "description",
    "notes",
    "note",
    "tips",
    "serving suggestions",
    "storage",
    // Français
    "description",
    "notes",
    "note",
    "conseils",
    "astuces",
    "suggestions de service",
    "suggestion de service",
    "conservation",
    // Español
    "descripción",
    "notas",
    "nota",
    "consejos",
    "sugerencias",
    "sugerencias de presentación",
    "conservación",
];

/// The line as a heading reads, whatever its case and however its colon is
/// spaced: `Préparation`, `PRÉPARATION :` and `préparation:` are one word.
fn as_heading(line: &str) -> String {
    line.trim().trim_end_matches(':').trim_end().to_lowercase()
}

fn is_section_word(line: &str) -> bool {
    SECTION_WORDS.contains(&as_heading(line).as_str())
}

/// Where a line opens a note, and how far that note runs.
enum NoteStart {
    /// `Description` or `Notes :` alone on its line. The heading is the
    /// field's own name, so it is not kept, and the block runs until
    /// something that is plainly the recipe again.
    Heading,
    /// `Suggestions de service : Le biscuit…`: the heading and the note on
    /// one line, kept whole. It runs on only through the lines that carry it
    /// on in lowercase, so `Note: it freezes well.` in the middle of a method
    /// takes its own line and not the steps after it.
    Inline,
}

fn note_start(line: &str) -> Option<NoteStart> {
    if NOTE_WORDS.contains(&as_heading(line).as_str()) {
        return Some(NoteStart::Heading);
    }
    let (heading, said) = line.split_once(':')?;
    (NOTE_WORDS.contains(&heading.trim().to_lowercase().as_str()) && !said.trim().is_empty())
        .then_some(NoteStart::Inline)
}

/// **Take the notes out before anything is split.** They are neither
/// ingredients nor steps, and left in they vote on the boundary: a
/// description is long prose, which reads as method, and put the split of
/// *Biscuit de Savoie* seven lines early.
///
/// A heading's block ends at the first line that is plainly the recipe
/// again ([`ends_a_note`]). Answers the lines left, and the note.
fn take_notes(lines: Vec<String>) -> (Vec<String>, Option<String>) {
    let mut kept = Vec::with_capacity(lines.len());
    let mut notes: Vec<Vec<String>> = Vec::new();
    let mut open: Option<NoteStart> = None;

    for line in lines {
        if let Some(start) = note_start(&line) {
            let mut block = Vec::new();
            if matches!(start, NoteStart::Inline) {
                block.push(line);
            }
            notes.push(block);
            open = Some(start);
            continue;
        }
        let carried_on = match open {
            Some(NoteStart::Heading) => !ends_a_note(&line),
            Some(NoteStart::Inline) => line.chars().next().is_some_and(char::is_lowercase),
            None => false,
        };
        match notes.last_mut() {
            Some(block) if carried_on => block.push(line),
            _ => {
                open = None;
                kept.push(line);
            }
        }
    }

    let note = notes
        .into_iter()
        .filter(|block| !block.is_empty())
        .map(|block| block.join("\n"))
        .collect::<Vec<_>>()
        .join("\n\n");
    (kept, Some(note).filter(|note| !note.is_empty()))
}

/// The recipe starting again after a note: one of its own headings, a
/// numbered step, a list item, or a short line that opens with an amount and
/// a Unit, which is what an ingredient is and a sentence of prose is not.
/// An amount alone is not a safe sign: `180°` opens prose as readily as a
/// list, so the Unit and the shortness are both asked for.
fn ends_a_note(line: &str) -> bool {
    let words = line.split_whitespace().count();
    let reads_as_an_ingredient = words <= SHORT && {
        let reading = reading::read_line(line).unwrap_or_default();
        reading.amount.is_some() && reading.unit.is_some()
    };
    is_section_word(line)
        || strip_step_number(line).is_some()
        || unbulleted(line).is_some()
        || reads_as_an_ingredient
        || (line.ends_with(':') && words <= LONGEST_SECTION)
}

/// A Section: a line ending in a colon, short, carrying no quantity. See the
/// module header for why the rule is not wider than that.
const LONGEST_SECTION: usize = 8;

/// A step's own number is not an amount. `1.` parses as one perfectly well —
/// which is exactly the trap, since a numbered method is the commonest thing
/// anybody pastes. Stripped only in front of prose: `1. flour` in a numbered
/// ingredient list keeps its number, because two words are not a sentence.
const SHORTEST_NUMBERED_STEP: usize = 7;

/// What follows this line's own step number, or nothing where it has none.
fn strip_step_number(line: &str) -> Option<&str> {
    let rest = line.trim_start();
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 || digits > 2 {
        return None;
    }
    let after = rest[digits..].strip_prefix(['.', ')'])?;
    (after.split_whitespace().count() >= SHORTEST_NUMBERED_STEP).then_some(after)
}

/// Everything the split needs to know about one line, found in a single pass.
///
/// The amount and the Unit are [`crate::reading`]'s answers rather than a
/// leading-digit test's, which is the point of resting on it: its vocabulary
/// is `units.rs`'s, in English, French and Spanish.
struct Looked {
    kind: Kind,
    words: usize,
    /// Opens with its own step number, which is not an amount.
    numbered: bool,
    amount: bool,
    unit: bool,
    /// Ends the way a sentence ends.
    sentence: bool,
}

fn look(line: &str) -> Looked {
    let text = line.trim();
    let words = text.split_whitespace().count();
    let numbered = strip_step_number(text).is_some();
    let reading = reading::read_line(strip_step_number(text).unwrap_or(text)).unwrap_or_default();

    let kind = if is_section_word(text)
        || (text.ends_with(':') && words <= LONGEST_SECTION && reading.amount.is_none())
    {
        Kind::Section
    } else {
        Kind::Line
    };
    Looked {
        kind,
        words,
        numbered,
        amount: reading.amount.is_some(),
        unit: reading.unit.is_some(),
        // A four-word line ending in a full stop is an abbreviation or an
        // aside, not a sentence: `Flour, approx.` is a real Ingredient Line.
        sentence: words > 4 && text.ends_with(['.', '!', '?']),
    }
}

// --- The one split -----------------------------------------------------------
//
// Each line is scored for how much it reads as an ingredient rather than as a
// step, and the boundary goes where the sum above it least resembles the sum
// below. The weights are round numbers because they are round: the corpus puts
// every neighbouring value within 1.2 points of this one on both scores, so
// nothing here is balanced on a knife edge and nothing was fitted to the 80
// recipes it was measured on.

/// A line opening with an amount. The strongest ingredient signal there is:
/// 71.8% of real Ingredient Lines against 1.3% of real steps.
const OPENS_WITH_AN_AMOUNT: f64 = 1.0;
/// A Unit behind the amount. Corroboration rather than evidence of its own,
/// which is why it is worth less than half of the amount.
const CARRIES_A_UNIT: f64 = 0.4;
/// A line ending the way a sentence ends. Nothing in 855 real Ingredient Lines
/// does; 80.5% of real steps do.
const ENDS_LIKE_A_SENTENCE: f64 = 1.0;
/// A line opening with its own step number.
const NUMBERED_LIKE_A_STEP: f64 = 1.0;

/// Short enough to be a thing rather than an instruction: 82.2% of Ingredient
/// Lines are this short and 15.7% of steps are.
const SHORT: usize = 6;
const IS_SHORT: f64 = 0.4;
/// Long enough to be prose: 2.8% of Ingredient Lines run this long, 61.6% of
/// steps do.
const LONG: usize = 12;
const IS_LONG: f64 = 0.8;

/// How much this line reads as an ingredient rather than as a step. Positive
/// is an ingredient, negative is a step, and zero is no opinion either way.
///
/// **A Section has no opinion.** It is real in both blocks, so letting one
/// vote would move the boundary by whichever block happened to have more of
/// them.
fn ingredient_score(line: &Looked) -> f64 {
    if line.kind == Kind::Section {
        return 0.0;
    }
    let mut score = 0.0;
    if line.amount {
        score += OPENS_WITH_AN_AMOUNT;
    }
    if line.unit {
        score += CARRIES_A_UNIT;
    }
    if line.numbered {
        score -= NUMBERED_LIKE_A_STEP;
    }
    if line.sentence {
        score -= ENDS_LIKE_A_SENTENCE;
    }
    if line.words <= SHORT {
        score += IS_SHORT;
    }
    if line.words >= LONG {
        score -= IS_LONG;
    }
    score
}

/// **Where the method starts.** The split at which everything above it reads
/// most like a list and everything below it least like one.
///
/// `0` and `lines.len()` are both allowed answers, so a paste that is all
/// method or all list comes back as one rather than being forced to give up a
/// line to the other side.
///
/// **A tie goes to the later split**, which keeps a trailing run of
/// quantity-less lines — `salt and pepper`, `a handful of coriander` — with
/// the ingredients above them rather than handing them to the method. Both
/// rules score the same within one line; this one is better on exact.
fn split(lines: &[Looked]) -> usize {
    let scores: Vec<f64> = lines.iter().map(ingredient_score).collect();
    let total: f64 = scores.iter().sum();

    let mut above = 0.0;
    let mut best = -total; // the split at 0: everything is method.
    let mut boundary = 0;
    for (at, score) in scores.iter().enumerate() {
        above += score;
        let separation = above - (total - above);
        if separation >= best {
            best = separation;
            boundary = at + 1;
        }
    }

    // A heading heads what follows it. A Section has no opinion, so the tie
    // above leaves `Préparation` at the foot of the ingredients, heading
    // nothing; it goes to the top of the method instead (#176).
    while boundary > 0 && lines[boundary - 1].kind == Kind::Section {
        boundary -= 1;
    }
    boundary
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split_of(text: &str) -> (Vec<String>, Vec<String>) {
        let paste = read_paste(text);
        let text_of = |rows: &[Line]| rows.iter().map(|row| row.text.clone()).collect();
        (
            text_of(&paste.lines[..paste.boundary]),
            text_of(&paste.lines[paste.boundary..]),
        )
    }

    const PLAIN: &str = "\
Chocolate Cake

200 g plain flour
2 eggs
a pinch of salt
Heat the oven to 180C and butter a tin.
Beat the eggs into the flour until it is smooth, then pour it in.
Bake for twenty-five minutes, until a skewer comes out clean.";

    #[test]
    fn a_pasted_recipe_becomes_a_title_a_list_and_a_method() {
        let paste = read_paste(PLAIN);
        assert_eq!(paste.title.as_deref(), Some("Chocolate Cake"));
        let (ingredients, steps) = split_of(PLAIN);
        assert_eq!(
            ingredients,
            ["200 g plain flour", "2 eggs", "a pinch of salt"]
        );
        assert_eq!(steps.len(), 3);
        assert!(steps[0].starts_with("Heat the oven"));
    }

    #[test]
    fn every_line_arrives_exactly_as_pasted() {
        let text = "  2 poignées de farine, environ  \n1 1/2 cup sugar\nMix it all together thoroughly and then bake it.";
        let paste = read_paste(text);
        // Trimmed of the whitespace a paste brings, and otherwise untouched:
        // no amount lifted out, no word rewritten, no line reordered.
        assert_eq!(paste.lines[0].text, "2 poignées de farine, environ");
        assert_eq!(paste.lines[1].text, "1 1/2 cup sugar");
        assert_eq!(paste.boundary, 2);
    }

    #[test]
    fn a_section_is_a_line_ending_in_a_colon_in_either_block() {
        let text = "\
Dan Dan Noodles

Dan Dan Sauce:
2 tbsp Chinese sesame paste
chilli oil, to taste
Noodles &amp; choi sum:
200 g noodles
Assemble:
Mix the sauce ingredients in a bowl and set it aside.
Pile the noodles in and top them with the pork mixture.";
        let paste = read_paste(text);
        let kinds: Vec<&str> = paste.lines.iter().map(|row| row.kind.as_str()).collect();
        assert_eq!(
            kinds,
            [
                "section", "line", "line", "section", "line", "section", "line", "line"
            ]
        );
        // The entity is undone, because it was never HTML (#69).
        assert_eq!(paste.lines[3].text, "Noodles & choi sum:");
        // `Assemble:` heads the method rather than trailing the list (#176).
        let (ingredients, steps) = split_of(text);
        assert_eq!(ingredients.len(), 5);
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn a_paste_with_no_sections_at_all_still_splits() {
        let (ingredients, steps) = split_of(PLAIN);
        assert_eq!(ingredients.len(), 3);
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn a_numbered_method_does_not_read_its_numbers_as_amounts() {
        let text = "\
300 g flour
2 eggs
1. Heat the oven to 180C and butter a twenty centimetre tin.
2. Beat the eggs into the flour until the batter is quite smooth.
3) Bake it for twenty-five minutes, until a skewer comes out clean.";
        let (ingredients, steps) = split_of(text);
        assert_eq!(ingredients, ["300 g flour", "2 eggs"]);
        assert_eq!(steps.len(), 3);
        // The number is part of the written line and is not stripped from it.
        assert!(steps[0].starts_with("1. Heat"));
    }

    #[test]
    fn a_numbered_ingredient_list_keeps_its_numbers() {
        // Two words are not a sentence, so `1.` here is an amount and not an
        // ordinal — which is the right answer either way, since both keep the
        // line above the boundary.
        let text = "1. flour\n2. sugar\nStir the two of them together in a large bowl and bake.";
        let (ingredients, _) = split_of(text);
        assert_eq!(ingredients, ["1. flour", "2. sugar"]);
    }

    #[test]
    fn the_title_is_only_taken_where_it_stands_alone() {
        // No blank line under it: the first line is an ordinary line, and the
        // title field stays empty rather than eating an ingredient.
        let paste =
            read_paste("200 g flour\n2 eggs\nMix them together well and then bake the lot.");
        assert_eq!(paste.title, None);
        assert_eq!(paste.lines[0].text, "200 g flour");

        // A blank line and nothing under it is not a title either.
        let paste = read_paste("200 g flour\n\n");
        assert_eq!(paste.title, None);
        assert_eq!(paste.lines.len(), 1);

        // And a first line that cleans away to nothing is no title, rather
        // than a title of the empty string — which would have put an empty
        // field on the screen and called it a guess. `&#160;` is a
        // non-breaking space, which a copied web page leaves behind on a line
        // that looked blank; `&nbsp;` is NOT this case, because
        // `entities::decode_entities` deliberately decodes only the five
        // XML-predefined names and the numeric references.
        let paste = read_paste("&#160;\n\n200 g flour\nMix it in well and bake the whole lot.");
        assert_eq!(paste.title, None);
        assert_eq!(paste.lines.len(), 2);
    }

    #[test]
    fn a_unit_in_another_language_is_read_as_one() {
        // The whole reason this rests on `reading.rs`: a leading-digit test
        // half-reads this line and `units.rs` reads all of it.
        let text = "20 cl de crème fraîche\n2 gousses d'ail\nFaites revenir l'ail dans une poêle bien chaude.";
        let (ingredients, steps) = split_of(text);
        assert_eq!(ingredients, ["20 cl de crème fraîche", "2 gousses d'ail"]);
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn a_paste_of_nothing_is_not_an_error() {
        let paste = read_paste("   \n\n  ");
        assert_eq!(paste.title, None);
        assert!(paste.lines.is_empty());
        assert_eq!(paste.boundary, 0);
    }

    #[test]
    fn a_paste_that_is_all_list_or_all_method_comes_back_as_one() {
        let list = read_paste("200 g flour\n2 eggs\n1 tsp salt\na pinch of pepper");
        assert_eq!(list.boundary, list.lines.len());

        let method = read_paste(
            "Heat the oven to 180C and butter a twenty centimetre tin.\n\
             Beat the eggs into the flour until the batter is quite smooth.\n\
             Bake it for twenty-five minutes, until a skewer comes out clean.",
        );
        assert_eq!(method.boundary, 0);
    }

    #[test]
    fn nothing_is_guessed_beyond_the_split_and_the_title() {
        // A pasted line naming a recipe on the shelf is an ordinary Ingredient
        // Line (ADR 0008), and there is nowhere here for a Yield, a time or a
        // Source to be invented: the shape carries none of them.
        let paste = read_paste(
            "Pizza\n\n500 g pizza dough\nServes 4\nPrep 20 minutes\nRoll the dough out thinly and top it with the sauce.",
        );
        assert_eq!(paste.title.as_deref(), Some("Pizza"));
        assert!(paste.lines.iter().all(|row| !row.text.is_empty()));
        assert_eq!(paste.lines.len(), 4);
    }

    #[test]
    fn a_list_marker_comes_off_and_nothing_else_does() {
        let paste = read_paste(
            "• Trois gros œufs\n- 150 g de sucre\n* 30 g de farine\n– 45 g de fécule\n•\n-5 °C freezer\n\
             Beat everything together in a bowl until it is pale and thick.",
        );
        let texts: Vec<&str> = paste.lines.iter().map(|row| row.text.as_str()).collect();
        // A marker on a line of its own is a line of nothing, and goes. A
        // minus sign in front of a number is part of what was written.
        assert_eq!(
            texts[..5],
            [
                "Trois gros œufs",
                "150 g de sucre",
                "30 g de farine",
                "45 g de fécule",
                "-5 °C freezer"
            ]
        );
    }

    #[test]
    fn a_known_heading_word_is_a_section_with_no_colon() {
        let paste = read_paste(
            "Ingredients\n200 g flour\n2 eggs\nPréparation\n\
             Heat the oven to 180C and butter a twenty centimetre tin.\n\
             Beat the eggs into the flour until the batter is quite smooth.",
        );
        let kinds: Vec<&str> = paste.lines.iter().map(|row| row.kind.as_str()).collect();
        assert_eq!(
            kinds,
            ["section", "line", "line", "section", "line", "line"]
        );
        // And it heads the method rather than trailing the ingredients.
        assert_eq!(paste.boundary, 3);

        // Only the whole line: a longer one merely starting with the word is
        // whatever it would have been.
        let paste = read_paste("Method for the sauce is below\n200 g flour");
        assert_eq!(paste.lines[0].kind, Kind::Line);
    }

    #[test]
    fn a_heading_heads_what_follows_it_rather_than_trailing_the_list() {
        let (ingredients, steps) = split_of(
            "Dan Dan Sauce:\n2 tbsp Chinese sesame paste\n200 g noodles\nAssemble:\n\
             Mix the sauce ingredients in a bowl and set it aside.",
        );
        assert_eq!(ingredients.len(), 3);
        assert_eq!(steps[0], "Assemble:");
    }

    #[test]
    fn a_block_about_the_recipe_goes_to_its_note() {
        let paste = read_paste(
            "Biscuit\n\nIngrédients :\n• 150 g de sucre\n• 30 g de farine\nDescription\n\
             Ce gros gâteau est incroyablement léger grâce aux blancs battus.\n\
             La cuisson : la chaleur du four doit être assez douce.\nPréparation\n\
             1. Préparer les jaunes : on ajoute le sucre et on travaille le mélange.\n\
             2. Incorporer les poudres : on incorpore la farine, puis la fécule.\n\
             Suggestions de service : avec une salade de fruits, une mousse\n\
             aux fruits, de la crème.",
        );
        assert_eq!(
            paste.note.as_deref(),
            Some(
                "Ce gros gâteau est incroyablement léger grâce aux blancs battus.\n\
                 La cuisson : la chaleur du four doit être assez douce.\n\n\
                 Suggestions de service : avec une salade de fruits, une mousse\n\
                 aux fruits, de la crème."
            )
        );
        let texts: Vec<&str> = paste.lines.iter().map(|row| row.text.as_str()).collect();
        assert_eq!(
            texts,
            [
                "Ingrédients :",
                "150 g de sucre",
                "30 g de farine",
                "Préparation",
                "1. Préparer les jaunes : on ajoute le sucre et on travaille le mélange.",
                "2. Incorporer les poudres : on incorpore la farine, puis la fécule.",
            ]
        );
        assert_eq!(paste.boundary, 3);
    }

    #[test]
    fn a_note_inside_a_method_takes_only_its_own_line() {
        let paste = read_paste(
            "1. Heat the oven to 180C and butter a twenty centimetre tin.\n\
             Note: it freezes well for a month.\n\
             2. Beat the eggs into the flour until the batter is quite smooth.",
        );
        assert_eq!(
            paste.note.as_deref(),
            Some("Note: it freezes well for a month.")
        );
        assert_eq!(paste.lines.len(), 2);
        assert_eq!(read_paste(PLAIN).note, None);
    }

    #[test]
    fn a_printed_line_that_wrapped_is_joined_back_into_one() {
        let paste = read_printed(
            "Biscuit de Savoie\n\n• Trois gros œufs\n\n• 150 g de sucre\n\n\
             1. Préparer les jaunes : On sépare les jaunes des blancs. On ajoute le sucre aux jaunes\n\n\
             d'œuf et on travaille longuement et soigneusement le mélange. Il doit à peu près doubler de\n\n\
             volume et devenir onctueux.\n\n\
             2. Incorporer les poudres : À ce moment, on incorpore la farine, puis la fécule.",
        );
        // The first line is the title with no blank line under it.
        assert_eq!(paste.title.as_deref(), Some("Biscuit de Savoie"));
        let texts: Vec<&str> = paste.lines.iter().map(|row| row.text.as_str()).collect();
        assert_eq!(
            texts,
            [
                "Trois gros œufs",
                "150 g de sucre",
                "1. Préparer les jaunes : On sépare les jaunes des blancs. On ajoute le sucre aux \
                 jaunes d'œuf et on travaille longuement et soigneusement le mélange. Il doit à peu \
                 près doubler de volume et devenir onctueux.",
                "2. Incorporer les poudres : À ce moment, on incorpore la farine, puis la fécule.",
            ]
        );
        assert_eq!(paste.boundary, 2);
    }

    #[test]
    fn a_short_line_did_not_wrap_whatever_follows_it() {
        // `2 eggs` stopped far short of the margin, so `salt and pepper` is a
        // line of its own.
        let paste = read_printed(
            "Omelette\n2 eggs\nsalt and pepper\n\
             Beat the eggs with the salt and pepper, then cook them gently in butter.",
        );
        let texts: Vec<&str> = paste.lines.iter().map(|row| row.text.as_str()).collect();
        assert_eq!(texts[..2], ["2 eggs", "salt and pepper"]);
    }

    #[test]
    fn pasted_text_keeps_its_line_breaks() {
        // The same wrapped lines, pasted rather than printed, are not joined:
        // the person's own line breaks are theirs.
        let paste = read_paste("volume et devenir\nonctueux et léger.");
        assert_eq!(paste.lines.len(), 2);
    }

    #[test]
    fn a_note_under_a_heading_stops_at_an_ingredient() {
        // No `Ingredients` heading after the description, and no bullets: the
        // first line that reads as an ingredient ends the note.
        let paste = read_paste(
            "Pancakes\n\nDescription\nThin French pancakes, best eaten straight from the pan.\n\
             250 g flour\n500 ml milk\n3 eggs\n\
             Whisk everything together and leave it to rest for an hour before cooking.",
        );
        assert_eq!(
            paste.note.as_deref(),
            Some("Thin French pancakes, best eaten straight from the pan.")
        );
        let texts: Vec<&str> = paste.lines.iter().map(|row| row.text.as_str()).collect();
        assert_eq!(texts[..3], ["250 g flour", "500 ml milk", "3 eggs"]);
        assert_eq!(paste.boundary, 3);
    }
}
