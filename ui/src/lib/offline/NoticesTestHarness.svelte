<!--
	Test-only: the cards as the layout places them, inside the provider, with a
	device of the test's choosing.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Notices from './Notices.svelte';
	import type { Device } from './device.svelte';

	interface Props {
		client: KamosuClient;
		device: Device;
	}

	let { client, device }: Props = $props();
</script>

<Kamosu {client}>
	<Notices {device} />
</Kamosu>
