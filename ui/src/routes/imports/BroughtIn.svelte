<!--
	Brought in, and the Report that is open (#108, #68, #200).

	**On the phone these are full pages**, as they always were: the sources at
	`/imports`, and a Report at `/imports/<jobId>`, reached through its
	source's screen.

	**On the wide layout a list of every import stays beside the open Report**
	(ADR 0044), the pattern Foods set in #199, so looking through what came in
	is open, read, next. The address of a Report is the same on every layout.
	Before a Report is open, the right side says to choose one.

	The list is drawn by the layout of this family of routes, so opening
	another Report leaves it standing with its scroll. The source screen
	(`/imports/from/<kind>`) is no part of this frame: it stays a full page on
	every layout, because forgetting lives there (#108).

	A Report that watches its import end tells the list through `ended`, so
	the row beside it stops saying the import is still running.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import { m } from '$lib/paraglide/messages';
	import { useRoom } from '$lib/room.svelte';
	import { provideEnded } from './ended.svelte';
	import Imports from './Imports.svelte';

	interface Props {
		/** The import whose Report the address names, where it names one. */
		open?: string;
		/** The open Report's page. */
		children?: Snippet;
	}

	let { open, children }: Props = $props();

	const room = useRoom();
	provideEnded();
</script>

<div class={[room.wide && 'list-beside']}>
	{#if room.wide || open === undefined}
		<Imports {open} />
	{/if}
	{#if room.wide || open !== undefined}
		<div class={[room.wide && 'min-w-0']}>
			{#if open !== undefined}
				{@render children?.()}
			{:else}
				<p class="flex h-dvh items-center justify-center px-gutter text-body text-ink-2">
					{m.imports_choose()}
				</p>
			{/if}
		</div>
	{/if}
</div>
