// PROTOTYPE — Direction A: "The letter".
//
// A divergence arrives the way a message arrives, and is read as SENTENCES.
// There is no diff, no columns, no colour-coded gutter. Kamosu tells you in words
// what Marc did, one line at a time, and each sentence has one offer: use his.
//
// Taking a line does not merge anything — it drops an unsaved change onto YOUR
// recipe, which you then read whole and save yourself. That is #16's "a person
// reading two recipes and editing one", made into two taps instead of a retype.

import { VERSIONS, BRANCHES, BRANCH_POINT, difference, sentence, text, isStep, THEM } from './data.js';
import { esc, chrome, crumb, shelf, recipePage, versionRow, attemptLabel } from './shared.js';

const d = () => difference(BRANCHES.mine.head, BRANCHES.theirs.head);

export function render(state) {
  if (state.screen === 'shelf') return shelfA();
  if (state.screen === 'diverge') return letter(state);
  if (state.screen === 'history') return historyA(state);
  return recipeA(state);
}

// ---- shelf: one card, one quiet line -------------------------------------
function shelfA() {
  return shelf(
    () => `<p class="card-meta a-shelfline">Marc has his own version of this — ${d().count} things differ</p>`
  );
}

// ---- recipe: the letter is announced, never shown here --------------------
function recipeA(state) {
  const n = d().count;
  const banner = state.saved
    ? `<div class="a-done">Saved. Your recipe has a new Version, and Marc's copy is still sitting beside it — unchanged, as it should be.</div>`
    : `<button class="flag" data-act="diverge">
        <span class="flag-k">Marc's copy came back · 14 Aug 2026</span>
        <span class="flag-t">Marc changed ${n} things after you and he parted.</span>
        <span class="flag-go">Read what he changed ›</span>
      </button>`;
  return recipePage(state, { banner, versionId: state.readId }) + unsaved(state);
}

// ---- the letter ----------------------------------------------------------
function letter(state) {
  const diff = d();
  const rows = (list) => list.filter((r) => r.kind !== 'same');
  const ing = rows(diff.ingredients);
  const st = rows(diff.steps);

  return `${crumb('What Marc changed')}
    <div class="a-preamble">
      <p>You and Marc had exactly the same recipe until <b>${VERSIONS[BRANCH_POINT].when}</b>.
      Since then you changed one thing, and he changed ${diff.count}.
      Nothing below happens on its own.</p>
    </div>

    <h2 class="h">In the ingredients</h2>
    <div class="a-lines">${ing.map((r) => line(r, state)).join('')}</div>

    <h2 class="h">In the method</h2>
    <div class="a-lines">${st.map((r) => line(r, state)).join('')}</div>

    ${diff.noteChanged ? `<h2 class="h">He left a note</h2><div class="note">${esc(VERSIONS[BRANCHES.theirs.head].note)}</div>` : ''}

    <h2 class="h">Or</h2>
    <button class="cta ghost" data-act="nothing">Read Marc's whole recipe</button>
    <button class="cta ghost" data-act="nothing">Keep Marc's as a second recipe of your own</button>
    <p class="foot">The second one is a <b>Copy</b> — a new Branch in your kitchen, exactly as if his file had arrived by email.
    It does not touch what you keep, and it does not merge anything.
    Marc's Branch stays on your shelf whatever you do here; there is no way to make it go away, and nothing here is a decision you have to make today.</p>`;
}

function line(r, state) {
  const s = sentence(r, 'Marc');
  const open = state.open.has(r.k);
  const took = state.taken.has(r.k);
  const theirs = text(r.right);
  const mine = text(r.left);
  const long = isStep(r.left) || isStep(r.right);

  return `<div class="a-line ${took ? 'is-took' : ''}">
    <button class="a-line-top" data-act="toggle" data-k="${r.k}" data-keepscroll="1">
      <span class="a-line-say">${esc(s)}${took ? ' — <b>taken</b>' : ''}</span>
      <span class="a-line-chev">${open ? '−' : '+'}</span>
    </button>
    ${
      open
        ? `<div class="a-line-body">
            ${r.left ? `<p class="a-side"><span class="a-side-k">Yours</span>${esc(mine)}</p>` : `<p class="a-side"><span class="a-side-k">Yours</span><em class="muted">nothing here</em></p>`}
            ${r.right ? `<p class="a-side a-side-theirs"><span class="a-side-k">Marc's</span>${esc(theirs)}</p>` : `<p class="a-side a-side-theirs"><span class="a-side-k">Marc's</span><em class="muted">he took it out</em></p>`}
            ${
              took
                ? `<button class="a-take is-off" data-act="untake" data-k="${r.k}" data-keepscroll="1">Put mine back</button>`
                : r.right
                ? `<button class="a-take" data-act="take" data-k="${r.k}" data-kind="${long ? 'step' : 'ing'}" data-line="${esc(theirs)}">Use Marc's${long ? ' step' : ''} instead</button>`
                : `<button class="a-take" data-act="take" data-k="${r.k}" data-kind="${long ? 'step' : 'ing'}" data-remove="1">Take it out of mine too</button>`
            }
          </div>`
        : ''
    }
  </div>`;
}

// ---- the unsaved edit sitting on your recipe ------------------------------
function unsaved(state) {
  if (!state.editing || !state.taken.size) return '';
  const items = [...state.taken.entries()];
  return `<div class="a-tray">
    <p class="a-tray-k">Not saved yet</p>
    <p class="a-tray-t">${items.length} line${items.length > 1 ? 's' : ''} from Marc, sitting on your recipe. Read it through before you keep it.</p>
    <ul class="a-tray-list">${items.map(([, v]) => `<li>${v.remove ? 'a line taken out' : esc(v.line.length > 90 ? v.line.slice(0, 90) + '…' : v.line)}</li>`).join('')}</ul>
    <label class="a-tray-label">What changed?</label>
    <textarea class="a-tray-note" rows="2">Took Marc's air-fryer step</textarea>
    <button class="a-tray-save" data-act="saveEdit">Save as a new Version</button>
    <button class="a-tray-drop" data-act="dropEdit">Throw it away</button>
  </div>`;
}

// ---- history: sentences, not a graph -------------------------------------
function historyA(state) {
  const mine = ['v3', 'v2', 'v1'];
  return `${crumb('Earlier versions')}
    <p class="a-preamble"><p>Every save is a Version. These are yours, newest first.</p></p>
    ${mine
      .map(
        (id) =>
          versionRow(id, { head: id === BRANCHES.mine.head, point: id === BRANCH_POINT }) +
          (id === BRANCH_POINT
            ? `<p class="a-fork">From here on, Marc's copy goes its own way. <button class="a-forklink" data-act="diverge">See what he did ›</button></p>`
            : '')
      )
      .join('')}
    <h2 class="h">Marc's, since you parted</h2>
    ${['m2', 'm1'].map((id) => versionRow(id, { head: id === BRANCHES.theirs.head })).join('')}
    <p class="foot">Tap any Version to read the recipe as it stood that day. You can cook from it — the Attempt then says which Version you cooked, so an old note never quietly becomes a lie. What you cannot do is edit one: an older Version is finished, and history is append-only.</p>`;
}
