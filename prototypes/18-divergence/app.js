// PROTOTYPE — throwaway. Router, state, variant switcher. Issue #18.

import * as A from './a.js';
import * as B from './b.js';
import * as C from './c.js';
import { BRANCHES } from './data.js';

const VARIANTS = {
  a: { mod: A, name: 'The letter — a divergence is a message, written in sentences' },
  b: { mod: B, name: 'The switch — no diff screen; you read the other recipe' },
  c: { mod: C, name: 'The thread — divergence is the history screen doing its job' },
};
const SCREENS = ['shelf', 'recipe', 'diverge', 'history'];

const params = new URLSearchParams(location.search);

export const state = {
  v: VARIANTS[params.get('v')] ? params.get('v') : 'a',
  screen: SCREENS.includes(params.get('s')) ? params.get('s') : 'shelf',

  branch: 'mine',        // which Branch the recipe page is showing (B)
  readId: null,          // reading back an older Version
  open: new Set(),       // which difference rows are expanded
  taken: new Map(),      // key -> the line carried across, not yet saved
  editing: false,        // the unsaved edit sitting on top of your recipe
  saved: false,          // the new Version has been written
  cookOld: null,         // an Attempt started from an old Version
};

const app = document.getElementById('app');
const bar = document.getElementById('bar');

function syncUrl() {
  history.replaceState(null, '', '?' + new URLSearchParams({ v: state.v, s: state.screen }));
}

export function render() {
  document.getElementById('vcss').href = state.v + '.css';
  document.body.dataset.variant = state.v;
  document.body.dataset.screen = state.screen;
  app.innerHTML = `<div class="wrap">${VARIANTS[state.v].mod.render(state)}</div>`;
  syncUrl();
}

const actions = {
  nothing: () => {},
  shelf: () => {
    state.screen = 'shelf';
    state.branch = 'mine';
    state.readId = null;
  },
  open: () => (state.screen = 'recipe'),
  back: () => (state.screen = 'recipe'),
  diverge: () => (state.screen = 'diverge'),
  history: () => (state.screen = 'history'),

  branch: (el) => {
    state.branch = el.dataset.b;
    state.readId = null;
  },
  toggle: (el) => {
    const k = el.dataset.k;
    state.open.has(k) ? state.open.delete(k) : state.open.add(k);
  },
  read: (el) => {
    state.readId = el.dataset.id;
    state.screen = 'recipe';
  },

  // Carrying a change across. Taking a line does NOT save anything — it drops an
  // unsaved edit onto your own recipe, which you then read whole and save yourself.
  take: (el) => {
    state.taken.set(el.dataset.k, { line: el.dataset.line || null, kind: el.dataset.kind || 'ing', remove: el.dataset.remove === '1' });
    state.editing = true;
    state.saved = false;
    state.screen = 'recipe';
    state.branch = 'mine';
    state.readId = null;
  },
  untake: (el) => {
    state.taken.delete(el.dataset.k);
    if (!state.taken.size) state.editing = false;
  },
  saveEdit: () => {
    state.saved = true;
    state.editing = false;
  },
  dropEdit: () => {
    state.taken.clear();
    state.editing = false;
  },
  cookOld: (el) => (state.cookOld = el.dataset.id),
  closeCook: () => (state.cookOld = null),
  reset: () => {
    state.taken.clear();
    state.editing = false;
    state.saved = false;
    state.readId = null;
    state.branch = 'mine';
    state.open.clear();
  },
};

app.addEventListener('click', (e) => {
  const el = e.target.closest('[data-act]');
  if (!el) return;
  const fn = actions[el.dataset.act];
  if (!fn) return;
  e.preventDefault();
  e.stopPropagation();
  fn(el);
  render();
  if (!el.dataset.keepscroll) window.scrollTo(0, 0);
});

// ---- switcher ------------------------------------------------------------
function drawBar() {
  bar.innerHTML = `
    <button data-cycle="-1" aria-label="Previous direction">‹</button>
    <span class="lbl"><b>${state.v.toUpperCase()}</b><em> — ${VARIANTS[state.v].name}</em></span>
    <button data-cycle="1" aria-label="Next direction">›</button>
    <span class="sep"></span>
    ${SCREENS.map(
      (s) =>
        `<button class="scr" data-screen="${s}" aria-current="${state.screen === s}">${
          { shelf: 'Shelf', recipe: 'Recipe', diverge: 'Difference', history: 'History' }[s]
        }</button>`
    ).join('')}
    <span class="sep"></span>
    <button data-screen="__reset" title="Throw away the unsaved edit">↺</button>`;
}

bar.addEventListener('click', (e) => {
  const cyc = e.target.closest('[data-cycle]');
  const scr = e.target.closest('[data-screen]');
  const keys = Object.keys(VARIANTS);
  if (cyc) {
    const i = keys.indexOf(state.v);
    state.v = keys[(i + Number(cyc.dataset.cycle) + keys.length) % keys.length];
  } else if (scr) {
    if (scr.dataset.screen === '__reset') actions.reset();
    else {
      state.screen = scr.dataset.screen;
      if (state.screen === 'diverge' && state.v === 'b') state.branch = 'theirs';
    }
  } else return;
  drawBar();
  render();
});

addEventListener('keydown', (e) => {
  if (e.target.matches('input, textarea, [contenteditable]')) return;
  const keys = Object.keys(VARIANTS);
  if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
    const i = keys.indexOf(state.v);
    state.v = keys[(i + (e.key === 'ArrowRight' ? 1 : -1) + keys.length) % keys.length];
    drawBar();
    render();
  }
});

export { BRANCHES };
drawBar();
render();
