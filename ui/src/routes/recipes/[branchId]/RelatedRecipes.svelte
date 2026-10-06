<!--
	A recipe's Related Recipes, on the recipe (#105, #52).

	**A strip of the shelf's own cards, between the Method and Cooked**, which is
	Aurélien's choice of 22 September 2026 against a section of rows near the
	foot and a chip row beside the Tags. The reasoning is his and worth keeping:
	a related recipe is a DISH, so it is drawn as a dish. It wears the face it
	wears on the shelf — its photograph, or its Cover — at the shelf's own width,
	and never the shape of a label. The cost accepted is that anything past the
	second card is off the right edge until the strip is slid.

	It sits after the Method and the annexe and before Cooked, inside the layout
	#81 settled rather than reopening it. The strip is drawn on EVERY recipe,
	linked or not, following the choice made for Tags (#104): a library that
	arrives with 86 recipes and no links grows none if the only way in is hidden.

	A LINK IS BETWEEN TWO LINEAGES, UNTYPED, AND ONE LINK IS BOTH DIRECTIONS
	(#52, ADR 0025). Making it from either end makes the same single link, so
	there is nothing here that could be "the other way round". Nothing on this
	strip says the two recipes are joined, because they are not: no Lineage is
	merged, no Version is minted, and a card is a way to walk over there and
	nothing more.

	IT NEVER TRAVELS. Related Recipes stay behind a Bundle, a Share and a Sheet
	(ADR 0020, ADR 0023). This is a note on your own shelf, like filing (#104).

	A LINK TO A LINEAGE THAT HAS LEFT THE SHELF READS AS A NAME. The Core answers
	those with a remembered title and no Branch id, deliberately, because text is
	better than a broken pointer — so the card is drawn without a picture, does
	not open, and says why. Since #105 it can also be taken off: `RelatedSheet`
	names it by its Lineage, which a deleted recipe still has.

	NOTHING HERE MINTS A VERSION (ADR 0035). Relating does not appear in the
	Thread and is never presented as a change to the recipe, so there is no
	unsaved tray, no *save* and no *this will make a new version*.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import Cover from '$lib/cover/Cover.svelte';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';
	import { fallbackLanguage } from '../matched';
	import RelatedSheet, { type Related } from './RelatedSheet.svelte';

	interface Props {
		branchId: string;
		/**
		 * Whether the reader writes this recipe's Cookbook, which keeps its
		 * links (#131, question 3): only its Co-authors make or break one.
		 */
		writes: boolean;
		/** The recipe's Related Recipes as the page read them. */
		related: Related[];
	}

	let { branchId, writes, related }: Props = $props();

	/**
	 * The links as they stand: what the page read, then what the sheet did.
	 * Held here rather than re-reading the whole recipe, for `Tags.svelte`'s
	 * reason — `set_related_recipe` answers with this recipe's links, so the
	 * strip already has its next state, and re-reading a recipe to learn one
	 * name would throw away a Correction somebody was in the middle of.
	 */
	let held = $state<Related[] | null>(null);
	const shown = $derived(held ?? related);

	let picking = $state(false);

	/**
	 * The fallback sentence for one card, or null where there is nothing to
	 * mark. A departed Lineage has no Branch and so no Language: nothing to say.
	 */
	const marked = (entry: Related) =>
		entry.language === null ? null : fallbackLanguage({ ...entry, language: entry.language });
</script>

<!--
	Headed the way Ingredients, Method and Cooked are headed, in the same words
	and the same classes, because it is a section of the recipe page and not a
	widget sitting on one.
-->
{#if writes || shown.length > 0}
	<h2 class="mx-gutter mt-8 mb-2 font-display text-label font-semibold text-accent uppercase">
		{m.related_title()}
	</h2>
	<!--
	The gutter is padding rather than a margin so the strip scrolls to its last
	card and stops with the gutter still there, instead of clipping it flush.
-->
	<ul class="flex snap-x gap-3 overflow-x-auto px-gutter pb-1">
		{#each shown as entry (entry.lineage_id)}
			<li class="shrink-0 snap-start" style="width: var(--tile-w)">
				{#if entry.branch_id}
					<a href="/recipes/{entry.branch_id}" class="brightens block">
						<!--
						The shelf tile's frame (`Tile.svelte`): 3/4, rounded, the
						picture absolutely filling it, lazily loaded because a strip
						runs off the right edge and most of it is never looked at.

						The TITLE IS SET BENEATH rather than on a band inside the
						frame, which is where the shelf puts it — Aurélien's choice
						of 22 September 2026, made against this drawn on the real
						library. The band exists so a grid of Covers is readable at
						a glance; a strip of three, under a heading, on a page you
						are already reading, is not that.
					-->
						<span class="relative block overflow-hidden rounded-sm" style="aspect-ratio: 3 / 4">
							{#if entry.main_photo}
								<img
									src="/api/photographs/{entry.main_photo}/card"
									alt=""
									loading="lazy"
									class="absolute inset-0 h-full w-full object-cover"
								/>
							{:else}
								<Cover
									lineageId={entry.lineage_id}
									title={entry.title}
									height="100%"
									band={false}
								/>
							{/if}
							<!--
							Shown in a Language the reader did not ask for, and saying
							so — the shelf tile's own chip, in the same place and the
							same beni, because it is the same mark about the same
							recipe (ADR 0006). The code is what fits on the cloth; the
							whole sentence is there for anyone listening rather than
							looking, since "FR" read aloud is not a fallback anyone
							would understand.
						-->
							{#if marked(entry)}
								<span
									class="absolute top-2 right-2 rounded-sm bg-support px-1 py-1 text-label text-ground uppercase"
								>
									<span aria-hidden="true">{entry.language}</span>
									<span class="sr-only">{marked(entry)}</span>
								</span>
							{/if}
						</span>
						<span class="mt-1 block text-tile-title text-ink">{entry.title}</span>
					</a>
				{:else}
					<!--
					Gone from this shelf. Drawn as the recessed ground rather than as
					a Cover: a Cover is what a recipe HAS, and there is no recipe
					here any more — only the name this Kitchen remembers it by.
				-->
					<span class="block rounded-sm bg-ground-2" style="aspect-ratio: 3 / 4"></span>
					<span class="mt-1 block text-tile-title text-ink-2">{entry.title}</span>
					<span class="block text-read text-ink-2">{m.related_gone()}</span>
				{/if}
			</li>
		{/each}

		<!--
		Dashed rather than filled, as the Tags row's own opener is: it is the one
		card in the strip that is not a recipe, and on a recipe with no links it
		is the whole strip, where a filled card would read as a recipe called
		*Relate a recipe*.

		It waits for the server, like Tagging and editing (#76, ADR 0013). An
		offline queue for this would be a merge of two Cookbooks' links, and
		Kamosu does not merge. Only a Co-author of the recipe's Cookbook sees
		it, since the Cookbook keeps the links (#131, question 3).
	-->
		{#if writes}
			<li class="shrink-0 snap-start" style="width: var(--tile-w); aspect-ratio: 3 / 4">
				<NeedsServer
					label={m.related_add()}
					waiting={m.offline_waits_edit()}
					onclick={() => (picking = true)}
					shapeClass="flex h-full w-full items-center justify-center rounded-sm p-3 text-center text-read"
					lookClass="border border-dashed border-rule text-accent"
					idleClass="border border-dashed border-rule text-ink-2 opacity-55"
				/>
			</li>
		{/if}
	</ul>
{/if}

{#if picking}
	<RelatedSheet
		{branchId}
		carried={shown}
		onChanged={(next) => (held = next)}
		onClose={() => (picking = false)}
	/>
{/if}
