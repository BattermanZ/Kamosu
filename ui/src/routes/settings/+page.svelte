<!--
	Settings: reached through the gear in the header (#102, #218) on the phone
	and the rail's fifth place on the wide layout, never from the tab bar.

	A menu of four rows (#221, option 2 of the audit of 9 October 2026), each
	opening a page of its own: Account, This device, Kitchens and cookbooks,
	This instance. The sections themselves are drawn by `Settings.svelte` on
	the page that holds them, and nothing is read here but the words.

	This is also where the signature transition is shown in the shell: the gear
	and this screen's title carry the same `view-transition-name`, so the gear
	grows into the page rather than the screen swapping (ADR 0012). Every recipe
	card that opens into its page later is this same pair, with a different name.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import Screen from '$lib/shell/Screen.svelte';
	import { PAGES } from './pages';
</script>

<Screen title={m.settings_title()} expandsFrom="settings">
	<ul class="border-t border-rule">
		{#each PAGES as entry (entry.page)}
			<li class="border-b border-rule">
				<a
					href={entry.href}
					class="flex min-h-[64px] items-center justify-between gap-3 py-[10px] text-ink"
				>
					<span class="min-w-0">
						<span class="block font-display text-line font-semibold">{entry.title()}</span>
						<span class="block text-read text-ink-2">{entry.inside()}</span>
					</span>
					<svg
						viewBox="0 0 24 24"
						aria-hidden="true"
						class="h-6 w-6 shrink-0 text-ink-2"
						fill="none"
						stroke="currentColor"
						stroke-width="1.75"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<path d="m9 5 7 7-7 7" />
					</svg>
				</a>
			</li>
		{/each}
	</ul>
</Screen>
