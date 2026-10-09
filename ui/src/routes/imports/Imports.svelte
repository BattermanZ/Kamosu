<!--
	Brought in (#108): option B, "three sources, each with its own history", as
	chosen on 22 September 2026 (the decision is a comment on #108).

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

	**On the wide layout this is the list beside the open Report** (#200, ADR
	0044), drawn by `BroughtIn.svelte`. A list that stays beside a Report has
	to list the imports themselves, so there it is every arrival under a
	heading for its source: the date and time, and under it what happened, in
	the words the source's own screen uses. The heading is the way to that
	screen, which stays a full page and is still the only place forgetting is
	offered. This was chosen on 6 October 2026 over one run of every import
	with the sources at its foot.

	**The arrows walk it**, as they walk Foods (#199). Up and down open the
	Report above or below the open one, in place of it, so going back leaves
	the list in one step. They pass from one source's arrivals to the next. Tab
	reaches each source's heading and one row.
-->
<script lang="ts">
	import { aDate } from '$lib/dates';
	import { m } from '$lib/paraglide/messages';
	import { untrack } from 'svelte';
	import { useKamosu } from '$lib/kamosu';
	import { useRoom } from '$lib/room.svelte';
	import { openInPlace, stepInList, typingInAField } from '$lib/shell/walk';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';
	import Section from '$lib/shell/Section.svelte';
	import Empty from '$lib/shell/Empty.svelte';
	import { sourceName, sourceSummary, whatHappened, whenArrived, type Import } from '$lib/imports';
	import { useEnded } from './ended.svelte';

	interface Props {
		/** The import whose Report is open beside this list, on the wide layout. */
		open?: string;
	}

	let { open }: Props = $props();

	const kamosu = useKamosu();
	const room = useRoom();
	const ended = useEnded();

	/** Where a Report is, on every layout. */
	const addressOf = (jobId: string) => `/imports/${jobId}`;
	const sourceAddress = (kind: string) => `/imports/from/${encodeURIComponent(kind)}`;

	let imports = $state<Import[] | undefined>(undefined);
	/** The Core's own refusal, where one arrived. Its words, not a paraphrase. */
	let said = $state<string | undefined>(undefined);

	/**
	 * Counts each Report opened from a row that still says its import is
	 * running. That Report may find the import long ended and so watch nothing
	 * end, and the row would go on saying it was running.
	 */
	let openedWhileRunning = $state(0);
	$effect(() => {
		const id = open;
		if (id === undefined) return;
		untrack(() => {
			const row = imports?.flatMap((source) => source.arrivals).find((a) => a.job_id === id);
			if (row?.status === 'queued' || row?.status === 'running') openedWhileRunning += 1;
		});
	});

	// Read on arrival, again each time the Report beside this list watches its
	// import end, and again for a row opened while it said running. What is
	// drawn stays until the new reading is in.
	$effect(() => {
		void ended?.count;
		void openedWhileRunning;
		let current = true;
		kamosu
			.listImports()
			.then((answer) => {
				if (current) imports = answer.imports;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				// A second reading that fails leaves the list that is drawn.
				if (current && imports === undefined) said = error.message || m.imports_unreachable();
			});
		return () => {
			current = false;
		};
	});

	const when = (at: string) => aDate(at);

	/** The newest arrival's date, which is what "last" on the card means. */
	const lastArrival = (source: Import) => source.arrivals[0]?.created_at;

	/** Every import in the order the list draws them, which is the order the arrows walk. */
	const drawn = $derived(
		(imports ?? []).flatMap((source) => source.arrivals.map((arrival) => arrival.job_id)),
	);

	/** The row Tab stops on: the open Report's, or the first where none of these is open. */
	const tabStop = $derived(open !== undefined && drawn.includes(open) ? open : drawn[0]);

	let rows = $state<HTMLElement | undefined>(undefined);

	function walk(event: KeyboardEvent) {
		if (!room.wide || typingInAField(event)) return;
		const to = stepInList(event, drawn, open);
		if (to === undefined) return;
		event.preventDefault();
		if (to === open) return;
		const row = rows?.querySelector<HTMLElement>(`a[href="${addressOf(to)}"]`);
		// The caret goes to the row, so Tab goes on into the Report it opened.
		row?.focus({ preventScroll: true });
		row?.scrollIntoView?.({ block: 'nearest' });
		void openInPlace(addressOf(to));
	}
</script>

<svelte:window onkeydown={walk} />

{#snippet nothingToList()}
	{#if said}
		<p class="border-l-3 border-support bg-card px-3 py-2 text-body" role="alert">{said}</p>
	{:else if imports === undefined}
		<p class="text-body text-ink-2">{m.loading()}</p>
	{:else if imports.length === 0}
		<Empty>{m.imports_none()}</Empty>
	{/if}
{/snippet}

<!-- The list beside what is open draws no Screen, so it names the tab itself (#224). -->
<svelte:head>
	{#if room.wide}
		<title>{m.imports_title()} · {m.app_name()}</title>
	{/if}
</svelte:head>

{#if room.wide}
	<div class="list-column">
		<div class="border-b border-rule px-gutter pb-3">
			<!-- The page's title until a Report is open, whose own title is the title then. -->
			<svelte:element
				this={open === undefined ? 'h1' : 'h2'}
				class="beside-back-arrow flex min-h-12 items-center font-display text-title font-semibold"
			>
				{m.imports_title()}
			</svelte:element>
		</div>
		<div class="min-h-0 flex-1 overflow-y-auto pb-6" bind:this={rows}>
			{#if said || !imports?.length}
				<div class="mt-4 px-gutter">{@render nothingToList()}</div>
			{:else}
				<!-- Keyed on source kind, as the phone's list is and for its reason. -->
				{#each imports as source (source.source_kind)}
					<a
						href={sourceAddress(source.source_kind)}
						class="flex items-center gap-3 border-b border-rule px-gutter pt-4 pb-2"
					>
						<span class="min-w-0 flex-1">
							<span class="block text-label font-medium text-accent uppercase">
								{sourceName(source.source_kind)}
							</span>
							<span class="block text-read text-ink-2">{sourceSummary(source)}</span>
						</span>
						<span aria-hidden="true" class="text-ink-2">›</span>
					</a>
					<ul>
						{#each source.arrivals as arrival (arrival.job_id)}
							{@const happened = whatHappened(arrival)}
							<li>
								<a
									href={addressOf(arrival.job_id)}
									data-sveltekit-keepfocus
									aria-current={arrival.job_id === open ? 'page' : undefined}
									tabindex={arrival.job_id === tabStop ? 0 : -1}
									class={[
										'block min-h-12 border-b border-l-3 border-rule px-gutter py-3',
										arrival.job_id === open ? 'border-l-accent bg-card' : 'border-l-transparent',
									]}
								>
									<span class="block text-body leading-tight text-ink">
										{whenArrived(arrival.created_at)}
									</span>
									<span class={['block text-read', happened.loss ? 'text-support' : 'text-ink-2']}>
										{happened.said}
									</span>
								</a>
							</li>
						{/each}
					</ul>
				{/each}
			{/if}
		</div>
	</div>
{:else}
	<Screen title={m.imports_title()}>
		<p class="mt-2 text-read text-ink-2">{m.imports_blurb()}</p>

		<Section heading={m.imports_sources()}>
			{#if said || !imports?.length}
				{@render nothingToList()}
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
								href={sourceAddress(source.source_kind)}
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
{/if}
