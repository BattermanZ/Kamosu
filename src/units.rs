//! Units, scaling and conversion — ADR 0016, #49.
//!
//! **The set of Units is open; the set Kamosu can convert is closed.** Whatever
//! the cook wrote is a Unit and is kept exactly as written: *poignée*, *noix*,
//! *sachet* and *bottle* are all perfectly legitimate, and simply never offer a
//! conversion. Against that open set sits [`UNITS`], a closed list Kamosu knows
//! well enough to do arithmetic on, each knowing its own spellings in the three
//! interface Languages — so `g`, `gr`, `gramme`, `grammes` and `grams` are one
//! Unit and spelling is not a data-entry problem.
//!
//! **A conversion is never a rewrite.** Nothing here touches an Ingredient Line
//! (ADR 0002). What it produces is one subordinate line — *how much, for you,
//! right now* — which is absent whenever it would only repeat the line
//! unchanged.
//!
//! **Scaling and conversion are one act, so they get one slot.** Both answer
//! the same question, and given one line they never have to negotiate:
//! scaled-not-converted, converted-not-scaled and both look identical.
//!
//! ## The two arithmetic decisions worth knowing
//!
//! **Round last, once.** Scale, then convert, then round the number actually
//! shown. Rounding twice turns three cups into 720 ml when it is 710.
//!
//! **The cup is 236.588 ml, not 240.** ADR 0016 states both figures and they
//! agree: 236.588 is the arithmetic, 240 is what one cup *displays* as once the
//! ladder in [`round_to`] has run — and the ADR's own worked example ("three
//! cups is 710, not 720") only holds on 236.588. The same fall-out gives a
//! tablespoon 15 ml and a teaspoon 5 ml, exactly as the ADR's consequences say.
//! A tablespoon is a sixteenth of a cup and a teaspoon a third of that, so one
//! Cup Weight yields every volume measure of a Food by arithmetic.
//!
//! ## What is deliberately not here
//!
//! No test seam. #49 requires all of this exercised through Operations, so the
//! tables below carry no unit tests of their own — `tests/behaviour.rs` drives
//! them through `get_recipe` against a real database, which is the only way to
//! prove both Doors got it.

use std::collections::HashMap;
use std::sync::LazyLock;

/// Which arithmetic a Unit takes part in. Cross-family conversion — a volume of
/// a dry good into grams — is the one crossing, and it runs off a Cup Weight.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Family {
    /// Base unit: the gram.
    Mass,
    /// Base unit: the millilitre.
    Volume,
}

/// Where a Unit comes from, which decides who needs it converted. A metric
/// reader needs `2 cups` turned into grams; she needs nothing done to `250 ml`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum System {
    Metric,
    /// American, imperial and the explicitly-named regional variants — every
    /// Unit a metric kitchen has no equipment for.
    Customary,
}

/// One member of the closed convertible set: what it is, how big it is, and
/// every way the three interface Languages spell it.
pub struct Unit {
    /// Stable internal name. Never shown; [`display`] renders what is.
    pub id: &'static str,
    pub family: Family,
    pub system: System,
    /// How many grams (Mass) or millilitres (Volume) one of these is.
    pub base: f64,
    /// Every spelling, written the way a person reads it — [`fold`] is applied
    /// to both this table and the cook's word, so punctuation and capitals need
    /// no thought here.
    ///
    /// Accented and unaccented forms ARE both listed, rather than being folded
    /// together: elsewhere in Kamosu accents are meaning (a Food Match will not
    /// equate *maïs* with *mais*), and a unit table that quietly stripped them
    /// would be the one place that rule did not hold. Listing `cuillere a cafe`
    /// beside `cuillère à café` says the same forgiveness out loud.
    pub spellings: &'static [&'static str],
}

/// The closed set. ADR 0016 names it: gram, kilogram, millilitre, litre,
/// teaspoon, tablespoon, cup, fluid ounce, ounce, pound, and a few explicitly
/// named regional variants.
///
/// Centilitre and decilitre are here for one measured reason: French recipes
/// write `20 cl` where English ones write `200 ml`, and Kamosu's corpus is
/// French-heavy. They are ordinary metric members, not variants.
///
/// `imperial tablespoon` and `Australian tablespoon` are ordinary members too,
/// at the cost of one line each — a recipe that names one gets it right without
/// anyone being asked anything. The accepted cost ADR 0016 records is the
/// Australian tablespoon nobody *named*: three tablespoons of soy sauce read as
/// 45 ml where the author meant 60.
pub const UNITS: &[Unit] = &[
    // --- Mass -------------------------------------------------------------
    Unit {
        id: "gram",
        family: Family::Mass,
        system: System::Metric,
        base: 1.0,
        spellings: &[
            "g", "gr", "gm", "gram", "grams", "gramme", "grammes", "gramo", "gramos",
        ],
    },
    Unit {
        id: "kilogram",
        family: Family::Mass,
        system: System::Metric,
        base: 1000.0,
        spellings: &[
            "kg",
            "kgs",
            "kilo",
            "kilos",
            "kilogram",
            "kilograms",
            "kilogramme",
            "kilogrammes",
            "kilogramo",
            "kilogramos",
        ],
    },
    Unit {
        id: "ounce",
        family: Family::Mass,
        system: System::Customary,
        base: 28.349_523_125,
        spellings: &["oz", "ounce", "ounces", "once", "onces", "onza", "onzas"],
    },
    Unit {
        id: "pound",
        family: Family::Mass,
        system: System::Customary,
        base: 453.592_37,
        spellings: &[
            "lb", "lbs", "pound", "pounds", "livre", "livres", "libra", "libras",
        ],
    },
    // --- Volume -----------------------------------------------------------
    Unit {
        id: "millilitre",
        family: Family::Volume,
        system: System::Metric,
        base: 1.0,
        spellings: &[
            "ml",
            "mls",
            "millilitre",
            "millilitres",
            "milliliter",
            "milliliters",
            "mililitro",
            "mililitros",
        ],
    },
    Unit {
        id: "centilitre",
        family: Family::Volume,
        system: System::Metric,
        base: 10.0,
        spellings: &[
            "cl",
            "centilitre",
            "centilitres",
            "centiliter",
            "centiliters",
            "centilitro",
            "centilitros",
        ],
    },
    Unit {
        id: "decilitre",
        family: Family::Volume,
        system: System::Metric,
        base: 100.0,
        spellings: &[
            "dl",
            "decilitre",
            "decilitres",
            "deciliter",
            "deciliters",
            "decilitro",
            "decilitros",
        ],
    },
    Unit {
        id: "litre",
        family: Family::Volume,
        system: System::Metric,
        base: 1000.0,
        spellings: &[
            "l", "lt", "litre", "litres", "liter", "liters", "litro", "litros",
        ],
    },
    Unit {
        id: "teaspoon",
        family: Family::Volume,
        system: System::Customary,
        // A third of a tablespoon, which is a sixteenth of a cup.
        base: 236.588_236_5 / 48.0,
        spellings: &[
            "tsp",
            "tsps",
            "ts",
            "teaspoon",
            "teaspoons",
            "cuillère à café",
            "cuillere a cafe",
            "cuillères à café",
            "cuilleres a cafe",
            "c à c",
            "càc",
            "cac",
            "cucharadita",
            "cucharaditas",
            "cdta",
        ],
    },
    Unit {
        id: "tablespoon",
        family: Family::Volume,
        system: System::Customary,
        // A sixteenth of a cup — which is what makes one Cup Weight enough to
        // answer every volume measure of a Food (ADR 0016).
        base: 236.588_236_5 / 16.0,
        spellings: &[
            "tbsp",
            "tbsps",
            "tbs",
            "tb",
            "tablespoon",
            "tablespoons",
            "cuillère à soupe",
            "cuillere a soupe",
            "cuillères à soupe",
            "cuilleres a soupe",
            "c à s",
            "càs",
            "cas",
            "cucharada",
            "cucharadas",
            "cda",
            "cdas",
        ],
    },
    Unit {
        id: "cup",
        family: Family::Volume,
        system: System::Customary,
        base: 236.588_236_5,
        spellings: &["cup", "cups", "tasse", "tasses", "taza", "tazas"],
    },
    Unit {
        id: "fluid ounce",
        family: Family::Volume,
        system: System::Customary,
        base: 29.573_529_562_5,
        spellings: &[
            "fl oz",
            "floz",
            "fluid ounce",
            "fluid ounces",
            "once liquide",
            "onces liquides",
            "onza líquida",
            "onza liquida",
            "onzas líquidas",
            "onzas liquidas",
        ],
    },
    Unit {
        id: "imperial tablespoon",
        family: Family::Volume,
        system: System::Customary,
        base: 17.758_164_0,
        spellings: &[
            "imperial tablespoon",
            "imperial tablespoons",
            "imperial tbsp",
            "cuillère à soupe impériale",
            "cuillere a soupe imperiale",
            "cucharada imperial",
        ],
    },
    Unit {
        id: "australian tablespoon",
        family: Family::Volume,
        system: System::Customary,
        base: 20.0,
        spellings: &[
            "australian tablespoon",
            "australian tablespoons",
            "australian tbsp",
            "aus tbsp",
            "cuillère à soupe australienne",
            "cuillere a soupe australienne",
            "cucharada australiana",
        ],
    },
];

/// How a person measures, held on their account beside their Reading Language.
/// The default is American, a stated convention rather than a guess about
/// anybody (ADR 0016).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Measures {
    Us,
    Metric,
    /// Leave every Unit alone. Scaling still happens: a recipe at double Yield
    /// has a different number on it whatever system it is written in.
    AsWritten,
}

impl Measures {
    /// Read the stored column. Anything unrecognised reads as the stated
    /// default rather than failing a recipe — nothing downstream of a
    /// preference should ever be able to stop a line rendering.
    pub fn from_stored(value: &str) -> Self {
        match value {
            "metric" => Measures::Metric,
            "as_written" => Measures::AsWritten,
            _ => Measures::Us,
        }
    }
}

/// Fold a written word for comparison against the tables below: the same
/// Unicode fold Kamosu uses on every other word (NFD, then the default case
/// fold, then NFD again — see `core::folded_word`), and additionally forgiving
/// about the punctuation and spacing a unit abbreviation carries, so `c. à c.`
/// and `càc` are one spelling.
///
/// The Unicode half is load-bearing rather than decorative. An *é* typed as one
/// character and an *e* followed by a combining accent look identical on screen
/// and are different bytes, and a French cookbook meets both depending on the
/// keyboard — so a table holding `maïzena` or `pépites de chocolat` and folding
/// only by `to_lowercase` would silently miss half the Foods it names. **Both
/// sides go through this**: the tables are folded at lookup time too, so a
/// literal written here can never be a spelling nothing can reach.
fn fold(word: &str) -> String {
    use caseless::Caseless;
    use unicode_normalization::UnicodeNormalization;
    let cased: String = word.chars().nfd().default_case_fold().nfd().collect();
    let mut folded = String::with_capacity(cased.len());
    let mut wants_space = false;
    for character in cased.chars() {
        if character.is_whitespace() || character == '.' {
            // A separator only matters between two kept characters, so a
            // trailing one can never survive: `c. à c.` folds to `c à c`.
            wants_space = !folded.is_empty();
            continue;
        }
        if wants_space {
            folded.push(' ');
            wants_space = false;
        }
        folded.push(character);
    }
    folded
}

/// Every spelling in the closed set, folded once, pointing at its Unit.
///
/// Built rather than searched so that the tables above can be written the way a
/// person reads them — accented, punctuated, in three languages — while every
/// lookup still compares fold against fold. A spelling listed twice, here or
/// across two Units, would be a table Kamosu could not answer consistently, so
/// it is caught at build time rather than left to shadow silently.
static BY_SPELLING: LazyLock<HashMap<String, &'static Unit>> = LazyLock::new(|| {
    let mut index = HashMap::new();
    for unit in UNITS {
        for spelling in unit.spellings {
            let folded = fold(spelling);
            assert!(
                !folded.is_empty(),
                "a Unit spelling folds away to nothing: {spelling:?}"
            );
            if let Some(taken) = index.insert(folded.clone(), unit) {
                panic!(
                    "the spelling {folded:?} names two Units: {} and {}",
                    taken.id, unit.id
                );
            }
        }
    }
    index
});

/// The Unit a written word names, or nothing — which is not an error. A word
/// outside the closed set is no less real a Unit; it simply never converts.
pub fn recognise(word: &str) -> Option<&'static Unit> {
    let folded = fold(word);
    if folded.is_empty() {
        return None;
    }
    BY_SPELLING.get(&folded).copied()
}

// --- Reading a quantity ------------------------------------------------------

/// The vulgar fractions a recipe actually contains, and what each is worth.
/// Both directions use this table: reading `4½` off a line, and writing `4½`
/// back out for a reader whose cups are marked that way.
const FRACTIONS: &[(char, f64)] = &[
    ('½', 0.5),
    ('⅓', 1.0 / 3.0),
    ('⅔', 2.0 / 3.0),
    ('¼', 0.25),
    ('¾', 0.75),
    ('⅕', 0.2),
    ('⅖', 0.4),
    ('⅗', 0.6),
    ('⅘', 0.8),
    ('⅙', 1.0 / 6.0),
    ('⅚', 5.0 / 6.0),
    ('⅛', 0.125),
    ('⅜', 0.375),
    ('⅝', 0.625),
    ('⅞', 0.875),
];

/// How much a written amount is worth, or nothing where Kamosu cannot read it.
///
/// Nothing is the ordinary answer for a great deal of real writing — `a pinch`,
/// `2-3`, `to taste` — and it is not a failure. A quantity Kamosu could not
/// read is shown whole rather than guessed at (ADR 0002), which here means no
/// subordinate line at all.
///
/// It reads `2`, `2.5`, `2,5` (the French decimal comma), `1/2`, `1 1/2`, `½`
/// and `4½`.
pub fn parse_amount(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    // A trailing vulgar fraction, with or without a whole number before it.
    if let Some(last) = text.chars().next_back()
        && let Some((_, value)) = FRACTIONS.iter().find(|(glyph, _)| *glyph == last)
    {
        let whole = text[..text.len() - last.len_utf8()].trim();
        if whole.is_empty() {
            return Some(*value);
        }
        return parse_plain(whole).map(|number| number + value);
    }

    // `1 1/2` — a whole number and a written fraction.
    if let Some((whole, rest)) = text.split_once(' ')
        && let Some(number) = parse_plain(whole)
        && let Some(fraction) = parse_ratio(rest.trim())
    {
        return Some(number + fraction);
    }

    parse_ratio(text).or_else(|| parse_plain(text))
}

/// `1/2`, and nothing else that contains a slash.
fn parse_ratio(text: &str) -> Option<f64> {
    let (top, bottom) = text.split_once('/')?;
    let top = parse_plain(top.trim())?;
    let bottom = parse_plain(bottom.trim())?;
    (bottom != 0.0).then_some(top / bottom)
}

/// A plain number, accepting the French decimal comma.
fn parse_plain(text: &str) -> Option<f64> {
    let text = text.trim().replace(',', ".");
    if text.is_empty() {
        return None;
    }
    text.parse::<f64>().ok().filter(|number| number.is_finite())
}

// --- Cup Weight --------------------------------------------------------------

/// What one cup of a staple weighs, about — the shipped half of Cup Weight.
///
/// ADR 0016 costed this precisely: of the 118 real cup rows in the corpus, 42
/// are liquids (pure arithmetic to millilitres) and 58 are baking staples, so
/// roughly twenty hand-checked entries plus plain arithmetic covers 100 of 118.
/// The remaining 18 — sliced mushrooms, spinach, blueberries — are where a cup
/// is a loose handful and a gram figure would be false precision. They are not
/// a gap to close; they are lines that offer millilitres instead.
///
/// A figure stored on the Food always beats the figure here (ADR 0016: "an
/// override always beats the shipped figure"), and #47 already owns that
/// column. This is a curated list checked against USDA SR Legacy's
/// `food_portion.csv`, not an ingest of it.
///
/// Each entry is (folded name, grams per cup). Names are listed in all three
/// interface Languages because a Food is known by its words and a French
/// recipe's *farine* is the same staple as an English one's *flour*.
const SHIPPED_CUP_WEIGHTS: &[(&str, f64)] = &[
    // Flours
    ("flour", 125.0),
    ("all-purpose flour", 125.0),
    ("plain flour", 125.0),
    ("farine", 125.0),
    ("harina", 125.0),
    ("bread flour", 127.0),
    ("wholemeal flour", 120.0),
    ("whole wheat flour", 120.0),
    ("farine complète", 120.0),
    ("harina integral", 120.0),
    ("cornflour", 120.0),
    ("cornstarch", 120.0),
    ("maïzena", 120.0),
    ("maizena", 120.0),
    // Sugars
    ("sugar", 200.0),
    ("granulated sugar", 200.0),
    ("caster sugar", 200.0),
    ("sucre", 200.0),
    ("azúcar", 200.0),
    ("azucar", 200.0),
    ("brown sugar", 213.0),
    ("cassonade", 213.0),
    ("azúcar moreno", 213.0),
    ("azucar moreno", 213.0),
    ("icing sugar", 120.0),
    ("powdered sugar", 120.0),
    ("sucre glace", 120.0),
    ("azúcar glas", 120.0),
    ("azucar glas", 120.0),
    // Fats and dairy. No milk, water, cream or yoghurt: a liquid poured into a
    // jug is pure arithmetic to millilitres, which is 42 of the corpus's 118
    // cup rows and needs no table at all. Giving them a Cup Weight would turn
    // `2 cups milk` into grams, which is not how anybody measures milk.
    ("butter", 227.0),
    ("beurre", 227.0),
    ("mantequilla", 227.0),
    ("grated cheese", 100.0),
    ("fromage râpé", 100.0),
    ("fromage rape", 100.0),
    ("queso rallado", 100.0),
    // Grains and pulses
    ("rice", 185.0),
    ("riz", 185.0),
    ("arroz", 185.0),
    ("rolled oats", 90.0),
    ("oats", 90.0),
    ("flocons d'avoine", 90.0),
    ("avena", 90.0),
    ("lentils", 200.0),
    ("lentilles", 200.0),
    ("lentejas", 200.0),
    // The rest of the baking shelf
    ("chocolate chips", 170.0),
    ("pépites de chocolat", 170.0),
    ("pepites de chocolat", 170.0),
    ("pepitas de chocolate", 170.0),
    ("cocoa powder", 85.0),
    ("cacao", 85.0),
    ("honey", 340.0),
    ("miel", 340.0),
    ("walnuts", 117.0),
    ("noix", 117.0),
    ("nueces", 117.0),
    ("pecans", 109.0),
    ("noix de pécan", 109.0),
    ("nueces pecanas", 109.0),
    ("almonds", 143.0),
    ("amandes", 143.0),
    ("almendras", 143.0),
    ("ground almonds", 96.0),
    ("poudre d'amandes", 96.0),
    ("almendra molida", 96.0),
    ("breadcrumbs", 108.0),
    ("chapelure", 108.0),
    ("pan rallado", 108.0),
    // No salt, vanilla, baking powder or spice mix. The corpus's teaspoon and
    // tablespoon rows are dominated by exactly those, and ADR 0016's reading of
    // them is blunt: nobody wants them in grams.
];

/// The shipped staples, folded once. Same reason as [`BY_SPELLING`]: a Food is
/// matched on the fold everywhere else in Kamosu, and this table names Foods.
static BY_STAPLE: LazyLock<HashMap<String, f64>> = LazyLock::new(|| {
    SHIPPED_CUP_WEIGHTS
        .iter()
        .map(|(staple, grams)| (fold(staple), *grams))
        .collect()
});

/// The shipped Cup Weight for a Food, found by any of its names. Nothing is the
/// ordinary answer and is not a gap: a Food with no Cup Weight is a line that
/// offers millilitres instead of grams.
///
/// The match is exact on the fold, exactly as a Food Match is (CONTEXT.md) —
/// `farine T55` is its own Food and gets silence rather than a guess at which
/// flour somebody meant. That silence is the designed failure mode, not a bug.
pub fn shipped_cup_weight<'a>(names: impl IntoIterator<Item = &'a str>) -> Option<f64> {
    names
        .into_iter()
        .find_map(|name| BY_STAPLE.get(&fold(name)).copied())
}

// --- The one subordinate line ------------------------------------------------

/// What the subordinate line ends up saying, before it is worded.
struct Measured {
    quantity: f64,
    /// The Unit it is in, where that is one Kamosu knows.
    unit: Option<&'static Unit>,
    /// The cook's own word, kept for a Unit outside the closed set — the
    /// quantity multiplies and the unit is untouched (ADR 0016).
    as_written: Option<String>,
}

/// **The one line beneath the written line**, or nothing.
///
/// Nothing is the right answer far more often than something. It is absent
/// where Kamosu read no quantity, where it could not read the quantity it
/// found, and — the common case — where the line is already in this reader's
/// measures at the Yield they are cooking, so a line would only repeat what is
/// already there.
///
/// `scale` is how far the Yield being cooked is from the Yield as written; 1.0
/// is a recipe read rather than scaled. `cup_weight_grams` is the Food's
/// effective Cup Weight — the override where somebody set one, the shipped
/// figure otherwise — and its absence is what turns a cup of something into
/// millilitres rather than into grams.
pub fn measured_line(
    amount: Option<&str>,
    unit: Option<&str>,
    scale: f64,
    measures: Measures,
    language: &str,
    cup_weight_grams: Option<f64>,
) -> Option<String> {
    let quantity = parse_amount(amount?)?;
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let scaled = quantity * scale;
    let source = unit.and_then(recognise);

    let measured = convert(scaled, source, unit, measures, cup_weight_grams);

    // Absent whenever it would only repeat the line unchanged: same Unit, same
    // number. The comparison is on the rounded figure, because that is the one
    // a reader would be comparing against the line above.
    let unchanged = measured.unit.map(|u| u.id) == source.map(|u| u.id)
        && round_to(measured.quantity, measured.unit) == round_to(quantity, source);
    if unchanged {
        return None;
    }

    Some(word_it(measured, language))
}

/// Scale, then convert — never the other way round, and never rounding in
/// between (ADR 0016: "round last, once").
fn convert(
    scaled: f64,
    source: Option<&'static Unit>,
    written: Option<&str>,
    measures: Measures,
    cup_weight_grams: Option<f64>,
) -> Measured {
    // A Unit outside the closed set, or a reader who asked for no conversion:
    // the quantity multiplies and the unit is untouched.
    let Some(source) = source.filter(|_| measures != Measures::AsWritten) else {
        return Measured {
            quantity: scaled,
            unit: source,
            as_written: written.map(str::to_string),
        };
    };

    // A recipe already in your measures is untouched (ADR 0016). Kamosu
    // converts BETWEEN systems and never re-expresses within one: an American
    // told that her `4 tsp` is `about 1¼ tbsp` has been given a second way to
    // say a thing she already has the spoons for, and a line that says nothing
    // new is exactly the line the ADR wanted absent. Scaling still applies —
    // it is what the fall-through below carries.
    let reader = match measures {
        Measures::Metric => System::Metric,
        Measures::Us => System::Customary,
        Measures::AsWritten => unreachable!("as-written left before conversion"),
    };
    if source.system == reader {
        return Measured {
            quantity: scaled,
            unit: Some(source),
            as_written: None,
        };
    }

    let base = scaled * source.base;
    match (measures, source.family) {
        // A metric kitchen has scales, so a customary volume of a dry good
        // becomes a weight — the one cross-family crossing there is, and the
        // whole reason a Cup Weight exists.
        (Measures::Metric, Family::Volume) if source.system == System::Customary => {
            match cup_weight_grams {
                Some(per_cup) => {
                    let grams = base / unit_by_id("cup").base * per_cup;
                    in_metric_mass(grams)
                }
                // A blank Cup Weight is not a gap to close. It is a line that
                // offers millilitres instead, and says nothing about grams.
                None => in_metric_volume(base),
            }
        }
        (Measures::Metric, Family::Volume) => in_metric_volume(base),
        (Measures::Metric, Family::Mass) => in_metric_mass(base),
        // An American kitchen has cups and a set of spoons, so a metric measure
        // becomes one of those. Nothing crosses families this way: she is not
        // short of a cup, and turning her cups into grams would take away the
        // equipment she has.
        (Measures::Us, Family::Volume) => in_customary_volume(base),
        (Measures::Us, Family::Mass) => in_customary_mass(base),
        (Measures::AsWritten, _) => unreachable!("as-written left before conversion"),
    }
}

fn in_metric_mass(grams: f64) -> Measured {
    let unit = if grams >= 1000.0 { "kilogram" } else { "gram" };
    at(grams, unit, if grams >= 1000.0 { 1000.0 } else { 1.0 })
}

fn in_metric_volume(millilitres: f64) -> Measured {
    let big = millilitres >= 1000.0;
    at(
        millilitres,
        if big { "litre" } else { "millilitre" },
        if big { 1000.0 } else { 1.0 },
    )
}

/// Which piece of American equipment this is a measure of. The thresholds are
/// the drawer, not the arithmetic: under a tablespoon you reach for teaspoons,
/// under a quarter cup for tablespoons, and above that for the cup itself.
fn in_customary_volume(millilitres: f64) -> Measured {
    let tablespoon = unit_by_id("tablespoon").base;
    let cup = unit_by_id("cup").base;
    if millilitres < tablespoon {
        at(millilitres, "teaspoon", unit_by_id("teaspoon").base)
    } else if millilitres < cup / 4.0 {
        at(millilitres, "tablespoon", tablespoon)
    } else {
        at(millilitres, "cup", cup)
    }
}

fn in_customary_mass(grams: f64) -> Measured {
    let pound = unit_by_id("pound").base;
    if grams < pound {
        at(grams, "ounce", unit_by_id("ounce").base)
    } else {
        at(grams, "pound", pound)
    }
}

fn at(base_quantity: f64, id: &'static str, per: f64) -> Measured {
    Measured {
        quantity: base_quantity / per,
        unit: Some(unit_by_id(id)),
        as_written: None,
    }
}

fn unit_by_id(id: &'static str) -> &'static Unit {
    UNITS
        .iter()
        .find(|unit| unit.id == id)
        .expect("every id named here is in UNITS")
}

// --- Rounding, which happens last and once -----------------------------------

/// Round to something a scale or a jug can actually show.
///
/// Every ladder here is equipment, not arithmetic. A kitchen scale reads to the
/// gram under 100 g and nobody trusts its last digit above that; a measuring jug
/// is marked every 10 ml through its useful range; and the cups in the drawer
/// are marked in eighths, which is why an American measure rounds to a fraction
/// and a metric one does not.
///
/// Kept as one function rather than a field on [`Unit`] deliberately: half of
/// these rules turn on the *magnitude* as well as the Unit — a gram rounds to 1
/// under 100 and to 5 above it — so a per-Unit field would carry a rule that
/// still needed reading here. One table of equipment, read top to bottom, is
/// what makes it checkable against a real drawer.
fn round_to(quantity: f64, unit: Option<&'static Unit>) -> f64 {
    let step = match unit.map(|unit| unit.id) {
        Some("gram") if quantity < 100.0 => 1.0,
        Some("gram") => 5.0,
        Some("kilogram") | Some("litre") => 0.05,
        Some("millilitre") if quantity < 25.0 => 1.0,
        Some("millilitre") if quantity < 100.0 => 5.0,
        Some("millilitre") => 10.0,
        Some("cup") => 0.125,
        Some("teaspoon") | Some("tablespoon") | Some("ounce") | Some("pound") => 0.25,
        // A Unit Kamosu does not know still scales, and the cook's own measure
        // is as likely to be halved as anything else.
        _ => 0.125,
    };
    (quantity / step).round() * step
}

// --- Wording it --------------------------------------------------------------

/// "about", in the three interface Languages.
///
/// It is unconditional. Kamosu rounded, every time, so "about" is true every
/// time — and a mark that appears *sometimes* teaches people that its absence
/// is a promise (ADR 0016, and ADR 0015 in a different guise). Spanish uses
/// *aprox.* rather than *unos*, which would have to agree with a noun this code
/// does not know the gender of.
fn about(language: &str) -> &'static str {
    match language {
        "fr" => "environ",
        "es" => "aprox.",
        _ => "about",
    }
}

/// How a Unit is written for a reader, singular and plural. Only the cup needs
/// both: every other member of the closed set is written as an invariant
/// symbol or abbreviation in all three Languages.
fn display(unit: &Unit, language: &str, plural: bool) -> String {
    let word = match (unit.id, language, plural) {
        ("cup", "fr", false) => "tasse",
        ("cup", "fr", true) => "tasses",
        ("cup", "es", false) => "taza",
        ("cup", "es", true) => "tazas",
        ("cup", _, false) => "cup",
        ("cup", _, true) => "cups",
        ("gram", _, _) => "g",
        ("kilogram", _, _) => "kg",
        ("millilitre", _, _) => "ml",
        ("centilitre", _, _) => "cl",
        ("decilitre", _, _) => "dl",
        ("litre", _, _) => "l",
        ("ounce", _, _) => "oz",
        ("pound", _, _) => "lb",
        ("fluid ounce", _, _) => "fl oz",
        ("teaspoon", "fr", _) => "c. à c.",
        ("teaspoon", "es", _) => "cdta",
        ("teaspoon", _, _) => "tsp",
        ("tablespoon", "fr", _) => "c. à s.",
        ("tablespoon", "es", _) => "cda",
        ("tablespoon", _, _) => "tbsp",
        ("imperial tablespoon", _, _) => "imperial tbsp",
        ("australian tablespoon", _, _) => "australian tbsp",
        (other, _, _) => other,
    };
    word.to_string()
}

fn word_it(measured: Measured, language: &str) -> String {
    let quantity = round_to(measured.quantity, measured.unit);
    // Metric stays whole numbers; the cups in the drawer are marked in
    // fractions, so an American measure is written the way the cup is marked.
    let fractional = measured
        .unit
        .is_none_or(|unit| unit.system == System::Customary);
    let number = if fractional {
        as_fraction(quantity)
    } else {
        as_decimal(quantity)
    };
    let unit = match (&measured.unit, &measured.as_written) {
        (Some(unit), _) => display(unit, language, quantity != 1.0),
        // The cook's own word, exactly as written.
        (None, Some(written)) => written.clone(),
        (None, None) => String::new(),
    };
    if unit.is_empty() {
        format!("{} {number}", about(language))
    } else {
        format!("{} {number} {unit}", about(language))
    }
}

/// `4½`, because that is how the cups in the drawer are marked.
fn as_fraction(quantity: f64) -> String {
    let whole = quantity.trunc();
    let part = quantity - whole;
    let glyph = FRACTIONS
        .iter()
        .find(|(_, value)| (value - part).abs() < 0.005)
        .map(|(glyph, _)| *glyph);
    match glyph {
        Some(glyph) if whole == 0.0 => glyph.to_string(),
        Some(glyph) => format!("{whole:.0}{glyph}"),
        None => as_decimal(quantity),
    }
}

/// A plain number with no trailing zeroes — `250`, `1.5`, never `1.50`.
fn as_decimal(quantity: f64) -> String {
    let rendered = format!("{quantity:.2}");
    rendered
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

// --- Temperatures in Step text -----------------------------------------------

/// The conventional oven ladder, never arithmetic (ADR 0016). 350°F becomes
/// 180°C because that is the number on the dial; 400°F becomes 200°C for the
/// same reason, where the arithmetic would offer 205°C and send the cook back
/// to make the judgement Kamosu was there to make.
///
/// The corpus contradicts itself here — `180C/350F` sits alongside
/// `350°F (175°C)`, four authors and two answers for one temperature — and that
/// disagreement is itself the evidence that no arithmetic result is expected.
const OVEN_LADDER: &[(i32, i32)] = &[
    (200, 95),
    (225, 110),
    (250, 120),
    (275, 140),
    (300, 150),
    (325, 160),
    (350, 180),
    (375, 190),
    (400, 200),
    (425, 220),
    (450, 230),
    (475, 245),
    (500, 260),
];

/// The other direction, and a table of its own rather than [`OVEN_LADDER`] read
/// backwards. Each rung is a setting a metric oven actually has, paired with the
/// American dial number nearest to it — so a French recipe's `170 °C` and
/// `210 °C`, neither of which appears anywhere in the American ladder, still
/// answer with a number a US cook can set.
///
/// This is still the ladder rather than the arithmetic, which is the whole of
/// ADR 0016's rule: every answer here is a real dial position, and none of them
/// is the 338°F or 410°F the conversion would otherwise hand back.
const METRIC_LADDER: &[(i32, i32)] = &[
    (90, 200),
    (100, 200),
    (110, 225),
    (120, 250),
    (130, 275),
    (140, 275),
    (150, 300),
    (160, 325),
    (170, 350),
    (180, 350),
    (190, 375),
    (200, 400),
    (210, 400),
    (220, 425),
    (230, 450),
    (240, 475),
    (250, 475),
    (260, 500),
];

/// A temperature found in a Step, and which system it was written in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scale {
    Fahrenheit,
    Celsius,
}

/// **The conversion offered beside a Step**, or nothing — an addition beside the
/// sentence and never written into it (CONTEXT.md: a Step's truth is its text).
///
/// Nothing is the answer where the step carries no temperature, where it
/// already carries both — which 20 of the 39 real steps with a temperature do —
/// and where the reader asked for no conversion.
pub fn step_temperature(text: &str, measures: Measures, language: &str) -> Option<String> {
    let wanted = match measures {
        Measures::Metric => Scale::Celsius,
        Measures::Us => Scale::Fahrenheit,
        Measures::AsWritten => return None,
    };
    let found = temperatures_in(text);
    if found.is_empty() {
        return None;
    }
    // Already carrying both: a recipe writer who dual-printed has said
    // everything there is to say, and an addition would repeat them.
    if found.iter().any(|(_, scale)| *scale == wanted) {
        return None;
    }
    // The first temperature that lands on a rung, rather than simply the first
    // number found. A French step can say `1 c. à s. d'huile ... à 180 °C`, and
    // the `1 c` there is a spoonful — the ladder is what tells the two apart,
    // because no oven is set to one degree.
    let converted = found
        .iter()
        .find_map(|(degrees, scale)| match (scale, wanted) {
            (Scale::Fahrenheit, Scale::Celsius) => ladder_from_fahrenheit(*degrees),
            (Scale::Celsius, Scale::Fahrenheit) => ladder_from_celsius(*degrees),
            _ => None,
        })?;
    let symbol = match wanted {
        Scale::Celsius => "°C",
        Scale::Fahrenheit => "°F",
    };
    Some(format!("{} {converted} {symbol}", about(language)))
}

/// Read an American dial and answer with the metric one. The rungs are 25°F
/// apart, so a temperature more than 12°F from one was never a dial setting —
/// a 700°F pizza oven is not something this table can honestly answer, and
/// silence beats a number nobody can set.
fn ladder_from_fahrenheit(degrees: i32) -> Option<i32> {
    nearest_rung(OVEN_LADDER, degrees, 12, |(fahrenheit, _)| *fahrenheit)
        .map(|(_, celsius)| *celsius)
}

/// Read a metric dial and answer with the American one, off [`METRIC_LADDER`]
/// rather than by reversing [`OVEN_LADDER`]. Reversing it leaves holes exactly
/// where French recipes live: its Celsius column is the *American* ladder
/// wearing metric numbers, and it has no 170 and no 210 — two perfectly
/// ordinary settings on an oven sold in France.
fn ladder_from_celsius(degrees: i32) -> Option<i32> {
    nearest_rung(METRIC_LADDER, degrees, 5, |(celsius, _)| *celsius)
        .map(|(_, fahrenheit)| *fahrenheit)
}

/// The rung within `tolerance` of the temperature written, or nothing.
fn nearest_rung(
    ladder: &'static [(i32, i32)],
    degrees: i32,
    tolerance: i32,
    of: impl Fn(&(i32, i32)) -> i32,
) -> Option<&'static (i32, i32)> {
    ladder
        .iter()
        .min_by_key(|rung| (of(rung) - degrees).abs())
        .filter(|rung| (of(rung) - degrees).abs() <= tolerance)
}

/// Every temperature in a Step's text, in the order they were written.
///
/// A temperature needs its letter. `turn 90 degrees` and `gas 5` both appear in
/// the real corpus and neither is a temperature, so a bare number — even one
/// followed by a degree sign — is never read as one.
fn temperatures_in(text: &str) -> Vec<(i32, Scale)> {
    let characters: Vec<char> = text.chars().collect();
    let mut found = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        if !characters[index].is_ascii_digit() {
            index += 1;
            continue;
        }
        let start = index;
        while index < characters.len() && characters[index].is_ascii_digit() {
            index += 1;
        }
        let digits: String = characters[start..index].iter().collect();
        if let Some(scale) = scale_after(&characters, index)
            && let Ok(degrees) = digits.parse::<i32>()
        {
            found.push((degrees, scale));
        }
    }
    found
}

/// What follows a number, where what follows it is a temperature marker:
/// optional spaces, an optional degree sign, then `C`, `F`, or the word
/// `degrees` followed by one of those.
fn scale_after(characters: &[char], mut index: usize) -> Option<Scale> {
    let skip_spaces = |index: &mut usize| {
        while characters.get(*index).is_some_and(|c| *c == ' ') {
            *index += 1;
        }
    };
    skip_spaces(&mut index);
    // `350 degrees F`, and the French and Spanish words for it.
    for word in ["degrees", "degree", "degrés", "degres", "grados"] {
        let letters: Vec<char> = word.chars().collect();
        if characters[index..].starts_with(&letters[..]) {
            index += letters.len();
            skip_spaces(&mut index);
            break;
        }
    }
    if characters.get(index) == Some(&'°') {
        index += 1;
        skip_spaces(&mut index);
    }
    let letter = characters.get(index)?;
    let scale = match letter {
        'C' | 'c' => Scale::Celsius,
        'F' | 'f' => Scale::Fahrenheit,
        _ => return None,
    };
    // `Celsius` and `centigrade` are the letter spelled out; `cook`, `cup` and
    // `for` are not temperatures, so the letter must not begin another word.
    let next = characters.get(index + 1);
    let ends_here = next.is_none_or(|c| !c.is_alphanumeric());
    let spelled_out = ["celsius", "centigrade", "fahrenheit"].iter().any(|word| {
        let letters: Vec<char> = word.chars().collect();
        characters[index..]
            .iter()
            .map(|c| c.to_ascii_lowercase())
            .take(letters.len())
            .eq(letters.iter().copied())
    });
    (ends_here || spelled_out).then_some(scale)
}
