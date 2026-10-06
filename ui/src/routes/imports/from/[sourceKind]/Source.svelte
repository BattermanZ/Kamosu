<!--
	One source, and everything that ever arrived through it (#108).

	**The URL is `/imports/from/<kind>` and not `/imports/<kind>`** because
	`/imports/<jobId>` is the Report and has been since #68. A source screen
	under the same segment would be a source called `j_578dfd…`.

	**This is where forgetting lives, and the only place.** `forget_import`
	throws away a whole channel's ledger — on the dev instance, all 89 Crouton
	recipes at once, across three separate runs — and cannot throw away one
	arrival. A screen about exactly one source is where that scope reads
	correctly; a link in a list of everything would not say which "these" it
	meant.

	**The asking is `Confirm`, the one sheet every irreversible act in Kamosu
	uses** (#103, #120). Its shape happens to be exactly what this act needs:
	the count leads in the largest type on the sheet, and here the count is the
	reassuring fact rather than the frightening one — *89 recipes stay exactly
	as they are*. The cost follows behind the red rule, because it is invisible
	until the next time the same file is brought in and so has to be spelt out
	rather than felt. `cancelLabel` is this ticket's one addition to that
	component: forgetting has a named opposite, *Keep the memory*, where
	deleting an account has only Cancel.

	**Forgetting does not empty this screen.** The ledger goes; the Jobs never do
	(`jobs` rows are terminal, never deleted), so the arrivals and their Reports
	stay listed with `import_id` null. Hiding them would recreate the exact
	unreachability this ticket closes.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';
	import WayBackLine from '$lib/shell/WayBackLine.svelte';
	import Section from '$lib/shell/Section.svelte';
	import Empty from '$lib/shell/Empty.svelte';
	import Confirm from '$lib/Confirm.svelte';
	import {
		canForget,
		forgetAsk,
		forgetCost,
		sourceName,
		sourceSummary,
		whatHappened,
		whenArrived,
		type Import,
	} from '$lib/imports';

	interface Props {
		/** The source kind this screen is about, read off the URL by the route. */
		sourceKind: string;
	}

	let { sourceKind }: Props = $props();

	const kamosu = useKamosu();

	let imports = $state<Import[] | undefined>(undefined);
	/** The Core's own refusal, where one arrived. Its words, not a paraphrase. */
	let said = $state<string | undefined>(undefined);
	let asking = $state(false);
	let forgetting = $state(false);
	let forgot = $state(false);

	/** Reading the list again after forgetting, with the same guard the first read has. */
	let reading = 0;

	async function load() {
		const mine = ++reading;
		try {
			const answer = await kamosu.listImports();
			if (mine === reading) imports = answer.imports;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			if (mine === reading) said = error.message || m.imports_unreachable();
		}
	}

	$effect(() => {
		load();
		// A read still in flight when this screen closes must not write into it.
		return () => {
			reading += 1;
		};
	});

	const source = $derived(imports?.find((row) => row.source_kind === sourceKind));
	const arrivals = $derived(source?.arrivals ?? []);

	async function forget() {
		const ledger = source?.import_id;
		if (!source || !ledger || !canForget(source)) return;
		forgetting = true;
		said = undefined;
		try {
			await kamosu.forgetImport({ import_id: ledger });
			forgot = true;
			asking = false;
			await load();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			said = error.message;
		} finally {
			forgetting = false;
		}
	}
</script>

<Screen title={source ? sourceName(source.source_kind) : sourceName(sourceKind)}>
	<WayBackLine href="/imports" label={m.imports_from_back()} />

	{#if said && !asking}
		<p class="mt-4 border-l-3 border-support bg-card px-3 py-2 text-body" role="alert">{said}</p>
	{/if}

	{#if imports === undefined}
		<p class="mt-4 text-body text-ink-2">{m.loading()}</p>
	{:else if !source}
		<!-- A URL naming a source nothing ever came through. Not the same thing
		     as an instance that has imported nothing, and not worded as if it
		     were: this Kamosu may have a full library and simply not this. -->
		<div class="mt-4"><Empty>{m.imports_source_unknown()}</Empty></div>
	{:else}
		<p class="mt-4 text-body text-ink-2">{sourceSummary(source)}</p>

		{#if forgot}
			<p class="mt-3 border-l-3 border-support-2 bg-card px-3 py-2 text-read" role="status">
				{m.imports_forgotten()}
			</p>
		{/if}

		<Section heading={m.imports_arrived_heading()}>
			<ul>
				{#each arrivals as arrival (arrival.job_id)}
					{@const happened = whatHappened(arrival)}
					<li class="border-b border-rule last:border-b-0">
						<a href="/imports/{arrival.job_id}" class="flex min-h-12 items-center gap-3 py-2">
							<span class="min-w-0 flex-1">
								<span class="block text-body leading-tight text-ink">
									{whenArrived(arrival.created_at)}
								</span>
								<span class="block text-read {happened.loss ? 'text-support' : 'text-ink-2'}">
									{happened.said}
								</span>
							</span>
							<span aria-hidden="true" class="text-ink-2">›</span>
						</a>
					</li>
				{/each}
			</ul>
		</Section>

		<!-- Offered only while there is a ledger to throw away. Once it is gone
		     this section goes with it, and the arrivals above stay. A Bundle's
		     Import never has one (ADR 0020), so recipe files never show it. -->
		{#if canForget(source)}
			<Section heading={m.imports_forgetting_heading()}>
				<p class="text-read text-ink-2">{m.imports_forget_blurb()}</p>
				<button
					type="button"
					class="mt-3 min-h-12 text-label text-accent underline"
					onclick={() => (asking = true)}
				>
					{m.imports_forget_open()}
				</button>
			</Section>
		{/if}
	{/if}
</Screen>

{#if asking && source}
	<Confirm
		title={forgetAsk(source.source_kind)}
		count={source.remembered}
		counting={source.remembered === 1
			? m.imports_forget_counting_one()
			: m.imports_forget_counting_many()}
		consequence="{m.imports_forget_cost_lead()} {forgetCost(source.source_kind, source.remembered)}"
		act={m.imports_forget_go()}
		cancelLabel={m.imports_forget_keep()}
		busy={forgetting}
		failed={said}
		run={forget}
		cancel={() => (asking = false)}
	>
		<p class="text-body text-ink-2">{m.imports_forget_untouched()}</p>
		<p class="mt-3 text-body text-ink-2">{m.imports_forget_when()}</p>
	</Confirm>
{/if}
