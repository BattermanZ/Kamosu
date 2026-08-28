/**
 * The Cover: what leads a recipe that has no Main Photo (#46).
 *
 * A Cover is a dyed ground carrying one large pasta shape, with a solid band
 * at the hem holding the recipe's title. Everything about how it looks —
 * which dye, which shape, how big, how turned, where placed — is derived from
 * the recipe's **Lineage id and nothing else**.
 *
 * That single rule is what the ticket's guarantees rest on:
 *
 * - a rename never changes a recipe's face, because the title is not an input;
 * - a Translation wears its source's face, because a Translation is a Branch
 *   of the same Lineage (ADR 0006) and Branch ids are not an input;
 * - the same Lineage wears the same face on every instance it reaches, because
 *   there is no per-instance state here at all.
 *
 * A Cover is generated at the moment of drawing. It is never stored as a
 * Photograph, is in no fingerprint and travels in no Bundle — there is nothing
 * to store, because this file *is* the Cover.
 *
 * Chosen by Aurélien on 2026-08-28 after four rounds of options; the reasoning
 * is recorded on issue #46.
 */
import { PASTA_SHAPES, type PastaShape } from './shapes';

/**
 * The dyes, in the identity's own register (see
 * `docs/design/2026-08-26-the-visual-identity.md`). All eight are deep enough
 * to carry kinari text at the type sizes the band uses.
 *
 * Order is part of the contract, exactly as it is for the shapes: a Cover is
 * chosen by index, so reordering this list repaints every existing recipe.
 */
export interface Dye {
	readonly key: string;
	/** The traditional name, for tests and for people reading the code. */
	readonly name: string;
	readonly hex: string;
}

export const DYES: readonly Dye[] = [
	{ key: 'ai', name: 'ai · indigo', hex: '#1d2b4c' },
	{ key: 'beni', name: 'beni · safflower', hex: '#8f3a3e' },
	{ key: 'matsuba', name: 'matsuba · pine', hex: '#42553a' },
	{ key: 'kuchiba', name: 'kuchiba · fallen leaf', hex: '#7a5327' },
	{ key: 'murasaki', name: 'murasaki · purple', hex: '#453a5c' },
	{ key: 'nando', name: 'nando · teal', hex: '#1c4a48' },
	{ key: 'sumi', name: 'sumi · charcoal', hex: '#33302c' },
	{ key: 'enji', name: 'enji · deep rose', hex: '#68304a' }
];

/**
 * The two ends a dye is blended towards: kinari, the app's unbleached-paper
 * ground, and sumi, its ink.
 *
 * These are the `--color-ground` and `--color-ink` design tokens, written out
 * again here because the blending below is arithmetic and needs real numbers,
 * not CSS custom properties that only exist once a browser has resolved them.
 * That duplication is the risk: repaint the palette in `ui/src/app.css`, run
 * `just css`, and every Cover would quietly keep the old shades. `cover.spec.ts`
 * reads the stylesheet and fails if these two drift from it, so the copy cannot
 * go stale silently.
 */
export const GROUND = '#f4efe3';
export const INK = '#1a1a1c';

/**
 * FNV-1a with an avalanche finaliser.
 *
 * The finaliser is not decoration. Plain FNV-1a leaves its low bits poorly
 * mixed, and since every choice below is a remainder of those low bits, the
 * first draft of this put the same dye on five of twelve recipes and the same
 * shape on five of twelve. Spreading every input bit across the whole word
 * before anything is sliced off it costs three instructions and is what makes
 * a shelf look like a shelf. `cover.spec.ts` measures the distribution rather
 * than trusting it.
 */
function hash(text: string): number {
	let h = 0x811c9dc5;
	for (let i = 0; i < text.length; i++) {
		h ^= text.charCodeAt(i);
		h = Math.imul(h, 0x01000193) >>> 0;
	}
	h ^= h >>> 16;
	h = Math.imul(h, 0x7feb352d) >>> 0;
	h ^= h >>> 15;
	h = Math.imul(h, 0x846ca68b) >>> 0;
	h ^= h >>> 16;
	return h >>> 0;
}

/** One independent choice from the Lineage id: an index below `count`. */
const index = (lineageId: string, aspect: string, count: number): number =>
	hash(`${lineageId}:${aspect}`) % count;

/** One independent choice from the Lineage id: a fraction in [0, 1). */
const unit = (lineageId: string, aspect: string): number =>
	hash(`${lineageId}:${aspect}`) / 4294967296;

/** Blend two hex colours. `t` of 0 is all `from`, 1 is all `to`. */
function blend(from: string, to: string, t: number): string {
	const parts = (hex: string) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
	const [fr, fg, fb] = parts(from);
	const [tr, tg, tb] = parts(to);
	const channel = (a: number, b: number) =>
		Math.round(a + (b - a) * t)
			.toString(16)
			.padStart(2, '0');
	return `#${channel(fr, tr)}${channel(fg, tg)}${channel(fb, tb)}`;
}

/** Everything needed to draw one Cover, and nothing that varies by screen. */
export interface Cover {
	readonly dye: Dye;
	/** The ground the whole Cover sits on. */
	readonly ground: string;
	/** The shape's fill: the dye lifted towards kinari. */
	readonly tone: string;
	/** The hem band behind the title: the dye pushed towards sumi. */
	readonly band: string;
	readonly shape: PastaShape;
	/** The shape's size as a multiple of the Cover's height. */
	readonly scale: number;
	/** Degrees, clockwise, about the shape's own centre. */
	readonly rotation: number;
	/** The shape's centre, as fractions of the Cover's width and height. */
	readonly centreX: number;
	readonly centreY: number;
}

/**
 * The one function. Same Lineage id in, same Cover out, for ever.
 *
 * The scale and placement ranges were fitted by eye against the twelve real
 * photograph-less recipes in Aurélien's Crouton export: large enough that a
 * drawn curve is allowed to be a curve, contained enough that you can still
 * tell which pasta it is. Some Covers bleed off an edge; most do not.
 */
export function coverFor(lineageId: string): Cover {
	const dye = DYES[index(lineageId, 'dye', DYES.length)];
	return {
		dye,
		ground: dye.hex,
		tone: blend(dye.hex, GROUND, 0.19),
		band: blend(dye.hex, INK, 0.22),
		shape: PASTA_SHAPES[index(lineageId, 'shape', PASTA_SHAPES.length)],
		scale: 0.66 + unit(lineageId, 'scale') * 0.36,
		rotation: unit(lineageId, 'rotation') * 360,
		centreX: 0.34 + unit(lineageId, 'placeX') * 0.32,
		centreY: 0.24 + unit(lineageId, 'placeY') * 0.2
	};
}
