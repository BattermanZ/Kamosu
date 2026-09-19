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
	import { provideUpload, type Uploader } from '$lib/api/upload';

	interface Props {
		client: KamosuClient;
		auth?: AuthClient;
		upload?: Uploader;
		children: Snippet;
	}

	let {
		client,
		auth = { authenticate: async () => {} },
		upload = async () => {
			throw new Error('this test sent a file without giving an uploader');
		},
		children,
	}: Props = $props();

	provideKamosu(() => client);
	provideAuth(() => auth);
	provideUpload(() => upload);
</script>

{@render children()}
