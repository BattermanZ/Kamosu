/**
 * A Food that was just corrected, said by its page to the list beside it
 * (#199).
 *
 * On the wide layout the Foods list stays while one Food after another is
 * corrected, so the list has to hear of each correction: left alone, a row
 * would go on showing the name it was read with, and the search would go on
 * answering to it. Every act on a Food's page answers with the whole Food, and
 * that answer is what is handed over, so the row is the Core's own words.
 */

import { getContext, setContext } from 'svelte';
import type { Food } from '$lib/foods';

export class Corrected {
	/** The last Food a page corrected, as the Core answered it. */
	latest = $state.raw<Food | undefined>(undefined);
}

const KEY = Symbol('corrected Food');

/** Made by the frame both screens are drawn in. */
export function provideCorrected(): Corrected {
	const corrected = new Corrected();
	setContext(KEY, corrected);
	return corrected;
}

/** Outside that frame there is no list to tell, and nothing is said. */
export function useCorrected(): Corrected | undefined {
	return getContext<Corrected | undefined>(KEY);
}
