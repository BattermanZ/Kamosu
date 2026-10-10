<!--
	Test-only. OlderVersion.svelte takes its ids as real props, so a test can
	drive it directly, and so needs this in place of the shared harness.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import OlderVersion from './OlderVersion.svelte';

	interface Props {
		client: KamosuClient;
		branchId: string;
		versionId: string;
		navigate?: (to: string, how?: { replaceState: boolean }) => Promise<void>;
	}

	let { client, branchId, versionId, navigate }: Props = $props();
</script>

<Kamosu {client}>
	<OlderVersion {branchId} {versionId} {navigate} />
</Kamosu>
