/**
 * Searching the shelf as you type (#121).
 *
 * Three screens have a search box over `search_recipes`: the shelf, the
 * Component picker and the Related Recipes sheet. Each needs the same three
 * things, and this is the one place they are written:
 *
 * 1. wait a moment after the last keystroke, so typing *curry* asks once
 *    rather than five times;
 * 2. ask `search_recipes`, the one Operation — Meaning Search arrived inside
 *    it rather than beside it (ADR 0029), so every box finds what the shelf
 *    finds for the same words;
 * 3. drop an answer if something newer has been asked since, so a slow reply
 *    cannot land on top of a fresher one.
 *
 * What each screen searches for stays its own: the shelf has filters and the
 * pickers have none (#62). A screen says that in `ask`, and nothing else.
 *
 * `ask` IS CALLED INSIDE THE EFFECT, BEFORE THE DELAY, and that is the whole
 * of what makes this work. Svelte re-asks only when a value it saw read
 * changes; read the search box inside the timer instead and the box goes on
 * working on first open, then silently stops following the thumb. The screen
 * tests type into the real boxes for exactly that reason.
 */

import { OperationError } from '$lib/api/client';
import { refreshed } from '$lib/offline/device.svelte';
import type { KamosuClient, SearchRecipesInput, SearchRecipesOutput } from '$lib/api/catalogue';

/**
 * Long enough that typing a recipe's name is one ask rather than twelve, short
 * enough that the list keeps up with the thumb.
 */
const SETTLE_MS = 180;

export class RecipeSearch {
	/** The last answer, for whatever was asked most recently. `undefined` until one arrives. */
	answer = $state<SearchRecipesOutput | undefined>(undefined);
	/** Whether the most recent ask was refused. */
	failed = $state(false);

	/**
	 * Made while a component is being set up, like any `$effect`; the search
	 * stops with the component.
	 *
	 * `invalidatedBy` is for something besides the input that changes what the
	 * same search finds. The shelf passes Meaning Search's generation: turning
	 * it on asks the search again, and an answer that left before the change is
	 * dropped rather than shown.
	 */
	constructor(
		kamosu: KamosuClient,
		ask: () => SearchRecipesInput,
		invalidatedBy: () => unknown = () => undefined,
	) {
		$effect(() => {
			const input = ask();
			const asked = invalidatedBy();
			// The phone answered first and the server has since answered
			// otherwise (#76). That matters most in the pickers: a stale entry on
			// the shelf is read, and a stale entry in a picker is written down.
			void refreshed.get('search_recipes');

			let current = true;
			const timer = setTimeout(() => {
				void (async () => {
					try {
						const found = await kamosu.searchRecipes(input);
						if (current && asked === invalidatedBy()) {
							this.answer = found;
							this.failed = false;
						}
					} catch (error) {
						if (!(error instanceof OperationError)) throw error;
						if (current) this.failed = true;
					}
				})();
			}, SETTLE_MS);

			return () => {
				current = false;
				clearTimeout(timer);
			};
		});
	}
}
