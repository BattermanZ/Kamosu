<!--
	Test-only. Source.svelte takes the source kind as a prop — the route reads it
	off the URL and hands it down — so a test drives it directly rather than
	through SvelteKit's router, which it is not testing.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Source from './Source.svelte';

	interface Props {
		client: KamosuClient;
		sourceKind: string;
	}

	let { client, sourceKind }: Props = $props();
</script>

<Kamosu {client}>
	<Source {sourceKind} />
</Kamosu>
