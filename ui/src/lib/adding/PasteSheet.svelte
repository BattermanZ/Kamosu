<!--
	What Kamosu made of a whole recipe, checked before it is made (#175): the
	title in a field of its own, the split drawn by `PasteCheck`, and "Make
	this recipe".

	Two things raise it: text pasted into the + (`PasteForm`), and a PDF chosen
	from it (#176), which `read_recipe_pdf` answers in the very shape a paste
	gets. Neither saves anything until "Make this recipe" makes the recipe from
	its title and opens its writing screen with the lines filled in and unsaved.

	A recipe needs a title (#6), and a paste that carried none must be given
	one here, before the recipe exists.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import type { ReadPastedRecipeOutput } from '$lib/api/catalogue';
	import PasteCheck from '$lib/PasteCheck.svelte';
	import { drafted } from '$lib/pasted.svelte';
	import type { Adding } from './adding.svelte';

	interface Props {
		adding: Adding;
		/** What the reader made of it. */
		pasted: ReadPastedRecipeOutput;
		/** The line over the sheet: which source this came from. */
		heading: string;
		/** Back to where it was raised from, making nothing. */
		onback: () => void;
	}

	let { adding, pasted, heading, onback }: Props = $props();

	const uid = $props.id();

	// Seeded once from the answer, then moved and typed by hand. A new answer
	// is a new sheet: whoever raises it draws it afresh.
	// svelte-ignore state_referenced_locally
	let boundary = $state(pasted.boundary);
	// svelte-ignore state_referenced_locally
	let title = $state(pasted.title?.trim() ?? '');
</script>

<div class="fixed inset-0 z-40 bg-accent/40"></div>
<div
	class="fixed inset-x-0 bottom-0 z-50 mx-auto max-h-[85vh] max-w-2xl overflow-y-auto bg-ground px-gutter pt-4 pb-safe"
	role="dialog"
	aria-modal="true"
	aria-labelledby="{uid}-called"
>
	<p id="{uid}-called" class="text-label text-ink-2 uppercase">{heading}</p>
	<label class="mt-3 block">
		<span class="block text-label text-ink-2 uppercase">{m.write_title_label()}</span>
		<input
			bind:value={title}
			required
			class="mt-1 min-h-12 w-full rounded-sm border border-rule bg-card px-3 text-body text-ink"
		/>
	</label>
	{#if (pasted.title ?? '').trim() === ''}
		<p class="mt-1 text-read text-ink-2">{m.plus_paste_no_title()}</p>
	{/if}
	<PasteCheck {pasted} bind:boundary />
	<p class="mt-4 text-read text-ink-2">{m.write_paste_fresh()}</p>
	<button
		type="button"
		class="mt-3 block w-full bg-accent p-4 text-center font-display text-body text-on-accent disabled:opacity-60"
		disabled={title.trim() === '' || adding.working !== null}
		onclick={() => adding.paste(title, drafted(pasted, boundary))}
	>
		{m.plus_paste_make()}
	</button>
	{#if adding.failed}
		<p class="mt-2 text-read text-support" role="alert">{adding.failed}</p>
	{/if}
	<button
		type="button"
		class="mt-2 block w-full border border-rule p-4 text-center font-display text-body text-ink-2"
		onclick={onback}
	>
		{m.write_back()}
	</button>
</div>
