<!--
	The shell as the root layout draws it, for its screen test: inside the
	provider, in the Room the test names, round a screen that says whether it
	was drawn bare and holds the account form or the story when asked to.
-->
<script lang="ts">
	import { standIn } from '$lib/api/stand-in';
	import type { Room } from '$lib/room.svelte';
	import Account from './Account.svelte';
	import Story from '$lib/story/Story.svelte';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Shell from '$lib/shell/Shell.svelte';

	interface Props {
		pathname: string;
		room: Room;
		/** What the screen holds: the account form, as `/` does for a stranger, or the story. */
		holds?: 'account' | 'story';
	}

	let { pathname, room, holds }: Props = $props();

	const kamosu = standIn({
		instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
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
				{/if}
			</main>
		{/snippet}
	</Shell>
</Kamosu>
