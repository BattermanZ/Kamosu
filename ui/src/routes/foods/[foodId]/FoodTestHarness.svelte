<!--
	Test-only. Food.svelte takes the Food's id as a prop — the route reads it off
	the URL and hands it down — so a test drives it directly rather than through
	SvelteKit's router, which it is not testing.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Food from './Food.svelte';

	interface Props {
		client: KamosuClient;
		foodId?: string;
	}

	let { client, foodId = 'f_caster' }: Props = $props();
</script>

<Kamosu {client}>
	<Food {foodId} />
</Kamosu>
