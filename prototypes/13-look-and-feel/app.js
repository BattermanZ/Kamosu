// PROTOTYPE — throwaway. Router, state and the variant switcher.
import * as A from './variant-a.js';
import * as B from './variant-b.js';
import * as C from './variant-c.js';
import { KATSU, cookedSteps } from './data.js';

const VARIANTS = {
  a: { mod: A, name: 'Noren — one step, full screen' },
  b: { mod: B, name: 'Ticket — the whole method, one rail' },
  c: { mod: C, name: 'Counter — split, ingredients pinned' },
};
const SCREENS = ['library', 'recipe', 'cook'];

const params = new URLSearchParams(location.search);

export const state = {
  v: VARIANTS[params.get('v')] ? params.get('v') : 'a',
  screen: SCREENS.includes(params.get('s')) ? params.get('s') : 'library',
  scale: 1,
  cook: {
    // An In Progress Attempt: which step, which ingredients ticked, the yield cooked to.
    started: false,
    i: 0,
    ticked: new Set(),
    yieldAmount: KATSU.yieldAmount,
    finished: false,
    rating: null,
    note: '',
  },
};

const app = document.getElementById('app');
const bar = document.getElementById('bar');

function syncUrl() {
  const p = new URLSearchParams({ v: state.v, s: state.screen });
  history.replaceState(null, '', '?' + p);
}

export function render() {
  document.getElementById('vcss').href = state.v + '.css';
  document.body.dataset.variant = state.v;
  document.body.dataset.screen = state.screen;
  app.innerHTML = VARIANTS[state.v].mod.render(state);
  VARIANTS[state.v].mod.after?.(state);
  bar.classList.toggle('tucked', state.screen === 'cook');
  syncUrl();
  window.scrollTo(0, 0);
}

// ---- actions -------------------------------------------------------------
const steps = cookedSteps(KATSU);

const actions = {
  open: () => (state.screen = 'recipe'),
  library: () => (state.screen = 'library'),
  back: () => (state.screen = 'recipe'),
  scale: (el) => {
    const y = Math.max(1, state.cook.yieldAmount + Number(el.dataset.d));
    state.cook.yieldAmount = y;
    state.scale = y / KATSU.yieldAmount;
  },
  startCook: () => {
    state.cook.started = true;
    state.cook.finished = false;
    state.screen = 'cook';
  },
  resume: () => {
    state.screen = 'cook';
  },
  next: () => (state.cook.i = Math.min(steps.length - 1, state.cook.i + 1)),
  prev: () => (state.cook.i = Math.max(0, state.cook.i - 1)),
  goto: (el) => (state.cook.i = Number(el.dataset.i)),
  tick: (el) => {
    const i = Number(el.dataset.i);
    state.cook.ticked.has(i) ? state.cook.ticked.delete(i) : state.cook.ticked.add(i);
  },
  pause: () => (state.screen = 'recipe'),
  finish: () => (state.cook.finished = true),
  rate: (el) => (state.cook.rating = Number(el.dataset.n)),
  save: () => {
    state.cook.started = false;
    state.cook.finished = false;
    state.cook.i = 0;
    state.cook.ticked = new Set();
    state.screen = 'recipe';
  },
  discard: () => {
    state.cook.started = false;
    state.cook.finished = false;
    state.cook.i = 0;
    state.cook.ticked = new Set();
    state.cook.rating = null;
    state.screen = 'recipe';
  },
};

app.addEventListener('click', (e) => {
  const el = e.target.closest('[data-act]');
  if (!el) return;
  const fn = actions[el.dataset.act];
  if (!fn) return;
  e.preventDefault();
  fn(el);
  render();
});

app.addEventListener('input', (e) => {
  if (e.target.matches('[data-note]')) state.cook.note = e.target.value;
});

// ---- switcher ------------------------------------------------------------
function drawBar() {
  const keys = Object.keys(VARIANTS);
  bar.innerHTML = `
    <button data-cycle="-1" aria-label="Previous variant">‹</button>
    <span class="lbl"><b>${state.v.toUpperCase()}</b><em> — ${VARIANTS[state.v].name}</em></span>
    <button data-cycle="1" aria-label="Next variant">›</button>
    <span class="sep"></span>
    ${SCREENS.map(
      (s) =>
        `<button class="scr" data-screen="${s}" aria-current="${state.screen === s}">${s[0].toUpperCase() + s.slice(1)}</button>`
    ).join('')}`;
}

bar.addEventListener('click', (e) => {
  const cyc = e.target.closest('[data-cycle]');
  const scr = e.target.closest('[data-screen]');
  const keys = Object.keys(VARIANTS);
  if (cyc) {
    const i = keys.indexOf(state.v);
    state.v = keys[(i + Number(cyc.dataset.cycle) + keys.length) % keys.length];
  } else if (scr) {
    state.screen = scr.dataset.screen;
    if (state.screen === 'cook') state.cook.started = true;
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

drawBar();
render();
