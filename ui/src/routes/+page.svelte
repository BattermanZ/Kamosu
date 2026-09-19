<!--
	`/` — **Home** to a Person, and the account form to anyone who is not one yet.

	The tab bar has pointed Home at `/` since the shell was built, and until now
	`/` was the login form. This is the other side of signing in (#64).

	**How it knows: it asks.** There is no cookie read here and no second idea of
	who is signed in kept beside the Core's — the screen asks for its shelves,
	and a refusal *is* the answer that nobody is signed in. One round trip does
	both jobs, so a signed-in Person never sees the login form flash first.

	An Invite or a recovery link goes straight to the form without asking, since
	the whole point of one is that you are not signed in yet — and asking would
	spend a request to be told what the address already says.
-->
<script lang="ts">
	import { page } from '$app/state';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { HomeShelvesOutput } from '$lib/api/catalogue';
	import { refreshed } from '$lib/offline/device.svelte';
	import Account from './Account.svelte';
	import Home from './Home.svelte';

	const kamosu = useKamosu();

	/** The shelves once they arrive; undefined while asking; null once refused. */
	let home = $state<HomeShelvesOutput | null | undefined>(undefined);
	let failed = $state(false);
	/** Bumped when the form reports a sign-in, which re-runs the ask below. */
	let asked = $state(0);

	const arriving = $derived(
		page.url.pathname.startsWith('/invite/') || page.url.pathname.startsWith('/recover/'),
	);

	/** The form says it worked: forget the refusal and ask again. */
	function signedIn() {
		home = undefined;
		asked += 1;
	}

	$effect(() => {
		if (arriving) return;
		// Read before the await, so signing in re-runs this rather than leaving
		// the form standing in front of a Person who is now signed in.
		void asked;
		// The phone answered first and the server has since answered otherwise —
		// new shelves, or a Session that has ended (#76). Ask again.
		void refreshed.get('home_shelves');

		let current = true;
		kamosu
			.homeShelves({})
			.then((shelves) => {
				if (current) {
					home = shelves;
					failed = false;
				}
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				if (!current) return;
				// Refused for want of a Credential is not a failure — it is the
				// answer, and the answer is the login form. Anything else is a
				// Kamosu that could not be reached, which says so instead.
				if (error.kind === 'unauthorized') {
					home = null;
				} else {
					failed = true;
				}
			});
		return () => {
			current = false;
		};
	});
</script>

{#if arriving || home === null}
	<Account onSignedIn={signedIn} />
{:else if home}
	<Home {home} />
{:else if failed}
	<div class="mx-auto max-w-2xl px-gutter pt-6">
		<p class="text-body text-support" role="alert">{m.home_failed()}</p>
	</div>
{:else}
	<div class="mx-auto max-w-2xl px-gutter pt-6">
		<p class="text-body text-ink-2">{m.loading()}</p>
	</div>
{/if}
