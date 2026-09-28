<!--
	A Sheet (#75, ADR 0023): this recipe, as it stands here, on paper. It waits
	for the server, since the server is what sets it. `sheet.svelte.ts` holds
	the waiting, and why it lives beside the screen rather than in here.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';
	import type { Sheet } from './sheet.svelte';

	interface Props {
		sheet: Sheet;
		/** Ask for one, at the amount on screen. */
		print: () => void;
	}

	let { sheet, print }: Props = $props();
</script>

{#if sheet.printing === 'ready'}
	<!--
		The Sheet is already on the phone, so sharing it waits for nothing
		(#149): the same button, no longer one that needs the server.
	-->
	<button
		type="button"
		onclick={sheet.share}
		class="mx-gutter mt-2 block w-[calc(100%-2*var(--spacing-gutter))] border border-rule p-4 text-center font-display text-body text-accent"
	>
		{m.recipe_share_sheet()}
	</button>
{:else}
	<NeedsServer
		label={sheet.setting ? m.recipe_print_setting() : m.recipe_print_sheet()}
		waiting={m.offline_waits_print()}
		onclick={print}
		disabled={sheet.setting}
		shapeClass="mx-gutter mt-2 block w-[calc(100%-2*var(--spacing-gutter))] p-4 text-center font-display text-body"
		lookClass="border border-rule text-accent"
	/>
{/if}
{#if sheet.printing === 'failed'}
	<p class="mx-gutter mt-2 text-read text-support" role="alert">{m.recipe_print_failed()}</p>
{:else if sheet.printing === 'stillSetting'}
	<p class="mx-gutter mt-2 text-read text-ink-2" role="status">
		{sheet.sharing ? m.recipe_share_still_going() : m.recipe_print_still_going()}
	</p>
{/if}
