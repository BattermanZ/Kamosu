<!--
	Test-only. Imports.svelte takes its client from context, never from fetch,
	so a test wraps it the same way the shell does.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import type { Room } from '$lib/room.svelte';
	import Imports from './Imports.svelte';

	interface Props {
		client: KamosuClient;
		/** How much room the window has; the phone's without one. */
		room?: Room;
	}

	let { client, room }: Props = $props();
</script>

<Kamosu {client} {room}>
	<Imports />
</Kamosu>
