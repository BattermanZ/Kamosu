<!--
	What Kamosu made of a pasted recipe, before anything is filled in (#94):
	the counts, the split drawn, and the split moveable.

	THE GUESS IS NEVER APPLIED SILENTLY. Measured on the real 86-recipe export,
	the boundary lands exactly on 77.5% of recipes and within one line on 95%.
	The 5% is why this exists at all: it names the counts, draws the split,
	and lets it be moved. A parser right four times in five and silent the
	fifth is worse than one that says so.

	Moving the split re-splits an answer already held. No line is read a second
	time and no request is made. It is off by one line on 95%, so the ordinary
	correction is one line (hence the two buttons) and the occasional one is a
	long way (hence every line being able to take the split itself).

	Two places draw it (#175): the writing screen's paste sheet, and the sheet
	the + on Recipes raises, for pasted text or a PDF (#176). Each says its own title line and its own buttons
	around it, because one fills a page and the other makes a recipe.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import type { ReadPastedRecipeOutput } from '$lib/api/catalogue';

	interface Props {
		pasted: ReadPastedRecipeOutput;
		/** Where the method starts: seeded from the answer, then moved by hand. */
		boundary: number;
	}

	let { pasted, boundary = $bindable() }: Props = $props();

	const above = $derived(pasted.lines.slice(0, boundary));
	const below = $derived(pasted.lines.slice(boundary));
	/** The counts it names. A Section is neither an ingredient nor a step. */
	const countOf = (rows: { kind: string }[]) => rows.filter((row) => row.kind === 'line').length;
	const sections = $derived(pasted.lines.filter((row) => row.kind === 'section').length);

	const QUIET = 'rounded-sm border border-rule px-2 py-1 text-read text-ink-2';
</script>

<p class="mt-1 text-read text-ink-2">
	{m.write_paste_made({ ingredients: countOf(above), steps: countOf(below) })}
	{#if sections > 0}
		{m.write_paste_headings({ headings: sections })}
	{/if}
</p>

<div class="mt-3 flex items-center gap-2">
	<span class="flex-1 text-label text-ink-2 uppercase">{m.write_paste_boundary()}</span>
	<button
		type="button"
		class={QUIET}
		disabled={boundary === 0}
		onclick={() => (boundary = Math.max(0, boundary - 1))}
	>
		{m.write_paste_earlier()}
	</button>
	<button
		type="button"
		class={QUIET}
		disabled={boundary >= pasted.lines.length}
		onclick={() => (boundary = Math.min(pasted.lines.length, boundary + 1))}
	>
		{m.write_paste_later()}
	</button>
</div>

<!--
	Every line, in the order it was pasted, on the side it landed. Each one
	takes the split itself, so a boundary eight lines out is one tap rather
	than eight, and the row is a real button, so it is reachable from a
	keyboard.
-->
<h3 class="mt-4 font-display text-label font-semibold text-accent uppercase">
	{m.recipe_ingredients()}
</h3>
{@render rows(above, 0, m.write_empty_ingredients())}
<h3
	class="mt-3 border-t border-rule pt-3 font-display text-label font-semibold text-accent uppercase"
>
	{m.recipe_method()}
</h3>
{@render rows(below, boundary, m.write_empty_steps())}

<!--
	What the paste said about the recipe rather than in it (#176): a
	description, serving suggestions. It is in neither list, so it is shown
	where it will land, and moving the split leaves it where it is.
-->
{#if pasted.note}
	<h3
		class="mt-3 border-t border-rule pt-3 font-display text-label font-semibold text-accent uppercase"
	>
		{m.write_paste_note()}
	</h3>
	<p class="py-1 text-read whitespace-pre-wrap text-ink-2">{pasted.note}</p>
{/if}

{#snippet rows(lines: ReadPastedRecipeOutput['lines'], from: number, empty: string)}
	{#if lines.length === 0}
		<p class="py-1 text-read text-ink-2">{empty}</p>
	{/if}
	<ul>
		{#each lines as row, index (from + index)}
			<li>
				<button
					type="button"
					aria-label={m.write_paste_start_here()}
					class="block w-full border-b border-rule py-1 text-start {row.kind === 'section'
						? 'text-label text-ink-2 uppercase'
						: 'text-line text-ink'}"
					onclick={() => (boundary = from + index)}
				>
					{row.text}
				</button>
			</li>
		{/each}
	</ul>
{/snippet}
