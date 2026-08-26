<!--
	The provider. In the app the root layout wraps everything in it; in a test a
	screen is wrapped in it holding a stand-in. One component, so there is only
	one way a client ever reaches a screen.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { KamosuClient } from '$lib/api/catalogue';
	import { provideKamosu } from '$lib/kamosu';

	interface Props {
		client: KamosuClient;
		children: Snippet;
	}

	let { client, children }: Props = $props();

	provideKamosu(() => client);
</script>

{@render children()}
