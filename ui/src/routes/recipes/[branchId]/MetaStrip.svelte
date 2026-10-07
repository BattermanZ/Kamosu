<!--
	The meta: one full-bleed strip, three cells, hairlines between (#81).
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import type { GetRecipeOutput } from '$lib/api/catalogue';
	import { figureOf, unitWord } from '$lib/duration';

	type Content = GetRecipeOutput['versions'][number]['content'];

	let { content }: { content: Pick<Content, 'prep_time_minutes' | 'cook_time_minutes' | 'yield'> } =
		$props();
</script>

<!--
	A time in the strip, with its unit in the figure: `15 min`, `1 h 30`, `9 h`
	(#175, reading option 1). It printed the stored minutes bare
	before, over "min prep", so a 9-hour prove read 540. The label beneath is
	now only Prep or Cook, since the figure says its own unit.
-->
{#snippet timeFigure(minutes: number)}
	<b class="block font-display text-panel-figure font-semibold">
		{#each figureOf(minutes) as part, index (index)}
			<span class={index > 0 ? 'ms-1' : ''}
				>{part.value}{#if part.unit}<small class="ms-1 text-read font-normal"
						>{unitWord(part.unit)}</small
					>{/if}</span
			>
		{/each}
	</b>
{/snippet}

{#if content.prep_time_minutes !== null || content.cook_time_minutes !== null || content.yield}
	<div class="mt-4 flex border-y border-rule">
		{#if content.prep_time_minutes !== null}
			<div class="flex-1 px-2 py-3 text-center">
				{@render timeFigure(content.prep_time_minutes)}
				<span class="mt-1 block text-label text-ink-2 uppercase">{m.recipe_prep()}</span>
			</div>
		{/if}
		{#if content.cook_time_minutes !== null}
			<div class="flex-1 border-l border-rule px-2 py-3 text-center first:border-l-0">
				{@render timeFigure(content.cook_time_minutes)}
				<span class="mt-1 block text-label text-ink-2 uppercase">{m.recipe_cook()}</span>
			</div>
		{/if}
		{#if content.yield}
			<div class="flex-1 border-l border-rule px-2 py-3 text-center first:border-l-0">
				<b class="block font-display text-panel-figure font-semibold">
					{content.yield.amount}
				</b>
				<span class="mt-1 block text-label text-ink-2 uppercase">{content.yield.noun}</span>
			</div>
		{/if}
	</div>
{/if}
