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
    // What a shop sells a thing in, seen translating recipes into French
    // (#181). `filet` above stays a Unit too: `un filet d'huile d'olive` is a
    // drizzle, and `2 filets de saumon` is two fillets of salmon.
    "bloc",
    "blocs",
    "bouteille",
    "bouteilles",
    "barquette",
    "barquettes",
    "brique",
    "briques",
    "tablette",
    "tablettes",
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
    "botella",
    "botellas",
    "tableta",
    "tabletas",
];

/// Words that stand exactly where a Unit stands and are not one: they describe
/// the thing rather than measure it. Dropped so `1 large onion` and `2 onions`
/// point at one Food rather than two.
///
/// Every form a French or Spanish size takes is listed, plurals included,
/// because the fold in [`folded`] strips accents but not endings: without
/// `petits`, `2 petits poireaux` named a Food *petits poireaux* (#185).
const SIZES: &[&str] = &[
    "large", "small", "medium", "big", "whole", "extra", "jumbo", "gros", "grosse", "grosses",
    "petit", "petite", "petits", "petites", "moyen", "moyenne", "moyens", "moyennes", "grand",
    "grande", "grands", "grandes", "pequeno", "pequena", "pequenos", "pequenas", "mediano",
    "mediana", "medianos", "medianas", "entier", "entiere", "entiers", "entieres",
];

/// The [`SIZES`] French and Spanish write after the food rather than before
/// it, `2 tomates moyennes` and `1 cebolla grande`, dropped from the end of
/// the Food's name as the others are from its start (#185).
///
/// **`entier` is not one of them.** After the food it says *whole* the way a
/// product does: `farine de blé entier` is whole-wheat flour, and `lait
/// entier` whole milk, neither the same thing to buy as plain flour or milk.
/// Nor is `gros`, for the same reason: `sel gros` is coarse salt.
/// English `whole` before the food is still dropped as a size, as it was
/// before #185, so `whole milk` reads as *milk*; that was left alone there.
///
/// Kept as its own list rather than derived from [`SIZES`]: a new French or
/// Spanish size goes into both.
const SIZES_AFTER: &[&str] = &[
    "petit", "petite", "petits", "petites", "moyen", "moyenne", "moyens", "moyennes", "grand",
    "grande", "grands", "grandes", "pequeno", "pequena", "pequenos", "pequenas", "mediano",
    "mediana", "medianos", "medianas",
];

/// The ways a recipe says how warm a food should be when it goes in, each
/// written as its words. How warm the butter is says nothing about which
/// butter to buy, so a phrase at either end of a Food's name is dropped:
/// `100 g de beurre froid` is a Reading of *beurre* (#185).
///
/// **`hot` and `glacé` are left out on purpose.** Hot sauce and a hot dog are
/// things to buy, and the fold in [`folded`] makes `glacé` one word with the
/// `glace` of *sucre glace*, which is icing sugar. `chaud` is in, although
/// *chocolat chaud* is a drink: the question Aurélien answered on #185 named
/// `chaud` among the words to drop, and an Ingredient Line far more often
/// means warm milk than hot chocolate.
///
/// A word that only says *very* goes with the temperature it strengthens, so
/// `eau très chaude` is *eau* and not *eau très* ([`VERY`]).
///
/// Only temperature, and never strength: `fond de volaille corsé` and
/// `moutarde forte` keep their names, because a strong mustard is not the
/// mustard beside it on the shelf.
const WARMTH: &[&[&str]] = &[
    // English
    &["cold"],
    &["warm"],
    &["lukewarm"],
    &["room-temperature"],
    &["room", "temperature"],
    &["at", "room", "temperature"],
    // French
    &["froid"],
    &["froide"],
    &["froids"],
    &["froides"],
    &["chaud"],
    &["chaude"],
    &["chauds"],
    &["chaudes"],
    &["tiède"],
    &["tièdes"],
    &["température", "ambiante"],
    &["à", "température", "ambiante"],
    &["à", "la", "température", "ambiante"],
    // Spanish
    &["frío"],
    &["fría"],
    &["fríos"],
    &["frías"],
    &["caliente"],
    &["calientes"],
    &["tibio"],
    &["tibia"],
    &["tibios"],
    &["tibias"],
    &["templado"],
    &["templada"],
    &["templados"],
    &["templadas"],
    &["temperatura", "ambiente"],
    &["a", "temperatura", "ambiente"],
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

/// Words that open a line to say *a few* or *about*. Kamosu can do no
/// arithmetic on *a few*, so one is never the amount; left standing it hid the
/// Unit behind it, and `quelques feuilles de menthe` named a Food after the
/// leaves (#181).
///
/// Dropped only at the start of the line and **before** the amount is read,
/// never as [`GLUE`]: `unos 200 g de harina` is Spanish for *about* 200 g, and
/// the 200 has to reach the amount rather than the Food.
const A_FEW: &[&str] = &["quelques", "unos", "unas"];

/// The ways a recipe says *as much as you like*, each written as its words.
/// Such a phrase measures nothing and names nothing, so it is dropped from the
/// front or the end of what a Reading reads (#163): `to taste sea salt` and
/// `salt and pepper to taste` read exactly as `sea salt` and `salt and pepper`
/// do. Inside brackets or after a comma it was already the cook's aside.
///
/// Only the two ends, because in the middle a phrase can mean something else:
/// `huile au goût de truffe` is truffle-flavoured oil, one thing to buy.
const TO_TASTE: &[&[&str]] = &[
    &["to", "taste"],
    &["to", "your", "taste"],
    &["au", "goût"],
    &["selon", "le", "goût"],
    &["selon", "votre", "goût"],
    &["selon", "goût"],
    &["à", "volonté"],
    &["al", "gusto"],
    &["a", "gusto"],
];

/// Words that say how a food is prepared or bought, never what it is: a Food
/// made of nothing else is not a Food (#163). `4 cloves, minced` measures
/// garlic that the line never names, and a Food called *minced* would sit on
/// the shopping list meaning nothing.
///
/// Beside a food they stay part of its name, as they always have — `minced
/// garlic` and `boneless chicken thighs` are what the cook wrote. Only a name
/// that is **all** of these is refused, and where a comma cut such a run off
/// from its food, as in `boneless, skinless chicken thighs`, the Food is read
/// on past the comma instead.
///
/// Every form a French or Spanish word takes is listed, because the fold in
/// [`folded`] strips accents but not endings. A word that is also a food is
/// left out, whatever else it means: *fondue*, *cocido*, *confit*, *cortado*
/// the coffee and *picada* the Catalan sauce.
const DESCRIBING: &[&str] = &[
    // English
    "boneless",
    "skinless",
    "bone-in",
    "skin-on",
    "minced",
    "divided",
    "chopped",
    "diced",
    "sliced",
    "grated",
    "shredded",
    "crushed",
    "peeled",
    "seeded",
    "deseeded",
    "pitted",
    "cubed",
    "halved",
    "quartered",
    "trimmed",
    "melted",
    "softened",
    "beaten",
    "sifted",
    "drained",
    "rinsed",
    "cooked",
    "uncooked",
    "toasted",
    "julienned",
    "zested",
    "juiced",
    "mashed",
    "packed",
    "heaping",
    "heaped",
    "level",
    "rounded",
    "optional",
    "fresh",
    "dried",
    "frozen",
    "ground",
    "raw",
    "finely",
    "roughly",
    "coarsely",
    "thinly",
    "freshly",
    "lightly",
    // French
    "haché",
    "hachée",
    "hachés",
    "hachées",
    "émincé",
    "émincée",
    "émincés",
    "émincées",
    "désossé",
    "désossée",
    "désossés",
    "désossées",
    "coupé",
    "coupée",
    "coupés",
    "coupées",
    "râpé",
    "râpée",
    "râpés",
    "râpées",
    "pelé",
    "pelée",
    "pelés",
    "pelées",
    "écrasé",
    "écrasée",
    "écrasés",
    "écrasées",
    "concassé",
    "concassée",
    "concassés",
    "concassées",
    "cuit",
    "cuite",
    "cuits",
    "cuites",
    "surgelé",
    "surgelée",
    "surgelés",
    "surgelées",
    "frais",
    "fraîche",
    "fraîches",
    "sec",
    "sèche",
    "secs",
    "sèches",
    "finement",
    "grossièrement",
    // Spanish
    "picado",
    "picados",
    "deshuesado",
    "deshuesada",
    "deshuesados",
    "deshuesadas",
    "rallado",
    "rallada",
    "rallados",
    "ralladas",
    "pelado",
    "pelada",
    "pelados",
    "peladas",
    "troceado",
    "troceada",
    "troceados",
    "troceadas",
    "molido",
    "molida",
    "molidos",
    "molidas",
    "fresco",
    "fresca",
    "frescos",
    "frescas",
    "seco",
    "seca",
    "secos",
    "secas",
    "finamente",
];

/// What may join two [`DESCRIBING`] words without naming anything itself:
/// `peeled and chopped`.
const DESCRIBING_JOINS: &[&str] = &["and", "et", "y"];

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
/// `to taste` alone gets, and what any line whose words Kamosu cannot place
/// gets, and the line goes on working exactly as written. `salt and pepper to
/// taste` is not such a line: the phrase is dropped and it reads as `salt and
/// pepper` does (#163).
pub fn read_line(line: &str) -> Option<Reading> {
    let line = without_brackets(line);
    // The slashes are stood apart only to find a restated measure. Where
    // there is none, the line is read as written, so `salt/2 tsp pepper`
    // names the Food the cook spelt and not a respaced copy of it (#182).
    match read_split(&slashes_apart(&line)) {
        (reading, true) => reading,
        _ => read_split(&line).0,
    }
}

/// [`read_line`]'s work on a line with its brackets gone, and whether a
/// measure restated after a slash was left out of the Food.
fn read_split(line: &str) -> (Option<Reading>, bool) {
    let mut clauses = split_clauses(line).into_iter();
    let head = split_elisions(clauses.next().unwrap_or_default());
    let mut tokens: Vec<&str> = head.split_whitespace().collect();
    let a_few = tokens.iter().take_while(|word| listed(A_FEW, word)).count();
    tokens.drain(..a_few);
    // `to taste` goes before anything else is read, so a line reads exactly
    // as it would without the phrase: `cloves to taste` as `cloves` (#163).
    let to_taste = to_taste_at(&tokens, End::Start);
    tokens.drain(..to_taste);
    let to_taste = to_taste_at(&tokens, End::Finish);
    tokens.truncate(tokens.len() - to_taste);
    if tokens.is_empty() {
        return (None, false);
    }
    // What follows the first comma is ordinarily the cook's aside, never part
    // of the Reading: in `garlic, minced` the cook is talking *about* the
    // garlic. It is kept here only for the cases below where the comma fell
    // between the measure and the thing measured, or inside the thing's name.
    let later: Vec<String> = clauses.map(split_elisions).collect();
    let past_the_comma = later.first();

    let mut reading = Reading::default();

    // The amount: the longest leading run of words Kamosu can read as one
    // quantity, so `1 1/2` beats `1` and `2` beats nothing at all.
    let mut rest = tokens.as_slice();
    if let Some(take) = amount_words(&tokens) {
        reading.amount = Some(tokens[..take].join(" "));
        rest = &tokens[take..];
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

    // The Food is what the measure leaves. Where it leaves nothing, and
    // something *was* measured, the Food is looked for past the comma — the
    // aside after *that* still dropped, so `4 , hamburger buns, toasted if
    // desired` is hamburger buns. Where it leaves only describing words, the
    // comma cut the Food's own name, and the name is read on past it (#163).
    // Where there is nothing there either, the Reading names no Food rather
    // than promoting the Unit or a describing word into one (ADR 0002).
    let mut named = &rest[unit_taken..];
    let mut restating = false;
    if reading.amount.is_some() && reading.unit.is_some() {
        while let Some(taken) = restated(named) {
            named = &named[taken..];
            restating = true;
        }
    }
    let food = without_warmth_or_size(strip_glue(named));
    let measured = reading.amount.is_some() || reading.unit.is_some();
    reading.target = if offers_a_choice(named) {
        None
    } else if !food.is_empty() && !only_describing(food) {
        rejoin(food)
    } else if !food.is_empty() || measured {
        food_past_the_comma(food, &later)
    } else {
        None
    };
    (reading.is_something().then_some(reading), restating)
}

/// The Food's name carried on past a comma: the describing words before it
/// (`boneless`), then each clause in turn while it too only describes
/// (`skinless`), until one names something (`chicken thighs`). The commas the
/// cook wrote stay in the name.
///
/// **Once describing words are carried, the next clause must open with one.**
/// That is what says the run goes on: `boneless, skinless chicken thighs` is
/// one name, and `boneless, cut into 1-inch pieces` is a describing word
/// followed by the cook's aside, which names nothing. With nothing carried,
/// a measure straight before the comma, the first clause is the Food, as
/// #160 made it. A choice, an empty clause, or running out of clauses before
/// anything is named leaves the Reading naming no Food.
fn food_past_the_comma(describing: &[&str], later: &[String]) -> Option<String> {
    let mut name: Vec<String> = describing.iter().map(|word| word.to_string()).collect();
    for clause in later {
        let words: Vec<&str> = clause.split_whitespace().collect();
        if offers_a_choice(&words) {
            return None;
        }
        let words = without_warmth_or_size(strip_glue(without_to_taste(&words)));
        let carries_on = words.first().is_some_and(|word| listed(DESCRIBING, word));
        if words.is_empty() || !(name.is_empty() || carries_on) {
            return None;
        }
        if let Some(last) = name.last_mut() {
            last.push(',');
        }
        name.extend(words.iter().map(|word| word.to_string()));
        if !only_describing(words) {
            let name: Vec<&str> = name.iter().map(String::as_str).collect();
            return rejoin(&name);
        }
    }
    None
}

/// Whether every word says how a food is prepared or bought and none says what
/// it is: [`DESCRIBING`] words, joined by [`DESCRIBING_JOINS`] at most.
fn only_describing(words: &[&str]) -> bool {
    words.iter().any(|word| listed(DESCRIBING, word))
        && words
            .iter()
            .all(|word| listed(DESCRIBING, word) || listed(DESCRIBING_JOINS, word))
}

/// **Whether a Food is made only of describing words**, `boneless` or
/// `minced`: the fault #163 fixed. The corpus test asks this of every Reading,
/// beside [`is_a_unit_word`], so it cannot come back unseen.
pub fn is_only_describing(target: &str) -> bool {
    let words: Vec<&str> = target
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|word| !word.is_empty())
        .collect();
    only_describing(&words)
}

/// One end of a run of words.
#[derive(Clone, Copy)]
enum End {
    Start,
    Finish,
}

/// How many words at one end are a [`TO_TASTE`] phrase, the longest first.
fn to_taste_at(words: &[&str], end: End) -> usize {
    phrase_at(TO_TASTE, words, end)
}

/// How many words at one end are one of `phrases`, the longest first.
fn phrase_at(phrases: &[&[&str]], words: &[&str], end: End) -> usize {
    phrases
        .iter()
        .filter(|phrase| {
            let Some(rest) = words.len().checked_sub(phrase.len()) else {
                return false;
            };
            let there = match end {
                End::Start => &words[..phrase.len()],
                End::Finish => &words[rest..],
            };
            same_words(phrase, there)
        })
        .map(|phrase| phrase.len())
        .max()
        .unwrap_or(0)
}

/// Whether two runs of words are the same once folded.
fn same_words(expected: &[&str], words: &[&str]) -> bool {
    expected.len() == words.len()
        && expected
            .iter()
            .zip(words)
            .all(|(expected, word)| folded(expected) == folded(word))
}

/// The words with a [`TO_TASTE`] phrase dropped from either end.
fn without_to_taste<'a>(mut words: &'a [&'a str]) -> &'a [&'a str] {
    words = &words[to_taste_at(words, End::Start)..];
    &words[..words.len() - to_taste_at(words, End::Finish)]
}

/// A Food's name without how warm it is ([`WARMTH`], at either end) or a size
/// ([`SIZES`] before it, [`SIZES_AFTER`] after), taken off one at a time until
/// neither end holds one: `oeufs moyens à température ambiante` is *oeufs*,
/// and `cold large eggs` is *eggs*.
fn without_warmth_or_size<'a>(mut words: &'a [&'a str]) -> &'a [&'a str] {
    loop {
        words = strip_glue(words);
        let from_start = if opens_a_fixed_name(words) {
            0
        } else {
            warmth_at(words, End::Start)
        };
        let from_end = match words.last() {
            Some(word) if listed(SIZES_AFTER, word) => 1,
            _ => warmth_at(words, End::Finish),
        };
        if from_start + from_end == 0 {
            return words;
        }
        words = &words[from_start..];
        words = &words[..words.len().saturating_sub(from_end)];
    }
}

/// **Whether a Food still carries a size or how warm it is** at either end of
/// its name: the fault #185 fixed, asked by the corpus test of every Reading
/// beside [`is_only_describing`].
pub fn keeps_warmth_or_size(target: &str) -> bool {
    let words: Vec<&str> = target.split_whitespace().collect();
    without_warmth_or_size(&words).len() < words.len()
}

/// Names that open with a [`SIZES`] or [`WARMTH`] word which is no size or
/// temperature there: *petits pois* are green peas, not small *pois*, and
/// *cold cuts* are not cold *cuts* (#185). Neither end-stripping touches the
/// opening word of one.
const FIXED_NAMES: &[&[&str]] = &[
    &["petit", "pois"],
    &["petits", "pois"],
    &["petit", "suisse"],
    &["petits", "suisses"],
    &["cold", "cuts"],
    &["cold", "brew"],
];

/// The words that strengthen a [`WARMTH`] word and mean nothing without one.
const VERY: &[&str] = &["very", "très", "bien", "muy"];

/// How many words at one end are a [`WARMTH`] phrase, with the [`VERY`] word
/// in front of it where there is one.
fn warmth_at(words: &[&str], end: End) -> usize {
    match end {
        End::Start => {
            let very = usize::from(words.first().is_some_and(|word| listed(VERY, word)));
            match phrase_at(WARMTH, &words[very..], End::Start) {
                0 => 0,
                warmth => very + warmth,
            }
        }
        End::Finish => match phrase_at(WARMTH, words, End::Finish) {
            0 => 0,
            warmth => {
                let very = words
                    .len()
                    .checked_sub(warmth + 1)
                    .is_some_and(|at| listed(VERY, words[at]));
                warmth + usize::from(very)
            }
        },
    }
}

fn opens_a_fixed_name(words: &[&str]) -> bool {
    phrase_at(FIXED_NAMES, words, End::Start) > 0
}

/// **Whether a Food still carries a [`TO_TASTE`] phrase** anywhere in its
/// name, in any of the three Languages: the other fault #163 fixed, asked by
/// the corpus test of every Reading.
pub fn keeps_to_taste(target: &str) -> bool {
    let words: Vec<&str> = target.split_whitespace().collect();
    TO_TASTE.iter().any(|phrase| {
        words
            .windows(phrase.len())
            .any(|window| same_words(phrase, window))
    })
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

/// How many words a measure restated after a slash takes: `/ 0.9 lb` in
/// `400 g / 0.9 lb onion`, the metric-then-imperial line RecipeTin Eats
/// writes (#182). It is the same amount said twice, so it is neither a second
/// amount nor part of the Food.
///
/// **Both halves must be there**: an amount, then a Unit — `0.9 lb`, glued
/// `0.6lb`, or one of the [`OPEN_UNITS`], `1 stick`. A slash before anything else is a Food
/// joined to a Food, and `salt/pepper` stays one name.
fn restated(words: &[&str]) -> Option<usize> {
    if words.first() != Some(&"/") {
        return None;
    }
    let words = &words[1..];
    if let Some(first) = words.first() {
        let digits = first.find(|c: char| c.is_alphabetic()).filter(|&at| at > 0);
        if let Some(at) = digits {
            let (amount, unit) = first.split_at(at);
            if units::parse_amount(amount).is_some() && units::recognise(unit).is_some() {
                return Some(2);
            }
        }
    }
    let amount = amount_words(words)?;
    let after = &words[amount..];
    (1..=after.len().min(3))
        .rev()
        .find(|&take| {
            let candidate = after[..take].join(" ");
            units::recognise(&candidate).is_some() || (take == 1 && listed(OPEN_UNITS, after[0]))
        })
        .map(|take| 1 + amount + take)
}

/// How many leading words read as one amount: a range if one opens the line,
/// and otherwise the longest run of up to three, so `1 1/2` beats `1`.
fn amount_words(words: &[&str]) -> Option<usize> {
    range_words(words).or_else(|| {
        (1..=words.len().min(3))
            .rev()
            .find(|&take| units::parse_amount(&words[..take].join(" ")).is_some())
    })
}

/// The words that join the two ends of a range written out, in each Language
/// Kamosu reads: `2 to 3`, `2 ou 3`, `2 o 3`, and `2 a 3`, which [`listed`]
/// folds so it is French `2 à 3` too. `or`, `ou` and `o` are
/// [`CHOICE_WORDS`] as well: between two numbers they join a range, and
/// anywhere else they offer a choice.
const RANGE_WORDS: &[&str] = &["to", "or", "a", "ou", "o"];

/// The most words a range takes: an amount of up to three either side of
/// the word that joins them, `1 1/2 to 2 1/2`.
const LONGEST_RANGE: usize = 7;

/// **How many leading words are a range**, `2-3` or `1 1/2 to 2` (#167).
///
/// Both ends must be amounts [`units::parse_amount`] reads, joined by a
/// hyphen, a dash or one of the [`RANGE_WORDS`], with nothing between
/// them. `4 g or 1 tsp` is therefore two measures and no range, and `1-inch`
/// is a number measuring a word.
///
/// The range is kept as written and is worth nothing: `parse_amount` answers
/// nothing to it, so a range is never scaled, converted or added up on a
/// shopping list (ADR 0016). What follows it is read as it is after any other
/// amount, so `2-3 basil leaves` names basil leaves.
fn range_words(words: &[&str]) -> Option<usize> {
    (1..=words.len().min(LONGEST_RANGE)).rev().find(|&take| {
        let words = &words[..take];
        let joined = words
            .iter()
            .position(|word| listed(RANGE_WORDS, word))
            .map(|at| (words[..at].join(" "), words[at + 1..].join(" ")));
        let dashed = || {
            let text = words.join(" ");
            let (low, high) = text.split_once(['-', '\u{2013}', '\u{2014}'])?;
            Some((low.to_string(), high.to_string()))
        };
        joined.or_else(dashed).is_some_and(|(low, high)| {
            units::parse_amount(&low).is_some() && units::parse_amount(&high).is_some()
        })
    })
}

/// Stand a slash that comes before a number apart as its own word, so
/// `400 g/0.9 lb` is the Unit `g` and a restated measure [`restated`] can see.
/// A slash with a digit on both sides is a fraction, `1/2`, and one before a
/// word joins two Foods, `salt/pepper`: both are left exactly as written.
fn slashes_apart(head: &str) -> String {
    let mut out = String::with_capacity(head.len() + 4);
    for (at, character) in head.char_indices() {
        let before = head[..at].trim_end().chars().next_back();
        let after = head[at + character.len_utf8()..]
            .trim_start()
            .chars()
            .next();
        let measure_follows = after.is_some_and(|c| c.is_ascii_digit());
        if character == '/' && measure_follows && !before.is_some_and(|c| c.is_ascii_digit()) {
            out.push_str(" / ");
        } else {
            out.push(character);
        }
    }
    out
}

/// The word that offers the cook a choice, in each Language Kamosu reads.
const CHOICE_WORDS: &[&str] = &["or", "ou", "o", "and/or", "et/ou", "y/o"];

/// **Whether the words offer a choice of two things to buy**, `butter or oil`
/// (#148). Such a line names no Food at any length. Taking the first
/// alternative is wrong more often than right, because the two usually share
/// their last word. `soft or silken tofu` is not a Food called *soft*. And
/// the whole of it, `butter or oil`, is a Food nothing will ever match.
///
/// Something has to follow the word, since a choice needs a second option.
/// It may open the words, as in `4 g or 1 rounded tsp instant yeast`, where
/// the choice is between two measures. An `or` straight after an elided
/// article is French for gold, so `poudre d'or` is one Food. `and/or` and its
/// French and Spanish forms offer the same choice.
fn offers_a_choice(words: &[&str]) -> bool {
    (0..words.len().saturating_sub(1)).any(|at| {
        listed(CHOICE_WORDS, words[at])
            && !at.checked_sub(1).is_some_and(|before| {
                let word = words[before].to_lowercase().replace('\u{2019}', "'");
                ELISIONS.contains(&word.as_str())
            })
    })
}

/// Glue and size words sit wherever they like — before the Unit as much as
/// after it — and belong to neither the Unit nor the Food. Stripped on both
/// sides of the Unit so `a pinch of salt` reads exactly as `pinch of salt`
/// does, which is the whole point: an article is not a measurement.
fn strip_glue<'a>(mut words: &'a [&'a str]) -> &'a [&'a str] {
    while !opens_a_fixed_name(words) && words.first().is_some_and(|word| is_glue(word)) {
        words = &words[1..];
    }
    words
}

/// The line cut at each comma that ends a clause. **A comma with a digit on
/// both sides ends nothing**: it is the decimal comma French and Spanish write
/// `1,2` with, and [`units::parse_amount`] reads it as part of the amount
/// (#180). Every other comma is a clause's end, so `salt, 2 pinches` still
/// cuts. An English thousands separator, `1,500 g`, reads as 1.5 by the same
/// rule; the text alone cannot tell the two apart.
fn split_clauses(line: &str) -> Vec<&str> {
    let digit = |at: Option<usize>| {
        at.and_then(|at| line.as_bytes().get(at))
            .is_some_and(u8::is_ascii_digit)
    };
    let mut pieces = Vec::new();
    let mut from = 0;
    for (at, _) in line.match_indices(',') {
        if !(digit(at.checked_sub(1)) && digit(Some(at + 1))) {
            pieces.push(&line[from..at]);
            from = at + 1;
        }
    }
    pieces.push(&line[from..]);
    pieces
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
/// pepper* is six words and a real thing to buy. Nearly everything longer in
/// 863 real lines is a paragraph. The exceptions are not prose, and none is a
/// Food either. A choice between two things, `1 tsp. Diamond Crystal or ½ tsp.
/// Morton kosher salt`, never gets this far, because [`offers_a_choice`]
/// refuses it at any length. An aside with no comma before it, `Thick
/// bellota steak burgers from the market`, is left unread by this limit on
/// purpose, because the words alone cannot tell the aside from the name
/// (#148).
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
    let name = without_warmth_or_size(&words[from..until])
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
    fn a_size_in_the_plural_is_dropped_like_one_in_the_singular() {
        // Two of the lines #185 was filed from, beside the singular that
        // already read as `oignon`.
        assert_eq!(
            read("2 petits poireaux, le blanc et le vert clair seulement, émincés finement"),
            parts(Some("2"), None, Some("poireaux"))
        );
        assert_eq!(
            read("5 petites échalotes, émincées finement"),
            parts(Some("5"), None, Some("échalotes"))
        );
        assert_eq!(
            read("1 petit oignon"),
            parts(Some("1"), None, Some("oignon"))
        );
        assert_eq!(read("3 grosses carottes"), read("3 carottes"));
        assert_eq!(read("2 pequeños tomates"), read("2 tomates"));
        // Green peas are *petits pois*, one name, not small *pois*.
        assert_eq!(
            read("200 g de petits pois"),
            parts(Some("200"), Some("g"), Some("petits pois"))
        );
    }

    #[test]
    fn a_size_written_after_the_food_is_dropped_too() {
        // French and Spanish put the size after the noun. From the dev
        // library, where it named a Food *tomates moyennes*.
        assert_eq!(read("2 tomates moyennes"), read("2 tomates"));
        assert_eq!(read("1 oignon moyen"), read("1 oignon"));
        // Coarse salt is a salt of its own, so `gros` after it stays.
        assert_eq!(
            read("1 c. à soupe de sel gros"),
            parts(Some("1"), Some("c. à soupe"), Some("sel gros"))
        );
        assert_eq!(
            read("1 cebolla grande"),
            parts(Some("1"), None, Some("cebolla"))
        );
        // *Whole* is not a size there: whole-wheat flour is a flour of its own.
        assert_eq!(
            read("500 g de farine de blé entier"),
            parts(Some("500"), Some("g"), Some("farine de blé entier"))
        );
    }

    #[test]
    fn how_warm_a_food_is_is_no_part_of_its_name() {
        // The line #185 was filed from, and Aurélien's choice on it: cold
        // butter is bought as butter.
        assert_eq!(
            read("100 g de beurre froid (7 c. à soupe)"),
            parts(Some("100"), Some("g"), Some("beurre"))
        );
        assert_eq!(read("1 tasse d'eau tiède"), read("1 tasse d'eau"));
        assert_eq!(read("250 ml de lait chaud"), read("250 ml de lait"));
        assert_eq!(read("1 taza de agua tibia"), read("1 taza de agua"));
        assert_eq!(read("1/2 cup cold butter"), read("1/2 cup butter"));
        assert_eq!(read("1 cup warm water"), read("1 cup water"));
        assert_eq!(read("1 cup lukewarm milk"), read("1 cup milk"));
        assert_eq!(read("2 oeufs à température ambiante"), read("2 oeufs"));
        assert_eq!(read("1 cup room temperature butter"), read("1 cup butter"));
        assert_eq!(read("1 cup room-temperature butter"), read("1 cup butter"));
        // *Hot* is left alone, because hot sauce is a thing to buy.
        assert_eq!(
            read("2 tbsp hot sauce"),
            parts(Some("2"), Some("tbsp"), Some("hot sauce"))
        );
        // So is a word for strength: *moutarde forte* is not *moutarde*, and
        // the stock stays as the cook named it.
        assert_eq!(
            read("300 g de fond de volaille corsé (ou 300 g d'eau)"),
            parts(Some("300"), Some("g"), Some("fond de volaille corsé"))
        );
        // A size behind the temperature goes too.
        assert_eq!(read("2 cold large eggs"), read("2 eggs"));
        assert_eq!(read("2 room temperature large eggs"), read("2 eggs"));
        // Where *cold* is the name's own word it stays.
        assert_eq!(
            read("200 g cold cuts"),
            parts(Some("200"), Some("g"), Some("cold cuts"))
        );
        assert_eq!(
            read("1 cup cold brew coffee"),
            parts(Some("1"), Some("cup"), Some("cold brew coffee"))
        );
        // *Very* goes with the temperature it strengthens.
        assert_eq!(read("500 ml d'eau très chaude"), read("500 ml d'eau"));
        assert_eq!(read("1 taza de agua muy fría"), read("1 taza de agua"));
        assert_eq!(read("1/2 cup very cold butter"), read("1/2 cup butter"));
        // A line that is only a temperature names no Food.
        assert_eq!(read("1 cup warm"), parts(Some("1"), Some("cup"), None));
    }

    #[test]
    fn a_step_amount_names_the_food_without_its_temperature() {
        let amounts = amounts_in_step("Ajouter 100 g de beurre froid en dés.");
        assert_eq!(amounts[0].food.as_deref(), Some("beurre"));
    }

    #[test]
    fn a_line_offering_a_choice_names_no_food() {
        // The lines #148 was filed from, as production read them on 2026-09-24.
        assert_eq!(read("Juice of 1 lemon or 3 tbsp calamansi juice"), None);
        assert_eq!(
            read("1 tsp. Diamond Crystal or ½ tsp. Morton kosher salt"),
            parts(Some("1"), Some("tsp."), None)
        );
        // Short enough to pass as one name before #148, which made it the
        // Food "butter or oil". That is neither butter nor oil.
        assert_eq!(
            read("1 cup butter or oil"),
            parts(Some("1"), Some("cup"), None)
        );
        // The two alternatives often share their last word, so the first
        // alone is not the Food either. This is not a Food called "soft".
        assert_eq!(
            read("16 oz soft or silken tofu"),
            parts(Some("16"), Some("oz"), None)
        );
        // Past a comma it is the same choice.
        assert_eq!(
            read("1 cup, butter or oil"),
            parts(Some("1"), Some("cup"), None)
        );
        assert_eq!(read("Olive oil or butter for cooking"), None);
        assert_eq!(
            read("2 tiges ciboule ou ciboulette"),
            parts(Some("2"), None, None)
        );
        assert_eq!(
            read("1 taza de mantequilla o aceite"),
            parts(Some("1"), Some("taza"), None)
        );
        assert_eq!(
            read("1 cup butter OR oil"),
            parts(Some("1"), Some("cup"), None)
        );
        assert_eq!(
            read("1 cup butter and/or oil"),
            parts(Some("1"), Some("cup"), None)
        );
        assert_eq!(
            read("1 c. à s. beurre et/ou huile"),
            parts(Some("1"), Some("c. à s."), None)
        );
        // Two measures of one Food is still a choice the cook makes.
        assert_eq!(
            read("4 g or 1 rounded tsp instant yeast"),
            parts(Some("4"), Some("g"), None)
        );
    }

    #[test]
    fn a_word_that_only_contains_or_is_no_choice() {
        // In French the "or" is gold, glued to its article, and it is the Food.
        assert_eq!(
            read("1 feuille d'or alimentaire"),
            parts(Some("1"), Some("feuille"), Some("or alimentaire"))
        );
        assert_eq!(
            read("1 g de poudre d'or"),
            parts(Some("1"), Some("g"), Some("poudre d'or"))
        );
        assert_eq!(read("1 orange"), parts(Some("1"), None, Some("orange")));
        assert_eq!(
            read("2 tbsp soy sauce"),
            parts(Some("2"), Some("tbsp"), Some("soy sauce"))
        );
        // Only an elided article makes the "or" part of a name. An apostrophe
        // that closes a possessive does not.
        assert_eq!(
            read("1 cup farmers' or goat cheese"),
            parts(Some("1"), Some("cup"), None)
        );
        // A choice said in the cook's aside is dropped with the aside, after
        // brackets and after a comma alike.
        assert_eq!(
            read("1 cup potato starch (or corn starch)"),
            parts(Some("1"), Some("cup"), Some("potato starch"))
        );
        assert_eq!(
            read("4 oz dark chocolate chunk, or your preference"),
            parts(Some("4"), Some("oz"), Some("dark chocolate chunk"))
        );
    }

    #[test]
    fn an_aside_with_no_comma_past_six_words_is_left_unread() {
        // "from the market" is the cook's aside, but with no comma to mark it
        // Kamosu cannot tell it from the name, and `from` can be part of what
        // is bought. Left unread on purpose (#148).
        assert_eq!(read("Thick bellota steak burgers from the market"), None);
    }

    #[test]
    fn a_decimal_comma_is_part_of_the_amount() {
        // Beef short ribs, translated into French (#180).
        assert_eq!(
            read("1,2 kg de viande"),
            parts(Some("1,2"), Some("kg"), Some("viande"))
        );
        assert_eq!(
            read("2,5 dl de lait"),
            parts(Some("2,5"), Some("dl"), Some("lait"))
        );
        assert_eq!(
            read("1.2 kg meat"),
            parts(Some("1.2"), Some("kg"), Some("meat"))
        );
        // A decimal comma and a clause comma on one line.
        assert_eq!(
            read("0,5 l de lait, tiède"),
            parts(Some("0,5"), Some("l"), Some("lait"))
        );
        // Any comma not held between two digits still ends the clause.
        assert_eq!(
            read("1 onion, chopped"),
            parts(Some("1"), None, Some("onion"))
        );
        assert_eq!(read("salt, 2 pinches"), parts(None, None, Some("salt")));
        assert_eq!(read("salt,2 pinches"), parts(None, None, Some("salt")));
    }

    #[test]
    fn a_range_is_the_amount_as_written() {
        // The lines #167 was filed from, as production read them on 2026-09-26.
        assert_eq!(
            read("2-3 basil leaves"),
            parts(Some("2-3"), None, Some("basil leaves"))
        );
        assert_eq!(
            read("225-250 g stale bread (about 1/2 loaf), torn"),
            parts(Some("225-250"), Some("g"), Some("stale bread"))
        );
        assert_eq!(
            read("½-1 cup cheese (shredded, goat cheese or feta)"),
            parts(Some("½-1"), Some("cup"), Some("cheese"))
        );
        // A choice names no Food (#148), so this line names none. The range
        // and the Unit are read all the same.
        assert_eq!(
            read("380-400 ml vegetable stock or water"),
            parts(Some("380-400"), Some("ml"), None)
        );
        // An en dash, an em dash, a mixed number either side, and a spaced
        // dash.
        assert_eq!(
            read("2—3 cloves garlic"),
            parts(Some("2—3"), Some("cloves"), Some("garlic"))
        );
        assert_eq!(
            read("2–3 cloves garlic"),
            parts(Some("2–3"), Some("cloves"), Some("garlic"))
        );
        assert_eq!(
            read("1–1½ cups flour"),
            parts(Some("1–1½"), Some("cups"), Some("flour"))
        );
        assert_eq!(
            read("1 1/2-2 cups sugar"),
            parts(Some("1 1/2-2"), Some("cups"), Some("sugar"))
        );
        assert_eq!(
            read("2 - 3 carrots"),
            parts(Some("2 - 3"), None, Some("carrots"))
        );
        assert_eq!(
            read("0.5-1 tsp chilli flakes"),
            parts(Some("0.5-1"), Some("tsp"), Some("chilli flakes"))
        );
    }

    #[test]
    fn a_range_written_with_a_word_is_a_range_too() {
        // Before #167 this read as two of a Food called `to 3 cloves garlic`.
        assert_eq!(
            read("2 to 3 cloves garlic"),
            parts(Some("2 to 3"), Some("cloves"), Some("garlic"))
        );
        // #148 caught this as a choice and named no Food. It is a range of
        // one Food, and the Food is onions.
        assert_eq!(
            read("1 or 2 onions"),
            parts(Some("1 or 2"), None, Some("onions"))
        );
        for (range, single) in [
            ("2 ou 3 gousses d'ail", "3 gousses d'ail"),
            ("2 à 3 c. à s. d'huile d'olive", "3 c. à s. d'huile d'olive"),
            ("2 a 3 dientes de ajo", "3 dientes de ajo"),
            ("1 o 2 cebollas", "2 cebollas"),
        ] {
            let (amount, unit, target) = read(range).expect(range);
            let (_, single_unit, single_target) = read(single).expect(single);
            assert_eq!(
                (unit, target),
                (single_unit, single_target),
                "{range} reads as {single} does"
            );
            let words: Vec<&str> = range.split(' ').take(3).collect();
            assert_eq!(amount.as_deref(), Some(words.join(" ").as_str()));
        }
        // A choice between two measures is still a choice, not a range: a
        // Unit stands between the numbers.
        assert_eq!(
            read("4 g or 1 rounded tsp instant yeast"),
            parts(Some("4"), Some("g"), None)
        );
    }

    #[test]
    fn a_range_leaves_what_already_read_alone() {
        assert_eq!(
            read("1½ cups flour"),
            parts(Some("1½"), Some("cups"), Some("flour"))
        );
        assert_eq!(
            read("1 1/2 cups flour"),
            parts(Some("1 1/2"), Some("cups"), Some("flour"))
        );
        // A hyphen with a word after it is no range: the number measures the
        // word, and nothing here is an amount of two numbers. The line reads
        // as it did before #167.
        assert_eq!(
            read("1-inch piece ginger"),
            parts(None, None, Some("1-inch piece ginger"))
        );
    }

    #[test]
    fn a_range_is_worth_nothing() {
        // No number, so no scaling, no conversion and no number on the
        // shopping list (ADR 0016).
        assert_eq!(units::parse_amount("2-3"), None);
        for scale in [1.0, 2.0] {
            assert_eq!(
                units::measured_line(
                    Some("½-1"),
                    Some("cup"),
                    scale,
                    units::Measures::Metric,
                    "en",
                    None
                ),
                None
            );
        }
        assert_eq!(
            units::how_much_of(Some("2-3"), Some("cups"), Some("4"), Some("cups")),
            None
        );
    }

    #[test]
    fn a_range_restated_after_a_slash_is_no_part_of_the_food() {
        assert_eq!(
            read("400-450 g / 0.9-1 lb onions"),
            parts(Some("400-450"), Some("g"), Some("onions"))
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
    fn the_french_count_words_are_counted_not_named() {
        // Seen translating recipes into French (#181).
        assert_eq!(
            read("1 bloc de feta"),
            parts(Some("1"), Some("bloc"), Some("feta"))
        );
        assert_eq!(
            read("1 bouteille de vin rouge"),
            parts(Some("1"), Some("bouteille"), Some("vin rouge"))
        );
        assert_eq!(
            read("1 bouteille de vin rouge sec"),
            parts(Some("1"), Some("bouteille"), Some("vin rouge sec"))
        );
        for (line, amount, unit, food) in [
            ("1 barquette de fraises", "1", "barquette", "fraises"),
            ("1 brique de crème", "1", "brique", "crème"),
            ("1 tablette de chocolat", "1", "tablette", "chocolat"),
            ("2 blocs de feta", "2", "blocs", "feta"),
            ("2 bouteilles de vin", "2", "bouteilles", "vin"),
            ("2 barquettes de fraises", "2", "barquettes", "fraises"),
            ("2 briques de crème", "2", "briques", "crème"),
            ("2 tablettes de chocolat", "2", "tablettes", "chocolat"),
            ("1 botella de vino tinto", "1", "botella", "vino tinto"),
            ("2 botellas de vino", "2", "botellas", "vino"),
            ("1 tableta de chocolate", "1", "tableta", "chocolate"),
            ("2 tabletas de chocolate", "2", "tabletas", "chocolate"),
        ] {
            assert_eq!(
                read(line),
                parts(Some(amount), Some(unit), Some(food)),
                "{line}"
            );
        }
        // Still the Food it names when there is nothing to count.
        assert_eq!(read("bouteille"), parts(None, None, Some("bouteille")));
    }

    #[test]
    fn a_few_is_no_amount_and_hides_no_unit() {
        // `quelques` counts nothing Kamosu can do arithmetic on, so it is
        // never the amount; it only stood in front of the Unit (#181).
        assert_eq!(
            read("quelques feuilles de menthe"),
            parts(None, Some("feuilles"), Some("menthe"))
        );
        assert_eq!(
            read("unas hojas de menta"),
            parts(None, Some("hojas"), Some("menta"))
        );
        assert_eq!(
            read("unos dientes de ajo"),
            parts(None, Some("dientes"), Some("ajo"))
        );
        // Before a number, `unos` is *about*, and the number is still the
        // amount rather than part of the Food.
        assert_eq!(
            read("unos 200 g de harina"),
            parts(Some("200"), Some("g"), Some("harina"))
        );
        assert_eq!(
            read("unas 3 hojas de laurel"),
            parts(Some("3"), Some("hojas"), Some("laurel"))
        );
    }

    #[test]
    fn the_count_words_that_already_read_still_do() {
        // What #181 must leave alone. `filet` stays a Unit: `un filet
        // d'huile d'olive` is a drizzle and reads right only that way.
        assert_eq!(
            read("2 filets de saumon"),
            parts(Some("2"), Some("filets"), Some("saumon"))
        );
        assert_eq!(
            read("1 filet d'huile d'olive"),
            parts(Some("1"), Some("filet"), Some("huile d'olive"))
        );
        assert_eq!(
            read("des feuilles de basilic"),
            parts(None, Some("feuilles"), Some("basilic"))
        );
        assert_eq!(
            read("1 cube de bouillon"),
            parts(Some("1"), Some("cube"), Some("bouillon"))
        );
        assert_eq!(
            read("2 c. à café de sel"),
            parts(Some("2"), Some("c. à café"), Some("sel"))
        );
        assert_eq!(
            read("1 boîte de tomates"),
            parts(Some("1"), Some("boîte"), Some("tomates"))
        );
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
    fn to_taste_is_no_part_of_the_food() {
        // The corpus's own lines: Crouton filed the phrase at the front (#163).
        for (line, food) in [
            ("to taste sea salt", "sea salt"),
            ("to taste pepper", "pepper"),
            ("to taste Salt and pepper", "Salt and pepper"),
            (
                "to taste Freshly ground black pepper",
                "Freshly ground black pepper",
            ),
            ("to taste Blue cheese", "Blue cheese"),
            ("to taste Grated old gouda", "Grated old gouda"),
            // At the end, it reads as the same line would without it.
            ("salt and pepper to taste", "salt and pepper"),
            ("sel au goût", "sel"),
            ("poivre selon le goût", "poivre"),
            ("à volonté sel", "sel"),
            ("sal al gusto", "sal"),
            ("pimienta a gusto", "pimienta"),
        ] {
            assert_eq!(read(line), parts(None, None, Some(food)), "{line}");
        }
        assert_eq!(read("salt and pepper to taste"), read("salt and pepper"));
        assert_eq!(
            read("2 tbsp olive oil to taste"),
            parts(Some("2"), Some("tbsp"), Some("olive oil"))
        );
        // Nothing left once the phrase is gone, so nothing is named.
        assert_eq!(read("to taste"), None);
        assert_eq!(read("2 tbsp to taste"), read("2 tbsp"));
        assert_eq!(read("cloves to taste"), read("cloves"));
        assert_eq!(read("cloves to taste"), parts(None, None, Some("cloves")));
        // Already right inside brackets, and still so.
        assert_eq!(
            read("black pepper (to taste)"),
            parts(None, None, Some("black pepper"))
        );
    }

    #[test]
    fn a_food_is_never_only_describing_words() {
        // The two corpus lines, where the comma cut the food off (#163).
        assert_eq!(
            read("1 1/2 lb boneless, skinless chicken thighs, cut into 1-inch pieces"),
            parts(
                Some("1 1/2"),
                Some("lb"),
                Some("boneless, skinless chicken thighs")
            )
        );
        assert_eq!(
            read("2 , boneless, skinless chicken breasts, trimmed (8-ounce)"),
            parts(Some("2"), None, Some("boneless, skinless chicken breasts"))
        );
        // The cook's aside past the comma, where the measure named no food
        // before it, is not a Food (#160 made this possible).
        assert_eq!(
            read("4 cloves, minced"),
            parts(Some("4"), Some("cloves"), None)
        );
        assert_eq!(
            read("2 tablespoons, divided"),
            parts(Some("2"), Some("tablespoons"), None)
        );
        assert_eq!(read("1 lb boneless"), parts(Some("1"), Some("lb"), None));
        assert_eq!(
            read("1 cup finely chopped"),
            parts(Some("1"), Some("cup"), None)
        );
        assert_eq!(
            read("500 g, désossées"),
            parts(Some("500"), Some("g"), None)
        );
        assert_eq!(
            read("2 gousses, hachées"),
            parts(Some("2"), Some("gousses"), None)
        );
        assert_eq!(
            read("2 dientes, picados"),
            parts(Some("2"), Some("dientes"), None)
        );
        // A describing word before the cook's aside is not the start of a
        // name: the run carries on past a comma only into another one.
        assert_eq!(
            read("1 lb boneless, cut into 1-inch pieces"),
            parts(Some("1"), Some("lb"), None)
        );
        assert_eq!(
            read("1 lb boneless, skinless, cut into pieces"),
            parts(Some("1"), Some("lb"), None)
        );
        assert_eq!(
            read("2 cups cooked, rice"),
            parts(Some("2"), Some("cups"), None)
        );
        // Neither a coffee nor a sauce is a describing word.
        assert_eq!(read("1 cortado"), parts(Some("1"), None, Some("cortado")));
        assert_eq!(
            read("2 tbsp picada"),
            parts(Some("2"), Some("tbsp"), Some("picada"))
        );
        // A describing word beside a food is still part of its name.
        assert_eq!(
            read("2 cloves minced garlic"),
            parts(Some("2"), Some("cloves"), Some("minced garlic"))
        );
        assert_eq!(
            read("1 lb boneless chicken thighs"),
            parts(Some("1"), Some("lb"), Some("boneless chicken thighs"))
        );
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

    #[test]
    fn a_measure_restated_after_a_slash_is_no_part_of_the_food() {
        // The Katsu curry, RecipeTin Eats style, and its French Translation
        // (#182).
        for (line, amount, unit, food) in [
            ("400 g / 0.9 lb onion", "400", "g", "onion"),
            ("250 g / 0.6lb potato (peeled)", "250", "g", "potato"),
            (
                "230 g / 0.5lb House Vermont Curry",
                "230",
                "g",
                "House Vermont Curry",
            ),
            ("400 g/0.9 lb onion", "400", "g", "onion"),
            ("400 g /0.9lb onion", "400", "g", "onion"),
            (
                "250 g / 0,6 lb de pommes de terre",
                "250",
                "g",
                "pommes de terre",
            ),
            (
                "230 g / 0,5 lb de roux de curry",
                "230",
                "g",
                "roux de curry",
            ),
            ("1 cup / 250 ml milk", "1", "cup", "milk"),
            ("1 1/2 cups / 375 ml stock", "1 1/2", "cups", "stock"),
            (
                "2 tbsp / 30 ml olive oil, divided",
                "2",
                "tbsp",
                "olive oil",
            ),
            ("1 cup / 240 ml / 8 fl oz water", "1", "cup", "water"),
            ("100 g / 1 stick butter", "100", "g", "butter"),
            ("2 tbsp / 3 cloves garlic", "2", "tbsp", "garlic"),
        ] {
            assert_eq!(
                read(line),
                parts(Some(amount), Some(unit), Some(food)),
                "{line}"
            );
        }
        // A slash with no measure after it still joins two Foods.
        assert_eq!(read("salt/pepper"), parts(None, None, Some("salt/pepper")));
        assert_eq!(
            read("salt / pepper"),
            parts(None, None, Some("salt / pepper"))
        );
        assert_eq!(
            read("1 tsp salt/pepper"),
            parts(Some("1"), Some("tsp"), Some("salt/pepper"))
        );
        // Stood apart only where a restated measure is then left out, so a
        // Food is never respaced.
        assert_eq!(
            read("1 tsp salt/2 tsp pepper"),
            parts(Some("1"), Some("tsp"), Some("salt/2 tsp pepper"))
        );
        assert_eq!(read("Salt/1 tsp"), parts(None, None, Some("Salt/1 tsp")));
        // A fraction is still a fraction.
        assert_eq!(
            read("1/2 cup sugar"),
            parts(Some("1/2"), Some("cup"), Some("sugar"))
        );
    }

    #[test]
    fn the_us_quart_is_a_unit() {
        // The Fusilli all'assassina and its French line (#182).
        for (line, unit, food) in [
            ("1 USqt Water", "USqt", "Water"),
            ("1 USqt d'eau", "USqt", "eau"),
            ("1 US qt water", "US qt", "water"),
            ("2 qt chicken stock", "qt", "chicken stock"),
            ("2 qts chicken stock", "qts", "chicken stock"),
            ("1 US quart water", "US quart", "water"),
        ] {
            let amount = &line[..1];
            assert_eq!(
                read(line),
                parts(Some(amount), Some(unit), Some(food)),
                "{line}"
            );
            assert_eq!(units::recognise(unit).map(|u| u.id), Some("quart"));
        }
        // In French a bare `quart` is a quarter, never a measure.
        assert_eq!(written("Laisser reposer 1 quart d'heure."), vec![]);
        assert_eq!(
            written("Bring 2 qt water to a boil."),
            vec![("2 qt", Some("water".into()))]
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
        // A Step's decimal comma stays inside its word, and one further on
        // only ends the Food (#180).
        assert_eq!(
            written("Ajoutez 1,2 kg de viande, 1,5 dl de lait."),
            vec![
                ("1,2 kg", Some("viande".into())),
                ("1,5 dl", Some("lait".into()))
            ]
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
