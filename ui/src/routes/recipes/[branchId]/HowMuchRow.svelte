<!--
	HOW MUCH, for the errands (#109). Under the Ingredients heading because it
	is a fact about every line beneath it, and a row rather than a control in
	the meta strip because a third of the library has no Yield to put a control
	on. In your own recipe only, and not while a Divergence is shown: see
	`pageScaledTo` on the recipe screen, which also holds the reading at
	another amount.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import HowMuch from '$lib/HowMuch.svelte';
	import { said, type Wanted, type Written } from '$lib/how-much';

	interface Props {
		written: Written;
		/** What the page is scaled to, or null where it is as written. */
		scaledTo: Wanted;
		/** The picker is open. */
		choosing: boolean;
		/** The last change of how much could not be read — no network, usually. */
		failed: boolean;
		onchoose: (wanted: Wanted) => void;
	}

	let { written, scaledTo, choosing = $bindable(), failed, onchoose }: Props = $props();
</script>

<div class="mx-gutter mb-2">
	<div class="flex items-center justify-between gap-3">
		<p class="min-w-0 text-read {scaledTo ? 'font-semibold text-accent' : 'text-ink-2'}">
			{scaledTo
				? m.recipe_how_much({ amount: said(scaledTo) ?? '' })
				: written
					? m.recipe_how_much({ amount: said(written) ?? '' })
					: m.recipe_how_much_as_written()}
		</p>
		<button
			type="button"
			class="tap-out h-8 shrink-0 text-read text-accent underline"
			aria-expanded={choosing}
			onclick={() => (choosing = !choosing)}
		>
			{choosing ? m.recipe_how_much_done() : m.recipe_how_much_change()}
		</button>
	</div>
	{#if choosing}
		<div class="mt-2 border-y border-rule py-3">
			<HowMuch {written} wanted={scaledTo} room="page" {onchoose} />
		</div>
	{/if}
	{#if failed}
		<p class="mt-2 text-read text-support" role="alert">{m.recipe_how_much_offline()}</p>
	{/if}
</div>
