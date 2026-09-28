/**
 * A Job that outlives the wait on it (#117), without a test sitting through
 * minutes of real time.
 *
 * Only `Date` is faked, so the poll's own `setTimeout` still runs for real;
 * each read of the Job jumps the clock past every caller's patience, so the
 * wait ends on the first look.
 */

import { vi } from 'vitest';

const PAST_EVERY_PATIENCE = 60 * 60 * 1000;

/** Run `test` with the clock under the test's hand, and hand it back after. */
export async function withTheClockFaked(test: () => Promise<void>) {
	vi.useFakeTimers({ toFake: ['Date'] });
	try {
		await test();
	} finally {
		vi.useRealTimers();
	}
}

/**
 * A `get_job` answer: `running` the first time it is read, with the clock
 * moved past the wait, and `then` on every read after — or `running` for
 * ever, when there is no `then`.
 */
export function outlivingTheWait<Running, Then = Running>(running: Running, then?: Then) {
	let reads = 0;
	return (): Running | Then => {
		reads += 1;
		if (reads > 1 && then) return then;
		vi.setSystemTime(Date.now() + PAST_EVERY_PATIENCE);
		return running;
	};
}
