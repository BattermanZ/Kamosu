<!--
	Choosing the recipe an Ingredient Line names (#87).

	COMPOSITION IS NEVER GUESSED (ADR 0008, in as many words): matching
	`500 g plain flour` against a flour recipe is the kind of cleverness that is
	wrong more often than it is right. So this opens with NOTHING SELECTED and
	nothing suggested. It is a library you search, and the only thing that makes
	a line a Component is somebody picking a recipe out of it.

	IT RAISES THE LIBRARY OVER THE PAGE YOU ARE STANDING ON, which is the
	treatment Aurélien chose on 20 September 2026 against direction A (#83) and
	is why this is a sheet rather than a route. Going to the shelf and coming
	back would lose the line you were on — and on the writing screen it would
	lose everything typed since the last save.

	IT NAMES A LINEAGE, NEVER A BRANCH OR A VERSION (ADR 0008). `search_recipes`
	answers one entry per Lineage already, so the pointer is the id that entry
	carries — which is what makes it go on resolving to whatever Branch of the
	dough its reader holds, rather than to the copy that happened to be on this
	screen the day somebody linked it.

	THE RECIPE YOU ARE STANDING IN IS NOT HIDDEN FROM THE LIST. It reads as an
	odd thing to pick, and it is, but refusing it here would be a rule of this
	screen's own invention: ADR 0008 settled that cycles are never refused and
	that unfolding stops at a repeat instead, because a loop can be assembled
	from two halves on two servers and arrive already formed.
-->
<script lang="ts" module>
	/**
	 * **The Recipe an Ingredient Line names** (#87, ADR 0008) — what the picker
	 * hands back, and what both screens hold against a line until it is saved.
	 *
	 * The title may be absent where this instance does not hold that recipe:
	 * the pointer is still good and is still carried, there is simply no name
	 * to draw. A pointer with no title is an ordinary state, not a broken one.
	 */
	export interface NamedRecipe {
		lineageId: string;
		title: string | null;
	}
</script>

<script lang="ts">
	import type { Attachment } from 'svelte/attachments';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { RecipeSearch } from '$lib/search.svelte';

	interface Props {
		/** The line this picker was opened for, said at the top so a sheet
		 *  raised over a long list still says which row it belongs to. */
		line: string;
		onChoose: (chosen: NamedRecipe) => void;
		onCancel: () => void;
	}

	let { line, onChoose, onCancel }: Props = $props();

	const kamosu = useKamosu();

	/** What is typed, moment to moment. The Operation is asked on a short delay. */
	let typed = $state('');

	/**
	 * The shelf's own search, through the same helper (#121).
	 *
	 * No filters. The shelf has two and holds them nowhere (#62); a filter here
	 * would be a mode inside a sheet you opened to do one thing.
	 */
	const search = new RecipeSearch(kamosu, () => {
		const query = typed.trim();
		return {
			query: query === '' ? null : query,
			kitchen_id: null,
			mine: false,
		};
	});
	const answer = $derived(search.answer);
	const failed = $derived(search.failed);

	const entries = $derived(answer?.recipes ?? []);
	/** Whether these matched or are merely the nearest there were (ADR 0027). */
	const closest = $derived(answer?.closest ?? false);

	/**
	 * The field takes the caret, so the sheet opens ready to be typed into —
	 * and so the focus is inside the dialog rather than left on the row behind
	 * it, which is where a keyboard would otherwise still be standing.
	 *
	 * AND IT IS GIVEN BACK when the sheet closes, whichever way it closed.
	 * Without that, cancelling out of the picker drops the focus on `<body>`
	 * and a keyboard user loses their place — which on the writing screen means
	 * tabbing back down a twenty-one line list to the row they were on.
	 *
	 * The control that opened this is often NOT there to go back to: choosing a
	 * recipe turns *Recipe* into *Names Pizza Dough*, so the button that was
	 * focused is destroyed by the very choice it was opened to make. The row
	 * around it survives, so the focus goes to whatever now stands in its place
	 * — which is the control that replaced it, and the right place to be.
	 *
	 * On a frame rather than at once, because at teardown the replacement has
	 * not been drawn yet: focusing then would find nothing and land on `<body>`
	 * after all.
	 */
	const focuses: Attachment<HTMLInputElement> = (node) => {
		const cameFrom = document.activeElement;
		const row = cameFrom instanceof HTMLElement ? cameFrom.parentElement : null;
		node.focus();
		return () => {
			requestAnimationFrame(() => {
				if (cameFrom instanceof HTMLElement && cameFrom.isConnected) {
					cameFrom.focus();
					return;
				}
				row?.querySelector('button')?.focus();
			});
		};
	};
</script>

<div class="fixed inset-0 z-40 bg-accent/40"></div>
<div
	class="fixed inset-x-0 bottom-0 z-50 mx-auto flex max-h-[78vh] max-w-2xl flex-col bg-ground pb-safe"
	role="dialog"
	aria-modal="true"
	aria-label={m.component_pick_title()}
>
	<div class="border-b border-rule px-gutter py-3">
		<p class="text-label text-support-2 uppercase">{m.component_pick_title()}</p>
		<p class="mt-1 text-read text-ink-2">{m.component_pick_for({ line })}</p>
		<label class="mt-3 block">
			<span class="sr-only">{m.component_pick_search()}</span>
			<input
				type="search"
				bind:value={typed}
				placeholder={m.component_pick_search()}
				{@attach focuses}
				class="min-h-12 w-full rounded-sm border border-rule bg-card px-3 text-body"
			/>
		</label>
	</div>

	<div class="min-h-0 flex-1 overflow-y-auto">
		{#if failed}
			<p class="px-gutter py-4 text-body text-support" role="alert">{m.component_pick_failed()}</p>
		{:else if !answer}
			<p class="px-gutter py-4 text-body text-ink-2">{m.loading()}</p>
		{:else if entries.length === 0}
			<p class="px-gutter py-4 text-body text-ink-2">{m.component_pick_nothing()}</p>
		{:else}
			{#if closest}
				<p class="px-gutter pt-3 text-read text-ink-2">{m.component_pick_closest()}</p>
			{/if}
			<ul>
				{#each entries as entry (entry.lineage_id)}
					<li>
						<button
							type="button"
							class="block min-h-12 w-full border-b border-rule px-gutter py-3 text-left"
							onclick={() => onChoose({ lineageId: entry.lineage_id, title: entry.title })}
						>
							<span class="block text-line">{entry.title}</span>
							<!--
								A recipe shown in a Language the reader did not ask for says
								so, here as on the shelf: a preference may never hide a
								recipe from its owner (ADR 0006).
							-->
							{#if entry.language_fallback}
								<span class="block text-read text-ink-2">
									{m.component_pick_language({ language: entry.language })}
								</span>
							{/if}
						</button>
					</li>
				{/each}
			</ul>
		{/if}
	</div>

	<button
		type="button"
		class="block w-full border-t border-rule p-4 text-center font-display text-body text-ink-2"
		onclick={onCancel}
	>
		{m.component_pick_cancel()}
	</button>
</div>
