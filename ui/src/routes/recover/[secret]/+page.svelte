<!--
	`/recover/<secret>` — the page a recovery link opens: the account form asking
	only for a new password, and Home once it is set (#126).

	Whether the link is still good is learned by using it. A spent, revoked or
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

<Account link="/recover/{params.secret}" onSignedIn={() => goto('/')} />
