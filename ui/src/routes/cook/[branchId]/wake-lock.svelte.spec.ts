/**
 * The Wake Lock, on its own. The screen test cannot reach this: jsdom has no
 * Wake Lock API at all, which is one of the two cases that matter and not the
 * interesting one.
 *
 * The interesting one is #61's actual words — "held only during a cook and
 * **re-acquired after an interruption**". A browser releases a screen lock by
 * itself whenever the page is hidden and hands nothing back on return, so a
 * cook whose phone locks in a pocket is exactly the case a naive
 * request-it-once implementation gets wrong, silently, for the rest of the
 * cooking.
 */

import { describe, expect, it, vi, afterEach } from 'vitest';
import { wakeLock } from './wake-lock.svelte';

/** A stand-in for the browser's own screen lock, which releases on hide. */
function fakeApi() {
	const granted: { released: boolean; release: () => Promise<void> }[] = [];
	let refuse = false;
	const wakeLockApi = {
		request: async () => {
			if (refuse) throw new Error('refused');
			const sentinel = {
				released: false,
				release: async () => {
					sentinel.released = true;
				},
			};
			granted.push(sentinel);
			return sentinel;
		},
	};
	Object.defineProperty(navigator, 'wakeLock', { value: wakeLockApi, configurable: true });
	return {
		granted,
		refuseNext() {
			refuse = true;
		},
		allow() {
			refuse = false;
		},
		/** What a browser does when the page is hidden: release, and say nothing. */
		async hide() {
			for (const sentinel of granted) sentinel.released = true;
			Object.defineProperty(document, 'visibilityState', {
				value: 'hidden',
				configurable: true,
			});
			document.dispatchEvent(new Event('visibilitychange'));
			await vi.waitFor(() => {});
		},
		async show() {
			Object.defineProperty(document, 'visibilityState', {
				value: 'visible',
				configurable: true,
			});
			document.dispatchEvent(new Event('visibilitychange'));
			await vi.waitFor(() => {});
		},
	};
}

afterEach(() => {
	Reflect.deleteProperty(navigator, 'wakeLock');
	Reflect.deleteProperty(document, 'visibilityState');
});

describe('the wake lock', () => {
	it('takes a real lock while cooking and gives it back on the way out', async () => {
		const api = fakeApi();
		const lock = wakeLock();
		expect(lock.available).toBe(true);
		lock.start();
		await vi.waitFor(() => expect(lock.held).toBe(true));
		expect(api.granted).toHaveLength(1);

		lock.stop();
		await vi.waitFor(() => expect(api.granted[0].released).toBe(true));
		expect(lock.held).toBe(false);
	});

	it('re-acquires it after an interruption', async () => {
		const api = fakeApi();
		const lock = wakeLock();
		lock.start();
		await vi.waitFor(() => expect(lock.held).toBe(true));

		// The phone locks in a pocket. The browser releases the lock itself.
		await api.hide();
		expect(lock.held).toBe(false);

		// Back at the stove. A second, live lock — not the first one still being
		// held onto, which is the bug this test exists for.
		await api.show();
		await vi.waitFor(() => expect(lock.held).toBe(true));
		expect(api.granted).toHaveLength(2);
		expect(api.granted[1].released).toBe(false);
		lock.stop();
	});

	it('stays off across an interruption once the cook has turned it off', async () => {
		const api = fakeApi();
		const lock = wakeLock();
		lock.start();
		await vi.waitFor(() => expect(lock.held).toBe(true));

		lock.toggle();
		await vi.waitFor(() => expect(lock.wanted).toBe(false));
		await api.hide();
		await api.show();
		// The switch is the cook's, and an interruption is not a reason to
		// overrule them.
		expect(lock.wanted).toBe(false);
		expect(lock.held).toBe(false);
		expect(api.granted).toHaveLength(1);
		lock.stop();
	});

	it('keeps the switch answering to the cook when the browser refuses', async () => {
		const api = fakeApi();
		api.refuseNext();
		const lock = wakeLock();
		lock.start();
		await vi.waitFor(() => expect(lock.held).toBe(false));
		// Wanted, and not held: the screen says the screen may sleep, which is
		// true. What must NOT happen is the switch reading as off — a cook
		// tapping it to turn it on would then turn their own intent off.
		expect(lock.wanted).toBe(true);

		api.allow();
		lock.toggle();
		expect(lock.wanted).toBe(false);
		lock.stop();
	});

	it('says it is unavailable rather than faking it', async () => {
		const lock = wakeLock();
		expect(lock.available).toBe(false);
		lock.start();
		await vi.waitFor(() => expect(lock.held).toBe(false));
		lock.stop();
	});
});
