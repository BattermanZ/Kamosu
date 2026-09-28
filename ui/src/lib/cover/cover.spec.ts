/**
 * The Cover's guarantees (#46).
 *
 * These are not tests of how a Cover looks — taste is Aurélien's and is
 * recorded on the issue. They are tests of the four promises the ticket makes
 * about a Cover, each of which is a property of `coverFor` alone.
 */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { coverFor, DYES, GROUND, INK } from './cover';
import { PASTA_SHAPES } from './shapes';

/** Lineage ids in Kamosu's real format: `l_` and eight bytes of hex. */
function lineageIds(count: number, seed = 1): string[] {
	// A small deterministic generator, so a failure is reproducible rather than
	// a different set of ids every run.
	let state = seed >>> 0;
	const next = () => {
		state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
		return state;
	};
	return Array.from(
		{ length: count },
		() =>
			'l_' +
			Array.from({ length: 4 }, () => next().toString(16).padStart(8, '0').slice(0, 4)).join(''),
	);
}

describe('a Cover is the same wherever it is drawn', () => {
	it('gives the identical Cover for the identical Lineage id', () => {
		for (const id of lineageIds(200)) {
			expect(coverFor(id)).toEqual(coverFor(id));
		}
	});

	it('reads the Lineage id and nothing else, so a rename cannot change it', () => {
		// The signature takes only the Lineage id, so this is really a test that
		// nobody has quietly added a second argument. It is worth keeping: the
		// title is the obvious thing to reach for when drawing a Cover, and
		// reaching for it would silently break every renamed recipe.
		expect(coverFor.length).toBe(1);
	});

	it('gives a Translation the same Cover as its source', () => {
		// A Translation is a Branch of the same Lineage carrying another
		// Language (ADR 0006). Nothing about the Branch reaches this function,
		// so the two cannot disagree.
		const lineage = 'l_2a7323feaf99ff8a';
		expect(coverFor(lineage)).toEqual(coverFor(lineage));
	});

	it('gives different Lineages different Covers', () => {
		const faces = new Set(
			lineageIds(400).map((id) => {
				const cover = coverFor(id);
				return `${cover.dye.key}/${cover.shape.key}/${cover.rotation}`;
			}),
		);
		// 400 ids over 64 dye-and-shape pairs, each with a continuous rotation:
		// collisions on the whole face should be vanishingly rare.
		expect(faces.size).toBeGreaterThan(395);
	});
});

describe('a Cover is drawable', () => {
	it('always lands on a real dye and a real shape', () => {
		for (const id of lineageIds(500)) {
			const cover = coverFor(id);
			expect(DYES).toContain(cover.dye);
			expect(PASTA_SHAPES).toContain(cover.shape);
		}
	});

	it('keeps scale, rotation and placement inside their ranges', () => {
		for (const id of lineageIds(500)) {
			const { scale, rotation, centreX, centreY } = coverFor(id);
			expect(scale).toBeGreaterThanOrEqual(0.66);
			expect(scale).toBeLessThan(1.02);
			expect(rotation).toBeGreaterThanOrEqual(0);
			expect(rotation).toBeLessThan(360);
			expect(centreX).toBeGreaterThanOrEqual(0.34);
			expect(centreX).toBeLessThan(0.66);
			expect(centreY).toBeGreaterThanOrEqual(0.24);
			expect(centreY).toBeLessThan(0.44);
		}
	});

	it('draws every shape from closed subpaths and nothing else', () => {
		for (const shape of PASTA_SHAPES) {
			expect(shape.body.length).toBeGreaterThan(0);
			for (const d of [...shape.body, ...shape.detail]) {
				expect(d.trimEnd().endsWith('Z')).toBe(true);
				// Only absolute move/line/curve commands: a stray transform or
				// relative hop is what silently breaks a rotated, scaled Cover.
				expect(d).toMatch(/^[MCQLZ0-9\s.,-]+$/);
			}
		}
	});
});

describe('a Cover is painted in the app’s own colours', () => {
	// `cover.ts` has to hold these two as real numbers to blend with, so they
	// are a hand-written copy of two design tokens. This is the guard that stops
	// the copy going stale: repaint `ui/src/app.css`, and this fails rather than
	// leaving every Cover quietly wearing last season's palette.
	const stylesheet = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8');

	const token = (name: string): string => {
		const found = stylesheet.match(new RegExp(`--${name}:\\s*(#[0-9a-fA-F]{3,8})`));
		if (!found) throw new Error(`no --${name} token in ui/src/app.css`);
		return found[1].toLowerCase();
	};

	it('blends towards the ground and ink the rest of the app uses', () => {
		expect(GROUND.toLowerCase()).toBe(token('color-ground'));
		expect(INK.toLowerCase()).toBe(token('color-ink'));
	});
});

describe('a shelf of Covers looks like a shelf', () => {
	// The first draft of this hashed with plain FNV-1a, whose low bits are
	// poorly mixed: on twelve real recipes one dye landed five times and one
	// shape landed five times. Determinism alone would have passed. These
	// measure the thing that actually went wrong.
	const ids = lineageIds(40000, 7);

	it('spreads the dyes evenly', () => {
		const counts = new Map<string, number>();
		for (const id of ids) {
			const key = coverFor(id).dye.key;
			counts.set(key, (counts.get(key) ?? 0) + 1);
		}
		expect(counts.size).toBe(DYES.length);
		const expected = ids.length / DYES.length;
		for (const [key, count] of counts) {
			expect(Math.abs(count - expected) / expected, `dye ${key}`).toBeLessThan(0.05);
		}
	});

	it('spreads the shapes evenly', () => {
		const counts = new Map<string, number>();
		for (const id of ids) {
			const key = coverFor(id).shape.key;
			counts.set(key, (counts.get(key) ?? 0) + 1);
		}
		expect(counts.size).toBe(PASTA_SHAPES.length);
		const expected = ids.length / PASTA_SHAPES.length;
		for (const [key, count] of counts) {
			expect(Math.abs(count - expected) / expected, `shape ${key}`).toBeLessThan(0.05);
		}
	});

	it('reaches every dye-and-shape pairing rather than only some', () => {
		const pairs = new Set(
			ids.map((id) => {
				const cover = coverFor(id);
				return `${cover.dye.key}/${cover.shape.key}`;
			}),
		);
		expect(pairs.size).toBe(DYES.length * PASTA_SHAPES.length);
	});

	it('does not correlate the dye with the shape', () => {
		// Independent choices, not one hash sliced twice: if dye and shape moved
		// together a library would show only eight faces instead of sixty-four.
		const perDye = new Map<string, Set<string>>();
		for (const id of ids) {
			const cover = coverFor(id);
			const seen = perDye.get(cover.dye.key) ?? new Set<string>();
			seen.add(cover.shape.key);
			perDye.set(cover.dye.key, seen);
		}
		for (const [dye, shapes] of perDye) {
			expect(shapes.size, `dye ${dye}`).toBe(PASTA_SHAPES.length);
		}
	});
});
