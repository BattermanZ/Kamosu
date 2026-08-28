// PROTOTYPE — throwaway. Router, state, variant switcher. Issue #55.

import * as A from './a.js';
import * as B from './b.js';
import * as C from './c.js';
import * as D from './d.js';
import { rows, trayHtml, saveModalHtml, look, esc, THEM } from './shared.js';
import { explain } from './pair.js';

const VARIANTS = {
	d: { render: D.render, name: 'CHOSEN — the threshold, captioned Ghosts, and an off switch' },
	a: { render: A.render, name: 'Two tabs — a caption on every marked line' },
	b: { render: B.render, name: 'Two rooms — the page changes, the Ghost stays quiet' },
	c: { render: C.render, name: 'A lever under your thumb, the Ghost on a slab' },
};

const params = new URLSearchParams(location.search);

const state = {
	v: VARIANTS[params.get('v')] ? params.get('v') : 'd',
	side: params.get('side') === 'theirs' ? 'theirs' : 'mine',
	marks: params.get('marks') !== 'off',
	open: new Set(),
	taken: new Map(),
	saving: false,
	pairing: false,
};

const app = document.getElementById('app');
const bar = document.getElementById('bar');

function render() {
	document.body.dataset.variant = state.v;
	document.body.dataset.side = state.side;
	const scroll = window.scrollY;
	let html = VARIANTS[state.v].render(state);
	if (state.v === 'c') html += C.lever(state);
	html += trayHtml(state);
	if (state.saving) html += saveModalHtml(state);
	if (state.pairing) html += pairingPanel();
	app.innerHTML = `<div class="wrap">${html}</div>`;
	history.replaceState(
		null,
		'',
		'?' + new URLSearchParams({ v: state.v, side: state.side, marks: state.marks ? 'on' : 'off' })
	);
	window.scrollTo(0, scroll);
}

// ---- carrying a line across ---------------------------------------------

function rowFor(key) {
	const list = key[0] === 'i' ? rows.ings : rows.steps;
	return list[Number(key.slice(1))];
}

const actions = {
	side: (el) => {
		state.side = el.dataset.side;
		state.open.clear();
	},
	marks: () => {
		state.marks = !state.marks;
		state.open.clear();
	},
	toggle: (el) => {
		const k = el.dataset.k;
		state.open.has(k) ? state.open.delete(k) : state.open.add(k);
	},
	take: (el) => {
		const k = el.dataset.k;
		const row = rowFor(k);
		const c = look(row, state.side, k);
		const isRemove = el.dataset.kind === 'remove';
		// On a Ghost the line is by definition the OTHER side's — the recipe you
		// are standing in hasn't got it. That is the whole point of a Ghost.
		const line = c.own ?? c.other;
		state.taken.set(k, {
			kind: isRemove ? 'remove' : 'write',
			text: line.t,
			isStep: row.kind === 'step',
			gist: gistOf(line.t),
		});
	},
	untake: (el) => state.taken.delete(el.dataset.k),
	dropAll: () => state.taken.clear(),
	save: () => (state.saving = true),
	closeSave: () => (state.saving = false),
};

// A step is too long to name in a sentence, so it is named by what it is about.
// Crude on purpose — the point being tested is that "what changed" is PROSE.
function gistOf(t) {
	const words = t.toLowerCase();
	if (words.includes('marinat') || words.includes('coat the chicken')) return 'marinade';
	if (words.includes('air fry') || words.includes('fry')) return 'frying';
	if (words.includes('sit') || words.includes('rest')) return 'resting';
	if (words.includes('sauce')) return 'sauce';
	return 'method';
}

app.addEventListener('click', (e) => {
	const el = e.target.closest('[data-act]');
	if (!el) return;
	const fn = actions[el.dataset.act];
	if (!fn) return;
	e.preventDefault();
	e.stopPropagation();
	fn(el);
	render();
});

// ---- scaffolding: what the Pairing reader concluded, and why -------------
// Not part of any design being judged. It exists so "no id is stapled to a line"
// can be checked instead of believed.

function pairingPanel() {
	const block = (title, list) => `<h4>${title}</h4><ul>${explain(list)
		.map(
			(e) => `<li><b>${esc(e.verdict)}</b>
				<span>${e.mine ? `yours: ${esc(e.mine)}` : '<i>nothing of yours</i>'}</span>
				<span>${e.theirs ? `${esc(THEM)}: ${esc(e.theirs)}` : `<i>nothing of ${esc(THEM)}’s</i>`}</span>
				<em>${esc(e.why)}</em></li>`
		)
		.join('')}</ul>`;
	return `<div class="scrim" data-act="none"></div>
		<div class="pairpanel">
			<p class="pairpanel-h">Scaffolding — not part of the design. What the Pairing reader worked
			out from the words alone, against the Branch Point of 14 Mar 2026. No line in this fixture
			carries an id.</p>
			${block('Ingredients', rows.ings)}
			${block('Method', rows.steps)}
		</div>`;
}

// ---- the switcher bar ----------------------------------------------------

function drawBar() {
	bar.innerHTML = `
		<button data-cycle="-1" aria-label="Previous direction">‹</button>
		<span class="lbl"><b>${state.v.toUpperCase()}</b><em> — ${VARIANTS[state.v].name}</em></span>
		<button data-cycle="1" aria-label="Next direction">›</button>
		<span class="sep"></span>
		<button class="scr" data-pairing="1" aria-current="${state.pairing}">Pairing</button>`;
}

bar.addEventListener('click', (e) => {
	const cyc = e.target.closest('[data-cycle]');
	const pr = e.target.closest('[data-pairing]');
	const keys = Object.keys(VARIANTS);
	if (cyc) {
		const i = keys.indexOf(state.v);
		state.v = keys[(i + Number(cyc.dataset.cycle) + keys.length) % keys.length];
		state.open.clear();
	} else if (pr) {
		state.pairing = !state.pairing;
	} else return;
	drawBar();
	render();
});

addEventListener('keydown', (e) => {
	if (e.target.matches('input, textarea, [contenteditable]')) return;
	const keys = Object.keys(VARIANTS);
	if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
		const i = keys.indexOf(state.v);
		state.v = keys[(i + (e.key === 'ArrowRight' ? 1 : -1) + keys.length) % keys.length];
		state.open.clear();
		drawBar();
		render();
	}
	if (e.key === 'Escape') {
		state.pairing = false;
		state.saving = false;
		drawBar();
		render();
	}
});

drawBar();
render();
