<!--
	What a place big enough for it wears while something is held over it
	(ADR 0044): a sheet of the paper with a dashed frame, and in the middle a
	drawing, a headline saying what letting go will do, and a sentence under
	it. This was chosen for Recipes on 7 October 2026 (#204, option 1) and
	kept unchanged for a recipe's photograph (#205).

	The one who draws it says where it lies with `class`: over the window, or
	over one place. Nothing here can be pressed, so the drag goes on reaching
	what is beneath.

	The frame's 3px, the 72px card, its 40px drawing and the 560px the words
	are held to are the prototype's, as chosen; the stylesheet has no token
	for any of them and nothing else is drawn at these sizes.
-->
<script lang="ts">
	interface Props {
		/** The path of a 24 by 24 line drawing. */
		icon: string;
		headline: string;
		sentence?: string | null;
		/** What is held does not go here: the alert colour. */
		refuses?: boolean;
		class: string;
	}

	let { icon, headline, sentence = null, refuses = false, class: where }: Props = $props();
</script>

<div
	role="status"
	class="pointer-events-none flex items-center justify-center bg-ground/90 p-8 {where} {refuses
		? 'text-support'
		: 'text-accent'}"
>
	<div class="absolute inset-4 rounded-sm border-3 border-dashed border-current"></div>
	<div class="relative max-w-[560px] rounded-sm bg-ground px-8 py-6 text-center">
		<span
			class="mx-auto mb-4 flex h-[72px] w-[72px] items-center justify-center rounded-sm border border-rule bg-card"
		>
			<svg
				viewBox="0 0 24 24"
				aria-hidden="true"
				class="h-[40px] w-[40px]"
				fill="none"
				stroke="currentColor"
				stroke-width="1.5"
				stroke-linecap="round"
				stroke-linejoin="round"><path d={icon} /></svg
			>
		</span>
		<p class="font-display text-title font-semibold">{headline}</p>
		{#if sentence}
			<p class="mt-2 text-body text-ink-2">{sentence}</p>
		{/if}
	</div>
</div>
