<!--
	Recipes: one shelf, and the one home for searching (#62, ADR 0027).

	Everything the Kitchens you cook in hold, merged, alphabetical, one card per
	Lineage. There is no Kitchen switcher and no modes — a switcher is filing
	through the back door, and it taxes every search with a permissions question
	you should never have to answer to find the katsu curry on a Tuesday.

	The three filters are **held here and nowhere else**, which is the whole of
	how "no filter sticks" is built: leaving the screen destroys this component,
	and coming back makes a new one on the whole shelf.

	The tag filter (#104) is the third, and it is the one exception to "nothing
	is written to the URL" — `?tag=` exists so that a chip on a recipe page can
	say which tag it meant, and the route reads it into a prop this screen takes
	once. It is a message from the other screen, not a place the filter lives:
	nothing here ever writes it, so the back button cannot resurrect a filter and
	arriving by the tab bar lands on the whole shelf as it always did.

	Searching is one Operation, the same one that answers the unsearched shelf.
	Meaning Search arrived inside that Operation rather than beside it, so
	nothing here had to learn a second way of asking (ADR 0029) — the answer
	simply gained two things: a match can say it was found by meaning, and an
	answer can say that nothing was close enough and these are the nearest.
-->
<script lang="ts">
	import { untrack } from 'svelte';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { MeaningSearch } from '$lib/meaning.svelte';
	import { RecipeSearch } from '$lib/search.svelte';
	import { byWord, tagWord, type Tag } from '$lib/tags';
	import type { ListKitchensOutput } from '$lib/api/catalogue';
	import Screen from '$lib/shell/Screen.svelte';
	import Empty from '$lib/shell/Empty.svelte';
	import Tile from './Tile.svelte';
	import AddOrImport from '$lib/AddOrImport.svelte';
	import Plus from './Plus.svelte';
	import MeaningOffer from './MeaningOffer.svelte';

	interface Props {
		/** The Tag named by `?tag=`, where the reader arrived from a chip (#104). */
		tag?: string | null;
	}

	/** Read once, on purpose: see `tag` below. */
	const { tag: arrivedTagged = null }: Props = $props();

	const kamosu = useKamosu();

	/** What is typed, moment to moment. The Operation is asked on a short delay. */
	let typed = $state('');
	/** Which filter is held right now: the whole shelf, this Person's own, or one Kitchen. */
	let filter = $state<{ kind: 'all' } | { kind: 'mine' } | { kind: 'kitchen'; id: string }>({
		kind: 'all',
	});
	/**
	 * The Tag this shelf is narrowed to, or nothing (#104). A third filter
	 * beside the two above and held exactly as they are — here, and nowhere
	 * else. It composes with both: the recipes Marc tagged *spicy*, searched
	 * for *chicken*, is one ask.
	 *
	 * Seeded once from the route's `?tag=`, because a chip on a recipe page
	 * arrives by navigating. The query string is how that screen says which
	 * tag, not a place this filter lives: coming back to Recipes by the tab
	 * destroys this component and makes a new one on the whole shelf, exactly as
	 * the other two filters already behave.
	 */
	let tag = $state<string | null>(untrack(() => arrivedTagged));

	let kitchens = $state<ListKitchensOutput['kitchens']>([]);
	/**
	 * Every word the Cookbooks on this shelf file by, one entry per word
	 * however many of them use it (#131, question 3): "Dessert" from two
	 * Cookbooks is one chip, and filtering by it finds what each filed there.
	 * Every Kitchen's own list is read beside it, once, so a filter tap swaps
	 * one row of chips for another already here instead of putting a round
	 * trip between the thumb and the row.
	 */
	let tags = $state<Tag[]>([]);
	/**
	 * Each Kitchen's own words, by Kitchen id: what its Cookbooks file by, one
	 * entry per word. Read with the rest rather than narrowed from `tags`,
	 * because `tags` keeps ONE Tag per word and that Tag may belong to a
	 * Cookbook the Kitchen does not see, which would hide a word the Kitchen
	 * really files by.
	 */
	let kitchenTags = $state<Record<string, Tag[]>>({});
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
			.then(async (held) => {
				if (!current) return;
				kitchens = held.kitchens;
				// Every word of every Cookbook on the shelf, merged by the Core
				// into one entry per word — the same merge the shelf itself is
				// (ADR 0027). Failing costs only the row, never the shelf.
				const read = (input: { everywhere: true } | { kitchen_id: string }) =>
					kamosu.listTags(input).catch((error: unknown) => {
						if (!(error instanceof OperationError)) throw error;
						return { tags: [] };
					});
				const [words, ...perKitchen] = await Promise.all([
					read({ everywhere: true }),
					...held.kitchens.map((kitchen) => read({ kitchen_id: kitchen.id })),
				]);
				if (current) {
					tags = words.tags;
					kitchenTags = Object.fromEntries(
						held.kitchens.map((kitchen, index) => [kitchen.id, perKitchen[index]?.tags ?? []]),
					);
				}
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

	/**
	 * The Tags to offer: those of the Cookbooks seen in the Kitchen being
	 * filtered to, or all of them.
	 * Narrowed HERE rather than by asking again, so the row follows a Kitchen
	 * chip the moment it is tapped — a chip that can only ever find nothing is
	 * worse than no chip.
	 */
	const chips = $derived.by(() => {
		// Read into a local first, as the search below does: a union held
		// in state does not stay narrowed across the closure that reads it.
		const held = filter;
		return byWord(held.kind === 'kitchen' ? (kitchenTags[held.id] ?? []) : tags);
	});

	/** The Tag being filtered by, once the list holding it has been read. */
	const filtering = $derived(
		[tags, ...Object.values(kitchenTags)].flat().find((held) => held.id === tag) ?? null,
	);
	/** Its word, for the count and the nothing-found line. */
	const filteringWord = $derived(filtering ? tagWord(filtering).name : null);

	/**
	 * A Tag chip that goes out of reach keeps its filter on with no way to turn
	 * it off — which is what happens when a Kitchen chip narrows the row past
	 * the Tag being filtered by. So the filter follows the row: taking the chip
	 * away takes the narrowing with it.
	 */
	$effect(() => {
		if (tag !== null && tags.length > 0 && !chips.some((held) => held.id === tag)) tag = null;
	});

	// Asked on open, and again whenever turning Meaning Search on or declining
	// it changed the answer — so the offer disappears the moment it is answered.
	$effect(() => meaning.ask());

	/**
	 * The shelf's search: whatever is typed, under whichever filters are held.
	 * Turning Meaning Search on changes what this same search finds, so it is
	 * asked again — which is the only confirmation worth giving: the recipe you
	 * were looking for appears.
	 */
	const search = new RecipeSearch(
		kamosu,
		() => {
			const query = typed.trim();
			// Read into a local first: a union held in state does not stay
			// narrowed across the property reads below.
			const asked = filter;
			return {
				query: query === '' ? null : query,
				kitchen_id: asked.kind === 'kitchen' ? asked.id : null,
				mine: asked.kind === 'mine',
				tag_id: tag,
			};
		},
		() => meaning.generation,
	);
	const answer = $derived(search.answer);
	const failed = $derived(search.failed);

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
	 * than a flag because the block below needs both facts at once: that
	 * nothing matched, and what did not match.
	 */
	const unmatched = $derived(query !== null && (entries.length === 0 || closest) ? query : null);

	// The things nothing-found offers live in `AddOrImport`, because Home
	// reaches the same dead end from the other direction (#64) and both must
	// *do* the thing rather than point at a screen to do it on (ADR 0027).
</script>

<Screen title={m.recipes_title()} blurb={m.recipes_blurb()}>
	<search>
		<!--
			The + for every new recipe sits at the end of the search box (#174),
			there with the shelf, while a search runs, and when it finds nothing.
		-->
		<Plus>
			<label class="block">
				<span class="sr-only">{m.recipes_search()}</span>
				<input
					type="search"
					bind:value={typed}
					placeholder={m.recipes_search()}
					class="min-h-12 w-full rounded-sm border border-rule bg-card px-3 text-body"
				/>
			</label>
		</Plus>

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

		<!--
			The tags, as ONE SIDEWAYS LINE and never a block that wraps (#104).
			Twenty-four tags wrapped is six rows of chips standing between the
			search field and the recipes, which is the measurement that decided
			this; Home already settled that a section carries a rail rather than
			a grid. Full-bleed, so the line visibly runs off the edge — that is
			what says there is more of it.

			Absent entirely until a Kitchen has a tag: on a new instance, and on
			this project's own 86-recipe library the day it was imported, this
			row would otherwise be a heading over nothing.
		-->
		{#if chips.length > 0}
			<h2 class="mt-4 text-label text-ink-2 uppercase">{m.tags_title()}</h2>
			<!-- `relative` holds each chip's screen-reader words inside this row.
			     `.sr-only` is positioned absolutely, and with nothing positioned
			     above it that means against the page: a chip scrolled off to the
			     right pushed the whole shelf sideways, 615px wide on a 390px phone,
			     as soon as a Tag carried a Language mark (#112). -->
			<div
				class="relative -mx-gutter mt-2 flex [scrollbar-width:none] flex-nowrap gap-2 overflow-x-auto px-gutter"
			>
				{#each chips as held (held.id)}
					{@const word = tagWord(held)}
					<button
						type="button"
						aria-pressed={tag === held.id}
						onclick={() => (tag = tag === held.id ? null : held.id)}
						class="min-h-8 shrink-0 rounded-sm border px-3 text-read {tag === held.id
							? 'border-accent bg-accent text-on-accent'
							: 'border-rule text-ink-2'}"
					>
						{word.name}
						{#if word.elsewhere}
							<span class="text-label uppercase {tag === held.id ? '' : 'text-support'}">
								<span aria-hidden="true">{word.elsewhere}</span>
								<span class="sr-only">{word.said}</span>
							</span>
						{/if}
					</button>
				{/each}
			</div>
		{/if}
	</search>

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
				<AddOrImport title={unmatched} />
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
		<!--
			A tag narrowing the shelf to nothing is not an empty library, and
			saying "you have no recipes" there would be a lie about the other
			eighty-five. It says which word found nothing and offers the way
			back out, which is the chip that is still lit above.
		-->
		<Empty>
			{filteringWord ? m.tags_shelf_none({ tag: filteringWord }) : m.recipes_empty()}
		</Empty>
	{:else}
		<p class="mt-4 mb-3 text-label text-ink-2 uppercase" role="status">
			{#if filteringWord && query === null}
				<!--
					Counted by the Tag rather than by the shelf, because that is
					the question the reader just asked. It counts what is on this
					answer, so it is the Tag's own number only while no other
					filter is narrowing it further — Settings counts a Tag across
					its whole Kitchen and says so there.
				-->
				{entries.length === 1
					? m.tags_shelf_count_one({ tag: filteringWord })
					: m.tags_shelf_count({ count: entries.length, tag: filteringWord })}
			{:else if query === null}
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
