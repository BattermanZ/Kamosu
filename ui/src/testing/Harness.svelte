<!--
	The wrapper a screen is rendered in by `renderScreen`: the provider, and the
	screen inside it. It exists so a test never reaches into context by hand.
-->
<script lang="ts">
	import type { Component } from 'svelte';
	import type { KamosuClient } from '$lib/api/catalogue';
	import type { Keeping } from '$lib/offline/outbox';
	import type { AuthClient } from '$lib/auth';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import type { Room } from '$lib/room.svelte';

	interface Props {
		component: Component<Record<string, never>>;
		client: KamosuClient;
		keeping?: Keeping;
		/** Signing in, for the one screen that does it; a no-op otherwise. */
		auth?: AuthClient;
		/** How much room the window has (#193); the phone's without one. */
		room?: Room;
	}

	let { component: Screen, client, keeping, auth, room }: Props = $props();
</script>

<Kamosu {client} {keeping} {auth} {room}>
	<Screen />
</Kamosu>
