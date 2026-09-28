<!--
	A screen that says which Room it was drawn in and nothing else, so the
	Room's own tests can check what a screen is handed (#193).
-->
<script lang="ts">
	import { useRoom } from '$lib/room.svelte';

	const room = useRoom();
</script>

<p data-testid="room" data-wide={room.wide} data-roomy={room.roomy}>{room.current}</p>
