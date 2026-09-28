/* THE SHAPE — one renderer, identical for all four directions.
   Carried from prototype/13-look-and-feel-attempt-2 (your direction A) and ADR 0011:
   Home is shelves that scroll sideways · search lives on Recipes, never on Home ·
   a recipe is one long page led by its photograph or its Cover ·
   cooking is one Step with the amounts for that step above it. */

const E = s => window.esc(s), HSH = s => window.hash(s);
let THEME = null;   // set by app.js before each render

const cover = (lineage, title, cls='') => {
  const [bg, ink] = THEME.covers[HSH(lineage) % THEME.covers.length];
  return `<div class="k-cov ${cls}" style="background:${bg};color:${ink}">
    <span class="k-cov-t">${E(title)}</span></div>`;
};
const art = (c, cls) => c.img ? `<img class="${cls}" src="${c.img}" alt="">` : cover(c.lineage, c.title, cls);

const sb = `<div class="k-sb"><span>9:41</span><span class="k-sb-r">▮▮▮ ▮</span></div>`;

const tabs = (t, on) => `<nav class="k-tabs">${
  [['tabHome','⌂'],['tabRecipes','☰'],['tabShopping','⬚'],['tabCooked','✓']].map(([k,g])=>
    `<a class="${k===on?'on':''}"><span class="k-tg">${g}</span>${E(t[k])}</a>`).join('')}</nav>`;

/* ------------------------------------------------------------------ home */
const tile = (c, t) => `<article class="k-tile">
  <div class="k-tile-i">${art(c,'k-tile-img')}${c.tag?`<span class="k-tag">${E(c.tag)}</span>`:''}</div>
  <h3>${E(c.title)}</h3>
  <p>${c.mins ? E(t.mins(c.mins)) : '·'} · ${c.cooked ? E(t.cookedTimes(c.cooked)) : E(t.neverCooked)}</p>
</article>`;

const home = (t) => `${sb}
  <header class="k-top">
    <span class="k-mark">${THEME.tileMark}</span>
    <span class="k-wordmark">Kamosu</span>
    <button class="k-kitchen">${E(t.kitchen)} ⌄</button>
  </header>
  <button class="k-resume">
    <span class="k-resume-k">${E(t.stillCooking)}</span>
    <span class="k-resume-t">Katsu Curry · ${E(t.stepOf(4,11))}</span>
    <span class="k-resume-go">${E(t.pickUp)}</span>
  </button>
  ${SHELVES.map(s=>`
    <section class="k-shelf">
      <div class="k-shelf-h"><h2>${E(t[s.key])}</h2><p>${E(t[s.blurb])}</p></div>
      <div class="k-scroller">${s.cards.map(c=>tile(c,t)).join('')}<span class="k-scroll-end"></span></div>
    </section>`).join('')}
  <button class="k-allbtn">${E(t.browseAll(86))}</button>
  ${tabs(t,'tabHome')}`;

/* --------------------------------------------------------------- recipes */
const recipes = (t) => `${sb}
  <header class="k-top k-top-plain">
    <span class="k-wordmark">${E(t.tabRecipes)}</span>
    <span class="k-kitchen k-kitchen-flat">${E(t.recipesCount(86))}</span>
  </header>
  <div class="k-searchwrap"><div class="k-search">${E(t.search)}</div></div>
  <div class="k-sortrow"><span class="k-sort on">${E(t.sortRecent)}</span><span class="k-sort">${E(t.sortAZ)}</span><span class="k-sort">${E(t.sortAdded)}</span></div>
  <div class="k-list">${LIBRARY.map(c=>`
    <article class="k-card">
      <div class="k-card-i">${art(c,'k-card-img')}</div>
      <div class="k-card-b">
        <h2>${E(c.title)}</h2>
        <p class="k-card-meta">${c.mins ? E(t.mins(c.mins)) : '·'} · ${c.cooked ? E(t.cookedTimes(c.cooked)) : E(t.neverCooked)}</p>
        ${c.tag?`<span class="k-badge">${E(c.tag)}</span>`:''}
      </div>
    </article>`).join('')}</div>
  ${tabs(t,'tabRecipes')}`;

/* ---------------------------------------------------------------- recipe */
const recipe = (r, t) => {
  const ings = r.ingredients.map(i=>`
    <li class="k-ing${i.reading?.component?' k-ing-comp':''}">
      <span class="k-ing-line">${E(i.line)}</span>
      ${i.reading ? `<span class="k-ing-read">${E(i.reading.amt)} · ${E(i.reading.food)}${i.reading.component?' ↗':''}</span>` : ''}
    </li>`).join('');
  let n = 0;
  const steps = r.steps.map(s => s.section
    ? `<li class="k-sec">${E(s.section)}</li>`
    : `<li class="k-step"><span class="k-step-n">${++n}</span><p>${E(s.t)}</p></li>`).join('');

  return `${sb}
  <div class="k-hero">${r.img ? `<img src="${r.img}" alt="">` : cover(r.lineage, r.title, 'k-cov-hero')}<span class="k-back">←</span></div>
  <div class="k-sheet">
    <p class="k-eyebrow">${E(t.from)} ${E(r.source)}</p>
    <h1 class="k-title">${E(r.title)}</h1>
    <div class="k-meta">
      <div><b>${r.prep}</b><span>${E(t.minPrep)}</span></div>
      <div><b>${r.cook}</b><span>${E(t.minCook)}</span></div>
      <div><b>${r.serves}</b><span>${E(t.servingsL)}</span></div>
    </div>
    <div class="k-scalerow"><span>${E(t.scale)}</span><button>−</button><b>${E(t.servingsN(r.serves))}</b><button>+</button></div>
    <h3 class="k-h">${E(t.ingredients)}</h3>
    <ul class="k-ings">${ings}</ul>
    <div class="k-note">${E(r.note)}</div>
    <h3 class="k-h">${E(t.method)}</h3>
    <ol class="k-steps">${steps}</ol>
    <h3 class="k-h">${E(t.cookedH)}</h3>
    <p class="k-hist">${E(t.timesLast(r.attempts, r.lastCooked))}</p>
  </div>
  <div class="k-cta"><button>${E(t.startCooking)}</button></div>
  ${tabs(t,'tabRecipes')}`;
};

/* ------------------------------------------------------------------ cook */
const cook = (c, t) => {
  const r = c.recipe, s = r.steps[c.step], shown = c.step < 8 ? c.step+1 : c.step;
  return `${sb}
  <div class="k-cook">
    <div class="k-noren" aria-hidden="true">${
      Array.from({length:c.n},(_,k)=>`<i class="${k<shown-1?'past':k===shown-1?'now':''}"></i>`).join('')}</div>
    <header class="k-cook-top">
      <span class="k-quit">${E(t.pause)}</span>
      <span class="k-cook-title">Katsu Curry · ${E(t.servingsN(4))}</span>
      <span class="k-cook-count">${shown}<i>/${c.n}</i></span>
    </header>
    ${c.section ? `<p class="k-cook-sec">${E(c.section)}</p>` : ''}
    <div class="k-needbox">
      <p class="k-need-h">${E(t.forThisStep)}</p>
      ${c.amounts.length
        ? `<div class="k-needs">${c.amounts.map(a=>
            `<span class="k-need${a.component?' k-need-c':''}"><b>${E(a.amt)}</b><span>${E(a.food)}</span>${
              a.component?`<em>${E(t.component)}</em>`:''}</span>`).join('')}</div>`
        : `<p class="k-need-none">${E(t.nothingNew)}</p>`}
    </div>
    <div class="k-cook-body">
      <p class="k-cook-step">${E(s.t)}</p>
      ${c.timer ? `<button class="k-timerstart">⏱ ${E(t.timer(c.timer))}</button>` : ''}
    </div>
    <div class="k-cook-foot">
      <button class="k-prevbtn">${E(t.back)}</button>
      <button class="k-nextbtn">${E(t.nextStep)}</button>
    </div>
    <p class="k-awake">${E(t.awake)}</p>
  </div>`;
};

const SHAPE = {
  setTheme(th){ THEME = th; },
  home, recipes,
  recipe:  t => recipe(KATSU, t),
  cover:   t => recipe(COQ, t),
  cookAmounts: t => cook(COOK.amounts, t),
  cookEmpty:   t => cook(COOK.empty, t),
  cookPlate:   t => cook(COOK.plate, t),
};
