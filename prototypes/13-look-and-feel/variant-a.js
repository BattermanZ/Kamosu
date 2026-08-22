// PROTOTYPE — Variant A, "Noren".
// Direction: indigo-dyed cloth over unbleached paper. Quiet, tactile, one thing at a time.
// Cooking answer: ONE STEP, FULL SCREEN. Everything else gets out of the way.
import { KATSU, LIBRARY, ME, cookedSteps, scaleLine, readingLabel, stars } from './data.js';

const esc = (s) => String(s).replace(/[&<>]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' }[c]));

export function render(state) {
  if (state.screen === 'library') return library(state);
  if (state.screen === 'cook') return cook(state);
  return recipe(state);
}

/* ---------------------------------------------------------------- library */
function library(state) {
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
    <div class="a-searchwrap"><input class="a-search" placeholder="Search 86 recipes" aria-label="Search recipes"></div>
    ${resume}
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
  </div>`;
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
    <div class="a-hero"><img src="${r.photo}" alt=""><button class="a-back" data-act="library">←</button></div>
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
  </div>`;
}

/* ------------------------------------------------------------------- cook */
function cook(state) {
  const r = KATSU;
  const steps = cookedSteps(r);
  const i = state.cook.i;
  const s = steps[i];
  const last = i === steps.length - 1;

  if (state.cook.finished) return finishSheet(state);

  const uses = (s.uses || [])
    .map((ix) => {
      const ing = r.ingredients[ix];
      const sc = scaleLine(ing, state.scale);
      return `<button class="a-use${state.cook.ticked.has(ix) ? ' is-done' : ''}" data-act="tick" data-i="${ix}">
        ${esc(sc || readingLabel(ing) || ing.line)}
      </button>`;
    })
    .join('');

  return `
  <div class="a a-cook">
    <div class="a-noren" aria-hidden="true">
      ${steps.map((_, k) => `<i class="${k < i ? 'past' : k === i ? 'now' : ''}"></i>`).join('')}
    </div>

    <header class="a-cook-top">
      <button class="a-quit" data-act="pause">Pause</button>
      <span class="a-cook-title">Katsu Curry</span>
      <span class="a-cook-count">${i + 1}<i>/${steps.length}</i></span>
    </header>

    <div class="a-cook-body">
      <p class="a-cook-step">${esc(s.text)}</p>
      ${uses ? `<div class="a-uses">${uses}</div>` : ''}
    </div>

    <div class="a-cook-foot">
      <button class="a-prevbtn" data-act="prev" ${i === 0 ? 'disabled' : ''}>Back</button>
      ${
        last
          ? `<button class="a-nextbtn a-done" data-act="finish">Finish cooking</button>`
          : `<button class="a-nextbtn" data-act="next">Next step</button>`
      }
    </div>
    <p class="a-awake">Screen stays awake · cooking 4 servings</p>
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
