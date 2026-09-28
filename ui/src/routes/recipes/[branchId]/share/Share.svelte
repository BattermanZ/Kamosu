<!--
	The share screen (#65, ADR 0026, ADR 0018).

	Two levels of visibility and no third: this recipe is seen by its Kitchen, or
	by anyone holding its link. So there is no scale on this screen and nothing
	to set a point on — one switch, on or off.

	THE STANDING LINE is on the screen at all times, in the same words every
	time. It is not a dialog, there is nothing to dismiss, and there is no
	first-time special case (ADR 0018): a warning seen eighteen months ago is a
	warning that is not there, and its absence afterwards reads as reassurance.
	It sits above the switch rather than beneath it, because it is what the act
	means rather than a footnote to it.

	WHAT TRAVELS WITH IT IS SAID PLAINLY, on the screen, above the switch (#50,
	ADR 0008): "this recipe includes Pizza Dough, which will be readable through
	this link". A Component travels with its parent as a Passenger, because a
	recipe that cannot tell you how to make its own dough is incomplete — and
	that is a thing a person is owed BEFORE they turn sharing on, in the same
	place and the same voice as the standing line, rather than as a dialog to
	dismiss. It does not change the dough's own Visibility: the dough gets no
	page and no link of its own, and is read only through this one.

	THE LINK IS SHOWN FOR AS LONG AS IT LIVES (#171, ADR 0031 as amended), in
	the block it had at minting: the address, Copy and Open, above "End the
	link" (option 1, Aurélien, 26 September 2026). Sending the recipe to a
	second person is copying it again, never ending the link and minting a new
	one, which would break every copy already sent. A link minted before Kamosu
	kept its address has none to show, and the screen says so and how to get one.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';
	import { Online, thisDevice, type Device } from '$lib/offline/device.svelte';
	import { BUNDLE, shareFile, sharesFiles, useFiles } from '$lib/api/files';
	import type { ExportBundleOutput, GetShareLinkOutput } from '$lib/api/catalogue';

	interface Props {
		branchId: string;
		/**
		 * Whether this is the app installed on an iPhone or iPad, where the
		 * recipe file goes to the share sheet rather than to an address (#156).
		 * A test hands in its own.
		 */
		device?: Pick<Device, 'installed' | 'apple'>;
	}

	let { branchId, device = thisDevice() }: Props = $props();

	const kamosu = useKamosu();
	const files = useFiles();
	/** Whether the file can be described or handed over at all (#76). */
	const online = new Online();

	let link = $state<GetShareLinkOutput | undefined>(undefined);
	/**
	 * The recipes this one is made of, which a link carries with it (#50). Read
	 * from the recipe rather than from the link, because it is true whether or
	 * not sharing is on — it is what turning it on would mean.
	 */
	let passengers = $state<string[]>([]);
	/**
	 * What the recipe file would hold, read when the screen opens (#65, #66).
	 *
	 * Read up front and not on the tap, because this screen says what an act
	 * means **before** you do it: the standing line does, the line about
	 * components does, and a file that announced itself only once it had gone
	 * would be the one thing here that breaks the habit (option C, Aurélien,
	 * 20 September 2026). The photograph that has gone missing is the case that
	 * proves it — worth knowing beforehand, worth nothing after.
	 *
	 * It describes the **file** rather than the recipe, so the Components it
	 * names are the ones actually travelling in the zip.
	 */
	let file = $state<ExportBundleOutput | undefined>(undefined);
	let address = $state('');
	let working = $state(false);
	let copied = $state(false);
	let failed = $state<string | undefined>(undefined);
	/** Where taking the recipe file has got to, in the installed app on Apple (#156). */
	let saving = $state<'idle' | 'getting' | 'ready' | 'failed'>('idle');
	/**
	 * The recipe file fetched and waiting for the tap that shares it (#156).
	 * Set only while `saving` is `ready`, and dropped with the screen.
	 */
	let prepared: File | undefined;
	/** Which recipe the screen is on, so a file fetched for the last one is not kept. */
	let visit = 0;

	$effect(() => {
		let current = true;
		void (async () => {
			try {
				const read = await kamosu.getShareLink({ branch_id: branchId });
				if (current) link = read;

				const recipe = await kamosu.getRecipe({ branch_id: branchId });
				if (!current) return;
				// Named once each, in the order the recipe meets them. A
				// Component this instance does not hold has no name to give and
				// nothing to carry, so it is not promised.
				passengers = [
					...new Set(
						(recipe.versions.at(-1)?.components ?? [])
							.filter((component) => component.held && !component.stopped)
							.map((component) => component.title)
							.filter((title): title is string => Boolean(title)),
					),
				];
			} catch (error) {
				if (!(error instanceof OperationError)) throw error;
				if (current) failed = m.share_failed();
			}
		})();
		return () => {
			current = false;
		};
	});

	/**
	 * What the recipe file would hold, read on its own.
	 *
	 * Apart from the link's own read on purpose, twice over. The file has
	 * nothing to do with the link, so a failure here must not say "the link
	 * could not be changed" — and this read is the one that is *expected* to
	 * fail, because `export_bundle` is answered straight through and never kept
	 * on the phone (`$lib/offline/reads`). Off the network it simply does not
	 * arrive, and the block below says so rather than disappearing.
	 */
	$effect(() => {
		let current = true;
		const asked = branchId;
		// A file fetched or ready for the last recipe is that recipe's (#156).
		visit += 1;
		saving = 'idle';
		prepared = undefined;
		void (async () => {
			try {
				const described = await kamosu.exportBundle({ branch_id: asked });
				if (current) file = described;
			} catch {
				// Nothing to say here. Offline the block already says what it
				// is waiting for, and a server that cannot describe the file
				// cannot hand it over either.
				if (current) file = undefined;
			}
		})();
		return () => {
			current = false;
		};
	});

	/** Whether the instance already knows where it is reachable from outside. */
	const knowsAddress = $derived(Boolean(link?.public_address));

	async function turnOn() {
		if (working) return;
		const offered = address.trim();
		if (!knowsAddress && !offered) {
			failed = m.share_address_missing();
			return;
		}
		working = true;
		failed = undefined;
		try {
			const answer = await kamosu.shareRecipe(
				knowsAddress ? { branch_id: branchId } : { branch_id: branchId, public_address: offered },
			);
			link = answer;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		} finally {
			working = false;
		}
	}

	async function end() {
		if (working) return;
		working = true;
		failed = undefined;
		try {
			link = await kamosu.endShareLink({ branch_id: branchId });
			copied = false;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		} finally {
			working = false;
		}
	}

	async function copy() {
		if (!link?.url) return;
		await navigator.clipboard.writeText(link.url);
		copied = true;
	}

	/** The recipes travelling inside the file besides this one (#50, ADR 0008). */
	const carried = $derived(
		(file?.passengers ?? [])
			.map((passenger) => passenger.title)
			.filter((title): title is string => Boolean(title)),
	);

	/**
	 * Take the file.
	 *
	 * A plain navigation and not a fetch: the server answers the zip under a
	 * Content-Disposition, the Session cookie travels with the request, and the
	 * browser saves it without this screen going anywhere. Nothing is held in
	 * memory, which matters for a recipe whose photographs run to megabytes.
	 *
	 * In the app installed on an iPhone or iPad that address would open in the
	 * app's own window, which has no Share or Save to Files (#156). There the
	 * file is fetched here, and this same button then shares it (`shareIt`;
	 * option A, Aurélien, 27 September 2026), as a Sheet is (#149). Holding
	 * the file until the second tap is the cost of that path.
	 */
	async function save() {
		if (!file) return;
		if (!sharesFiles(device, BUNDLE)) {
			window.location.assign(file.fetch_at);
			return;
		}
		const forVisit = visit;
		saving = 'getting';
		const leftBehind = () => closed || visit !== forVisit;
		try {
			const fetched = await files(file.fetch_at, BUNDLE);
			// Left while it was fetched: nothing is kept for a page not shown.
			if (leftBehind()) return;
			prepared = fetched;
			saving = 'ready';
		} catch {
			if (!leftBehind()) saving = 'failed';
		}
	}

	/** Set once this screen closes, so a file still being fetched is not kept. */
	let closed = false;
	$effect(() => () => {
		closed = true;
	});

	/**
	 * Hand the fetched file to the share sheet, straight from the tap with
	 * nothing awaited first (#149). Closing the share sheet keeps the file for
	 * another try; once it has gone somewhere, the next tap fetches afresh.
	 */
	function shareIt() {
		const held = prepared;
		if (!held) return;
		void shareFile(held).then((outcome) => {
			if (outcome === 'kept' || prepared !== held) return;
			prepared = undefined;
			saving = outcome === 'shared' ? 'idle' : 'failed';
		});
	}
</script>

<Screen title={m.share_title()} blurb={m.share_blurb()}>
	<!-- The standing line: the same words every time, on this screen and on the
	     export-a-bundle path alike. -->
	<p class="border-l-2 border-accent py-1 pl-4 text-read text-ink-2">{m.share_standing()}</p>

	<!--
		What a link carries besides this recipe (#50, ADR 0008). Beside the
		standing line and in the same voice: it is part of what sharing means,
		not a footnote to it, and it is on screen whether the link is on or off.
	-->
	{#if passengers.length}
		<p class="mt-3 border-l-2 border-support-2 py-1 pl-4 text-read text-ink-2">
			{m.share_carries_components({ recipes: passengers.join(', ') })}
		</p>
	{/if}

	{#if failed}
		<p class="mt-4 text-read text-support" role="alert">{failed}</p>
	{/if}

	{#if !link && !failed}
		<!-- Only while there is still something to wait for. A read that failed
		     has already said so above, and a screen that goes on saying
		     "Loading…" underneath it is telling the reader to keep waiting for
		     something that is not coming. -->
		<p class="mt-6 text-body text-ink-2">{m.loading()}</p>
	{:else if link?.shared}
		<p class="mt-6 text-body">{m.share_is_shared()}</p>
		{#if link.shared_by && link.created_at}
			<p class="mt-1 text-read text-ink-2">
				{m.share_shared_by({
					name: link.shared_by,
					when: new Date(link.created_at).toLocaleDateString(),
				})}
			</p>
		{/if}

		{#if link.url}
			<p class="mt-4 border border-rule bg-card p-3 text-read break-all">{link.url}</p>
			<button
				type="button"
				onclick={copy}
				class="mt-3 block w-full bg-accent p-4 text-center font-display text-body text-on-accent"
			>
				{copied ? m.share_copied() : m.share_copy()}
			</button>
			<a
				href={link.url}
				target="_blank"
				rel="noreferrer"
				class="mt-2 block border border-rule p-4 text-center font-display text-body text-accent"
			>
				{m.share_open()}
			</a>
		{:else}
			<!-- Minted before Kamosu kept a link's address: it still opens for
			     whoever holds it, but there is nothing here to copy (#171). -->
			<p class="mt-4 border-l-2 border-support-2 py-1 pl-4 text-read text-ink-2">
				{m.share_link_made_before()}
			</p>
		{/if}

		<button
			type="button"
			onclick={end}
			disabled={working}
			class="mt-6 block w-full border border-support p-4 text-center font-display text-body text-support"
		>
			{working ? m.share_ending() : m.share_end()}
		</button>
	{:else if link}
		<p class="mt-6 text-body text-ink-2">{m.share_not_shared()}</p>

		{#if !knowsAddress}
			<!-- Asked once, at the first Share Link, and kept in the database —
			     never an environment variable (#65). -->
			<label class="mt-6 block text-label text-ink-2 uppercase" for="public-address">
				{m.share_address_label()}
			</label>
			<input
				id="public-address"
				type="url"
				bind:value={address}
				placeholder="https://kamosu.example.com"
				class="mt-1 w-full rounded-sm border border-rule bg-card p-3 text-body"
			/>
			<p class="mt-2 text-read text-ink-2">{m.share_address_hint()}</p>
		{/if}

		<button
			type="button"
			onclick={turnOn}
			disabled={working}
			class="mt-6 block w-full bg-accent p-4 text-center font-display text-body text-on-accent"
		>
			{m.share_turn_on()}
		</button>
	{/if}

	<!--
		The recipe file (#65, #66, ADR 0020), described before it is taken.

		Outside the link's own branches on purpose: the file has nothing to do
		with the link and is offered whether sharing is on or off. It is the one
		way a recipe leaves Kamosu, wanted as readily for a backup or for moving
		to another instance as for a friend — which is why the line says a Kamosu
		takes it in, the half of the format nobody can guess from a zip.

		It waits for the server, since the server is what builds it.
	-->
	{#if file || !online.current}
		<div class="mt-8 border border-rule bg-card p-4">
			<p class="text-label text-ink-2 uppercase">{m.share_file_label()}</p>
			{#if file}
				<p class="mt-2 text-body">{file.file_name}</p>
				<p class="mt-1 text-read text-ink-2">
					{m.share_file_holds({
						notes:
							file.notes.length === 1
								? m.share_file_notes_one()
								: m.share_file_notes({ count: file.notes.length }),
						photographs:
							file.photographs === 0
								? m.share_file_photographs_none()
								: file.photographs === 1
									? m.share_file_photographs_one()
									: m.share_file_photographs({ count: file.photographs }),
					})}
				</p>
				<p class="mt-1 text-read text-ink-2">{m.share_file_opens()}</p>
				{#if carried.length}
					<p class="mt-2 text-read text-support-2">
						{carried.length === 1
							? m.share_file_carries_one({ recipes: carried[0] })
							: m.share_file_carries({ recipes: carried.join(', ') })}
					</p>
				{/if}
				{#if file.missing_photographs.length}
					<p class="mt-2 text-read text-support">
						{file.missing_photographs.length === 1
							? m.share_file_missing_one()
							: m.share_file_missing({ count: file.missing_photographs.length })}
					</p>
				{/if}
			{:else}
				<!-- Offline. `export_bundle` is never kept on the phone, so what
				     the file holds cannot be said here — but the button stays
				     where it is rather than vanishing from under the reader. -->
				<p class="mt-2 text-read text-ink-2">{m.share_file_waiting()}</p>
			{/if}
			{#if saving === 'ready'}
				<!-- The file is already on the phone, so sharing it waits for
				     nothing (#156): the same button, no longer one that needs
				     the server. -->
				<button
					type="button"
					onclick={shareIt}
					class="mt-4 block w-full border border-rule p-3 text-center font-display text-body text-accent"
				>
					{m.share_file_share()}
				</button>
			{:else}
				<NeedsServer
					label={saving === 'getting' ? m.share_file_getting() : m.share_file_save()}
					waiting={m.offline_waits_file()}
					onclick={save}
					disabled={saving === 'getting'}
					shapeClass="mt-4 block w-full p-3 text-center font-display text-body"
					lookClass="border border-rule text-accent"
				/>
			{/if}
			{#if saving === 'failed'}
				<p class="mt-2 text-read text-support" role="alert">{m.share_file_failed()}</p>
			{/if}
		</div>
	{/if}
</Screen>
