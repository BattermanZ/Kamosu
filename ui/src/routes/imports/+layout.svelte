<!--
	The layout of Brought in and a Report: reads the address and hands it to
	BroughtIn.svelte, which holds everything the two screens share.

	The list lives here rather than in the list's own page so that it is still
	standing when a Report is opened beside it on the wide layout (#200).

	What one source brought (`from/<kind>`) is under this layout too and takes
	nothing from it: it is a full page on every layout.
-->
<script lang="ts">
	import { page } from '$app/state';
	import BroughtIn from './BroughtIn.svelte';

	let { children } = $props();
</script>

{#if page.params.sourceKind !== undefined}
	{@render children()}
{:else}
	<BroughtIn open={page.params.jobId}>
		{@render children()}
	</BroughtIn>
{/if}
