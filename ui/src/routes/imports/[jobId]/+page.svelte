<!--
	The route itself: reads the URL's Job id and hands it to Report.svelte,
	which holds everything the screen actually does.

	Keyed on the id, so each import gets a Report of its own. On the wide
	layout one Report follows another under the same route (#200), and a
	Report kept across two imports would show the first one's pairs, already
	answered, under the second one's address.
-->
<script lang="ts">
	import { page } from '$app/state';
	import Report from './Report.svelte';
</script>

{#if page.params.jobId}
	{#key page.params.jobId}
		<Report jobId={page.params.jobId} />
	{/key}
{/if}
