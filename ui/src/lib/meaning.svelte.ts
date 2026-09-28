/**
 * Meaning Search as a screen holds it (#63, ADR 0029).
 *
 * Two screens need the same three things: what state Meaning Search is in,
 * whether this reader may act on it, and the running commentary of turning it
 * on. They are different screens with different jobs — the shelf carries the
 * offer, Settings carries the management — but the asking is identical, and
 * written twice it would be two subtly different askings.
 */

import { OperationError } from '$lib/api/client';
import { m } from '$lib/paraglide/messages';
import { waitForJob } from '$lib/api/job';
import type { KamosuClient, MeaningSearchStatusOutput } from '$lib/api/catalogue';

/**
 * A model download is minutes on a slow line, and reading a library is minutes
 * more. Both are watched through `get_job`, so the wait is bounded generously
 * rather than by the number that suits the shortest Job in Kamosu.
 */
const LONG_ENOUGH = 30 * 60_000;

export class MeaningSearch {
	/** What the instance last said. `undefined` until it has answered once. */
	status = $state<MeaningSearchStatusOutput | undefined>(undefined);
	/** Whether a turn-on is running, and what it is doing right now. */
	working = $state(false);
	saying = $state('');
	/** The last refusal, in the words it arrived in. */
	failed = $state<string | undefined>(undefined);

	#kamosu: KamosuClient;
	/**
	 * Bumped on every answered question, so a slow reply to an older ask cannot
	 * land on top of a newer one — and so a caller can watch this to know the
	 * answer changed.
	 */
	generation = $state(0);

	constructor(kamosu: KamosuClient) {
		this.#kamosu = kamosu;
	}

	/** Ask again. Safe to call from an `$effect`; cleans up after itself. */
	ask(): () => void {
		const asking = this.generation;
		let current = true;
		this.#kamosu
			.meaningSearchStatus({})
			.then((held) => {
				if (current && asking === this.generation) this.status = held;
			})
			.catch((error: unknown) => {
				// Costs only what this screen says about Meaning Search. The
				// shelf and Settings are each for something else.
				if (!(error instanceof OperationError)) throw error;
			});
		return () => {
			current = false;
		};
	}

	/**
	 * Accept, download, index — three Operations in a row, two of them Jobs,
	 * because 220 MB of weights and a whole library's worth of inference are
	 * exactly what a Job is for (ADR 0032).
	 */
	async turnOn(): Promise<void> {
		this.working = true;
		this.failed = undefined;
		try {
			this.saying = m.meaning_accepting();
			await this.#kamosu.acceptMeaningSearchTerms({});

			this.saying = m.meaning_fetching();
			await this.#watch((await this.#kamosu.downloadMeaningModel({})).job_id);

			this.saying = m.meaning_reading();
			await this.#watch((await this.#kamosu.buildMeaningIndex({})).job_id);

			this.generation += 1;
		} catch (error) {
			// A failed Job is raised as an ordinary Error by `waitForJob`, so
			// both kinds are caught and both are shown as what they say.
			if (!(error instanceof Error)) throw error;
			this.failed = error.message;
		}
		this.working = false;
	}

	async decline(): Promise<void> {
		await this.#act(() => this.#kamosu.declineMeaningSearch({}));
	}

	async turnOff(): Promise<void> {
		await this.#act(() => this.#kamosu.turnOffMeaningSearch({}));
	}

	async #act(run: () => Promise<unknown>): Promise<void> {
		this.failed = undefined;
		try {
			await run();
			this.generation += 1;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			this.failed = error.message;
		}
	}

	async #watch(jobId: string): Promise<void> {
		await waitForJob(this.#kamosu, jobId, {
			giveUpAfter: LONG_ENOUGH,
			whileWaiting: (job) => {
				if (job.progress.message) this.saying = job.progress.message;
			},
		});
	}
}
