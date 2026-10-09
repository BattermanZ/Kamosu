<!--
	Test-only: the cards as the layout places them, inside the provider, with a
	device of the test's choosing.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Notices from './Notices.svelte';
	import type { Device } from './device.svelte';
	import type { Room } from '$lib/room.svelte';

	interface Props {
		client: KamosuClient;
		device: Device;
		room?: Room;
	}

	let { client, device, room = 'phone' }: Props = $props();
</script>

<Kamosu {client} {room}>
	<Notices {device} />
</Kamosu>
