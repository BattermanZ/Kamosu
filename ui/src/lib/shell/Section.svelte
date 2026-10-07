<!--
	A labelled block within a screen: an uppercase heading over a rule, and what
	it introduces. One component so every section on every screen is the same
	distance from the thing above it.

	Anything else it is given goes onto the section itself, which is how one
	that takes a dropped file listens for it (#205).
-->
<script lang="ts">
	import type { Snippet } from 'svelte';

	import type { HTMLAttributes } from 'svelte/elements';

	interface Props extends HTMLAttributes<HTMLElement> {
		heading: string;
		children: Snippet;
	}

	let { heading, children, class: more, ...rest }: Props = $props();
</script>

<section {...rest} class={['mt-8', more]}>
	<h2 class="mb-3 border-b border-rule pb-2 text-label font-medium text-accent uppercase">
		{heading}
	</h2>
	{@render children()}
</section>
