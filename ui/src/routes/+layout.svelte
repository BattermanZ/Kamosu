<script lang="ts">
	import { afterNavigate, onNavigate } from '$app/navigation';
	import { page } from '$app/state';
	import '../app.css';
	import { getLocale } from '$lib/paraglide/runtime';
	import Header from '$lib/shell/Header.svelte';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import TabBar from '$lib/shell/TabBar.svelte';
	import { realKamosu } from '$lib/kamosu';
	import { realAuth } from '$lib/auth';
	import { realPhotographUpload, realUpload } from '$lib/api/upload';
	import { realFiles } from '$lib/api/files';
	import { readToken } from '$lib/tokens';
	import Notices from '$lib/offline/Notices.svelte';
	import WentWrong from '$lib/WentWrong.svelte';
	import { watchForMistakes, wentWrong } from '$lib/mistake.svelte';
	import Arrived from '$lib/Arrived.svelte';
	import { listenToTheWorker, reach, retryWhileUnreachable } from '$lib/offline/device.svelte';
	import { realOutbox } from '$lib/offline/outbox';
	import { m } from '$lib/paraglide/messages';
	import { story } from '$lib/story/showing.svelte';
	import { WindowRoom } from '$lib/room.svelte';

	let { children } = $props();

	const outbox = realOutbox();
	const client = realKamosu(outbox);
	const auth = realAuth();
	const upload = realUpload();
	const photograph = realPhotographUpload();
	const files = realFiles();

	// How much room the window has (#193), read back from the stylesheet and
	// followed as the window is resized or a tablet turned.
	const room = new WindowRoom();

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

	// The service worker says when a read it answered from the phone has since
	// changed on the server, and whether the server is being reached at all
	// (#76): the screens that show a changed read read again, and "offline"
	// means the server is out of reach, not only that the browser thinks so.
	$effect(() => listenToTheWorker());
	$effect(() => retryWhileUnreachable(client));

	// The last of the three nets under a mistake in Kamosu's own code (#98): the
	// client catches what an Operation call throws, the boundary below catches
	// what rendering throws, and this catches the rest — everything a screen
	// throws after an await, which Svelte awaits neither for an event handler nor
	// for a fire-and-forget `$effect`.
	$effect(() => watchForMistakes());

	// What was written with no network goes as soon as the server answers
	// again (#77): on opening, whenever the server is found again, and when the
	// browser says the network is back.
	$effect(() => {
		if (reach.server) void outbox.flush();
	});
	$effect(() => {
		const again = () => void outbox.flush();
		addEventListener('online', again);
		return () => removeEventListener('online', again);
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

	/**
	 * The story (#158) covers the whole window with corners of its own, so it
	 * wears no shell either: a header and tab bar drawn under it would still be
	 * read out and tabbed through behind a page that hides them.
	 */
	const bare = $derived(cooking || story.showing);

	/**
	 * The boundary below has taken the screen down (#119). A handled boundary
	 * drops its content and does not draw it again by itself, even for the next
	 * route, so it is redrawn once the cook has gone somewhere else: without
	 * that, the way back offered on the cooking screen would lead to a page as
	 * blank as the one it left.
	 */
	let redrawScreen = $state<(() => void) | undefined>(undefined);
	function screenWentWrong(error: unknown, reset: () => void) {
		redrawScreen = reset;
		wentWrong(error);
	}
	afterNavigate(() => {
		const redraw = redrawScreen;
		redrawScreen = undefined;
		redraw?.();
	});
</script>

<Kamosu {client} {auth} {upload} {photograph} {files} keeping={outbox} room={room.current}>
	{#if !bare}
		<Header {onSettings} />
	{/if}

	<main>
		<!-- Not under the story either (#158): it covers the window, and a card
		     drawn beneath it would be read out before the story it hides behind. -->
		{#if !bare}
			<!-- Kamosu itself went wrong (#98), above the rest, because it outranks
			     every "not right now" card: those say what cannot be done, and this
			     says the thing you just did went wrong.

			     Not on the cooking screen, for the same reason as everything else
			     here (ADR 0011) — and, drawn there, not even visible: that screen is
			     `fixed inset-0 z-30`, so a card in ordinary flow is painted
			     underneath it, unseen by the cook while a screen reader still
			     announces the alert. The cooking screen says it in two words of its
			     own instead, on the row it already has, and opens this card only on
			     a tap (#119, option B — Aurélien, 23 September 2026). The mistake is
			     remembered either way, and the card is waiting here the moment the
			     cook leaves the step. -->
			<WentWrong />

			<!-- What Kamosu cannot do right now, said once at the top (#76). Not on
			     the cooking screen, which carries nothing but the Step. -->
			<Notices />
			<!-- And what bringing a recipe file in just said, above the recipe it
			     brought (#93). Said here rather than inside the recipe screen
			     because this is where Kamosu says a thing once and it is put
			     away — the same place, and the same card, as the rest. -->
			<Arrived pathname={page.url.pathname} />
		{:else if cooking && redrawScreen}
			<!-- The cooking screen itself went wrong while it was being drawn, and
			     the boundary took it away (#119). There is no Step left to protect,
			     so #98's card is drawn after all, on the bare page the boundary
			     left, with the way back the cooking screen would have offered. -->
			<WentWrong />
			<a
				href="/recipes/{page.params.branchId}"
				class="mx-gutter block min-h-12 rounded-sm bg-accent px-4 py-3 text-center font-semibold text-on-accent"
			>
				{m.cook_back_to_recipe()}
			</a>
		{/if}

		<!--
			The screen itself, walled off (#98). A mistake made while rendering, or
			inside an effect, reaches no promise at all — a boundary is the only thing
			that sees it, and without one it tears down the whole app, the card
			included, so the one surface meant to report the mistake would go with it.

			The boundary sits INSIDE main and the card outside it, which is what keeps
			that from happening: the screen is what is walled off, and the card is
			what survives to say so. A handled boundary drops its content, so the
			half-drawn screen goes rather than sitting there looking like it is still
			loading — which is the whole complaint this issue was filed about.
		-->
		<svelte:boundary onerror={screenWentWrong}>
			{@render children()}
		</svelte:boundary>
	</main>

	{#if !bare}
		<TabBar />
	{/if}
</Kamosu>
