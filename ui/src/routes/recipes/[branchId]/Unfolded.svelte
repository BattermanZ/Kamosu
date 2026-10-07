<!--
	A COMPONENT UNFOLDED: the inner recipe's own Ingredient Lines, indented
	under the row that names them behind a matcha rule, on the recessed ground
	— visibly another recipe's inside without being a card.

	Its Steps are NOT here. They are set at the foot of the page by `Annexe`,
	which is the treatment chosen (#50), and it is what keeps this a
	list rather than a method with a shopping list around it.

	The amounts beneath each line are already scaled by how much of that recipe
	this line asks for and converted to this reader's measures — both worked
	out in the Core, by the same code that words every other Ingredient Line's
	slot. A Component inside a Component nests here too; `at` finds it by its
	path.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import Unfolded from './Unfolded.svelte';
	import { pathKey, type Component, type Unfolding } from './unfolding.svelte';

	interface Props {
		component: Component;
		/** Finds a Component inside this one by its path. */
		at: Unfolding['at'];
	}

	let { component, at }: Props = $props();
</script>

{#if component.content}
	<ul class="mt-3 border-l-2 border-support-2 bg-ground-2 py-1 pl-3">
		{#each component.content.ingredients as item, index (index)}
			{@const within = at(component.path, index)}
			{#if item.kind === 'section'}
				<li class="border-b border-rule py-3 pb-1 last:border-b-0">
					<h4 class="font-display text-label text-ink-2 uppercase">{item.text}</h4>
				</li>
			{:else}
				<li class="flex gap-3 border-b border-rule py-3 last:border-b-0">
					<span
						class="ingredient-marker shrink-0 {within ? 'bg-support-2' : 'bg-accent'}"
						aria-hidden="true"
					></span>
					<div class="min-w-0 flex-1">
						<span class="block text-line">{item.text}</span>
						{#if within}
							<span class="block text-read text-support-2">{within.said}</span>
						{:else if component.measured?.ingredients[index]}
							<span class="block text-read text-ink-2">
								{component.measured.ingredients[index]}
							</span>
						{/if}
						{#if within}
							<Unfolded component={within} {at} />
						{/if}
					</div>
				</li>
			{/if}
		{/each}
	</ul>
	<!--
		WHERE THE METHOD WENT. Under treatment B a Component is in two places,
		and the second one is a long way down the page — so the row says where,
		and the saying is a link that takes you there. Absent for a Component
		with no Steps: there is nothing at the foot to point at.
	-->
	{#if component.content.steps.length}
		<a
			href="#annexe-{pathKey(component.path)}"
			class="mt-2 block text-read text-support-2 underline underline-offset-2"
		>
			{m.recipe_component_method_below()}
		</a>
	{/if}
{/if}
