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
    "cube",
    "cubes",
    "bundle",
    "bundles",
    // French. `cube` and `cubes` are spelt the same, so the English pair above
    // covers both (#161).
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
    "pastilla",
    "pastillas",
    "cubito",
    "cubitos",
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
    let line = without_brackets(line);
    let mut clauses = line.split(',');
    let head = split_elisions(clauses.next().unwrap_or_default());
    let tokens: Vec<&str> = head.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    // What follows the first comma is ordinarily the cook's aside, never part
    // of the Reading: in `garlic, minced` the cook is talking *about* the
    // garlic. It is kept here only for the one case below where the comma
    // fell between the measure and the thing measured.
    let past_the_comma = clauses.next().map(split_elisions);

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

    rest = strip_glue(rest);

    // The Unit: the longest window the closed set recognises — `fl oz` and
    // `c. à c.` are two and three words — and failing that one word from the
    // open set.
    //
    // **A Unit is only a Unit while something is left for it to measure.**
    // `pinch of salt` is a pinch of salt; a line reading `cloves` alone is
    // naming the spice, and reading it as a Unit of nothing would put an
    // empty measure on a perfectly good line. A comma straight after it ends
    // the measure as surely as a word does: `1 cup, panko bread crumbs` is a
    // cup of what follows (#160).
    let longest = if past_the_comma.is_some() {
        rest.len()
    } else {
        rest.len().saturating_sub(1)
    };
    let mut unit_taken = 0;
    for take in (1..=longest.min(3)).rev() {
        let candidate = rest[..take].join(" ");
        if units::recognise(&candidate).is_some() || (take == 1 && listed(OPEN_UNITS, rest[0])) {
            reading.unit = Some(candidate);
            unit_taken = take;
            break;
        }
    }

    // The Food is what the measure leaves. Only where it leaves nothing, and
    // something *was* measured, is the Food looked for past the comma — the
    // aside after *that* still dropped, so `4 , hamburger buns, toasted if
    // desired` is hamburger buns. Where there is nothing there either, the
    // Reading names no Food rather than promoting the Unit into one
    // (ADR 0002).
    let named = strip_glue(&rest[unit_taken..]);
    reading.target = if !named.is_empty() {
        rejoin(named)
    } else if reading.amount.is_some() || reading.unit.is_some() {
        let words: Vec<&str> = past_the_comma
            .as_deref()
            .unwrap_or_default()
            .split_whitespace()
            .collect();
        rejoin(strip_glue(&words))
    } else {
        None
    };
    reading.is_something().then_some(reading)
}

/// Whether a word is a measure: a Unit the closed set recognises, or one of
/// the [`OPEN_UNITS`]. A Food named by one of these is almost always a Unit
/// left standing where the Food should be, the fault #160 fixed, and the
/// corpus test asks this of every Reading so it cannot come back unseen.
///
/// *Almost* always: a line that is only `cloves` names the spice and reads
/// that way on purpose. The corpus holds no such line, so the test can afford
/// the stricter question; a library that does hold one should not.
pub fn is_a_unit_word(word: &str) -> bool {
    units::recognise(word).is_some() || listed(OPEN_UNITS, word)
}

/// Glue and size words sit wherever they like — before the Unit as much as
/// after it — and belong to neither the Unit nor the Food. Stripped on both
/// sides of the Unit so `a pinch of salt` reads exactly as `pinch of salt`
/// does, which is the whole point: an article is not a measurement.
fn strip_glue<'a>(mut words: &'a [&'a str]) -> &'a [&'a str] {
    while words.first().is_some_and(|word| is_glue(word)) {
        words = &words[1..];
    }
    words
}

/// Anything in brackets is what the cook said *about* this line — what it
/// weighed in the shop, whether to skip it. It is part of the written line
/// and never part of the Reading. What follows a comma is the same kind of
/// thing, and [`read_line`] drops it once it has found the Food.
fn without_brackets(line: &str) -> String {
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
    kept
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

/// One amount written inside a Step's text: `1 lb.` in *Place 1 lb. ground
/// chicken in the center* (#150).
///
/// `start..end` is where the amount and its Unit sit in the Step's text, in
/// bytes, so the addition can be placed straight after them. `food` is what
/// the words after the Unit name, glue stripped — what joins this amount to an
/// Ingredient Line, the same way a Step's `uses` is joined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepAmount {
    pub start: usize,
    pub end: usize,
    pub amount: String,
    pub unit: String,
    pub food: Option<String>,
}

impl StepAmount {
    /// The amount and Unit exactly as the Step wrote them.
    pub fn written<'a>(&self, text: &'a str) -> &'a str {
        &text[self.start..self.end]
    }
}

/// **Every amount a Step writes that Kamosu can convert**, in the order
/// written (#150).
///
/// The split is [`read_line`]'s, run at every number in a sentence rather than
/// once at the start of a line: the amount is what [`units::parse_amount`]
/// reads, the Unit is what [`units::recognise`] knows. No second list of Units.
///
/// **Only the closed set counts.** An Ingredient Line may carry *2 cloves* or
/// a bare *2 eggs*, because the line is an amount of something by definition.
/// A sentence is not: *2 minutes*, *step 3* and *28–35 minutes* are numbers
/// too, and the one test that tells an amount from any other number in prose
/// is a Unit Kamosu can convert standing right after it. A range, `1–1½
/// cups`, reads as nothing, exactly as it does on an Ingredient Line.
pub fn amounts_in_step(text: &str) -> Vec<StepAmount> {
    let words = words_with_places(text);
    let mut found = Vec::new();
    let mut index = 0;
    while index < words.len() {
        match amount_at(text, &words, index) {
            Some((amount, taken)) => {
                index += taken;
                found.push(amount);
            }
            None => index += 1,
        }
    }
    found
}

/// Each whitespace-separated word of `text`, with where it starts and ends.
fn words_with_places(text: &str) -> Vec<(usize, usize)> {
    let mut words = Vec::new();
    let mut start = None;
    for (at, character) in text.char_indices() {
        match (character.is_whitespace(), start) {
            (true, Some(from)) => {
                words.push((from, at));
                start = None;
            }
            (false, None) => start = Some(at),
            _ => {}
        }
    }
    if let Some(from) = start {
        words.push((from, text.len()));
    }
    words
}

/// The amount starting at word `index`, and how many words it took.
fn amount_at(text: &str, words: &[(usize, usize)], index: usize) -> Option<(StepAmount, usize)> {
    let word = |at: usize| words.get(at).map(|(from, to)| &text[*from..*to]);
    // An opening bracket belongs to the sentence, not to the amount:
    // `(½ cup)` is an amount of half a cup.
    let first = word(index)?;
    let opened = first.len() - first.trim_start_matches(['(', '[']).len();

    // The amount: `1 1/2` before `1`, as on an Ingredient Line.
    let (amount, amount_words) = [2, 1].into_iter().find_map(|take| {
        let last = index + take - 1;
        let candidate = &text[words[index].0 + opened..words.get(last)?.1];
        units::parse_amount(candidate).map(|_| (candidate.to_string(), take))
    })?;

    // The Unit: the longest window the closed set recognises, `c. à s.` being
    // three words. Only its last word can carry the sentence's punctuation.
    let unit_from = index + amount_words;
    let (unit_end, unit_words) = (1..=3).rev().find_map(|take| {
        let last = unit_from + take - 1;
        let (from, to) = (words.get(unit_from)?.0, words.get(last)?.1);
        let end = from
            + text[from..to]
                .trim_end_matches(|c: char| !c.is_alphanumeric() && c != '.')
                .len();
        units::recognise(&text[from..end]).map(|_| (end, take))
    })?;
    let unit_start = words[unit_from].0;
    let end = full_stop_left_out(text, unit_start, unit_end);

    let after = unit_from + unit_words;
    let food = food_after(&text[end..]);
    Some((
        StepAmount {
            start: words[index].0 + opened,
            end,
            amount,
            unit: text[unit_start..end].to_string(),
            food,
        },
        after - index,
    ))
}

/// Whether the dot after a Unit is the Unit's own or the sentence's. The
/// addition goes after the Unit, so `add 2 cups. Stir` must answer `2 cups`
/// and `3 oz. Parmesan` must answer `3 oz.`.
///
/// Where the sentence plainly goes on, the dot is the Unit's. Otherwise the
/// word's shape decides, which keeps this from being a second list of Units:
/// an abbreviation is two letters or fewer, or has no vowel (`oz`, `lb`,
/// `Tbsp`, `tsp`), and a written-out word has both (`cup`, `cups`, `tasse`).
fn full_stop_left_out(text: &str, start: usize, end: usize) -> usize {
    let Some(word) = text[start..end].strip_suffix('.') else {
        return end;
    };
    let goes_on = text[end..]
        .trim_start()
        .chars()
        .next()
        .is_some_and(|c| c.is_lowercase() || c.is_ascii_digit() || c == '(');
    let last = word.rsplit(' ').next().unwrap_or(word);
    let letters: Vec<char> = last.chars().filter(|c| c.is_alphabetic()).collect();
    let abbreviated = letters.len() <= 2
        || !letters
            .iter()
            .any(|c| "aeiouyàâéèêëîïôûùü".contains(c.to_ascii_lowercase()));
    if goes_on || abbreviated { end } else { end - 1 }
}

/// The words that end what an amount in a Step is an amount of: *1 cup of
/// water **to** the flour* is water, and without this the words would run on
/// to the flour and weigh the water by its Cup Weight. Prepositions and
/// conjunctions in the three interface Languages; `or` is not one, because
/// *homemade or store-bought marinara sauce* is one Food.
const FOOD_ENDS: &[&str] = &[
    // English
    "to", "into", "onto", "over", "with", "in", "on", "for", "from", "and", "then", "until",
    // French
    "dans", "sur", "avec", "pour", "et", "puis", "jusqu'à", "a", "au", "aux", "en",
    // Spanish
    "con", "sobre", "para", "y", "luego", "hasta",
];

/// What the words after a Unit name, glue stripped, up to the end of the
/// clause or the first word in [`FOOD_ENDS`]: `ground chicken` from *1 lb.
/// ground chicken in the center*. Nothing where nothing is named, as in
/// `(½ cup)`.
fn food_after(rest: &str) -> Option<String> {
    let clause = rest
        .split([',', ';', ':', '(', ')', '.', '!', '?'])
        .next()
        .unwrap_or_default();
    let clause = split_elisions(clause);
    let words: Vec<&str> = clause.split_whitespace().collect();
    let mut from = 0;
    while words.get(from).is_some_and(|word| is_glue(word)) {
        from += 1;
    }
    let until = words[from..]
        .iter()
        .position(|word| listed(FOOD_ENDS, word))
        .map_or(words.len(), |at| from + at);
    let name = words[from..until]
        .join(" ")
        .replace("' ", "'")
        .replace("\u{2019} ", "\u{2019}");
    (!name.is_empty()).then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line's Reading as its three parts, `None` where a part is absent.
    fn read(line: &str) -> Option<(Option<String>, Option<String>, Option<String>)> {
        read_line(line).map(|reading| (reading.amount, reading.unit, reading.target))
    }

    fn parts(
        amount: Option<&str>,
        unit: Option<&str>,
        target: Option<&str>,
    ) -> Option<(Option<String>, Option<String>, Option<String>)> {
        Some((
            amount.map(str::to_string),
            unit.map(str::to_string),
            target.map(str::to_string),
        ))
    }

    #[test]
    fn a_comma_between_the_measure_and_the_food_is_read_past() {
        // The Air-Fryer Spicy Fried-Chicken Sandwich, as Crouton filed it (#160).
        assert_eq!(
            read("1 cup, panko bread crumbs"),
            parts(Some("1"), Some("cup"), Some("panko bread crumbs"))
        );
        assert_eq!(
            read("2 tablespoons, extra-virgin olive oil"),
            parts(
                Some("2"),
                Some("tablespoons"),
                Some("extra-virgin olive oil")
            )
        );
        assert_eq!(
            read("½ teaspoon, garlic powder"),
            parts(Some("½"), Some("teaspoon"), Some("garlic powder"))
        );
        assert_eq!(
            read("4 , hamburger buns, toasted if desired"),
            parts(Some("4"), None, Some("hamburger buns"))
        );
        // A size is dropped past the comma exactly as it is without one, so
        // this is the same Food as `1 large egg`.
        assert_eq!(read("1 , large egg"), read("1 large egg"));
        assert_eq!(read("1 , large egg"), parts(Some("1"), None, Some("egg")));
        // No amount, but a Unit with something past the comma to measure.
        assert_eq!(
            read("a handful , halved cherry tomatoes"),
            parts(None, Some("handful"), Some("halved cherry tomatoes"))
        );
        assert_eq!(
            read("cloves, roughly chopped garlic"),
            parts(None, Some("cloves"), Some("roughly chopped garlic"))
        );
    }

    #[test]
    fn a_stock_cube_or_a_bundle_is_counted_not_named() {
        // Both Beef Bourguignons, doubled word and all (#161).
        assert_eq!(
            read("3 cubes beef broth cubes"),
            parts(Some("3"), Some("cubes"), Some("beef broth cubes"))
        );
        assert_eq!(
            read("1 cube de bouillon"),
            parts(Some("1"), Some("cube"), Some("bouillon"))
        );
        assert_eq!(
            read("2 pastillas de caldo"),
            parts(Some("2"), Some("pastillas"), Some("caldo"))
        );
        assert_eq!(
            read("1 cubito de caldo de pollo"),
            parts(Some("1"), Some("cubito"), Some("caldo de pollo"))
        );
        // The Dan Dan Noodles, typed in by hand rather than imported.
        assert_eq!(
            read("2 bundles fresh wheat noodles"),
            parts(Some("2"), Some("bundles"), Some("fresh wheat noodles"))
        );
        // Still the Food it names when there is nothing to count.
        assert_eq!(read("cubes"), parts(None, None, Some("cubes")));
    }

    #[test]
    fn the_short_french_spoons_are_spoons() {
        // Seen translating recipes into French (#161).
        for (line, unit) in [
            ("1 c. à café de cassonade", "c. à café"),
            ("1 c. a cafe de cassonade", "c. a cafe"),
            ("1 cuil. à café de cassonade", "cuil. à café"),
            ("1 cuill. à café de cassonade", "cuill. à café"),
        ] {
            assert_eq!(read(line), parts(Some("1"), Some(unit), Some("cassonade")));
            assert_eq!(units::recognise(unit).map(|u| u.id), Some("teaspoon"));
        }
        for (line, unit) in [
            ("2 c. à soupe d'huile", "c. à soupe"),
            ("2 cuil. à soupe d'huile", "cuil. à soupe"),
            ("2 cuill. a soupe d'huile", "cuill. a soupe"),
        ] {
            assert_eq!(read(line), parts(Some("2"), Some(unit), Some("huile")));
            assert_eq!(units::recognise(unit).map(|u| u.id), Some("tablespoon"));
        }
    }

    #[test]
    fn what_follows_the_food_is_still_the_cooks_aside() {
        assert_eq!(read("garlic, minced"), parts(None, None, Some("garlic")));
        assert_eq!(
            read("1 tbsp olive oil, plus more for drizzling"),
            parts(Some("1"), Some("tbsp"), Some("olive oil"))
        );
        assert_eq!(
            read("2 onions (about 300 g), finely chopped"),
            parts(Some("2"), None, Some("onions"))
        );
        // Nothing before the comma measures anything, so nothing after it is
        // read as the thing measured.
        assert_eq!(read(", Salt and pepper"), None);
        // A Unit with nothing at all to measure is still the Food it names.
        assert_eq!(read("cloves"), parts(None, None, Some("cloves")));
    }

    #[test]
    fn a_unit_is_never_left_standing_as_the_food() {
        // Where the comma leaves nothing to name, the Reading names nothing
        // rather than promoting the Unit into a Food (ADR 0002).
        assert_eq!(read("1 cup,"), parts(Some("1"), Some("cup"), None));
        assert_eq!(
            read("2 tbsp, (to taste)"),
            parts(Some("2"), Some("tbsp"), None)
        );
    }

    fn written(text: &str) -> Vec<(&str, Option<String>)> {
        amounts_in_step(text)
            .into_iter()
            .map(|amount| (amount.written(text), amount.food.clone()))
            .collect()
    }

    #[test]
    fn a_step_gives_up_every_amount_with_a_unit_kamosu_converts() {
        // The chicken parm's second step (#150).
        let text = "Whisk 2 large eggs in a small bowl with a fork to combine. Mix together 3 oz. Parmesan, finely grated (½ cup), 1 cup panko, a pinch of kosher salt, and a pinch of freshly ground pepper in another small bowl.";
        assert_eq!(
            written(text),
            vec![
                ("3 oz.", Some("Parmesan".into())),
                ("½ cup", None),
                ("1 cup", Some("panko".into())),
            ]
        );
        let first = &amounts_in_step(text)[0];
        assert_eq!((first.amount.as_str(), first.unit.as_str()), ("3", "oz."));
    }

    #[test]
    fn a_number_without_a_convertible_unit_is_not_an_amount() {
        let text = "Place 1 lb. ground chicken on it, leaving 1\"–2\" border. Bake 28–35 minutes, then 2 more minutes; scatter 8 oz. mozzarella, coarsely grated (1–1½ cups), on top at 425°.";
        assert_eq!(
            written(text),
            vec![
                ("1 lb.", Some("ground chicken".into())),
                ("8 oz.", Some("mozzarella".into())),
            ]
        );
    }

    #[test]
    fn what_an_amount_measures_ends_where_the_sentence_moves_on() {
        // Without the stop, this cup would run on to the flour and be weighed
        // as flour.
        assert_eq!(
            written("Add 1 cup of water to the flour."),
            vec![("1 cup", Some("water".into()))]
        );
        assert_eq!(
            written("Spread 1½ cups homemade or store-bought marinara sauce over."),
            vec![(
                "1½ cups",
                Some("homemade or store-bought marinara sauce".into())
            )]
        );
    }

    #[test]
    fn a_full_stop_is_not_part_of_the_unit() {
        assert_eq!(written("Add 2 cups. Stir well."), vec![("2 cups", None)]);
        assert_eq!(written("Add 1 cup."), vec![("1 cup", None)]);
        // An abbreviation keeps its dot even at a sentence's end, since
        // `3 oz. Parmesan` looks exactly like it. What it then names, `Stir`,
        // is no Ingredient Line, so it joins to nothing.
        assert_eq!(written("Add 1 lb. Stir.")[0].0, "1 lb.");
        assert_eq!(
            written("Scatter 2 Tbsp. all-purpose flour over."),
            vec![("2 Tbsp.", Some("all-purpose flour".into()))]
        );
    }

    #[test]
    fn french_amounts_are_read_with_their_own_units_and_glue() {
        assert_eq!(
            written("Ajouter 1 c. à s. d'huile puis 200 g de farine et 1 1/2 cup de lait."),
            vec![
                ("1 c. à s.", Some("huile".into())),
                ("200 g", Some("farine".into())),
                ("1 1/2 cup", Some("lait".into())),
            ]
        );
    }
}
