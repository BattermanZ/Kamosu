<!--
	The provider. In the app the root layout wraps everything in it; in a test a
	screen is wrapped in it holding a stand-in. One component, so there is only
	one way a client ever reaches a screen.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { KamosuClient } from '$lib/api/catalogue';
	import { provideKamosu } from '$lib/kamosu';
	import { provideAuth, type AuthClient } from '$lib/auth';

	interface Props {
		client: KamosuClient;
		auth?: AuthClient;
		children: Snippet;
	}

	let { client, auth = { authenticate: async () => {} }, children }: Props = $props();

	provideKamosu(() => client);
	provideAuth(() => auth);
</script>

{@render children()}
