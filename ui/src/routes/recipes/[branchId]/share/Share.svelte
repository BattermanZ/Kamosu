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

	THE LINK IS SHOWN ONCE, at the moment it is minted, and the screen says so.
	Kamosu keeps only the Secret's hash, exactly as it does for every other
	Secret (ADR 0031), so afterwards this screen can say a link exists and
	cannot reprint it. That is honest about what the instance actually holds,
	and it is why the copy button is offered while it can still do something.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';
	import type { GetShareLinkOutput } from '$lib/api/catalogue';

	interface Props {
		branchId: string;
	}

	let { branchId }: Props = $props();

	const kamosu = useKamosu();

	let link = $state<GetShareLinkOutput | undefined>(undefined);
	/** The link itself, held only for as long as this screen is open. */
	let minted = $state<string | undefined>(undefined);
	let address = $state('');
	let working = $state(false);
	let copied = $state(false);
	let failed = $state<string | undefined>(undefined);

	$effect(() => {
		let current = true;
		void (async () => {
			try {
				const read = await kamosu.getShareLink({ branch_id: branchId });
				if (current) link = read;
			} catch (error) {
				if (!(error instanceof OperationError)) throw error;
				if (current) failed = m.share_failed();
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
			minted = answer.url ?? undefined;
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
			minted = undefined;
			copied = false;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		} finally {
			working = false;
		}
	}

	async function copy() {
		if (!minted) return;
		await navigator.clipboard.writeText(minted);
		copied = true;
	}
</script>

<Screen title={m.share_title()} blurb={m.share_blurb()}>
	<!-- The standing line: the same words every time, on this screen and on the
	     export-a-bundle path alike. -->
	<p class="border-l-2 border-accent py-1 pl-4 text-read text-ink-2">{m.share_standing()}</p>

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

		{#if minted}
			<p class="mt-4 border border-rule bg-card p-3 text-read break-all">{minted}</p>
			<p class="mt-2 text-read text-ink-2">{m.share_link_shown_once()}</p>
			<button
				type="button"
				onclick={copy}
				class="mt-3 block w-full bg-accent p-4 text-center font-display text-body text-on-accent"
			>
				{copied ? m.share_copied() : m.share_copy()}
			</button>
			<a
				href={minted}
				target="_blank"
				rel="noreferrer"
				class="mt-2 block border border-rule p-4 text-center font-display text-body text-accent"
			>
				{m.share_open()}
			</a>
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
</Screen>
