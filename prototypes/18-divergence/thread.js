// PROTOTYPE — the thread. Was Direction C's history screen; Aurélien picked it over
// the other two, so it is now shared and B uses it as well.
//
// One line runs down the page, oldest at the top, and forks where the two of you
// part. Attempts hang off it, because an Attempt belongs to the Lineage rather
// than to either Branch (ADR 0005).

import { VERSIONS, BRANCHES, BRANCH_POINT, ATTEMPTS, stars } from './data.js';
import { esc, crumb, attemptLabel } from './shared.js';

// `compare` is the direction's own way of putting the two ends against each other.
export function thread({ compareAct = 'diverge', compareLabel = 'Put the two ends against each other' } = {}) {
  return `${crumb('Korean Fried Chicken — the thread')}
    <p class="c-lede">Every save is a Version, and nothing is ever rewritten. Read down the line.</p>

    <div class="c-thread">
      ${['v1', 'v2'].map((id) => node(id, 'trunk')).join('')}
      <div class="c-forkmark">
        <p class="c-forkmark-t">You and Marc part here</p>
        <p class="c-forkmark-s">Kamosu worked this out rather than being told: from ${VERSIONS[BRANCH_POINT].when} the two of you hold different content, so this is the last Version you share.</p>
      </div>
      <div class="c-rails">
        <div class="c-rail c-rail-mine">
          <p class="c-rail-h"><span class="pill mine">Yours</span></p>
          ${['v3'].map((id) => node(id, 'mine')).join('')}
        </div>
        <div class="c-rail c-rail-theirs">
          <p class="c-rail-h"><span class="pill theirs">Marc's</span> <em class="tiny muted">arrived 14 Aug</em></p>
          ${['m1', 'm2'].map((id) => node(id, 'theirs')).join('')}
        </div>
      </div>
    </div>

    <button class="cta" data-act="${compareAct}" data-b="theirs">${esc(compareLabel)}</button>

    <h2 class="h">Cooked, along the way</h2>
    ${ATTEMPTS.map(
      (a) => `<div class="c-att">
        <span class="c-att-when">${a.when}</span>
        <div><p class="c-att-t">${esc(a.who)} <span class="is-removed">${stars(a.rating)}</span></p>
        ${a.note ? `<p class="c-att-n">${esc(a.note)}</p>` : ''}
        <p class="c-att-v">${attemptLabel(a)}</p></div>
      </div>`
    ).join('')}
    <p class="foot">Tap any Version to read the recipe as it stood that day, and cook from it if you like — the Attempt records the Version, so the older notes above stay true about the state they described.
    An older Version cannot be edited. If you want an old one back, you save it again as a new Version at the end of the line: history only ever grows.</p>`;
}

function node(id, where) {
  const v = VERSIONS[id];
  const head = id === BRANCHES.mine.head || id === BRANCHES.theirs.head;
  return `<button class="c-node c-node-${where} ${head ? 'is-head' : ''}" data-act="read" data-id="${id}">
    <span class="c-dot"></span>
    <span class="c-node-when">${v.when}</span>
    <span class="c-node-who">${esc(v.who)}${v.name ? ` · “${esc(v.name)}”` : ''}</span>
    <span class="c-node-what">${esc(v.changed || 'Saved with nothing written down.')}</span>
    ${head ? `<span class="c-node-head">${where === 'theirs' ? "Marc's latest" : 'what you keep'}</span>` : ''}
  </button>`;
}
