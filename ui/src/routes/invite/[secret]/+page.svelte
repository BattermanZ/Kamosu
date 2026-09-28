<!--
	`/invite/<secret>` — the page an Invite link opens: the story first (#158),
	then the account form in its Invite mode, and Home once it has made the
	account (#126). The story's last button is the way to the form; Skip reaches
	that button from any stage. An Invite that makes an Operator reads the same.

	Whether the Invite is still good is learned by using it. A spent, revoked or
	unknown one is refused when the form is sent, in the Core's own words.

	The secret arrives as the `params` prop rather than off `$app/state`, which
	every other route reads. That is so a test can render this page itself, with
	a secret of its choosing (`LinkTestHarness`): what went missing in #126 was
	the route, not the form, so the route is what the test has to hold.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import Account from '../../Account.svelte';
	import Story from '$lib/story/Story.svelte';
	import type { PageProps } from './$types';

	let { params }: PageProps = $props();

	let reading = $state(true);
</script>

{#if reading}
	<Story ending="invite" onCreate={() => (reading = false)} />
{:else}
	<Account link="/invite/{params.secret}" onSignedIn={() => goto('/')} />
{/if}
