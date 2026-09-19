<!--
	The wrapper a screen is rendered in by `renderScreen`: the provider, and the
	screen inside it. It exists so a test never reaches into context by hand.
-->
<script lang="ts">
	import type { Component } from 'svelte';
	import type { KamosuClient } from '$lib/api/catalogue';
	import type { Keeping } from '$lib/offline/outbox';
	import Kamosu from '$lib/shell/Kamosu.svelte';

	interface Props {
		component: Component<Record<string, never>>;
		client: KamosuClient;
		keeping?: Keeping;
	}

	let { component: Screen, client, keeping }: Props = $props();
</script>

<Kamosu {client} {keeping}>
	<Screen />
</Kamosu>
