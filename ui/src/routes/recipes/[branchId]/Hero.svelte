<!--
	What leads a recipe: its Main Photo, or — for the roughly one recipe in
	three that has none — its Cover (#46). A Cover is not a placeholder for a
	missing picture; it is what a recipe without one wears.

	THE LAYOUT IS #81'S, and it is here rather than in a screen because there
	are now two screens that draw it: the recipe as it reads, and the recipe
	being written on (#83). Two renderings of one hero is exactly the drift
	that put the wash in `app.css` in the first place — its fitted height and
	stops are recorded beside it there, and that reasoning only holds while
	there is one copy of the thing it applies to.

	What stands ON the hero differs between the two and is the caller's: a
	title and its Source line on the reading page, the title as a field on the
	writing one. The wash exists so that whatever it is can be read over a
	photograph whose colours nobody chose; a Cover never wears it, because its
	dye was chosen (#46, #81).
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import Cover from '$lib/cover/Cover.svelte';

	interface Props {
		lineageId: string;
		/** The Cover's own word, where there is no photograph to lead with. */
		title: string;
		photo: string | null;
		/** What stands on it, at the hem. */
		over: Snippet;
	}

	let { lineageId, title, photo, over }: Props = $props();
</script>

<div class="relative overflow-hidden">
	{#if photo}
		<img
			src="/api/photographs/{photo}/page"
			alt=""
			class="block w-full object-cover"
			style="height: var(--hero-h)"
		/>
		<!-- The wash. A photograph's colours are nobody's choice, so what sits
		     on it is given ground of its own rather than hoping. -->
		<div class="pointer-events-none absolute inset-x-0 bottom-0 wash"></div>
	{:else}
		<Cover {lineageId} {title} band={false} />
	{/if}
	<div class="absolute inset-x-0 bottom-0 px-gutter pt-8 pb-4">
		{@render over()}
	</div>
</div>
