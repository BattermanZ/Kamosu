<!--
	Test-only, for the same reason the recipe screen's harness exists:
	Cooking.svelte takes `branchId` as a real prop so a test can drive it
	directly, which the shared prop-less `renderScreen` helper cannot do.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import type { Keeping } from '$lib/offline/outbox';
	import type { Room } from '$lib/room.svelte';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Cooking from './Cooking.svelte';

	interface Props {
		client: KamosuClient;
		branchId: string;
		keeping?: Keeping;
		/** How much room the window has (#193). The phone's unless a test says. */
		room?: Room;
	}

	let { client, branchId, keeping, room = 'phone' }: Props = $props();
</script>

<Kamosu {client} {keeping} {room}>
	<Cooking {branchId} />
</Kamosu>
