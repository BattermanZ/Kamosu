// PROTOTYPE — Direction B: "The switch".
//
// The bet: there is no such thing as a difference screen. Marc's Branch is a
// RECIPE, so you read it as a recipe — whole, in order, cookable. Kamosu's only
// job is to mark the handful of lines that are not the same as yours, and to let
// you look at yours without leaving his page.
//
// This is the one direction where the comparison is never abstracted. You are
// always standing inside one recipe or the other, never above both.

import { VERSIONS, BRANCHES, BRANCH_POINT, difference, text, isStep } from './data.js';
import { esc, crumb, shelf, recipeHead, cookedList, versionRow } from './shared.js';

const d = () => difference(BRANCHES.mine.head, BRANCHES.theirs.head);

export function render(state) {
  if (state.screen === 'shelf') return shelfB();
  if (state.screen === 'history') return historyB(state);
  // 'diverge' is not a screen here — it is the recipe page showing Marc's Branch.
  if (state.screen === 'diverge') return recipeB({ ...state, branch: 'theirs' });
  return recipeB(state);
}

function shelfB() {
  return shelf(
    () => `<p class="card-meta"><span class="b-two">Yours</span><span class="b-two b-two-theirs">Marc's</span></p>`
  );
}

// ---- the recipe page, with a Branch switch at the top ---------------------
function recipeB(state) {
  const theirs = state.branch === 'theirs';
  const diff = d();
  const changed = new Map();
  [...diff.ingredients, ...diff.steps].forEach((r) => r.kind !== 'same' && changed.set(r.k, r));

  const chip = `<div class="b-switch">
      <button class="b-sw ${!theirs ? 'on' : ''}" data-act="branch" data-b="mine">Yours</button>
      <button class="b-sw ${theirs ? 'on' : ''}" data-act="branch" data-b="theirs">Marc's</button>
    </div>
    <p class="b-note">${
      theirs
        ? `Marc's own version, last saved ${VERSIONS[BRANCHES.theirs.head].when}. You can read it and cook from it. Editing it would make it yours — which is a <b>Copy</b>, not a change to his.`
        : `The recipe you keep. Marc has his own, since ${VERSIONS[BRANCH_POINT].when}.`
    }</p>`;

  const v = VERSIONS[state.readId || (theirs ? BRANCHES.theirs.head : BRANCHES.mine.head)];
  const other = theirs ? 'left' : 'right';

  const body = `
    <h2 class="h">Ingredients<span class="b-count">${countIn(diff.ingredients)} differ${theirs && hidden(diff.ingredients) ? `, ${hidden(diff.ingredients)} not shown` : ''}</span></h2>
    <div style="padding:0 22px"><ul class="ings">${v.ingredients
      .map((x) => row(x, changed, theirs, state))
      .join('')}</ul></div>
    <h2 class="h">Method<span class="b-count">${countIn(diff.steps)} of these differ</span></h2>
    <div style="padding:0 22px"><ol class="steps">${v.steps
      .map((s, i) => stepRow(s, i, changed, theirs, state))
      .join('')}</ol></div>
    ${v.note ? `<div class="note">${esc(v.note)}</div>` : ''}`;

  return recipeHead({ chip, versionId: state.readId }) + body + cookedList() + tail(state, theirs) + unsaved(state);
}

const countIn = (rows) => rows.filter((r) => r.kind !== 'same').length;

// The cost of this direction, said out loud: a line Marc REMOVED cannot be
// marked on Marc's recipe, because it is not there. Reading his page alone, you
// would never learn the brown sugar is gone.
const hidden = (rows) => rows.filter((r) => r.kind === 'removed').length;

function row(x, changed, theirs, state) {
  if (x.section) return `<li class="sec">${esc(x.section)}</li>`;
  const r = changed.get(x.key);
  const mark = r && (r.kind === 'changed' || r.kind === 'added' || r.kind === 'removed');
  if (!mark) return `<li class="ing"><span class="ing-line">${esc(x.line)}</span>${x.reading ? `<span class="ing-read">${esc(x.reading)}</span>` : ''}</li>`;
  return `<li class="ing b-diff ${state.open.has(x.key) ? 'is-open' : ''}" data-act="toggle" data-k="${x.key}" data-keepscroll="1">
    <span class="ing-line">${esc(x.line)}</span>
    <span class="b-mark">${theirs ? 'not yours' : "not Marc's"}</span>
    ${state.open.has(x.key) ? peek(r, theirs, state) : ''}
  </li>`;
}

function stepRow(s, i, changed, theirs, state) {
  const r = changed.get(s.key);
  if (!r) return `<li class="step"><span class="step-n">${i + 1}</span><p>${esc(s.text)}</p></li>`;
  return `<li class="step b-diff ${state.open.has(s.key) ? 'is-open' : ''}" data-act="toggle" data-k="${s.key}" data-keepscroll="1">
    <span class="step-n">${i + 1}</span>
    <div><p>${esc(s.text)}</p><span class="b-mark">${theirs ? 'not yours' : "not Marc's"}</span>
    ${state.open.has(s.key) ? peek(r, theirs, state) : ''}</div>
  </li>`;
}

// The other side, unfolded in place. Never a second column, never a second screen.
function peek(r, theirs, state) {
  const otherSide = theirs ? r.left : r.right;
  const label = theirs ? 'Yours' : "Marc's";
  const took = state.taken.has(r.k);
  return `<div class="b-peek">
    <p class="b-peek-k">${label}</p>
    <p class="b-peek-t">${otherSide ? esc(text(otherSide)) : `<em class="muted">${theirs ? 'you do not have this line' : 'Marc took this out'}</em>`}</p>
    ${
      theirs && otherSide === null
        ? ''
        : theirs
        ? took
          ? `<button class="b-take is-off" data-act="untake" data-k="${r.k}" data-keepscroll="1">Put mine back</button>`
          : `<button class="b-take" data-act="take" data-k="${r.k}" data-kind="${isStep(r.right) ? 'step' : 'ing'}" data-line="${esc(text(r.right))}">Write this into mine</button>`
        : ''
    }
  </div>`;
}

function tail(state, theirs) {
  return `<button class="cta" data-act="nothing">Cook ${theirs ? "Marc's" : 'this'}</button>
    <button class="cta ghost" data-act="history">Earlier versions</button>
    <p class="foot">${
      theirs
        ? `You never see the two recipes at once. The claim being tested is that you do not need to. Its known cost is on this page: Marc <b>removed</b> your ¼ cup brown sugar, and nothing on his recipe can mark a line that is not there — switch to yours to find it.`
        : 'Switch to Marc’s at the top. The same lines are marked from his side, so the comparison reads the same way whichever recipe you are standing in.'
    }</p>`;
}

function unsaved(state) {
  if (!state.editing || !state.taken.size) return '';
  const items = [...state.taken.entries()];
  return `<div class="b-tray">
    <p class="b-tray-t">${items.length} line${items.length > 1 ? 's' : ''} from Marc, written into your recipe and not yet saved.</p>
    <button class="b-tray-go" data-act="branch" data-b="mine">Read mine through</button>
    <button class="b-tray-save" data-act="saveEdit">Save</button>
    <button class="b-tray-drop" data-act="dropEdit">Undo</button>
  </div>`;
}

function historyB(state) {
  return `${crumb('Earlier versions')}
    <div class="b-histswitch">
      <button class="b-sw ${state.branch !== 'theirs' ? 'on' : ''}" data-act="branch" data-b="mine">Yours</button>
      <button class="b-sw ${state.branch === 'theirs' ? 'on' : ''}" data-act="branch" data-b="theirs">Marc's</button>
    </div>
    ${(state.branch === 'theirs' ? ['m2', 'm1', 'v2', 'v1'] : ['v3', 'v2', 'v1'])
      .map((id) =>
        versionRow(id, {
          head: id === BRANCHES[state.branch === 'theirs' ? 'theirs' : 'mine'].head,
          point: id === BRANCH_POINT,
        })
      )
      .join('')}
    <p class="foot">Both lists run back through the same two Versions — everything up to ${VERSIONS[BRANCH_POINT].when} is shared, and shows in both. That shared tail is the Branch Point, and it is a fact Kamosu works out rather than something anyone declared.
    Tap a Version to read the recipe as it stood that day; you can cook from it, and the Attempt will say which Version you cooked.</p>`;
}
