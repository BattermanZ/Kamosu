<!--
	Test-only. RelatedRecipes.svelte takes the recipe's links as a real prop —
	Recipe.svelte reads them off `get_recipe` and hands them down — so a test
	drives it directly rather than through a whole recipe page it is not testing.
-->
<script lang="ts">
	import type { GetRecipeOutput, KamosuClient } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import RelatedRecipes from './RelatedRecipes.svelte';

	interface Props {
		client: KamosuClient;
		related: GetRecipeOutput['related_recipes'];
	}

	let { client, related }: Props = $props();
</script>

<Kamosu {client}>
	<RelatedRecipes branchId="b_1" kitchenId="k_home" {related} />
</Kamosu>
