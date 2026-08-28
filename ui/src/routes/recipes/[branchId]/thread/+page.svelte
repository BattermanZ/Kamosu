<!-- The route itself: reads the URL's Branch id and hands it to Thread.svelte, which holds everything the screen actually does. -->
<script lang="ts">
	import { page } from '$app/state';
	import Thread from './Thread.svelte';
</script>

{#if page.params.branchId}
	<Thread branchId={page.params.branchId} />
{/if}
