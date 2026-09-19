<!--
	The Import Report for a Crouton library (#69): option A, "one page, three
	parts", as Aurélien chose it on 2026-09-19 (the decision is a comment on #69).

	It owes three different kinds of thing at once, and each is drawn as what it
	is rather than flattened into one list: what is **waiting for a tap** (work,
	on a card with the one indigo button), what **could not be read** (a loss,
	with the red edge and the file named), and what **arrived** (a fact — a
	count, then the list behind a disclosure you open if you want to).

	The Report is the Job's own result, read through `get_job` (ADR 0025,
	ADR 0032), so it is still here after the screen closes and an agent asking
	how the import went reads the same thing. While the Job runs, the page is
	already here: the bar fills and the count climbs. The pairs arrive only at
	the end, because a pair needs both of its halves in.

	Relating is ordinary `set_related_recipe`, one call per pair whose answer
	changed — a pair unticked after "Change" is unlinked again, so the card
	never says less was linked than was. Keeping a pair apart is not an
	Operation — it is simply the absence of a link — so *that the card was
	answered* is remembered in this browser, and on another device the pairs
	are offered again, harmlessly.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { waitForJob } from '$lib/api/job';
	import type { GetJobOutput, ImportCroutonOutput } from '$lib/api/catalogue';
	import Screen from '$lib/shell/Screen.svelte';
	import Section from '$lib/shell/Section.svelte';

	interface Props {
		jobId: string;
		/** How long to wait between reads of a running Job. Tests shorten it. */
		every?: number;
	}

	let { jobId, every = 700 }: Props = $props();

	const kamosu = useKamosu();

	type Report = ImportCroutonOutput;
	type Pair = Report['related_candidates'][number];

	let job = $state<GetJobOutput | undefined>(undefined);
	let unreachable = $state<string | undefined>(undefined);

	// Read the Job until it ends — its row is the truth about it (ADR 0032).
	// Leaving the page stops the reading, never the import.
	$effect(() => {
		const id = jobId;
		let current = true;
		waitForJob(kamosu, id, {
			every,
			giveUpAfter: 6 * 60 * 60 * 1000,
			whileWaiting: (read) => {
				if (current) job = read;
			},
			stopped: () => !current,
		})
			.then((read) => {
				if (current) job = read;
			})
			.catch((error: unknown) => {
				if (!(error instanceof Error)) throw error;
				if (current) unreachable = error.message;
			});
		return () => {
			current = false;
		};
	});

	const report = $derived(
		job?.status === 'completed' ? (job.result as Report | undefined) : undefined,
	);
	const running = $derived(job !== undefined && ['queued', 'running'].includes(job.status));
	const done = $derived(job?.progress.done ?? 0);
	const total = $derived(job?.progress.total ?? 0);

	const arrived = $derived(report?.arrived ?? []);
	const pairs = $derived(report?.related_candidates ?? []);
	const photos = $derived(arrived.filter((row) => row.main_photo).length);
	const bare = $derived(arrived.filter((row) => row.bare));
	const siteIcons = $derived(
		(report?.left_out ?? [])
			.filter((row) => row.what === 'site_icon')
			.reduce((sum, row) => sum + row.count, 0),
	);
	/** Each distinct site icon once: nine Instagram recipes carry one icon. */
	const icons = $derived([
		...new Set(
			(report?.left_out ?? [])
				.map((row) => (row.what === 'site_icon' ? row.icon : undefined))
				.filter((icon): icon is string => Boolean(icon)),
		),
	]);
	const extraPhotos = $derived(
		(report?.left_out ?? []).filter((row) => row.what === 'extra_photos'),
	);
	const brokenPhotos = $derived(
		(report?.left_out ?? []).filter((row) => row.what === 'unreadable_photos'),
	);

	// --- The pairs -------------------------------------------------------------

	const pairKey = (pair: Pair) => pair.recipes.map((side) => side.branch_id).join('+');
	const memory = $derived(`kamosu.import-report.${jobId}`);

	/** The pairs this page has linked, as far as it knows, and what is ticked now. */
	let linked = $state<string[]>([]);
	let ticked = $state<string[]>([]);
	let settled = $state(false);
	let relating = $state(false);
	let relateError = $state<string | undefined>(undefined);

	$effect(() => {
		try {
			const kept = JSON.parse(localStorage.getItem(memory) ?? 'null') as {
				related: string[];
			} | null;
			if (kept) {
				linked = kept.related;
				ticked = kept.related;
				settled = true;
			}
		} catch {
			// A private window or blocked storage: the pairs are simply offered again.
		}
	});

	function remember() {
		try {
			localStorage.setItem(memory, JSON.stringify({ related: linked }));
		} catch {
			// As above: forgetting that the card was answered costs a second look.
		}
	}

	function tick(pair: Pair, on: boolean) {
		const key = pairKey(pair);
		ticked = on ? [...ticked, key] : ticked.filter((k) => k !== key);
	}

	/** Make the links match the ticks: link what was ticked, unlink what no longer is. */
	async function answer(wanted: string[]) {
		relating = true;
		relateError = undefined;
		try {
			for (const pair of pairs) {
				const key = pairKey(pair);
				const want = wanted.includes(key);
				if (want === linked.includes(key)) continue;
				await kamosu.setRelatedRecipe({
					branch_id: pair.recipes[0].branch_id,
					related_branch_id: pair.recipes[1].branch_id,
					related: want,
				});
				linked = want ? [...linked, key] : linked.filter((k) => k !== key);
			}
			ticked = wanted;
			settled = true;
			remember();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			relateError = error.message;
		} finally {
			relating = false;
		}
	}

	const linkedPairs = $derived(pairs.filter((pair) => linked.includes(pairKey(pair))));

	/** A pair is named by its shorter title: "Pandebonos", not "(Version 2)". */
	const pairName = (pair: Pair) =>
		pair.recipes
			.map((side) => side.title)
			.reduce((shorter, title) => (title.length < shorter.length ? title : shorter));

	const why = (pair: Pair) =>
		pair.shared
			.map((shared) => (shared === 'name' ? m.report_pair_same_name() : m.report_pair_same_page()))
			.join(' · ');

	const thumb = (photo: string | null | undefined) =>
		photo ? `/api/photographs/${photo}/card` : undefined;
</script>

<Screen title={m.report_title()}>
	<a href="/settings" class="text-read text-ink-2">‹ {m.report_back()}</a>

	{#if unreachable}
		<p class="mt-4 border-l-3 border-support bg-card px-3 py-2 text-body" role="alert">
			{unreachable}
		</p>
	{:else if job === undefined || job.status === 'queued'}
		<p class="mt-4 text-body text-ink-2">{m.report_waiting()}</p>
	{:else if running}
		<p class="mt-4 text-body text-ink-2">{m.report_running({ done, total })}</p>
		<div
			class="mt-3 h-1 overflow-hidden rounded-sm bg-ground-2"
			role="progressbar"
			aria-valuemin={0}
			aria-valuemax={total}
			aria-valuenow={done}
		>
			<div class="h-full bg-accent" style="width: {total ? (done / total) * 100 : 0}%"></div>
		</div>

		<Section heading={m.report_needs_you()}>
			<p class="rounded-sm border border-rule bg-card px-3 py-3 text-read text-ink-2">
				{m.report_needs_you_waiting()}
			</p>
		</Section>
	{:else if job.status !== 'completed' || !report}
		<p class="mt-4 border-l-3 border-support bg-card px-3 py-2 text-body" role="alert">
			{m.report_stopped({ reason: job.error ?? job.status })}
		</p>
	{:else}
		<p class="mt-4 text-body text-ink-2">
			{m.report_lede({ count: arrived.length })}
			{#if report.unreadable.length}
				{m.report_lede_unreadable({ count: report.unreadable.length })}
			{/if}
			{#if pairs.length}
				{m.report_lede_pairs({ count: pairs.length })}
			{/if}
		</p>

		<Section heading={m.report_needs_you()}>
			{#if pairs.length === 0 && report.offered.length === 0}
				<p class="text-read text-ink-2">{m.report_needs_you_none()}</p>
			{/if}

			{#if pairs.length}
				<div class="rounded-sm border border-rule bg-card px-3 pb-3">
					{#if settled}
						<p class="pt-3 text-read">
							{#if linkedPairs.length === pairs.length}
								{m.report_settled_all({ titles: linkedPairs.map(pairName).join(', ') })}
							{:else if linkedPairs.length}
								{m.report_settled({
									titles: linkedPairs.map(pairName).join(', '),
									count: pairs.length - linkedPairs.length,
								})}
							{:else}
								{m.report_settled_none({ count: pairs.length })}
							{/if}
							<button
								type="button"
								class="ml-1 text-accent underline"
								onclick={() => (settled = false)}
							>
								{m.report_settled_change()}
							</button>
						</p>
					{:else}
						<p class="pt-3 pb-1 text-read text-ink-2">{m.report_pairs_blurb()}</p>
						<ul>
							{#each pairs as pair (pairKey(pair))}
								<li class="border-b border-rule last:border-b-0">
									<label class="flex cursor-pointer items-start gap-3 py-2">
										<input
											type="checkbox"
											class="mt-1 size-6 shrink-0 accent-accent"
											checked={ticked.includes(pairKey(pair))}
											onchange={(event) => tick(pair, event.currentTarget.checked)}
										/>
										<span class="flex-1">
											<span class="block text-body leading-tight">
												{pair.recipes[0].title}
												<span class="text-ink-2">&amp;</span>
												{pair.recipes[1].title}
											</span>
											<span class="block text-read text-ink-2">
												{why(pair)} · {m.report_pair_ingredients({
													first: pair.recipes[0].ingredients,
													second: pair.recipes[1].ingredients,
												})}
											</span>
										</span>
										<span class="flex shrink-0 gap-1">
											{#each pair.recipes as side (side.branch_id)}
												{#if thumb(side.main_photo)}
													<img
														src={thumb(side.main_photo)}
														alt=""
														class="size-8 rounded-sm object-cover"
													/>
												{:else}
													<span class="size-8 rounded-sm bg-ground-2"></span>
												{/if}
											{/each}
										</span>
									</label>
								</li>
							{/each}
						</ul>
						{#if relateError}
							<p class="text-read text-support" role="alert">{relateError}</p>
						{/if}
						<div class="mt-3 flex items-center gap-2">
							<button
								type="button"
								class="min-h-12 rounded-sm bg-accent px-4 font-semibold text-on-accent disabled:opacity-40"
								disabled={ticked.length === 0 || relating}
								onclick={() => answer(ticked)}
							>
								{ticked.length ? m.report_relate({ count: ticked.length }) : m.report_relate_none()}
							</button>
							<button
								type="button"
								class="min-h-12 px-2 text-ink-2"
								disabled={relating}
								onclick={() => answer([])}
							>
								{m.report_relate_dismiss()}
							</button>
						</div>
					{/if}
				</div>
			{/if}

			{#each report.offered as changed (changed.branch_id)}
				<p class="mt-2 text-read">
					<a href="/recipes/{changed.branch_id}" class="text-accent underline"
						>{m.report_changed({ title: changed.title })}</a
					>
				</p>
			{/each}
		</Section>

		<Section heading={m.report_unreadable()}>
			{#if report.unreadable.length === 0}
				<p class="text-read text-ink-2">{m.report_unreadable_none()}</p>
			{:else}
				<ul class="grid gap-2">
					{#each report.unreadable as loss, index (index)}
						<li class="border-l-3 border-support bg-card px-3 py-2">
							<p class="text-body font-semibold">
								{loss.name ?? loss.foreign_id ?? m.report_unreadable_unnamed()}
							</p>
							<p class="text-read text-ink-2">{loss.reason}</p>
						</li>
					{/each}
				</ul>
			{/if}
		</Section>

		<Section heading={m.report_arrived()}>
			<p class="text-body">{m.report_arrived_count({ recipes: arrived.length, photos })}</p>
			{#if bare.length}
				<p class="mt-2 text-read">
					{m.report_bare({ count: bare.length, titles: bare.map((row) => row.title).join(', ') })}
				</p>
			{/if}
			{#if siteIcons}
				<p class="mt-2 text-read text-ink-2">{m.report_site_icons({ count: siteIcons })}</p>
				{#if icons.length}
					<p class="mt-1 flex flex-wrap gap-1" aria-hidden="true">
						{#each icons as icon (icon)}
							<img src={icon} alt="" class="size-4 rounded-sm opacity-70 grayscale" />
						{/each}
					</p>
				{/if}
			{/if}
			{#each extraPhotos as extra (extra.branch_id)}
				<p class="mt-2 text-read text-ink-2">
					{extra.count === 1
						? m.report_extra_photo_one({ title: extra.title })
						: m.report_extra_photo_many({ title: extra.title, count: extra.count })}
				</p>
			{/each}
			{#each brokenPhotos as broken (broken.branch_id)}
				<p class="mt-2 text-read text-ink-2">
					{m.report_unreadable_photos({ title: broken.title })}
				</p>
			{/each}

			{#if arrived.length}
				<details class="mt-3">
					<summary class="cursor-pointer py-2 font-medium text-accent">
						{m.report_show_all({ count: arrived.length })}
					</summary>
					<ul>
						{#each arrived as row (row.branch_id)}
							<li class="flex items-center gap-3 border-b border-rule py-2 last:border-b-0">
								{#if thumb(row.main_photo)}
									<img
										src={thumb(row.main_photo)}
										alt=""
										loading="lazy"
										class="size-12 shrink-0 rounded-sm object-cover"
									/>
								{:else}
									<span
										class="grid size-12 shrink-0 place-items-center rounded-sm bg-ground-2 text-label text-ink-2"
										>{m.report_no_photo()}</span
									>
								{/if}
								<span>
									<a href="/recipes/{row.branch_id}" class="block text-body leading-tight"
										>{row.title}</a
									>
									{#if row.bare || row.status === 'unchanged'}
										<span class="block text-read text-ink-2">
											{[
												row.bare ? m.report_bare_recipe() : '',
												row.status === 'unchanged' ? m.report_unchanged() : '',
											]
												.filter(Boolean)
												.join(' · ')}
										</span>
									{/if}
								</span>
							</li>
						{/each}
					</ul>
				</details>
			{/if}
		</Section>
	{/if}
</Screen>
