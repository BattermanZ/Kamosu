/**
 * An import that just ended, said by its Report to the list beside it (#200).
 *
 * On the wide layout the list of imports stays while a Report is open, and a
 * Report can be open while its import is still running. The list read that
 * import as running, so left alone its row would go on saying *Bringing them
 * in now* beside a Report that says 86 recipes arrived. The Report counts each
 * import it watched end, and the list reads itself again when the count moves.
 */

import { getContext, setContext } from 'svelte';

export class Ended {
	/** How many imports a Report here has watched end. */
	count = $state(0);

	/** Said by a Report whose import ended while it watched. */
	say(): void {
		this.count += 1;
	}
}

const KEY = Symbol('ended import');

/** Made by the frame both screens are drawn in. */
export function provideEnded(): Ended {
	const ended = new Ended();
	setContext(KEY, ended);
	return ended;
}

/** Outside that frame there is no list to tell, and nothing is said. */
export function useEnded(): Ended | undefined {
	return getContext<Ended | undefined>(KEY);
}
