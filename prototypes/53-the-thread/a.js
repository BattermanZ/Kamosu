// PROTOTYPE — Direction A: LANES. A spatial graph — trunk down the middle,
// then a column per Branch at each fork, side by side. This is #18's thread,
// scaled up. Its own bet: forking reads as a PLACE (a fork in a road you can
// see both sides of at once). Its own risk, deliberately not hidden: a
// SECOND fork inside a rail (Camille off Marc) cannot become a third column
// on a phone, so it drops out of the grid and re-forks full-width below —
// judge whether that reads as "the same idea, once more" or as a seam.

import { childrenOf, esc, ATTEMPTS_BY_VERSION } from './data.js';
import { attemptChip, walkSegment, forkMarker } from './shared.js';

const RUN_THRESHOLD = 4;
const isQuiet = (v) => !v.name && !v.changed;

function node(v, branches) {
  const b = branches[v.branch];
  const atts = ATTEMPTS_BY_VERSION[v.id] || [];
  return `<button class="a-node" data-act="openVersion" data-id="${v.id}">
    <span class="a-dot" style="background:var(${b.color})"></span>
    <span class="a-body">
      <span class="a-when">${v.when}${v.language === 'fr' ? ' · fr' : ''}</span>
      <span class="a-who">${esc(b.who)}${v.name ? ` · “${esc(v.name)}”` : ''}</span>
      <span class="a-what">${v.changed ? esc(v.changed) : 'Saved with nothing written down.'}</span>
    </span>
    ${atts.length ? `<span class="a-atts">${atts.map(attemptChip).join('')}</span>` : ''}
  </button>`;
}

function flushRun(chain, branches, state) {
  let html = '';
  let run = [];
  const flush = () => {
    if (!run.length) return;
    if (run.length >= RUN_THRESHOLD) {
      const key = run[0].id;
      if (state.a.expanded.has(key)) {
        html += run.map((v) => node(v, branches)).join('');
        html += `<button class="a-collapse-back" data-act="toggleRun" data-scope="a" data-key="${key}">▴ collapse</button>`;
      } else {
        html += `<button class="a-collapsed" data-act="toggleRun" data-scope="a" data-key="${key}">
          <span class="a-dot a-dot-ghost"></span>
          <span class="a-body"><span class="a-what">${run.length} quiet saves, ${run[0].when} – ${run[run.length - 1].when}</span></span>
          <span class="a-expand">▾ show</span>
        </button>`;
      }
    } else {
      html += run.map((v) => node(v, branches)).join('');
    }
    run = [];
  };
  for (const v of chain) {
    if (isQuiet(v)) run.push(v);
    else {
      flush();
      html += node(v, branches);
    }
  }
  flush();
  return html;
}

function renderForkAt(forkId, versionsArr, versionsById, branches, state) {
  const kids = childrenOf(versionsArr, forkId);
  let out = forkMarker(kids, branches);
  out += `<div class="a-rails" style="grid-template-columns:repeat(${kids.length},1fr)">`;
  let nested = '';
  for (const child of kids) {
    const seg = walkSegment(child.id, versionsArr, versionsById);
    const b = branches[child.branch];
    out += `<div class="a-rail" style="--bc:var(${b.color})">
      <p class="a-rail-h"><span class="chip" style="background:var(${b.color})">${b.letter}</span> ${esc(b.label)}</p>
      ${flushRun(seg.chain, branches, state)}
    </div>`;
    if (seg.forkId) nested += renderForkAt(seg.forkId, versionsArr, versionsById, branches, state);
  }
  out += `</div>`;
  if (nested) out += `<div class="a-nested">${nested}</div>`;
  return out;
}

export function renderA(state, data) {
  const { versionsArr, versionsById, branches } = data;
  const root = versionsArr.find((v) => v.parent === null);
  const seg = walkSegment(root.id, versionsArr, versionsById);
  let html = `<div class="a-thread">${flushRun(seg.chain, branches, state)}`;
  if (seg.forkId) html += renderForkAt(seg.forkId, versionsArr, versionsById, branches, state);
  html += `</div>`;
  return html;
}
