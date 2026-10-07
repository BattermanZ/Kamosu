<!--
	`/cookbook-invite/<secret>`: the page a Cookbook Invite link opens (#131,
	screen choice 2 of 24 September 2026).

	It says what saying yes does before anything happens: who sent it (one
	Person, where their Cookbook may have several writers),
	how many recipes on each side become one, and that leaving is always
	possible with a copy of everything. Opening the link changes nothing. Only
	*Write together* does, and *Not now* leaves the link unspent for later.

	Where either Cookbook already has other writers, it names them first:
	each will be asked, and nothing joins until they all say yes (#135). Once
	accepted, the same link says who it still waits for.

	A Cookbook Invite is for somebody who already has an account; making one is
	a Kitchen Invite's job. So a reader who is not signed in gets the ordinary
	sign-in form here, and the same question once they are.

	The secret arrives as the `params` prop, as on `/invite/<secret>`, so a test
	can render this route itself with a secret of its choosing.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { joinedNames } from '$lib/cookbook';
	import type { ReadCookbookInviteOutput } from '$lib/api/catalogue';
	import Account from '../../Account.svelte';
	import type { PageProps } from './$types';

	let { params }: PageProps = $props();

	const kamosu = useKamosu();

	/**
	 * What the link would do; undefined while asking; `'sign-in'` when there
	 * is nobody to ask for; `'gone'` for a link used, ended or never minted.
	 */
	let invite = $state<ReadCookbookInviteOutput | 'sign-in' | 'gone' | undefined>(undefined);
	let failed = $state<string | undefined>(undefined);
	let busy = $state(false);
	/** Bumped when the sign-in form succeeds, which asks again. */
	let asked = $state(0);

	$effect(() => {
		void asked;
		const secret = params.secret;
		let current = true;
		invite = undefined;
		kamosu
			.readCookbookInvite({ secret })
			.then((answer) => {
				if (current) invite = answer;
			})
			.catch(async (error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				if (!current) return;
				if (error.kind !== 'unauthorized') {
					invite = 'gone';
					return;
				}
				// The Core refuses a used, ended or unknown secret exactly as it
				// refuses a reader with no Session, so nothing about a secret
				// leaks from the refusal. Whether you are signed in is asked
				// separately: signed in, the link is the thing that is gone.
				const signedIn = await kamosu.getCookbook().then(
					() => true,
					(refused: unknown) => {
						if (!(refused instanceof OperationError)) throw refused;
						return false;
					},
				);
				if (current) invite = signedIn ? 'gone' : 'sign-in';
			});
		return () => {
			current = false;
		};
	});

	async function accept() {
		busy = true;
		failed = undefined;
		try {
			const answer = await kamosu.acceptCookbookInvite({ secret: params.secret });
			if (answer.joins.some((join) => join.state === 'waiting' && join.you === 'accepted')) {
				// Waiting on the others: read it again, which says who.
				asked += 1;
				return;
			}
			// Settings, where the joined Cookbook's card now names you both.
			await goto('/settings');
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		} finally {
			busy = false;
		}
	}
</script>

{#if invite === 'sign-in'}
	<Account onSignedIn={() => (asked += 1)} />
{:else}
	<div class="mx-auto max-w-2xl px-gutter pt-6 pb-8">
		{#if invite === undefined}
			<p class="text-body text-ink-2">{m.loading()}</p>
		{:else if invite === 'gone'}
			<p class="text-body text-ink" role="alert">{m.cookbook_invite_gone()}</p>
			<a class="mt-4 inline-block text-label text-accent underline" href="/">{m.home_title()}</a>
		{:else if invite.already_yours}
			<p class="text-body text-ink">{m.cookbook_invite_already()}</p>
			<a class="mt-4 inline-block text-label text-accent underline" href="/settings"
				>{m.settings_title()}</a
			>
		{:else}
			{@const names = invite.invited_by.name}
			<p class="text-label text-ink-2 uppercase">{m.cookbook_invite_kicker()}</p>
			<h1 class="mt-1 font-display text-title font-semibold text-ink">
				{m.cookbook_invite_title({ names })}
			</h1>
			{@const counts = {
				yours: invite.your_recipes,
				theirs: invite.their_recipes,
				total: invite.together_recipes,
			}}
			{@const others = joinedNames(invite.asks.map((person) => person.name))}
			<p class="mt-3 text-body text-ink">
				{invite.asks.length > 0
					? m.cookbook_invite_join_said_all(counts)
					: m.cookbook_invite_join_said(counts)}
			</p>
			{#if invite.waiting}
				<p class="mt-3 text-body text-ink" role="status">
					{m.cookbook_invite_waiting_on({ names: others })}
				</p>
				<a class="mt-4 inline-block text-label text-accent underline" href="/settings"
					>{m.settings_title()}</a
				>
			{:else}
				{#if invite.asks.length > 0}
					<p class="mt-3 text-body text-ink">{m.cookbook_invite_asks({ names: others })}</p>
				{/if}
				<p class="mt-3 text-read text-ink-2">{m.cookbook_invite_leave_any_time()}</p>
				{#if failed}
					<p class="mt-3 text-body text-support" role="alert">{failed}</p>
				{/if}
				<div class="mt-6 flex flex-col gap-2">
					<button
						type="button"
						onclick={accept}
						disabled={busy}
						class="min-h-12 rounded-sm bg-accent px-4 text-body font-semibold text-on-accent
					disabled:opacity-60"
					>
						{m.cookbook_invite_accept()}
					</button>
					<a
						href="/"
						class="flex min-h-12 items-center justify-center rounded-sm border border-rule px-4 text-body
					font-medium text-ink-2"
					>
						{m.cookbook_invite_not_now()}
					</a>
				</div>
			{/if}
		{/if}
	</div>
{/if}
