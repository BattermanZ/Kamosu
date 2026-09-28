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
	import { providePhotograph, provideUpload, type Uploader } from '$lib/api/upload';
	import { provideFiles, type FileFetcher } from '$lib/api/files';
	import { Library, provideLibrary } from '$lib/offline/library.svelte';
	import { provideKeeping, type Keeping } from '$lib/offline/outbox';
	import { provideRoom, type Room } from '$lib/room.svelte';

	interface Props {
		client: KamosuClient;
		auth?: AuthClient;
		upload?: Uploader;
		/** A picture that must reach the server now, and answer its real name. */
		photograph?: Uploader;
		/** A Sheet or a recipe file, fetched as a file to share (#149, #156). */
		files?: FileFetcher;
		/** What is on the phone (#76). One per app; a test may bring its own. */
		library?: Library;
		/** What the phone holds for the server (#77). A test may bring its own. */
		keeping?: Keeping;
		/**
		 * How much room the window has (#193). The layout follows the window's;
		 * a test says which it is drawing, and without one it is the phone.
		 */
		room?: Room;
		children: Snippet;
	}

	let {
		client,
		auth = { authenticate: async () => {} },
		upload = async () => {
			throw new Error('this test sent a file without giving an uploader');
		},
		photograph = async () => {
			throw new Error('this test sent a picture without giving a photograph uploader');
		},
		files = async () => {
			throw new Error('this test fetched a file without giving a file fetcher');
		},
		library,
		keeping = {
			holds: () => false,
			keepPhotograph: async () => {
				throw new Error('this test took a photograph without giving an outbox');
			},
			photographSrc: async (id, size) => `/api/photographs/${id}/${size}`,
		},
		room = 'phone',
		children,
	}: Props = $props();

	// Made once, from the client this provider was given. `untrack` because it
	// is a thing to hold, not a value to follow.
	const theLibrary = untrack(() => library ?? new Library(client));

	provideKamosu(() => client);
	provideAuth(() => auth);
	provideUpload(() => upload);
	providePhotograph(() => photograph);
	provideFiles(() => files);
	provideLibrary(() => theLibrary);
	provideKeeping(() => keeping);
	provideRoom(() => room);
</script>

{@render children()}
