<!--
	Test-only. BroughtIn.svelte takes the import whose Report is open as a prop
	(the route reads it off the address and hands it down) and its client from
	context, never from fetch, so a test wraps it the same way the shell does.
	The open Report's page is stood in for by OpenReportTestStandIn.svelte.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import type { Room } from '$lib/room.svelte';
	import BroughtIn from './BroughtIn.svelte';
	import OpenReport from './OpenReportTestStandIn.svelte';

	interface Props {
		client: KamosuClient;
		/** How much room the window has; the phone's without one. */
		room?: Room;
		open?: string;
	}

	let { client, room, open }: Props = $props();
</script>

<Kamosu {client} {room}>
	<BroughtIn {open}>
		<OpenReport {open} />
	</BroughtIn>
</Kamosu>
