<!--
	A whole recipe pasted as text, read, checked, and made (#175).

	Aurélien put this in the + beside the Recipes search box as a fourth source,
	because you usually hold the text before any recipe page exists: making a
	title up first, only for the paste to replace it, was the long way round.
	It opens under the search row like From a link does (ADR 0027).

	Reading asks `read_pasted_recipe`, which saves nothing. What it made is
	shown in a sheet, the same check the writing screen draws (`PasteCheck`),
	with the title in a field of its own: a recipe needs one (#6), and a paste
	that carried none must be given one here, before the recipe exists.
	"Make this recipe" then makes it from the title and opens its writing
	screen with the lines filled in and unsaved.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import type { ReadPastedRecipeOutput } from '$lib/api/catalogue';
	import PasteCheck from '$lib/PasteCheck.svelte';
	import { drafted, readPasted } from '$lib/pasted.svelte';
	import type { Adding } from './adding.svelte';

	interface Props {
		adding: Adding;
	}

	let { adding }: Props = $props();

	const kamosu = useKamosu();
	const uid = $props.id();

	let text = $state('');
	let reading = $state(false);
	let failed = $state<string | undefined>(undefined);
	/** What came back, or nothing while the text has not been read yet. */
	let pasted = $state<ReadPastedRecipeOutput | null>(null);
	let boundary = $state(0);
	let title = $state('');

	async function read() {
		reading = true;
		failed = undefined;
		try {
			const answer = await readPasted(kamosu, text);
			if (typeof answer === 'string') {
				failed = answer;
			} else {
				pasted = answer;
				boundary = answer.boundary;
				title = answer.title?.trim() ?? '';
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
	<div class="fixed inset-0 z-40 bg-accent/40"></div>
	<div
		class="fixed inset-x-0 bottom-0 z-50 mx-auto max-h-[85vh] max-w-2xl overflow-y-auto bg-ground px-gutter pt-4 pb-safe"
		role="dialog"
		aria-modal="true"
		aria-labelledby="{uid}-called"
	>
		<p id="{uid}-called" class="text-label text-ink-2 uppercase">{m.plus_paste()}</p>
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
			onclick={() => pasted && adding.paste(title, drafted(pasted, boundary))}
		>
			{m.plus_paste_make()}
		</button>
		{#if adding.failed}
			<p class="mt-2 text-read text-support" role="alert">{adding.failed}</p>
		{/if}
		<button
			type="button"
			class="mt-2 block w-full border border-rule p-4 text-center font-display text-body text-ink-2"
			onclick={() => (pasted = null)}
		>
			{m.write_back()}
		</button>
	</div>
{/if}
