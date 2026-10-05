<!--
	The frame round every screen: which navigation is drawn, by how much room the
	window has (ADR 0044, #194).

	- **Phone:** the header above and the tab bar below, as always.
	- **Wide:** the sidebar down the left edge and neither of those. The screen
	  keeps its own column, centred in what is left beside the sidebar, until
	  its own ticket widens it.
	- **Bare:** nothing, on any layout. The cooking screen (ADR 0011) is one Step
	  filling the window, used with wet hands, and a way to Shopping one stray
	  touch from the Step is the one thing it may not cost. The story (#158)
	  covers the whole window with corners of its own, so navigation drawn under
	  it would still be read out and tabbed through behind a page that hides it.
	- **Signing in, an Invite, a Cookbook Invite, a recovery link:** no sidebar
	  on the wide layout, so nothing suggests somebody is in the app before they
	  are. The mark and the name stand above the page in its place. The phone
	  layout draws these pages as it always has.

	The Room decides, not a style rule, so a screen test sees which one is drawn.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import { useRoom } from '$lib/room.svelte';
	import { story } from '$lib/story/showing.svelte';
	import Header from './Header.svelte';
	import Masthead from './Masthead.svelte';
	import Sidebar from './Sidebar.svelte';
	import TabBar from './TabBar.svelte';
	import { signingIn } from './signing-in.svelte';

	interface Props {
		/** The address on screen. */
		pathname: string;
		/** The screen. It is told whether it is drawn bare. */
		screen: Snippet<[boolean]>;
	}

	let { pathname, screen }: Props = $props();

	const room = useRoom();

	const bare = $derived(pathname.startsWith('/cook/') || pathname === '/about' || story.showing);

	/**
	 * A page somebody may be reading before they are in the app: the account
	 * form wherever it is drawn, and the three link routes whoever opens them.
	 *
	 * The form is only drawn once an Operation has refused, so the moment
	 * before it goes by what this device last heard. That matters at the two
	 * addresses a stranger arrives at and where the form may follow: `/`, and
	 * `/import`, where a Share Link sends its reader. There the sidebar waits
	 * for a device that knows a Person is signed in.
	 *
	 * Everywhere else it is drawn whatever the device last heard. A page that
	 * could not be read for want of a Session is no sign-in page, and without
	 * the sidebar a wide window would hold no way from it to `/`, where the
	 * form is.
	 */
	const arriving = $derived(pathname === '/' || pathname === '/import');
	const outside = $derived(
		signingIn.showing ||
			(arriving && signingIn.signedIn !== true) ||
			/^\/(invite|recover|cookbook-invite)\//.test(pathname),
	);

	const sidebar = $derived(room.wide && !bare && !outside);
	const masthead = $derived(room.wide && !bare && outside);
	const headerAndTabs = $derived(!room.wide && !bare);
</script>

{#if headerAndTabs}
	<Header onSettings={pathname.startsWith('/settings')} />
{:else if sidebar}
	<Sidebar {pathname} />
{:else if masthead}
	<Masthead />
{/if}

<div class={sidebar ? 'beside-rail' : undefined}>
	{@render screen(bare)}
</div>

{#if headerAndTabs}
	<TabBar {pathname} />
{/if}
