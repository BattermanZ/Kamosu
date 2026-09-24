<!--
	Which recipes this one goes with (#105).

	**Making and unmaking a link are the same question asked twice**, so they
	live in one sheet: a list you tick. There is no second way to do it — no
	remove button on the strip behind, no long-press, no swipe. `TagSheet` proved
	the shape and this is deliberately the same one (a scrim, a header saying
	what the sheet is for, a field that takes the caret, a scrolling list of
	ticks, one way out at the foot), because relating and filing are the same
	kind of act and a person who has met one sheet has met both.

	IT IS A SHEET RAISED OVER THE PAGE, not a route, for `ComponentPicker`'s
	reason: going to another screen and coming back loses the place you were
	standing in — and on a recipe page that is the step you had reached.

	THE LINKS THIS RECIPE ALREADY HAS ARE LISTED FIRST, above whatever the search
	found. That is not decoration: a link to a Lineage that has left the shelf
	appears in NO search, because it is not on the shelf to be found, so listing
	only search results would leave exactly the link you most want to remove
	unreachable. It is named by its Lineage when unticked, which a deleted recipe
	still has, and that is the whole reason `set_related_recipe` learned to take
	one (#105).

	IT NAMES A LINEAGE, NEVER A BRANCH OR A VERSION (#52, ADR 0025). The link is
	between two Lineages and one link is both directions, so ticking from either
	end changes the same row. Nothing here joins the two: no Lineage is merged,
	and there is no operation that would.

	NOTHING HERE IS A CHANGE TO THE RECIPE (ADR 0035). No Version is minted,
	nothing reaches the Thread, and no *unsaved changes* tray is drawn. The sheet
	says so once, in the empty state, and then stops explaining itself.
-->
<script lang="ts" module>
	/**
	 * One Related Recipe as the Core answers it: the Lineage the link names, the
	 * Branch to open where this Kitchen still holds one, the title, the picture
	 * to draw, and whether that Branch is in a Language the reader did not ask
	 * for.
	 *
	 * `branch_id`, `main_photo` and `language` are all null once the Lineage has
	 * left the shelf, because there is no recipe left to have any of them. That
	 * is an ordinary state and not a broken one: the row kept the name it was
	 * known by precisely so the line stays readable (#52).
	 */
	export interface Related {
		lineage_id: string;
		branch_id: string | null;
		title: string;
		main_photo: string | null;
		language: string | null;
		language_fallback: boolean;
	}
</script>

<script lang="ts">
	import type { Attachment } from 'svelte/attachments';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { refreshed } from '$lib/offline/device.svelte';
	import { fallbackLanguage } from '../matched';
	import type { SearchRecipesOutput } from '$lib/api/catalogue';

	/**
	 * What a row in either list holds. A link this recipe already has and a
	 * recipe the search turned up are the same three facts as far as a row is
	 * concerned, which is what lets one row shape serve both.
	 */
	type Listed = {
		lineage_id: string;
		branch_id: string | null;
		title: string;
		language: string | null;
		language_fallback: boolean;
	};

	interface Props {
		branchId: string;
		/** The recipe's links as the page has them, so the ticks are right on open. */
		carried: Related[];
		/** The recipe's links after a change, for the strip behind this sheet. */
		onChanged: (related: Related[]) => void;
		onClose: () => void;
	}

	let { branchId, carried, onChanged, onClose }: Props = $props();

	const kamosu = useKamosu();

	/** What is typed, moment to moment. The Operation is asked on a short delay. */
	let typed = $state('');
	let answer = $state<SearchRecipesOutput | undefined>(undefined);
	let failed = $state(false);
	/** The link being made or broken right now, so a row cannot be double-tapped. */
	let working = $state<string | null>(null);
	/** A refusal, said in the words it came in. */
	let refused = $state<string | null>(null);

	/**
	 * The same ask the shelf makes, with the same delay, and deliberately the
	 * same Operation: Meaning Search arrived inside `search_recipes` rather than
	 * beside it (ADR 0029), so a picker that asked its own way would find fewer
	 * recipes than the shelf does for the same words.
	 */
	$effect(() => {
		const query = typed.trim();
		// The phone answered first and the server has since answered otherwise (#76).
		void refreshed.get('search_recipes');
		let current = true;
		const timer = setTimeout(() => {
			void (async () => {
				try {
					const found = await kamosu.searchRecipes({
						// The whole shelf: a link may reach any recipe the reader
						// sees, in whichever Cookbook (#131, question 8).
						query: query === '' ? null : query,
						kitchen_id: null,
						mine: false,
					});
					if (current) {
						answer = found;
						failed = false;
					}
				} catch (error) {
					if (!(error instanceof OperationError)) throw error;
					if (current) failed = true;
				}
			})();
			// Long enough that typing a recipe's name is one ask rather than
			// twelve, short enough that the list keeps up with the thumb — the
			// shelf's own delay, because it is the same search.
		}, 180);
		return () => {
			current = false;
			clearTimeout(timer);
		};
	});

	/** Which Lineages this recipe is linked to, as the server last said. */
	const on = $derived(new Set(carried.map((entry) => entry.lineage_id)));
	/** Whether these matched or are merely the nearest there were (ADR 0027). */
	const closest = $derived(answer?.closest ?? false);
	/**
	 * What the search found, less the recipe standing on this screen and less
	 * everything already listed above as a link. A recipe cannot be related to
	 * itself, so offering it would be offering a refusal.
	 */
	const found = $derived(
		(answer?.recipes ?? []).filter(
			(entry) => !on.has(entry.lineage_id) && entry.branch_id !== branchId,
		),
	);

	/**
	 * Put a link on or take it off. The far end is named by its Branch where
	 * the reader can see one and by its Lineage where they cannot, which is the
	 * case that could not be reached at all before #105.
	 */
	async function toggle(entry: Listed) {
		if (working !== null) return;
		working = entry.lineage_id;
		refused = null;
		try {
			const next = await kamosu.setRelatedRecipe({
				branch_id: branchId,
				...(entry.branch_id
					? { related_branch_id: entry.branch_id }
					: { related_lineage_id: entry.lineage_id }),
				related: !on.has(entry.lineage_id),
			});
			onChanged(next.related_recipes);
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			refused = error.message;
		} finally {
			working = null;
		}
	}

	/**
	 * The field takes the caret, so the sheet opens ready to be typed into, and
	 * gives it back on the way out — `TagSheet`'s attachment, for the same
	 * reason: a keyboard user who cancels out of a sheet must not lose their
	 * place on the page behind it.
	 */
	const focuses: Attachment<HTMLInputElement> = (node) => {
		const cameFrom = document.activeElement;
		node.focus();
		return () => {
			requestAnimationFrame(() => {
				if (cameFrom instanceof HTMLElement && cameFrom.isConnected) cameFrom.focus();
			});
		};
	};

	/** Escape closes it, as it closes every other sheet in Kamosu. */
	function onKey(event: KeyboardEvent) {
		if (event.key === 'Escape') onClose();
	}
</script>

<svelte:window onkeydown={onKey} />

<!--
	One row, ticked or not, for both lists. The two differ only in what they hold
	— a link this recipe already has, or a recipe the search turned up — and a
	row that looked different in the two places would say they were different
	kinds of thing.
-->
{#snippet row(entry: Listed, ticked: boolean)}
	{@const mark = fallbackLanguage({ ...entry, language: entry.language ?? '' })}
	<button
		type="button"
		role="switch"
		aria-checked={ticked}
		disabled={working !== null}
		onclick={() => toggle(entry)}
		class="flex min-h-12 w-full items-center gap-3 border-b border-rule px-gutter py-2
		text-left disabled:opacity-60"
	>
		<!--
			Shape only, no tick glyph in the markup: the state is on the switch
			above, where a screen reader reads it, and the square is what an eye
			reads.
		-->
		<span class="block h-4 w-4 shrink-0 rounded-sm border border-accent {ticked ? 'bg-accent' : ''}"
		></span>
		<span class="min-w-0 flex-1">
			<span class="block text-line">{entry.title}</span>
			{#if entry.branch_id === null}
				<span class="block text-read text-ink-2">{m.related_gone()}</span>
			{:else if mark}
				<!--
					Shown in a Language the reader did not ask for, and saying so in
					the shelf's own words: a preference may never hide a recipe from
					its owner (ADR 0006).
				-->
				<span class="block text-read text-ink-2">{mark}</span>
			{/if}
		</span>
	</button>
{/snippet}

<div class="fixed inset-0 z-40 bg-accent/40"></div>
<div
	class="fixed inset-x-0 bottom-0 z-50 mx-auto flex max-h-[78vh] max-w-2xl flex-col bg-ground pb-safe"
	role="dialog"
	aria-modal="true"
	aria-label={m.related_title()}
>
	<div class="border-b border-rule px-gutter py-3">
		<p class="text-label text-support-2 uppercase">{m.related_title()}</p>
		<label class="mt-3 block">
			<span class="sr-only">{m.related_search()}</span>
			<input
				type="search"
				bind:value={typed}
				placeholder={m.related_search()}
				{@attach focuses}
				class="min-h-12 w-full rounded-sm border border-rule bg-card px-3 text-body"
			/>
		</label>
	</div>

	<div class="min-h-0 flex-1 overflow-y-auto">
		{#if refused}
			<p class="px-gutter pt-3 text-read text-support" role="alert">{refused}</p>
		{/if}

		{#if carried.length > 0}
			<h3 class="px-gutter pt-3 text-label text-ink-2 uppercase">{m.related_carried()}</h3>
			<ul>
				{#each carried as entry (entry.lineage_id)}
					<li>{@render row(entry, true)}</li>
				{/each}
			</ul>
		{/if}

		<!--
			A recipe with no links yet is told what a Related Recipe is, and told
			outright that making one does not change either recipe — the one thing
			ADR 0035 asks any of these screens to make plain. Said here and nowhere
			else: a sentence on every recipe forever would be a lecture.

			DRAWN ABOVE THE LIST RATHER THAN INSTEAD OF IT. An empty query answers
			with the whole shelf, so a shelf holding any recipes at all never has
			an empty list to put this in — which is exactly the state every real
			library is in, and is how the first draft of this contrived to say the
			one required sentence nowhere.
		-->
		{#if carried.length === 0 && typed.trim() === ''}
			<p class="px-gutter pt-3 pb-1 text-body text-ink-2">{m.related_none_yet()}</p>
		{/if}

		{#if failed}
			<p class="px-gutter py-4 text-body text-support" role="alert">{m.related_failed()}</p>
		{:else if !answer}
			<p class="px-gutter py-4 text-body text-ink-2">{m.loading()}</p>
		{:else if found.length === 0}
			<p class="px-gutter py-4 text-body text-ink-2">{m.related_nothing()}</p>
		{:else}
			{#if closest}
				<p class="px-gutter pt-3 text-read text-ink-2">{m.related_closest()}</p>
			{/if}
			<ul>
				{#each found as entry (entry.lineage_id)}
					<li>{@render row(entry, false)}</li>
				{/each}
			</ul>
		{/if}
	</div>

	<button
		type="button"
		class="block w-full border-t border-rule p-4 text-center font-display text-body text-accent"
		onclick={onClose}
	>
		{m.related_done()}
	</button>
</div>
