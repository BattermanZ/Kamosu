<!--
	The two things a person with nothing to look at was about to do anyway:
	write a recipe, or bring one in from a link.

	It lives here rather than inside one screen because two screens reach the
	same dead end from different directions — Recipes, when a search matched
	nothing (#62, ADR 0027), and Home, when there is nothing on the shelf to
	suggest from (#64). Both are the same situation: you do not have it yet, and
	adding it was the next thing you were going to do.

	**Both offers *do* the thing rather than pointing at a screen to do it on.**
	That is the whole point of them: an offer that only navigates has put a
	screen between a person and the one act they came for — and on an empty
	instance it can put them on a second empty screen, which is worse than
	saying nothing.

	A recipe needs only a title (#6), so writing one is a title and a tap. Where
	the caller already knows the title — Recipes knows it, because you typed it
	into the search box — it passes it and the field disappears: asking for a
	word somebody has just finished typing is asking them to type it twice.

	Reading a web page is slow, so importing is a Job (ADR 0032): it asks, waits
	on `get_job` — the one way any Job is ever read back — and opens whatever
	arrived.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { waitForJob } from '$lib/api/job';
	import type { ImportWebLinkOutput, ListKitchensOutput } from '$lib/api/catalogue';

	interface Props {
		/**
		 * The title to offer to write, where the caller already knows it. Absent
		 * means ask for one.
		 */
		title?: string;
		/**
		 * The Kitchens this Person cooks in, where the caller has already asked
		 * for them. The new recipe lands in the Home Kitchen; absent means ask.
		 */
		kitchens?: ListKitchensOutput['kitchens'];
	}

	let { title, kitchens }: Props = $props();

	const kamosu = useKamosu();

	/** Asked for only when the caller did not already have them. */
	let known = $state<ListKitchensOutput['kitchens']>([]);
	let typed = $state('');
	let adding = $state(false);
	let importing = $state<'no' | 'asking' | 'working'>('no');
	let link = $state('');
	let failed = $state<string | undefined>(undefined);

	const held = $derived(kitchens ?? known);
	/** What *Write a recipe* would write. The caller's title wins where there is one. */
	const naming = $derived((title ?? typed).trim());

	$effect(() => {
		if (kitchens) return;
		let current = true;
		kamosu
			.listKitchens({})
			.then((all) => {
				if (current) known = all.kitchens;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
			});
		return () => {
			current = false;
		};
	});

	/** A recipe needs only a title (#6), so the title *is* the recipe. */
	async function add() {
		const home = held.find((kitchen) => kitchen.is_home) ?? held[0];
		if (!home || naming === '') return;
		adding = true;
		failed = undefined;
		try {
			const made = await kamosu.createRecipe({ kitchen_id: home.id, title: naming });
			await goto(`/recipes/${made.branch_id}`);
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
			adding = false;
		}
	}

	async function importLink() {
		importing = 'working';
		failed = undefined;
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
			importing = 'asking';
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
			importing = 'asking';
		}
	}
</script>

<div class="grid gap-2">
	{#if title === undefined}
		<!--
			No title was handed down, so this is the one place it can come from.
			Submitting the field is the same act as the button, because a person
			who has typed a name and pressed enter has already asked.
		-->
		<form
			class="grid gap-2"
			onsubmit={(event) => {
				event.preventDefault();
				add();
			}}
		>
			<label class="grid gap-1 text-read text-ink-2">
				{m.add_title()}
				<input
					type="text"
					bind:value={typed}
					required
					class="min-h-12 rounded-sm border border-rule bg-card px-3 text-body text-ink"
				/>
			</label>
			<button
				class="min-h-12 rounded-sm bg-accent px-4 py-3 font-display text-body text-on-accent disabled:opacity-60"
				disabled={adding || importing === 'working'}
			>
				{m.add_write()}
			</button>
		</form>
	{:else}
		<button
			type="button"
			onclick={add}
			disabled={adding || importing === 'working'}
			class="min-h-12 rounded-sm bg-accent px-4 py-3 text-left font-display text-body text-on-accent disabled:opacity-60"
		>
			{m.recipes_nothing_add({ query: title })}
		</button>
	{/if}

	{#if importing === 'no'}
		<button
			type="button"
			onclick={() => (importing = 'asking')}
			disabled={adding}
			class="min-h-12 rounded-sm border border-rule bg-card px-4 py-3 text-left font-display text-body text-accent disabled:opacity-60"
		>
			{m.recipes_nothing_import()}
		</button>
	{:else}
		<form
			class="grid gap-2"
			onsubmit={(event) => {
				event.preventDefault();
				importLink();
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
				disabled={importing === 'working'}
			>
				{importing === 'working' ? m.recipes_import_working() : m.recipes_nothing_import()}
			</button>
		</form>
	{/if}

	{#if failed}
		<p class="text-read text-support" role="alert">{failed}</p>
	{/if}
</div>
