<!--
	`/invite/<secret>` — the page an Invite link opens: the account form in its
	Invite mode, and Home once it has made the account (#126).

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
	import type { PageProps } from './$types';

	let { params }: PageProps = $props();
</script>

<Account link="/invite/{params.secret}" onSignedIn={() => goto('/')} />
