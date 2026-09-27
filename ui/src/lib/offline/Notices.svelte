<!--
	Which "not right now" card the top of the page carries, if any (#76).

	One at a time, the most pressing first: offline, then an insecure
	connection, then the library waiting, then the Home Screen. Each is said
	once and put away. Putting away the warning and the nudge is remembered on
	this device, and Settings keeps both facts for anyone who wants them back.
	The offline card is put away for as long as this spell offline lasts, and
	"When I'm on wifi" only until Kamosu is next opened, because on an iPhone
	that tap is the only way the library ever arrives.

	Nothing here ever stands between a person and the page: a stranger arriving
	from a Share Link must not meet a wall (ADR 0013).
-->
<script lang="ts">
	import { untrack } from 'svelte';
	import { m } from '$lib/paraglide/messages';
	import Notice from './Notice.svelte';
	import InstallSteps from './InstallSteps.svelte';
	import { Online, install, onNetworkChange, thisDevice, type Device } from './device.svelte';
	import { readableSize, sessions, useLibrary } from './library.svelte';
	import { standing } from './standing.svelte';
	import { putAway, wasPutAway } from './put-away';

	interface Props {
		/** The facts the cards are chosen from. The app reads this browser; a test brings its own. */
		device?: Device;
	}

	let { device = thisDevice() }: Props = $props();

	const online = new Online();
	const library = useLibrary();

	/** The spell offline whose card was put away. */
	let offlineAway = $state<number | undefined>(undefined);
	let insecureAway = $state(wasPutAway('insecure'));
	let installAway = $state(wasPutAway('install'));
	let libraryLater = $state(false);

	$effect(() => {
		// Looked at again whenever a Session begins here.
		void sessions.began;
		untrack(() => void library.start(device.wifi()));
		// Where the phone says when it reaches wifi, a waiting library starts then.
		return onNetworkChange(() => {
			if (library.phase === 'absent' && device.wifi() === true) void library.fill();
		});
	});

	const card = $derived.by(() => {
		if (!online.current) return offlineAway === online.spell ? undefined : 'offline';
		if (!device.secure) return insecureAway ? undefined : 'insecure';
		if (library.phase === 'filling') return 'filling';
		if (library.phase === 'absent' && !libraryLater) return 'library';
		if (!device.installed && !install.accepted && !installAway) return 'install';
		return undefined;
	});

	const when = (date: Date) => date.toLocaleDateString();
</script>

{#if card === 'offline'}
	<Notice
		title={m.offline_title()}
		actions={[{ label: m.notice_got_it(), act: () => (offlineAway = online.spell) }]}
	>
		{#if standing.branchId && library.onlyOpened(standing.branchId) && standing.keptAt}
			<p>{m.offline_cached({ date: when(standing.keptAt) })}</p>
		{:else if library.filledAt}
			<p>{m.offline_held()}</p>
		{:else}
			<p>{m.offline_unfilled()}</p>
		{/if}
	</Notice>
{:else if card === 'insecure'}
	<Notice
		title={m.offline_insecure_title()}
		tone="warning"
		actions={[
			{
				label: m.notice_got_it(),
				act: () => {
					putAway('insecure');
					insecureAway = true;
				},
			},
		]}
	>
		<p>{m.offline_insecure_body()}</p>
	</Notice>
{:else if card === 'library'}
	<Notice
		title={m.offline_library_title()}
		tone="pause"
		actions={[
			{ label: m.offline_library_fetch(), act: () => void library.fill(), primary: true },
			{ label: m.offline_library_later(), act: () => (libraryLater = true) },
		]}
	>
		<p>
			{m.offline_library_body({ count: library.count, size: readableSize(library.bytes) })}
		</p>
	</Notice>
{:else if card === 'filling'}
	<Notice title={m.offline_library_title()} tone="pause">
		<p>{m.offline_library_filling({ done: library.done, total: library.total })}</p>
	</Notice>
{:else if card === 'install'}
	<Notice
		title={m.offline_install_title()}
		actions={[
			{
				label: m.offline_not_now(),
				act: () => {
					putAway('install');
					installAway = true;
				},
			},
		]}
	>
		<InstallSteps apple={device.apple} />
	</Notice>
{/if}
