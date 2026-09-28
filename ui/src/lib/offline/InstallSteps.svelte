<!--
	How to put Kamosu on the Home Screen. Shared by the card and by Settings,
	where the same fact lives once the card is put away.

	On an iPhone it is explained rather than offered as a button, because no
	browser there lets a page trigger it — adding is always Share, then Add to
	Home Screen (ADR 0013). Elsewhere a Chromium browser may offer to install
	Kamosu, and while it does the steps give way to a button that opens the
	browser's own dialog (#173, option B — Aurélien, 27 September 2026). Where
	it does not, in Firefox or after the dialog was turned down, the steps are
	written out.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { install, installKamosu } from './device.svelte';

	interface Props {
		apple: boolean;
	}

	let { apple }: Props = $props();
</script>

{#if apple}
	<p>{m.offline_install_apple()}</p>
	<ol class="mt-2 list-decimal pl-6">
		<li>
			{m.offline_install_apple_share()}
			<svg
				viewBox="0 0 14 16"
				aria-hidden="true"
				class="inline-block h-4 w-4 align-[-2px] text-accent"
				fill="none"
				stroke="currentColor"
				stroke-width="1.4"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<path d="M7 1v9M4 4l3-3 3 3M4.5 7H2v8h10V7H9.5" />
			</svg>
		</li>
		<li>{m.offline_install_apple_add()}</li>
	</ol>
{:else}
	<p>{m.offline_install_other()}</p>
	<!-- Once installed from here there is nothing left to do, and this tab still
	     is not the app, so the reason stands alone: neither the button nor the
	     steps. -->
	{#if install.offer && !install.accepted}
		<!-- Drawn like a card's main action, but here in the body: the card's own
		     row keeps only "Not now", and Settings has no card to put it in. -->
		<button
			type="button"
			onclick={() => void installKamosu()}
			class="mt-3 w-full border border-accent bg-accent p-3 text-read text-on-accent"
		>
			{m.offline_install_other_button()}
		</button>
	{:else if !install.accepted}
		<ol class="mt-2 list-decimal pl-6">
			<li>
				{m.offline_install_other_menu()}
				<svg
					viewBox="0 0 4 16"
					aria-hidden="true"
					class="inline-block h-4 w-2 align-[-2px] text-accent"
					fill="currentColor"
				>
					<circle cx="2" cy="3" r="1.5" />
					<circle cx="2" cy="8" r="1.5" />
					<circle cx="2" cy="13" r="1.5" />
				</svg>
			</li>
			<li>{m.offline_install_other_choose()}</li>
		</ol>
	{/if}
{/if}
