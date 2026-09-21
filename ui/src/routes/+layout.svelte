<script lang="ts">
	import { onNavigate } from '$app/navigation';
	import { page } from '$app/state';
	import '../app.css';
	import { getLocale } from '$lib/paraglide/runtime';
	import Header from '$lib/shell/Header.svelte';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import TabBar from '$lib/shell/TabBar.svelte';
	import { realKamosu } from '$lib/kamosu';
	import { realAuth } from '$lib/auth';
	import { realPhotographUpload, realUpload } from '$lib/api/upload';
	import { readToken } from '$lib/tokens';
	import Notices from '$lib/offline/Notices.svelte';
	import Arrived from '$lib/Arrived.svelte';
	import { listenToTheWorker, reach, retryWhileUnreachable } from '$lib/offline/device.svelte';
	import { realOutbox } from '$lib/offline/outbox';

	let { children } = $props();

	const outbox = realOutbox();
	const client = realKamosu(outbox);
	const auth = realAuth();
	const upload = realUpload();
	const photograph = realPhotographUpload();

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
</script>

<Kamosu {client} {auth} {upload} {photograph} keeping={outbox}>
	{#if !cooking}
		<Header {onSettings} />
	{/if}

	<main>
		<!-- What Kamosu cannot do right now, said once at the top (#76). Not on
		     the cooking screen, which carries nothing but the Step. -->
		{#if !cooking}
			<Notices />
			<!-- And what bringing a recipe file in just said, above the recipe it
			     brought (#93). Said here rather than inside the recipe screen
			     because this is where Kamosu says a thing once and it is put
			     away — the same place, and the same card, as the rest. -->
			<Arrived pathname={page.url.pathname} />
		{/if}
		{@render children()}
	</main>

	{#if !cooking}
		<TabBar />
	{/if}
</Kamosu>
