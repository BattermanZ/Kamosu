<!-- The route itself: reads the URL's Food id and hands it to Food.svelte. -->
<script lang="ts">
	import { page } from '$app/state';
	import Food from './Food.svelte';
</script>

{#if page.params.foodId}
	<Food foodId={page.params.foodId} />
{/if}
