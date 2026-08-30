<!-- The route: reads the URL's Branch id and hands it to Cooking.svelte, which holds everything the screen actually does. -->
<script lang="ts">
	import { page } from '$app/state';
	import Cooking from './Cooking.svelte';
</script>

{#if page.params.branchId}
	<Cooking branchId={page.params.branchId} />
{/if}
