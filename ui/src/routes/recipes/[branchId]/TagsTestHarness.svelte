<!--
	Test-only. Tags.svelte takes the recipe's tags as a real prop — Recipe.svelte
	reads them off `get_recipe` and hands them down — so a test drives it
	directly rather than through a whole recipe page it is not testing.
-->
<script lang="ts">
	import type { GetRecipeOutput, KamosuClient } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Tags from './Tags.svelte';

	interface Props {
		client: KamosuClient;
		tags: GetRecipeOutput['tags'];
	}

	let { client, tags }: Props = $props();
</script>

<Kamosu {client}>
	<Tags branchId="b_1" kitchenId="k_home" {tags} />
</Kamosu>
