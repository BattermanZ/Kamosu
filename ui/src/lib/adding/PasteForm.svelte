<!--
	A whole recipe pasted as text, read, checked, and made (#175).

	Aurélien put this in the + beside the Recipes search box as a fourth source,
	because you usually hold the text before any recipe page exists: making a
	title up first, only for the paste to replace it, was the long way round.
	It opens under the search row like From a link does (ADR 0027).

	Reading asks `read_pasted_recipe`, which saves nothing. What it made is
	shown in `PasteSheet`, the sheet a PDF chosen from the + raises too (#176).
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import type { ReadPastedRecipeOutput } from '$lib/api/catalogue';
	import { readPasted } from '$lib/pasted.svelte';
	import type { Adding } from './adding.svelte';
	import PasteSheet from './PasteSheet.svelte';

	interface Props {
		adding: Adding;
	}

	let { adding }: Props = $props();

	const kamosu = useKamosu();

	let text = $state('');
	let reading = $state(false);
	let failed = $state<string | undefined>(undefined);
	/** What came back, or nothing while the text has not been read yet. */
	let pasted = $state<ReadPastedRecipeOutput | null>(null);

	async function read() {
		reading = true;
		failed = undefined;
		try {
			const answer = await readPasted(kamosu, text);
			if (typeof answer === 'string') {
				failed = answer;
			} else {
				pasted = answer;
			}
		} finally {
			reading = false;
		}
	}
</script>

<form
	class="grid gap-2"
	onsubmit={(event) => {
		event.preventDefault();
		read();
	}}
>
	<p class="text-read text-ink-2">{m.plus_paste_hint()}</p>
	<textarea
		bind:value={text}
		rows="8"
		aria-label={m.write_paste_label()}
		class="block w-full rounded-sm border border-rule bg-card p-2 text-body text-ink"></textarea>
	{#if failed}
		<p class="text-read text-support" role="alert">{failed}</p>
	{/if}
	<button
		class="min-h-12 rounded-sm bg-accent px-4 py-3 font-display text-body text-on-accent disabled:opacity-60"
		disabled={reading || adding.working !== null}
	>
		{reading ? m.write_paste_reading() : m.write_paste_read()}
	</button>
</form>

{#if pasted}
	<PasteSheet {adding} {pasted} heading={m.plus_paste()} onback={() => (pasted = null)} />
{/if}
