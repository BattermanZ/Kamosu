<!-- The route itself: reads the URL's Branch and Version ids and hands them to OlderVersion.svelte, which holds everything the screen actually does. -->
<script lang="ts">
	import { page } from '$app/state';
	import OlderVersion from '../OlderVersion.svelte';
</script>

{#if page.params.branchId && page.params.versionId}
	<OlderVersion branchId={page.params.branchId} versionId={page.params.versionId} />
{/if}
