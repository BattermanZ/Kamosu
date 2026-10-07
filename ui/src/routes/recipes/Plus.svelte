<!--
	The + at the end of the search box: one way in for every new recipe (#174).

	Tapping it opens a small list hanging from it with five sources, and each
	one does its thing in place (ADR 0027): a link opens the address field under
	the search row, a Bundle (a Kamosu zip file) opens the phone's own picker
	straight away, a PDF opens it too and raises the sheet a paste is checked
	on (#176, a row of its own by Aurélien's choice on #174), pasted text opens
	a box for it under the search row (#175, Aurélien's choice), and writing one
	asks for a title under the search row. The acts are `$lib/adding`'s, the
	same ones the dead ends below offer.

	**The arrangement is Aurélien's, chosen on 27 September 2026** from two
	options drawn over the real Recipes and Home screens (option 1, recorded on
	#174): beside the search box, a list hanging from the +, Recipes only. It
	replaced the quiet "Add a recipe from a link · from a recipe file" row #93
	put above the shelf, which could not write a recipe at all. Before this the
	only way to write one on a full library was a search that matched nothing,
	and "Chicken curry" in a library holding a chicken dish never does.

	**The + stays when nothing is found**, beside the three big offers there.
	It is small and always in the same place, and hiding it on exactly the
	screen where adding is on your mind would be the stranger choice.

	The list is a disclosure, not an ARIA menu: five ordinary buttons behind a
	button that says whether it is open. A menu role promises arrow-key
	behaviour, and five buttons in the tab order need none of it.

	**Offline all five wait for the server** and say so in their own row,
	greyed, as every act that needs it does (#76). None is queued: the outbox
	keeps Attempts and the shopping list, never a new recipe (ADR 0013). The
	rows are drawn here rather than with `NeedsServer`, whose button holds one
	line of words and no icon.

	**A PDF, a Kamosu zip file or a link can be dropped on Recipes instead**
	(#204), and is the same act as its row here: `DropARecipe` holds no act of
	its own, only this +'s `adding`, and offline it says what the row says.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import { m } from '$lib/paraglide/messages';
	import { Online } from '$lib/offline/device.svelte';
	import { useAdding, type Act } from '$lib/adding/adding.svelte';
	import AddressForm from '$lib/adding/AddressForm.svelte';
	import TitleForm from '$lib/adding/TitleForm.svelte';
	import PasteForm from '$lib/adding/PasteForm.svelte';
	import PasteSheet from '$lib/adding/PasteSheet.svelte';
	import FilePicker from '$lib/adding/FilePicker.svelte';
	import Said from '$lib/adding/Said.svelte';
	import { ICON, WAITS_FOR_SERVER } from '$lib/adding/sources';
	import DropARecipe from './DropARecipe.svelte';

	interface Props {
		/** What sits beside the +: the search field. */
		children: Snippet;
	}

	let { children }: Props = $props();

	const adding = useAdding();
	const online = new Online();
	const listId = $props.id();

	/** Whether the list of sources is showing. */
	let open = $state(false);
	/** The source whose field is revealed under the search row, if any. */
	let showing = $state<Exclude<Act, 'file' | 'pdf'> | null>(null);
	let picker = $state<FilePicker | undefined>(undefined);
	let pdfPicker = $state<FilePicker | undefined>(undefined);
	/** The + and its list, so a tap anywhere else can close the list. */
	let anchor = $state<HTMLDivElement | undefined>(undefined);

	function choose(act: Act) {
		open = false;
		if (act === 'file') {
			showing = null;
			picker?.open();
		} else if (act === 'pdf') {
			showing = null;
			pdfPicker?.open();
		} else {
			showing = act;
		}
	}

	const sources: { act: Act; label: () => string; why: () => string }[] = [
		{ act: 'link', label: m.plus_link, why: m.plus_link_why },
		{ act: 'file', label: m.plus_file, why: m.plus_file_why },
		{ act: 'pdf', label: m.plus_pdf, why: m.plus_pdf_why },
		{ act: 'paste', label: m.plus_paste, why: m.plus_paste_why },
		{ act: 'write', label: m.plus_write, why: m.plus_write_why },
	];

	/**
	 * Something was dropped on Recipes (#204). It is a new choice, as tapping
	 * a row is, so the list and whatever field was open make way for what it
	 * says under the search row.
	 */
	function dropped() {
		open = false;
		showing = null;
	}
</script>

<svelte:window
	onkeydown={(event) => {
		if (open && event.key === 'Escape') open = false;
	}}
	onpointerdown={(event) => {
		if (open && anchor && !anchor.contains(event.target as Node)) open = false;
	}}
/>

<FilePicker {adding} bind:this={picker} />
<FilePicker {adding} kind="pdf" bind:this={pdfPicker} />
<DropARecipe {adding} offline={!online.current} ondrop={dropped} />

<div class="flex items-stretch gap-2">
	<div class="min-w-0 flex-1">{@render children()}</div>

	<div class="relative shrink-0" bind:this={anchor}>
		<button
			type="button"
			aria-label={m.plus_label()}
			aria-expanded={open}
			aria-controls={listId}
			disabled={adding.working !== null}
			onclick={() => {
				open = !open;
				if (open) showing = null;
			}}
			class="flex h-12 w-12 items-center justify-center rounded-sm border disabled:opacity-60 {open ||
			showing
				? 'border-accent bg-card text-accent'
				: 'border-accent bg-accent text-on-accent'}"
		>
			<svg
				viewBox="0 0 24 24"
				aria-hidden="true"
				class="h-6 w-6"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"><path d="M12 5v14M5 12h14" /></svg
			>
		</button>

		{#if open}
			<div
				id={listId}
				class="absolute top-full right-0 z-30 mt-2 w-[18.75rem] max-w-[calc(100vw-2*var(--spacing-gutter))] rounded-sm border border-rule bg-card shadow-[0_8px_24px_color-mix(in_srgb,var(--color-ink)_12%,transparent)]"
			>
				<p class="border-b border-rule px-3 pt-3 pb-2 text-label text-ink-2 uppercase">
					{m.plus_label()}
				</p>
				<ul>
					{#each sources as source (source.act)}
						{@const waits = online.current ? null : WAITS_FOR_SERVER[source.act]}
						<li class="border-b border-rule last:border-b-0">
							<button
								type="button"
								disabled={waits !== null}
								onclick={() => choose(source.act)}
								class="flex min-h-12 w-full items-center gap-3 px-3 py-2 text-left disabled:opacity-55"
							>
								<span
									class="flex h-8 w-8 shrink-0 items-center justify-center rounded-sm border border-rule bg-ground-2 text-accent"
								>
									<svg
										viewBox="0 0 24 24"
										aria-hidden="true"
										class="h-6 w-6"
										fill="none"
										stroke="currentColor"
										stroke-width="1.5"
										stroke-linecap="round"
										stroke-linejoin="round"><path d={ICON[source.act]} /></svg
									>
								</span>
								<span class="min-w-0">
									{#if waits}
										<span class="block text-body text-ink-2">{waits()}</span>
									{:else}
										<span class="block font-display text-body text-accent">{source.label()}</span>
										<span class="block text-read text-ink-2">{source.why()}</span>
									{/if}
								</span>
							</button>
						</li>
					{/each}
				</ul>
			</div>
		{/if}
	</div>
</div>

{#if showing && online.current}
	<div class="mt-3 rounded-sm border border-rule bg-ground-2 p-3">
		<div class="mb-2 flex items-baseline justify-between gap-3">
			<h2 class="text-label text-ink-2 uppercase">
				{sources.find((source) => source.act === showing)?.label()}
			</h2>
			<button type="button" onclick={() => (showing = null)} class="text-read text-accent underline"
				>{m.plus_cancel()}</button
			>
		</div>
		{#if showing === 'link'}
			<AddressForm {adding} />
		{:else if showing === 'paste'}
			<PasteForm {adding} />
		{:else}
			<TitleForm {adding} />
		{/if}
	</div>
{/if}

{#if adding.working === 'file'}
	<p class="mt-2 text-read text-ink-2" role="status">{m.bring_in_file_working()}</p>
{:else if adding.working === 'pdf'}
	<p class="mt-2 text-read text-ink-2" role="status">{m.plus_pdf_working()}</p>
{:else if adding.working === 'link' && showing !== 'link'}
	<!-- A link that was dropped (#204): the address field, whose button says this, is not open. -->
	<p class="mt-2 text-read text-ink-2" role="status">{m.recipes_import_working()}</p>
{/if}

<!-- A PDF, read, is checked on the sheet a paste is checked on (#176). -->
{#if adding.fromPdf}
	{#key adding.fromPdf}
		<PasteSheet
			{adding}
			pasted={adding.fromPdf}
			heading={m.plus_pdf()}
			onback={() => (adding.fromPdf = null)}
		/>
	{/key}
{/if}

<!-- A paste says its own refusal inside the sheet it was made from, over this. -->
{#if showing !== 'paste' && !adding.fromPdf}
	<Said {adding} />
{/if}
