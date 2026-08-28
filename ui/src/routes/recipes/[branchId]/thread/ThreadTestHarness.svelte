<!--
	Test-only. `renderScreen` only supports the prop-less route components
	every other screen here is (their data all arrives through `$app/state` or
	context) — Thread.svelte takes `branchId` as a real prop instead, precisely
	so a test can drive it directly, so it needs this instead of the shared
	harness.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Thread from './Thread.svelte';

	interface Props {
		client: KamosuClient;
		branchId: string;
	}

	let { client, branchId }: Props = $props();
</script>

<Kamosu {client}>
	<Thread {branchId} />
</Kamosu>
