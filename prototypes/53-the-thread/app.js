// PROTOTYPE — throwaway. Router, state, variant switcher. Issue #53.

import { renderA } from './a.js';
import { renderB } from './b.js';
import { renderC } from './c.js';
import { datasetFor, VERSIONS } from './data.js';
import { header, versionSheetHtml, attemptSheetHtml } from './shared.js';

const VARIANTS = {
  a: { fn: renderA, name: 'Lanes — a column per Branch, side by side' },
  b: { fn: renderB, name: 'One column — pick a Branch to follow forward' },
  c: { fn: renderC, name: 'One list — every Branch merged by date, tagged not laned' },
};
const SIZES = { small: 'Small (3 Versions, no fork)', large: 'Large (27 Versions, 3 Branches, nested fork)' };

const params = new URLSearchParams(location.search);

const state = {
  v: VARIANTS[params.get('v')] ? params.get('v') : 'a',
  size: SIZES[params.get('s')] ? params.get('s') : 'large',
  a: { expanded: new Set() },
  b: { choice: new Map() },
  c: { expanded: new Set() },
  openVersionId: null,
  openAttemptIdx: null,
};

const app = document.getElementById('app');
const bar = document.getElementById('bar');

function syncUrl() {
  history.replaceState(null, '', '?' + new URLSearchParams({ v: state.v, s: state.size }));
}

function render() {
  const data = datasetFor(state.size);
  document.body.dataset.variant = state.v;
  let html = header(data.versionsArr.length, Object.keys(data.branches).length);
  html += VARIANTS[state.v].fn(state, data);
  html += `<p class="foot">Tap "Show" on a collapsed run to expand it, or a Version to read it back. State resets on reload.</p>`;
  if (state.openVersionId) html += versionSheetHtml(VERSIONS[state.openVersionId]);
  if (state.openAttemptIdx != null) html += attemptSheetHtml(state.openAttemptIdx);
  app.innerHTML = `<div class="wrap">${html}</div>`;
  syncUrl();
}

const actions = {
  openVersion: (el) => (state.openVersionId = el.dataset.id),
  openAttempt: (el) => (state.openAttemptIdx = Number(el.dataset.idx)),
  closeSheet: () => {
    state.openVersionId = null;
    state.openAttemptIdx = null;
  },
  toggleRun: (el) => {
    const set = state[el.dataset.scope].expanded;
    const key = el.dataset.key;
    set.has(key) ? set.delete(key) : set.add(key);
  },
  chooseFork: (el) => state.b.choice.set(el.dataset.fork, el.dataset.child),
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

// ---- switcher: variant (arrows) + dataset size (its own pair of buttons) --
function drawBar() {
  bar.innerHTML = `
    <button data-cycle="-1" aria-label="Previous direction">‹</button>
    <span class="lbl"><b>${state.v.toUpperCase()}</b><em> — ${VARIANTS[state.v].name}</em></span>
    <button data-cycle="1" aria-label="Next direction">›</button>
    <span class="sep"></span>
    ${Object.keys(SIZES)
      .map((s) => `<button class="scr" data-size="${s}" aria-current="${state.size === s}">${s === 'small' ? 'Small' : 'Large'}</button>`)
      .join('')}`;
}

bar.addEventListener('click', (e) => {
  const cyc = e.target.closest('[data-cycle]');
  const sz = e.target.closest('[data-size]');
  const keys = Object.keys(VARIANTS);
  if (cyc) {
    const i = keys.indexOf(state.v);
    state.v = keys[(i + Number(cyc.dataset.cycle) + keys.length) % keys.length];
  } else if (sz) {
    state.size = sz.dataset.size;
    state.a.expanded.clear();
    state.b.choice.clear();
    state.c.expanded.clear();
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
