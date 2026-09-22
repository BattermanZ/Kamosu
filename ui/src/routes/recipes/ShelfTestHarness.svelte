<!--
	Test-only. Shelf.svelte takes the arriving `?tag=` as a real prop — the route
	reads it off `$app/state` and hands it down — so a test drives that case
	directly rather than fighting a router the screen deliberately does not know
	about.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Shelf from './Shelf.svelte';

	interface Props {
		client: KamosuClient;
		tag: string | null;
	}

	let { client, tag }: Props = $props();
</script>

<Kamosu {client}>
	<Shelf {tag} />
</Kamosu>
