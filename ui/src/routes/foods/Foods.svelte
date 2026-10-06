<!--
	Foods, and the Food that is open (#107, #199).

	**On the phone these are two full pages**, as they always were: the list at
	`/foods`, a Food at `/foods/<id>`.

	**On the wide layout the list stays beside the open Food** (ADR 0044), so
	tidying is open, fix, next, without going back each time. The address of an
	open Food is the same on every layout, so a link copied at the computer
	opens on the phone. Before a Food is open, the right side says to choose
	one.

	The list is drawn by the layout of this family of routes, so opening
	another Food leaves it standing with its scroll and what was typed in it.
	On the phone it goes when a Food opens and is read afresh on the way back,
	which is what two pages always did.

	Both halves keep their place in this one frame whichever layout is showing,
	so a window dragged across the line keeps the Food being corrected, and a
	list on screen keeps the word being searched for. A wide window narrowed
	with a Food open is the phone's full page of that Food, with no list to
	keep.

	A Food corrected on its page is told to the list through `corrected`, so
	the row beside it changes with it.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import { m } from '$lib/paraglide/messages';
	import { useRoom } from '$lib/room.svelte';
	import { provideCorrected } from './corrected.svelte';
	import FoodList from './FoodList.svelte';

	interface Props {
		/** The word the Reading corrector arrived with, where it sent one. */
		q?: string;
		/** The Food the address names, where it names one. */
		open?: string;
		/** The open Food's page. */
		children?: Snippet;
	}

	let { q = '', open, children }: Props = $props();

	const room = useRoom();
	provideCorrected();
</script>

<div class={[room.wide && 'list-beside']}>
	{#if room.wide || open === undefined}
		<FoodList {q} {open} />
	{/if}
	{#if room.wide || open !== undefined}
		<div class={[room.wide && 'min-w-0']}>
			{#if open !== undefined}
				{@render children?.()}
			{:else}
				<p class="flex h-dvh items-center justify-center px-gutter text-body text-ink-2">
					{m.foods_choose()}
				</p>
			{/if}
		</div>
	{/if}
</div>
