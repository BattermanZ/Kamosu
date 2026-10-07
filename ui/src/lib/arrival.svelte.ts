/**
 * What bringing a recipe file in just said (#93).
 *
 * The choice was that a recipe file does not end on the Import Report the way a
 * Crouton library does: one file is one recipe, so it opens the recipe and says
 * one line above it (option C, 21 September 2026). That line has to survive one
 * `goto`, so it is held here rather than passed through the URL — the note is a
 * *just happened* fact, and a fact that outlives the navigation it belongs to
 * would greet somebody who reloaded the page an hour later.
 *
 * The Report itself is not duplicated here: it is the Job's own result, read
 * back through `get_job` at `/imports/<job_id>` like every other Job's
 * (ADR 0032), and the line links to it for what a line cannot hold.
 */

import type { ImportBundleOutput } from '$lib/api/catalogue';

/** A recipe as it landed, and what landing meant for it. */
export interface Landed {
	/** The Branch this instance now holds it under — never the id it travelled under. */
	branchId: string;
	title: string;
	/**
	 * `arrived`: it is on the shelf, and `status` says whether the file made it,
	 * carried it forward, or told this instance nothing it did not have.
	 * `kept`: its history could not be read, so its words became a recipe of
	 * this Person's own with no history at all (#67) — a different sentence,
	 * because nothing was inherited.
	 */
	how: 'arrived' | 'kept';
	status?: 'created' | 'extended' | 'unchanged';
	/** Why its history could not be read. Only when `how` is `kept`. */
	reason?: string;
	/** What travelled with it because the recipe is made of them (ADR 0008). */
	passengers: string[];
}

/** One arrival, waiting for the screen that shows it. */
export interface Arrival extends Landed {
	/** The Job, so the line can point at the whole Report. */
	jobId: string;
	/**
	 * Where it came from, when not a file somebody chose: a Share Link imported
	 * from its own page (#170). The line then names no file.
	 */
	from?: 'link';
	/** Who writes it, where the line names them: its newer Versions are theirs. */
	writer?: string;
	/** How many newer Versions importing brought, when it brought some. */
	added?: number;
}

/** What the screen should do with a Report, once the Job has ended. */
export type Outcome =
	/** Open this recipe and say the line above it. */
	| { show: 'recipe'; landed: Landed }
	/** Open the Import Report: what happened is more than a line can name. */
	| { show: 'report' }
	/** Open nothing and say this where the file was chosen. */
	| { show: 'reason'; reason: string };

/**
 * What a Bundle's Report means for the screen that asked for it.
 *
 * Three answers, because a recipe file has three endings and only one of them
 * is the happy path a line was written for.
 *
 * **One recipe arrived.** A Bundle names its subject, so that is what is looked
 * for; the rest of `arrived` are Passengers, travelling because the recipe is
 * made of them (ADR 0008). This is the ordinary case and the reason the line
 * exists.
 *
 * **Its history was damaged.** Then it is not in `arrived` at all: #67 keeps
 * each note's words as a recipe of this Person's own, and reports them under
 * `unreadable` with `kept_as`. One kept recipe is still one recipe, so it
 * opens. **Several is not**, and the screen must not guess which one you
 * wanted — the rows arrive sorted by note filename, which is alphabetical by
 * title and says nothing about which was the subject. So several goes to the
 * Report, which is exactly what the Report is for: more happened than a
 * sentence can hold.
 *
 * **Nothing landed.** Not a Bundle, or one nothing could be read from. It is a
 * completed Job with an empty Report rather than a failure, so its own reason
 * is the answer.
 */
export function outcomeOf(report: ImportBundleOutput): Outcome {
	const subject = report.arrived.find((row) => row.subject) ?? report.arrived[0];
	if (subject) {
		return {
			show: 'recipe',
			landed: {
				branchId: subject.branch_id,
				title: subject.title,
				how: 'arrived',
				status: subject.status,
				passengers: report.arrived
					.filter((row) => row.branch_id !== subject.branch_id)
					.map((row) => row.title),
			},
		};
	}

	const kept = report.unreadable.filter((row) => row.kept_as);
	if (kept.length > 1) return { show: 'report' };
	const damaged = kept[0];
	if (damaged?.kept_as) {
		return {
			show: 'recipe',
			landed: {
				branchId: damaged.kept_as.branch_id,
				title: damaged.kept_as.title,
				how: 'kept',
				reason: damaged.reason,
				passengers: [],
			},
		};
	}

	return { show: 'reason', reason: report.unreadable[0]?.reason ?? '' };
}

let held = $state<Arrival | undefined>(undefined);

/** Said by whoever brought the file in, just before it opens the recipe. */
export function noteArrival(arrival: Arrival): void {
	held = arrival;
}

/** Read by the one line that shows it. Nothing waiting is the ordinary case. */
export function theArrival(): Arrival | undefined {
	return held;
}

/** Said once. Leaving the recipe it belongs to is what forgets it. */
export function forgetArrival(): void {
	held = undefined;
}
