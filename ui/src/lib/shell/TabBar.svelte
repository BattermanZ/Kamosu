<!--
	The tab bar: Home · Recipes · Shopping · Cooked.

	Four sections, fixed to the bottom of the screen, sized for a thumb at arm's
	length with wet hands. Settings are not here — they live behind the *You*
	card in the header (#102), because they are not somewhere you go while
	cooking. A fifth tab would cost the four that matter for a room nobody
	reaches mid-recipe (ADR 0011).
-->
<script lang="ts">
	import { page } from '$app/state';
	import { m } from '$lib/paraglide/messages';

	interface Section {
		href: string;
		label: () => string;
		/** Drawn at 24px in the current colour; one path each, no icon library. */
		path: string;
	}

	const sections: Section[] = [
		{
			href: '/',
			label: () => m.nav_home(),
			path: 'M3 10.5 12 3l9 7.5M5.5 9.5V20h13V9.5',
		},
		{
			href: '/recipes',
			label: () => m.nav_recipes(),
			path: 'M4 4h11a2 2 0 0 1 2 2v14H6a2 2 0 0 1-2-2V4Zm13 0h3v16M8 8h6M8 12h6',
		},
		{
			href: '/shopping',
			label: () => m.nav_shopping(),
			path: 'M4 6h3l2 11h9l2-8H8M10 21h.01M17 21h.01',
		},
		{
			href: '/cooked',
			label: () => m.nav_cooked(),
			path: 'M4 15h16a4 4 0 0 1-4 4H8a4 4 0 0 1-4-4ZM7 11c0-2 1.5-2 1.5-4M12 11c0-2 1.5-2 1.5-4M17 11c0-2 1.5-2 1.5-4',
		},
	];

	/** The section a path belongs to: `/recipes/soba` is still Recipes. */
	function isCurrent(href: string, pathname: string): boolean {
		return href === '/' ? pathname === '/' : pathname === href || pathname.startsWith(`${href}/`);
	}
</script>

<nav
	aria-label={m.nav_sections()}
	class="fixed inset-x-0 bottom-0 z-20 border-t border-rule bg-card pb-safe"
>
	<ul class="mx-auto flex max-w-2xl">
		{#each sections as section (section.href)}
			{@const current = isCurrent(section.href, page.url.pathname)}
			<li class="flex-1">
				<a
					href={section.href}
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
						<path d={section.path} />
					</svg>
					{section.label()}
				</a>
			</li>
		{/each}
	</ul>
</nav>
