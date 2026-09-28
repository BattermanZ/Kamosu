<!-- The route itself: reads the URL's Job id and hands it to Report.svelte, which holds everything the screen actually does. -->
<script lang="ts">
	import { page } from '$app/state';
	import Report from './Report.svelte';
</script>

{#if page.params.jobId}
	<Report jobId={page.params.jobId} />
{/if}
