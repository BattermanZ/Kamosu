<!--
	Test-only. Share.svelte takes `branchId` as a real prop so a test can drive
	it directly, the way Thread.svelte does.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Share from './Share.svelte';

	interface Props {
		client: KamosuClient;
		branchId: string;
	}

	let { client, branchId }: Props = $props();
</script>

<Kamosu {client}>
	<Share {branchId} />
</Kamosu>
