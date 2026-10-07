/**
 * Print a Sheet (#75, ADR 0023): the recipe as it stands on this screen, set
 * for paper by the server as a Job. The PDF opens in a tab of its own and
 * printing is the browser's own Print — Kamosu has no dialog of its own, and
 * no second place where scaling could disagree with this screen.
 *
 * The tab is opened at the tap and filled once the Job ends: a tab opened
 * later, from a promise, is one a browser is entitled to block.
 *
 * A Sheet that outlasts the ordinary wait has not failed (#117). The screen
 * says it is still being set and goes on waiting while the cook stays on this
 * recipe, so the tab it promised is the tab the Sheet arrives in. Leaving —
 * for another screen, or for another recipe on this one — stops the reading: a
 * Sheet not ready by then is not waited for, and its empty tab closes. The Job
 * itself carries on regardless (ADR 0032). One that was ready as the cook left
 * still fills its tab, but never moves the page they have gone to.
 *
 * In the app installed on an iPhone or iPad no tab opens (#149): the Sheet is
 * fetched here once the Job ends, and this same button then shares it
 * (`share`; option A, chosen 25 September 2026).
 *
 * **It lives beside the recipe screen rather than inside the button it
 * draws**, because the button is not always on the page: writing on the
 * recipe swaps the whole reading page out (#83), and a Sheet being set while
 * the cook taps Edit is still the Sheet they asked for. So the recipe screen
 * makes one of these as it opens, tells it when the recipe changes
 * (`leave`), and it stops waiting on its own when the screen closes.
 */
import type { KamosuClient, MakeSheetOutput } from '$lib/api/catalogue';
import { StillRunning, waitForJob } from '$lib/api/job';
import { SHEET, shareFile, sharesFiles, type FileFetcher } from '$lib/api/files';
import type { Device } from '$lib/offline/device.svelte';
import type { Wanted } from '$lib/how-much';

export type Printing = 'idle' | 'setting' | 'stillSetting' | 'ready' | 'failed';

export interface Sheet {
	/** Where asking for a Sheet has got to (#75). */
	readonly printing: Printing;
	/** Being set, however long it takes: one Sheet at a time, and never a second tab. */
	readonly setting: boolean;
	/** The Sheet being set goes to the share sheet rather than a tab (#149). */
	readonly sharing: boolean;
	/**
	 * Ask for a Sheet of this recipe, at this amount — or as the page stands
	 * unasked where the amount is undefined.
	 */
	print(branchId: string, wanted: Wanted | undefined): Promise<void>;
	/** Hand the prepared Sheet to the share sheet. */
	share(): void;
	/**
	 * The page moved to another recipe. A Sheet being set was the last
	 * recipe's, and `print` stops waiting on it once it sees that (#117); a
	 * Sheet ready to share was that recipe's too (#149).
	 */
	leave(): void;
}

export function sheet(
	kamosu: KamosuClient,
	files: FileFetcher,
	device: () => Pick<Device, 'installed' | 'apple'>,
): Sheet {
	let printing = $state<Printing>('idle');
	let sharing = $state(false);
	/**
	 * The Sheet fetched and waiting for the tap that shares it (#149), in the
	 * installed app on an Apple device. Set only while `printing` is `ready`.
	 */
	let prepared: File | undefined;
	/**
	 * Counts the recipes this screen has shown, so work begun on one can tell
	 * it has been left — even for the same recipe opened again (#117).
	 */
	let visit = 0;
	/** Set once the screen closes, so a Sheet still being waited on stops being read. */
	let closed = false;
	$effect(() => () => {
		closed = true;
	});

	return {
		get printing() {
			return printing;
		},
		get setting() {
			return printing === 'setting' || printing === 'stillSetting';
		},
		get sharing() {
			return sharing;
		},
		async print(branchId, wanted) {
			const forVisit = visit;
			const leftBehind = () => closed || visit !== forVisit;
			const toShare = sharesFiles(device(), SHEET);
			sharing = toShare;
			printing = 'setting';
			const tab = toShare ? null : window.open('', '_blank');
			try {
				const asked = await kamosu.makeSheet(
					wanted === undefined
						? { branch_id: branchId }
						: { branch_id: branchId, wanted_yield: wanted },
				);
				const job = await waitForJob(kamosu, asked.job_id, { stopped: leftBehind }).catch(
					(error: unknown) => {
						if (!(error instanceof StillRunning)) throw error;
						printing = 'stillSetting';
						return waitForJob(kamosu, asked.job_id, {
							giveUpAfter: Infinity,
							stopped: leftBehind,
						});
					},
				);
				if (job.status !== 'completed') {
					// Left before the Sheet was ready: nothing is left to fill.
					tab?.close();
					return;
				}
				const at = (job.result as MakeSheetOutput).fetch_at;
				if (toShare) {
					const file = await files(at, SHEET);
					// Left while it was fetched: nothing is kept for a page not shown.
					if (leftBehind()) return;
					prepared = file;
					printing = 'ready';
					return;
				}
				if (tab) tab.location.href = at;
				else if (!leftBehind()) window.location.assign(at);
				if (!leftBehind()) printing = 'idle';
			} catch {
				tab?.close();
				if (!leftBehind()) printing = 'failed';
			}
		},
		/**
		 * Share the prepared Sheet: Save to Files, Print, AirDrop. Called
		 * straight from the tap with nothing awaited first, since iOS only
		 * opens the share sheet for a tap it can still see (#149). Closing the
		 * share sheet is not a failure, and keeps the Sheet for another try;
		 * once it has gone somewhere, the next tap sets a fresh one.
		 */
		share() {
			const file = prepared;
			if (!file) return;
			const forVisit = visit;
			const moved = () => closed || visit !== forVisit || prepared !== file;
			void shareFile(file).then((outcome) => {
				if (outcome === 'kept' || moved()) return;
				prepared = undefined;
				printing = outcome === 'shared' ? 'idle' : 'failed';
			});
		},
		leave() {
			visit += 1;
			printing = 'idle';
			prepared = undefined;
		},
	};
}
