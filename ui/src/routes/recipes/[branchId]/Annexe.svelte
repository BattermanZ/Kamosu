<!--
	THE ANNEXE (#50): one Component's own Steps, at the foot of the page, under
	a heading in this page's own Section grammar but in matcha — so it reads as
	belonging to the Component rather than to this recipe's method.

	Never spliced into the Method above it. Composition says WHAT and never
	WHEN: Kamosu does not know the dough is made the day before, and where the
	timing matters the cook writes a Step saying so.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { numbering } from './divergence';
	import { pathKey, type Component } from './unfolding.svelte';

	let { component }: { component: Component } = $props();
</script>

{#if component.content}
	{@const number = numbering()}
	<div id="annexe-{pathKey(component.path)}" class="mt-8 scroll-mt-12">
		<h2 class="mx-gutter mb-1 font-display text-label font-semibold text-support-2 uppercase">
			{component.title} · {m.recipe_component_method()}
		</h2>
		<p class="mx-gutter mb-2 text-read text-ink-2">{component.said}</p>
		<ol class="mx-gutter border-l-2 border-support-2 bg-ground-2 py-1 pl-3">
			{#each component.content.steps as item, index (index)}
				{#if item.kind === 'section'}
					<li class="border-b border-rule py-4 pb-1">
						<h3 class="font-display text-label text-ink-2 uppercase">{item.text}</h3>
					</li>
				{:else}
					<li class="flex gap-3 border-b border-rule py-3 last:border-b-0">
						<span class="w-6 shrink-0 font-display text-line font-semibold text-accent">
							{number(false)}
						</span>
						<p class="min-w-0 flex-1 text-body">{item.text}</p>
					</li>
				{/if}
			{/each}
		</ol>
	</div>
{/if}
