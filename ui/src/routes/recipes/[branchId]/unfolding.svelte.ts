/**
 * The Components of the recipe you are standing in, and which of them are
 * unfolded (#50, ADR 0008).
 *
 * The Core unfolds them — flat, depth first, each carrying the `path` of line
 * indexes that reaches it — so both sides of a Divergence carry their own,
 * and a dough unfolds whichever recipe you are standing in. What is held here
 * is only which of them the cook has opened.
 */
import type { GetRecipeOutput } from '$lib/api/catalogue';

/** One Component of the recipe being read, unfolded by the Core (ADR 0008). */
export type Component = GetRecipeOutput['versions'][number]['components'][number];

/** A Component's path as one string: what the annexe's anchor and the open set are keyed by. */
export const pathKey = (path: number[]) => path.join('.');

export interface Unfolding {
	/** One Component, or nothing: the entry sitting at `index` of the list at `parent`. */
	at(parent: number[], index: number): Component | undefined;
	isOpen(path: number[]): boolean;
	toggle(path: number[]): void;
	/** Fold everything away. */
	closeAll(): void;
	/**
	 * **Every open Component's Steps, in the order the page meets them** — the
	 * foot of the page under treatment B. Folding a Component away takes its
	 * method with it, so the foot of the page holds exactly what the list above
	 * says is open. A Component with no Steps, one this instance does not hold,
	 * and one that stopped at a repeat all contribute nothing.
	 */
	readonly annexes: Component[];
}

export function unfolding(components: () => Component[]): Unfolding {
	/**
	 * Which Components are open. **Closed by default** (ADR 0008): the row
	 * says which recipe it names and how much of it, and the recipe itself is
	 * a tap away. Keyed by path, so a Component inside a Component opens on
	 * its own.
	 *
	 * Putting the marks away or bringing them back closes everything, for the
	 * reason the marks' open rows and the corrector are cleared there: the
	 * list is rebuilt from a different set of rows, so a path that meant the
	 * dough a moment ago means another line now.
	 */
	let unfolded = $state(new Set<string>());
	const isOpen = (path: number[]) => unfolded.has(pathKey(path));
	const annexes = $derived(
		components().filter((component) => component.content?.steps.length && isOpen(component.path)),
	);

	return {
		at(parent, index) {
			return components().find(
				(component) =>
					component.path.length === parent.length + 1 &&
					parent.every((step, depth) => component.path[depth] === step) &&
					component.path[parent.length] === index,
			);
		},
		isOpen,
		toggle(path) {
			const next = new Set(unfolded);
			const key = pathKey(path);
			if (next.has(key)) next.delete(key);
			else next.add(key);
			unfolded = next;
		},
		closeAll() {
			unfolded = new Set();
		},
		get annexes() {
			return annexes;
		},
	};
}
