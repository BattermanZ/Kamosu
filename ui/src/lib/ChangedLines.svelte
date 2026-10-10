<!--
	The lines a cooking did not leave alone, as the recipe page's offer (#58)
	and the diary's (#210) both show them before the cooking is kept: what the
	line says now, with what the recipe said struck under it. A line the cook
	left out reads as the recipe has it, struck, because that is what would go.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import type { ChangedLine } from '$lib/as-cooked';

	let { lines }: { lines: ChangedLine[] } = $props();
</script>

<ul>
	{#each lines as line (line.key)}
		<li class="border-b border-rule py-2">
			{#if line.state === 'only-mine'}
				<span class="block text-label text-ink-2 uppercase">
					{m.recipe_as_cooked_dropped()}
				</span>
				<span class="block font-display text-line text-ink-2 line-through">{line.was}</span>
			{:else}
				{#if line.state === 'only-theirs'}
					<span class="block text-label text-ink-2 uppercase">
						{m.recipe_as_cooked_added()}
					</span>
				{/if}
				<span class="block font-display text-line">{line.now}</span>
				{#if line.was}
					<span class="block text-read text-ink-2 line-through">{line.was}</span>
				{/if}
			{/if}
		</li>
	{/each}
</ul>
