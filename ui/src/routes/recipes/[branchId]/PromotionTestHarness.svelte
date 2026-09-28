<!--
	Test-only. Promotion.svelte takes the cookings and the Branch's chain as
	real props — Recipe.svelte reads both and hands them down — so a test drives
	it directly rather than through a whole recipe page it is not testing.
-->
<script lang="ts">
	import type { GetRecipeOutput, GetThreadOutput, KamosuClient } from '$lib/api/catalogue';
	import type { Whose } from '$lib/cookbook';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Promotion from './Promotion.svelte';

	interface Props {
		client: KamosuClient;
		attempts: GetThreadOutput['attempts'];
		versions: GetRecipeOutput['versions'];
		/** Whether the cook writes this recipe's Cookbook; they do, unless a test says not. */
		writes?: boolean;
		/** Why keeping makes a Copy where it does: Hélène's Cookbook, unless a test says otherwise (#132). */
		whose?: Omit<Whose, 'writes'>;
	}

	let {
		client,
		attempts,
		versions,
		writes = true,
		whose = {
			mine: false,
			arrived: false,
			cookbook: { id: 'c_h', name: null, authors: [{ person_id: 'p_h', name: 'Hélène' }] },
		},
	}: Props = $props();
</script>

<Kamosu {client}>
	<Promotion
		branchId="b_1"
		whose={{ ...whose, writes }}
		{attempts}
		{versions}
		promoted={() => {}}
	/>
</Kamosu>
