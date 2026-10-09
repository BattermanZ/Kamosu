<!--
	One "not right now" card (#76, option C, chosen 19 September 2026).

	Kamosu says a thing once, in sentences, at the top of the page, and the
	person puts it away. Four tones share this one shape so they read as one
	voice: a rule along the top edge in beni for the warning (plain http://) and
	in indigo for the pause (the library waiting), and none for the rest.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Action {
		label: string;
		act: () => void;
		/** The one to reach for, drawn in indigo. */
		primary?: boolean;
	}

	interface Props {
		title: string;
		tone?: 'plain' | 'warning' | 'pause';
		actions?: Action[];
		children: Snippet;
	}

	let { title, tone = 'plain', actions = [], children }: Props = $props();
</script>

<!-- On the wide layout the card sits in the page's column rather than running
     the window's width with two enormous buttons (#226). -->
<section
	class="mx-gutter my-4 border border-rule bg-card p-4 text-read text-ink wide:mx-auto wide:w-[calc(100%-2*var(--spacing-gutter))] wide:max-w-2xl
	{tone === 'warning' ? 'border-t-3 border-t-support' : ''}
	{tone === 'pause' ? 'border-t-3 border-t-accent' : ''}"
	aria-label={title}
	role={tone === 'warning' ? 'alert' : 'status'}
>
	<h2 class="mb-1 font-display text-line font-semibold {tone === 'warning' ? 'text-support' : ''}">
		{title}
	</h2>
	{@render children()}
	{#if actions.length > 0}
		<div class="mt-3 flex gap-2">
			{#each actions as action (action.label)}
				<button
					type="button"
					onclick={action.act}
					class="flex-1 border p-3 text-read wide:max-w-[240px]
					{action.primary ? 'border-accent bg-accent text-on-accent' : 'border-rule text-accent'}"
				>
					{action.label}
				</button>
			{/each}
		</div>
	{/if}
</section>
