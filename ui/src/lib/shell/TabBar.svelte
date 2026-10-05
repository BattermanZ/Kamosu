<!--
	The tab bar: Home · Recipes · Shopping · Cooked.

	Four sections, fixed to the bottom of the screen, sized for a thumb at arm's
	length with wet hands. Settings are not here — they live behind the *You*
	card in the header (#102), because they are not somewhere you go while
	cooking. A fifth tab would cost the four that matter for a room nobody
	reaches mid-recipe (ADR 0011).
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { isCurrent, places } from './places';

	interface Props {
		/** The address on screen, which says which place is the current one. */
		pathname: string;
	}

	let { pathname }: Props = $props();
</script>

<nav
	aria-label={m.nav_sections()}
	class="fixed inset-x-0 bottom-0 z-20 border-t border-rule bg-card pb-safe"
>
	<ul class="mx-auto flex max-w-2xl">
		{#each places as place (place.href)}
			{@const current = isCurrent(place.href, pathname)}
			<li class="flex-1">
				<a
					href={place.href}
					aria-current={current ? 'page' : undefined}
					class="flex min-h-12 flex-col items-center justify-center gap-1 pt-2 text-label uppercase
						{current ? 'text-accent' : 'text-ink-2'}"
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
		{/each}
	</ul>
</nav>
