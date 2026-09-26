/**
 * Receiving a recipe file already staged on the server, and going where it
 * leads: the one ending two screens share (#93, #170).
 *
 * `BringIn` stages a file somebody chose; the page a Share Link's *Import this
 * recipe* opens has the file its preview staged. Both then ask `import_bundle`
 * with the upload's id and end the same way, which is `outcomeOf`'s three
 * answers: open the recipe with its line above it, open the Import Report,
 * or stay and say why nothing landed.
 */

import { goto } from '$app/navigation';
import { m } from '$lib/paraglide/messages';
import { waitForJob } from '$lib/api/job';
import { noteArrival, outcomeOf, type Arrival } from '$lib/arrival.svelte';
import type { ImportBundleOutput, KamosuClient } from '$lib/api/catalogue';

/** What the arrival line should know besides what the Report says. */
export type Told = Pick<Arrival, 'from' | 'writer' | 'added'>;

/**
 * Import the staged file and go to what it brought. Answers the sentence to
 * say where the file was chosen when nothing landed, and nothing once it has
 * gone to the recipe or the Report. A Job that fails, or outlasts the wait
 * (`StillRunning`), is thrown for the screen to say.
 */
export async function receiveStaged(
	kamosu: KamosuClient,
	uploadId: string,
	told: Told = {},
): Promise<string | undefined> {
	const asked = await kamosu.importBundle({ upload_id: uploadId });
	const finished = await waitForJob(kamosu, asked.job_id, { giveUpAfter: 10 * 60 * 1000 });
	const outcome = outcomeOf(finished.result as ImportBundleOutput);
	if (outcome.show === 'reason') {
		// Not a recipe file, or one nothing could be read from. It is a
		// completed Job with an empty Report, not a failure, so what it says is
		// the answer.
		return outcome.reason === '' ? m.bring_in_unreadable() : outcome.reason;
	}
	if (outcome.show === 'report') {
		// More happened than a line can name, so the ledger is the honest
		// answer rather than one of several recipes picked arbitrarily.
		await goto(`/imports/${asked.job_id}`);
		return undefined;
	}
	noteArrival({ ...outcome.landed, jobId: asked.job_id, ...told });
	await goto(`/recipes/${outcome.landed.branchId}`);
	return undefined;
}
