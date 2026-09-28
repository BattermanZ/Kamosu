<!--
	Test-only. The three pieces of #106 take plain facts as props — `Recipe.svelte`
	reads them off `get_recipe` and `get_thread` and hands them down — so a test
	drives them directly rather than through a whole recipe page it is not
	testing. Each is drawn only when the test asks for it, so a test of the line
	is not also a test of the sheet.
-->
<script lang="ts">
	import type { GetRecipeOutput, KamosuClient } from '$lib/api/catalogue';
	import type { BranchLanguage, WrittenLanguage } from '$lib/language';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Language, { type OtherBranch } from './Language.svelte';
	import LanguageSheet from './LanguageSheet.svelte';
	import LanguageOffer from './LanguageOffer.svelte';

	interface Props {
		client: KamosuClient;
		language: string;
		translation?: GetRecipeOutput['translation'];
		others?: OtherBranch[];
		/** Draw the sheet, and with what the page would have worked out for it. */
		sheet?: { inAFamily: boolean; taken: string[] };
		/** Draw the offer, for a save that answered this Language. */
		offer?: BranchLanguage;
		/** What the page would have done with an answer, recorded for the test. */
		said?: (landed: { branch_id: string; language: string }) => void;
		translate?: (language: WrittenLanguage) => void;
	}

	let {
		client,
		language,
		translation = null,
		others = [],
		sheet,
		offer,
		said = () => {},
		translate = () => {},
	}: Props = $props();
</script>

<Kamosu {client}>
	<Language {language} {translation} {others} />
	{#if offer}
		<LanguageOffer branchId="b_1" filed={language} offered={offer} onSaid={said} />
	{/if}
	{#if sheet}
		<LanguageSheet
			branchId="b_1"
			{language}
			inAFamily={sheet.inAFamily}
			taken={sheet.taken}
			onSaid={said}
			onTranslate={translate}
			onClose={() => {}}
		/>
	{/if}
</Kamosu>
