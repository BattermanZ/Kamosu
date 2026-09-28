<!--
	A labelled block within a screen: an uppercase heading over a rule, and what
	it introduces. One component so every section on every screen is the same
	distance from the thing above it.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		heading: string;
		children: Snippet;
	}

	let { heading, children }: Props = $props();
</script>

<section class="mt-8">
	<h2 class="mb-3 border-b border-rule pb-2 text-label font-medium text-accent uppercase">
		{heading}
	</h2>
	{@render children()}
</section>
