<!--
	Test-only: ImportCrouton.svelte takes its uploader from context, so a test
	hands the provider a stand-in for `POST /api/uploads`, with no network.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import type { Uploader } from '$lib/api/upload';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import ImportCrouton from './ImportCrouton.svelte';

	interface Props {
		client: KamosuClient;
		upload: Uploader;
	}

	let { client, upload }: Props = $props();
</script>

<Kamosu {client} {upload}>
	<ImportCrouton />
</Kamosu>
