// PROTOTYPE — pieces every direction shares, so that the only difference between
// A, B and C is how a DIVERGENCE reads. Shelf, recipe page and version rows are
// deliberately identical in all three.

import { RECIPE, VERSIONS, BRANCHES, BRANCH_POINT, ATTEMPTS, LIBRARY, ME, MY_KITCHEN, difference, stars } from './data.js';

export const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);

export function chrome() {
  return `<div class="top">
    <div class="mark">醸</div><div class="wordmark">Kamosu</div>
    <div class="kitchen">${MY_KITCHEN}</div>
  </div>`;
}

export function crumb(label, act = 'back') {
  return `<div class="crumb"><button class="back" data-act="${act}">‹ Back</button><h1>${esc(label)}</h1></div>`;
}

// ---- the shelf -----------------------------------------------------------
// Every direction answers "one card or two?" differently, so the card body is
// handed in by the direction.

export function shelf(cardExtra) {
  return `${chrome()}
    <h2 class="h">Your recipes</h2>
    <div class="list">
      ${LIBRARY.map(
        (r) => `<div class="card" data-act="${r.id === 'kfc' ? 'open' : 'nothing'}">
          <div class="card-img"><img src="${r.photo}" alt=""></div>
          <h2>${esc(r.title)}</h2>
          <p class="card-meta">${r.mins} min<span class="dot">·</span>cooked ${r.cooks}×<span class="dot">·</span>last ${r.last}</p>
          ${r.branches > 1 ? cardExtra() : ''}
        </div>`
      ).join('')}
    </div>
    <p class="foot">Only Korean Fried Chicken opens. It is the one with two Branches — Marc's copy came back on 14 Aug 2026, six months after you sent him yours.</p>`;
}

// ---- the recipe page -----------------------------------------------------

// `taken` is the unsaved edit: Marc's lines written into your recipe but not yet
// saved. They are marked as unsaved in place, never quietly swapped, because the
// whole claim is that you read your own recipe through before keeping it.
export function ingredientList(v, taken = new Map()) {
  return `<ul class="ings">${v.ingredients
    .map((x) => {
      if (x.section) return `<li class="sec">${esc(x.section)}</li>`;
      const e = taken.get(x.key);
      if (e?.remove)
        return `<li class="ing is-unsaved"><span class="ing-line was">${esc(x.line)}</span>
          <span class="unsaved-k">taken out, as Marc did · not saved</span></li>`;
      const t = e?.line;
      if (!t) return `<li class="ing"><span class="ing-line">${esc(x.line)}</span>${x.reading ? `<span class="ing-read">${esc(x.reading)}</span>` : ''}</li>`;
      return `<li class="ing is-unsaved"><span class="ing-line">${esc(t)}</span>
        <span class="unsaved-k">from Marc · not saved</span>
        <span class="ing-read was">was ${esc(x.line)}</span></li>`;
    })
    .join('')}${leftovers(v.ingredients, taken, 'ing')}</ul>`;
}

// A line Marc added has no counterpart of yours to sit on, so it lands at the end,
// marked. Where it *should* go is a real question this prototype cannot answer.
function leftovers(list, taken, kind) {
  const have = new Set(list.map((x) => x.key).filter(Boolean));
  return [...taken.entries()]
    .filter(([k, t]) => !have.has(k) && t.kind === kind && !t.remove)
    .map(([, t]) => `<li class="ing is-unsaved"><span class="ing-line">${esc(t.line)}</span>
      <span class="unsaved-k">added from Marc · not saved</span></li>`)
    .join('');
}

export function stepList(v, taken = new Map()) {
  return `<ol class="steps">${v.steps
    .map((s, i) => {
      const e = taken.get(s.key);
      const t = e?.remove ? null : e?.line;
      return `<li class="step ${t ? 'is-unsaved' : ''}"><span class="step-n">${i + 1}</span>
        <div><p>${esc(t || s.text)}</p>
        ${t ? `<span class="unsaved-k">from Marc · not saved</span><p class="ing-read was">was: ${esc(s.text)}</p>` : ''}</div></li>`;
    })
    .join('')}${stepLeftovers(v.steps, taken)}</ol>`;
}

function stepLeftovers(list, taken) {
  const have = new Set(list.map((x) => x.key).filter(Boolean));
  return [...taken.entries()]
    .filter(([k, t]) => !have.has(k) && t.kind === 'step' && !t.remove)
    .map(([, t]) => `<li class="step is-unsaved"><span class="step-n">+</span><div><p>${esc(t.line)}</p><span class="unsaved-k">added from Marc \u00b7 not saved</span></div></li>`)
    .join('');
}

// `banner` is the direction's own entry point into divergence; `versionId` lets a
// direction show a Version other than the branch head (reading back history).
export function recipeHead({ banner = '', versionId = null, chip = '' } = {}) {
  const v = VERSIONS[versionId || BRANCHES.mine.head];
  const isOld = versionId && versionId !== BRANCHES.mine.head;
  return `<div class="hero"><img src="${RECIPE.photo}" alt=""><button class="back" data-act="shelf">‹</button></div>
    <div class="sheet">
      <p class="eyebrow">${esc(RECIPE.source)}</p>
      <h1 class="title">${esc(RECIPE.title)}</h1>
      ${chip}
      <div class="meta">
        <div><b>${RECIPE.mins}</b><span>minutes</span></div>
        <div><b>${RECIPE.yieldAmount}</b><span>${RECIPE.yieldNoun}</span></div>
        <div><b>${ATTEMPTS.length}</b><span>cooked</span></div>
      </div>
    </div>
    ${banner}
    ${isOld ? `<div class="note">You are reading a Version from ${v.when}, not the one you keep. Reading and cooking, yes; editing, no — an older Version is finished, and history is append-only.</div>` : ''}`;
}

export function cookedList() {
  return `<h2 class="h">Cooked</h2>
    ${ATTEMPTS.map(
      (a) => `<div class="vrow">
        <div class="vrow-top"><span class="vrow-who">${esc(a.who)}</span>
          <span class="is-removed">${stars(a.rating)}</span>
          <span class="vrow-when">${a.when}</span></div>
        ${a.note ? `<p class="vrow-what">${esc(a.note)}</p>` : ''}
        <p class="vrow-what tiny">${attemptLabel(a)}</p>
      </div>`
    ).join('')}`;
}

export function recipePage(state, opts = {}) {
  const v = VERSIONS[opts.versionId || BRANCHES.mine.head];
  const old = opts.versionId && opts.versionId !== BRANCHES.mine.head;
  const taken = old ? new Map() : state.taken;
  return `${recipeHead(opts)}
    <h2 class="h">Ingredients</h2><div style="padding:0 22px">${ingredientList(v, taken)}</div>
    <h2 class="h">Method</h2><div style="padding:0 22px">${stepList(v, taken)}</div>
    ${v.note ? `<div class="note">${esc(v.note)}</div>` : ''}
    ${cookedList()}
    <button class="cta" data-act="nothing">${old ? `Cook this Version` : 'Start cooking'}</button>
    <button class="cta ghost" data-act="history">Earlier versions</button>`;
}

// An Attempt names the Version cooked — including "before these diverged" (ADR 0005).
export function attemptLabel(a) {
  const v = VERSIONS[a.version];
  if (a.version === BRANCH_POINT || a.version === 'v1') return `cooked before yours and Marc's diverged`;
  return `cooked ${v.branch === 'mine' ? 'your' : "Marc's"} version of ${v.when}`;
}

// ---- version rows --------------------------------------------------------

export function versionRow(id, { head = false, point = false, tapAct = 'read' } = {}) {
  const v = VERSIONS[id];
  return `<button class="vrow ${head ? 'is-head' : ''} ${point ? 'is-point' : ''}" data-act="${tapAct}" data-id="${id}">
    <div class="vrow-top">
      <span class="vrow-who">${esc(v.who)}</span>
      ${v.name ? `<span class="vrow-name">“${esc(v.name)}”</span>` : ''}
      <span class="vrow-when">${v.when}</span>
    </div>
    ${v.changed ? `<p class="vrow-what">${esc(v.changed)}</p>` : `<p class="vrow-what muted">Saved with nothing written down.</p>`}
    ${head ? `<p class="vrow-what tiny"><span class="pill ${v.branch === 'mine' ? 'mine' : 'theirs'}">${v.branch === 'mine' ? 'what you keep' : "Marc's latest"}</span></p>` : ''}
    ${point ? `<p class="vrow-what tiny"><span class="pill" style="color:var(--matcha)">where you and Marc part</span></p>` : ''}
  </button>`;
}

// ---- summary line shared by every direction's entry point -----------------

export function diffSummary() {
  const d = difference(BRANCHES.mine.head, BRANCHES.theirs.head);
  return d;
}

export { VERSIONS, BRANCHES, BRANCH_POINT, RECIPE, ATTEMPTS, ME, MY_KITCHEN };
