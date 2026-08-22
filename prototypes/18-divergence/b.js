// PROTOTYPE — Direction B: "The switch". Aurélien's pick.
//
// The bet: there is no such thing as a difference screen. Marc's Branch is a
// RECIPE, so you read it as a recipe — whole, in order, cookable. Kamosu's only
// job is to mark the handful of lines that are not the same as yours, and to let
// you look at yours without leaving his page.
//
// The GHOST LINE closes the hole the first pass left open. A line the other
// person doesn't have was said to be unshowable, because it is not on the page to
// mark. It is showable: put it on the page, struck through, in the position it
// occupies in the recipe that really has it, and say whose it is. One mechanism
// covers both directions at once — a removal seen from Marc's side and an
// addition seen from yours are the same object — so the page reads identically
// whichever recipe you are standing in.
//
// History here is the thread, which won on its own.

import { VERSIONS, BRANCHES, BRANCH_POINT, difference, text, isStep } from './data.js';
import { esc, shelf, recipeHead, cookedList } from './shared.js';
import { thread } from './thread.js';

const d = () => difference(BRANCHES.mine.head, BRANCHES.theirs.head);

export function render(state) {
  if (state.screen === 'shelf') return shelfB();
  if (state.screen === 'history') return thread({ compareAct: 'branch', compareLabel: "Read Marc's" });
  // 'diverge' is not a screen here — it is the recipe page showing Marc's Branch.
  if (state.screen === 'diverge') return recipeB({ ...state, branch: 'theirs' });
  return recipeB(state);
}

function shelfB() {
  return shelf(
    () => `<p class="card-meta"><span class="b-two">Yours</span><span class="b-two b-two-theirs">Marc's</span></p>`
  );
}

// ---- laying the two recipes over each other ------------------------------
// You read the current Branch's own list, in its own order, with the other side's
// orphan lines slotted in as ghosts where the pairing puts them.

function laid(rows, theirs) {
  const mineSide = !theirs;
  const out = [];
  rows.forEach((r) => {
    const own = mineSide ? r.left : r.right;
    const orphanHere = mineSide ? r.kind === 'added' : r.kind === 'removed';
    if (orphanHere) {
      out.push({ ghost: true, row: r, item: mineSide ? r.right : r.left });
      return;
    }
    if (own) out.push({ ghost: false, row: r, item: own });
  });
  return out;
}

function recipeB(state) {
  const theirs = state.branch === 'theirs';
  const diff = d();

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
  const reading = state.readId ? null : { ings: laid(diff.ingredients, theirs), steps: laid(diff.steps, theirs) };

  const body = reading
    ? `<h2 class="h">Ingredients<span class="b-count">${countIn(diff.ingredients)} differ</span></h2>
       <div style="padding:0 22px"><ul class="ings">${reading.ings.map((e) => ingRow(e, theirs, state)).join('')}</ul></div>
       <h2 class="h">Method<span class="b-count">${countIn(diff.steps)} differ</span></h2>
       <div style="padding:0 22px"><ol class="steps">${numbered(reading.steps, theirs, state)}</ol></div>
       ${v.note ? `<div class="note">${esc(v.note)}</div>` : ''}`
    : `<h2 class="h">Ingredients</h2><div style="padding:0 22px"><ul class="ings">${v.ingredients
        .map((x) => (x.section ? `<li class="sec">${esc(x.section)}</li>` : plainIng(x)))
        .join('')}</ul></div>
       <h2 class="h">Method</h2><div style="padding:0 22px"><ol class="steps">${v.steps
         .map((s, i) => `<li class="step"><span class="step-n">${i + 1}</span><p>${esc(s.text)}</p></li>`)
         .join('')}</ol></div>`;

  return recipeHead({ chip, versionId: state.readId }) + body + cookedList() + tail(state, theirs) + unsaved(state);
}

const countIn = (rows) => rows.filter((r) => r.kind !== 'same').length;
const plainIng = (x) =>
  `<li class="ing"><span class="ing-line">${esc(x.line)}</span>${x.reading ? `<span class="ing-read">${esc(x.reading)}</span>` : ''}</li>`;

function ingRow(e, theirs, state) {
  const { row: r, item: x, ghost } = e;
  if (x.section) return `<li class="sec">${esc(x.section)}</li>`;
  const open = state.open.has(r.k);

  if (ghost) {
    return `<li class="ing b-ghost ${open ? 'is-open' : ''}" data-act="toggle" data-k="${r.k}" data-keepscroll="1">
      <span class="ing-line was">${esc(x.line)}</span>
      <span class="b-mark b-mark-ghost">${theirs ? "yours — Marc hasn't got it" : "Marc's — you haven't got it"}</span>
      ${open ? peek(r, theirs, state, true) : ''}
    </li>`;
  }

  if (r.kind === 'same') return plainIng(x);

  return `<li class="ing b-diff ${open ? 'is-open' : ''}" data-act="toggle" data-k="${r.k}" data-keepscroll="1">
    <span class="ing-line">${esc(x.line)}</span>
    <span class="b-mark">${theirs ? 'not yours' : "not Marc's"}</span>
    ${open ? peek(r, theirs, state) : ''}
  </li>`;
}

// A ghost step takes no number — it is not part of this method.
function numbered(entries, theirs, state) {
  let n = 0;
  return entries
    .map((e) => {
      const { row: r, item: s, ghost } = e;
      const open = state.open.has(r.k);
      if (ghost)
        return `<li class="step b-ghost ${open ? 'is-open' : ''}" data-act="toggle" data-k="${r.k}" data-keepscroll="1">
          <span class="step-n">·</span>
          <div><p class="was">${esc(s.text)}</p>
          <span class="b-mark b-mark-ghost">${theirs ? "a step of yours Marc hasn't got" : "a step of Marc's you haven't got"}</span>
          ${open ? peek(r, theirs, state, true) : ''}</div>
        </li>`;
      n += 1;
      if (r.kind === 'same') return `<li class="step"><span class="step-n">${n}</span><p>${esc(s.text)}</p></li>`;
      return `<li class="step b-diff ${open ? 'is-open' : ''}" data-act="toggle" data-k="${r.k}" data-keepscroll="1">
        <span class="step-n">${n}</span>
        <div><p>${esc(s.text)}</p><span class="b-mark">${theirs ? 'not yours' : "not Marc's"}</span>
        ${open ? peek(r, theirs, state, false) : ''}</div>
      </li>`;
    })
    .join('');
}

// The other side, unfolded in place. Never a second column, never a second screen.
function peek(r, theirs, state, ghost = false) {
  const otherSide = theirs ? r.left : r.right;
  const took = state.taken.has(r.k);
  const long = isStep(r.left) || isStep(r.right);

  // On a ghost the line shown IS the other side's, so the peek explains rather
  // than repeats — and the offer is the interesting half.
  const said = ghost
    ? theirs
      ? 'Marc took this out of his. You still have it.'
      : 'Marc added this to his. You have not got it.'
    : otherSide
    ? esc(text(otherSide))
    : '';

  return `<div class="b-peek">
    <p class="b-peek-k">${ghost ? 'What happened' : theirs ? 'Yours' : "Marc's"}</p>
    <p class="b-peek-t">${said}</p>
    ${
      !theirs
        ? ''
        : took
        ? `<button class="b-take is-off" data-act="untake" data-k="${r.k}" data-keepscroll="1">Put mine back</button>`
        : ghost
        ? `<button class="b-take" data-act="take" data-k="${r.k}" data-kind="${long ? 'step' : 'ing'}" data-remove="1">Take it out of mine too</button>`
        : `<button class="b-take" data-act="take" data-k="${r.k}" data-kind="${isStep(r.right) ? 'step' : 'ing'}" data-line="${esc(text(r.right))}">Write this into mine</button>`
    }
  </div>`;
}

function tail(state, theirs) {
  return `<button class="cta" data-act="nothing">Cook ${theirs ? "Marc's" : 'this'}</button>
    <button class="cta ghost" data-act="history">The thread</button>
    <p class="foot">${
      theirs
        ? `You never see the two recipes at once — the claim being tested is that you do not need to.
           The struck-through lines are <b>ghosts</b>: things one of you has and the other hasn't, shown where they sit in the recipe that really has them. Marc removed your ¼ cup brown sugar, and there it is in the sauce, crossed out.`
        : `Switch to Marc's at the top. Every mark is symmetric — the same lines are marked and the same ghosts appear whichever recipe you are standing in. Marc's gochugaru is a ghost here for exactly the reason your brown sugar is a ghost there.`
    }</p>`;
}

function unsaved(state) {
  if (!state.editing || !state.taken.size) return '';
  const items = [...state.taken.entries()];
  return `<div class="b-tray">
    <p class="b-tray-t">${items.length} change${items.length > 1 ? 's' : ''} from Marc, written into your recipe and not yet saved.</p>
    <button class="b-tray-go" data-act="branch" data-b="mine">Read mine through</button>
    <button class="b-tray-save" data-act="saveEdit">Save</button>
    <button class="b-tray-drop" data-act="dropEdit">Undo</button>
  </div>`;
}
