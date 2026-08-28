// DIRECTION B — "Two rooms, and a quiet Ghost."
//
// The switch is not a control, it is a THRESHOLD. A band across the top names
// the kitchen you are standing in, and crossing to the other one is a single
// wide target that says where it goes: `Chez Marc →`. The paper, the rule
// colours and the spine down the sheet all change with the room, so at any point
// in a long scroll you know whose recipe you are in without looking up.
//
// And because the room already says whose recipe this is, no line has to. A
// Ghost is struck through and dimmed, marked only by a dot in the margin in the
// other person's colour. Nothing is captioned until you ask.
//
// The bet: a recipe should look like a recipe. Kamosu marks the divergence with
// the smallest thing that can carry it and holds the words back until tapped.
//
// The risk it takes on purpose: a dot is not a sentence. Someone who has never
// seen a Ghost may not know a struck-through line is a real line of a real
// recipe rather than something crossed off a list.

import { esc, look, peekHtml, head, tail, lists, THEM, ME, MINE, THEIRS } from './shared.js';

function ing(c, state) {
	const open = state.open.has(c.key);
	const took = state.taken.has(c.key);
	const x = c.own ?? c.other;
	return `<li class="ing b-row ${c.ghost ? 'b-ghost' : 'b-diff'} ${open ? 'is-open' : ''} ${took ? 'is-took' : ''}"
		data-act="toggle" data-k="${c.key}">
		<span class="line">${esc(x.t)}</span>
		${x.reading ? `<span class="read">${esc(x.reading)}</span>` : ''}
		${open ? peekHtml(c, state.taken) : ''}
	</li>`;
}

function step(c, state, n) {
	const open = state.open.has(c.key);
	const took = state.taken.has(c.key);
	const x = c.own ?? c.other;
	return `<li class="step b-row ${c.ghost ? 'b-ghost' : 'b-diff'} ${open ? 'is-open' : ''} ${took ? 'is-took' : ''}"
		data-act="toggle" data-k="${c.key}">
		<span class="n">${n ?? ''}</span>
		<div class="b-body">
			<p>${esc(x.t)}</p>
			${open ? peekHtml(c, state.taken) : ''}
		</div>
	</li>`;
}

// The threshold. Two halves: where you are, and the way across.
function switchHtml(state) {
	const theirs = state.side === 'theirs';
	const here = theirs ? THEIRS : MINE;
	const there = theirs ? MINE : THEIRS;
	return `<div class="b-thresh">
		<div class="b-here">
			<p class="b-here-k">You are in</p>
			<p class="b-here-t">${esc(here.kitchen)}</p>
		</div>
		<button class="b-cross" data-act="side" data-side="${theirs ? 'mine' : 'theirs'}">
			<span class="b-cross-k">Cross to</span>
			<span class="b-cross-t">${esc(there.kitchen)} <em>→</em></span>
		</button>
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
				? `The paper has gone cool and the spine is red: you are in <b>${esc(THEM)}’s kitchen</b>,
				   reading his recipe whole. The struck-through lines are <b>Ghosts</b> — lines of yours
				   he hasn’t got. Nothing is labelled; tap a marked line and Kamosu says what happened.`
				: `Warm paper, indigo spine: your own recipe. <b>Judge the quiet:</b> with no captions at
				   all, can you tell a Ghost from a line you have crossed off — and does the room alone
				   keep you sure whose recipe you are reading, six screens down?`
		)
	);
}
