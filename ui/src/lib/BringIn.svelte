<!--
	The two ways a recipe arrives from outside Kamosu: a web page, and a recipe
	file somebody sent you (#93, ADR 0020).

	They live together because they are one act from two sources — *I do not
	have this yet, and here is where it is* — and because Aurélien chose that
	the recipe file belongs beside the link import rather than in Settings
	beside the Crouton library (option C, 21 September 2026). A library is a
	one-off act of setting an instance up; one file is one recipe, which is the
	ordinary business of a shelf.

	**Both acts *do* the thing rather than pointing at a screen to do it on**,
	which is ADR 0027's rule and the reason the link offer reveals a field in
	place instead of navigating anywhere.

	Two looks, because this appears in two quite different places and the drawing
	is all that differs between them:

	- `offer` — full-width buttons, at the dead ends `AddOrImport` draws: an
	  empty Home (#64) and a search that matched nothing (#62).
	- `quiet` — one line above the shelf, where it is there every day and
	  must not shout over the recipes.

	The second look is named `quiet` and not the obvious word for a run of text,
	because Tailwind reads every file under `ui/src/` and that word is also a
	utility: written anywhere here, even inside a comment, it emits a real rule
	into the stylesheet that ships. `ui/src/app.css` tells the same story about
	the word that once leaked in from a config comment.

	Where they end differs by source, and deliberately. A link is one recipe and
	always has been, so it opens it. A recipe file is also one recipe, so it
	opens that too and says one line above it (`$lib/arrival.svelte.ts`) — not
	the Import Report, which was built for 86 recipes at once (#68) and is a
	whole page of ledger for a single one. The Report is still there under the
	Job's id for anything the line cannot hold.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { useUpload } from '$lib/api/upload';
	import { StillRunning, waitForJob } from '$lib/api/job';
	import { noteArrival, outcomeOf } from '$lib/arrival.svelte';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';
	import { Online } from '$lib/offline/device.svelte';
	import type { ImportBundleOutput, ImportWebLinkOutput } from '$lib/api/catalogue';

	interface Props {
		look: 'offer' | 'quiet';
		/** Stop both acts while the caller is busy with something of its own. */
		disabled?: boolean;
		/**
		 * Said whenever either act starts or stops working, so a caller can
		 * quiet buttons of its own while one runs. A one-way notice rather than
		 * bound state: nothing here ever reads it back.
		 */
		onBusy?: (working: boolean) => void;
	}

	let { look, disabled = false, onBusy }: Props = $props();

	const kamosu = useKamosu();
	const upload = useUpload();
	const online = new Online();

	let asking = $state(false);
	let working = $state<'no' | 'link' | 'file'>('no');
	let link = $state('');
	let failed = $state<string | undefined>(undefined);
	/**
	 * The wait ran out before the Job did (#117). Not a failure: the recipe
	 * still arrives, and this says so rather than what went wrong.
	 */
	let stillGoing = $state(false);
	/**
	 * The file field, clicked by the button beside it. A file cannot be chosen
	 * from a `<button>`, and the button has to be `NeedsServer`'s so that
	 * offline it greys and says what it is waiting for (#76) rather than failing
	 * when pressed — so the field is here, hidden, and the button reaches it.
	 */
	let picker = $state<HTMLInputElement | undefined>(undefined);

	/**
	 * Both acts start and end through here, so `onBusy` is said once in one
	 * place. Not an `$effect` watching `working`: an effect for keeping one
	 * value in step with another is the thing Svelte's own guidance rules out,
	 * and there is nothing asynchronous about saying it here.
	 */
	function nowWorking(what: 'no' | 'link' | 'file') {
		working = what;
		onBusy?.(what !== 'no');
	}

	/** Reading a web page is slow, so importing is a Job (ADR 0032). */
	async function fromLink() {
		nowWorking('link');
		failed = undefined;
		stillGoing = false;
		try {
			const asked = await kamosu.importWebLink({ url: link.trim() });
			const finished = await waitForJob(kamosu, asked.job_id);
			const result = finished.result as ImportWebLinkOutput;
			const landed = result.arrived[0] ?? result.offered[0];
			if (landed) {
				await goto(`/recipes/${landed.branch_id}`);
				return;
			}
			// The page was reached and held no recipe this instance could read.
			// Saying so is the whole answer; there is nothing to open.
			failed = result.unreadable[0]?.reason ?? m.recipes_import_unreadable();
		} catch (error) {
			if (error instanceof StillRunning) stillGoing = true;
			else if (error instanceof OperationError) failed = error.message;
			else throw error;
		} finally {
			nowWorking('no');
		}
	}

	/**
	 * A recipe file, sent as its own bytes and then named to the Operation.
	 *
	 * One recipe carrying photographs is megabytes, and base64 inside a JSON
	 * body would be a third larger again and would sit whole in the Job's stored
	 * input — so it travels to `POST /api/uploads` and `import_bundle` is asked
	 * with the id that answers (#93, ADR 0001).
	 */
	async function fromFile(event: Event & { currentTarget: HTMLInputElement }) {
		const file = event.currentTarget.files?.[0];
		// Cleared at once, so choosing the same file twice is two choices.
		event.currentTarget.value = '';
		if (!file) return;
		nowWorking('file');
		failed = undefined;
		stillGoing = false;
		try {
			const uploadId = await upload(file);
			const asked = await kamosu.importBundle({ upload_id: uploadId });
			const finished = await waitForJob(kamosu, asked.job_id, { giveUpAfter: 10 * 60 * 1000 });
			const outcome = outcomeOf(finished.result as ImportBundleOutput);
			if (outcome.show === 'reason') {
				// Not a recipe file, or one nothing could be read from. It is a
				// completed Job with an empty Report, not a failure, so what it
				// says is the answer — said here, where the file was chosen.
				failed = outcome.reason === '' ? m.bring_in_unreadable() : outcome.reason;
				return;
			}
			if (outcome.show === 'report') {
				// More happened than a line can name, so the ledger is the honest
				// answer rather than one of several recipes picked arbitrarily.
				await goto(`/imports/${asked.job_id}`);
				return;
			}
			noteArrival({ ...outcome.landed, jobId: asked.job_id });
			await goto(`/recipes/${outcome.landed.branchId}`);
		} catch (error) {
			// A Job that failed is raised as a plain Error by `waitForJob`, and a
			// refused Operation as an OperationError. A Job that outlasted the
			// wait is neither, and says it is still going (#117). All are this
			// act's to say:
			// a person who has just chosen a file is owed a sentence, never a
			// silence (#93).
			if (!(error instanceof Error)) throw error;
			if (error instanceof StillRunning) stillGoing = true;
			else failed = error.message;
		} finally {
			nowWorking('no');
		}
	}
</script>

<!--
	Outside both looks: the field is the same field either way, and it is never
	seen or reached directly. The button beside it is its control.

	**Kept out of the accessibility tree, not merely off the screen.** A file
	field maps to `role="button"` in Chromium, so an `sr-only` one sitting beside
	the real button announced the same act twice and put a stop in the tab order
	that looked identical to the next one — found on the dev instance with a
	screen-reader tree, 21 September 2026, not by any test here. `display: none`
	costs nothing: a file picker opens from a programmatic `.click()` whether the
	field is drawn or not.

	The label stays because it is what says *which* field this is when anything
	does reach it, and it is how a screen test grips it.
-->
<input
	type="file"
	accept=".zip,application/zip"
	class="hidden"
	aria-label={m.bring_in_file_full()}
	aria-hidden="true"
	tabindex="-1"
	bind:this={picker}
	disabled={disabled || working !== 'no'}
	onchange={fromFile}
/>

<!--
	The address field, drawn the same in both looks: revealing it in place is
	what makes the link offer *do* the thing rather than navigate (ADR 0027), and
	there is no reason the two looks should ask for an address differently.
-->
{#snippet address()}
	<form
		class="grid gap-2"
		onsubmit={(event) => {
			event.preventDefault();
			fromLink();
		}}
	>
		<label class="grid gap-1 text-read text-ink-2">
			{m.recipes_import_link()}
			<input
				type="url"
				bind:value={link}
				required
				placeholder="https://"
				class="min-h-12 rounded-sm border border-rule bg-card px-3 text-body text-ink"
			/>
		</label>
		<button
			class="min-h-12 rounded-sm bg-accent px-4 py-3 font-display text-body text-on-accent disabled:opacity-60"
			disabled={working !== 'no'}
		>
			{working === 'link' ? m.recipes_import_working() : m.recipes_nothing_import()}
		</button>
	</form>
{/snippet}

{#if look === 'offer'}
	<div class="grid gap-2">
		{#if !asking}
			<NeedsServer
				label={m.recipes_nothing_import()}
				waiting={m.offline_waits_import_link()}
				disabled={disabled || working !== 'no'}
				onclick={() => (asking = true)}
				shapeClass="min-h-12 rounded-sm px-4 py-3 text-left font-display text-body"
				lookClass="border border-rule bg-card text-accent disabled:opacity-60"
			/>
		{:else}
			{@render address()}
		{/if}

		<NeedsServer
			label={working === 'file' ? m.bring_in_file_working() : m.bring_in_file()}
			waiting={m.offline_waits_bring_in()}
			disabled={disabled || working !== 'no'}
			onclick={() => picker?.click()}
			shapeClass="min-h-12 rounded-sm px-4 py-3 text-left font-display text-body"
			lookClass="border border-rule bg-card text-accent disabled:opacity-60"
		/>
	</div>
{:else}
	<!--
		The quiet line above the shelf: a lead-in and the two sources, sized like
		the shelf's own meta text rather than like a control. The Recipes screen
		is deliberately spare — a search box, two filters, the shelf — and this
		is there every day for something most people do a handful of times, so it
		does not get a button's weight.

		Each act's visible words are a fragment, so each carries the whole act as
		its accessible name, with the fragment inside it (WCAG 2.5.3).

		The link trigger toggles here and only sets in the `offer` look, which is
		not an oversight: there the button is *replaced* by the address field, so
		there is nothing left to toggle with, while here it stays beside the field
		and a trigger that did nothing on a second tap would be a control that
		teaches people controls do nothing.

		**Offline the row becomes one sentence rather than two greyed acts.** Both
		need the server, and neither is ever queued — an offline import queue is a
		merge, and Kamosu never merges (#76, ADR 0013). But `NeedsServer` puts a
		whole sentence where each act's two words were, and on one line that came
		out as *Add a recipe Importing from a link waits for the server · Bringing
		a recipe in waits for the server*, under an offline card already saying the
		server is gone. Seen on the dev instance, 21 September 2026. So the row
		says the one thing once. The `offer` look keeps `NeedsServer` per act,
		because there each is its own full-width block and a sentence fits.
	-->
	{#if online.current}
		<p class="mt-3 text-read text-ink-2">
			{m.bring_in_add()}
			<button
				type="button"
				aria-label={m.bring_in_link_full()}
				onclick={() => (asking = !asking)}
				disabled={disabled || working !== 'no'}
				class="text-accent underline disabled:opacity-60">{m.bring_in_link()}</button
			>
			·
			<button
				type="button"
				aria-label={m.bring_in_file_full()}
				onclick={() => picker?.click()}
				disabled={disabled || working !== 'no'}
				class="text-accent underline disabled:opacity-60"
				>{working === 'file' ? m.bring_in_file_working() : m.bring_in_file_short()}</button
			>
		</p>
	{:else}
		<p class="mt-3 text-read text-ink-2">{m.offline_waits_bring_in_row()}</p>
	{/if}

	{#if asking && online.current}
		<div class="mt-2">{@render address()}</div>
	{/if}
{/if}

{#if failed}
	<p class="mt-2 text-read text-support" role="alert">{failed}</p>
{:else if stillGoing}
	<p class="mt-2 text-read text-ink-2" role="status">
		{m.bring_in_still_going({ where: `${m.settings_title()} › ${m.settings_imports()}` })}
	</p>
{/if}
