<!--
	Recipes: one shelf, and the one home for searching (#62, ADR 0027).

	Everything the Kitchens you cook in hold, merged, alphabetical, one card per
	Lineage. There is no Kitchen switcher and no modes — a switcher is filing
	through the back door, and it taxes every search with a permissions question
	you should never have to answer to find the katsu curry on a Tuesday.

	The two filters are **held here and nowhere else**, which is the whole of
	how "neither filter sticks" is built: leaving the screen destroys this
	component, and coming back makes a new one on the whole shelf. Nothing is
	written to the URL either, or a filter would survive in the back button.

	Searching is one Operation, the same one that answers the unsearched shelf.
	Meaning Search arrived inside that Operation rather than beside it, so
	nothing here had to learn a second way of asking (ADR 0029) — the answer
	simply gained two things: a match can say it was found by meaning, and an
	answer can say that nothing was close enough and these are the nearest.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { waitForJob } from '$lib/api/job';
	import { MeaningSearch } from '$lib/meaning.svelte';
	import type {
		ImportWebLinkOutput,
		ListKitchensOutput,
		SearchRecipesOutput,
	} from '$lib/api/catalogue';
	import Screen from '$lib/shell/Screen.svelte';
	import Empty from '$lib/shell/Empty.svelte';
	import Tile from './Tile.svelte';
	import MeaningOffer from './MeaningOffer.svelte';

	const kamosu = useKamosu();

	/** What is typed, moment to moment. The Operation is asked on a short delay. */
	let typed = $state('');
	/** Which filter is held right now: the whole shelf, this Person's own, or one Kitchen. */
	let filter = $state<{ kind: 'all' } | { kind: 'mine' } | { kind: 'kitchen'; id: string }>({
		kind: 'all',
	});

	let kitchens = $state<ListKitchensOutput['kitchens']>([]);
	let answer = $state<SearchRecipesOutput | undefined>(undefined);
	let failed = $state(false);
	/**
	 * Whether this Kamosu matches on meaning, and whether this reader is the
	 * one to be asked about it. Both come from the Operation rather than being
	 * worked out here: only an Operator can turn it on, and it is never offered
	 * twice to somebody who declined (ADR 0029).
	 */
	const meaning = new MeaningSearch(kamosu);

	/**
	 * A filter naming a Kitchen only appears where there is more than one to
	 * name. With a single Kitchen it would filter nothing, and a control that
	 * does nothing teaches people that controls do nothing.
	 */
	const several = $derived(kitchens.length > 1);

	$effect(() => {
		let current = true;
		kamosu
			.listKitchens({})
			.then((held) => {
				if (current) kitchens = held.kitchens;
			})
			.catch((error: unknown) => {
				// The shelf itself is what this screen is for; failing to learn
				// the Kitchens costs only the filters, so it is not an error
				// the whole screen wears.
				if (!(error instanceof OperationError)) throw error;
			});
		return () => {
			current = false;
		};
	});

	// Asked on open, and again whenever turning Meaning Search on or declining
	// it changed the answer — so the offer disappears the moment it is answered.
	$effect(() => meaning.ask());

	$effect(() => {
		// Read every input this ask depends on before the delay, so the effect
		// re-runs when any of them changes rather than only on the first.
		const query = typed.trim();
		const asked = filter;
		// Turning Meaning Search on changes what this same search finds, so the
		// search is asked again — which is the only confirmation worth giving:
		// the recipe you were looking for appears.
		const generation = meaning.generation;

		let current = true;
		const timer = setTimeout(() => {
			void (async () => {
				try {
					const found = await kamosu.searchRecipes({
						query: query === '' ? null : query,
						kitchen_id: asked.kind === 'kitchen' ? asked.id : null,
						mine: asked.kind === 'mine',
					});
					if (current && generation === meaning.generation) {
						answer = found;
						failed = false;
					}
				} catch (error) {
					if (!(error instanceof OperationError)) throw error;
					if (current) failed = true;
				}
			})();
			// Long enough that typing a recipe's name is one ask rather than
			// twelve, short enough that the shelf keeps up with the thumb.
		}, 180);

		return () => {
			current = false;
			clearTimeout(timer);
		};
	});

	const entries = $derived(answer?.recipes ?? []);
	/** What the answer was actually for — never what the field says now. */
	const query = $derived(answer?.query ?? null);
	/**
	 * Whether these are matches or merely the nearest there were. Meaning
	 * Search always has a nearest neighbour, so *nothing found* means *nothing
	 * close enough* — and showing the closest under that label is not the same
	 * act as letting them pass as matches (ADR 0027).
	 */
	const closest = $derived(answer?.closest ?? false);

	// --- the two things nothing-found offers ------------------------------
	//
	// Both of them *do* the thing rather than pointing at a screen to do it
	// on. With a library this size a search that finds nothing usually means
	// you do not have that recipe yet, and adding it was the next thing you
	// were going to do (ADR 0027) — so an offer that only navigates somewhere
	// has put a screen between a person and the one act they came for.

	let adding = $state(false);
	let importing = $state<'no' | 'asking' | 'working'>('no');
	let link = $state('');
	let offerFailed = $state<string | undefined>(undefined);

	/**
	 * Add the recipe that was searched for. A recipe needs only a title (#6),
	 * so the query *is* the recipe — it lands in the Home Kitchen and opens.
	 */
	async function add(title: string) {
		const home = kitchens.find((kitchen) => kitchen.is_home) ?? kitchens[0];
		if (!home) return;
		adding = true;
		offerFailed = undefined;
		try {
			const made = await kamosu.createRecipe({ kitchen_id: home.id, title });
			await goto(`/recipes/${made.branch_id}`);
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			offerFailed = error.message;
			adding = false;
		}
	}

	/**
	 * Import from a link. Reading a web page is slow, so it is a Job (ADR 0032):
	 * this asks, waits on `get_job` — the one way any Job is ever read back —
	 * and opens whatever arrived.
	 */
	async function importLink() {
		importing = 'working';
		offerFailed = undefined;
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
			offerFailed = result.unreadable[0]?.reason ?? m.recipes_import_unreadable();
			importing = 'asking';
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			offerFailed = error.message;
			importing = 'asking';
		}
	}
</script>

<Screen title={m.recipes_title()} blurb={m.recipes_blurb()}>
	<search>
		<label class="block">
			<span class="sr-only">{m.recipes_search()}</span>
			<input
				type="search"
				bind:value={typed}
				placeholder={m.recipes_search()}
				class="min-h-12 w-full rounded-sm border border-rule bg-card px-3 text-body"
			/>
		</label>

		<div class="mt-3 flex flex-wrap gap-2">
			{#snippet chip(name: string, held: boolean, choose: () => void)}
				<button
					type="button"
					aria-pressed={held}
					onclick={choose}
					class="min-h-8 rounded-sm border px-3 text-read {held
						? 'border-accent bg-accent text-on-accent'
						: 'border-rule text-ink-2'}"
				>
					{name}
				</button>
			{/snippet}

			{@render chip(
				m.recipes_filter_all(),
				filter.kind === 'all',
				() => (filter = { kind: 'all' }),
			)}
			{@render chip(
				m.recipes_filter_mine(),
				filter.kind === 'mine',
				() => (filter = { kind: 'mine' }),
			)}
			{#if several}
				{#each kitchens as kitchen (kitchen.id)}
					{@render chip(
						kitchen.nickname ?? kitchen.name,
						filter.kind === 'kitchen' && filter.id === kitchen.id,
						() => (filter = { kind: 'kitchen', id: kitchen.id }),
					)}
				{/each}
			{/if}
		</div>
	</search>

	{#if failed}
		<p class="mt-6 text-body text-support" role="alert">{m.recipes_failed()}</p>
	{:else if !answer}
		<p class="mt-6 text-body text-ink-2">{m.loading()}</p>
	{:else if query !== null && (entries.length === 0 || closest)}
		<!--
			Nothing found is not an empty screen. It says what was looked
			through, and offers the two things you were about to do anyway —
			because with a library this size a search that finds nothing usually
			means you do not have it yet (ADR 0027).
		-->
		<div class="mt-6">
			<h2 class="font-display text-shelf-heading font-semibold">
				{m.recipes_nothing_title({ query })}
			</h2>
			<p class="mt-2 text-read text-ink-2">
				{meaning.status?.on ? m.recipes_nothing_why_meaning() : m.recipes_nothing_why()}
			</p>
			<div class="mt-4 grid gap-2">
				<button
					type="button"
					onclick={() => add(query)}
					disabled={adding || importing === 'working'}
					class="min-h-12 rounded-sm bg-accent px-4 py-3 text-left font-display text-body text-on-accent disabled:opacity-60"
				>
					{m.recipes_nothing_add({ query })}
				</button>

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

				{#if offerFailed}
					<p class="text-read text-support" role="alert">{offerFailed}</p>
				{/if}
			</div>
			<!--
				Offered here rather than buried in settings: this is the moment a
				person can see exactly what they are missing (ADR 0029). Whether
				to make the offer at all is the Operation's answer, not this
				screen's — it is never made to somebody who cannot act on it,
				and never again to an Operator who declined.
			-->
			{#if meaning.status?.offer}
				<MeaningOffer {meaning} status={meaning.status} />
			{/if}
		</div>

		{#if closest}
			<!--
				The closest anyway, under exactly that label. Meaning-matching
				always has a nearest neighbour, so an empty screen would be a
				lie about what Kamosu knows — and an unlabelled list would be a
				worse one, letting a weak match pass as a good one (ADR 0027).
			-->
			<p class="mt-6 mb-3 text-label text-ink-2 uppercase" role="status">
				{m.recipes_closest()}
			</p>
			<ul class="grid grid-cols-2 gap-3">
				{#each entries as entry (entry.lineage_id)}
					<Tile {entry} />
				{/each}
			</ul>
		{/if}
	{:else if entries.length === 0}
		<Empty>{m.recipes_empty()}</Empty>
	{:else}
		<p class="mt-4 mb-3 text-label text-ink-2 uppercase" role="status">
			{#if query === null}
				{entries.length === 1 ? m.recipes_count_one() : m.recipes_count({ count: entries.length })}
			{:else}
				{entries.length === 1 ? m.recipes_found_one() : m.recipes_found({ count: entries.length })}
			{/if}
		</p>
		<ul class="grid grid-cols-2 gap-3">
			{#each entries as entry (entry.lineage_id)}
				<Tile {entry} />
			{/each}
		</ul>
	{/if}
</Screen>
