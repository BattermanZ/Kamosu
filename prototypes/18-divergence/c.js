// PROTOTYPE — Direction C: "The thread".
//
// The bet: divergence is not a special event, it is what a history screen looks
// like when two people are writing in it. One vertical line runs down the page,
// oldest at the top, and where you and Marc part the line simply forks. There is
// no "a copy came back" moment to design, because the thread was always there.
//
// The difference view here is deliberately STACKED, never side by side: yours on
// top with a rule through it, theirs underneath. A phone is 390px wide and a
// recipe line is long.

import { VERSIONS, BRANCHES, BRANCH_POINT, ATTEMPTS, difference, text, isStep, stars } from './data.js';
import { esc, crumb, shelf, recipePage } from './shared.js';
import { thread } from './thread.js';

const d = () => difference(BRANCHES.mine.head, BRANCHES.theirs.head);

export function render(state) {
  if (state.screen === 'shelf') return shelfC();
  if (state.screen === 'diverge') return compare(state);
  if (state.screen === 'history') return thread();
  return recipeC(state);
}

function shelfC() {
  return shelf(() => `<p class="card-meta c-fork">⑂ two branches — yours and Marc's</p>`);
}

function recipeC(state) {
  const banner = state.saved
    ? `<div class="c-saved">Saved. That is one more Version on your line — and Marc's line is untouched beside it.</div>`
    : `<button class="flag" data-act="history">
        <span class="flag-k">This recipe's thread</span>
        <span class="flag-t">${5} versions, two hands, forking on ${VERSIONS[BRANCH_POINT].when}.</span>
        <span class="flag-go">Follow it ›</span>
      </button>`;
  return recipePage(state, { banner, versionId: state.readId }) + tray(state);
}

// ---- the comparison, stacked ---------------------------------------------
function compare(state) {
  const diff = d();
  const block = (title, rows) => `<h2 class="h">${title}</h2>
    ${rows
      .filter((r) => r.kind !== 'same')
      .map((r) => pairRow(r, state))
      .join('')}`;
  return `${crumb('Yours against Marc’s')}
    <p class="c-lede">Your latest, and his. ${diff.count} things differ. Everything not listed is word-for-word the same.</p>
    ${block('Ingredients', diff.ingredients)}
    ${block('Method', diff.steps)}
    ${diff.noteChanged ? `<h2 class="h">Note</h2><div class="c-pair"><p class="c-side c-theirs"><span class="c-side-k">Marc's</span>${esc(VERSIONS[BRANCHES.theirs.head].note)}</p><p class="c-side"><span class="c-side-k">Yours</span><em class="muted">you have no note</em></p></div>` : ''}
    <p class="foot">Stacked, never side by side. Two columns at 390px wide give each recipe line about twenty characters, which is how a diff becomes unreadable exactly where it matters.
    Taking a line does not merge the two: it writes that one line into your recipe and leaves it unsaved, so what you save is a recipe you read through — which is what “Kamosu never merges” means in practice.</p>
    ${tray(state)}`;
}

function pairRow(r, state) {
  const took = state.taken.has(r.k);
  const long = isStep(r.left) || isStep(r.right);
  return `<div class="c-pair ${took ? 'is-took' : ''}">
    ${
      r.left
        ? `<p class="c-side ${r.right ? 'c-was' : 'c-only'}"><span class="c-side-k">Yours</span>${esc(text(r.left))}</p>`
        : `<p class="c-side c-none"><span class="c-side-k">Yours</span><em class="muted">nothing here</em></p>`
    }
    ${
      r.right
        ? `<p class="c-side c-theirs"><span class="c-side-k">Marc's</span>${esc(text(r.right))}</p>`
        : `<p class="c-side c-none"><span class="c-side-k">Marc's</span><em class="muted">taken out</em></p>`
    }
    ${
      took
        ? `<button class="c-take is-off" data-act="untake" data-k="${r.k}" data-keepscroll="1">Put mine back</button>`
        : r.right
        ? `<button class="c-take" data-act="take" data-k="${r.k}" data-kind="${long ? 'step' : 'ing'}" data-line="${esc(text(r.right))}">Take Marc's${long ? ' step' : ' line'}</button>`
        : `<button class="c-take" data-act="take" data-k="${r.k}" data-kind="${long ? 'step' : 'ing'}" data-remove="1">Take it out of mine too</button>`
    }
  </div>`;
}

function tray(state) {
  if (!state.editing || !state.taken.size) return '';
  const items = [...state.taken.entries()];
  return `<div class="c-tray">
    <p class="c-tray-t">${items.length} of Marc's line${items.length > 1 ? 's' : ''} written into your recipe. Not saved.</p>
    <button class="c-tray-read" data-act="back">Read mine</button>
    <button class="c-tray-save" data-act="saveEdit">Save a Version</button>
    <button class="c-tray-drop" data-act="dropEdit">Undo</button>
  </div>`;
}
