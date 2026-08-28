// PROTOTYPE — everything the three directions have in common, so the only thing
// that differs between A, B and C is the SWITCH and how a GHOST reads.

import { esc, RECIPE, MINE, THEIRS, BRANCH_POINT, ME, THEM, THEIR_KITCHEN } from './data.js';
import { pairing } from './pair.js';

export const V = { mine: MINE, theirs: THEIRS };

// The Pairing is read once, from the two Branches and their Branch Point, and
// every direction renders the same rows. Nothing below knows how it was worked
// out, which is the point: the reader is not a test seam or a design choice.
const ROWS = {
	ings: pairing(BRANCH_POINT.ingredients, MINE.ingredients, THEIRS.ingredients),
	steps: pairing(BRANCH_POINT.steps, MINE.steps, THEIRS.steps),
};
export const rows = ROWS;

export const differing = (list) => list.filter((r) => r.state !== 'same').length;

// ---- one row, seen from wherever you are standing ------------------------
// `own` is the line the recipe you are IN has here; when it is null, the recipe
// you are in hasn't got this line, and what goes on the page is a Ghost.

export function look(row, side, key) {
	const mineSide = side === 'mine';
	const own = mineSide ? row.mine : row.theirs;
	const other = mineSide ? row.theirs : row.mine;
	return {
		row,
		key,
		side,
		own,
		other,
		ghost: !own,
		// A Branch Point line the other side dropped, versus a line the other side
		// wrote after you parted. Same Ghost; different sentence.
		theyRemoved: !!(row.base && !other === false && !own),
		kind: row.kind,
	};
}

// What Kamosu says about a Ghost, in words, when it is asked. One sentence, and
// it never hedges: by the time you read it both texts are already on the page.
export function ghostSentence(c) {
	const theirsSide = c.side === 'theirs';
	if (c.row.base) {
		return theirsSide
			? `${THEM} took this out of his. You still have it in yours.`
			: `You have this. ${THEM} took it out of his.`;
	}
	return theirsSide
		? `You wrote this after you and ${THEM} parted. He hasn't got it.`
		: `${THEM} wrote this after you parted. You haven't got it.`;
}

// The offer, and where it lives: only on Marc's page. Reading his recipe is
// where you would want a line of it; his Branch is his Kitchen's, so nothing you
// do here writes to it (ADR 0007) — every offer writes into YOUR recipe.
export function offer(c, taken) {
	if (c.side !== 'theirs') return null;
	const has = taken.has(c.key);
	if (has) return { kind: 'undo', label: 'Put mine back' };
	if (c.ghost) {
		// A line of yours he hasn't got. Only offerable when he actually took it
		// out; when it is one you added after parting there is nothing to carry.
		return c.row.base ? { kind: 'remove', label: 'Take it out of mine as well' } : null;
	}
	if (c.row.state === 'changed') return { kind: 'write', label: 'Write his into mine' };
	if (c.row.state === 'only-theirs') return { kind: 'write', label: 'Write this into mine' };
	return null;
}

export function peekHtml(c, taken) {
	const o = offer(c, taken);
	const heading = c.ghost ? 'What happened' : c.side === 'theirs' ? 'Yours' : `${THEM}’s`;
	const said = c.ghost ? ghostSentence(c) : c.other ? esc(c.other.t) : ghostSentence(c);
	return `<div class="peek">
		<p class="peek-k">${heading}</p>
		<p class="peek-t">${said}</p>
		${
			o
				? `<button class="take ${o.kind === 'undo' ? 'off' : ''}" data-act="${o.kind === 'undo' ? 'untake' : 'take'}"
					 data-k="${c.key}" data-kind="${o.kind}">${o.label}</button>`
				: c.side === 'mine'
					? `<p class="locked">Switch to ${esc(THEM)}’s to carry it across.</p>`
					: `<p class="locked">Nothing to carry — this one is yours already.</p>`
		}
	</div>`;
}

// ---- the recipe chrome, identical everywhere ----------------------------

export function head(state, switchHtml) {
	const theirs = state.side === 'theirs';
	const v = theirs ? THEIRS : MINE;
	return `<div class="hero"><span>醸</span></div>
		${switchHtml}
		<div class="sheet"><div class="pad">
			<p class="eyebrow">From ${esc(RECIPE.source)}</p>
			<h1 class="title">${esc(RECIPE.title)}</h1>
			<div class="meta">
				<div><b>${RECIPE.prep}</b><span>min prep</span></div>
				<div><b>${RECIPE.cook}</b><span>min cook</span></div>
				<div><b>${RECIPE.serves}</b><span>servings</span></div>
			</div>
		</div></div>`;
}

export function plainIng(x) {
	if (x.kind === 'sec') return `<li class="sec">${esc(x.t)}</li>`;
	return `<li class="ing"><span class="line">${esc(x.t)}</span>${
		x.reading ? `<span class="read">${esc(x.reading)}</span>` : ''
	}</li>`;
}

export function tail(state, footHtml) {
	const theirs = state.side === 'theirs';
	return `<button class="cta">Cook ${theirs ? `${esc(THEM)}’s` : 'this'}</button>
		<button class="cta quiet">The thread</button>
		<p class="foot">${footHtml}</p>`;
}

export function note(state) {
	const v = state.side === 'theirs' ? THEIRS : MINE;
	return v.note ? `<div class="note">${esc(v.note)}</div>` : '';
}

// ---- the two lists, built once ------------------------------------------
// A direction supplies how ONE row looks; the walk over the rows, the numbering
// and the section headings are the same in all three. A Ghost step takes no
// number, because it is not a step of the recipe you are standing in.

export function lists(state, renderIng, renderStep) {
	// Marking off: the recipe you are standing in, plain. Every Ghost goes —
	// a Ghost is a line of the OTHER recipe, so with the marking off it has no
	// business on the page at all — and what is left is exactly the list you
	// would shop from and cook.
	if (state.marks === false) {
		let pn = 0;
		return `<h2 class="h">Ingredients</h2>
			<div class="pad"><ul class="ings">${ROWS.ings
				.map((row) => look(row, state.side, ''))
				.filter((c) => c.own)
				.map((c) => plainIng(c.own))
				.join('')}</ul></div>
			<h2 class="h">Method</h2>
			<div class="pad"><ol class="steps">${ROWS.steps
				.map((row) => look(row, state.side, ''))
				.filter((c) => c.own)
				.map((c) => `<li class="step"><span class="n">${++pn}</span><p>${esc(c.own.t)}</p></li>`)
				.join('')}</ol></div>
			${note(state)}`;
	}

	const ings = ROWS.ings
		.map((row, i) => {
			const c = look(row, state.side, `i${i}`);
			if (c.kind === 'sec') return `<li class="sec">${esc((c.own ?? c.other).t)}</li>`;
			if (row.state === 'same') return plainIng(c.own);
			return renderIng(c, state);
		})
		.join('');

	let n = 0;
	const steps = ROWS.steps
		.map((row, i) => {
			const c = look(row, state.side, `s${i}`);
			if (!c.ghost) n += 1;
			if (row.state === 'same')
				return `<li class="step"><span class="n">${n}</span><p>${esc(c.own.t)}</p></li>`;
			return renderStep(c, state, c.ghost ? null : n);
		})
		.join('');

	return `<h2 class="h">Ingredients<em>${differing(ROWS.ings)} not shared</em></h2>
		<div class="pad"><ul class="ings">${ings}</ul></div>
		<h2 class="h">Method<em>${differing(ROWS.steps)} not shared</em></h2>
		<div class="pad"><ol class="steps">${steps}</ol></div>
		${note(state)}`;
}

// ---- carrying a line across, and saving ---------------------------------

// What a taken line becomes on your recipe, and the prose it pre-fills when you
// save. Never a structured pointer — a sentence a person would have written.
export function proseFor(taken) {
	const phrase = (t) =>
		t.isStep ? `the ${t.gist} step` : t.text.length > 42 ? t.text.slice(0, 39).trimEnd() + '…' : t.text;
	const join = (xs) => (xs.length === 1 ? xs[0] : xs.slice(0, -1).join(', ') + ' and ' + xs.at(-1));

	const all = [...taken.values()];
	const wrote = all.filter((t) => t.kind !== 'remove').map(phrase);
	const removed = all.filter((t) => t.kind === 'remove').map(phrase);

	const sentences = [];
	if (wrote.length) sentences.push(`Took ${join(wrote)} from ${THEIR_KITCHEN}.`);
	if (removed.length) sentences.push(`Took out ${join(removed)}, as ${THEM} has.`);
	return sentences.join(' ');
}

export function trayHtml(state) {
	const n = state.taken.size;
	if (!n) return '';
	return `<div class="tray">
		<p>${n} line${n > 1 ? 's' : ''} from ${esc(THEM)}, sitting on your recipe. Not saved.</p>
		<div class="tray-row">
			<button data-act="save">Save a Version</button>
			<button class="quiet" data-act="dropAll">Undo</button>
		</div>
	</div>`;
}

export function saveModalHtml(state) {
	return `<div class="scrim" data-act="closeSave"></div>
		<div class="modal" role="dialog" aria-modal="true">
			<h3>Save a Version</h3>
			<label for="wc">What changed</label>
			<textarea id="wc" rows="3">${esc(proseFor(state.taken))}</textarea>
			<p class="hint">Written for you as a sentence, and yours to rewrite. Kamosu stores what you
			say happened, not a pointer at ${esc(THEM)}’s recipe — his Branch could be gone tomorrow and
			this line would still mean something.</p>
			<button class="cta" data-act="closeSave">Save</button>
			<button class="cta quiet" data-act="closeSave">Cancel</button>
		</div>`;
}

export { esc, ME, THEM, THEIR_KITCHEN, MINE, THEIRS, RECIPE };
