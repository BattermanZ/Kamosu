<!--
	Test-only. An Invite link and a recovery link are routes of their own, and
	SvelteKit hands a route its secret as the `params` prop (#126). This renders
	the real route with a secret the test picks, inside the provider, so a test
	drives the page the link opens rather than the form alone.
-->
<script lang="ts">
	import type { Component } from 'svelte';
	import type { KamosuClient } from '$lib/api/catalogue';
	import type { AuthClient } from '$lib/auth';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	// Either route's: the two are handed the same thing.
	import type { PageProps } from './invite/[secret]/$types';

	interface Props {
		route: Component<PageProps>;
		secret: string;
		client: KamosuClient;
		auth: AuthClient;
	}

	let { route: Route, secret, client, auth }: Props = $props();
</script>

<Kamosu {client} {auth}>
	<Route params={{ secret }} data={{}} />
</Kamosu>
