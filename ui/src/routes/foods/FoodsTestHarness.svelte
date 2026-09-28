<!--
	Test-only. Foods.svelte takes the word the corrector arrived with as a prop —
	the route reads it off `?q=` and hands it down — so a test drives it directly
	rather than through SvelteKit's router, which it is not testing.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Foods from './Foods.svelte';

	interface Props {
		client: KamosuClient;
		q?: string;
	}

	let { client, q = '' }: Props = $props();
</script>

<Kamosu {client}>
	<Foods {q} />
</Kamosu>
