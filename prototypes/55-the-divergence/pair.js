// PROTOTYPE — the Pairing reader (ADR 0019), for real this time.
//
// #18's prototype cheated: every line carried a hand-written `key`, so "Marc's
// ¾ cup potato starch is your 1 cup potato starch, changed" was asserted rather
// than worked out. ADR 0019 then refused to staple an id to a line at all. So
// this file does the thing the ADR promised — it reads the two Branches against
// their BRANCH POINT and decides, from the words alone, which line is which.
//
// The three rules it is built from:
//
//   1. The Branch Point makes the guessing surface small. A line neither side
//      touched is identical on both sides and is matched exactly, for free. Only
//      lines somebody actually edited need reading — one to three, typically.
//   2. Kind is part of identity. A section heading never pairs with an
//      ingredient; a step never pairs with either.
//   3. When it is unsure, it DECLINES. Two lines that both arrived after the
//      Branch Point are only paired if they genuinely resemble each other.
//      Otherwise both stand, unjoined, and nothing is labelled a guess.

// How alike two lines have to be. Two different numbers on purpose:
//
//  · EDITED — one known Branch-Point line against a line that survived on this
//    side. The candidate pool is tiny and already position-ordered, so a fairly
//    generous bar is safe: `1 cup potato starch (or corn starch)` → `¾ cup potato
//    starch` must land, and it is only a 0.6-ish match.
//
//  · ARRIVED — a line that is new on MY side against a line that is new on
//    MARC'S. Nothing anchors these to each other, so the bar is high. This is
//    the number that decides whether `1 Tbsp rice vinegar` and `1 tsp gochugaru`
//    get falsely joined. ADR 0019: a bad Pairing is much worse than none.
const EDITED = 0.5;
const ARRIVED = 0.72;

const norm = (s) => String(s).toLowerCase().replace(/\s+/g, ' ').trim();

function bigrams(s) {
	const t = norm(s);
	const out = new Map();
	for (let i = 0; i < t.length - 1; i++) {
		const g = t.slice(i, i + 2);
		out.set(g, (out.get(g) ?? 0) + 1);
	}
	return out;
}

// Sørensen–Dice over character bigrams: 0 = nothing in common, 1 = identical.
// Character bigrams rather than words because the edits that matter here are
// mostly inside a word or a number — `¾ cup` for `1 cup` shares almost every
// bigram of `cup potato starch`, which is exactly the signal we want.
function dice(a, b) {
	if (norm(a) === norm(b)) return 1;
	const A = bigrams(a);
	const B = bigrams(b);
	let shared = 0;
	let total = 0;
	for (const n of A.values()) total += n;
	for (const [g, n] of B) {
		total += n;
		shared += Math.min(n, A.get(g) ?? 0);
	}
	return total === 0 ? 0 : (2 * shared) / total;
}

// ---- step one: where did each of this side's lines come from? -------------
// Returns an array parallel to `side`: the index in `base` each line descends
// from, or -1 for a line that arrived after the Branch Point.

function origins(base, side) {
	const from = new Array(side.length).fill(-1);
	const usedBase = new Set();

	// Pass one — untouched lines, matched exactly. Most of the list, always.
	side.forEach((item, i) => {
		for (let b = 0; b < base.length; b++) {
			if (usedBase.has(b) || base[b].kind !== item.kind) continue;
			if (norm(base[b].t) !== norm(item.t)) continue;
			from[i] = b;
			usedBase.add(b);
			return;
		}
	});

	// Pass two — the handful somebody edited, matched by reading. Best match
	// wins, and only if it clears the bar; the rest are genuinely new lines.
	side.forEach((item, i) => {
		if (from[i] !== -1) return;
		let best = -1;
		let bestScore = EDITED;
		for (let b = 0; b < base.length; b++) {
			if (usedBase.has(b) || base[b].kind !== item.kind) continue;
			const score = dice(base[b].t, item.t);
			if (score > bestScore) {
				bestScore = score;
				best = b;
			}
		}
		if (best !== -1) {
			from[i] = best;
			usedBase.add(best);
		}
	});

	return from;
}

// ---- step two: lay the two sides over each other --------------------------
// A row is one place in the merged recipe. `mine` and `theirs` are the line each
// side has there; either may be null, and a null is what a Ghost is made of.

export function pairing(base, mine, theirs) {
	const fromMine = origins(base, mine);
	const fromTheirs = origins(base, theirs);

	const mineByBase = new Map();
	fromMine.forEach((b, i) => b !== -1 && mineByBase.set(b, i));
	const theirsByBase = new Map();
	fromTheirs.forEach((b, i) => b !== -1 && theirsByBase.set(b, i));

	// The spine: every Branch-Point line at least one side still has, in order.
	const rows = [];
	const rowAtBase = new Map();
	for (let b = 0; b < base.length; b++) {
		const mi = mineByBase.get(b);
		const ti = theirsByBase.get(b);
		if (mi === undefined && ti === undefined) continue; // both took it out
		rowAtBase.set(b, rows.length);
		rows.push({
			kind: base[b].kind,
			mine: mi === undefined ? null : mine[mi],
			theirs: ti === undefined ? null : theirs[ti],
			base: base[b],
		});
	}

	// The lines that arrived after the Branch Point, each remembering the last
	// line above it that both sides can still locate. That anchor is what puts a
	// Ghost "in the position it holds over there" — a best effort, and ADR 0014
	// says so out loud.
	const arrivals = (side, from) => {
		const out = [];
		let anchor = -1;
		side.forEach((item, i) => {
			if (from[i] !== -1) anchor = from[i];
			else out.push({ item, anchor });
		});
		return out;
	};

	const newMine = arrivals(mine, fromMine);
	const newTheirs = arrivals(theirs, fromTheirs);

	// Do any of my new lines and any of Marc's turn out to be the same line, both
	// of us having written it after we parted? Only if they really look it.
	const claimed = new Set();
	const extra = [];
	for (const m of newMine) {
		let best = null;
		let bestScore = ARRIVED;
		for (const t of newTheirs) {
			if (claimed.has(t) || t.item.kind !== m.item.kind) continue;
			const score = dice(m.item.t, t.item.t);
			if (score > bestScore) {
				bestScore = score;
				best = t;
			}
		}
		if (best) claimed.add(best);
		// When `best` is null this is the declining branch: my line stands on its
		// own, Marc's stands on its own, and neither is marked as a maybe.
		extra.push({ kind: m.item.kind, mine: m.item, theirs: best ? best.item : null, base: null, anchor: m.anchor });
	}
	for (const t of newTheirs) {
		if (claimed.has(t)) continue;
		extra.push({ kind: t.item.kind, mine: null, theirs: t.item, base: null, anchor: t.anchor });
	}

	// Slot each arrival in under its anchor, keeping the order it was written in.
	const cursor = new Map();
	for (const row of extra) {
		const at = rowAtBase.has(row.anchor) ? rowAtBase.get(row.anchor) : -1;
		const offset = (cursor.get(row.anchor) ?? 0) + 1;
		cursor.set(row.anchor, offset);
		const target = Math.min(at + offset, rows.length);
		rows.splice(target, 0, row);
		// Everything after the insertion point shifted down by one.
		for (const [b, idx] of rowAtBase) if (idx >= target) rowAtBase.set(b, idx + 1);
	}

	return rows.map((r) => ({ ...r, state: state(r) }));
}

// What this row is, said once so no variant has to work it out again.
//   same    — both sides have it and the words are identical
//   changed — both sides have it and the words are not
//   only-mine / only-theirs — one side has it, which is a Ghost seen from the other
function state(r) {
	if (r.mine && r.theirs) return norm(r.mine.t) === norm(r.theirs.t) ? 'same' : 'changed';
	return r.mine ? 'only-mine' : 'only-theirs';
}

// Everything the reader concluded, in words, for the scaffolding panel. Not part
// of any design being judged — it exists so the claim "no id is stapled to a
// line" can be checked rather than believed.
export function explain(rows) {
	return rows
		.filter((r) => r.state !== 'same')
		.map((r) => {
			const t = (x) => (x ? x.t : null);
			if (r.state === 'changed') return { verdict: 'paired', mine: t(r.mine), theirs: t(r.theirs), why: r.base ? 'both descend from the same Branch Point line' : 'both arrived after the Branch Point and read alike' };
			if (r.base) return { verdict: r.mine ? 'Marc took it out' : 'Marc added it', mine: t(r.mine), theirs: t(r.theirs), why: 'it is a Branch Point line only one side still has' };
			return { verdict: 'left unpaired', mine: t(r.mine), theirs: t(r.theirs), why: 'it arrived after the Branch Point and nothing on the other side resembles it' };
		});
}
