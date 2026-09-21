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
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { MeaningSearch } from '$lib/meaning.svelte';
	import type { ListKitchensOutput, SearchRecipesOutput } from '$lib/api/catalogue';
	import Screen from '$lib/shell/Screen.svelte';
	import Empty from '$lib/shell/Empty.svelte';
	import Tile from './Tile.svelte';
	import AddOrImport from '$lib/AddOrImport.svelte';
	import BringIn from '$lib/BringIn.svelte';
	import MeaningOffer from './MeaningOffer.svelte';
	import { refreshed } from '$lib/offline/device.svelte';

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
		// The phone answered first and the server has since answered otherwise (#76).
		void refreshed.get('search_recipes');

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
	/**
	 * What was searched for and not found, or `null` where this screen is
	 * showing a shelf rather than *nothing found*. Carries the query rather
	 * than a flag because the block below needs both facts at once — that
	 * nothing matched, and what did not match — and because two things read it:
	 * that block, and the row above the shelf that stands down while it is up
	 * (#93).
	 */
	const unmatched = $derived(query !== null && (entries.length === 0 || closest) ? query : null);

	// The two things nothing-found offers live in `AddOrImport`, because Home
	// reaches the same dead end from the other direction (#64) and both must
	// *do* the thing rather than point at a screen to do it on (ADR 0027).
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

	<!--
		The two ways a recipe comes from outside, above the shelf and there every
		day (#93). They used to appear only at the dead ends below — a search that
		matched nothing, an empty Home — which is the right place to *offer* them
		and the wrong place to *keep* them: a person holding a recipe file a friend
		has just sent has no failed search to arrive through.

		Not drawn in the nothing-found state, because `AddOrImport` already offers
		both acts there, and saying the same thing twice on one screen is worse
		than saying it once in the wrong place.
	-->
	{#if answer && unmatched === null}
		<BringIn look="quiet" />
	{/if}

	{#if failed}
		<p class="mt-6 text-body text-support" role="alert">{m.recipes_failed()}</p>
	{:else if !answer}
		<p class="mt-6 text-body text-ink-2">{m.loading()}</p>
	{:else if unmatched !== null}
		<!--
			Nothing found is not an empty screen. It says what was looked
			through, and offers the two things you were about to do anyway —
			because with a library this size a search that finds nothing usually
			means you do not have it yet (ADR 0027).
		-->
		<div class="mt-6">
			<h2 class="font-display text-shelf-heading font-semibold">
				{m.recipes_nothing_title({ query: unmatched })}
			</h2>
			<p class="mt-2 text-read text-ink-2">
				{meaning.status?.on ? m.recipes_nothing_why_meaning() : m.recipes_nothing_why()}
			</p>
			<div class="mt-4">
				<AddOrImport title={unmatched} {kitchens} />
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
