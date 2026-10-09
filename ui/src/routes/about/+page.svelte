<!--
	`/about` — the story (#158) for anyone, signed in or not, with no Invite:
	a link somebody sends on its own. It ends asking the reader to get an Invite
	from whoever sent it, and its corner takes them Home, which is the sign-in
	form to somebody who has no account.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { m } from '$lib/paraglide/messages';
	import Story from '$lib/story/Story.svelte';
</script>

<svelte:head>
	<title>{m.app_name()}</title>
</svelte:head>

<Story ending="about" onClose={() => goto('/')} />
