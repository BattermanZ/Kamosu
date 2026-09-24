<!--
	Test-only, for the same reason the recipe's own harness exists: Writing
	takes real props, which the shared prop-less `renderScreen` helper cannot
	supply. It also gives the screen an outbox, because a photograph goes
	through one.
-->
<script lang="ts">
	import type { KamosuClient, GetRecipeOutput } from '$lib/api/catalogue';
	import type { WrittenLanguage } from '$lib/language';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Writing from './Writing.svelte';

	interface Props {
		client: KamosuClient;
		content: GetRecipeOutput['versions'][number]['content'];
		/** Whether the reader writes this recipe's Cookbook; a Copy is saved where not. */
		writes?: boolean;
		/** The lines that already name a Recipe, as the Core unfolds them (#87). */
		components?: GetRecipeOutput['versions'][number]['components'];
		photograph?: (file: Blob) => Promise<string>;
		/** Translating into this Language rather than editing (#106). */
		translatingInto?: WrittenLanguage;
		onCancel?: () => void;
		onSaved?: (landed: {
			branch_id: string;
			collapsed: boolean;
			copied: boolean;
			named: boolean;
			language_offer: string | null;
		}) => void;
	}

	let {
		client,
		content,
		writes = true,
		components = [],
		photograph,
		translatingInto,
		onCancel = () => {},
		onSaved = () => {},
	}: Props = $props();
</script>

<Kamosu {client} {photograph}>
	<Writing
		branchId="mine"
		lineageId="l_1"
		{writes}
		{content}
		{components}
		{translatingInto}
		{onCancel}
		{onSaved}
	/>
</Kamosu>
