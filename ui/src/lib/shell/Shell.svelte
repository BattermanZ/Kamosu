<!--
	The frame round every screen: which navigation is drawn, by how much room the
	window has (ADR 0044, #194).

	- **Phone:** the header above and the tab bar below, as always.
	- **Wide:** the sidebar down the left edge and neither of those. The screen
	  keeps its own column, centred in what is left beside the sidebar, until
	  its own ticket widens it.
	  A page the sidebar does not list carries a back arrow (#195), since a
	  tablet's installed app has no browser button to go back with. The phone
	  carries the same arrow on the same pages since #219.
	- **Bare:** nothing, on any layout. The cooking screen (ADR 0011) is one Step
	  filling the window, used with wet hands, and a way to Shopping one stray
	  touch from the Step is the one thing it may not cost. The story (#158)
	  covers the whole window with corners of its own, so navigation drawn under
	  it would still be read out and tabbed through behind a page that hides it.
	- **Signing in, an Invite, a recovery link:** no sidebar on the wide layout
	  and no tab bar or gear on the phone, so nothing suggests somebody is in
	  the app before they are (#214). The mark and the name stand on the page
	  all the same: centred above it on the wide layout, in the header's place
	  on the phone. One answer, `outside`, decides it for both layouts.
	- **A Cookbook Invite:** the same for somebody not signed in. A Person who
	  is signed in is in the app, and keeps their navigation there (#214,
	  choice of 11 October 2026).

	The Room decides, not a style rule, so a screen test sees which one is drawn.

	`/` goes to the recipe search wherever the navigation is drawn (#203). The
	cooking screen and the story take no key of Kamosu's, and neither does a
	page somebody reads before they are in the app.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import { useRoom } from '$lib/room.svelte';
	import { story } from '$lib/story/showing.svelte';
	import BackArrow from './BackArrow.svelte';
	import Header from './Header.svelte';
	import Masthead from './Masthead.svelte';
	import Sidebar from './Sidebar.svelte';
	import TabBar from './TabBar.svelte';
	import { signingIn } from './signing-in.svelte';
	import { slashToSearch } from './slash';
	import { parentOf } from './way-back';

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
	 * form wherever it is drawn, and an Invite or a recovery link whoever
	 * opens it, since each of those makes or mends an account.
	 *
	 * The form is only drawn once an Operation has refused, so the moment
	 * before it goes by what this device last heard. That matters at the
	 * addresses a stranger arrives at and where the form may follow: `/`,
	 * `/import`, where a Share Link sends its reader, and a Cookbook Invite,
	 * which is for a Person and asks anybody else to sign in. There the
	 * navigation waits for a device that knows a Person is signed in.
	 *
	 * Everywhere else it is drawn whatever the device last heard. A page that
	 * could not be read for want of a Session is no sign-in page, and without
	 * its navigation a window would hold no way from it to `/`, where the
	 * form is.
	 */
	const arriving = $derived(
		pathname === '/' || pathname === '/import' || pathname.startsWith('/cookbook-invite/'),
	);
	const outside = $derived(
		signingIn.showing ||
			(arriving && signingIn.signedIn !== true) ||
			/^\/(invite|recover)\//.test(pathname),
	);

	const sidebar = $derived(room.wide && !bare && !outside);
	const masthead = $derived(room.wide && !bare && outside);
	const header = $derived(!room.wide && !bare);
	const tabs = $derived(header && !outside);

	/** Where this page's back arrow goes with nothing behind it, on a page that carries one. */
	const backTo = $derived(sidebar || tabs ? parentOf(pathname) : undefined);

	function onKey(event: KeyboardEvent) {
		if (!bare && !outside) slashToSearch(event);
	}
</script>

<svelte:window onkeydown={onKey} />

{#if header}
	<Header onSettings={pathname.startsWith('/settings')} bare={outside} />
{:else if sidebar}
	<Sidebar {pathname} />
{:else if masthead}
	<Masthead />
{/if}

<div class={[sidebar && 'beside-rail', backTo && 'clears-back-arrow']}>
	{#if backTo}
		<BackArrow parent={backTo} layout={sidebar ? 'wide' : 'phone'} />
	{/if}
	{@render screen(bare)}
</div>

{#if tabs}
	<TabBar {pathname} />
{/if}
