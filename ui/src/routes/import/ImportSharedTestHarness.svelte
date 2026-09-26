<!--
	Test-only. The page a Share Link's *Import this recipe* opens, inside the
	provider, with the link, the reader's origin and the way out to another
	Kamosu all chosen by the test (#170). `Arrived` is drawn above it as the
	app's layout draws it, since importing ends on that line.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import type { AuthClient } from '$lib/auth';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Arrived from '$lib/Arrived.svelte';
	import ImportShared from './ImportShared.svelte';

	interface Props {
		client: KamosuClient;
		auth: AuthClient;
		link: string | null;
		origin: string;
		leave: (address: string) => void;
		pathname?: string;
	}

	let { client, auth, link, origin, leave, pathname = '/import' }: Props = $props();
</script>

<Kamosu {client} {auth}>
	<Arrived {pathname} />
	<ImportShared {link} {origin} {leave} />
</Kamosu>
