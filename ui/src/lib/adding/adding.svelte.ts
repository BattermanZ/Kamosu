/**
 * The three ways a new recipe starts, written once (#174): a web page, a
 * Bundle somebody sent you (a Kamosu zip file, ADR 0020), and a title you type yourself.
 *
 * Two places offer them. The + beside the search box offers all three every
 * day, and the dead ends `AddOrImport` draws (a search that matched nothing,
 * an empty Home) offer them as full-width buttons. Each place holds its own
 * `Adding`, so one screen's work never quiets the other's buttons, but the acts
 * themselves live here and nowhere else. One act written twice is one act that
 * will eventually be two, which is how #93 moved the first two out of
 * `AddOrImport` and why #174 moved the third out after them.
 *
 * **Every act *does* the thing and ends on the recipe** (ADR 0027). A link and
 * a file are one recipe each and open it; a title *is* a recipe (#6), so
 * writing one opens it too. Nobody is asked where it goes: a recipe written or
 * brought in lands in its writer's own Cookbook (ADR 0041).
 */

import { goto } from '$app/navigation';
import { m } from '$lib/paraglide/messages';
import { useKamosu } from '$lib/kamosu';
import { useUpload, type Uploader } from '$lib/api/upload';
import { OperationError } from '$lib/api/client';
import { StillRunning, waitForJob } from '$lib/api/job';
import { receiveStaged } from '$lib/receive';
import type { ImportWebLinkOutput, KamosuClient } from '$lib/api/catalogue';

export type Act = 'link' | 'file' | 'write';

export class Adding {
	/** The act running now, or none. Every button offering an act quiets while one runs. */
	working = $state<Act | null>(null);
	/** Why the last act landed nothing, in the words it failed with. */
	failed = $state<string | undefined>(undefined);
	/**
	 * The wait ran out before the Job did (#117). Not a failure: the recipe
	 * still arrives, and the screen says so rather than what went wrong.
	 */
	stillGoing = $state(false);

	#kamosu: KamosuClient;
	#upload: Uploader;

	constructor(kamosu: KamosuClient, upload: Uploader) {
		this.#kamosu = kamosu;
		this.#upload = upload;
	}

	/** Reading a web page is slow, so importing is a Job (ADR 0032). */
	link = (url: string) =>
		this.#run('link', async () => {
			const asked = await this.#kamosu.importWebLink({ url: url.trim() });
			const finished = await waitForJob(this.#kamosu, asked.job_id);
			const result = finished.result as ImportWebLinkOutput;
			// A Share Link arrives as the Bundle it serves (#169), so its Report
			// names the shared recipe as its subject beside the Passengers that
			// travelled with it. A web page's single row names no subject.
			const landed =
				result.arrived.find((row) => row.subject) ?? result.arrived[0] ?? result.offered[0];
			if (landed) {
				await goto(`/recipes/${landed.branch_id}`);
				return undefined;
			}
			// The page was reached and held no recipe this instance could read.
			// Saying so is the whole answer; there is nothing to open.
			return result.unreadable[0]?.reason ?? m.recipes_import_unreadable();
		});

	/**
	 * A Bundle, sent as its own bytes and then named to the Operation.
	 *
	 * One recipe carrying photographs is megabytes, and base64 inside a JSON
	 * body would be a third larger again and would sit whole in the Job's
	 * stored input, so it travels to `POST /api/uploads` and `import_bundle` is
	 * asked with the id that answers (#93, ADR 0001).
	 */
	file = (file: File) =>
		this.#run('file', async () => receiveStaged(this.#kamosu, await this.#upload(file)));

	/**
	 * A recipe needs only a title (#6), so the title *is* the recipe. A title of
	 * nothing but spaces is not one, and asks nothing.
	 */
	write = (title: string) =>
		this.#run('write', async () => {
			if (title.trim() === '') return undefined;
			const made = await this.#kamosu.createRecipe({ title: title.trim() });
			await goto(`/recipes/${made.branch_id}`);
			return undefined;
		});

	/**
	 * Every act starts and ends through here. An act answers the sentence to
	 * say where it was asked when nothing landed, or nothing once it has gone
	 * to the recipe.
	 *
	 * A Job that failed arrives as a plain Error from `waitForJob`, and a
	 * refused Operation as an OperationError: both are the act's to say, because
	 * a person who has just chosen a file or pasted a link is owed a sentence,
	 * never a silence (#93). An ended Share Link is the first kind: its Job
	 * fails in the share page's own words, which the reader is owed (#169).
	 */
	async #run(act: Act, perform: () => Promise<string | undefined>) {
		if (this.working) return;
		this.working = act;
		this.failed = undefined;
		this.stillGoing = false;
		try {
			this.failed = await perform();
		} catch (error) {
			if (!(error instanceof Error)) throw error;
			// Writing is one Operation and no Job, so only a refusal is its to
			// say; anything else is a Mistake for the layers that catch those.
			// Kept here rather than in `write`, so every act leaves `working`
			// the same way.
			if (act === 'write' && !(error instanceof OperationError)) throw error;
			if (error instanceof StillRunning) this.stillGoing = true;
			else this.failed = error.message;
		} finally {
			this.working = null;
		}
	}
}

/** An `Adding` on this screen's client and uploader. Call it while the component is being made. */
export function useAdding(): Adding {
	return new Adding(useKamosu(), useUpload());
}
