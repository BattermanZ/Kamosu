<!--
	Test-only. Shelf.svelte takes the arriving `?tag=` as a real prop — the route
	reads it off `$app/state` and hands it down — so a test drives that case
	directly rather than fighting a router the screen deliberately does not know
	about. A test of the + can hand it an uploader, for the Kamosu zip file.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import type { Uploader } from '$lib/api/upload';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Shelf from './Shelf.svelte';

	interface Props {
		client: KamosuClient;
		tag?: string | null;
		upload?: Uploader;
	}

	let { client, tag = null, upload }: Props = $props();
</script>

<Kamosu {client} {upload}>
	<Shelf {tag} />
</Kamosu>
