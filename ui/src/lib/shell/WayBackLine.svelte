<!--
	The small line under a page's title that names where the page was reached
	from: "‹ Settings" on Brought in, "‹ Brought in" on what one source brought.

	Drawn on the phone layout only. The wide layout draws a back arrow on these
	pages (#195), and two ways back a thumb apart would be one too many.
-->
<script lang="ts">
	import { useRoom } from '$lib/room.svelte';

	interface Props {
		href: string;
		/** The name of the page it leads to. */
		label: string;
	}

	let { href, label }: Props = $props();

	const room = useRoom();
</script>

{#if !room.wide}
	<a {href} class="text-read text-ink-2">‹ {label}</a>
{/if}
