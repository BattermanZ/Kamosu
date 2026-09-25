<!--
	One line the two Branches do not share — an Ingredient Line or a Step, and
	the same component for both, because a Ghost is one mechanism seen from two
	sides rather than two mechanisms.

	At rest it says in words what it is. A struck-through line with nothing
	beside it reads like something crossed off a shopping list rather than a real
	line of a real recipe, so every marked line and every Ghost is named.

	Tapped, it unfolds the other side IN PLACE — never a second column, never a
	second screen — and offers to carry the line across into your recipe.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import StepPhoto from '$lib/StepPhoto.svelte';
	import StepWords from '$lib/StepWords.svelte';
	import type { StepConversions } from '$lib/step-conversions';
	import {
		see,
		caption,
		sentence,
		offer,
		carriedInPlace,
		type Row,
		type Side,
		type Taken,
	} from './divergence';

	interface Props {
		row: Row;
		side: Side;
		/** Always the Kitchen you are NOT in: every caption is written about them. */
		otherKitchen: string;
		/** A Step's number, or null for a Ghost step — it is not a step of this recipe. */
		number?: number | null;
		/**
		 * The one subordinate line under this row, already written out — the
		 * converted amount where there is one, what Kamosu read where there is
		 * not (#49). A Step's conversions sit inside its words instead (#150).
		 * One slot, never two. Never shown on a Ghost: it isn't your line.
		 */
		beneath?: string;
		/**
		 * A Step's conversions — its oven and its amounts — each drawn after
		 * what it converts (#150). Never on a Ghost, a struck line or one
		 * carried across, whose words are not the ones these figures describe.
		 */
		conversions?: StepConversions;
		open: boolean;
		taken: Taken | undefined;
		onToggle: () => void;
		onCarry: (what: Taken | 'undo') => void;
		/**
		 * Offered inside the unfolded panel when this row is a line of YOUR
		 * recipe — a marked row's tap already means "show me the other side", so
		 * correcting what Kamosu read of it has to be a second target rather
		 * than a second meaning on the first. A Ghost gets none: it is not your
		 * line, so there is no Reading of yours to correct.
		 */
		onFixReading?: (() => void) | undefined;
		/**
		 * This Step's photograph in YOUR recipe, where it has one (#110). Beside
		 * the row rather than inside it, because the row is already a button.
		 */
		photo?: string | null;
	}

	let {
		row,
		side,
		otherKitchen,
		number = undefined,
		beneath = '',
		conversions = [],
		open,
		taken,
		onToggle,
		onCarry,
		onFixReading = undefined,
		photo = null,
	}: Props = $props();

	const seen = $derived(see(row, side));
	const isStep = $derived(row.kind === 'step');

	// A line you have carried across is written into YOUR recipe at once and
	// left unsaved — so standing in your own recipe you read the new words in
	// place, with what they replaced underneath (ADR 0014). Standing in theirs,
	// nothing moves: carrying changes nothing about their Branch.
	const carried = $derived(side === 'mine' ? carriedInPlace(row, taken) : null);
	const shown = $derived(carried?.text ?? seen.own?.text ?? seen.other?.text ?? '');
	const struck = $derived(carried ? carried.leaving : seen.ghost);
	const what = $derived(offer(row, taken));
</script>

<li class="border-b border-rule py-3 pl-3 {struck ? 'mark-ghost' : 'mark-diff'}">
	<div class="flex gap-3">
		<button type="button" class="flex min-w-0 flex-1 gap-3 text-left" onclick={onToggle}>
			{#if isStep}
				<span class="w-6 shrink-0 font-display text-line font-semibold text-accent">
					{number ?? '·'}
				</span>
			{:else}
				<!--
				The same marker an unmarked Ingredient Line carries (#81), so the
				two kinds of row keep one rhythm and their text sits on one
				left edge. A Ghost keeps it too: it is still a line of a list,
				and the strike and the caption are what say whose.
			-->
				<span class="ingredient-marker shrink-0" aria-hidden="true"></span>
			{/if}
			<span class="min-w-0 flex-1">
				<span class="block {isStep ? 'text-body' : 'text-line'} {struck ? 'ghost-text' : ''}">
					<StepWords
						text={shown}
						conversions={struck || carried ? [] : conversions}
						readingClass="text-read text-ink-2"
					/>
				</span>
				{#if beneath && !struck && !carried}
					<span class="block text-read text-ink-2">{beneath}</span>
				{/if}
				{#if carried?.replaced}
					<span class="block text-read text-ink-2 line-through">
						{m.divergence_replacing({ text: carried.replaced })}
					</span>
				{/if}
				<span class="mt-1 block text-label uppercase {struck ? 'text-support' : 'text-accent'}">
					{#if carried}
						{carried.leaving ? m.divergence_leaving() : m.divergence_carried()}
					{:else}
						{caption(seen, side, otherKitchen)}
					{/if}
				</span>
			</span>
		</button>
		{#if isStep && photo && number && !struck}
			<StepPhoto
				photograph={photo}
				{number}
				shapeClass="h-[var(--photo-thumb)] w-[var(--photo-thumb)]"
			/>
		{/if}
	</div>

	{#if open}
		<div class="mt-2 border border-rule bg-card p-3">
			<p class="text-label text-ink-2 uppercase">
				{#if seen.ghost}
					{m.divergence_what_happened()}
				{:else if side === 'theirs'}
					{m.divergence_your_line()}
				{:else}
					{m.divergence_their_line({ kitchen: otherKitchen })}
				{/if}
			</p>
			<p class="mt-1 text-body">{sentence(seen, side, otherKitchen)}</p>
			{#if what}
				<button
					type="button"
					onclick={() => onCarry(what)}
					class="mt-3 block w-full p-2 text-center text-read {what === 'undo'
						? 'border border-rule text-ink-2'
						: 'bg-accent text-on-accent'}"
				>
					{#if what === 'undo'}
						{m.divergence_untake()}
					{:else if what === 'remove'}
						{m.divergence_take_remove()}
					{:else}
						{m.divergence_take_write()}
					{/if}
				</button>
			{:else}
				<p class="mt-2 text-read text-ink-2">{m.divergence_nothing_to_carry()}</p>
			{/if}

			<!--
				The Reading is corrected from wherever the line is read (#81), and
				a marked line is still a line of the recipe. It sits under the
				carry offer because the two are different questions: one is about
				the other Kitchen's words, this one is about Kamosu's own.
			-->
			{#if onFixReading}
				<button
					type="button"
					onclick={onFixReading}
					class="mt-2 block w-full border border-rule p-2 text-center text-read text-accent"
				>
					{m.reading_fix()}
				</button>
			{/if}
		</div>
	{/if}
</li>

<style>
	/* A marked line carries a rule in the margin, in the colour of the room you
	   are standing in; a Ghost carries the other room's colour, because it is a
	   line of the other recipe. `--whose` is set by the screen. */
	.mark-diff,
	.mark-ghost {
		position: relative;
	}
	.mark-diff::before,
	.mark-ghost::before {
		content: '';
		position: absolute;
		left: 0;
		top: 0.5rem;
		bottom: 0.5rem;
		width: 2px;
		background: var(--whose);
	}
	.mark-ghost::before {
		background: var(--color-support);
	}
	:global([data-side='theirs']) .mark-ghost::before {
		background: var(--color-accent);
	}

	/* The marker's shape is the `ingredient-marker` utility, shared with the
	   unmarked rows so one list keeps one rhythm. Only its colour is this
	   component's: the room you are standing in, or — on a Ghost — the other
	   room's, because a Ghost is a line of the other recipe. */
	.ingredient-marker {
		background: var(--whose);
	}
	.mark-ghost .ingredient-marker {
		background: var(--color-support);
	}
	:global([data-side='theirs']) .mark-ghost .ingredient-marker {
		background: var(--color-accent);
	}

	/* The Ghost itself. Struck through and dimmed, so it is plainly not part of
	   the list you would shop from — and never without its caption beside it. */
	.ghost-text {
		text-decoration: line-through;
		text-decoration-thickness: 1px;
		color: var(--color-ink-2);
	}
</style>
