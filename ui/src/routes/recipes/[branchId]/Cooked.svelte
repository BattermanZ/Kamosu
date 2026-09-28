<!--
	Cooked (#59). How this dish has actually gone: how many times, when last,
	and each Person's most recent verdict with their name.

	There is no average here and no way to build one, which is ADR 0015 working
	rather than a rule anybody has to remember — a rating is a word, and only
	the newest one each Person gave ever arrives, so a verdict somebody has
	since superseded cannot drag down a recipe that was fixed months ago. A
	Person who cooked and said nothing simply does not appear: silence is not a
	score of zero.

	Worded impersonally, as the delete confirmation is (`DeleteRecipe.svelte`):
	the count is the household's, never the reader's.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import { m } from '$lib/paraglide/messages';
	import type { GetRecipeOutput } from '$lib/api/catalogue';
	import { ratingLabel } from '$lib/rating';

	interface Props {
		cooked: GetRecipeOutput['cooked'];
		/** What else belongs among how the dish has gone: your own pictures of it. */
		children?: Snippet;
	}

	let { cooked, children }: Props = $props();
</script>

<h2 class="mx-gutter mt-8 mb-2 font-display text-label font-semibold text-accent uppercase">
	{m.recipe_cooked()}
</h2>
<div class="px-gutter">
	{#if cooked.count === 0 || !cooked.last_cooked_at}
		<p class="text-read text-ink-2">{m.recipe_cooked_never()}</p>
	{:else}
		{@const when = new Date(cooked.last_cooked_at).toLocaleDateString()}
		<p class="text-read text-ink-2">
			{cooked.count === 1
				? m.recipe_cooked_once({ when })
				: m.recipe_cooked_times({ count: cooked.count, when })}
		</p>
		<ul>
			{#each cooked.ratings as verdict (verdict.person_id)}
				<li class="flex items-baseline justify-between gap-3 border-b border-rule py-2">
					<span class="text-line">{verdict.name}</span>
					<span class="text-read text-accent uppercase">{ratingLabel(verdict.rating)}</span>
				</li>
			{/each}
		</ul>
	{/if}
	{@render children?.()}
</div>
