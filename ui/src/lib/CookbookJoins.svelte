<!--
	**A Cookbook join waiting on answers** (#135, choice C): the card Home and
	Settings both show, so a join reads the same wherever it is met.

	Nobody writes in a joined Cookbook without having said yes. Where either
	Cookbook already has other writers, accepting an Invite joins nothing yet:
	each of them is asked here, with *Write together* and *No*. Everyone else in
	it sees who it still waits for, and may call it off. The one who accepted
	also sees a no, and what would let them join anyway, for as long as the
	Invite stays open.

	Home passes `onlyAsked`: there the card is a question waiting on you, and
	nothing else about Cookbooks belongs on that screen.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { joinedNames } from '$lib/cookbook';
	import type { GetCookbookOutput } from '$lib/api/catalogue';

	type Join = GetCookbookOutput['joins'][number];

	interface Props {
		/** The viewer's own Cookbook, whose `joins` this draws. */
		cookbook: GetCookbookOutput;
		/** Draw only the joins waiting on the viewer's own answer. */
		onlyAsked?: boolean;
		/** The viewer's Cookbook as the answer left it: joined, or still waiting. */
		onAnswered: (cookbook: GetCookbookOutput) => void;
	}

	let { cookbook, onlyAsked = false, onAnswered }: Props = $props();

	const kamosu = useKamosu();

	const shown = $derived(cookbook.joins.filter((join) => !onlyAsked || join.you === 'asked'));
	let busy = $state<string | undefined>(undefined);
	let failed = $state<string | undefined>(undefined);

	/** The people on the other side of the join from the viewer: whom it brings them. */
	function others(join: Join): string {
		const side = join.into.id === cookbook.id ? join.joining : join.into;
		return joinedNames(side.authors.map((author) => author.name));
	}

	function names(people: Join['waiting_on']): string {
		return joinedNames(people.map((person) => person.name));
	}

	async function answer(join: Join, yes: boolean) {
		busy = join.join_id;
		failed = undefined;
		try {
			onAnswered(await kamosu.answerCookbookJoin({ join_id: join.join_id, yes }));
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		} finally {
			busy = undefined;
		}
	}
</script>

{#each shown as join (join.join_id)}
	<div class="rounded-sm border border-t-4 border-rule border-t-accent bg-card p-3">
		{#if join.state === 'refused' && join.refused_by}
			<p class="text-body text-ink" role="status">
				{m.cookbook_join_refused({ name: join.refused_by.name })}
			</p>
			{#if join.refused_by_co_author}
				<p class="mt-2 text-read text-ink-2">{m.cookbook_join_refused_leave_first()}</p>
			{/if}
		{:else if join.you === 'asked'}
			<h2 class="font-display text-shelf-heading font-semibold text-ink">
				{m.cookbook_join_asked_title({ names: others(join) })}
			</h2>
			<p class="mt-2 text-body text-ink">
				{m.cookbook_join_who_asked({
					inviter: join.invited_by.name,
					accepter: join.accepted_by.name,
				})}
			</p>
			<p class="mt-2 text-read text-ink-2">
				{m.cookbook_join_asked_said({ total: join.together_recipes })}
			</p>
			<div class="mt-3 flex flex-col gap-2">
				<button
					type="button"
					onclick={() => answer(join, true)}
					disabled={busy === join.join_id}
					class="min-h-12 rounded-sm bg-accent px-4 text-body font-semibold text-on-accent
					disabled:opacity-60"
				>
					{m.cookbook_invite_accept()}
				</button>
				<button
					type="button"
					onclick={() => answer(join, false)}
					disabled={busy === join.join_id}
					class="min-h-12 rounded-sm border border-rule px-4 text-body font-medium text-ink-2
					disabled:opacity-60"
				>
					{m.cookbook_join_no()}
				</button>
			</div>
		{:else}
			<h2 class="text-label font-medium text-accent uppercase">
				{m.cookbook_join_waiting_title({ names: others(join) })}
			</h2>
			<p class="mt-2 text-body text-ink">
				{m.cookbook_join_waiting_on({ names: names(join.waiting_on) })}
			</p>
			<button
				type="button"
				class="mt-2 text-label text-support underline disabled:opacity-60"
				disabled={busy === join.join_id}
				onclick={() => answer(join, false)}
			>
				{join.you === 'accepted' ? m.cookbook_join_take_back() : m.cookbook_join_call_off()}
			</button>
		{/if}
	</div>
{/each}
{#if failed}
	<p class="mt-2 text-body text-support" role="alert">{failed}</p>
{/if}
