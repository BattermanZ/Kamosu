// PROTOTYPE — Direction C: ONE LIST, NO GRAPH. Every Version and every
// Attempt from every Branch, merged into a single list by real date, each
// row tagged with a small coloured letter instead of a lane. A fork is one
// divider line, not a shape. Its own bet: nothing ever gets spatially wider,
// so it should hold up exactly as well at thirty Versions as at three. Its
// own risk: once two Branches are both being saved in the same week, their
// rows interleave — judge whether the letter tag is enough to keep straight
// "whose save is this" without a lane to lean on.

import { childrenOf, esc, ATTEMPTS, stars } from './data.js';
import { forkMarker } from './shared.js';

const isQuiet = (v) => !v.name && !v.changed;
const MONTHS = { Jan: 0, Feb: 1, Mar: 2, Apr: 3, May: 4, Jun: 5, Jul: 6, Aug: 7, Sep: 8, Oct: 9, Nov: 10, Dec: 11 };
const parseWhen = (s) => {
  const [d, mon, y] = s.split(' ');
  return new Date(+y, MONTHS[mon], +d).getTime();
};

function buildMerged(versionsArr, versionsById) {
  const moments = versionsArr.map((v) => ({ type: 'version', when: parseWhen(v.when), v }));
  for (const a of ATTEMPTS) {
    if (versionsById[a.version]) moments.push({ type: 'attempt', when: parseWhen(a.when), a });
  }
  moments.sort((x, y) => x.when - y.when);
  const out = [];
  for (const m of moments) {
    out.push(m);
    if (m.type === 'version') {
      const kids = childrenOf(versionsArr, m.v.id);
      if (kids.length > 1) out.push({ type: 'fork', kids });
    }
  }
  return out;
}

function versionRow(v, branches) {
  const b = branches[v.branch];
  return `<button class="c-row" data-act="openVersion" data-id="${v.id}">
    <span class="chip" style="background:var(${b.color})">${b.letter}</span>
    <span class="c-body">
      <span class="c-when">${v.when}${v.language === 'fr' ? ' · fr' : ''}</span>
      <span class="c-who">${esc(b.who)}${v.name ? ` · “${esc(v.name)}”` : ''}</span>
      <span class="c-what">${v.changed ? esc(v.changed) : 'Saved with nothing written down.'}</span>
    </span>
  </button>`;
}

function attemptRow(a, versionsById, branches) {
  const b = branches[versionsById[a.version].branch];
  return `<button class="c-row c-row-att" data-act="openAttempt" data-idx="${a.idx}">
    <span class="chip" style="background:var(${b.color})">🍲</span>
    <span class="c-body">
      <span class="c-when">${a.when}</span>
      <span class="c-who">${esc(a.who)} cooked it ${a.rating != null ? stars(a.rating) : ''}</span>
      ${a.note ? `<span class="c-what">${esc(a.note)}</span>` : ''}
    </span>
  </button>`;
}

function forkDivider(kids, branches) {
  return `<div class="c-forkdiv">${forkMarker(kids, branches)}</div>`;
}

export function renderC(state, data) {
  const { versionsArr, versionsById, branches } = data;
  const merged = buildMerged(versionsArr, versionsById);
  let html = '';
  let run = [];
  const flushRun = () => {
    if (!run.length) return;
    if (run.length >= 4) {
      const key = run[0].v.id;
      const b = branches[run[0].v.branch];
      if (state.c.expanded.has(key)) {
        html += run.map((m) => versionRow(m.v, branches)).join('');
        html += `<button class="c-collapse-back" data-act="toggleRun" data-scope="c" data-key="${key}">▴ collapse</button>`;
      } else {
        html += `<button class="c-collapsed" data-act="toggleRun" data-scope="c" data-key="${key}">
          <span class="chip" style="background:var(${b.color})">${b.letter}</span>
          <span class="c-body"><span class="c-what">${esc(b.who)} — ${run.length} quiet saves, ${run[0].v.when} – ${run[run.length - 1].v.when}</span></span>
          <span class="c-expand">▾ show</span>
        </button>`;
      }
    } else {
      html += run.map((m) => versionRow(m.v, branches)).join('');
    }
    run = [];
  };
  for (const m of merged) {
    if (m.type === 'version') {
      if (isQuiet(m.v)) {
        if (run.length && run[run.length - 1].v.branch !== m.v.branch) flushRun();
        run.push(m);
      } else {
        flushRun();
        html += versionRow(m.v, branches);
      }
    } else {
      flushRun();
      if (m.type === 'attempt') html += attemptRow(m.a, versionsById, branches);
      else html += forkDivider(m.kids, branches);
    }
  }
  flushRun();
  return `<div class="c-thread">${html}</div>`;
}
