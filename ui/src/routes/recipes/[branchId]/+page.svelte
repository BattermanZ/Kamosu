<!-- The route itself: reads the URL's Branch id and hands it to Recipe.svelte, which holds everything the screen actually does. -->
<script lang="ts">
	import { page } from '$app/state';
	import Recipe from './Recipe.svelte';
</script>

{#if page.params.branchId}
	<Recipe branchId={page.params.branchId} />
{/if}
