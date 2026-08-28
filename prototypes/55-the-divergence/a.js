// DIRECTION A — "Two tabs, and every Ghost says what it is."
//
// The switch is a segmented control, the thing a phone user has tapped ten
// thousand times, and it sticks to the top of the screen so it never scrolls
// away mid-recipe. A Ghost is struck through IN THE FLOW of the list, with a
// small caption under it naming whose line it is.
//
// The bet: nothing is left for you to infer. Every marked line and every Ghost
// carries a few words saying what it is, so the page is readable cold, on a
// phone, by someone who has never seen a divergence before and never will again.
//
// The risk it takes on purpose: eight captions is eight captions. The page stops
// looking like a recipe and starts looking like a report about a recipe.

import { esc, look, peekHtml, head, tail, lists, THEM } from './shared.js';

const capt = (c) =>
	c.ghost
		? c.side === 'theirs'
			? c.row.base
				? `yours — ${esc(THEM)} took it out`
				: `yours — ${esc(THEM)} hasn’t got it`
			: `${esc(THEM)}’s — you haven’t got it`
		: c.side === 'theirs'
			? 'not yours'
			: `not ${esc(THEM)}’s`;

function ing(c, state) {
	const open = state.open.has(c.key);
	const took = state.taken.has(c.key);
	const x = c.own ?? c.other;
	return `<li class="ing a-row ${c.ghost ? 'a-ghost' : 'a-diff'} ${open ? 'is-open' : ''} ${took ? 'is-took' : ''}"
		data-act="toggle" data-k="${c.key}">
		<span class="line">${esc(x.t)}</span>
		${x.reading ? `<span class="read">${esc(x.reading)}</span>` : ''}
		<span class="a-capt">${capt(c)}</span>
		${open ? peekHtml(c, state.taken) : ''}
	</li>`;
}

function step(c, state, n) {
	const open = state.open.has(c.key);
	const took = state.taken.has(c.key);
	const x = c.own ?? c.other;
	return `<li class="step a-row ${c.ghost ? 'a-ghost' : 'a-diff'} ${open ? 'is-open' : ''} ${took ? 'is-took' : ''}"
		data-act="toggle" data-k="${c.key}">
		<span class="n">${n ?? '·'}</span>
		<div class="a-body">
			<p>${esc(x.t)}</p>
			<span class="a-capt">${capt(c)}</span>
			${open ? peekHtml(c, state.taken) : ''}
		</div>
	</li>`;
}

function switchHtml(state) {
	const theirs = state.side === 'theirs';
	return `<div class="a-switch">
		<div class="a-seg" role="tablist">
			<button role="tab" aria-selected="${!theirs}" class="${!theirs ? 'on' : ''}" data-act="side" data-side="mine">Yours</button>
			<button role="tab" aria-selected="${theirs}" class="${theirs ? 'on' : ''}" data-act="side" data-side="theirs">${esc(THEM)}’s</button>
		</div>
	</div>`;
}

export function render(state) {
	const theirs = state.side === 'theirs';
	return (
		head(state, switchHtml(state)) +
		lists(state, ing, step) +
		tail(
			state,
			theirs
				? `You are reading <b>${esc(THEM)}’s recipe</b>, whole and cookable. Struck-through lines are
				   <b>Ghosts</b> — lines of yours he hasn’t got, sitting where they sit in your recipe.
				   Every mark says what it is in words. Tap one to see your side and carry it across.`
				: `Every line that isn’t shared carries a caption. Switch to ${esc(THEM)}’s at the top —
				   the same lines are marked and the same Ghosts appear from either side.
				   <b>Judge the noise:</b> is a caption on every line reassuring, or is it a wall?`
		)
	);
}
