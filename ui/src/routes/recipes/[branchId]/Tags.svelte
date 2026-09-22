<!--
	A recipe's Tags, on the recipe (#104).

	**The row is on every recipe, tagged or not**, which is Aurélien's choice of
	22 September 2026 against the quieter option that drew nothing until a recipe
	had a Tag. The reasoning is his and worth keeping: a library that arrives
	with 86 recipes and no Tags at all grows none if the only way in is a button
	at the foot of a long page. The cost accepted is one quiet row on the recipes
	nobody ever Tags.

	It sits under the meta strip, where the recipe's facts end and its content
	begins — the Tags are what this Kitchen says about the dish, above the dish
	itself.

	A TAG IS A WAY OF GETTING AROUND, so a chip is a link to the shelf filtered
	by it rather than a label you can only read (#104, ADR 0027). It is a real
	filter and not a search for the word: *spicy* means the recipes you tagged,
	never the one whose title happens to say spicy.

	NOTHING HERE MINTS A VERSION (ADR 0035). Tagging does not appear in the
	Thread and is never presented as a change to the recipe, so there is no
	unsaved tray, no *save* and no *this will make a new version*. Opening an old
	Version shows today's Tags, on purpose: filing is present tense.

	Adding one waits for the server, like editing (#76, ADR 0013). An offline
	queue for this would be a merge of two Kitchens' filing, and Kamosu does not
	merge.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';
	import { byWord, tagWord, type Tag } from '$lib/tags';
	import TagSheet from './TagSheet.svelte';

	interface Props {
		branchId: string;
		kitchenId: string;
		/** The recipe's tags as the page read them. */
		tags: Tag[];
	}

	let { branchId, kitchenId, tags }: Props = $props();

	/**
	 * The Tags as they stand, which starts as what the page read and then
	 * follows what the sheet did. Held here rather than re-reading the whole
	 * recipe: `set_recipe_tag` answers with the recipe's Tags, so the row
	 * already has its next state, and re-reading a recipe to learn one word
	 * would throw away a Correction somebody was in the middle of.
	 */
	let held = $state<Tag[] | null>(null);
	const shown = $derived(held ?? tags);

	let picking = $state(false);
</script>

<!--
	Headed the way Ingredients and Method are headed, in the same words and the
	same classes, because it is a section of the recipe page and not a widget
	sitting on one.
-->
<h2 class="mx-gutter mt-8 mb-2 font-display text-label font-semibold text-accent uppercase">
	{m.tags_title()}
</h2>
<div class="flex flex-wrap gap-2 px-gutter">
	{#each byWord(shown) as tag (tag.id)}
		{@const named = tagWord(tag)}
		<a
			href="/recipes?tag={encodeURIComponent(tag.id)}"
			class="inline-flex min-h-8 items-baseline gap-2 rounded-sm border border-rule bg-card px-3
			py-1 text-read text-ink"
		>
			{named.name}
			{#if named.elsewhere}
				<!--
					Beni, the identity's Language-badge colour, and `Tile.svelte`'s
					rule for what it says: the code is what fits beside a word, and
					the whole sentence is there for anyone listening rather than
					looking, since "FR" read aloud is not a fallback anyone would
					understand.
				-->
				<span class="text-label text-support uppercase">
					<span aria-hidden="true">{named.elsewhere}</span>
					<span class="sr-only">{named.said}</span>
				</span>
			{/if}
		</a>
	{/each}

	<!--
		Dashed rather than filled: it is the one chip in the row that is not a
		Tag, and on an untagged recipe it is the whole row, where a filled chip
		would read as a Tag called *Add a Tag*.
	-->
	<NeedsServer
		label={m.tags_add()}
		waiting={m.offline_waits_tagging()}
		onclick={() => (picking = true)}
		shapeClass="inline-flex min-h-8 items-center gap-1 rounded-sm px-3 py-1 text-read"
		lookClass="border border-dashed border-rule text-accent"
		idleClass="border border-dashed border-rule text-ink-2 opacity-55"
	/>
</div>

{#if picking}
	<TagSheet
		{branchId}
		{kitchenId}
		carried={shown}
		onChanged={(next) => (held = next)}
		onClose={() => (picking = false)}
	/>
{/if}
