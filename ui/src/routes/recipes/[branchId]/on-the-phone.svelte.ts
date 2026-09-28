/**
 * What the recipe screen knows about the phone it is on (#76): whether the
 * copy it shows is only the one kept on the day it was opened, and whether
 * the server has since said something the phone did not.
 *
 * Made once, as the screen opens, and it keeps itself level with the recipe
 * the screen is on through the getter it is handed — the screen is reused
 * across `/recipes/A` → `/recipes/B` rather than remade.
 */
import { untrack } from 'svelte';
import { Online, refreshed } from '$lib/offline/device.svelte';
import { useLibrary } from '$lib/offline/library.svelte';
import { keptAt } from '$lib/offline/reads';
import { standing } from '$lib/offline/standing.svelte';

export interface OnThePhone {
	/**
	 * When the phone kept the copy being shown, for a recipe the Person's
	 * Kitchens do not hold. Their own recipes are the library and always on
	 * the phone; anything else is only the copy from the day it was opened,
	 * and offline the page says so.
	 */
	readonly keptOn: Date | undefined;
	/** Offline, and showing that kept copy. */
	readonly onlyKept: boolean;
}

/**
 * `reread` is called when the page should read the recipe again: the phone
 * answered first and the server has since said something else.
 */
export function onThePhone(branchId: () => string, reread: () => void): OnThePhone {
	const online = new Online();
	const library = useLibrary();
	const keptOn = $derived(standing.branchId === branchId() ? standing.keptAt : undefined);
	const onlyKept = $derived(
		!online.current && library.onlyOpened(branchId()) && keptOn !== undefined,
	);

	$effect(() => {
		const id = branchId();
		standing.branchId = id;
		void keptAt('get_recipe', { branch_id: id }).then((at) => {
			if (standing.branchId === id) standing.keptAt = at;
		});
		return () => {
			if (standing.branchId === id) {
				standing.branchId = undefined;
				standing.keptAt = undefined;
			}
		};
	});

	// The phone answered first and the server has since said something else:
	// read again, from the copy that is now level.
	let seenRefreshes: number | undefined;
	$effect(() => {
		const seen =
			(refreshed.get('get_recipe') ?? 0) +
			(refreshed.get('get_thread') ?? 0) +
			(refreshed.get('divergence') ?? 0);
		if (seenRefreshes !== undefined && seen > seenRefreshes) untrack(reread);
		seenRefreshes = seen;
	});

	return {
		get keptOn() {
			return keptOn;
		},
		get onlyKept() {
			return onlyKept;
		},
	};
}
