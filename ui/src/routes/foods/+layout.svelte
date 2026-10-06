<!--
	The layout of Foods and a Food: reads the address and hands it to
	Foods.svelte, which holds everything the two screens share.

	The list lives here rather than in the list's own page so that it is still
	standing when a Food is opened beside it on the wide layout (#199).
-->
<script lang="ts">
	import { page } from '$app/state';
	import Foods from './Foods.svelte';

	let { children } = $props();

	const q = $derived(page.url.searchParams.get('q') ?? '');
</script>

<Foods {q} open={page.params.foodId}>
	{@render children()}
</Foods>
