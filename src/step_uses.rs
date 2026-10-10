//! **Which Ingredient Lines a Step uses**, read out of the Step's own words
//! and the Readings beside it (ADR 0011, GLOSSARY.md "Step"). Nothing is stored
//! and nobody types a link (ADR 0019): this runs on every read of a Version.
//!
//! A recipe names an ingredient in full once, in its list, and shortens it
//! from then on. *Maple syrup* is *the syrup* by the second Step and *cacao en
//! poudre* is *le cacao*, so a Step that had to repeat the whole name linked
//! almost nothing (#184). Here a run of words inside the name counts, as
//! long as it holds a word a cook would call the food by
//! ([`reading::names_nothing_alone`] is the rest) and the name's main word,
//! the thing the Food is (#212): *vinegar* stands for *rice vinegar*, and
//! *rice* does not. Three rules then keep that looseness from linking what
//! the Step never adds:
//!
//! - **The longer mention wins.** *The vegetable oil* is one mention of the
//!   vegetable oil, not also one of the sesame oil's *oil*. A match on an
//!   ingredient's whole name always stands, as it did before #184.
//! - **One food on two lines links one of them.** Soy sauce in the filling and
//!   again in the dipping sauce: a Step naming it takes the first such line no
//!   earlier Step has used, or the first line again when every one is used.
//!   Two *different* foods sharing a short word (*sugar* with brown and white
//!   sugar) are both linked, and the cook picks. Both were chosen
//!   on #184.
//! - **What comes after *until* is not going in.** *Steam until most of the
//!   water has evaporated* adds no water, so a mention after *until*,
//!   *jusqu'à* or *hasta* counts for nothing up to the next comma or full stop.
//!
//! Whole words only, as always: *rice* is never found in *price*, nor *ail* in
//! half the French language. One trailing `s` on a word is forgiven, so a line
//! read as *egg* is used by a Step that says *eggs*.

use std::ops::Range;

use serde_json::Value;

use crate::core::folded_for_search;
use crate::reading;

/// The words that open a clause describing how things should look, never what
/// goes in: *until*, *jusqu'à* (folded to two words, the first `jusqu`), and
/// *hasta*. *Till* is English's short *until*.
///
/// French and Spanish also write these for *up to* before an amount, as in
/// *ajoutez jusqu'à 100 ml d'eau*, where the water does go in. So a number
/// straight after one, past an `a`, opens no such clause.
const UNTIL: &[&str] = &["until", "till", "jusqu", "hasta"];

/// What ends such a clause: a comma, the end of a sentence, or anything that
/// sets words apart as firmly as one.
const CLAUSE_ENDS: &[char] = &[',', '.', ';', ':', '!', '?', '(', ')', '\n'];

/// One word of folded text, and whether it sits in an *until* clause.
struct Word {
    text: String,
    until: bool,
}

/// What sets two foods apart inside one name, as *salt & pepper* and
/// *thyme, basil* are written. A word that joins two foods does the same
/// ([`reading::joins_two_foods`]).
const NAME_BREAKS: &[char] = &[',', '&', '/', '+', ';', '(', ')'];

/// The folded words of `text`, split at anything that is not a letter or a
/// digit, each with the number of `breaks` that came before it.
fn split_words(text: &str, breaks: &[char]) -> Vec<(String, usize)> {
    let mut words: Vec<(String, usize)> = Vec::new();
    let mut part = 0;
    let mut current = String::new();
    for c in folded_for_search(text).chars() {
        if c.is_alphanumeric() {
            current.push(c);
            continue;
        }
        if !current.is_empty() {
            words.push((std::mem::take(&mut current), part));
        }
        if breaks.contains(&c) {
            part += 1;
        }
    }
    if !current.is_empty() {
        words.push((current, part));
    }
    words
}

/// The folded words of a Step, each marked with whether an [`UNTIL`] clause
/// holds it.
fn words_of(text: &str) -> Vec<Word> {
    // Each word, and the clause it sits in.
    let words = split_words(text, CLAUSE_ENDS);

    let mut marked = Vec::with_capacity(words.len());
    let mut until_in: Option<usize> = None;
    for (at, (text, clause)) in words.iter().enumerate() {
        if until_in.is_some_and(|open| open != *clause) {
            until_in = None;
        }
        if until_in.is_none() && UNTIL.contains(&text.as_str()) {
            let next = words[at + 1..]
                .iter()
                .find(|(word, _)| word != "a")
                .map(|(word, _)| word.as_str());
            let an_amount = next.is_some_and(|word| word.starts_with(|c: char| c.is_ascii_digit()));
            if !an_amount {
                until_in = Some(*clause);
            }
        }
        marked.push(Word {
            text: text.clone(),
            until: until_in.is_some(),
        });
    }
    marked
}

/// Two folded words are one word here if they differ by at most one trailing
/// `s` — the plural tolerance a Step has always had.
fn same_word(a: &str, b: &str) -> bool {
    a == b || a.strip_suffix('s') == Some(b) || b.strip_suffix('s') == Some(a)
}

/// A word with one trailing `s` taken off, for telling whether two lines hold
/// one food.
fn singular(word: &str) -> &str {
    word.strip_suffix('s')
        .filter(|w| !w.is_empty())
        .unwrap_or(word)
}

/// One Ingredient Line a Step could name: its index, every run of its name's
/// folded words that may stand for it (with whether the run is the whole
/// name), and the key two lines holding the same food share.
struct Line {
    index: usize,
    runs: Vec<(Vec<String>, bool)>,
    food: String,
}

/// The folded words of a name, and where each food in it begins and ends: a
/// name is one food unless a [`NAME_BREAKS`] mark or a word that
/// [`reading::joins_two_foods`] sets two apart, as in *fresh thyme and basil*.
/// A split made in error only loosens: it gives a name one main word more.
fn name_words(target: &str) -> (Vec<String>, Vec<Range<usize>>) {
    let mut words = Vec::new();
    let mut foods: Vec<Range<usize>> = Vec::new();
    let mut last_part = None;
    for (at, (word, part)) in split_words(target, NAME_BREAKS).into_iter().enumerate() {
        let joins = reading::joins_two_foods(&word);
        if joins || last_part != Some(part) {
            foods.push(at..at);
        }
        last_part = Some(part);
        foods.last_mut().expect("just pushed").end = at + 1;
        words.push(word);
    }
    (words, foods)
}

/// Which words of a name are its **main words**: the thing the Food is, which
/// a part of the name must hold to stand for it (#212). *Vinegar* in *rice
/// vinegar*, so a Step's *steamed rice* is no mention of it.
///
/// Which end the main word sits at is the Language's: the last word in
/// English, the first in French and Spanish (*vinaigre de riz*, *vinagre de
/// arroz*). Two things move it in English. A name written with *of* is read
/// from the front, *bicarbonate of soda* being *the bicarbonate*. And what
/// follows a word that [`reading::opens_a_tail`] is no part of the name, so
/// *oil for frying* is *the oil*. A Branch with no Language, or one stated
/// Unknown, is read from both ends.
///
/// From that end, a word that [`reading::gives_way`] is a main word and so is
/// the next one in: *thighs* and *chicken* in *chicken thighs*, *hauts*,
/// *cuisse* and *poulet* in *hauts de cuisse de poulet*. Words that only join
/// are stepped over. A word that [`reading::names_its_kind`] is a main word
/// wherever it sits. A name holding two foods has the main words of each.
fn main_words(words: &[String], foods: &[Range<usize>], language: Option<&str>) -> Vec<bool> {
    let mut main: Vec<bool> = words
        .iter()
        .map(|word| reading::names_its_kind(word))
        .collect();
    let mut walk = |inward: &mut dyn Iterator<Item = usize>| {
        for at in inward {
            if reading::only_joins(&words[at]) {
                continue;
            }
            main[at] = true;
            if !reading::gives_way(&words[at]) {
                break;
            }
        }
    };
    for food in foods {
        // The food as English reads it: up to the first word that opens a tail.
        let tail = food
            .clone()
            .find(|&at| at > food.start && reading::opens_a_tail(&words[at]))
            .unwrap_or(food.end);
        let headed = food.start..tail;
        let written_with_of = words[headed.clone()].iter().any(|word| word == "of");
        match language {
            Some("fr" | "es") => walk(&mut food.clone()),
            Some("en") if written_with_of => walk(&mut headed.clone()),
            Some("en") => walk(&mut headed.rev()),
            _ => {
                walk(&mut food.clone());
                walk(&mut headed.rev());
            }
        }
    }
    main
}

/// The runs of a name's words a Step may name it by: the whole name, and
/// every shorter run that [`names_the_food`] allows and that holds one of the
/// name's [`main_words`]. Worked out once per line rather than once per Step,
/// since it depends on nothing a Step says.
fn runs_of(target: &str, language: Option<&str>) -> (Vec<String>, Vec<(Vec<String>, bool)>) {
    let (words, foods) = name_words(target);
    let n = words.len();
    let main = main_words(&words, &foods, language);
    let mut runs = Vec::new();
    for from in 0..n {
        for to in (from + 1)..=n {
            let run = &words[from..to];
            let whole = from == 0 && to == n;
            if whole || (main[from..to].contains(&true) && names_the_food(run)) {
                runs.push((run.to_vec(), whole));
            }
        }
    }
    (words, runs)
}

/// One place a Step names a line: the words `start..start + len`, and whether
/// that was the line's whole name.
#[derive(Clone, Copy)]
struct Mention {
    line: usize,
    start: usize,
    len: usize,
    whole: bool,
}

impl Mention {
    fn overlaps(&self, other: &Mention) -> bool {
        self.start < other.start + other.len && other.start < self.start + self.len
    }

    /// Whether `other` is the better reading of the words both cover: longer,
    /// or as long and the other line's whole name where this is only part of
    /// its own.
    fn beaten_by(&self, other: &Mention) -> bool {
        other.len > self.len || (other.len == self.len && other.whole && !self.whole)
    }
}

/// Whether a run of a name's words may stand for the whole name: it holds a
/// word a cook would call the food by, and neither begins nor ends on a word
/// that only joins two others. *Shredded coconut* may stand for *unsweetened
/// shredded coconut*; *de coco* may not stand for anything.
fn names_the_food(run: &[String]) -> bool {
    match run {
        [] => false,
        [only] => only.chars().count() >= 3 && !reading::names_nothing_alone(only),
        [first, .., last] => {
            !reading::only_joins(first)
                && !reading::only_joins(last)
                && run.iter().any(|word| !reading::names_nothing_alone(word))
        }
    }
}

/// Every place in `step` this line is named, whole or by a part of its name.
fn mentions_of(line: &Line, step: &[Word]) -> Vec<Mention> {
    let mut found = Vec::new();
    for (run, whole) in &line.runs {
        for start in 0..step.len().saturating_sub(run.len() - 1) {
            let there = &step[start..start + run.len()];
            if there
                .iter()
                .zip(run)
                .all(|(word, want)| same_word(&word.text, want))
            {
                found.push(Mention {
                    line: line.index,
                    start,
                    len: run.len(),
                    whole: *whole,
                });
            }
        }
    }
    found
}

/// **Which Ingredient Lines each Step uses**: one slot per row of `steps`,
/// `None` on a row that is not a Step, and otherwise the lines' indices in
/// order. `readings` is one slot per Ingredient Line, a Reading or null, and
/// `language` is the Branch's, which says where a name's main word sits.
pub fn step_uses(
    steps: &[Value],
    readings: &[Value],
    language: Option<&str>,
) -> Vec<Option<Vec<usize>>> {
    let lines: Vec<Line> = readings
        .iter()
        .enumerate()
        .filter_map(|(index, reading)| {
            let target = reading.get("target")?.as_str()?;
            let (words, runs) = runs_of(target, language);
            (!words.is_empty()).then(|| Line {
                index,
                food: words
                    .iter()
                    .map(|w| singular(w))
                    .collect::<Vec<_>>()
                    .join(" "),
                runs,
            })
        })
        .collect();

    let mut used_before: Vec<usize> = Vec::new();
    steps
        .iter()
        .map(|row| {
            if row["kind"] != "step" {
                return None;
            }
            let text = row["text"].as_str()?;
            let uses = uses_of(&words_of(text), &lines, &used_before);
            used_before.extend(&uses);
            Some(uses)
        })
        .collect()
}

/// The lines one Step uses, given the lines earlier Steps already used.
fn uses_of(step: &[Word], lines: &[Line], used_before: &[usize]) -> Vec<usize> {
    let mentions: Vec<Mention> = lines
        .iter()
        .flat_map(|line| mentions_of(line, step))
        .filter(|mention| !step[mention.start].until)
        .collect();
    let kept = mentions.iter().filter(|mention| {
        mention.whole
            || !mentions.iter().any(|other| {
                other.line != mention.line && mention.overlaps(other) && mention.beaten_by(other)
            })
    });

    let mut named: Vec<usize> = kept.map(|mention| mention.line).collect();
    named.sort_unstable();
    named.dedup();

    // One food on several named lines: keep the first no earlier Step used.
    let food_of = |index: usize| &lines.iter().find(|line| line.index == index).unwrap().food;
    let mut uses = Vec::new();
    for &index in &named {
        let same: Vec<usize> = named
            .iter()
            .copied()
            .filter(|&other| food_of(other) == food_of(index))
            .collect();
        let chosen = same
            .iter()
            .copied()
            .find(|line| !used_before.contains(line))
            .unwrap_or(same[0]);
        if chosen == index {
            uses.push(index);
        }
    }
    uses
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A recipe's Steps and Readings as `step_uses` takes them: every step a
    /// Step (a `#` prefix makes a Section), and every target a Reading (an
    /// empty one leaves the line unread).
    fn uses(steps: &[&str], targets: &[&str]) -> Vec<Option<Vec<usize>>> {
        uses_in(None, steps, targets)
    }

    /// The same, on a Branch written in `language`.
    fn uses_in(
        language: Option<&str>,
        steps: &[&str],
        targets: &[&str],
    ) -> Vec<Option<Vec<usize>>> {
        let steps: Vec<Value> = steps
            .iter()
            .map(|text| match text.strip_prefix('#') {
                Some(section) => json!({ "kind": "section", "text": section }),
                None => json!({ "kind": "step", "text": text }),
            })
            .collect();
        let readings: Vec<Value> = targets
            .iter()
            .map(|target| {
                if target.is_empty() {
                    Value::Null
                } else {
                    json!({ "target": target })
                }
            })
            .collect();
        step_uses(&steps, &readings, language)
    }

    fn step(indices: &[usize]) -> Option<Vec<usize>> {
        Some(indices.to_vec())
    }

    /// Chocolate coconut snack cake, as written on prod on 2026-09-27: the
    /// first case #184 reported.
    const CAKE_TARGETS: &[&str] = &[
        "eggs",
        "maple syrup",
        "Greek yogurt",
        "vanilla extract",
        "almond flour",
        "unsweetened shredded coconut",
        "cocoa powder",
        "brown sugar",
        "baking soda",
        "salt",
        "espresso powder",
    ];
    const CAKE_STEPS: &[&str] = &[
        "Preheat your oven to 350°F. Line a 7×7 inch brownie pan with parchment paper.",
        "In a large bowl, whisk together the eggs, syrup, yogurt and vanilla until smooth.",
        "Add in the almond flour, shredded coconut, cocoa powder, brown sugar, baking soda, salt and espresso powder, and mix together until incorporated.",
        "Transfer the mixture to the brownie pan and spread out evenly.",
        "Bake for 25-30 minutes or until a toothpick comes out clean.",
        "Allow to set in the pan for 15 minutes.",
        "Carefully transfer to a wire rack and allow to cool completely before slicing.",
        "Garnish with shredded coconut.",
    ];

    #[test]
    fn a_step_links_an_ingredient_by_the_short_name_the_recipe_uses_for_it() {
        assert_eq!(
            uses(CAKE_STEPS, CAKE_TARGETS),
            [
                step(&[]),
                step(&[0, 1, 2, 3]),
                step(&[4, 5, 6, 7, 8, 9, 10]),
                step(&[]),
                step(&[]),
                step(&[]),
                step(&[]),
                step(&[5]),
            ]
        );
    }

    /// The cake's French Translation.
    const FRENCH_CAKE_TARGETS: &[&str] = &[
        "œufs",
        "sirop d'érable",
        "yaourt grec",
        "extrait de vanille",
        "farine d'amande",
        "noix de coco râpée non sucrée",
        "cacao en poudre",
        "cassonade",
        "bicarbonate de soude",
        "sel",
        "espresso en poudre",
    ];
    const FRENCH_CAKE_STEPS: &[&str] = &[
        "Préchauffez le four à 350°F. Chemisez un moule carré de 7×7 pouces de papier sulfurisé.",
        "Dans un grand saladier, fouettez les œufs, le sirop, le yaourt et la vanille jusqu'à obtenir un mélange lisse.",
        "Ajoutez la farine d'amande, la noix de coco râpée, le cacao, la cassonade, le bicarbonate, le sel et l'espresso en poudre, puis mélangez jusqu'à ce que tout soit incorporé.",
        "Versez la pâte dans le moule et étalez-la uniformément.",
        "Enfournez pour 25-30 minutes, jusqu'à ce qu'un cure-dent en ressorte propre.",
        "Parsemez de noix de coco râpée.",
    ];

    #[test]
    fn a_french_step_links_le_cacao_and_le_bicarbonate() {
        assert_eq!(
            uses(FRENCH_CAKE_STEPS, FRENCH_CAKE_TARGETS),
            [
                step(&[]),
                step(&[0, 1, 2, 3]),
                step(&[4, 5, 6, 7, 8, 9, 10]),
                step(&[]),
                step(&[]),
                step(&[5]),
            ]
        );
    }

    /// No-fold crispy dumplings, as written on prod: soy sauce and spring
    /// onion each on two lines, ginger and garlic read as *fresh ginger* and
    /// *garlic clove*, and water named only in passing.
    const DUMPLING_TARGETS: &[&str] = &[
        "",
        "pork mince",
        "soy sauce",
        "sesame oil",
        "salt",
        "spring onions",
        "fresh ginger",
        "garlic clove",
        "vegetable oil",
        "gyoza wrappers",
        "spring onion",
        "",
        "cornflour",
        "plain flour",
        "water",
        "",
        "soy sauce",
        "Chinese black vinegar",
        "crispy chilli oil",
    ];
    const DUMPLING_STEPS: &[&str] = &[
        "#Make the filling",
        "In a medium bowl, mix the pork mince, soy sauce, sesame oil, salt, spring onions, ginger and garlic until the mixture is well combined and sticky. This helps ensure the filling stays juicy and tender.",
        "#Scoop and sear",
        "Heat the vegetable oil in a large (about 25 cm or 10-inch) non-stick frying pan with a lid over medium-high heat.",
        "Using a small ice cream scoop or two spoons, place scoops of the filling (about 1 tablespoon each) directly into the pan, arranging them in a circular pattern with a little space between each one.",
        "Cook for 2-3 minutes without moving them, until the bottoms are lightly browned.",
        "Quickly lay the gyoza wrappers over the top of each scoop of filling, pressing down gently so they adhere.",
        "#Make the crispy skirt",
        "In a jug, whisk together the cornflour, plain flour and water to make the slurry.",
        "Pour the slurry all over the dumplings in the pan. It should bubble up immediately.",
        "Cover with the lid, reduce the heat to low and steam for 5-7 minutes, or until the pork is cooked through and most of the water has evaporated.",
        "Remove the lid, increase the heat slightly and cook for another 2-3 minutes, until the skirt is golden and crispy. You can check by gently lifting an edge with a spatula.",
        "#The big reveal",
        "Turn off the heat and run a spatula around the edge of the pan to make sure nothing is sticking.",
        "Place a large serving plate over the top of the pan. With one hand firmly on the plate and the other on the pan handle, flip the pan over confidently and quickly.",
        "Lift the pan away to reveal the crispy dumpling base.",
        "Mix the soy sauce, vinegar and chilli oil for the dipping sauce.",
        "Drizzle over the dumplings and scatter with spring onion. Serve immediately.",
    ];

    #[test]
    fn a_food_on_two_lines_links_the_one_no_earlier_step_used() {
        assert_eq!(
            uses(DUMPLING_STEPS, DUMPLING_TARGETS),
            [
                None,
                step(&[1, 2, 3, 4, 5, 6, 7]),
                None,
                step(&[8]),
                step(&[]),
                step(&[]),
                step(&[9]),
                None,
                step(&[12, 13, 14]),
                step(&[]),
                step(&[]),
                step(&[]),
                None,
                step(&[]),
                step(&[]),
                step(&[]),
                step(&[16, 17, 18]),
                step(&[10]),
            ]
        );
    }

    /// The dumplings' French Translation.
    const FRENCH_DUMPLING_TARGETS: &[&str] = &[
        "",
        "porc haché",
        "sauce soja",
        "huile de sésame",
        "sel",
        "oignons nouveaux",
        "gingembre",
        "ail",
        "huile végétale",
        "feuilles à gyoza",
        "oignon nouveau",
        "",
        "fécule de maïs",
        "farine",
        "eau",
        "",
        "sauce soja",
        "vinaigre noir chinois",
        "chili crisp",
    ];
    const FRENCH_DUMPLING_STEPS: &[&str] = &[
        "#Préparer la farce",
        "Dans un saladier moyen, mélangez le porc haché, la sauce soja, l'huile de sésame, le sel, les oignons nouveaux, le gingembre et l'ail jusqu'à ce que le mélange soit homogène et collant. La farce reste ainsi juteuse et tendre.",
        "#Déposer et saisir",
        "Faites chauffer l'huile végétale à feu moyen-vif dans une grande poêle antiadhésive (environ 25 cm ou 10 pouces) munie d'un couvercle.",
        "À l'aide d'une petite cuillère à glace ou de deux cuillères, déposez des boules de farce (environ 1 cuillère à soupe chacune) directement dans la poêle, en cercle, en laissant un peu d'espace entre elles.",
        "Faites cuire 2-3 minutes sans les bouger, jusqu'à ce que le dessous soit légèrement doré.",
        "Posez rapidement une feuille à gyoza sur chaque boule de farce, en appuyant doucement pour qu'elle adhère.",
        "#Faire la jupe croustillante",
        "Dans un pichet, fouettez la fécule de maïs, la farine et l'eau pour obtenir l'appareil.",
        "Versez l'appareil sur tous les raviolis dans la poêle. Il doit bouillonner immédiatement.",
        "Couvrez, baissez le feu et laissez cuire à la vapeur 5-7 minutes, jusqu'à ce que le porc soit cuit et que l'essentiel de l'eau se soit évaporé.",
        "Retirez le couvercle, montez légèrement le feu et poursuivez la cuisson 2-3 minutes, jusqu'à ce que la jupe soit dorée et croustillante. Vérifiez en soulevant délicatement un bord avec une spatule.",
        "#Le démoulage",
        "Éteignez le feu et passez une spatule tout autour de la poêle pour que rien n'accroche.",
        "Posez une grande assiette de service sur la poêle. Une main bien à plat sur l'assiette, l'autre sur le manche, retournez la poêle d'un geste sûr et rapide.",
        "Soulevez la poêle pour découvrir la base croustillante.",
        "Mélangez la sauce soja, le vinaigre et le chili crisp pour la sauce.",
        "Arrosez-en les raviolis et parsemez d'oignon nouveau. Servez aussitôt.",
    ];

    #[test]
    fn the_french_dumplings_link_the_same_lines() {
        assert_eq!(
            uses(FRENCH_DUMPLING_STEPS, FRENCH_DUMPLING_TARGETS),
            [
                None,
                step(&[1, 2, 3, 4, 5, 6, 7]),
                None,
                step(&[8]),
                step(&[]),
                step(&[]),
                step(&[9]),
                None,
                step(&[12, 13, 14]),
                step(&[]),
                step(&[]),
                step(&[]),
                None,
                step(&[]),
                step(&[]),
                step(&[]),
                step(&[16, 17, 18]),
                step(&[10]),
            ]
        );
    }

    #[test]
    fn a_short_word_two_different_foods_share_links_both() {
        assert_eq!(
            uses(
                &["Stir in the sugar.", "Add the white sugar."],
                &["brown sugar", "white sugar"]
            ),
            [step(&[0, 1]), step(&[1])],
            "the choice on #184: both, and the cook picks; a longer \
             mention settles it where the Step gives one"
        );
    }

    #[test]
    fn a_whole_name_beats_a_part_of_another_name() {
        assert_eq!(
            uses(
                &["Add the sugar.", "Heat the vegetable oil."],
                &["sugar", "brown sugar", "sesame oil", "vegetable oil"]
            ),
            [step(&[0]), step(&[3])],
        );
    }

    #[test]
    fn a_food_every_line_of_which_is_used_links_the_first_again() {
        assert_eq!(
            uses(
                &[
                    "Salt the water.",
                    "Season the sauce with salt.",
                    "Salt to finish."
                ],
                &["salt", "salt", "water"]
            ),
            [step(&[0, 2]), step(&[1]), step(&[0])],
        );
    }

    #[test]
    fn a_mention_after_until_is_not_going_in_up_to_the_next_comma() {
        assert_eq!(
            uses(
                &[
                    "Cook until the onions soften, then add the garlic.",
                    "Simmer hasta que el agua se evapore.",
                ],
                &["onion", "garlic", "agua"]
            ),
            [step(&[1]), step(&[])],
        );
        assert_eq!(
            uses(
                &[
                    "Ajoutez jusqu'à 100 ml d'eau.",
                    "Añade hasta 200 ml de agua.",
                    "Laissez cuire jusqu'à ce que l'eau soit évaporée.",
                ],
                &["eau", "agua"]
            ),
            [step(&[0]), step(&[1]), step(&[])],
            "`jusqu'à` and `hasta` before an amount say *up to*, and the water \
             goes in"
        );
    }

    #[test]
    fn a_describing_word_alone_links_nothing() {
        assert_eq!(
            uses(
                &[
                    "Fry until crispy. Serve with the crispy base and fresh herbs.",
                    "Pour in the Greek dressing.",
                ],
                &["crispy chilli oil", "fresh ginger", "Greek yogurt"]
            ),
            [step(&[]), step(&[])],
        );
    }

    #[test]
    fn a_measure_or_a_form_alone_links_nothing() {
        assert_eq!(
            uses(
                &[
                    "Line a baking sheet. You can cut the bread into cubes and set aside.",
                    "Stir to a smooth paste.",
                ],
                &[
                    "baking soda",
                    "can chickpeas",
                    "beef broth cubes",
                    "tomato paste"
                ]
            ),
            [step(&[]), step(&[])],
        );
    }

    #[test]
    fn a_fragment_is_never_a_mention_and_a_step_naming_nothing_links_nothing() {
        assert_eq!(
            uses(
                &[
                    "Beat the eggs.",
                    "Check the price of the eggshell substitute.",
                    "Faire revenir l'ail.",
                    "Cover and leave it alone.",
                ],
                &["egg", "rice", "ail"]
            ),
            [step(&[0]), step(&[]), step(&[2]), step(&[])],
        );
    }

    #[test]
    fn an_english_step_links_a_food_by_its_main_word_and_not_by_a_qualifier() {
        assert_eq!(
            uses_in(
                Some("en"),
                &[
                    "Serve the chicken with steamed rice.",
                    "Add the chicken.",
                    "Shred the beef, sprinkle over almonds and add the vegetables.",
                    "Add the vinegar, then the broth.",
                ],
                &[
                    "chicken thighs",
                    "rice vinegar",
                    "chicken broth",
                    "beef stock",
                    "almond meal",
                    "vegetable oil",
                ]
            ),
            [step(&[0]), step(&[0]), step(&[]), step(&[1, 2])],
        );
    }

    #[test]
    fn a_french_or_spanish_step_links_a_food_by_the_first_word_of_its_name() {
        assert_eq!(
            uses_in(
                Some("fr"),
                &[
                    "Servez le poulet avec du riz vapeur.",
                    "Ajoutez le vinaigre et le bouillon.",
                ],
                &[
                    "hauts de cuisse de poulet",
                    "vinaigre de riz",
                    "bouillon de poulet",
                ]
            ),
            [step(&[0]), step(&[1, 2])],
        );
        assert_eq!(
            uses_in(
                Some("es"),
                &["Sirve el pollo con arroz.", "Añade el vinagre y el caldo."],
                &["muslos de pollo", "vinagre de arroz", "caldo de pollo"]
            ),
            [step(&[0]), step(&[1, 2])],
        );
    }

    #[test]
    fn a_step_still_links_a_food_it_names_without_its_cut_or_its_form() {
        let targets = [
            "chicken thighs",
            "chicken breast",
            "chicken drumsticks",
            "garlic cloves",
            "kale leaves",
            "salmon fillet",
            "pork mince",
            "vanilla extract",
            "miso paste",
            "Parmesan cheese",
            "cheddar cheese",
            "soy sauce",
            "bicarbonate of soda",
        ];
        assert_eq!(
            uses_in(
                Some("en"),
                &[
                    "Brown the chicken.",
                    "Add the garlic, kale, salmon and pork.",
                    "Stir in the vanilla and the miso.",
                    "Grate over the parmesan.",
                    "Melt the cheese.",
                    "Sear the thighs, then add the soy and the bicarbonate.",
                ],
                &targets
            ),
            [
                step(&[0, 1, 2]),
                step(&[3, 4, 5, 6]),
                step(&[7, 8]),
                step(&[9]),
                step(&[9, 10]),
                step(&[0, 12]),
            ],
            "the cut by itself still links its line; *the soy* no longer \
             links soy sauce, the one loss accepted on #212"
        );
    }

    #[test]
    fn the_recipes_of_184_link_the_same_lines_read_in_their_own_language() {
        let cases: &[(&str, &[&str], &[&str])] = &[
            ("en", CAKE_STEPS, CAKE_TARGETS),
            ("fr", FRENCH_CAKE_STEPS, FRENCH_CAKE_TARGETS),
            ("en", DUMPLING_STEPS, DUMPLING_TARGETS),
            ("fr", FRENCH_DUMPLING_STEPS, FRENCH_DUMPLING_TARGETS),
        ];
        for (language, steps, targets) in cases {
            assert_eq!(
                uses_in(Some(language), steps, targets),
                uses(steps, targets),
                "{language}: {}",
                targets[1]
            );
        }
        assert_eq!(
            uses_in(
                Some("en"),
                &["Grate the ginger."],
                &["fresh ginger", "ginger beer"]
            ),
            [step(&[0])],
        );
    }

    #[test]
    fn a_branch_with_no_language_reads_a_name_from_both_ends() {
        let steps = ["Serve with rice.", "Ajoutez le vinaigre, puis le sel."];
        let targets = ["rice vinegar", "vinaigre de riz", "sel de mer"];
        for language in [None, Some("unknown")] {
            assert_eq!(
                uses_in(language, &steps, &targets),
                [step(&[0]), step(&[1, 2])],
                "with no Language to say which end the main word sits at, \
                 either end stands for the name"
            );
        }
        assert_eq!(
            uses_in(Some("en"), &steps, &targets),
            [step(&[]), step(&[])]
        );
    }

    /// Names the reader left untidy on a development library, each of which
    /// lost a right link to the first cut of the main-word rule (#212).
    #[test]
    fn a_name_holding_two_foods_an_amount_or_a_tail_keeps_its_main_words() {
        assert_eq!(
            uses_in(
                Some("en"),
                &[
                    "Season with a pinch of salt, then the thyme.",
                    "Slice the chicken.",
                    "Add sherry and toss the panko with the oil. Serve with udon noodles.",
                ],
                &[
                    "salt & freshly ground black pepper",
                    "fresh thyme and basil",
                    "chicken drumsticks and bone-in thigh pieces",
                    "dry sherry such as Amontillado",
                    "panko bread crumbs",
                    "vegetable oil for frying",
                    "pouches ready to wok udon noodles",
                    "chicken or vegetable stock",
                ]
            ),
            [step(&[0, 1]), step(&[2]), step(&[3, 4, 5, 6])],
        );
        assert_eq!(
            uses_in(
                Some("fr"),
                &["Disposez les tomates, puis les enoki.", "Chauffez l'huile."],
                &[
                    "3 paquets d'Enoki",
                    "peu huile pour la cuisson",
                    "pâte à pizza",
                    "pâte de miso"
                ]
            ),
            [step(&[0]), step(&[1])],
        );
        assert_eq!(
            uses_in(
                Some("fr"),
                &["Garnissez la pizza.", "Délayez le miso."],
                &["pâte à pizza", "pâte de miso"]
            ),
            [step(&[]), step(&[1])],
        );
    }

    /// The whole-name rule as it stood before #184, kept here to hold the new
    /// one to its promise: every link it made is still made, except where one
    /// food on two lines now links one of them, or the mention sits after
    /// *until*.
    fn linked_before(steps: &[&str], targets: &[&str]) -> Vec<Vec<usize>> {
        steps
            .iter()
            .map(|text| {
                let text = folded_for_search(text);
                targets
                    .iter()
                    .enumerate()
                    .filter(|(_, target)| !target.is_empty())
                    .filter(|(_, target)| {
                        let target = folded_for_search(target.trim());
                        let target = target.strip_suffix('s').unwrap_or(&target);
                        text.match_indices(target).any(|(at, _)| {
                            let mut end = at + target.len();
                            if text[end..].starts_with('s') {
                                end += 1;
                            }
                            !text[..at]
                                .chars()
                                .next_back()
                                .is_some_and(char::is_alphanumeric)
                                && !text[end..]
                                    .chars()
                                    .next()
                                    .is_some_and(char::is_alphanumeric)
                        })
                    })
                    .map(|(index, _)| index)
                    .collect()
            })
            .collect()
    }

    /// One recipe's Steps and Readings, and the (Step, line) links it may lose.
    type Case<'a> = (&'a [&'a str], &'a [&'a str], &'a [(usize, usize)]);

    #[test]
    fn every_link_the_whole_name_rule_made_is_still_made() {
        // Each case, with the (Step, line) links it is allowed to lose: one
        // food on two lines now linking one of them, or a mention after
        // *until* (choices 2 and 3 on #184). Nothing else may go.
        let cases: &[Case] = &[
            (CAKE_STEPS, CAKE_TARGETS, &[]),
            (
                DUMPLING_STEPS,
                DUMPLING_TARGETS,
                &[
                    // The filling takes the filling's spring onions and soy
                    // sauce, not those to serve or for the dipping sauce.
                    (1, 10),
                    (1, 16),
                    // `until ... the water has evaporated`.
                    (10, 14),
                    // The dipping sauce's soy sauce, not the filling's.
                    (16, 2),
                    // The spring onion to serve, not the filling's.
                    (17, 5),
                ],
            ),
            (
                &[
                    "Coat the chicken breast in panko.",
                    "Pour in the water and simmer for about 7 minutes.",
                    "Cover and leave it alone.",
                ],
                &["", "chicken", "panko", "water", "", ""],
                &[],
            ),
            (
                &[
                    "Brown the minced garlic and the chopped onion in olive oil.",
                    "Add the brown sugar, the sugar and a pinch of sea salt.",
                    "Pour in the double cream and the tomatoes.",
                ],
                &[
                    "minced garlic",
                    "onion",
                    "olive oil",
                    "sugar",
                    "brown sugar",
                    "sea salt",
                    "double cream",
                    "tomates",
                ],
                &[],
            ),
        ];
        for (steps, targets, may_lose) in cases {
            let before = linked_before(steps, targets);
            let now = uses(steps, targets);
            let mut lost = Vec::new();
            for (at, (before, now)) in before.iter().zip(&now).enumerate() {
                let Some(now) = now else {
                    continue;
                };
                lost.extend(
                    before
                        .iter()
                        .filter(|line| !now.contains(line))
                        .map(|&line| (at, line)),
                );
            }
            assert_eq!(
                lost, *may_lose,
                "the (Step, line) links the whole-name rule made that are gone"
            );
        }
    }
}
