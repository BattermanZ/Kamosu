<script lang="ts">
	import { onNavigate } from '$app/navigation';
	import { page } from '$app/state';
	import '../app.css';
	import { m } from '$lib/paraglide/messages';
	import { getLocale } from '$lib/paraglide/runtime';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import TabBar from '$lib/shell/TabBar.svelte';
	import { realKamosu } from '$lib/kamosu';
	import { realAuth } from '$lib/auth';
	import { readToken } from '$lib/tokens';

	let { children } = $props();

	const client = realKamosu();
	const auth = realAuth();

	// The document's language is the locale Paraglide resolved, which is what
	// tells a screen reader — and Safari's translation offer — what it is reading.
	$effect(() => {
		document.documentElement.lang = getLocale();
	});

	// The browser chrome takes the app's ground colour — read from the stylesheet
	// the page loaded, so it is the same colour every screen is drawn on and
	// there is no second copy of it to fall behind.
	$effect(() => {
		document
			.querySelector('meta[name="theme-color"]')
			?.setAttribute('content', readToken('--color-ground'));
	});

	// The signature transition (ADR 0012). Everything about how it *looks* is CSS
	// in app.css; this is the one line of script the API needs, and a browser
	// without it simply navigates.
	onNavigate((navigation) => {
		if (!document.startViewTransition) return;
		return new Promise((resolve) => {
			document.startViewTransition(async () => {
				resolve();
				await navigation.complete;
			});
		});
	});

	const onSettings = $derived(page.url.pathname.startsWith('/settings'));

	/**
	 * The cooking screen wears no shell (ADR 0011): it is one Step filling the
	 * phone, used at arm's length with wet hands. The header would cost it the
	 * height, and the tab bar would put Shopping one wet thumb away from the step
	 * you are on — a tap the cook did not intend, which is the one thing this
	 * screen is not allowed to cost.
	 */
	const cooking = $derived(page.url.pathname.startsWith('/cook/'));
</script>

<Kamosu {client} {auth}>
	{#if !cooking}
		<header class="sticky top-0 z-10 border-b border-rule bg-ground pt-safe">
			<div class="mx-auto flex max-w-2xl px-gutter py-3">
				<!-- The Kitchen's card. Settings live behind the Kitchen's name — the
			     name is the way in, which is what leaves the tab bar its four
			     sections and nothing else.

			     It is also the shell's expanding pair: this card and the Settings
			     screen's title share `view-transition-name: kitchen`, so the card
			     grows into the page (ADR 0012). Every recipe card that opens into
			     its page later is the same pair under a different name. -->
				<a
					href="/settings"
					aria-current={onSettings ? 'page' : undefined}
					class="flex min-h-12 items-center gap-3 rounded-sm border px-3 py-2 font-display text-title font-semibold
					{onSettings ? 'border-accent bg-card text-accent' : 'border-rule bg-card text-ink'}"
					style={onSettings
						? undefined
						: 'view-transition-name: kitchen; view-transition-class: expanding'}
				>
					<img src="/assets/img/kamosu-mark.svg" alt="" class="h-8 w-8 rounded-sm" />
					{m.app_name()}
					<svg
						viewBox="0 0 24 24"
						aria-hidden="true"
						class="h-4 w-4 text-ink-2"
						fill="none"
						stroke="currentColor"
						stroke-width="1.75"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<path d="m9 5 7 7-7 7" />
					</svg>
					<span class="sr-only">— {m.open_settings()}</span>
				</a>
			</div>
		</header>
	{/if}

	<main>
		{@render children()}
	</main>

	{#if !cooking}
		<TabBar />
	{/if}
</Kamosu>
