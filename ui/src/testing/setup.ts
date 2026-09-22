/**
 * What every test runs inside.
 *
 * Screen tests are the only thing standing between a bad-day component and the
 * kitchen (ADR 0012), so the environment they run in is part of the guarantee:
 * a real DOM, the browser build of Svelte, and nothing stubbed that a screen
 * could rely on by accident.
 */

import '@testing-library/jest-dom/vitest';
import { afterAll, afterEach, beforeAll } from 'vitest';
import { cleanup } from '@testing-library/svelte';

/**
 * Anything thrown or rejected with nobody catching it (#98).
 *
 * Vitest notices these and exits non-zero for them — but it prints
 * `Tests 371 passed` with the real failure in a footnote underneath, and a
 * footnote under a green suite is what a human reads past. That is how #87
 * shipped past one. So a stray is turned into a failing test with a name.
 *
 * `process` rather than `window`, because **jsdom never fires
 * `unhandledrejection`** — which is also why the browser-side net in
 * `$lib/mistake.svelte` cannot be tested here at all.
 */
const strays: { how: string; what: unknown }[] = [];

beforeAll(() => {
	// Added to vitest's own listeners rather than replacing them, so its report
	// still appears. Node runs every listener it has.
	process.on('unhandledRejection', (what) => strays.push({ how: 'rejected', what }));
	process.on('uncaughtException', (what) => strays.push({ how: 'was thrown', what }));
});

/**
 * Node reports a rejection only once the microtask queue has drained, which is
 * after the test that caused it has returned. Waiting on a real timer crosses
 * that boundary, so the stray is attributed to the test that made it rather
 * than to the innocent one after it — and a stray left by a file's LAST test is
 * seen at all, which neither vitest nor an `afterEach` alone managed: the run
 * printed nothing and exited 0.
 */
const letThePendingSettle = () => new Promise((resolve) => setTimeout(resolve, 0));

function blame(when: string): void {
	if (strays.length === 0) return;
	const found = strays.splice(0, strays.length);
	const [first] = found;
	const said =
		first.what instanceof Error ? (first.what.stack ?? first.what.message) : String(first.what);
	const also = found.length > 1 ? ` (and ${found.length - 1} more)` : '';
	throw new Error(`something ${first.how} with nobody catching it ${when}${also}: ${said}`);
}

afterEach(async () => {
	cleanup();
	await letThePendingSettle();
	blame('during this test');
});

afterAll(async () => {
	await letThePendingSettle();
	blame('in this file, after its last test');
});
