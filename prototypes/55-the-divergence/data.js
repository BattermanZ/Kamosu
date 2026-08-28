// PROTOTYPE — throwaway. Issue #55: the switch, and how a Ghost reads.
//
// One Lineage (Korean Fried Chicken), two Branches, one Branch Point. The base
// recipe and every line of it are REAL — lifted from Aurélien's own Crouton
// export (samples/crouton/recipes/Korean Fried Chicken{,-1}.crumb) by way of the
// #18 prototype. Marc's Branch is invented, but invented to be plausible: a flat
// with an air fryer and less of a sweet tooth.
//
// NOTHING here carries a line id (ADR 0019). A line is its text and its position,
// full stop. Which line matches which is worked out in pair.js by reading the two
// Branches against the Branch Point — see that file.

export const ME = 'Aurélien';
export const MY_KITCHEN = 'Maison Batterman';
export const THEM = 'Marc';
export const THEIR_KITCHEN = 'Chez Marc';

// ---- how a line is written ----------------------------------------------
// Three kinds, and the kind is part of the identity: a section heading never
// pairs with an ingredient, and a step never pairs with either.

const ing = (t, reading = null) => ({ kind: 'ing', t, reading });
const sec = (t) => ({ kind: 'sec', t });
const step = (t) => ({ kind: 'step', t });

// ---- the Branch Point ----------------------------------------------------
// The last Version the two Branches share: Aurélien's 14 March save, where he
// split the list into sections. Everything either of them did afterwards is read
// against this.

export const BRANCH_POINT = {
  id: 'c07e42',
  when: '14 Mar 2026',
  ingredients: [
    sec('Chicken'),
    ing('1.4 kg whole chicken', '1.4 kg whole chicken'),
    ing('2 Tbsp rice wine', '2 Tbsp rice wine'),
    ing('2 tsp minced ginger', '2 tsp minced ginger'),
    ing('1 tsp fine sea salt', '1 tsp fine sea salt'),
    ing('½ tsp ground black pepper', '0.5 tsp black pepper'),
    ing('1 cup potato starch (or corn starch)', '1 cup potato starch'),
    ing('Some cooking oil (for deep frying)', null),
    sec('Sauce'),
    ing('3 Tbsp Ketchup', '3 Tbsp ketchup'),
    ing('2 Tbsp gochujang, add ½ Tbsp more to make it spicier (Korean chilli paste)', '2 Tbsp gochujang'),
    ing('¼ cup honey', '0.25 cup honey'),
    ing('¼ cup brown sugar', '0.25 cup brown sugar'),
    ing('2 Tbsp soy sauce', '2 Tbsp soy sauce'),
    ing('2 Tbsp minced garlic', '2 Tbsp minced garlic'),
    ing('1 Tbsp sesame oil', '1 Tbsp sesame oil'),
    sec('Optional toppings'),
    ing('toasted sesame seeds', null),
    ing('green onion (finely chopped or thinly shredded)', null),
  ],
  steps: [
    step(
      'In a bowl, place the chicken, rice wine, ginger, salt and black pepper. Combine them well. Then evenly coat the chicken with the starch and set aside. (To get the effect like the picture, dip the individual chicken pieces into the bowl of starch, roll the chicken around a bit then take them out and set aside.)'
    ),
    step(
      'In a deep saucepan (or fryer) add a generous amount of oil and heat it until the oil temperature reaches 175 C / 347 F. Start adding the battered chicken carefully and fry them until they cook (between 3 to 5 mins, depending on the size of chicken). Do not overcrowd the pan. Take out the done chicken and place them onto some kitchen paper while frying the remaining pieces. Then deep fry the chicken again once the oil is back up to temperature, until the batter is golden and crisp (2 to 3 mins). Set aside.'
    ),
    step(
      'In a separate saucepan, add in the Korean fried chicken sauce ingredients. Heat the sauce over low medium heat and stir well. Once it starts bubbling, remove the pan from the heat. Place the double fried chicken into a large mixing bowl then pour the sauce over the chicken to coat. Mix them lightly and thoroughly. Serve hot immediately.'
    ),
  ],
  note: null,
};

// ---- my Branch's head ----------------------------------------------------
// Three things happened on my side since the Branch Point:
//   · the oil line got a parenthetical back
//   · the second fry went up to 190
//   · rice vinegar went into the sauce   ← a line Marc has no counterpart for
// And one thing that must NOT read as a change: `2 Tbsp minced garlic` was
// select-all-deleted and retyped, character for character. ADR 0019's whole
// argument rests on this producing nothing.

export const MINE = {
  id: 'a1b8d0',
  who: ME,
  kitchen: MY_KITCHEN,
  when: '9 Aug 2026',
  name: 'Hotter second fry',
  changed: 'Second fry at 190 instead of 175 — the coating was going soft by the time it got to the table',
  ingredients: [
    sec('Chicken'),
    ing('1.4 kg whole chicken', '1.4 kg whole chicken'),
    ing('2 Tbsp rice wine', '2 Tbsp rice wine'),
    ing('2 tsp minced ginger', '2 tsp minced ginger'),
    ing('1 tsp fine sea salt', '1 tsp fine sea salt'),
    ing('½ tsp ground black pepper', '0.5 tsp black pepper'),
    ing('1 cup potato starch (or corn starch)', '1 cup potato starch'),
    ing('Some cooking oil (for deep frying — rice bran is best)', null),
    sec('Sauce'),
    ing('3 Tbsp Ketchup', '3 Tbsp ketchup'),
    ing('2 Tbsp gochujang, add ½ Tbsp more to make it spicier (Korean chilli paste)', '2 Tbsp gochujang'),
    ing('¼ cup honey', '0.25 cup honey'),
    ing('¼ cup brown sugar', '0.25 cup brown sugar'),
    ing('2 Tbsp soy sauce', '2 Tbsp soy sauce'),
    ing('2 Tbsp minced garlic', '2 Tbsp minced garlic'), // retyped, identical on purpose
    ing('1 Tbsp rice vinegar', '1 Tbsp rice vinegar'), //   new here, nothing of Marc's to pair with
    ing('1 Tbsp sesame oil', '1 Tbsp sesame oil'),
    sec('Optional toppings'),
    ing('toasted sesame seeds', null),
    ing('green onion (finely chopped or thinly shredded)', null),
  ],
  steps: [
    BRANCH_POINT.steps[0],
    step(
      'In a deep saucepan (or fryer) add a generous amount of oil and heat it until the oil temperature reaches 190 C / 374 F. Start adding the battered chicken carefully and fry them until they cook (between 3 to 5 mins, depending on the size of chicken). Do not overcrowd the pan. Take out the done chicken and place them onto some kitchen paper while frying the remaining pieces. Then deep fry the chicken again once the oil is back up to temperature, until the batter is golden and crisp (2 to 3 mins). Set aside.'
    ),
    BRANCH_POINT.steps[2],
  ],
  note: null,
};

// ---- Marc's Branch's head ------------------------------------------------
// Built to exercise every kind of change at once: a quantity altered, a line
// added, a line REMOVED, a step rewritten past recognition, a step inserted,
// and a note appearing where there was none.

export const THEIRS = {
  id: '5d4409',
  who: THEM,
  kitchen: THEIR_KITCHEN,
  when: '28 Jul 2026',
  arrived: '14 Aug 2026',
  name: null,
  changed: 'Cut the sugar right down and put chilli flakes in the sauce',
  ingredients: [
    sec('Chicken'),
    ing('1.4 kg whole chicken', '1.4 kg whole chicken'),
    ing('2 Tbsp rice wine', '2 Tbsp rice wine'),
    ing('2 tsp minced ginger', '2 tsp minced ginger'),
    ing('1 tsp fine sea salt', '1 tsp fine sea salt'),
    ing('½ tsp ground black pepper', '0.5 tsp black pepper'),
    ing('¾ cup potato starch', '0.75 cup potato starch'), //     changed from 1 cup
    ing('2 Tbsp plain flour', '2 Tbsp plain flour'), //          new on his side
    ing('neutral oil, in a spray bottle', null), //              changed, and far from mine
    sec('Sauce'),
    ing('3 Tbsp Ketchup', '3 Tbsp ketchup'),
    ing('2 Tbsp gochujang, add ½ Tbsp more to make it spicier (Korean chilli paste)', '2 Tbsp gochujang'),
    ing('¼ cup honey', '0.25 cup honey'),
    // ¼ cup brown sugar is gone — a Ghost when you stand in Marc's recipe
    ing('2 Tbsp soy sauce', '2 Tbsp soy sauce'),
    ing('2 Tbsp minced garlic', '2 Tbsp minced garlic'),
    ing('1 tsp gochugaru', '1 tsp gochugaru'), //                new here, and it lands right where
    ing('1 Tbsp sesame oil', '1 Tbsp sesame oil'), //            my rice vinegar landed
    sec('Optional toppings'),
    ing('toasted sesame seeds', null),
    ing('green onion (finely chopped or thinly shredded)', null),
  ],
  steps: [
    BRANCH_POINT.steps[0],
    step(
      'Spray the basket and the chicken all over. Air fry at 200 C for 18 minutes, shaking the basket twice. Spray again and give it 4 more minutes if it is not properly gold.'
    ),
    step('Let it sit 5 minutes before saucing, or the coating goes straight to mush.'),
    BRANCH_POINT.steps[2],
  ],
  note: 'Brown sugar is gone entirely — the honey is plenty. Do not skip the rest.',
};

export const RECIPE = {
  title: 'Korean Fried Chicken',
  source: 'mykoreankitchen.com',
  prep: 20,
  cook: 30,
  serves: 4,
};

export const esc = (s) =>
  String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
