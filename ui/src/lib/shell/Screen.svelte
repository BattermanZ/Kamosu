<!--
	One screen: its title, a line saying what it is for, and whatever it holds.

	Every section reads the same way because they are all drawn by this, which is
	also what keeps the gutter and the room above the tab bar in one place.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		title: string;
		blurb?: string;
		/**
		 * Names this screen's title as the other half of an expanding pair, so a
		 * card elsewhere grows into it rather than the screen swapping (ADR 0012).
		 */
		expandsFrom?: string;
		/**
		 * Drawn as a card, where the screen is what is open beside a list on
		 * the wide layout (#199).
		 */
		card?: boolean;
		children?: Snippet;
	}

	let { title, blurb, expandsFrom, card = false, children }: Props = $props();
</script>

<div class={card ? 'open-card' : 'mx-auto max-w-2xl px-gutter pt-6 pb-tabbar'}>
	<h1
		class="font-display text-title font-semibold"
		style={expandsFrom
			? `view-transition-name: ${expandsFrom}; view-transition-class: expanding`
			: undefined}
	>
		{title}
	</h1>
	{#if blurb}
		<p class="mt-1 text-read text-ink-2">{blurb}</p>
	{/if}

	{#if children}
		<div class="mt-6">{@render children()}</div>
	{/if}
</div>
