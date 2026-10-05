<!--
	Brought in (#108): option B, "three sources, each with its own history", as
	Aurélien chose it on 22 September 2026 (the decision is a comment on #108).

	**Why this screen exists.** Forty-eight imports had run on the dev instance
	and exactly one was reachable — `ImportCrouton.svelte` offered a link to the
	newest Crouton job and nothing linked to any other. Every Report survived its
	screen, exactly as #68 promised, and no screen could find one.

	**Why it lists sources and not arrivals.** An Import is one durable channel
	per Kitchen per source, reused by every run after the first (ADR 0025), so
	three Imports sat behind those forty-eight runs. This screen is those three.
	The arrivals are one tap down, on the source's own screen, which is also the
	only place forgetting is offered — because `forget_import` throws away a
	whole channel's ledger and a screen entirely about one source is where that
	scope cannot be misread.

	**Nothing here is a provenance badge.** ADR 0025 keeps where a recipe came
	from off the recipe: an imported recipe is an ordinary recipe. This is a list
	of events, and the recipes it names are reached through the Report, on the
	recipe pages they already have.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { getLocale } from '$lib/paraglide/runtime';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';
	import WayBackLine from '$lib/shell/WayBackLine.svelte';
	import Section from '$lib/shell/Section.svelte';
	import Empty from '$lib/shell/Empty.svelte';
	import { sourceName, sourceSummary, type Import } from '$lib/imports';

	const kamosu = useKamosu();

	let imports = $state<Import[] | undefined>(undefined);
	/** The Core's own refusal, where one arrived. Its words, not a paraphrase. */
	let said = $state<string | undefined>(undefined);

	$effect(() => {
		let current = true;
		kamosu
			.listImports()
			.then((answer) => {
				if (current) imports = answer.imports;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				if (current) said = error.message || m.imports_unreachable();
			});
		return () => {
			current = false;
		};
	});

	const when = (at: string) =>
		new Date(at).toLocaleDateString(getLocale(), {
			day: 'numeric',
			month: 'long',
			year: 'numeric',
		});

	/** The newest arrival's date, which is what "last" on the card means. */
	const lastArrival = (source: Import) => source.arrivals[0]?.created_at;
</script>

<!-- The blurb is drawn here rather than passed to `Screen`, so the way back sits
     directly under the title as it does on the Report (#68) and everywhere else.
     Screen puts its own `blurb` between the two, which left the link floating. -->
<Screen title={m.imports_title()}>
	<WayBackLine href="/settings" label={m.imports_back()} />
	<p class="mt-2 text-read text-ink-2">{m.imports_blurb()}</p>

	<Section heading={m.imports_sources()}>
		{#if said}
			<p class="border-l-3 border-support bg-card px-3 py-2 text-body" role="alert">{said}</p>
		{:else if imports === undefined}
			<p class="text-body text-ink-2">{m.loading()}</p>
		{:else if imports.length === 0}
			<Empty>{m.imports_none()}</Empty>
		{:else}
			<!-- Keyed on source kind, which `list_imports` guarantees is unique: an
			     Import is unique per `(cookbook_id, source_kind)` and the answer
			     holds one Cookbook's, the caller's own (#131). Two rows of one
			     kind would be a duplicate key AND two identical hrefs. -->
			<ul class="grid gap-2">
				{#each imports as source (source.source_kind)}
					{@const last = lastArrival(source)}
					<li>
						<a
							href="/imports/from/{encodeURIComponent(source.source_kind)}"
							class="flex min-h-12 items-center gap-3 rounded-sm border border-rule bg-card px-3 py-3"
						>
							<span class="min-w-0 flex-1">
								<span class="block text-body font-medium text-ink">
									{sourceName(source.source_kind)}
								</span>
								<span class="block text-read text-ink-2">
									{sourceSummary(source)}
								</span>
								{#if last}
									<span class="block text-read text-ink-2">
										{m.imports_last({ date: when(last) })}
									</span>
								{/if}
							</span>
							<span aria-hidden="true" class="text-ink-2">›</span>
						</a>
					</li>
				{/each}
			</ul>
		{/if}
	</Section>
</Screen>
