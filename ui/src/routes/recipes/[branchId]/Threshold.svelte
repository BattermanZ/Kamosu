<!--
	The switch between two whole recipes — and it is deliberately not a control.

	ADR 0014's argument is that a recipe is a thing you stand IN. A pair of tabs
	would have said the opposite: two views of one object, which is the single
	reconciled recipe ADR 0004 refuses to imply. So this is a threshold. A band
	names the Kitchen you are standing in, one wide target says where it goes,
	and the page's paper and rules change with the room, so six screens down a
	long scroll you still know whose recipe you are reading.

	Under it sits the one control that governs the marking. It belongs here
	because this strip already answers "whose recipe is this"; putting the
	divergence away is the same question answered differently.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';

	interface Props {
		hereKitchen: string;
		thereKitchen: string;
		/** How many lines the two Branches do not share, marked or not. */
		unshared: number;
		marks: boolean;
		cross: () => void;
		toggleMarks: () => void;
	}

	let { hereKitchen, thereKitchen, unshared, marks, cross, toggleMarks }: Props = $props();
</script>

<div class="threshold flex items-stretch bg-[var(--whose)] text-on-accent">
	<div class="min-w-0 flex-1 px-gutter py-3">
		<p class="text-label uppercase opacity-60">{m.divergence_you_are_in()}</p>
		<p class="truncate font-display text-line font-semibold">{hereKitchen}</p>
	</div>
	<button
		type="button"
		onclick={cross}
		class="max-w-[58%] border-l border-on-accent/25 py-3 pr-gutter pl-4 text-right"
	>
		<span class="block text-label uppercase opacity-60">{m.divergence_cross_to()}</span>
		<span class="block truncate font-display text-line font-semibold">{thereKitchen} →</span>
	</button>
</div>

<div class="flex items-center gap-3 border-b border-rule bg-ground px-gutter py-2">
	<p class="flex-1 text-read text-ink-2">
		{#if !marks}
			{m.divergence_put_away({ count: unshared })}
		{:else if unshared === 1}
			{m.divergence_unshared_one({ kitchen: thereKitchen })}
		{:else}
			{m.divergence_unshared({ count: unshared, kitchen: thereKitchen })}
		{/if}
	</p>
	<button
		type="button"
		onclick={toggleMarks}
		class="rounded-sm border border-rule bg-card px-3 py-1 text-read text-accent"
	>
		{marks ? m.divergence_hide() : m.divergence_show()}
	</button>
</div>

<style>
	/* The noren: a split curtain cut into the band's own bottom edge, so it
	   hangs below the band without ever sitting over the page. */
	.threshold {
		padding-bottom: 8px;
		-webkit-mask-image:
			linear-gradient(#000 0 0),
			repeating-linear-gradient(90deg, #000 0 26px, transparent 26px 32px);
		-webkit-mask-size:
			100% calc(100% - 8px),
			100% 8px;
		-webkit-mask-position:
			0 0,
			0 100%;
		-webkit-mask-repeat: no-repeat;
		mask-image:
			linear-gradient(#000 0 0),
			repeating-linear-gradient(90deg, #000 0 26px, transparent 26px 32px);
		mask-size:
			100% calc(100% - 8px),
			100% 8px;
		mask-position:
			0 0,
			0 100%;
		mask-repeat: no-repeat;
	}
</style>
