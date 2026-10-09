<!--
	Deleting this recipe (#120). The last row of the acts, in beni (#220,
	option 3 of the audit of 9 October 2026, chosen with that row in view).
	It was small underlined text apart from the stack from 22 September 2026,
	on the thought that a destructive row under *Add to shopping list* was a
	mis-tap waiting to happen; the rows are plain words now rather than
	button shapes a habit reaches for, the colour marks this one out, and the
	confirmation below is what stands between a tap and the deed.

	NOT in Writing, where the mockup drew it: that screen holds an unsaved
	draft the whole time it is open, and a screen that can both lose your
	typing and destroy the recipe is asking two very different questions with
	one set of buttons.

	`NeedsServer` because deleting is a write against the recipes, which are
	the server's side of the line and never queued (ADR 0013). The outbox
	carries your own history, never the recipes — and it queues from an
	allowlist, so this is already true rather than arranged.

	Only on a recipe you write (#131): deleting is a change, and anybody else's
	version is theirs to keep or delete, so offering it would only ever be
	offering a refusal. The recipe screen draws this only there.

	The confirmation is drawn here too, on the reading page, and goes with it:
	a read of the recipe that fails while it is open takes it away along with
	the recipe it was about (#188).
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Confirm from '$lib/Confirm.svelte';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';

	interface Props {
		branchId: string;
		/** The recipe's title, so there is no doubt which copy you meant. */
		title: string;
		/** How many times the household cooked it, across the whole Lineage. */
		cookings: number;
	}

	let { branchId, title, cookings }: Props = $props();

	const kamosu = useKamosu();

	/** The confirmation is open. */
	let confirming = $state(false);
	/** …and the act itself is running, so the sheet cannot be fired twice. */
	let deleting = $state(false);
	/** A refusal that came back, shown in the words it came in. */
	let failed = $state<string | undefined>(undefined);
	/**
	 * Whether a Share Link is live, asked only when the confirmation opens.
	 *
	 * The confirmation mentions the link **only when there is one** — a warning
	 * about a link nobody minted is noise, and noise in a sheet like this is
	 * how the sentence that matters stops being read. The recipe page does not
	 * otherwise need to know, so this is one call at the moment it becomes a
	 * fact worth having rather than a field on every read.
	 *
	 * Three states and not two, because the ask can fail. Neither silence nor
	 * an invented warning is honest then: one hides a link that really is
	 * about to stop working, the other frightens somebody about a link they
	 * never minted. `unknown` says which of the two it is and lets the reader
	 * decide, which is the only thing a screen that does not know can do.
	 */
	let shareIsLive = $state<'no' | 'yes' | 'unknown'>('no');

	/**
	 * Open the confirmation, and ask whether a Share Link is live while it
	 * opens rather than before — the sheet is drawn immediately either way,
	 * and the line appears if the answer arrives saying there is one.
	 */
	async function ask() {
		failed = undefined;
		shareIsLive = 'no';
		confirming = true;
		try {
			const link = await kamosu.getShareLink({ branch_id: branchId });
			shareIsLive = link.shared ? 'yes' : 'no';
		} catch {
			shareIsLive = 'unknown';
		}
	}

	/**
	 * Take this recipe off the shelf (#120). One Branch: the translation beside
	 * it, and any other Kitchen's copy, are untouched, and so is every cooking
	 * ever made from it.
	 *
	 * Back to the shelf afterwards, because there is nothing left to stand on.
	 * `replaceState` so the back button does not walk into a recipe that is
	 * gone — the page it would land on answers *no such Branch*, which is a
	 * true sentence and a baffling one to be shown for pressing back.
	 */
	async function deleteRecipe() {
		deleting = true;
		failed = undefined;
		try {
			await kamosu.deleteRecipe({ branch_id: branchId });
			await goto('/recipes', { replaceState: true });
		} catch (error: unknown) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		} finally {
			deleting = false;
		}
	}
</script>

<NeedsServer
	label={m.recipe_delete()}
	waiting={m.offline_waits_delete()}
	onclick={ask}
	shapeClass="list-row"
	lookClass="text-support"
	idleClass="text-ink-2 opacity-55"
/>

<!--
	The confirmation, in the one sheet Kamosu asks every irreversible thing
	through (#103).

	It names three facts and no more. The recipe's name, so there is no doubt
	which copy you meant — the route is per-Branch, so a delete offered here is
	never ambiguous. That the cooking history stays, because that is the thing
	a person would most expect a delete to take and the reassurance is the
	whole point of saying it. And the Share Link, **only when one is live**.

	No big figure leads it. The sheet's own note says the number should be the
	thing you are deciding about, and here there is not one: a large `11` over
	a delete button reads as eleven things going, which is the exact opposite
	of what this sheet is promising.

	THE COUNT IS THE HOUSEHOLD'S, NEVER THE READER'S, so every phrase here is
	impersonal — "all 11 times this was cooked", not "you cooked". A recipe is
	held by a Kitchen rather than a person (ADR 0007) and `cooked.count` tallies
	every member's Attempts across the whole Lineage, the translation's
	included. "You cooked this 11 times", said to somebody who cooked it twice,
	is a plain untruth in the one sheet that most needs to be believed. The
	Cooked section on the recipe screen is worded impersonally for exactly this
	reason; these must not drift apart.
-->
{#if confirming}
	<Confirm
		title={m.recipe_delete_title({ title })}
		consequence="{cookings === 0
			? m.recipe_delete_keeps_none()
			: cookings === 1
				? m.recipe_delete_keeps_one()
				: m.recipe_delete_keeps({ count: cookings })} {m.recipe_delete_gone()}"
		act={m.recipe_delete_act()}
		busy={deleting}
		{failed}
		run={deleteRecipe}
		cancel={() => (confirming = false)}
	>
		{#if shareIsLive === 'yes'}
			<p class="text-body text-support">{m.recipe_delete_shared()}</p>
		{:else if shareIsLive === 'unknown'}
			<p class="text-body text-support">{m.recipe_delete_share_unknown()}</p>
		{/if}
	</Confirm>
{/if}
