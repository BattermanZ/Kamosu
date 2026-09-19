<!--
	The provider. In the app the root layout wraps everything in it; in a test a
	screen is wrapped in it holding a stand-in. One component, so there is only
	one way a client ever reaches a screen.
-->
<script lang="ts">
	import { untrack, type Snippet } from 'svelte';
	import type { KamosuClient } from '$lib/api/catalogue';
	import { provideKamosu } from '$lib/kamosu';
	import { provideAuth, type AuthClient } from '$lib/auth';
	import { provideUpload, type Uploader } from '$lib/api/upload';
	import { Library, provideLibrary } from '$lib/offline/library.svelte';

	interface Props {
		client: KamosuClient;
		auth?: AuthClient;
		upload?: Uploader;
		/** What is on the phone (#76). One per app; a test may bring its own. */
		library?: Library;
		children: Snippet;
	}

	let {
		client,
		auth = { authenticate: async () => {} },
		upload = async () => {
			throw new Error('this test sent a file without giving an uploader');
		},
		library,
		children,
	}: Props = $props();

	// Made once, from the client this provider was given. `untrack` because it
	// is a thing to hold, not a value to follow.
	const theLibrary = untrack(() => library ?? new Library(client));

	provideKamosu(() => client);
	provideAuth(() => auth);
	provideUpload(() => upload);
	provideLibrary(() => theLibrary);
</script>

{@render children()}
