<!--
	The wrapper a screen is rendered in by `renderScreen`: the provider, and the
	screen inside it. It exists so a test never reaches into context by hand.
-->
<script lang="ts">
	import type { Component } from 'svelte';
	import type { KamosuClient } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';

	interface Props {
		component: Component<Record<string, never>>;
		client: KamosuClient;
	}

	let { component: Screen, client }: Props = $props();
</script>

<Kamosu {client}>
	<Screen />
</Kamosu>
