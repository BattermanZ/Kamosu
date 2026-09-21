<!--
	Test-only, for the same reason the recipe's own harness exists: Writing
	takes real props, which the shared prop-less `renderScreen` helper cannot
	supply. It also gives the screen an outbox, because a photograph goes
	through one.
-->
<script lang="ts">
	import type { KamosuClient, GetRecipeOutput } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Writing from './Writing.svelte';

	interface Props {
		client: KamosuClient;
		content: GetRecipeOutput['versions'][number]['content'];
		kitchenId: string;
		/** The lines that already name a Recipe, as the Core unfolds them (#87). */
		components?: GetRecipeOutput['versions'][number]['components'];
		photograph?: (file: Blob) => Promise<string>;
		onCancel?: () => void;
		onSaved?: (landed: {
			branch_id: string;
			collapsed: boolean;
			copied: boolean;
			named: boolean;
		}) => void;
	}

	let {
		client,
		content,
		kitchenId,
		components = [],
		photograph,
		onCancel = () => {},
		onSaved = () => {},
	}: Props = $props();
</script>

<Kamosu {client} {photograph}>
	<Writing
		branchId="mine"
		lineageId="l_1"
		{kitchenId}
		{content}
		{components}
		{onCancel}
		{onSaved}
	/>
</Kamosu>
