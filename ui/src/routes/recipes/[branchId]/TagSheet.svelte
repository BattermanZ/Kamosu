<!--
	Which Tags this recipe has (#104).

	**It does one job.** Renaming, merging and deleting a Tag are not here, and
	that is Aurélien's choice of 22 September 2026, made after seeing them here:
	"I don't think a recipe page is the place to rename tags, it makes no sense."
	They live in Settings → Tags, because they are about the whole library rather
	than about the dish on screen. What is left is the only question this sheet
	should ask: which of your words describe this recipe.

	IT IS A SHEET RAISED OVER THE PAGE, not a route, for `ComponentPicker`'s
	reason: going to another screen and coming back loses the place you were
	standing in. The two are deliberately the same shape — a scrim, a header
	that says what the sheet is for, a field that takes the caret, a scrolling
	list, one way out at the foot — because a person who has met one has met
	both.

	A WORD NOBODY HAS USED YET IS MADE HERE. `create_tag` answers with the Tag
	the Cookbook already has for that word rather than a second one, so two people
	reaching for *dessert* have agreed rather than collided (#51) — which means
	this screen never has to ask whether a word is taken.

	NOTHING HERE IS A CHANGE TO THE RECIPE (ADR 0035). No Version is minted,
	nothing appears in the Thread, and no *unsaved changes* tray is drawn. The
	sheet says so once, in the empty state, and then stops explaining itself.
-->
<script lang="ts">
	import { untrack } from 'svelte';
	import type { Attachment } from 'svelte/attachments';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { byWord, tagWord, type ReadingLanguage, type Tag } from '$lib/tags';

	interface Props {
		branchId: string;
		/** The recipe's Tags as the page has them, so the ticks are right on open. */
		carried: Tag[];
		/** The recipe's Tags after a change, for the row behind this sheet. */
		onChanged: (tags: Tag[]) => void;
		onClose: () => void;
	}

	let { branchId, carried, onChanged, onClose }: Props = $props();

	const kamosu = useKamosu();

	/** Every word this Cookbook files by, or nothing yet. */
	let all = $state<Tag[] | undefined>(undefined);
	let failed = $state(false);
	/** What is typed, moment to moment. No delay: the list is already here. */
	let typed = $state('');
	/** The Tag being put on or taken off right now, so a row cannot be double-tapped. */
	let working = $state<string | null>(null);
	/** A refusal, said in the words it came in. */
	let refused = $state<string | null>(null);
	/**
	 * The Language a new Tag is named in: the one this reader reads recipes in,
	 * held on their account rather than in this browser (#49, ADR 0016). Not the
	 * interface locale, which is a separate setting — somebody reading Kamosu in
	 * French and their recipes in English is naming an English word.
	 *
	 * Only the *writes* need it. Which name to SHOW, and whether to mark it, are
	 * the Core's answers already (`tagWord`).
	 */
	let naming = $state<ReadingLanguage | undefined>(undefined);

	/**
	 * Which ids this recipe carries. Held here rather than derived so that a tap
	 * answers at once, and seeded from the prop with `untrack` because that is a
	 * read of what it was when the sheet opened: every change after this comes
	 * from `set_recipe_tag`'s own answer, which is the server's word rather than
	 * this component's guess.
	 */
	let on = $state(untrack(() => new Set(carried.map((tag) => tag.id))));

	$effect(() => {
		let current = true;
		void (async () => {
			try {
				const [held, preferences] = await Promise.all([
					kamosu.listTags({}),
					kamosu.getReadingPreferences(),
				]);
				if (!current) return;
				all = held.tags;
				naming = preferences.reading_language;
				failed = false;
			} catch (error) {
				if (!(error instanceof OperationError)) throw error;
				if (current) failed = true;
			}
		})();
		return () => {
			current = false;
		};
	});

	const listed = $derived.by(() => {
		const looking = typed.trim().toLocaleLowerCase();
		return byWord(
			(all ?? []).filter(
				(tag) => looking === '' || tagWord(tag).name.toLocaleLowerCase().includes(looking),
			),
		);
	});

	/**
	 * Whether what is typed is a word this Cookbook has no tag for — the case
	 * that offers to make one. Compared against every tag's every name rather
	 * than only the shown one: a Cookbook holding *goûter* in French must not
	 * offer to make a second when somebody types it while reading English.
	 */
	const newWord = $derived.by(() => {
		const word = typed.trim();
		if (word === '' || all === undefined || naming === undefined) return null;
		const folded = word.toLocaleLowerCase();
		const known = all.some((tag) =>
			tag.names.some((named) => named.name.toLocaleLowerCase() === folded),
		);
		return known ? null : word;
	});

	async function toggle(tag: Tag) {
		if (working !== null) return;
		working = tag.id;
		refused = null;
		const carry = !on.has(tag.id);
		try {
			const answer = await kamosu.setRecipeTag({
				branch_id: branchId,
				tag_id: tag.id,
				carried: carry,
			});
			on = new Set(answer.tags.map((held) => held.id));
			onChanged(answer.tags);
			// The count on every row moved, so the list is read again rather
			// than patched here: one of these numbers being wrong is worse than
			// the extra ask, and the ask is a cheap one.
			all = (await kamosu.listTags({})).tags;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			refused = error.message;
		} finally {
			working = null;
		}
	}

	/**
	 * Make the typed word and put it on this recipe, in one tap. Two Operations,
	 * because making a Tag and using it are two facts: a word made here and
	 * then refused on the recipe is still the Cookbook's word, which is why the
	 * failure below does not pretend nothing happened.
	 */
	async function create(word: string) {
		if (working !== null) return;
		working = 'new';
		refused = null;
		try {
			const made = await kamosu.createTag({
				// Named in the Language this reader reads RECIPES in. Nothing
				// reads the word itself to guess: a French cook typing an
				// English word has typed an English word, and Settings is where
				// that is put right.
				language: naming ?? 'en',
				name: word,
			});
			const answer = await kamosu.setRecipeTag({
				branch_id: branchId,
				tag_id: made.id,
				carried: true,
			});
			on = new Set(answer.tags.map((held) => held.id));
			onChanged(answer.tags);
			all = (await kamosu.listTags({})).tags;
			typed = '';
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			refused = error.message;
		} finally {
			working = null;
		}
	}

	/**
	 * The field takes the caret, so the sheet opens ready to be typed into, and
	 * gives it back on the way out — `ComponentPicker`'s attachment, for the
	 * same reason: a keyboard user who cancels out of a sheet must not lose
	 * their place on the page behind it.
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

<div class="fixed inset-0 z-40 bg-accent/40"></div>
<div
	class="fixed inset-x-0 bottom-0 z-50 mx-auto flex max-h-[78vh] max-w-2xl flex-col bg-ground pb-safe"
	role="dialog"
	aria-modal="true"
	aria-label={m.tags_title()}
>
	<div class="border-b border-rule px-gutter py-3">
		<p class="text-label text-support-2 uppercase">{m.tags_title()}</p>
		<label class="mt-3 block">
			<span class="sr-only">{m.tags_search()}</span>
			<input
				type="search"
				bind:value={typed}
				placeholder={m.tags_search()}
				{@attach focuses}
				class="min-h-12 w-full rounded-sm border border-rule bg-card px-3 text-body"
			/>
		</label>
	</div>

	<div class="min-h-0 flex-1 overflow-y-auto">
		{#if refused}
			<p class="px-gutter pt-3 text-read text-support" role="alert">{refused}</p>
		{/if}

		{#if failed}
			<p class="px-gutter py-4 text-body text-support" role="alert">{m.tags_failed()}</p>
		{:else if all === undefined}
			<p class="px-gutter py-4 text-body text-ink-2">{m.loading()}</p>
		{:else}
			<ul>
				<!--
					The word nobody has used yet, offered as the first row so the
					thumb is already there: somebody typing a word the Cookbook
					does not have is telling you they want it.
				-->
				{#if newWord}
					<li>
						<button
							type="button"
							disabled={working !== null}
							onclick={() => create(newWord)}
							class="flex min-h-12 w-full items-center gap-3 border-b border-rule px-gutter py-2
							text-left disabled:opacity-60"
						>
							<span class="block h-4 w-4 shrink-0 rounded-sm border border-accent"></span>
							<span class="font-display text-line text-accent">
								{m.tags_create({ word: newWord })}
							</span>
						</button>
					</li>
				{/if}

				{#each listed as tag (tag.id)}
					{@const word = tagWord(tag)}
					<li>
						<button
							type="button"
							role="switch"
							aria-checked={on.has(tag.id)}
							disabled={working !== null}
							onclick={() => toggle(tag)}
							class="flex min-h-12 w-full items-center gap-3 border-b border-rule px-gutter py-2
							text-left disabled:opacity-60"
						>
							<!--
								Shape only, no tick glyph in the markup: the state is
								on the switch above, where a screen reader reads it,
								and the square is what an eye reads.
							-->
							<span
								class="block h-4 w-4 shrink-0 rounded-sm border border-accent {on.has(tag.id)
									? 'bg-accent'
									: ''}"
							></span>
							<span class="min-w-0 flex-1 text-line">
								{word.name}
								{#if word.elsewhere}
									<span class="text-label text-support uppercase">
										<span aria-hidden="true">{word.elsewhere}</span>
										<span class="sr-only">{word.said}</span>
									</span>
								{/if}
							</span>
							<span class="shrink-0 text-read text-ink-2">{tag.recipes}</span>
						</button>
					</li>
				{/each}
			</ul>

			{#if all.length === 0 && !newWord}
				<!--
					A Cookbook that files nothing yet explains what a Tag is and
					says outright that using one does not change the recipe,
					which is the one thing ADR 0035 asks any of these screens to
					make plain. It is said here and nowhere else: a sentence on
					every recipe forever would be a lecture.
				-->
				<p class="px-gutter py-4 text-body text-ink-2">{m.tags_none_yet()}</p>
			{:else if listed.length === 0 && !newWord}
				<p class="px-gutter py-4 text-body text-ink-2">{m.tags_no_match()}</p>
			{/if}
		{/if}
	</div>

	<button
		type="button"
		class="block w-full border-t border-rule p-4 text-center font-display text-body text-accent"
		onclick={onClose}
	>
		{m.tags_done()}
	</button>
</div>
