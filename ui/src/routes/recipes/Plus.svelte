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

	const sources: { act: Act; label: () => string; why: () => string; icon: string }[] = [
		{
			act: 'link',
			label: m.plus_link,
			why: m.plus_link_why,
			icon: 'M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1',
		},
		{
			act: 'file',
			label: m.plus_file,
			why: m.plus_file_why,
			icon: 'M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8zM14 3v5h5',
		},
		{
			act: 'pdf',
			label: m.plus_pdf,
			why: m.plus_pdf_why,
			icon: 'M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8zM14 3v5h5M9 13h6M9 17h4',
		},
		{
			act: 'paste',
			label: m.plus_paste,
			why: m.plus_paste_why,
			icon: 'M9 4h6v3H9zM8 5H6a1 1 0 0 0-1 1v14a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V6a1 1 0 0 0-1-1h-2M8 12h8M8 16h5',
		},
		{
			act: 'write',
			label: m.plus_write,
			why: m.plus_write_why,
			icon: 'M4 20h4L19 9l-4-4L4 16zM13.5 6.5l4 4',
		},
	];

	/** What an act says offline instead of its label. */
	const waiting: Record<Act, () => string> = {
		link: m.offline_waits_import_link,
		file: m.offline_waits_bring_in,
		pdf: m.offline_waits_pdf,
		write: m.offline_waits_write,
		paste: m.offline_waits_paste,
	};
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
						{@const waits = online.current ? null : waiting[source.act]}
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
										stroke-linejoin="round"><path d={source.icon} /></svg
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
