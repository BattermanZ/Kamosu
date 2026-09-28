<!-- The route itself: reads the URL's Branch id and hands it to Share.svelte, which holds everything the screen actually does. -->
<script lang="ts">
	import { page } from '$app/state';
	import Share from './Share.svelte';
</script>

{#if page.params.branchId}
	<Share branchId={page.params.branchId} />
{/if}
