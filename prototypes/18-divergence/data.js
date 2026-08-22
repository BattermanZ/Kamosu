// PROTOTYPE — throwaway. Issue #18: what divergence looks like.
//
// One Lineage (Korean Fried Chicken), two Branches. The base recipe, the two
// Crouton saves and every ingredient line are REAL — lifted from Aurélien's own
// export (samples/crouton/recipes/Korean Fried Chicken{,-1}.crumb), which is one
// of the three hand-made duplicate pairs #5 found. Marc's branch is invented, but
// invented to be plausible: someone with an air fryer and less of a sweet tooth.
//
// Vocabulary is CONTEXT.md's: Lineage, Version, Branch, Branch Point, Attempt.

export const ME = 'Aurélien';
export const MY_KITCHEN = 'Maison Batterman';
export const THEM = 'Marc';
export const THEIR_KITCHEN = 'Chez Marc';

// ---- ingredient lines ----------------------------------------------------
// `key` is the prototype's cheat: it pairs a line across two Versions so a change
// reads as "changed" rather than "removed, then added". Whether Kamosu can do this
// for real is an open question this prototype exists to raise — see the README.

const L = (key, line, reading) => ({ key, line, reading });

const CHICKEN_BASE = [
  { section: 'Chicken' },
  L('chicken', '1.4 kg whole chicken', '1.4 kg whole chicken'),
  L('winw', '2 Tbsp rice wine', '2 Tbsp rice wine'),
  L('ginger', '2 tsp minced ginger', '2 tsp minced ginger'),
  L('salt', '1 tsp fine sea salt', '1 tsp fine sea salt'),
  L('pepper', '½ tsp ground black pepper', '0.5 tsp black pepper'),
];

const SAUCE_TAIL = [
  L('gochujang', '2 Tbsp gochujang, add ½ Tbsp more to make it spicier (Korean chilli paste)', '2 Tbsp gochujang'),
  L('honey', '¼ cup honey', '0.25 cup honey'),
  L('soy', '2 Tbsp soy sauce', '2 Tbsp soy sauce'),
  L('garlic', '2 Tbsp minced garlic', '2 Tbsp minced garlic'),
  L('sesameoil', '1 Tbsp sesame oil', '1 Tbsp sesame oil'),
  { section: 'Optional toppings' },
  L('seeds', 'toasted sesame seeds', null),
  L('onion', 'green onion (finely chopped or thinly shredded)', null),
];

const STEP_MARINATE = {
  key: 'marinate',
  text: 'In a bowl, place the chicken, rice wine, ginger, salt and black pepper. Combine them well. Then evenly coat the chicken with the starch and set aside. (To get the effect like the picture, dip the individual chicken pieces into the bowl of starch, roll the chicken around a bit then take them out and set aside.)',
};

const STEP_SAUCE = {
  key: 'sauce',
  text: 'In a separate saucepan, add in the Korean fried chicken sauce ingredients. Heat the sauce over low medium heat and stir well. Once it starts bubbling, remove the pan from the heat. Place the double fried chicken into a large mixing bowl then pour the sauce over the chicken to coat. Mix them lightly and thoroughly. Serve hot immediately.',
};

const fryStep = (temp) => ({
  key: 'fry',
  text: `In a deep saucepan (or fryer) add a generous amount of oil and heat it until the oil temperature reaches ${temp}. Start adding the battered chicken carefully and fry them until they cook (between 3 to 5 mins, depending on the size of chicken). Do not overcrowd the pan. Take out the done chicken and place them onto some kitchen paper while frying the remaining pieces. Then deep fry the chicken again once the oil is back up to temperature, until the batter is golden and crisp (2 to 3 mins). Set aside.`,
});

// ---- the Versions --------------------------------------------------------
// Fingerprints stand in for the real content hash. They are shown on screen in
// exactly one place on purpose — see the README.

export const VERSIONS = {
  // --- shared trunk ---
  v1: {
    id: '3f9c1a',
    who: ME,
    when: '3 Mar 2026',
    name: null,
    changed: 'Imported from mykoreankitchen.com',
    parent: null,
    branch: 'mine',
    ingredients: [
      L('chicken', '1.4 kg whole chicken', '1.4 kg whole chicken'),
      L('winw', '2 Tbsp rice wine', '2 Tbsp rice wine'),
      L('ginger', '2 tsp minced ginger', '2 tsp minced ginger'),
      L('salt', '1 tsp fine sea salt', '1 tsp fine sea salt'),
      L('pepper', '½ tsp ground black pepper', '0.5 tsp black pepper'),
      L('starch', '1 cup potato starch (or corn starch)', '1 cup potato starch'),
      L('oil', 'Some cooking oil (for deep frying (I used rice bran oil))', null),
      L('ketchup', '3 Tbsp tomato sauce (/ ketchup)', '3 Tbsp ketchup'),
      ...SAUCE_TAIL.filter((x) => !x.section),
    ],
    steps: [STEP_MARINATE, fryStep('175 C / 347 F'), STEP_SAUCE],
    note: null,
  },

  v2: {
    id: 'c07e42',
    who: ME,
    when: '14 Mar 2026',
    name: null,
    changed: 'Split it into sections so I stop scrolling past the sauce',
    parent: 'v1',
    branch: 'mine',
    ingredients: [
      ...CHICKEN_BASE,
      L('starch', '1 cup potato starch (or corn starch)', '1 cup potato starch'),
      L('oil', 'Some cooking oil (for deep frying)', null),
      { section: 'Sauce' },
      L('ketchup', '3 Tbsp Ketchup', '3 Tbsp ketchup'),
      ...SAUCE_TAIL,
    ],
    steps: [STEP_MARINATE, fryStep('175 C / 347 F'), STEP_SAUCE],
    note: null,
  },

  // --- my branch, after the branch point ---
  v3: {
    id: 'a1b8d0',
    who: ME,
    when: '9 Aug 2026',
    name: 'Hotter second fry',
    changed: 'Second fry at 190 instead of 175 — the coating was going soft by the time it got to the table',
    parent: 'v2',
    branch: 'mine',
    ingredients: [
      ...CHICKEN_BASE,
      L('starch', '1 cup potato starch (or corn starch)', '1 cup potato starch'),
      L('oil', 'Some cooking oil (for deep frying)', null),
      { section: 'Sauce' },
      L('ketchup', '3 Tbsp Ketchup', '3 Tbsp ketchup'),
      ...SAUCE_TAIL,
    ],
    steps: [STEP_MARINATE, fryStep('190 C / 374 F'), STEP_SAUCE],
    note: null,
  },

  // --- Marc's branch, from the same branch point ---
  m1: {
    id: '77e5b3',
    who: THEM,
    when: '2 Jun 2026',
    name: null,
    changed: 'Air fryer — no deep frying in a flat',
    parent: 'v2',
    branch: 'theirs',
    ingredients: [
      ...CHICKEN_BASE,
      L('starch', '¾ cup potato starch', '0.75 cup potato starch'),
      L('flour', '2 Tbsp plain flour', '2 Tbsp plain flour'),
      L('oil', 'neutral oil, in a spray bottle', null),
      { section: 'Sauce' },
      L('ketchup', '3 Tbsp Ketchup', '3 Tbsp ketchup'),
      ...SAUCE_TAIL,
    ],
    steps: [
      STEP_MARINATE,
      {
        key: 'fry',
        text: 'Spray the basket and the chicken all over. Air fry at 200 C for 18 minutes, shaking the basket twice. Spray again and give it 4 more minutes if it is not properly gold.',
      },
      STEP_SAUCE,
    ],
    note: null,
  },

  m2: {
    id: '5d4409',
    who: THEM,
    when: '28 Jul 2026',
    name: null,
    changed: 'Cut the sugar right down and put chilli flakes in the sauce',
    parent: 'm1',
    branch: 'theirs',
    ingredients: [
      ...CHICKEN_BASE,
      L('starch', '¾ cup potato starch', '0.75 cup potato starch'),
      L('flour', '2 Tbsp plain flour', '2 Tbsp plain flour'),
      L('oil', 'neutral oil, in a spray bottle', null),
      { section: 'Sauce' },
      L('ketchup', '3 Tbsp Ketchup', '3 Tbsp ketchup'),
      L('gochujang', '2 Tbsp gochujang, add ½ Tbsp more to make it spicier (Korean chilli paste)', '2 Tbsp gochujang'),
      L('gochugaru', '1 tsp gochugaru', '1 tsp gochugaru'),
      L('honey', '¼ cup honey', '0.25 cup honey'),
      L('soy', '2 Tbsp soy sauce', '2 Tbsp soy sauce'),
      L('garlic', '2 Tbsp minced garlic', '2 Tbsp minced garlic'),
      L('sesameoil', '1 Tbsp sesame oil', '1 Tbsp sesame oil'),
      { section: 'Optional toppings' },
      L('seeds', 'toasted sesame seeds', null),
      L('onion', 'green onion (finely chopped or thinly shredded)', null),
    ],
    steps: [
      STEP_MARINATE,
      {
        key: 'fry',
        text: 'Spray the basket and the chicken all over. Air fry at 200 C for 18 minutes, shaking the basket twice. Spray again and give it 4 more minutes if it is not properly gold.',
      },
      { key: 'rest', text: 'Let it sit 5 minutes before saucing, or the coating goes straight to mush.' },
      STEP_SAUCE,
    ],
    note: 'Brown sugar is gone entirely — the honey is plenty. Do not skip the rest.',
  },
};

// Brown sugar sits on my branch only — Marc's "cut the sugar right down" removed it.
// Added here rather than in each list so the three of mine stay identical by construction.
// m1 still has it; m2's "cut the sugar right down" is where it goes.
for (const v of ['v1', 'v2', 'v3', 'm1']) {
  const ings = VERSIONS[v].ingredients;
  const i = ings.findIndex((x) => x.key === 'honey');
  ings.splice(i + 1, 0, L('brownsugar', '¼ cup brown sugar', '0.25 cup brown sugar'));
}

export const RECIPE = {
  lineage: 'kfc',
  title: 'Korean Fried Chicken',
  photo: 'img/kfc.jpg',
  source: 'mykoreankitchen.com',
  sourceUrl: 'https://mykoreankitchen.com/korean-fried-chicken/',
  mins: 30,
  yieldAmount: 4,
  yieldNoun: 'servings',
  tags: ['korean', 'fried'],
  visibility: MY_KITCHEN,
};

export const BRANCHES = {
  mine: { id: 'mine', label: 'Yours', who: ME, kitchen: MY_KITCHEN, head: 'v3', chain: ['v1', 'v2', 'v3'] },
  theirs: { id: 'theirs', label: "Marc's", who: THEM, kitchen: THEIR_KITCHEN, head: 'm2', chain: ['v1', 'v2', 'm1', 'm2'], arrived: '14 Aug 2026' },
};

export const BRANCH_POINT = 'v2';

// ---- Attempts ------------------------------------------------------------
// Attempts belong to the Lineage, not a Branch (ADR 0005), and are labelled with
// the Version cooked — including "cooked before these diverged".

export const ATTEMPTS = [
  { who: ME, when: '9 Aug 2026', rating: 5, version: 'v3', note: 'Hotter second fry. Stayed crisp all the way through dinner.' },
  { who: ME, when: '2 May 2026', rating: 3, version: 'v2', note: 'Soggy by the time everyone sat down.' },
  { who: 'Camille', when: '18 Apr 2026', rating: 4, version: 'v2', note: '' },
  { who: ME, when: '21 Mar 2026', rating: null, version: 'v1', note: '' },
];

// ---- the other recipes on the shelf --------------------------------------

export const LIBRARY = [
  { id: 'kfc', title: 'Korean Fried Chicken', photo: 'img/kfc.jpg', mins: 55, cooks: 4, last: '9 Aug', branches: 2 },
  { id: 'katsu', title: 'Katsu Curry', photo: 'img/katsu.jpg', mins: 25, cooks: 4, last: '12 Aug', branches: 1 },
];

// ---- difference ----------------------------------------------------------
// Pairs two Versions' ingredient lines and steps by `key`, and classifies each
// row as same / changed / added / removed. Sections are compared as their own row
// so "Marc split the sauce out" is a visible change rather than silent reordering.

function pair(left, right) {
  const rows = [];
  const rk = right.map((x) => x.key ?? '§' + x.section);
  const seen = new Set();
  left.forEach((l) => {
    const k = l.key ?? '§' + l.section;
    const j = rk.indexOf(k);
    if (j === -1) {
      rows.push({ kind: 'removed', left: l, right: null, k });
    } else {
      seen.add(k);
      const r = right[j];
      const same = (l.line ?? l.text ?? l.section) === (r.line ?? r.text ?? r.section);
      rows.push({ kind: same ? 'same' : 'changed', left: l, right: r, k });
    }
  });
  right.forEach((r, j) => {
    const k = rk[j];
    if (!seen.has(k) && !left.some((l) => (l.key ?? '§' + l.section) === k)) {
      // insert near where it belongs: after the previous right-hand row we already have
      const prevK = j > 0 ? rk[j - 1] : null;
      const at = rows.findIndex((row) => row.k === prevK);
      const row = { kind: 'added', left: null, right: r, k };
      at === -1 ? rows.push(row) : rows.splice(at + 1, 0, row);
    }
  });
  return rows;
}

export function difference(aId, bId) {
  const a = VERSIONS[aId];
  const b = VERSIONS[bId];
  const ingredients = pair(a.ingredients, b.ingredients);
  const steps = pair(a.steps, b.steps);
  const noteChanged = (a.note || '') !== (b.note || '');
  const all = [...ingredients, ...steps];
  return {
    a,
    b,
    ingredients,
    steps,
    noteChanged,
    count: all.filter((r) => r.kind !== 'same').length + (noteChanged ? 1 : 0),
  };
}

export const text = (x) => (x ? x.line ?? x.text ?? x.section : '');
export const isStep = (x) => !!(x && x.text);

// A change written as a sentence, in the app's own voice rather than a diff's.
const gist = (s) => {
  const first = String(s).split(/(?<=[.!?])\s/)[0];
  return first.length > 64 ? first.slice(0, 61).trimEnd() + '…' : first;
};

export function sentence(row, whose) {
  const t = (x) => text(x);
  if (row.kind === 'added')
    return row.right.section
      ? `${whose} added a section, ${t(row.right)}`
      : isStep(row.right)
      ? `${whose} added a step — “${gist(row.right.text)}”`
      : `${whose} added ${t(row.right)}`;
  if (row.kind === 'removed')
    return row.left.section
      ? `${whose} took out the ${t(row.left)} section`
      : isStep(row.left)
      ? `${whose} took out a step — “${gist(row.left.text)}”`
      : `${whose} took out ${t(row.left)}`;
  if (isStep(row.left)) return `${whose} rewrote the step that starts “${gist(row.left.text)}”`;
  return `${whose} has ${t(row.right)} where you have ${t(row.left)}`;
}

export const stars = (n) => (n == null ? '—' : '★'.repeat(n) + '☆'.repeat(5 - n));

// The Versions of one branch, newest first.
export const chainOf = (b) => BRANCHES[b].chain.slice().reverse();

// Everything, oldest first, for the thread.
export const ALL_VERSIONS = ['v1', 'v2', 'm1', 'm2', 'v3'];
