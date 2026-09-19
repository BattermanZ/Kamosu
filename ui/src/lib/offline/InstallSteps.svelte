<!--
	How to put Kamosu on the Home Screen. Explained rather than offered as a
	button, because no browser on an iPhone lets a page trigger it — adding is
	always Share, then Add to Home Screen (ADR 0013). Shared by the card and by
	Settings, where the same fact lives once the card is put away.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';

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
	<p class="mt-2">{m.offline_install_other_step()}</p>
{/if}
