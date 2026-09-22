<!--
	The one way anybody is asked to confirm something that cannot be undone
	(#103).

	It was written for the Operator's three acts and lived under `lib/operator/`
	until #120 gave a cook one of their own: deleting a recipe. It moved up here
	rather than being copied down, for the reason the next paragraph already
	gives — a second sheet is a second chance to word the frightening one
	carelessly, and the cook's is the most frightening of the four.

	Item 99 asks a Merge to say how many Ingredient Lines it is about to move
	before it moves them. The choice of 21 September 2026 took that pattern and
	gave it to the other two irreversible acts as well, so deleting an account
	and deleting a Food are asked in the same words and the same shape as a
	Merge. One component, because three confirmations written separately are
	three chances to word the frightening one carelessly.

	The count leads, in the largest type on the sheet, because the number is the
	thing a person is actually deciding about — "14 Ingredient Lines move" is a
	fact you can weigh, where "are you sure?" is not. `count` is optional: an act
	whose size is not a number (sweeping, standing somebody down) says its
	consequence in words alone rather than inventing a figure to look consistent.

	Deliberately NOT hold-to-confirm, which the mockup carried and the choice
	dropped: holding a button for a second is hard for anyone whose hands shake,
	and the sentence above it is doing the real work.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import type { Snippet } from 'svelte';

	interface Props {
		/** What is about to happen, as a title. */
		title: string;
		/** The figure this act is about, where it has one. */
		count?: number;
		/** What that figure counts — "Ingredient Lines move", "Kitchens left". */
		counting?: string;
		/** The consequence, said plainly. Always present. */
		consequence: string;
		/** The word on the button that does it. */
		act: string;
		/** A gentler alternative, offered beside the irreversible one. */
		instead?: { label: string; run: () => void };
		/** Whether the act is running, so the sheet cannot be fired twice. */
		busy?: boolean;
		/** A refusal that arrived, shown in the words it came in. */
		failed?: string;
		run: () => void;
		cancel: () => void;
		children?: Snippet;
	}

	let {
		title,
		count,
		counting,
		consequence,
		act,
		instead,
		busy = false,
		failed,
		run,
		cancel,
		children,
	}: Props = $props();

	/** Escape closes it, as it closes every other sheet in Kamosu. */
	function onKey(event: KeyboardEvent) {
		if (event.key === 'Escape' && !busy) cancel();
	}
</script>

<svelte:window onkeydown={onKey} />

<div class="fixed inset-0 z-40 bg-accent/40"></div>
<div
	class="fixed inset-x-0 bottom-0 z-50 mx-auto max-h-[78vh] max-w-2xl overflow-y-auto border-t
	border-rule bg-ground px-gutter pt-6 pb-safe"
	role="dialog"
	aria-modal="true"
	aria-labelledby="confirm-title"
>
	<h2 id="confirm-title" class="font-display text-list-title font-semibold text-ink">{title}</h2>

	{#if count !== undefined && counting}
		<p class="mt-3 flex items-baseline gap-3">
			<span class="font-display text-title font-semibold text-ink">{count}</span>
			<span class="flex-1 text-body text-ink-2">{counting}</span>
		</p>
	{/if}

	<p class="mt-3 border-l-2 border-support pl-3 text-body text-ink-2">{consequence}</p>

	{#if children}
		<div class="mt-3">{@render children()}</div>
	{/if}

	{#if failed}
		<p class="mt-3 text-body text-support" role="alert">{failed}</p>
	{/if}

	<div class="mt-6 mb-4 flex flex-col gap-2">
		<button
			type="button"
			onclick={run}
			disabled={busy}
			class="min-h-12 rounded-sm border border-support bg-support px-4 text-body font-medium
			text-card disabled:opacity-60"
		>
			{busy ? m.operator_working() : act}
		</button>
		{#if instead}
			<button
				type="button"
				onclick={instead.run}
				disabled={busy}
				class="min-h-12 rounded-sm border border-accent px-4 text-body font-medium text-accent
				disabled:opacity-60"
			>
				{instead.label}
			</button>
		{/if}
		<button
			type="button"
			onclick={cancel}
			disabled={busy}
			class="min-h-12 rounded-sm px-4 text-body font-medium text-accent disabled:opacity-60"
		>
			{m.operator_cancel()}
		</button>
	</div>
</div>
