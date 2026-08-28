// DIRECTION C — "A lever under your thumb, and a Ghost that is visibly not a line."
//
// The switch lives at the BOTTOM, pinned, always there. On a phone the top of
// the screen is the furthest place from your hand, and a divergence is read by
// flicking between the two recipes several times — so the switch is put where
// the flicking actually happens, and it never scrolls away because it was never
// in the scroll.
//
// And a Ghost is pulled OUT of the list. It is indented onto its own recessed
// slab, unnumbered, with the name of the recipe it really belongs to written on
// it. It cannot be mistaken for a line of the recipe you are standing in,
// because it does not look like one.
//
// The bet: the dangerous failure is a cook reading a Ghost as an instruction.
// A struck-through line inside the list is still inside the list; a line on a
// slab in the margin is plainly a note about somewhere else.
//
// The risk it takes on purpose: an inset slab breaks the column. ADR 0014 wanted
// a Ghost "in the position it holds over there" — indenting it keeps the
// position but loosens the sense that it is genuinely part of the sequence.

import { esc, look, peekHtml, head, tail, lists, THEM, MINE, THEIRS } from './shared.js';

const owner = (c) => (c.side === 'theirs' ? 'In yours' : `In ${esc(THEM)}’s`);

function ghostSlab(c, state, body) {
	const open = state.open.has(c.key);
	const took = state.taken.has(c.key);
	return `<li class="c-ghostwrap ${open ? 'is-open' : ''} ${took ? 'is-took' : ''}"
		data-act="toggle" data-k="${c.key}">
		<div class="c-slab">
			<span class="c-slab-k">${owner(c)}</span>
			${body}
			${open ? peekHtml(c, state.taken) : ''}
		</div>
	</li>`;
}

function ing(c, state) {
	const open = state.open.has(c.key);
	const took = state.taken.has(c.key);
	const x = c.own ?? c.other;
	if (c.ghost) return ghostSlab(c, state, `<p class="c-slab-t">${esc(x.t)}</p>`);
	return `<li class="ing c-diff ${open ? 'is-open' : ''} ${took ? 'is-took' : ''}" data-act="toggle" data-k="${c.key}">
		<span class="line">${esc(x.t)}</span>
		${x.reading ? `<span class="read">${esc(x.reading)}</span>` : ''}
		${open ? peekHtml(c, state.taken) : ''}
	</li>`;
}

function step(c, state, n) {
	const open = state.open.has(c.key);
	const took = state.taken.has(c.key);
	const x = c.own ?? c.other;
	if (c.ghost) return ghostSlab(c, state, `<p class="c-slab-t c-slab-step">${esc(x.t)}</p>`);
	return `<li class="step c-diff ${open ? 'is-open' : ''} ${took ? 'is-took' : ''}" data-act="toggle" data-k="${c.key}">
		<span class="n">${n}</span>
		<div class="c-body"><p>${esc(x.t)}</p>${open ? peekHtml(c, state.taken) : ''}</div>
	</li>`;
}

// The lever. Not in the page — under it, where your thumb already is.
export function lever(state) {
	const theirs = state.side === 'theirs';
	return `<div class="c-lever">
		<button class="${!theirs ? 'on' : ''}" data-act="side" data-side="mine">
			<span>Yours</span><em>${esc(MINE.when)}</em></button>
		<button class="${theirs ? 'on' : ''}" data-act="side" data-side="theirs">
			<span>${esc(THEM)}’s</span><em>${esc(THEIRS.when)}</em></button>
	</div>`;
}

export function render(state) {
	const theirs = state.side === 'theirs';
	return (
		head(state, '') +
		lists(state, ing, step) +
		tail(
			state,
			theirs
				? `${esc(THEM)}’s recipe, whole and cookable. The indented slabs are <b>Ghosts</b> — lines
				   of yours he hasn’t got, parked where they sit in your recipe but plainly outside his
				   method, so nothing on the slab can be misread as a step to follow.`
				: `The switch is under your thumb and stays there for the whole scroll.
				   <b>Judge the slabs:</b> pulling a Ghost out of the list makes it unmistakable — but
				   does the recipe still read as one sequence, or does it keep getting interrupted?`
		)
	);
}
