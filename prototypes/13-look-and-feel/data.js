// PROTOTYPE — throwaway. Issue #13: what Kamosu looks and feels like.
// Real content, lifted from Aurélien's own 86-recipe Crouton export (samples/crouton/).
// Ingredient Lines are the originals; Readings sit over them (ADR 0002).

export const KITCHEN = 'Maison Batterman';
export const ME = 'Aurélien';

export const CUTLETS = {
  id: 'cutlets',
  title: 'Chicken Katsu Cutlets',
  yieldAmount: 4,
  yieldNoun: 'cutlets',
  ingredients: [
    { line: '2 chicken breasts, butterflied', reading: { qty: 2, unit: '', food: 'chicken breast' } },
    { line: '50g plain flour', reading: { qty: 50, unit: 'g', food: 'plain flour' } },
    { line: '1 egg, beaten', reading: { qty: 1, unit: '', food: 'egg' } },
    { line: '80g panko breadcrumbs', reading: { qty: 80, unit: 'g', food: 'panko' } },
    { line: 'oil, for frying', reading: { qty: null, unit: '', food: 'oil' } },
  ],
  steps: [
    { text: 'Season the chicken, then dredge in flour, egg and panko in that order.' },
    { text: 'Shallow-fry 4 minutes a side over medium heat until deep gold.' },
    { text: 'Rest on a rack for 2 minutes, then slice into 1" strips.' },
  ],
};

export const KATSU = {
  id: 'katsu',
  title: 'Katsu Curry',
  subtitle: 'Japanese curry rice with chicken cutlet',
  photo: 'img/katsu.jpg',
  lang: 'English',
  kitchen: KITCHEN,
  visibility: 'Maison Batterman',
  source: 'japan.recipetineats.com',
  sourceUrl: 'https://japan.recipetineats.com/katsu-curry-japanese-curry-rice-with-chicken-cutlet/',
  prep: 5,
  cook: 20,
  yieldAmount: 4,
  yieldNoun: 'servings',
  tags: ['japanese', 'weeknight', 'curry'],
  note: 'The roux thickens a lot as it cools — pull it off the heat looser than you want it.',
  nutrition: '640 kcal per serving',
  version: { name: 'Extra cube of roux', when: '12 Aug 2026', changed: '1 line changed', branchOf: 'your branch' },
  translation: { lang: 'Français', behind: 2 },
  ingredients: [
    { line: '400g / 0.9lb onion (sliced into 1cm / ⅜" wide pieces)', reading: { qty: 400, unit: 'g', food: 'onion' } },
    { line: '250g / 0.6lb potato (cut into 1.5cm / ⅝" cubes)', reading: { qty: 250, unit: 'g', food: 'potato' } },
    { line: '100g / 3.5oz carrot (sliced to 7mm / ¼" thick pieces, note 1)', reading: { qty: 100, unit: 'g', food: 'carrot' } },
    { line: '1 tbsp oil', reading: { qty: 1, unit: 'tbsp', food: 'oil' } },
    { line: '230g / 0.5lb House Vermont Curry (Mild, note 2)', reading: { qty: 230, unit: 'g', food: 'Japanese curry roux' } },
    { line: '800ml / 1.7pt water', reading: { qty: 800, unit: 'ml', food: 'water' } },
    { line: '4 cups cooked rice, hot', reading: { qty: 4, unit: 'cups', food: 'cooked rice' } },
    { line: '4 Chicken Cutlets (cut into 2.5cm / 1" wide strips, note 3)', reading: { qty: 4, unit: '', recipe: CUTLETS } },
    { line: 'fukujinzuke, to serve', reading: null },
  ],
  steps: [
    { text: 'Add oil to a pot and heat over medium high heat.', uses: [3] },
    { text: 'Add onion and sauté for a few minutes or until the onion becomes translucent and edges start getting slightly burnt.', uses: [0] },
    { text: 'Add potatoes and carrots into the pot and stir for a couple of minutes or until the surface of the vegetables starts getting cooked.', uses: [1, 2] },
    { text: 'Add water and turn the heat up to bring it to a boil. Then reduce the heat to medium low and simmer for about 7 minutes or until the vegetables are nearly cooked through.', uses: [5] },
    { text: 'Break the curry roux cake into small blocks along the lines and add them into the pot. Stir gently to blend the curry roux.', uses: [4] },
    { text: 'Reduce the heat to low, place a lid on and cook for about 10 minutes or until the curry roux is completely dissolved. Stir occasionally as the curry tends to stick to the bottom of the pot.' },
    { text: "Check the consistency of the sauce. It should be like béchamel sauce. If it's too thick, adjust with some water. If too thin, cook further without the lid. It will thicken when cooled down as well." },
    { text: 'Turn the heat off.' },
    { section: 'Serving' },
    { text: 'Place a cup of hot cooked rice onto one side of a plate. Place the chicken cutlet pieces next to the rice, leaning them on the rice so that there will be a space to pour the curry.', uses: [6, 7] },
    { text: 'Pour curry next to the chicken cutlet, put fukujinzuke on the side and serve immediately.', uses: [8] },
  ],
  attempts: [
    { who: 'Aurélien', when: '12 Aug 2026', rating: 4, note: 'Added one extra cube of curry. Add some spices to the chicken next time?', asCooked: true },
    { who: 'Camille', when: '14 Feb 2026', rating: 3, note: 'Too thick. Needed a good splash more water at the end.' },
    { who: 'Aurélien', when: '3 May 2026', rating: 5, note: '' },
    { who: 'Aurélien', when: '2 Jan 2026', rating: null, note: '' },
  ],
};

export const LIBRARY = [
  { id: 'katsu', title: 'Katsu Curry', photo: 'img/katsu.jpg', lang: null, mins: 25, cooks: 4, last: '12 Aug', tags: ['japanese'] },
  { id: 'moules', title: 'Moules Marinières', photo: 'img/moules.jpg', lang: null, mins: 30, cooks: 2, last: '4 Jul', tags: ['french'] },
  { id: 'iles', title: 'Îles Flottantes', photo: 'img/iles.jpg', lang: 'Français', mins: 40, cooks: 1, last: '25 Dec', tags: ['dessert'] },
  { id: 'kfc', title: 'Korean Fried Chicken', photo: 'img/kfc.jpg', lang: null, mins: 55, cooks: 6, last: '9 Aug', tags: ['korean'], branches: 2 },
  { id: 'dandan', title: 'Dan Dan Noodles', photo: 'img/dandan.jpg', lang: null, mins: 35, cooks: 3, last: '30 Jul', tags: ['sichuan'] },
  { id: 'meringue', title: 'Meringue', photo: 'img/meringue.jpg', lang: 'Français', mins: 40, cooks: 0, last: null, tags: ['dessert'] },
  { id: 'tatin', title: 'French Onion Soup Tarte Tatin', photo: 'img/tatin.jpg', lang: null, mins: 90, cooks: 1, last: '2 Feb', tags: ['french'] },
  { id: 'ribs', title: 'Beef Short Ribs', photo: 'img/ribs.jpg', lang: null, mins: 240, cooks: 2, last: '18 Jan', tags: ['slow'] },
  { id: 'gateau', title: 'Gâteau au Chocolat', photo: 'img/gateau.jpg', lang: 'Français', mins: 45, cooks: 5, last: '11 Aug', tags: ['dessert'] },
  { id: 'dough', title: 'Easy Homemade Pizza Dough', photo: 'img/dough.jpg', lang: null, mins: 120, cooks: 8, last: '16 Aug', tags: ['bread'] },
  { id: 'ramen', title: 'Cheese Ramen', photo: 'img/ramen.jpg', lang: null, mins: 15, cooks: 3, last: '21 Jul', tags: ['quick'] },
  { id: 'puree', title: 'Purée de Pommes de Terre', photo: 'img/puree.jpg', lang: 'Français', mins: 30, cooks: 4, last: '6 Aug', tags: ['side'] },
];

// Helpers shared by every variant — data, not layout.
export const cookedSteps = (r) => r.steps.filter((s) => !s.section);

export function scaleLine(ing, factor) {
  if (!ing.reading || ing.reading.qty == null || factor === 1) return null;
  const v = ing.reading.qty * factor;
  const n = v % 1 === 0 ? v : Math.round(v * 100) / 100;
  return `${n}${ing.reading.unit ? ' ' + ing.reading.unit : ''}`;
}

export function readingLabel(ing) {
  if (!ing.reading) return null;
  const { qty, unit, food, recipe } = ing.reading;
  const what = recipe ? recipe.title : food;
  const amount = qty == null ? '' : `${qty}${unit ? ' ' + unit : ''} `;
  return `${amount}${what}`;
}

export const stars = (n) => (n == null ? '' : '★'.repeat(n) + '☆'.repeat(5 - n));
