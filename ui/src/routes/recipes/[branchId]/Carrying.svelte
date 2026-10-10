<!--
	What has been carried across from the other version and not yet saved, and
	the sheet that saves it. It becomes real only when an ordinary Version is
	saved — there is no other kind of save here. `marking.svelte.ts` holds the
	state, since the reading page it sits on can be swapped out for writing.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';
	import SheetFrame from '$lib/SheetFrame.svelte';
	import type { Marking } from './marking.svelte';
	import { between, type Words } from './divergence';

	interface Props {
		marking: Marking;
		/** Whose the other version is, named plainly. Left out where `words` is given. */
		otherKitchen?: string;
		/** What it says where the other recipe is an older Version (#211). */
		words?: Words;
	}

	let { marking, otherKitchen = '', words = undefined }: Props = $props();
	const say = $derived(words ?? between(otherKitchen));
</script>

{#if marking.taken.size > 0}
	<div
		class="fixed inset-x-0 bottom-tabbar z-30 mx-auto max-w-2xl border-t border-on-accent/25 bg-accent px-gutter py-3 text-on-accent"
	>
		<p class="mb-2 text-read">
			{say.unsaved(marking.taken.size)}
		</p>
		<div class="flex gap-2">
			<!-- Saving a Version is editing the recipe: it waits for the server,
			     and what was carried across stays carried until then (#76). -->
			<NeedsServer
				label={m.divergence_save()}
				waiting={m.offline_waits_save()}
				onclick={marking.startSaving}
				shapeClass="flex-1 p-2 text-center text-read"
				lookClass="bg-on-accent text-accent"
				idleClass="border border-on-accent/40 text-on-accent opacity-55"
			/>
			<button
				type="button"
				onclick={marking.undoAll}
				class="flex-1 border border-on-accent/40 p-2 text-center text-read"
			>
				{m.divergence_undo()}
			</button>
		</div>
	</div>
{/if}

{#if marking.saving}
	<SheetFrame
		label={m.divergence_save()}
		tall={78}
		class="overflow-y-auto bg-ground px-gutter pt-4"
		onclose={() => (marking.saving = false)}
	>
		<h3 class="font-display text-title font-semibold">{m.divergence_save()}</h3>
		<label class="mt-4 block text-label text-ink-2 uppercase" for="what-changed">
			{m.divergence_what_changed()}
		</label>
		<textarea
			id="what-changed"
			rows="3"
			bind:value={marking.changeNote}
			class="mt-1 w-full rounded-sm border border-rule bg-card p-3 text-body"></textarea>
		<p class="mt-2 text-read text-ink-2">
			{say.saveHint}
		</p>
		<NeedsServer
			label={m.divergence_save()}
			waiting={m.offline_waits_save()}
			onclick={marking.save}
			shapeClass="mt-4 block w-full p-4 text-center font-display text-body"
			lookClass="bg-accent text-on-accent"
		/>
		<button
			type="button"
			onclick={() => (marking.saving = false)}
			class="mt-2 block w-full border border-rule p-4 text-center font-display text-body text-accent"
		>
			{m.divergence_cancel()}
		</button>
	</SheetFrame>
{/if}
