<!-- The route: reads the source kind off the URL and hands it to Source.svelte. -->
<script lang="ts">
	import { page } from '$app/state';
	import Source from './Source.svelte';
</script>

{#if page.params.sourceKind}
	<Source sourceKind={page.params.sourceKind} />
{/if}
