<!--
	The tab bar: Home · Recipes · Shopping · Cooked.

	Four sections, fixed to the bottom of the screen, sized for a thumb at arm's
	length with wet hands. Settings are not here — they live behind the gear in
	the header (#102, #218), because they are not somewhere you go while
	cooking. A fifth tab would cost the four that matter for a room nobody
	reaches mid-recipe (ADR 0011).

	Since #218 it wears the rail's look (#194): indigo, each place an icon over
	its word, the current one lit. `PlaceLink.svelte` draws a place for both,
	so the phone and the tablet read as one app at the bottom of the screen.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import PlaceLink from './PlaceLink.svelte';
	import { isCurrent, places } from './places';

	interface Props {
		/** The address on screen, which says which place is the current one. */
		pathname: string;
	}

	let { pathname }: Props = $props();
</script>

<nav aria-label={m.nav_sections()} class="fixed inset-x-0 bottom-0 z-20 bg-accent pb-safe">
	<ul class="mx-auto flex max-w-2xl">
		{#each places as place (place.href)}
			<li class="flex flex-1 flex-col">
				<PlaceLink {place} current={isCurrent(place.href, pathname)} edge="top" />
			</li>
		{/each}
	</ul>
</nav>
