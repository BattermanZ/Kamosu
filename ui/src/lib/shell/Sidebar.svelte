<!--
	The sidebar: what the wide layout draws in place of the header and the tab
	bar (ADR 0044, #194).

	A narrow indigo rail down the left edge, chosen by Aurélien on 5 October 2026
	over a wider paper sidebar and over keeping the header: it gives the page a
	band of the app's own colour, and at 88 wide it leaves an upright tablet
	almost all of its width. The places are drawn as the tab bar draws them, an
	icon over its uppercase label, so the two layouts read as one app.

	Kamosu's name is written under the mark. The first prototype drew the mark
	alone and his one objection was that the name stood nowhere on the screen.

	Settings is the fifth place, at the foot. The phone reaches it through the
	*You* card in the header (#102); with no header here there is no card, and
	the rail is where a wide screen looks for it.

	Indigo is the cooking screen's colour too, so the quieter ink, the darker
	ground of the current place and the rule round the mark are that room's
	tokens rather than new ones.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { isCurrent, places, settings, type Place } from './places';

	interface Props {
		/** The address on screen, which says which place is the current one. */
		pathname: string;
	}

	let { pathname }: Props = $props();
</script>

{#snippet link(place: Place)}
	{@const current = isCurrent(place.href, pathname)}
	<li>
		<a
			href={place.href}
			aria-current={current ? 'page' : undefined}
			class="flex min-h-[68px] flex-col items-center justify-center gap-1 border-l-3 text-label uppercase
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
	</li>
{/snippet}

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
			{@render link(place)}
		{/each}
	</ul>

	<ul>
		{@render link(settings)}
	</ul>
</nav>
