<!--
	Test-only: Report.svelte takes `jobId` as a real prop so a test can drive
	it directly, which the shared prop-less `renderScreen` helper cannot do.
	With `beside` it is drawn where the route draws it, in the frame that keeps
	the list of imports beside it on the wide layout.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import type { Room } from '$lib/room.svelte';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import BroughtIn from '../BroughtIn.svelte';
	import Report from './Report.svelte';

	interface Props {
		client: KamosuClient;
		jobId: string;
		room?: Room;
		beside?: boolean;
	}

	let { client, jobId, room, beside = false }: Props = $props();
</script>

<Kamosu {client} {room}>
	{#if beside}
		<BroughtIn open={jobId}>
			<Report {jobId} every={5} />
		</BroughtIn>
	{:else}
		<Report {jobId} every={5} />
	{/if}
</Kamosu>
