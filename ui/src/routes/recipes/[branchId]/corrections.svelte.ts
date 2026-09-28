/**
 * Correcting a Reading on the recipe screen (#32 item 193), and the one line
 * beneath an Ingredient Line that a correction changes.
 *
 * `set_reading` makes no Version, so there is nothing to refetch and nothing
 * that would show up in the Thread — the line simply reads differently from
 * now on. What was corrected here is laid over what was fetched until the
 * page is read again. `Correcting.svelte` owns why that makes no Version; this
 * owns which line has the corrector open and what each line now says.
 *
 * Everything is keyed by a line's index into the list on screen, so anything
 * that moves those indexes — a save, crossing to the other side of a
 * Divergence, putting the marks away — forgets what is here (`forget`,
 * `close`).
 */
import type { GetRecipeOutput } from '$lib/api/catalogue';
import { reading } from './divergence';

type Version = GetRecipeOutput['versions'][number];
/** One slot of `readings`: what Kamosu understood of a line, or nothing. */
export type Slot = Version['readings'][number];
/**
 * A line corrected here: the Reading as it now stands, and the one
 * subordinate line it now produces. They travel together because
 * `set_reading` answers with both — the conversion is the Core's, and this
 * screen only ever displays it.
 */
type Fixed = { reading: Slot; measured: string | null };

/** What the page is showing, read afresh each time it is asked. */
export interface Page {
	readings(): Slot[];
	/** The converted amount beneath each Ingredient Line, worded by the Core. */
	measured(): Version['measured'];
	/** The written Ingredient Line at this index. */
	written(index: number): string;
}

export interface Corrections {
	/** Which Ingredient Line has the corrector open, by index into the list. */
	readonly correcting: number | null;
	toggle(index: number): void;
	/** Close the corrector, keeping what was corrected. */
	close(): void;
	/** The Reading on one line, with anything corrected here laid over it. */
	readingAt(index: number): Slot;
	/** The one line beneath an Ingredient Line, or '' where there is none. */
	beneathLine(index: number): string;
	/**
	 * Lay a correction over the line and close the corrector. `measured` is
	 * null where the answer's line cannot be shown (see `corrected` on the
	 * recipe screen).
	 */
	lay(index: number, reading: Slot, measured: string | null): void;
	/** The page was read again, and carries every correction made here since. */
	dropOverlay(): void;
	/** The list moved underneath: nothing keyed by its indexes survives. */
	forget(): void;
}

export function corrections(page: Page): Corrections {
	let correcting = $state<number | null>(null);
	let fixed = $state(new Map<number, Fixed>());

	/*
	 * The overlay is keyed by line index into the list on screen, and the two
	 * sides of a Divergence have their own lists — so index 2 on the other
	 * side is a different ingredient entirely. A page stays on one side for
	 * as long as it shows one recipe, and everything that rebuilds the list
	 * calls `forget` or `close`, so a correction is never read through onto
	 * another line.
	 */
	const readingAt = (index: number): Slot =>
		fixed.has(index) ? (fixed.get(index)?.reading ?? null) : (page.readings()[index] ?? null);

	return {
		get correcting() {
			return correcting;
		},
		toggle(index) {
			correcting = correcting === index ? null : index;
		},
		close() {
			correcting = null;
		},
		readingAt,
		/**
		 * **The one line beneath an Ingredient Line**, and the whole of the
		 * rule: the converted amount where this reader needs one, the echo of
		 * what Kamosu read where she does not, and nothing at all where there is
		 * neither. Never both (#49, ADR 0016).
		 *
		 * **The echo is silent where it would only repeat the line above it**,
		 * which is the same rule the conversion already obeys. It was written
		 * when a Reading existed only because somebody had typed one, so an echo
		 * was the only way to see what Kamosu held; since #71 Kamosu reads every
		 * line it can, and an echo that parrots the line is two things it must
		 * not be — a repetition ADR 0016 says must be absent, and a mark on
		 * exactly the lines Kamosu managed to read, which is the badge ADR 0002
		 * refuses. What is left is the echo that earns its place: the Reading
		 * somebody **corrected**, which says something the line does not.
		 *
		 * The overlay is read as `readingAt` reads it, by index into the list
		 * on screen.
		 */
		beneathLine(index) {
			if (index < 0) return '';
			const converted = fixed.has(index)
				? (fixed.get(index)?.measured ?? null)
				: (page.measured().ingredients[index] ?? null);
			if (converted) return converted;
			const echo = reading(readingAt(index));
			return echo && saysMoreThan(echo, page.written(index)) ? echo : '';
		},
		lay(index, reading, measured) {
			const next = new Map(fixed);
			next.set(index, { reading, measured });
			fixed = next;
			correcting = null;
		},
		dropOverlay() {
			fixed = new Map();
		},
		forget() {
			correcting = null;
			fixed = new Map();
		},
	};
}

/**
 * Whether an echo is worth showing under the line it was read from: it is,
 * only where some word of it is not already up there. Compared as bare
 * letters and digits, because the echo drops the punctuation and the articles
 * the line keeps — `2 gousses ail` is entirely inside `2 gousses d’ail` and
 * says nothing new, while a corrected `250 g farine de blé` under `200 g de
 * farine` says two things.
 */
export function saysMoreThan(echo: string, line: string): boolean {
	const bare = (text: string) => text.toLowerCase().replace(/[^\p{L}\p{N} ]/gu, '');
	const written = bare(line);
	return bare(echo)
		.split(/\s+/)
		.filter(Boolean)
		.some((word) => !written.includes(word));
}
