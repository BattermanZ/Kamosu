<!--
	Keeping a tweaked cooking as a Version, from the diary (#210): a section of
	the open cooking, between its photographs and its note.

	THE RECIPE PAGE ASKS; THIS ONLY OFFERS. `Promotion.svelte` is a band on the
	recipe that puts the question. Here there is nothing on the closed row and
	one section inside a cooking the cook opened themselves, which was option A
	of 10 October 2026. It keeps through the same Operation,
	`promote_as_cooked`, so what lands is the same Version either way.

	PRESSING KEEP DOES NOT MINT A VERSION, for the reason the band gives: the
	save is a second, deliberate tap, in place, with the moved-on warning and
	the Copy sentence said first where they apply.

	NOT NOW ANSWERS NOTHING, the second choice of that day. It closes the
	confirmation and never calls `decline_promotion`, and a cooking answered
	"Leave it in the diary" on the recipe page is still offered here. The
	diary is where the changes were left, so it is where they can be kept.

	WHETHER THERE IS ANYTHING TO KEEP IS THE CORE'S ANSWER, `unkept` on the
	entry, as are the lines shown (`as_cooked.against`). This screen compares
	no Version ids and matches no line to a line.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { ListAttemptsOutput, PromoteAsCookedOutput } from '$lib/api/catalogue';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';
	import { copySaid } from '$lib/cookbook';
	import { changedLines } from '$lib/as-cooked';
	import ChangedLines from '$lib/ChangedLines.svelte';

	type Entry = ListAttemptsOutput['attempts'][number];

	interface Props {
		/** The cooking, with the As Cooked it holds and where keeping it lands. */
		entry: Entry;
		/** What keeping answered, once it has: it names the Branch the Version is on now. */
		kept: PromoteAsCookedOutput | null;
		onKept: (landed: PromoteAsCookedOutput) => void;
	}

	let { entry, kept, onKept }: Props = $props();

	/**
	 * Where keeping lands, as the Core answers it, and null once it is kept.
	 * `?? null` because a diary the phone kept before #210 has none.
	 */
	const keeping = $derived(entry.unkept ?? null);

	const kamosu = useKamosu();

	let confirming = $state(false);
	let saving = $state(false);
	let failed = $state(false);

	const marked = $derived(changedLines(entry.as_cooked?.against));

	/** Keeping onto a recipe that is not the cook's to change starts their own. */
	const forking = $derived(keeping !== null && !keeping.writes);

	async function keep() {
		if (!keeping) return;
		saving = true;
		failed = false;
		try {
			onKept(await kamosu.promoteAsCooked({ attempt_id: entry.id, branch_id: keeping.branch_id }));
			confirming = false;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = true;
		} finally {
			saving = false;
		}
	}
</script>

<div class="mt-3 border-t border-rule pt-3">
	<h3 class="mb-2 text-label text-ink-2 uppercase">{m.cooked_changed()}</h3>
	{#if kept}
		<p class="mt-2 text-read text-accent" role="status">
			{kept.copied ? m.cooked_kept_copy() : m.cooked_kept()}
			<a href={`/recipes/${kept.branch_id}`} class="underline">
				{kept.copied ? m.cooked_open_copy() : m.cooked_open_recipe()}
			</a>
		</p>
	{:else if keeping}
		<ChangedLines lines={marked} />

		{#if confirming}
			{#if keeping.moved_on}
				<p class="mt-3 text-read text-support">{m.recipe_moved_on_since()}</p>
			{/if}
			{#if forking && entry.as_cooked}
				<p class="mt-4 text-label text-support uppercase">{m.write_will_fork()}</p>
				<p class="mt-1 text-read">{copySaid(keeping, entry.as_cooked.content.title)}</p>
			{/if}
			<NeedsServer
				label={forking ? m.write_do_fork() : m.recipe_save_as_version()}
				waiting={m.offline_waits_keep()}
				disabled={saving}
				onclick={keep}
				shapeClass="mt-3 block min-h-12 w-full rounded-sm px-4 py-3 text-center font-display text-body"
				lookClass="{forking ? 'bg-support' : 'bg-accent'} text-on-accent disabled:opacity-60"
			/>
			<button
				type="button"
				onclick={() => (confirming = false)}
				class="mt-2 block min-h-12 w-full rounded-sm border border-rule px-4 py-3 text-center font-display text-body text-ink-2"
			>
				{m.recipe_not_now()}
			</button>
		{:else}
			<NeedsServer
				label={forking ? m.recipe_keep_as_copy() : m.recipe_keep_as_version()}
				waiting={m.offline_waits_keep()}
				onclick={() => (confirming = true)}
				shapeClass="mt-3 block min-h-12 w-full rounded-sm px-4 py-3 text-center font-display text-body"
				lookClass="border border-accent text-accent"
			/>
		{/if}

		{#if failed}
			<p class="mt-2 text-read text-support" role="alert">{m.recipe_promote_failed()}</p>
		{/if}
	{/if}
</div>
