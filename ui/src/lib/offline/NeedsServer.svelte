<!--
	A button for something only the server can do (#76, ADR 0013).

	Editing a recipe, keeping an As Cooked as a Version and taking a Copy are on
	the recipe's side of the line, and none of them is ever queued: an offline
	edit queue is a merge, and Kamosu never merges. So offline the button stays
	where it is, greyed, and its words change to say what it is waiting for —
	"Editing waits for the server" — rather than failing when pressed (option C,
	Aurélien, 19 September 2026).

	Edit (#83) and Take a Copy (#65) are drawn with this when they arrive, with
	`offline_waits_edit` and `offline_waits_copy`.
-->
<script lang="ts">
	import { Online } from './device.svelte';

	interface Props {
		label: string;
		/** What it says offline: what it is waiting for. */
		waiting: string;
		onclick: () => void;
		disabled?: boolean;
		/** Size and place, kept in both states. */
		shapeClass: string;
		/** Colour online. */
		lookClass: string;
		/**
		 * Colour offline: greyed on paper unless the button sits on indigo,
		 * where grey ink would not read.
		 */
		idleClass?: string;
	}

	let {
		label,
		waiting,
		onclick,
		disabled = false,
		shapeClass,
		lookClass,
		idleClass = 'border border-rule text-ink-2 opacity-55',
	}: Props = $props();

	const online = new Online();
</script>

{#if online.current}
	<button type="button" {disabled} {onclick} class="{shapeClass} {lookClass}">{label}</button>
{:else}
	<button type="button" disabled class="{shapeClass} {idleClass}">
		{waiting}
	</button>
{/if}
