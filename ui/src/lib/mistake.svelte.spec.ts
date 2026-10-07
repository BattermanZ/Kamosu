/**
 * When Kamosu itself goes wrong (#98, option C, chosen 22 September 2026).
 *
 * What these guard: a mistake raised from behind the screen seam reaches the
 * cook; a refusal the Core meant to send still reaches nothing, because it is
 * not a mistake and the card must not tell that small lie; a mistake made while
 * rendering takes the screen down and leaves the card standing; the card is put
 * away and the next mistake says so again; and the net under both is installed
 * and taken down again.
 *
 * What they cannot guard is that net catching a *rejected* promise, because
 * **jsdom never fires `unhandledrejection`** — probed directly, and the reason
 * `$lib/mistake.svelte` is verified in a real browser as well as here.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/svelte';
import { standIn } from '$lib/api/stand-in';
import { mistake, putMistakeAway, watchForMistakes } from '$lib/mistake.svelte';
import WentWrong from '$lib/WentWrong.svelte';
import MistakeTestHarness from '$lib/MistakeTestHarness.svelte';

/** The card, by the words a cook reads. */
const card = () => screen.queryByRole('heading', { name: 'Kamosu went wrong' });

/**
 * A script threw, as the browser reports it.
 *
 * Cancelled by a listener registered last — after the net's, so the net still
 * sees it first — because jsdom otherwise reports a dispatched `error` event as
 * an uncaught exception of its own. What is under test is whether the net saw
 * it, not what the environment does next.
 */
function threw(error?: unknown): void {
	addEventListener('error', (event) => event.preventDefault(), { once: true });
	dispatchEvent(new ErrorEvent('error', { error, cancelable: true }));
}

beforeEach(() => {
	putMistakeAway();
	// A mistake is written where a developer looks, which here is the test's own
	// output. Kept quiet so a passing run reads as one.
	vi.spyOn(console, 'error').mockImplementation(() => {});
});

afterEach(() => {
	putMistakeAway();
	vi.restoreAllMocks();
});

describe('a mistake in Kamosu', () => {
	it('raises a card saying so when one escapes an Operation call', async () => {
		const kamosu = standIn({});
		render(WentWrong);
		expect(card()).toBeNull();

		// A real Error, not a refusal: the stand-in was asked for something this
		// test did not answer. Exactly what #87 hit.
		await expect(kamosu.client.listJobs()).rejects.toThrow(/did not answer/);

		await waitFor(() => expect(card()).toBeInTheDocument());
		expect(screen.getByText(/a mistake in Kamosu, not you/)).toBeInTheDocument();
	});

	it('is put away by the cook, and the next one says so again', async () => {
		const kamosu = standIn({});
		render(WentWrong);

		await expect(kamosu.client.listJobs()).rejects.toThrow();
		await waitFor(() => expect(card()).toBeInTheDocument());

		putMistakeAway();
		await waitFor(() => expect(card()).toBeNull());

		// A second, different mistake. The card is not spent.
		await expect(kamosu.client.listKitchens()).rejects.toThrow();
		await waitFor(() => expect(card()).toBeInTheDocument());
	});

	it('is said once, however many layers see the same one', async () => {
		const kamosu = standIn({});
		const thrown = await kamosu.client.listJobs().then(
			() => undefined,
			(error: unknown) => error,
		);
		expect(mistake.made).toBe(thrown);

		// The cook has read it. The net seeing the very same error afterwards —
		// which is what happens in a browser, where the screen re-throws it — must
		// not put the card back up.
		putMistakeAway();
		const stop = watchForMistakes();
		threw(thrown);
		stop();

		expect(mistake.made).toBeUndefined();
	});
});

describe('a refusal the Core meant to send', () => {
	it('is no mistake and raises no card', async () => {
		const kamosu = standIn({ list_jobs: { refuse: 'not_found' } });
		render(WentWrong);

		await expect(kamosu.client.listJobs()).rejects.toThrow();

		// Given a tick to be wrong in.
		await Promise.resolve();
		expect(card()).toBeNull();
		expect(mistake.made).toBeUndefined();
	});

	it('is no mistake even where Kamosu was never reached', async () => {
		const kamosu = standIn({ list_jobs: { refuse: 'internal' } });
		await expect(kamosu.client.listJobs()).rejects.toThrow();
		expect(mistake.made).toBeUndefined();
	});
});

describe('a mistake made while rendering', () => {
	it('is caught by the boundary, and the card outside it survives to say so', async () => {
		render(MistakeTestHarness, { props: { broken: true } });

		await waitFor(() => expect(card()).toBeInTheDocument());
		// The boundary drops what it was walling off, so the half-drawn screen
		// goes rather than sitting there looking like it is still loading.
		expect(screen.queryByText('the screen, drawn')).toBeNull();
	});

	it('leaves a screen that renders perfectly well alone', async () => {
		render(MistakeTestHarness, { props: { broken: false } });

		expect(await screen.findByText('the screen, drawn')).toBeInTheDocument();
		expect(card()).toBeNull();
	});
});

describe('the net under the seam', () => {
	it('catches a script that threw with nobody catching it', async () => {
		render(WentWrong);
		const stop = watchForMistakes();

		threw(new TypeError('undefined is not a Step'));

		await waitFor(() => expect(card()).toBeInTheDocument());
		stop();
	});

	it('lets go when it is taken down', () => {
		const stop = watchForMistakes();
		stop();

		threw(new TypeError('after the teardown'));

		expect(mistake.made).toBeUndefined();
	});

	it('ignores an event carrying no error, which is a picture that failed to load', () => {
		const stop = watchForMistakes();

		threw(undefined);
		stop();

		expect(mistake.made).toBeUndefined();
	});
});
