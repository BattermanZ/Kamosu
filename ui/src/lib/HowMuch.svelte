<!--
	HOW MUCH ARE YOU MAKING (#109) — one picker, in two rooms.

	The cooking screen asks it before a fresh cooking starts, in the place the
	amounts and the Step will stand; the recipe page opens it under the
	ingredients' heading, for the errands. Aurélien chose that shape on
	23 September 2026 from three built against the real corpus (option B on
	#109); the reasoning is there.

	− and + count in the recipe's own noun, one serving at a time. The four
	quick choices are ×½ · as written · ×2 · ×3, each labelled with what it comes
	to where it counts (`×2 · 24`). A recipe that never said what it makes —
	a third of the real library — has nothing to count from, so it gets the
	multipliers alone and a sentence saying why.

	It chooses and says nothing else. What a choice does to the amounts comes
	back from the Core (ADR 0016), and whoever holds this decides when to ask.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import {
		choices,
		counts,
		isMultiplier,
		said,
		same,
		stepped,
		type Wanted,
		type Written,
	} from '$lib/how-much';

	interface Props {
		written: Written;
		wanted: Wanted;
		onchoose: (wanted: Wanted) => void;
		/** The indigo room at the stove, or the paper of every other screen. */
		room: 'cook' | 'page';
	}

	let { written, wanted, onchoose, room }: Props = $props();

	const quick = $derived(choices(written));
	const fewer = $derived(stepped(written, wanted, -1));
	const more = $derived(stepped(written, wanted, 1));
	/** What is chosen, counted in the recipe's noun: the number between − and +. */
	const shown = $derived(
		wanted && written && wanted.noun === written.noun ? wanted.amount : (written?.amount ?? ''),
	);
	const cook = $derived(room === 'cook');
</script>

<p class="text-label uppercase {cook ? 'text-cook-ink-2' : 'text-ink-2'}">
	{m.how_much_question()}
</p>
{#if counts(written) && written}
	<div class="flex items-center gap-4 py-3">
		<button
			type="button"
			class="min-h-12 w-12 rounded-sm border font-display text-title disabled:opacity-40
				{cook ? 'border-cook-accent text-cook-accent' : 'border-accent text-accent'}"
			disabled={fewer === undefined}
			aria-label={m.how_much_fewer()}
			onclick={() => fewer !== undefined && onchoose(fewer)}
		>
			−
		</button>
		<p class="min-w-0 flex-1 text-center font-display text-title font-semibold" aria-live="polite">
			{isMultiplier(wanted) ? said(wanted) : shown}
			<span class="text-body font-normal {cook ? 'text-cook-ink-2' : 'text-ink-2'}">
				{isMultiplier(wanted) ? '' : written.noun}
			</span>
		</p>
		<button
			type="button"
			class="min-h-12 w-12 rounded-sm border font-display text-title disabled:opacity-40
				{cook ? 'border-cook-accent text-cook-accent' : 'border-accent text-accent'}"
			disabled={more === undefined}
			aria-label={m.how_much_more()}
			onclick={() => more !== undefined && onchoose(more)}
		>
			+
		</button>
	</div>
{:else}
	<p class="py-3 text-read {cook ? 'text-cook-ink-2' : 'text-ink-2'}">
		<!--
			Nothing to count from: no Yield at all, or one like `a dozen` or
			`1½` that a whole serving cannot be stepped off. The second is not
			the recipe saying nothing, and is not told it is.
		-->
		{written ? m.how_much_multiply_only() : m.how_much_says_nothing()}
	</p>
{/if}
<div class="flex flex-wrap gap-2">
	{#each quick as choice (choice.times)}
		{@const chosen = same(choice.wanted, wanted)}
		<button
			type="button"
			aria-pressed={chosen}
			class="min-h-12 rounded-sm border px-4 text-read font-semibold
				{cook
				? chosen
					? 'border-cook-accent bg-cook-accent text-cook-on-accent'
					: 'border-cook-rule text-cook-ink'
				: chosen
					? 'border-accent bg-accent text-on-accent'
					: 'border-rule text-ink'}"
			onclick={() => onchoose(choice.wanted)}
		>
			{choice.times ?? m.how_much_as_written()}{choice.count ? ` · ${choice.count}` : ''}
		</button>
	{/each}
</div>
