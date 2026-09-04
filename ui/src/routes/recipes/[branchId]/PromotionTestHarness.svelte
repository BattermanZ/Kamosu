<!--
	Test-only. Promotion.svelte takes the cookings and the Branch's chain as
	real props — Recipe.svelte reads both and hands them down — so a test drives
	it directly rather than through a whole recipe page it is not testing.
-->
<script lang="ts">
	import type { GetRecipeOutput, GetThreadOutput, KamosuClient } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Promotion from './Promotion.svelte';

	interface Props {
		client: KamosuClient;
		attempts: GetThreadOutput['attempts'];
		versions: GetRecipeOutput['versions'];
	}

	let { client, attempts, versions }: Props = $props();
</script>

<Kamosu {client}>
	<Promotion branchId="b_1" {attempts} {versions} promoted={() => {}} />
</Kamosu>
