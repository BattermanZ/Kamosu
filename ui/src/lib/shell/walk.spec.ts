/**
 * Opening what an arrow led to (#199, #200).
 *
 * The one thing tested here is the thing no screen test can see. An arrow
 * pressed before the last one finished opening ends that navigation and
 * SvelteKit rejects its promise, which in a browser reached the net under the
 * app and drew the *Kamosu went wrong* card for a key held down. jsdom never
 * fires `unhandledrejection`, so the promise itself is what is asked.
 */

import { describe, expect, it } from 'vitest';
import { went } from '../../testing/navigation';
import { openInPlace } from './walk';

describe('opening in place', () => {
	it('replaces the entry, and keeps the caret and the scroll', async () => {
		await openInPlace('/imports/j_1');

		expect(went).toHaveBeenLastCalledWith('/imports/j_1', {
			replaceState: true,
			keepFocus: true,
			noScroll: true,
		});
	});

	it('is not a Mistake when the next arrow overtakes it', async () => {
		went.mockRejectedValueOnce(new Error('navigation aborted'));

		await expect(openInPlace('/imports/j_1')).resolves.toBeUndefined();
	});
});
