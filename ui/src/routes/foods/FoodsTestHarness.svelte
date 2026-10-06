<!--
	Test-only. Foods.svelte takes the word the corrector arrived with and the
	Food that is open as props (the route reads both off the address and hands
	them down), so a test drives it directly rather than through SvelteKit's
	router, which it is not testing. The open Food's page is stood in for by
	OpenFoodTestStandIn.svelte.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import type { Food } from '$lib/foods';
	import type { Room } from '$lib/room.svelte';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Foods from './Foods.svelte';
	import OpenFood from './OpenFoodTestStandIn.svelte';

	interface Props {
		client: KamosuClient;
		q?: string;
		room?: Room;
		open?: string;
		correction?: Food;
	}

	let { client, q = '', room = 'phone', open, correction }: Props = $props();
</script>

<Kamosu {client} {room}>
	<Foods {q} {open}>
		<OpenFood {open} {correction} />
	</Foods>
</Kamosu>
