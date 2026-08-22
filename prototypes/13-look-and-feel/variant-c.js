// PROTOTYPE — Variant C, "Counter".
// Direction: warm light over a dark counter. Soft, layered, app-like, thumb-first.
// Cooking answer: THE SPLIT. What this step needs stays pinned at the top for the whole
// step; the step itself fills the middle; a filmstrip in thumb reach moves you along.
// You never scroll away from a quantity.
import { KATSU, LIBRARY, ME, cookedSteps, scaleLine, readingLabel, stars } from './data.js';

const esc = (s) => String(s).replace(/[&<>]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' }[c]));

export function render(state) {
  if (state.screen === 'home' || state.screen === 'recipes') return library(state);
  if (state.screen === 'cook') return cook(state);
  return recipe(state);
}

/* ---------------------------------------------------------------- library */
function shelf(title, items) {
  return `
  <section class="c-shelf">
    <h2>${esc(title)}</h2>
    <div class="c-scroller">
      ${items
        .map(
          (r) => `
        <article class="c-tile" data-act="${r.id === 'katsu' ? 'open' : ''}">
          <div class="c-tile-img"><img src="${r.photo}" alt="">
            ${r.lang ? `<span class="c-badge">${r.lang}</span>` : ''}
          </div>
          <h3>${esc(r.title)}</h3>
          <p>${r.mins} min${r.cooks ? ` · ${r.cooks}×` : ''}</p>
        </article>`
        )
        .join('')}
    </div>
  </section>`;
}

function library(state) {
  const by = (ids) => LIBRARY.filter((r) => ids.includes(r.id));
  return `
  <div class="c">
    <header class="c-top">
      <div>
        <p class="c-hello">Evening, ${ME}</p>
        <h1>What are we cooking?</h1>
      </div>
      <div class="c-avatar">A</div>
    </header>
    <div class="c-searchwrap"><input class="c-search" placeholder="Search recipes, ingredients, anything" aria-label="Search"></div>

    ${
      state.cook.started
        ? `<button class="c-continue" data-act="resume">
             <img src="img/katsu.jpg" alt="">
             <span class="c-cont-t">
               <b>Pick up where you left off</b>
               <em>Katsu Curry · step ${state.cook.i + 1} of ${cookedSteps(KATSU).length}</em>
             </span>
             <span class="c-cont-go">▶</span>
           </button>`
        : ''
    }

    ${shelf('Cooked most', by(['dough', 'kfc', 'gateau', 'katsu', 'puree']))}
    ${shelf('Quick tonight', by(['ramen', 'katsu', 'dandan', 'puree']))}
    ${shelf('Never cooked', by(['meringue', 'tatin', 'iles', 'ribs']))}
  </div>`;
}

/* ----------------------------------------------------------------- recipe */
function recipe(state) {
  const r = KATSU;
  const f = state.scale;

  const ings = r.ingredients
    .map((ing) => {
      const sc = scaleLine(ing, f);
      const comp = ing.reading?.recipe;
      return `
      <li class="c-ing">
        <p class="c-ing-line">${esc(ing.line)}</p>
        <div class="c-ing-under">
          ${sc ? `<span class="c-pill c-pill-hot">${esc(sc)} for ${state.cook.yieldAmount}</span>` : ''}
          ${ing.reading && !comp ? `<span class="c-pill">${esc(ing.reading.food)}</span>` : ''}
          ${!ing.reading ? `<span class="c-pill c-pill-ghost">not read yet</span>` : ''}
        </div>
        ${
          comp
            ? `<div class="c-comp">
                 <p class="c-comp-h">${esc(comp.title)}</p>
                 ${comp.ingredients.map((c) => `<p class="c-comp-l">${esc(c.line)}</p>`).join('')}
               </div>`
            : ''
        }
      </li>`;
    })
    .join('');

  let n = 0;
  const steps = r.steps
    .map((s) => {
      if (s.section) return `<li class="c-sec">${esc(s.section)}</li>`;
      n += 1;
      return `<li class="c-step"><span>${n}</span><p>${esc(s.text)}</p></li>`;
    })
    .join('');

  return `
  <div class="c c-recipe">
    <div class="c-hero">
      <img src="${r.photo}" alt="">
      <button class="c-back" data-act="recipes">←</button>
      <div class="c-hero-txt">
        <p class="c-hero-src">${esc(r.source)}</p>
        <h1>${esc(r.title)}</h1>
        <p class="c-hero-sub">${esc(r.subtitle)}</p>
      </div>
    </div>

    <div class="c-quick">
      <div><b>${r.prep + r.cook}</b><span>minutes</span></div>
      <div class="c-yield">
        <button data-act="scale" data-d="-1">−</button>
        <b>${state.cook.yieldAmount}</b><span>servings</span>
        <button data-act="scale" data-d="1">+</button>
      </div>
      <div><b>4</b><span>cooks</span></div>
    </div>

    <div class="c-tabs">
      <span class="on">Ingredients</span><span>Method</span><span>History</span>
    </div>

    <ul class="c-ings">${ings}</ul>

    <div class="c-panel">
      <p class="c-panel-h">Note</p>
      <p class="c-panel-b">${esc(r.note)}</p>
    </div>

    <ol class="c-steps">${steps}</ol>

    <div class="c-panel">
      <p class="c-panel-h">Cooked ${r.attempts.length} times · last ${r.attempts[0].when}</p>
      ${r.attempts
        .filter((a) => a.rating)
        .map(
          (a) =>
            `<p class="c-att"><b>${esc(a.who)}</b><i>${stars(a.rating)}</i><em>${a.when}</em></p>`
        )
        .join('')}
      <p class="c-panel-b">“${esc(r.attempts[0].note)}”</p>
    </div>

    <p class="c-prov">${esc(r.version.name)} · ${r.version.when} · visible to ${esc(r.visibility)} · Français translation ${r.translation.behind} versions behind</p>

    <div class="c-cta"><button data-act="startCook">Start cooking</button></div>
  </div>`;
}

/* ------------------------------------------------------------------- cook */
function cook(state) {
  const r = KATSU;
  const steps = cookedSteps(r);
  const i = state.cook.i;
  const s = steps[i];

  if (state.cook.finished) return finishSheet(state);

  const need = (s.uses || []).length
    ? (s.uses || [])
        .map((ix) => {
          const ing = r.ingredients[ix];
          const rd = ing.reading;
          const on = state.cook.ticked.has(ix);
          const amount = rd && rd.qty != null ? `${rd.qty * state.scale}${rd.unit ? ' ' + rd.unit : ''}` : '';
          const what = rd?.recipe ? rd.recipe.title : rd?.food || ing.line;
          return `<button class="c-need${on ? ' on' : ''}" data-act="tick" data-i="${ix}">
            <b>${esc(amount)}</b><span>${esc(what)}</span>
          </button>`;
        })
        .join('')
    : `<p class="c-need-none">Nothing new to add — just the pot.</p>`;

  return `
  <div class="c c-cook">
    <header class="c-ctop">
      <button data-act="pause">✕</button>
      <span>Katsu Curry · ${state.cook.yieldAmount} servings</span>
      <span class="c-awake">☾ awake</span>
    </header>

    <div class="c-need-panel">
      <p class="c-need-h">For this step</p>
      <div class="c-needs">${need}</div>
    </div>

    <div class="c-stepwrap">
      <p class="c-step-n">Step ${i + 1} of ${steps.length}</p>
      <p class="c-step-t">${esc(s.text)}</p>
    </div>

    <div class="c-strip">
      ${steps
        .map(
          (st, k) =>
            `<button class="c-frame${k === i ? ' now' : k < i ? ' past' : ''}" data-act="goto" data-i="${k}">
               <b>${k + 1}</b><span>${esc(st.text.slice(0, 34))}…</span>
             </button>`
        )
        .join('')}
    </div>

    <div class="c-cfoot">
      <button class="c-cprev" data-act="prev" ${i === 0 ? 'disabled' : ''}>←</button>
      ${
        i === steps.length - 1
          ? `<button class="c-cnext c-cfin" data-act="finish">Finish</button>`
          : `<button class="c-cnext" data-act="next">Next</button>`
      }
    </div>
  </div>`;
}

export function after(state) {
  if (state.screen !== 'cook' || state.cook.finished) return;
  const el = document.querySelector('.c-frame.now');
  el?.scrollIntoView({ inline: 'center', block: 'nearest', behavior: 'instant' });
}

function finishSheet(state) {
  return `
  <div class="c c-cook c-fin">
    <div class="c-fin-card">
      <p class="c-fin-eyebrow">Katsu Curry</p>
      <h1>Nice one.</h1>
      <p class="c-fin-sub">Already saved as a cooking today. Add a rating or a note if you want to.</p>
      <div class="c-rate">
        ${[1, 2, 3, 4, 5]
          .map((n) => `<button data-act="rate" data-n="${n}" class="${state.cook.rating >= n ? 'on' : ''}">★</button>`)
          .join('')}
      </div>
      <textarea data-note placeholder="Next time…">${esc(state.cook.note)}</textarea>
      <button class="c-fin-save" data-act="save">Done</button>
      <button class="c-fin-bin" data-act="discard">Delete this cooking</button>
    </div>
  </div>`;
}
