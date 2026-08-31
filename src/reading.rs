//! Reading an Ingredient Line — laying a **Reading** over what the cook wrote
//! without touching a character of it (ADR 0002, ADR 0021, #71).
//!
//! **The written line is the truth and nothing here changes it.** What this
//! module produces is the optional second half of ADR 0002: how much, in what
//! **Unit**, of what. A line it cannot read gets no Reading, which is not an
//! error and not a rare case — 239 of the 863 real Ingredient Lines in the
//! corpus carry no quantity at all, and every one of them is a working line.
//!
//! **A Reading produced here is Kamosu's reading, not the cook's** (ADR 0021).
//! It makes no Version, appears in no Thread, and is no part of a fingerprint,
//! which is exactly what lets a future change to this file improve every
//! Reading in the library without re-fingerprinting a single recipe.
//!
//! ## Why this is Kamosu's own code and not a library
//!
//! #71 originally deferred this to an `ingredient-parser-nlp` subprocess
//! worker. Measured against the corpus, the two credible libraries both lost
//! to ninety lines resting on [`crate::units`] — because both carry their own
//! **English-only** vocabulary of Units, and Kamosu already has that
//! vocabulary in three Languages. `20 cl de crème fraîche` is a line neither
//! library reads and this one does. See ADR 0036 for the measurement.
//!
//! **So the split is the only thing here.** Where the number ends, where the
//! Unit ends and where the Food begins is this module's whole question;
//! *what the number is worth* is [`crate::units::parse_amount`] and *what the
//! Unit is* is [`crate::units::recognise`], both of which already existed.
//!
//! ## The one rule that shapes the rest
//!
//! **The set of Units is open; the set Kamosu can convert is closed.** A word
//! is read as a Unit when the closed set knows it, or when it is one of the
//! vessels and handfuls below — never merely because it stands where a Unit
//! would stand. Without that restraint `3 chicken breasts` reads *chicken* as
//! a Unit of breasts, and a Food list fills with words that are not foods.
//!
//! [`OPEN_UNITS`] is therefore not a second vocabulary competing with
//! [`crate::units`], which was the whole objection to the libraries: it is the
//! **open** half ADR 0016 says `units.rs` deliberately does not hold, because
//! nothing in it converts to anything. `units.rs` answers *what is this Unit
//! worth*; these words never have an answer, and are listed here only so the
//! split knows where a Food begins. Every word that does convert is looked up
//! there and defined nowhere else.

use crate::units;

/// What Kamosu read off one Ingredient Line: the three parts of a Reading,
/// each optional and each meaningless without the written line above it.
///
/// `amount` is kept as the text that was written rather than a number, because
/// that is what a Reading stores and what `set_reading` has always taken —
/// `1 1/2` stays `1 1/2`, and what it is worth is `units::parse_amount`'s
/// business at the moment somebody needs the arithmetic.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Reading {
    pub amount: Option<String>,
    pub unit: Option<String>,
    pub target: Option<String>,
}

impl Reading {
    /// Whether there is anything here worth storing. A Reading of three
    /// nothings is the absence of a Reading, and storing one would be a row
    /// claiming Kamosu read a line it did not.
    fn is_something(&self) -> bool {
        self.amount.is_some() || self.unit.is_some() || self.target.is_some()
    }
}

/// Units outside the closed set that a recipe writes constantly: a vessel, a
/// piece, a handful. None of them converts to anything — a *gousse* is not a
/// quantity of anything measurable — and that is fine: ADR 0016 keeps the set
/// of Units open precisely so these are ordinary rather than exceptional.
///
/// They are listed in the three interface Languages, unaccented spellings
/// included, because the fold in [`folded`] strips accents but a French
/// keyboard produces both.
const OPEN_UNITS: &[&str] = &[
    // English
    "clove",
    "cloves",
    "can",
    "cans",
    "tin",
    "tins",
    "packet",
    "packets",
    "package",
    "packages",
    "sachet",
    "sachets",
    "pinch",
    "pinches",
    "bunch",
    "bunches",
    "head",
    "heads",
    "stick",
    "sticks",
    "slice",
    "slices",
    "strip",
    "strips",
    "leaf",
    "leaves",
    "sprig",
    "sprigs",
    "stalk",
    "stalks",
    "strand",
    "strands",
    "jar",
    "jars",
    "bottle",
    "bottles",
    "box",
    "boxes",
    "bag",
    "bags",
    "piece",
    "pieces",
    "drop",
    "drops",
    "dash",
    "dashes",
    "handful",
    "handfuls",
    "knob",
    "knobs",
    "square",
    "squares",
    "sheet",
    "sheets",
    "block",
    "blocks",
    // French
    "gousse",
    "gousses",
    "boite",
    "boites",
    "pincee",
    "pincees",
    "bouquet",
    "bouquets",
    "tranche",
    "tranches",
    "brin",
    "brins",
    "feuille",
    "feuilles",
    "poignee",
    "poignees",
    "noix",
    "filet",
    "filets",
    "pot",
    "pots",
    "botte",
    "bottes",
    "branche",
    "branches",
    "morceau",
    "morceaux",
    "trait",
    "traits",
    "cuillere",
    "cuilleres",
    "verre",
    "verres",
    "paquet",
    "paquets",
    // Spanish
    "diente",
    "dientes",
    "rama",
    "ramas",
    "hoja",
    "hojas",
    "punado",
    "punados",
    "rodaja",
    "rodajas",
    "lata",
    "latas",
    "manojo",
    "manojos",
    "pizca",
    "pizcas",
    "trozo",
    "trozos",
    "ramita",
    "ramitas",
    "chorrito",
    "chorritos",
];

/// Words that stand exactly where a Unit stands and are not one: they describe
/// the thing rather than measure it. Dropped so `1 large onion` and `2 onions`
/// point at one Food rather than two.
const SIZES: &[&str] = &[
    "large", "small", "medium", "big", "whole", "extra", "jumbo", "gros", "grosse", "petit",
    "petite", "moyen", "moyenne", "grand", "grande", "pequeno", "pequena", "mediano", "mediana",
    "grandes", "entier", "entiere",
];

/// The article or preposition gluing an amount to what it is an amount of.
/// A name never keeps one: `200 g de farine` is a Reading of *farine*, and a
/// Food called *de farine* would never match the *farine* beside it.
///
/// This is the whole of why the corpus's French lines read correctly here and
/// do not in either library measured in ADR 0036.
///
/// **`d` and `l` are deliberately absent.** They are glue only as `d'` and
/// `l'`, which [`split_elisions`] leaves carrying their apostrophe and
/// [`is_glue`] recognises by that apostrophe — because a bare `l` is a litre,
/// and `1 l milk` is a real line in the corpus.
const GLUE: &[&str] = &[
    "of", "de", "du", "des", "la", "le", "les", "un", "une", "el", "los", "las", "al", "the", "a",
];

/// The elided articles French and Spanish write against the next word. Split
/// off so [`GLUE`] can see them, and glued back wherever one survives inside a
/// name — `un filet d'huile d'olive` keeps its *d'huile*.
const ELISIONS: &[&str] = &["d'", "l'", "qu'", "n'"];

/// Fold a word for comparison against the lists above: the same Unicode fold
/// [`crate::units`] uses, then everything that is not a letter or a digit
/// thrown away — combining marks included — so neither punctuation nor an
/// accent can make one spelling two. `pincée`, `pincee` and `Pincée` are one
/// word here, which is what lets each list be written once.
fn folded(word: &str) -> String {
    use caseless::Caseless;
    use unicode_normalization::UnicodeNormalization;
    use unicode_normalization::char::is_combining_mark;
    word.chars()
        .nfd()
        .default_case_fold()
        .nfd()
        .filter(|c| c.is_alphanumeric() && !is_combining_mark(*c))
        .collect()
}

/// Whether a word is holding an amount to a Food rather than being part of
/// either: an article, a preposition, a size, or an elided article left
/// carrying its apostrophe by [`split_elisions`]. That last test is by the
/// apostrophe and not by the letter, because the fold in [`folded`] throws
/// punctuation away and `l'` and `l` would otherwise be one word — one of
/// which is a litre.
fn is_glue(word: &str) -> bool {
    word.ends_with('\'') || word.ends_with('\u{2019}') || listed(GLUE, word) || listed(SIZES, word)
}

fn listed(list: &[&str], word: &str) -> bool {
    let word = folded(word);
    !word.is_empty() && list.iter().any(|entry| folded(entry) == word)
}

/// **Read one Ingredient Line**, or answer nothing where there is nothing
/// honest to say.
///
/// Nothing is an ordinary answer and never an error (ADR 0002). It is what
/// `salt and pepper to taste` gets, and what any line whose words Kamosu
/// cannot place gets, and the line goes on working exactly as written.
pub fn read_line(line: &str) -> Option<Reading> {
    let head = strip_the_cooks_aside(line);
    let head = split_elisions(&head);
    let tokens: Vec<&str> = head.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }

    let mut reading = Reading::default();

    // The amount: the longest leading run of words Kamosu can read as one
    // quantity, so `1 1/2` beats `1` and `2` beats nothing at all.
    let mut rest = tokens.as_slice();
    for take in (1..=tokens.len().min(3)).rev() {
        let candidate = tokens[..take].join(" ");
        if units::parse_amount(&candidate).is_some() {
            reading.amount = Some(candidate);
            rest = &tokens[take..];
            break;
        }
    }

    // Glue and size words sit wherever they like — before the Unit as much as
    // after it — and belong to neither the Unit nor the Food. Stripped on both
    // sides of the Unit so `a pinch of salt` reads exactly as `pinch of salt`
    // does, which is the whole point: an article is not a measurement.
    fn strip_glue<'a>(mut words: &'a [&'a str]) -> &'a [&'a str] {
        while words.first().is_some_and(|word| is_glue(word)) {
            words = &words[1..];
        }
        words
    }
    rest = strip_glue(rest);

    // The Unit: the longest window the closed set recognises — `fl oz` and
    // `c. à c.` are two and three words — and failing that one word from the
    // open set.
    //
    // **A Unit is only a Unit while something is left for it to measure.**
    // `pinch of salt` is a pinch of salt; a line reading `cloves` alone is
    // naming the spice, and reading it as a Unit of nothing would put an
    // empty measure on a perfectly good line.
    let mut unit_taken = 0;
    for take in (1..=rest.len().saturating_sub(1).min(3)).rev() {
        let candidate = rest[..take].join(" ");
        if units::recognise(&candidate).is_some() || (take == 1 && listed(OPEN_UNITS, rest[0])) {
            reading.unit = Some(candidate);
            unit_taken = take;
            break;
        }
    }
    reading.target = rejoin(strip_glue(&rest[unit_taken..]));
    reading.is_something().then_some(reading)
}

/// Everything after the first comma, and anything in brackets, is what the
/// cook said *about* this line — how to cut it, whether to skip it, what it
/// weighed in the shop. It is part of the written line and never part of the
/// Reading, so it is dropped here and nowhere else.
fn strip_the_cooks_aside(line: &str) -> String {
    let mut kept = String::with_capacity(line.len());
    let mut depth = 0i32;
    for character in line.chars() {
        match character {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = (depth - 1).max(0),
            _ if depth == 0 => kept.push(character),
            _ => {}
        }
    }
    kept.split(',')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// Put a space after an elided article so it is a word the lists can see.
/// [`rejoin`] takes it back out again.
///
/// **The word itself is never rewritten**, only cut. A cook writing `Za’tar`
/// with a typographic apostrophe gets a Food named `Za’tar`: the two
/// apostrophes are one character for the purpose of spotting `d'`, and are
/// still two characters in what gets stored, because a Reading lays over a
/// line and never edits it (ADR 0002).
fn split_elisions(head: &str) -> String {
    let mut out = String::with_capacity(head.len() + 8);
    for word in head.split_whitespace() {
        let lowered = word.to_lowercase().replace('\u{2019}', "'");
        // Cut by characters and never by bytes: `’` is three bytes where `'`
        // is one, so a byte offset taken from the folded copy would land
        // inside the apostrophe of the original and panic.
        let cut = ELISIONS
            .iter()
            .find(|elision| lowered.starts_with(**elision))
            .map(|elision| elision.chars().count())
            .and_then(|chars| word.char_indices().nth(chars).map(|(at, _)| at));
        match cut {
            Some(at) => {
                let (article, remainder) = word.split_at(at);
                out.push_str(article);
                out.push(' ');
                out.push_str(remainder);
            }
            None => out.push_str(word),
        }
        out.push(' ');
    }
    out
}

/// **A Food is one thing to buy, and a sentence is not one.** Past this many
/// words Kamosu is looking at prose, not at a food — the real corpus keeps
/// whole cooking steps and a Q-and-A about substituting wine inside ingredient
/// entries, and a Food named after one of those is a Food nothing will ever
/// match.
///
/// Six is where the corpus separates: *Kosher salt and freshly ground black
/// pepper* is six words and a real thing to buy, and everything longer in 863
/// real lines is a paragraph.
const LONGEST_FOOD_NAME: usize = 6;

/// The Food's name, put back together: elisions re-glued to their own word,
/// nothing left if what remains is punctuation, and nothing left if what
/// remains is prose. Refusing to name a Food is ADR 0002's "shown whole rather
/// than guessed at" — the line still reads exactly as written.
fn rejoin(words: &[&str]) -> Option<String> {
    if words.len() > LONGEST_FOOD_NAME {
        return None;
    }
    let name = words
        .join(" ")
        .replace("' ", "'")
        .replace("\u{2019} ", "\u{2019}")
        .trim()
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_string();
    (!name.is_empty() && name.chars().any(char::is_alphanumeric)).then_some(name)
}
