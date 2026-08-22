// PROTOTYPE — Variant A, "Noren" — the candidate, revised after Aurélien's first pass.
// Direction: indigo-dyed cloth over unbleached paper. Quiet, tactile, one thing at a time.
// Kept: the look, and the big photography.
// Taken from C: shelves on the home screen ("cooked most" / "quick tonight" / "never cooked").
// Changed: the cooking screen now carries the amounts for the current step above the
// instruction, so the one thing you interrupt cooking to look up is already on screen.
// Parked: the indigo of the cooking screen — a colour question for later, not a shape one.
import { KATSU, LIBRARY, ME, cookedSteps, scaleLine, readingLabel, stars } from './data.js';
import { timerLeft } from './app.js';

// Durations live in the words, so this reads them rather than asking anyone to type them.
function timersIn(text) {
  const out = [];
  const re = /(\d+)\s*(hours?|hrs?|minutes?|mins?)\b/gi;
  let m;
  while ((m = re.exec(text))) {
    const n = Number(m[1]);
    const isHour = /^h/i.test(m[2]);
    out.push({ secs: n * (isHour ? 3600 : 60), label: `${n} ${isHour ? (n > 1 ? 'hours' : 'hour') : 'min'}` });
  }
  return out.slice(0, 2);
}

const esc = (s) => String(s).replace(/[&<>]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' }[c]));

export function render(state) {
  if (state.screen === 'home') return home(state);
  if (state.screen === 'recipes') return recipes(state);
  if (state.screen === 'cook') return cook(state);
  return recipe(state);
}

// Home and all-recipes are two destinations, not one long page.
function tabs(screen) {
  const t = (id, label, glyph) =>
    `<button class="a-tab${screen === id ? ' on' : ''}" data-act="${id}">
       <span class="a-tab-g">${glyph}</span><span>${label}</span>
     </button>`;
  return `<nav class="a-tabs">${t('home', 'Home', '⌂')}${t('recipes', 'Recipes', '☰')}</nav>`;
}

/* ---------------------------------------------------------------- library */
function shelf(title, blurb, ids) {
  const items = ids.map((id) => LIBRARY.find((r) => r.id === id)).filter(Boolean);
  return `
  <section class="a-shelf">
    <div class="a-shelf-h">
      <h2>${esc(title)}</h2>
      <p>${esc(blurb)}</p>
    </div>
    <div class="a-scroller">
      ${items
        .map(
          (r) => `
        <article class="a-tile" data-act="${r.id === 'katsu' ? 'open' : ''}">
          <div class="a-tile-img"><img src="${r.photo}" alt="">
            ${r.lang ? `<span class="a-tile-lang">${r.lang}</span>` : ''}
          </div>
          <h3>${esc(r.title)}</h3>
          <p>${r.mins} min${r.cooks ? ` · cooked ${r.cooks}×` : ' · never cooked'}</p>
        </article>`
        )
        .join('')}
    </div>
  </section>`;
}

function home(state) {
  const resume = state.cook.started
    ? `<button class="a-resume" data-act="resume">
         <span class="a-resume-k">Still cooking</span>
         <span class="a-resume-t">Katsu Curry · step ${state.cook.i + 1} of ${cookedSteps(KATSU).length}</span>
         <span class="a-resume-go">Pick up →</span>
       </button>`
    : '';

  return `
  <div class="a">
    <header class="a-top">
      <div class="a-mark">醸</div>
      <div class="a-wordmark">Kamosu</div>
      <div class="a-kitchen">Maison Batterman</div>
    </header>
    ${resume}

    ${shelf('Cooked most', 'The ones that earned their place', ['dough', 'kfc', 'gateau', 'katsu', 'puree', 'dandan'])}
    ${shelf('Quick tonight', 'Under 35 minutes, start to plate', ['ramen', 'katsu', 'dandan', 'puree', 'meringue'])}
    ${shelf('Never cooked', 'Saved and still waiting', ['meringue', 'tatin', 'iles', 'ribs'])}

    <button class="a-allbtn" data-act="recipes">Browse all 86 recipes →</button>
  </div>
  ${tabs('recipes' === state.screen ? 'recipes' : 'home')}`;
}

function recipes(state) {
  return `
  <div class="a">
    <header class="a-top a-top-plain">
      <div class="a-wordmark">Recipes</div>
      <div class="a-kitchen">86 · Maison Batterman</div>
    </header>
    <div class="a-searchwrap"><input class="a-search" placeholder="Search titles, ingredients, anything" aria-label="Search recipes"></div>
    <div class="a-sortrow">
      <span class="a-sort on">Recently cooked</span><span class="a-sort">A–Z</span><span class="a-sort">Added</span>
    </div>
    <div class="a-list">
      ${LIBRARY.map(
        (r) => `
        <article class="a-card" data-act="${r.id === 'katsu' ? 'open' : ''}">
          <div class="a-card-img"><img src="${r.photo}" alt=""></div>
          <div class="a-card-body">
            <h2>${esc(r.title)}</h2>
            <p class="a-card-meta">
              ${r.mins} min
              ${r.cooks ? `<span class="a-dot">·</span> cooked ${r.cooks}×` : `<span class="a-dot">·</span> never cooked`}
              ${r.last ? `<span class="a-dot">·</span> ${r.last}` : ''}
            </p>
            ${r.lang ? `<span class="a-lang">${r.lang}</span>` : ''}
            ${r.branches ? `<span class="a-lang a-lang-alt">2 branches</span>` : ''}
          </div>
        </article>`
      ).join('')}
    </div>
  </div>
  ${tabs('recipes')}`;
}

/* ----------------------------------------------------------------- recipe */
function recipe(state) {
  const r = KATSU;
  const f = state.scale;
  const mine = r.attempts.filter((a) => a.who === ME);

  const ings = r.ingredients
    .map((ing, i) => {
      const sc = scaleLine(ing, f);
      const rd = readingLabel(ing);
      const comp = ing.reading?.recipe;
      return `
      <li class="a-ing${comp ? ' a-ing-comp' : ''}">
        <span class="a-ing-line">${esc(ing.line)}</span>
        ${sc ? `<span class="a-ing-scaled">${esc(sc)}</span>` : ''}
        ${rd && !comp ? `<span class="a-ing-read">${esc(rd)}</span>` : ''}
        ${
          comp
            ? `<div class="a-comp">
                 <div class="a-comp-h">${esc(comp.title)} · unfolds here</div>
                 <ul>${comp.ingredients.map((c) => `<li>${esc(c.line)}</li>`).join('')}</ul>
               </div>`
            : ''
        }
      </li>`;
    })
    .join('');

  let n = 0;
  const steps = r.steps
    .map((s) => {
      if (s.section) return `<li class="a-sec">${esc(s.section)}</li>`;
      n += 1;
      return `<li class="a-step"><span class="a-step-n">${n}</span><p>${esc(s.text)}</p></li>`;
    })
    .join('');

  return `
  <div class="a a-recipe">
    <div class="a-hero"><img src="${r.photo}" alt=""><button class="a-back" data-act="recipes">←</button></div>
    <div class="a-sheet">
      <p class="a-eyebrow">${esc(r.source)}</p>
      <h1 class="a-title">${esc(r.title)}</h1>
      <p class="a-sub">${esc(r.subtitle)}</p>

      <div class="a-meta">
        <div><b>${r.prep}</b><span>min prep</span></div>
        <div><b>${r.cook}</b><span>min cook</span></div>
        <div><b>${state.cook.yieldAmount}</b><span>servings</span></div>
      </div>

      <div class="a-scale">
        <span>Scale</span>
        <button data-act="scale" data-d="-1" aria-label="Fewer servings">−</button>
        <b>${state.cook.yieldAmount} servings</b>
        <button data-act="scale" data-d="1" aria-label="More servings">+</button>
      </div>

      <h3 class="a-h">Ingredients</h3>
      <ul class="a-ings">${ings}</ul>

      <div class="a-note">${esc(r.note)}</div>

      <h3 class="a-h">Method</h3>
      <ol class="a-steps">${steps}</ol>

      <h3 class="a-h">Cooked</h3>
      <div class="a-hist">
        <p class="a-hist-top">${r.attempts.length} times · last ${r.attempts[0].when}</p>
        ${r.attempts
          .filter((a) => a.rating)
          .map((a) => `<p class="a-hist-row"><span>${esc(a.who)}</span><i>${stars(a.rating)}</i><em>${a.when}</em></p>`)
          .join('')}
        <p class="a-hist-note">“${esc(r.attempts[0].note)}”</p>
      </div>

      <div class="a-prov">
        <span>${esc(r.version.name)} · ${r.version.when}</span>
        <span>${esc(r.visibility)}</span>
        <span>Français translation, ${r.translation.behind} versions behind</span>
      </div>
    </div>
    <div class="a-cta"><button data-act="startCook">Start cooking</button></div>
  </div>
  ${tabs('recipes')}`;
}

/* ------------------------------------------------------------------- cook */
function cook(state) {
  const r = KATSU;
  const steps = cookedSteps(r);
  const i = state.cook.i;
  const s = steps[i];
  const last = i === steps.length - 1;

  if (state.cook.finished) return finishSheet(state);

  // Durations are read out of the step's own text — never typed beside it (#6).
  // "simmer for about 7 minutes" offers a 7-minute timer; nothing is stored.
  const durations = timersIn(s.text);
  const t = state.cook.timer;
  const left = timerLeft();

  // The change: what this step needs is on screen for the whole step, so the one
  // thing you'd otherwise break out of cooking to look up is already here.
  const needs = (s.uses || []).length
    ? (s.uses || [])
        .map((ix) => {
          const ing = r.ingredients[ix];
          const rd = ing.reading;
          const on = state.cook.ticked.has(ix);
          const amount = rd && rd.qty != null ? `${rd.qty * state.scale}${rd.unit ? ' ' + rd.unit : ''}` : '';
          const what = rd?.recipe ? rd.recipe.title : rd?.food || ing.line;
          return `<button class="a-need${on ? ' is-done' : ''}" data-act="tick" data-i="${ix}">
            ${amount ? `<b>${esc(amount)}</b>` : ''}<span>${esc(what)}</span>
          </button>`;
        })
        .join('')
    : `<p class="a-need-none">Nothing new to add — just the pot.</p>`;

  return `
  <div class="a a-cook">
    <div class="a-noren" aria-hidden="true">
      ${steps.map((_, k) => `<i class="${k < i ? 'past' : k === i ? 'now' : ''}"></i>`).join('')}
    </div>

    <header class="a-cook-top">
      <button class="a-quit" data-act="pause">Pause</button>
      <span class="a-cook-title">Katsu Curry · ${state.cook.yieldAmount} servings</span>
      <span class="a-cook-count">${i + 1}<i>/${steps.length}</i></span>
    </header>

    <div class="a-needbox">
      <p class="a-need-h">For this step</p>
      <div class="a-needs">${needs}</div>
    </div>

    <div class="a-cook-body">
      <p class="a-cook-step">${esc(s.text)}</p>
      ${
        t
          ? `<div class="a-timer${left === 0 ? ' is-up' : ''}" data-timer>
               <span class="a-timer-label">${esc(t.label)}</span>
               <span class="a-timer-count" data-timer-count>${left === 0 ? 'Time' : `${Math.floor(left / 60)}:${String(left % 60).padStart(2, '0')}`}</span>
               <button data-act="stopTimer">${left === 0 ? 'Clear' : 'Stop'}</button>
             </div>`
          : durations
              .map(
                (d) =>
                  `<button class="a-timerstart" data-act="startTimer" data-secs="${d.secs}" data-label="${esc(d.label)}">
                     ⏱ Time ${esc(d.label)}
                   </button>`
              )
              .join('')
      }
    </div>

    <div class="a-cook-foot">
      <button class="a-prevbtn" data-act="prev" ${i === 0 ? 'disabled' : ''}>Back</button>
      ${
        last
          ? `<button class="a-nextbtn a-done" data-act="finish">Finish cooking</button>`
          : `<button class="a-nextbtn" data-act="next">Next step</button>`
      }
    </div>
    <p class="a-awake">Screen stays awake</p>
  </div>`;
}

function finishSheet(state) {
  return `
  <div class="a a-cook a-fin">
    <div class="a-fin-inner">
      <p class="a-eyebrow">Katsu Curry</p>
      <h1 class="a-title">How was it?</h1>
      <p class="a-fin-sub">Cooked today by ${ME}. This is already recorded — the rating is optional.</p>
      <div class="a-rate">
        ${[1, 2, 3, 4, 5]
          .map((n) => `<button data-act="rate" data-n="${n}" class="${state.cook.rating >= n ? 'on' : ''}">★</button>`)
          .join('')}
      </div>
      <textarea data-note placeholder="Anything worth remembering next time?">${esc(state.cook.note)}</textarea>
      <button class="a-fin-save" data-act="save">Save this cooking</button>
      <button class="a-fin-bin" data-act="discard">Delete it — false start</button>
    </div>
  </div>`;
}
