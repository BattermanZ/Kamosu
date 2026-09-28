<!--
	Test-only. SvelteKit takes the route's `snapshot` as a reader leaves Recipes
	and hands it back when they go back (#191). A test has no browser history to
	walk, so this renders the real route and lends out that same `snapshot`: the
	test plays SvelteKit's part, leaving with `capture` and coming back with
	`restore`.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Recipes from './+page.svelte';

	interface Props {
		client: KamosuClient;
	}

	let { client }: Props = $props();

	let route = $state<ReturnType<typeof Recipes>>();

	/** The route's own `snapshot`, exactly what SvelteKit is given. */
	export function snapshot() {
		if (!route) throw new Error('the route has not been built yet');
		return route.snapshot;
	}
</script>

<Kamosu {client}>
	<Recipes bind:this={route} />
</Kamosu>
