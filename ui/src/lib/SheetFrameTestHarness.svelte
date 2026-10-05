<!--
	Test-only. A page with one control that opens a sheet, so a test can watch
	the caret go in and come back, in a Room it chooses.
-->
<script lang="ts">
	import SheetFrame from '$lib/SheetFrame.svelte';
	import { provideRoom, type Room } from '$lib/room.svelte';

	interface Props {
		room?: Room;
		kind?: 'sheet' | 'cover';
		dim?: 'paper' | 'ink' | 'cook';
		/** Whether the sheet holds a field that asks for the caret. */
		field?: boolean;
		/** Whether the sheet has one clear thing to confirm. */
		confirms?: boolean;
		/** Whether closing replaces the control that opened it, as choosing a
		 *  recipe for a line does. */
		replaces?: boolean;
		onconfirm?: () => void;
	}

	let {
		room = 'phone',
		kind = 'sheet',
		dim = 'paper',
		field = false,
		confirms = false,
		replaces = false,
		onconfirm = () => {},
	}: Props = $props();

	provideRoom(() => room);

	let open = $state(false);
	let replaced = $state(false);

	function close() {
		open = false;
		if (replaces) replaced = true;
	}
</script>

<input aria-label="A search on the page" />
<p>
	{#if replaced}
		<button type="button">Names Pizza Dough</button>
	{:else}
		<button type="button" onclick={() => (open = true)}>Open</button>
	{/if}
</p>

{#if open}
	<SheetFrame
		label="A sheet"
		{kind}
		{dim}
		tall={78}
		onclose={close}
		onconfirm={confirms ? onconfirm : undefined}
	>
		{#if field}
			<input aria-label="A name" data-sheet-focus />
		{/if}
		<textarea aria-label="A note"></textarea>
		<button type="button" onclick={close}>Done</button>
	</SheetFrame>
{/if}
