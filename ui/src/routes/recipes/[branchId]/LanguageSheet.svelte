<!--
	Saying what Language a recipe is in, and starting a Translation (#106,
	ADR 0006).

	IT IS A SHEET RAISED OVER THE PAGE, the same shape as `TagSheet` and
	`ComponentPicker`: a scrim, a header saying what it is for, a scrolling
	list, one way out at the foot. A person who has met one has met all three.

	THIS ONE MINTS A VERSION, AND SAYS SO — which is exactly what separates it
	from the two sheets it looks like. Tagging (#104) and relating (#105) are
	notes on a Kitchen's own shelf and appear nowhere in the Thread. Saying
	what Language a recipe is in is a change TO the recipe: it appends an
	occurrence of the head content carrying the new Language, so it leaves a
	trace in an otherwise append-only history (ADR 0006). The sentence under
	the heading says so before anything is tapped, and it is not an apology —
	it is the difference a reader has to know.

	UNKNOWN IS OFFERED, AND REFUSED IN PLACE WHERE IT CANNOT APPLY. A recipe
	that is a Translation, or that has one, is by definition written in the one
	Language that Translation renders, so the Core refuses Unknown on it. The
	row is drawn either way and says why it cannot be taken, rather than
	vanishing: a choice that disappears without explanation reads as a bug, and
	the reason is the interesting part.

	TRANSLATING IS A WRITING ACT, NOT A TAP. `start_translation` takes the
	whole recipe as it now reads in the new Language — it is a Branch with
	content, not a flag — so this sheet cannot make one on its own. It hands
	the chosen Language up, and the page opens the writing screen on a draft of
	this recipe's words for them to be replaced. That is what translating is.

	NOTHING HERE NAMES A TRANSLATION AS ANYTHING BUT A RECIPE. There is no
	Translation object in Kamosu and no screen may invent one: what this makes
	is an ordinary Branch of the same Lineage, and the sentence beside the
	control says so in those terms.
-->
<script lang="ts">
	import type { Attachment } from 'svelte/attachments';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import {
		BRANCH_LANGUAGES,
		WRITTEN_LANGUAGES,
		isUnknown,
		languageName,
		type BranchLanguage,
		type WrittenLanguage,
	} from '$lib/language';

	interface Props {
		branchId: string;
		/** The Language this recipe carries now, so the list opens marked. */
		language: string;
		/**
		 * Whether this recipe is a Translation, or has one. Either way Unknown
		 * cannot apply: the Core refuses it, and the row says why rather than
		 * waiting to be tapped and then reporting a refusal.
		 */
		inAFamily: boolean;
		/** The Languages this Lineage already exists in, which are not worth translating into again. */
		taken: string[];
		/**
		 * A Language was said. Carries the Branch it landed on, because saying
		 * this about a recipe another Kitchen writes is a Copy like any other
		 * change (ADR 0020) and the answer then names a NEW Branch.
		 */
		onSaid: (landed: { branch_id: string; language: string }) => void;
		/** Translate into this Language — the page opens the writing screen on it. */
		onTranslate: (language: WrittenLanguage) => void;
		onClose: () => void;
	}

	let { branchId, language, inAFamily, taken, onSaid, onTranslate, onClose }: Props = $props();

	const kamosu = useKamosu();

	/** Which row is being said right now, so a row cannot be double-tapped. */
	let saying = $state<string | null>(null);
	/** A refusal, said in the words it came in — `TagSheet`'s rule. */
	let refused = $state<string | null>(null);

	/**
	 * The Languages worth translating into: every written Language this Lineage
	 * does not already hold. Unknown is absent by construction — a recipe
	 * honestly written in two can neither be a Translation nor have one, which
	 * is why `WRITTEN_LANGUAGES` and not `BRANCH_LANGUAGES` is walked here.
	 */
	const translatable = $derived(
		WRITTEN_LANGUAGES.filter((each) => each !== language && !taken.includes(each)),
	);

	async function say(next: BranchLanguage) {
		if (saying !== null || next === language) return;
		saying = next;
		refused = null;
		try {
			const answer = await kamosu.setRecipeLanguage({ branch_id: branchId, language: next });
			onSaid({ branch_id: answer.branch_id, language: answer.language });
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			refused = error.message;
		} finally {
			saying = null;
		}
	}

	/**
	 * The sheet takes the caret on the way in and gives it back on the way
	 * out — `TagSheet`'s attachment, for the same reason: a keyboard user who
	 * cancels out of a sheet must not lose their place on the page behind it.
	 *
	 * It focuses the dialog itself rather than a control, because this sheet
	 * has no field to type into and landing on the first row would put the
	 * caret on a choice that changes the recipe.
	 */
	const focuses: Attachment<HTMLElement> = (node) => {
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
	aria-label={m.recipe_language_title()}
	tabindex="-1"
	{@attach focuses}
>
	<div class="border-b border-rule px-gutter py-3">
		<p class="text-label text-support-2 uppercase">{m.recipe_language_title()}</p>
		<!--
			Said once, before anything is tapped: this is the sheet that changes
			the recipe, and the Thread is where that shows up.
		-->
		<p class="mt-1 text-read text-ink-2">{m.recipe_language_what()}</p>
	</div>

	<div class="min-h-0 flex-1 overflow-y-auto">
		{#if refused}
			<p class="px-gutter pt-3 text-read text-support" role="alert">{refused}</p>
		{/if}

		<ul>
			{#each BRANCH_LANGUAGES as each (each)}
				{@const here = each === language}
				{@const barred = isUnknown(each) && inAFamily}
				<li>
					<button
						type="button"
						disabled={here || barred || saying !== null}
						aria-current={here ? 'true' : undefined}
						onclick={() => say(each)}
						class="flex min-h-12 w-full items-center gap-3 border-b border-rule px-gutter py-2
						text-left disabled:opacity-60"
					>
						<span class="min-w-0 flex-1">
							<span class="block text-line {barred ? 'text-ink-2' : 'text-ink'}">
								{languageName(each)}
							</span>
							{#if isUnknown(each)}
								<!--
									What Unknown is for, said where it is chosen and
									nowhere else. A sentence about bilingual recipes
									on every recipe page forever would be a lecture;
									here it is the one place somebody is deciding.
								-->
								<span class="mt-1 block text-read text-ink-2">
									{barred ? m.recipe_language_unknown_not_here() : m.recipe_language_unknown_why()}
								</span>
							{/if}
						</span>
						{#if here}
							<span class="shrink-0 text-label text-accent uppercase">
								{m.recipe_language_this_one()}
							</span>
						{/if}
					</button>
				</li>
			{/each}
		</ul>

		{#if translatable.length > 0}
			<div class="px-gutter pt-6">
				<p class="text-label text-support-2 uppercase">{m.recipe_language_translate()}</p>
				<p class="mt-1 text-read text-ink-2">{m.recipe_language_translate_why()}</p>
			</div>
			<!--
				The rows say *Into French*, not *French*. The list above already
				has a row called French — the one that says what this recipe is
				written in — and two buttons with the same name in one dialog
				are two buttons a screen reader cannot tell apart, whatever the
				heading above them says.
			-->
			<ul class="mt-3">
				{#each translatable as into (into)}
					<li>
						<button
							type="button"
							disabled={saying !== null}
							onclick={() => onTranslate(into)}
							class="flex min-h-12 w-full items-center border-b border-rule px-gutter py-2
							text-left text-line text-accent disabled:opacity-60"
						>
							{m.recipe_language_translate_into({ language: languageName(into) })}
						</button>
					</li>
				{/each}
			</ul>
		{/if}
	</div>

	<button
		type="button"
		class="block w-full border-t border-rule p-4 text-center font-display text-body text-accent"
		onclick={onClose}
	>
		{m.recipe_language_done()}
	</button>
</div>
