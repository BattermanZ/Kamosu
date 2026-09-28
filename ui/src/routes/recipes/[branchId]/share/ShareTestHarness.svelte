<!--
	Test-only. Share.svelte takes `branchId` as a real prop so a test can drive
	it directly, the way Thread.svelte does.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import type { FileFetcher } from '$lib/api/files';
	import type { Device } from '$lib/offline/device.svelte';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Share from './Share.svelte';

	interface Props {
		client: KamosuClient;
		branchId: string;
		device?: Pick<Device, 'installed' | 'apple'>;
		files?: FileFetcher;
	}

	let { client, branchId, device, files }: Props = $props();
</script>

<Kamosu {client} {files}>
	<Share {branchId} {device} />
</Kamosu>
