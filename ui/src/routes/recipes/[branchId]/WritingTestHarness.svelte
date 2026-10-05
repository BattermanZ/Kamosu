<!--
	Test-only, for the same reason the recipe's own harness exists: Writing
	takes real props, which the shared prop-less `renderScreen` helper cannot
	supply. It also gives the screen an outbox, because a photograph goes
	through one.
-->
<script lang="ts">
	import type { KamosuClient, GetRecipeOutput } from '$lib/api/catalogue';
	import type { WrittenLanguage } from '$lib/language';
	import type { Whose } from '$lib/cookbook';
	import type { Room } from '$lib/room.svelte';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Writing from './Writing.svelte';

	interface Props {
		client: KamosuClient;
		content: GetRecipeOutput['versions'][number]['content'];
		/** Whether the reader writes this recipe's Cookbook; a Copy is saved where not. */
		writes?: boolean;
		/** Why a save makes a Copy where it does: Hélène's Cookbook, unless a test says otherwise (#132). */
		whose?: Omit<Whose, 'writes'>;
		/** The lines that already name a Recipe, as the Core unfolds them (#87). */
		components?: GetRecipeOutput['versions'][number]['components'];
		photograph?: (file: Blob) => Promise<string>;
		/** How much room the window has (#193). The phone's unless a test says. */
		room?: Room;
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
		whose = {
			mine: false,
			arrived: false,
			cookbook: { id: 'c_h', name: null, authors: [{ person_id: 'p_h', name: 'Hélène' }] },
		},
		components = [],
		photograph,
		room = 'phone',
		translatingInto,
		onCancel = () => {},
		onSaved = () => {},
	}: Props = $props();
</script>

<Kamosu {client} {photograph} {room}>
	<Writing
		branchId="mine"
		lineageId="l_1"
		whose={{ ...whose, writes }}
		{content}
		{components}
		{translatingInto}
		{onCancel}
		{onSaved}
	/>
</Kamosu>
