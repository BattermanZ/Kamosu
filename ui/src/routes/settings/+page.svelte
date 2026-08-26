<!--
	Settings: reached through the Kitchen's name, never from the tab bar.

	This is also where the signature transition is shown in the shell: the card
	bearing the Kitchen's name in the header and this screen's title carry the
	same `view-transition-name`, so the card grows into the page rather than the
	screen swapping (ADR 0012). Every recipe card that opens into its page later
	is this same pair, with a different name.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { getLocale, locales, setLocale, type Locale } from '$lib/paraglide/runtime';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';
	import Section from '$lib/shell/Section.svelte';
	import type { InstanceStatusOutput } from '$lib/api/catalogue';

	const kamosu = useKamosu();

	let status = $state<InstanceStatusOutput | undefined>(undefined);
	let failed = $state(false);

	// The one Operation this shell calls: the instance says what version it is and
	// whether setup has happened. Everything else arrives with later tickets.
	$effect(() => {
		let current = true;
		kamosu
			.instanceStatus()
			.then((answer) => {
				if (current) status = answer;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				if (current) failed = true;
			});
		return () => {
			current = false;
		};
	});

	const names: Record<Locale, () => string> = {
		en: () => m.language_en(),
		fr: () => m.language_fr(),
		es: () => m.language_es()
	};
</script>

<Screen title={m.settings_title()} expandsFrom="kitchen">
	<Section heading={m.settings_language()}>
		<!-- Paraglide compiles every phrase to a function, so this list can only
		     offer languages that were actually compiled. -->
		<ul class="flex flex-wrap gap-2">
			{#each locales as locale (locale)}
				<li>
					<button
						type="button"
						aria-pressed={getLocale() === locale}
						onclick={() => setLocale(locale)}
						class="min-h-12 rounded-sm border px-4 text-body
							{getLocale() === locale
							? 'border-accent bg-accent text-on-accent'
							: 'border-rule bg-card text-ink'}"
					>
						{names[locale]()}
					</button>
				</li>
			{/each}
		</ul>
	</Section>

	<Section heading={m.settings_instance()}>
		<p class="text-body text-ink-2">
			{#if failed}
				{m.instance_unreachable()}
			{:else if status}
				{m.instance_version({ version: status.version })} ·
				{status.setup_complete ? m.instance_setup_done() : m.instance_setup_pending()}
			{:else}
				{m.loading()}
			{/if}
		</p>
	</Section>
</Screen>
