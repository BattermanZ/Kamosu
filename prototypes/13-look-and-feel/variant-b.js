// PROTOTYPE — Variant B, "Ticket".
// Direction: the kitchen order ticket. Dense, printed, monospace data, no hero photo.
// Cooking answer: THE RAIL. Every step stays on screen; the one you're on is highlighted,
// the ones behind you collapse to a struck line. You always see what's coming.
import { KATSU, LIBRARY, ME, cookedSteps, scaleLine, readingLabel, stars } from './data.js';

const esc = (s) => String(s).replace(/[&<>]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' }[c]));
const pad = (n) => String(n).padStart(2, '0');

export function render(state) {
  if (state.screen === 'home' || state.screen === 'recipes') return library(state);
  if (state.screen === 'cook') return cook(state);
  return recipe(state);
}

/* ---------------------------------------------------------------- library */
function library(state) {
  return `
  <div class="b">
    <header class="b-head">
      <div class="b-brand">KAMOSU</div>
      <div class="b-headmeta">MAISON BATTERMAN · 86 RECIPES</div>
    </header>
    <input class="b-search" placeholder="search" aria-label="Search recipes">
    ${
      state.cook.started
        ? `<button class="b-resume" data-act="resume">
             <span class="b-resume-tag">ON THE PASS</span>
             <span class="b-resume-txt">Katsu Curry — step ${pad(state.cook.i + 1)}</span>
             <span class="b-resume-arrow">RESUME</span>
           </button>`
        : ''
    }
    <table class="b-table">
      <tbody>
      ${LIBRARY.map(
        (r) => `
        <tr data-act="${r.id === 'katsu' ? 'open' : ''}">
          <td class="b-thumb"><img src="${r.photo}" alt=""></td>
          <td class="b-name">
            ${esc(r.title)}
            ${r.lang ? `<span class="b-flag">${r.lang}</span>` : ''}
            ${r.branches ? `<span class="b-flag b-flag-x">2 branches</span>` : ''}
          </td>
          <td class="b-num">${r.mins}m</td>
          <td class="b-num">${r.cooks ? `${r.cooks}×` : '—'}</td>
          <td class="b-num b-dim">${r.last || 'never'}</td>
        </tr>`
      ).join('')}
      </tbody>
    </table>
  </div>`;
}

/* ----------------------------------------------------------------- recipe */
function recipe(state) {
  const r = KATSU;
  const f = state.scale;

  const ings = r.ingredients
    .map((ing) => {
      const rd = ing.reading;
      const comp = rd?.recipe;
      const amount = rd && rd.qty != null ? `${rd.qty * f}${rd.unit ? ' ' + rd.unit : ''}` : '—';
      return `
      <tr class="${comp ? 'b-i-comp' : ''}">
        <td class="b-qty">${esc(amount)}</td>
        <td class="b-line">
          ${esc(ing.line)}
          ${!rd ? `<span class="b-noread">no reading</span>` : ''}
          ${
            comp
              ? `<div class="b-sub">
                   <div class="b-sub-h">↳ ${esc(comp.title)} — makes ${comp.yieldAmount} ${comp.yieldNoun}</div>
                   ${comp.ingredients.map((c) => `<div class="b-sub-l">${esc(c.line)}</div>`).join('')}
                 </div>`
              : ''
          }
        </td>
      </tr>`;
    })
    .join('');

  let n = 0;
  const steps = r.steps
    .map((s) => {
      if (s.section) return `<div class="b-sec">— ${esc(s.section).toUpperCase()} —</div>`;
      n += 1;
      return `<div class="b-mstep"><span class="b-mnum">${pad(n)}</span><p>${esc(s.text)}</p></div>`;
    })
    .join('');

  return `
  <div class="b b-recipe">
    <header class="b-rhead">
      <button class="b-back" data-act="recipes">←</button>
      <div class="b-rhead-t">
        <h1>${esc(r.title)}</h1>
        <p>${esc(r.subtitle)}</p>
      </div>
      <img class="b-rthumb" src="${r.photo}" alt="">
    </header>

    <div class="b-strip">
      <span>PREP <b>${r.prep}m</b></span>
      <span>COOK <b>${r.cook}m</b></span>
      <span>YIELD <b>${state.cook.yieldAmount} servings</b></span>
      <span>SRC <b>${esc(r.source)}</b></span>
      <span>KITCHEN <b>${esc(r.kitchen)}</b></span>
      <span>${esc(r.nutrition).toUpperCase()}</span>
    </div>

    <div class="b-scale">
      <button data-act="scale" data-d="-1">−</button>
      <span>SCALE TO <b>${state.cook.yieldAmount}</b></span>
      <button data-act="scale" data-d="1">+</button>
      <em>quantities below follow; the written line never changes</em>
    </div>

    <h2 class="b-h">Ingredients <i>${r.ingredients.length}</i></h2>
    <table class="b-ings"><tbody>${ings}</tbody></table>

    <h2 class="b-h">Method <i>${cookedSteps(r).length}</i></h2>
    <div class="b-method">${steps}</div>

    <h2 class="b-h">Note</h2>
    <p class="b-note">${esc(r.note)}</p>

    <h2 class="b-h">Cooked <i>${r.attempts.length}</i></h2>
    <table class="b-hist"><tbody>
      ${r.attempts
        .map(
          (a) => `<tr>
            <td class="b-hwho">${esc(a.who)}</td>
            <td class="b-hdate">${a.when}</td>
            <td class="b-hstars">${a.rating ? stars(a.rating) : '<em>no rating</em>'}</td>
            <td class="b-hnote">${esc(a.note)}${a.asCooked ? '<span class="b-ascooked">as cooked</span>' : ''}</td>
          </tr>`
        )
        .join('')}
    </tbody></table>

    <div class="b-foot">
      <span>${esc(r.version.name)} · ${r.version.when} · ${esc(r.version.changed)}</span>
      <span>VISIBILITY ${esc(r.visibility).toUpperCase()}</span>
      <span>FR TRANSLATION — ${r.translation.behind} VERSIONS BEHIND</span>
    </div>

    <div class="b-cta"><button data-act="startCook">START COOKING</button></div>
  </div>`;
}

/* ------------------------------------------------------------------- cook */
function cook(state) {
  const r = KATSU;
  const steps = cookedSteps(r);
  const i = state.cook.i;

  if (state.cook.finished) return finishSheet(state);

  const ings = r.ingredients
    .map((ing, ix) => {
      const on = state.cook.ticked.has(ix);
      const rd = ing.reading;
      const amount = rd && rd.qty != null ? `${rd.qty * state.scale}${rd.unit ? ' ' + rd.unit : ''}` : '';
      return `<button class="b-chk${on ? ' on' : ''}" data-act="tick" data-i="${ix}">
        <i></i><b>${esc(amount)}</b><span>${esc(rd?.recipe ? rd.recipe.title : rd?.food || ing.line)}</span>
      </button>`;
    })
    .join('');

  const rail = steps
    .map((s, k) => {
      const cls = k < i ? 'done' : k === i ? 'now' : 'todo';
      return `<div class="b-rl b-rl-${cls}" data-act="goto" data-i="${k}">
        <span class="b-rl-n">${pad(k + 1)}</span>
        <p>${esc(s.text)}</p>
      </div>`;
    })
    .join('');

  return `
  <div class="b b-cook">
    <header class="b-ctop">
      <button class="b-cquit" data-act="pause">PAUSE</button>
      <span class="b-ctitle">KATSU CURRY · 4 SERVINGS</span>
      <span class="b-cnum">${pad(i + 1)}/${pad(steps.length)}</span>
    </header>

    <details class="b-mise" open>
      <summary>MISE EN PLACE — ${state.cook.ticked.size}/${r.ingredients.length} ticked</summary>
      <div class="b-chks">${ings}</div>
    </details>

    <div class="b-rail">${rail}</div>

    <div class="b-cfoot">
      <button class="b-cprev" data-act="prev" ${i === 0 ? 'disabled' : ''}>BACK</button>
      ${
        i === steps.length - 1
          ? `<button class="b-cnext b-cfin" data-act="finish">FINISH</button>`
          : `<button class="b-cnext" data-act="next">NEXT ${pad(i + 2)}</button>`
      }
    </div>
  </div>`;
}

export function after(state) {
  if (state.screen !== 'cook' || state.cook.finished) return;
  const el = document.querySelector('.b-rl-now');
  el?.scrollIntoView({ block: 'center', behavior: 'instant' });
}

function finishSheet(state) {
  return `
  <div class="b b-cook b-fin">
    <header class="b-ctop"><span class="b-ctitle">KATSU CURRY — SERVICE DONE</span></header>
    <div class="b-fin-in">
      <p class="b-fin-lead">Recorded as cooked today by ${ME}. Rating and note are optional.</p>
      <div class="b-rate">
        ${[1, 2, 3, 4, 5]
          .map((n) => `<button data-act="rate" data-n="${n}" class="${state.cook.rating >= n ? 'on' : ''}">★</button>`)
          .join('')}
      </div>
      <textarea data-note placeholder="What happened?">${esc(state.cook.note)}</textarea>
      <button class="b-fin-save" data-act="save">SAVE ATTEMPT</button>
      <button class="b-fin-bin" data-act="discard">delete — false start</button>
    </div>
  </div>`;
}
