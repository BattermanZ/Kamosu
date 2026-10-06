<!--
	The shell as the root layout draws it, for its screen test: inside the
	provider, in the Room the test names, round a screen that says whether it
	was drawn bare and holds the account form, the story or the Recipes shelf
	when asked to.
-->
<script lang="ts">
	import { standIn } from '$lib/api/stand-in';
	import type { Room } from '$lib/room.svelte';
	import Account from './Account.svelte';
	import Story from '$lib/story/Story.svelte';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Shell from '$lib/shell/Shell.svelte';
	import Shelf from './recipes/Shelf.svelte';

	interface Props {
		pathname: string;
		room: Room;
		/**
		 * What the screen holds: the account form, as `/` does for a stranger,
		 * the story, or the shelf Recipes draws, with its search box.
		 */
		holds?: 'account' | 'story' | 'shelf';
	}

	let { pathname, room, holds }: Props = $props();

	const kamosu = standIn({
		instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
		list_tags: { tags: [] },
		list_kitchens: { kitchens: [] },
		meaning_search_status: {
			state: 'unasked',
			on: false,
			offer: false,
			may_change: false,
			model: 'EmbeddingGemma-300M (4-bit)',
			terms_url: 'https://ai.google.dev/gemma/terms',
			prohibited_use_policy_url: 'https://ai.google.dev/gemma/prohibited_use_policy',
			terms_version: '2026-04-01',
			accepted_by: null,
			accepted_via_access_key: null,
			accepted_at: null,
			declined_at: null,
			model_present: false,
			indexed_at: null,
			recipes_not_yet_indexed: 0,
		},
		search_recipes: { query: null, closest: false, recipes: [] },
	});
</script>

<Kamosu client={kamosu.client} {room}>
	<Shell {pathname}>
		{#snippet screen(bare)}
			<main data-bare={bare}>
				{#if holds === 'account'}
					<Account />
				{:else if holds === 'story'}
					<Story ending="about" />
				{:else if holds === 'shelf'}
					<Shelf tag={null} />
				{/if}
			</main>
		{/snippet}
	</Shell>
</Kamosu>
