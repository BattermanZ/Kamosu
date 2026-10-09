<!--
	The sidebar: what the wide layout draws in place of the header and the tab
	bar (ADR 0044, #194).

	A narrow indigo rail down the left edge, chosen on 5 October 2026
	over a wider paper sidebar and over keeping the header: it gives the page a
	band of the app's own colour, and at 88 wide it leaves an upright tablet
	almost all of its width. The places are drawn by `PlaceLink.svelte`, which
	the phone's bar draws them with too since #218, so the two layouts read as
	one app.

	Kamosu's name is written under the mark. The first prototype drew the mark
	alone and his one objection was that the name stood nowhere on the screen.

	Settings is the fifth place, at the foot. The phone reaches it through the
	gear in the header (#102, #218); with no header here there is no gear, and
	the rail is where a wide screen looks for it.

	Indigo is the cooking screen's colour too, so the quieter ink, the darker
	ground of the current place and the rule round the mark are that room's
	tokens rather than new ones.

	What a place looks like under the pointer is in `app.css` with every other
	control's (#203): its words turn to the paper's colour over a faint patch,
	and the current place stays as it is.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import PlaceLink from './PlaceLink.svelte';
	import { isCurrent, places, settings } from './places';

	interface Props {
		/** The address on screen, which says which place is the current one. */
		pathname: string;
	}

	let { pathname }: Props = $props();
</script>

<nav
	aria-label={m.nav_sections()}
	class="fixed inset-y-0 left-0 z-20 flex w-rail flex-col overflow-y-auto bg-accent pt-safe pb-safe"
>
	<p
		class="flex flex-col items-center gap-2 pt-3 pb-4 font-display text-tile-title font-semibold text-on-accent"
	>
		<img
			src="/assets/img/kamosu-mark.svg"
			alt=""
			class="h-[40px] w-[40px] rounded-sm border border-cook-rule"
		/>
		{m.app_name()}
	</p>

	<ul class="flex-1">
		{#each places as place (place.href)}
			<li><PlaceLink {place} current={isCurrent(place.href, pathname)} edge="left" /></li>
		{/each}
	</ul>

	<ul>
		<li><PlaceLink place={settings} current={isCurrent(settings.href, pathname)} edge="left" /></li>
	</ul>
</nav>
