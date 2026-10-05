<!--
	`/` — **Home** to a Person, and the account form to anyone who is not one yet.

	The tab bar has pointed Home at `/` since the shell was built, and until now
	`/` was the login form. This is the other side of signing in (#64).

	**How it knows: it asks.** There is no cookie read here and no second idea of
	who is signed in kept beside the Core's — the screen asks for its shelves,
	and a refusal *is* the answer that nobody is signed in. One round trip does
	both jobs, so a signed-in Person never sees the login form flash first.

	An Invite or a recovery link opens a route of its own, not this one (#126).
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { GetCookbookOutput, HomeShelvesOutput } from '$lib/api/catalogue';
	import { refreshed } from '$lib/offline/device.svelte';
	import Account from './Account.svelte';
	import Home from './Home.svelte';
	import CookbookJoins from '$lib/CookbookJoins.svelte';
	import { heardWhetherSignedIn } from '$lib/shell/signing-in.svelte';

	const kamosu = useKamosu();

	/** The shelves once they arrive; undefined while asking; null once refused. */
	let home = $state<HomeShelvesOutput | null | undefined>(undefined);
	let failed = $state(false);
	/** Bumped when the form reports a sign-in, which re-runs the ask below. */
	let asked = $state(0);

	/**
	 * The reader's Cookbook, asked for after the shelves, for one thing only: a
	 * join waiting on their answer is asked on Home, the screen they open every
	 * time, since nothing else would tell them (#135, answer 2).
	 */
	let cookbook = $state<GetCookbookOutput | undefined>(undefined);

	/** The form says it worked: forget the refusal and ask again. */
	function signedIn() {
		home = undefined;
		asked += 1;
	}

	$effect(() => {
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
				// The shell draws its sidebar on this answer (#194).
				heardWhetherSignedIn(true);
				if (current) {
					home = shelves;
					failed = false;
				}
				// Only ever an addition to Home: a Cookbook that cannot be read
				// just now, offline say, leaves the shelves standing without it.
				return kamosu.getCookbook().then(
					(answer) => {
						if (current) cookbook = answer;
					},
					(refused: unknown) => {
						if (!(refused instanceof OperationError)) throw refused;
					},
				);
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				if (!current) return;
				// Refused for want of a Credential is not a failure — it is the
				// answer, and the answer is the login form. Anything else is a
				// Kamosu that could not be reached, which says so instead.
				if (error.kind === 'unauthorized') {
					heardWhetherSignedIn(false);
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

{#snippet asking()}
	{#if cookbook}
		<CookbookJoins
			{cookbook}
			onlyAsked
			onAnswered={(answered) => {
				cookbook = answered;
				// A join that went ahead puts more recipes on the shelves.
				asked += 1;
			}}
		/>
	{/if}
{/snippet}

{#if home === null}
	<Account onSignedIn={signedIn} />
{:else if home}
	<Home {home} asking={cookbook?.joins.some((join) => join.you === 'asked') ? asking : undefined} />
{:else if failed}
	<div class="mx-auto max-w-2xl px-gutter pt-6">
		<p class="text-body text-support" role="alert">{m.home_failed()}</p>
	</div>
{:else}
	<div class="mx-auto max-w-2xl px-gutter pt-6">
		<p class="text-body text-ink-2">{m.loading()}</p>
	</div>
{/if}
