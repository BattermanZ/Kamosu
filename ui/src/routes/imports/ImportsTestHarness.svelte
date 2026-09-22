<!--
	Test-only. Imports.svelte takes its client from context, never from fetch,
	so a test wraps it the same way the shell does.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Imports from './Imports.svelte';

	interface Props {
		client: KamosuClient;
	}

	let { client }: Props = $props();
</script>

<Kamosu {client}>
	<Imports />
</Kamosu>
