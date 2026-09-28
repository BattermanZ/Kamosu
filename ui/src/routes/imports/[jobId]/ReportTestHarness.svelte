<!--
	Test-only: Report.svelte takes `jobId` as a real prop so a test can drive
	it directly, which the shared prop-less `renderScreen` helper cannot do.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Report from './Report.svelte';

	interface Props {
		client: KamosuClient;
		jobId: string;
	}

	let { client, jobId }: Props = $props();
</script>

<Kamosu {client}>
	<Report {jobId} every={5} />
</Kamosu>
