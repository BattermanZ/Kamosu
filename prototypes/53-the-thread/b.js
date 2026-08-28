// PROTOTYPE — Direction B: ONE COLUMN, PICK A BRANCH. Never two lanes side
// by side, at any depth. At a fork, a row of chips asks which Branch to
// follow forward; the thread below the fork is always whichever one is
// picked, and switching is one tap. Its own bet: a phone is one column, so
// the thread should be too — legibility never degrades with more Branches,
// because you are only ever looking at one at a time. Its own risk: you
// cannot see two Branches at once, so "did Marc's turn out differently"
// needs a tap you might not think to make.

import { childrenOf, esc, ATTEMPTS_BY_VERSION } from './data.js';
import { attemptChip, walkSegment, forkMarker } from './shared.js';

const isQuiet = (v) => !v.name && !v.changed;

function node(v, branches) {
  const b = branches[v.branch];
  const atts = ATTEMPTS_BY_VERSION[v.id] || [];
  const quiet = isQuiet(v);
  return `<button class="b-node ${quiet ? 'is-quiet' : ''}" data-act="openVersion" data-id="${v.id}">
    <span class="b-dot" style="background:var(${b.color})"></span>
    <span class="b-body">
      <span class="b-when">${v.when}${v.language === 'fr' ? ' · fr' : ''}</span>
      <span class="b-who">${esc(b.who)}${v.name ? ` · “${esc(v.name)}”` : ''}</span>
      ${quiet ? '' : `<span class="b-what">${esc(v.changed)}</span>`}
    </span>
    ${atts.length ? `<span class="b-atts">${atts.map(attemptChip).join('')}</span>` : ''}
  </button>`;
}

export function renderB(state, data) {
  const { versionsArr, versionsById, branches } = data;
  const root = versionsArr.find((v) => v.parent === null);
  let html = '';
  let cur = root.id;
  const crumbs = [];
  while (cur) {
    const seg = walkSegment(cur, versionsArr, versionsById);
    html += seg.chain.map((v) => node(v, branches)).join('');
    if (!seg.forkId) break;
    const kids = childrenOf(versionsArr, seg.forkId);
    if (!state.b.choice.has(seg.forkId)) state.b.choice.set(seg.forkId, kids[0].id);
    const chosen = state.b.choice.get(seg.forkId);
    html += forkMarker(kids, branches);
    html += `<div class="b-chips">${kids
      .map((k) => {
        const b = branches[k.branch];
        const on = k.id === chosen;
        return `<button class="b-chip ${on ? 'on' : ''}" data-act="chooseFork" data-fork="${seg.forkId}" data-child="${k.id}"
          style="${on ? `background:var(${b.color});color:var(--paper);border-color:var(${b.color})` : `color:var(${b.color});border-color:var(${b.color})`}">
          <span class="chip" style="background:var(${b.color})">${b.letter}</span> Follow ${esc(b.label)}
        </button>`;
      })
      .join('')}</div>`;
    crumbs.push(branches[versionsById[chosen].branch].label);
    cur = chosen;
  }
  return html;
}
