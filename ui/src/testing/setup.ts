/**
 * What every test runs inside.
 *
 * Screen tests are the only thing standing between a bad-day component and the
 * kitchen (ADR 0012), so the environment they run in is part of the guarantee:
 * a real DOM, the browser build of Svelte, and nothing stubbed that a screen
 * could rely on by accident.
 */

import '@testing-library/jest-dom/vitest';
import { afterAll, afterEach, beforeAll, vi } from 'vitest';
import { cleanup } from '@testing-library/svelte';
import { went } from './navigation';
import { forgetArrival } from '$lib/arrival.svelte';
import { mistake } from '$lib/mistake.svelte';
import { reach, refreshed } from '$lib/offline/device.svelte';
import { sessions } from '$lib/offline/library.svelte';
import { standing } from '$lib/offline/standing.svelte';

/**
 * One `goto` for the whole run, cleared after every test. Why it is not mocked
 * per file is in `./navigation`.
 */
vi.mock('$app/navigation', async () => ({ goto: (await import('./navigation')).went }));

/**
 * Every file starts where isolation would have started it (#122).
 *
 * Test files share one environment per worker (isolation is off in the config),
 * which saves building a fresh jsdom for each of them. What that shares with it
 * is anything a file can change and not put back: a module's `$state` made once
 * on first import, the browser's storage, and a property a test defined on
 * `navigator`.
 * This file is run again before each test file, so the reset below happens at
 * the top of every file, before the file's own code, and after the last
 * `afterAll` of the one before it.
 *
 * A new module-level `$state`, or a `SvelteMap` or `SvelteSet` made at module
 * scope, belongs in this list. A leak it leaves out does not fail where it is
 * made: it fails some later file, and a different one each run.
 */
function startAsIsolationWould(): void {
	emptied(standing);
	emptied(mistake);
	Object.assign(reach, { server: true, lost: 0 });
	Object.assign(sessions, { began: 0 });
	refreshed.clear();
	forgetArrival();
	// Cleared after every test below as well. This one catches a `goto` made in
	// the last file's `afterAll`, which no `afterEach` follows.
	went.mockClear();

	localStorage.clear();
	sessionStorage.clear();

	// jsdom keeps everything on the prototypes, so an own property on either of
	// these was put there by a test — `onLine`, `serviceWorker`, `wakeLock`,
	// `visibilityState`. `document.location` is jsdom's own and stays.
	for (const name of Object.getOwnPropertyNames(navigator)) takenOff(navigator, name);
	for (const name of Object.getOwnPropertyNames(document)) {
		if (name !== 'location') takenOff(document, name);
	}

	vi.useRealTimers();
	vi.unstubAllGlobals();
}

/** As a fresh import made it: `{}`, not keys left holding `undefined`. */
function emptied(state: object): void {
	for (const key of Object.keys(state)) Reflect.deleteProperty(state, key);
}

/**
 * A property defined without `configurable: true` cannot be deleted, and
 * `delete` says so only by returning false. Left quiet, it would leak into
 * every later file; said here, it fails the file that inherited it.
 */
function takenOff(from: object, name: string): void {
	if (!Reflect.deleteProperty(from, name)) {
		throw new Error(`a test left \`${name}\` on ${from} and it cannot be taken off`);
	}
}

startAsIsolationWould();

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

const rejected = (what: unknown) => strays.push({ how: 'rejected', what });
const threw = (what: unknown) => strays.push({ how: 'was thrown', what });

beforeAll(() => {
	// Added to vitest's own listeners rather than replacing them, so its report
	// still appears. Node runs every listener it has.
	process.on('unhandledRejection', rejected);
	process.on('uncaughtException', threw);
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
	went.mockClear();
	await letThePendingSettle();
	blame('during this test');
});

afterAll(async () => {
	await letThePendingSettle();
	// Taken off again, because this file runs once per test file in the same
	// process: left on, each file would add two more, and Node warns past ten.
	process.off('unhandledRejection', rejected);
	process.off('uncaughtException', threw);
	blame('in this file, after its last test');
});
