<!--
	One place of Kamosu's navigation, drawn the one way both navigations draw
	it (#218): an icon over its uppercase word in the rail's quiet ink, and the
	current place in kinari on the darker ground with a 3px kinari line along
	one edge. The rail (#194) draws the line down the left edge; the phone's
	bar, which took the rail's look on #218, draws it along the top.

	Written once so the two layouts cannot drift: the audit of 9 October 2026
	found the bar still paper with indigo for the current place, the one
	thing on screen that said the phone and the tablet were two apps.
-->
<script lang="ts">
	import type { Place } from './places';

	interface Props {
		place: Place;
		current: boolean;
		/** Which edge the current place's line runs along: the rail's left, the bar's top. */
		edge: 'left' | 'top';
	}

	let { place, current, edge }: Props = $props();
</script>

<a
	href={place.href}
	aria-current={current ? 'page' : undefined}
	class="flex flex-col items-center justify-center gap-1 text-label uppercase
		{edge === 'left' ? 'min-h-[68px] border-l-3' : 'min-h-[56px] border-t-3 pt-[6px] pb-1'}
		{current ? 'border-on-accent bg-cook-ground text-on-accent' : 'border-transparent text-cook-ink-2'}"
>
	<svg
		viewBox="0 0 24 24"
		aria-hidden="true"
		class="h-6 w-6"
		fill="none"
		stroke="currentColor"
		stroke-width={current ? 2 : 1.5}
		stroke-linecap="round"
		stroke-linejoin="round"
	>
		<path d={place.path} />
	</svg>
	{place.label()}
</a>
