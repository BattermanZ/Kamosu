// DIRECTION D — Aurélien's pick, assembled: B's threshold, A's captions, and the
// thing neither of them had — a way to put the marking away.
//
// The switch is B's: a THRESHOLD, not a control. A band names the kitchen you
// are standing in, the paper and the spine change colour when you cross, and one
// wide target says where it goes.
//
// The Ghost is A's: struck through in the flow of the list, with a caption
// underneath in words. B's silent Ghost was the half that did not survive —
// a struck-through line with no explanation reads like something crossed off a
// shopping list, not like a real line of a real recipe.
//
// And a divergence is not a thing you want to be looking at while cooking. So
// the marking has an off switch. With it off you get the recipe you are standing
// in, plain: no marks, no captions, and no Ghosts at all — a Ghost is a line of
// the OTHER recipe, so with the marking off it has no business on the page.

import { esc, look, peekHtml, head, tail, lists, differing, rows, THEM, MINE, THEIRS } from './shared.js';

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
	return `<li class="ing d-row ${c.ghost ? 'd-ghost' : 'd-diff'} ${open ? 'is-open' : ''} ${took ? 'is-took' : ''}"
		data-act="toggle" data-k="${c.key}">
		<span class="line">${esc(x.t)}</span>
		${x.reading ? `<span class="read">${esc(x.reading)}</span>` : ''}
		<span class="d-capt">${capt(c)}</span>
		${open ? peekHtml(c, state.taken) : ''}
	</li>`;
}

function step(c, state, n) {
	const open = state.open.has(c.key);
	const took = state.taken.has(c.key);
	const x = c.own ?? c.other;
	return `<li class="step d-row ${c.ghost ? 'd-ghost' : 'd-diff'} ${open ? 'is-open' : ''} ${took ? 'is-took' : ''}"
		data-act="toggle" data-k="${c.key}">
		<span class="n">${n ?? '·'}</span>
		<div class="d-body">
			<p>${esc(x.t)}</p>
			<span class="d-capt">${capt(c)}</span>
			${open ? peekHtml(c, state.taken) : ''}
		</div>
	</li>`;
}

// The threshold, and directly under it the one control that governs the marking.
// It sits here because this is already the strip that answers "whose recipe is
// this" — turning the marking off is the same question answered differently.
function switchHtml(state) {
	const theirs = state.side === 'theirs';
	const here = theirs ? THEIRS : MINE;
	const there = theirs ? MINE : THEIRS;
	const n = differing(rows.ings) + differing(rows.steps);
	const on = state.marks !== false;
	return `<div class="d-thresh">
			<div class="d-here">
				<p class="d-here-k">You are in</p>
				<p class="d-here-t">${esc(here.kitchen)}</p>
			</div>
			<button class="d-cross" data-act="side" data-side="${theirs ? 'mine' : 'theirs'}">
				<span class="d-cross-k">Cross to</span>
				<span class="d-cross-t">${esc(there.kitchen)} <em>→</em></span>
			</button>
		</div>
		<div class="d-marks ${on ? '' : 'is-off'}">
			<p>${
				on
					? `${n} line${n === 1 ? '' : 's'} you and ${esc(THEM)} don’t share`
					: `Just the recipe. ${n} difference${n === 1 ? '' : 's'} put away.`
			}</p>
			<button data-act="marks">${on ? 'Hide them' : 'Show them'}</button>
		</div>`;
}

export function render(state) {
	const theirs = state.side === 'theirs';
	const on = state.marks !== false;
	return (
		head(state, switchHtml(state)) +
		lists(state, ing, step) +
		tail(
			state,
			!on
				? `The marking is put away, so this is simply <b>${
						theirs ? `${esc(THEM)}’s recipe` : 'your recipe'
					}</b> — nothing struck through, nothing captioned, and no Ghosts, because a Ghost is a
					line of the other recipe. This is the page you would cook from.`
				: theirs
					? `The paper has gone cool and the spine is red: you are in <b>${esc(THEM)}’s kitchen</b>,
					   reading his recipe whole. Struck-through lines are <b>Ghosts</b> — lines of yours he
					   hasn’t got, sitting where they sit in your recipe, each saying what it is.
					   Tap one to see your side and carry it across.`
					: `Your own recipe, with every line you and ${esc(THEM)} don’t share marked and named.
					   Cross over at the top; the same lines are marked from either side.
					   <b>Hide them</b> when you just want the recipe.`
		)
	);
}
