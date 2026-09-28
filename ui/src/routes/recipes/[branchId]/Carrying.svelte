<!--
	What has been carried across from the other version and not yet saved, and
	the sheet that saves it. It becomes real only when an ordinary Version is
	saved — there is no other kind of save here. `marking.svelte.ts` holds the
	state, since the reading page it sits on can be swapped out for writing.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';
	import type { Marking } from './marking.svelte';

	interface Props {
		marking: Marking;
		/** Whose the other version is, named plainly. */
		otherKitchen: string;
	}

	let { marking, otherKitchen }: Props = $props();
</script>

{#if marking.taken.size > 0}
	<div
		class="fixed inset-x-0 bottom-tabbar z-30 mx-auto max-w-2xl border-t border-on-accent/25 bg-accent px-gutter py-3 text-on-accent"
	>
		<p class="mb-2 text-read">
			{marking.taken.size === 1
				? m.divergence_unsaved_one({ kitchen: otherKitchen })
				: m.divergence_unsaved({
						count: marking.taken.size,
						kitchen: otherKitchen,
					})}
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
	<div class="fixed inset-0 z-40 bg-accent/40"></div>
	<div
		class="fixed inset-x-0 bottom-0 z-50 mx-auto max-h-[78vh] max-w-2xl overflow-y-auto bg-ground px-gutter pt-4 pb-safe"
		role="dialog"
		aria-modal="true"
		aria-label={m.divergence_save()}
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
			{m.divergence_save_hint({ kitchen: otherKitchen })}
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
	</div>
{/if}
