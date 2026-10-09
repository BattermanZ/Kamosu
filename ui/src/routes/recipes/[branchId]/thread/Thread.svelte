<!--
	The Thread (issue #53): a recipe's whole life on one screen, forking at the
	Branch Point, Attempts hanging off it. Reading, never editing (GLOSSARY.md,
	"Thread") — a past Version opens here to be read in full and cooked from,
	never changed. The one thing written here is a Version's name, by the cook
	who saved it (#115, ADR 0015): it is outside the fingerprint, so naming
	moves no id and mints no Version.

	Takes `branchId` as an ordinary prop — rather than reading `$app/state`
	itself — so the screen-seam test can drive it directly, the same way every
	other screen here is tested against the Catalogue-derived stand-in.
-->
<script lang="ts">
	import { aDate } from '$lib/dates';
	import type { Snippet } from 'svelte';
	import { m } from '$lib/paraglide/messages';
	import SheetFrame from '$lib/SheetFrame.svelte';
	import { useKamosu } from '$lib/kamosu';
	import { ratingLabel } from '$lib/rating';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';
	import ThreadGraph from './ThreadGraph.svelte';
	import { languageSaidAt, type ThreadVersion } from './tree';
	import type { GetThreadOutput, GetRecipeOutput } from '$lib/api/catalogue';

	type Attempt = GetThreadOutput['attempts'][number];
	type VersionContent = GetRecipeOutput['versions'][number]['content'];

	interface Props {
		branchId: string;
	}

	let { branchId }: Props = $props();

	const kamosu = useKamosu();

	let thread = $state<GetThreadOutput | undefined>(undefined);
	let failed = $state(false);
	/**
	 * The reader's own Hand, which is their Person id (#115): the Versions it
	 * saved are the ones the screen offers to rename. Until it is known, or
	 * if it cannot be, nothing is offered — the Thread still reads.
	 */
	let me = $state<string | undefined>(undefined);

	$effect(() => {
		let current = true;
		thread = undefined;
		failed = false;
		kamosu
			.getThread({ branch_id: branchId })
			.then((answer) => {
				if (current) thread = answer;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				if (current) failed = true;
			});
		return () => {
			current = false;
		};
	});

	$effect(() => {
		kamosu
			.getPerson()
			.then((person) => (me = person.person_id))
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
			});
	});

	/**
	 * Read the Thread again after a rename, in place: the screen stays drawn
	 * while it is asked, so the sentence saying what the rename did stays too.
	 */
	async function reread() {
		const asked = branchId;
		const answer = await kamosu.getThread({ branch_id: asked });
		if (asked === branchId) thread = answer;
	}

	/**
	 * Which entries said what Language this recipe is in (#106, ADR 0006).
	 * Worked out once, over the whole Thread, because the comparison runs down
	 * a Branch and a group only ever holds part of one.
	 */
	const languageSaid = $derived(languageSaidAt(thread?.versions ?? []));

	const attemptsByVersion = $derived.by(() => {
		const map = new Map<string, Attempt[]>();
		for (const attempt of thread?.attempts ?? []) {
			const list = map.get(attempt.version_id) ?? [];
			list.push(attempt);
			map.set(attempt.version_id, list);
		}
		return map;
	});

	// ---- reading a past Version in full, and cooking from it -----------------

	let openVersion = $state<ThreadVersion | undefined>(undefined);
	let openContent = $state<VersionContent | undefined>(undefined);
	let cooking = $state<'idle' | 'starting' | 'started' | 'failed'>('idle');

	async function openVersionDetail(version: ThreadVersion) {
		openVersion = version;
		openContent = undefined;
		cooking = 'idle';
		const recipe = await kamosu.getRecipe({ branch_id: version.branch_id });
		openContent = recipe.versions.find((v) => v.sequence === version.sequence)?.content;
	}

	async function cookThisVersion() {
		if (!openVersion) return;
		cooking = 'starting';
		try {
			await kamosu.startAttempt({
				branch_id: openVersion.branch_id,
				version_id: openVersion.version_id,
			});
			cooking = 'started';
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			cooking = 'failed';
		}
	}

	let openAttempt = $state<Attempt | undefined>(undefined);

	/** What an Attempt's sheet is called, and what it says first. */
	const whenCooked = (attempt: Attempt) =>
		m.thread_attempt_cooked({ when: aDate(attempt.created_at) });

	function closeSheets() {
		openVersion = undefined;
		openContent = undefined;
		openAttempt = undefined;
	}
</script>

<Screen title={m.thread_title()} blurb={m.thread_blurb()}>
	{#if failed}
		<p class="text-body text-accent" role="alert">{m.thread_failed()}</p>
	{:else if !thread}
		<p class="text-body text-ink-2">{m.loading()}</p>
	{:else}
		<ThreadGraph
			{thread}
			{me}
			{attemptsByVersion}
			{languageSaid}
			onOpenVersion={openVersionDetail}
			onOpenAttempt={(attempt) => (openAttempt = attempt)}
			onRenamed={reread}
		/>
	{/if}
</Screen>

{#snippet sheet(called: string, scrollable: boolean, children: Snippet)}
	<!-- Not one of the twelve #196 counted, since it never called itself a
	     dialog, but a bottom sheet all the same, and ADR 0044 makes every one
	     of those a window. On the phone it keeps its darker dimming, and on
	     both layouts its own card. -->
	<SheetFrame
		label={called}
		dim="ink"
		tall={scrollable ? 70 : undefined}
		safe={false}
		class="rounded-sm bg-card p-4 {scrollable ? 'overflow-y-auto' : ''}"
		onclose={closeSheets}
	>
		<button type="button" class="float-right text-label text-ink-2 uppercase" onclick={closeSheets}>
			{m.thread_close()}
		</button>
		{@render children()}
	</SheetFrame>
{/snippet}

{#if openVersion}
	{@render sheet(openContent?.title ?? m.loading(), true, versionDetail)}
{/if}
{#snippet versionDetail()}
	<p class="font-display text-title font-semibold text-ink">
		{openContent?.title ?? m.loading()}
	</p>
	{#if openContent}
		<h2 class="mt-4 text-label text-accent uppercase">{m.thread_ingredients()}</h2>
		<ul class="mt-2 grid gap-1">
			{#each openContent.ingredients as line, index (index)}
				<li class="text-line text-ink">{line.text}</li>
			{/each}
		</ul>
		<h2 class="mt-4 text-label text-accent uppercase">{m.thread_method()}</h2>
		<ol class="mt-2 grid gap-1">
			{#each openContent.steps as step, index (index)}
				<li class="text-body text-ink">{step.text}</li>
			{/each}
		</ol>
	{/if}
	<button
		type="button"
		class="mt-4 min-h-12 w-full rounded-sm bg-accent px-4 font-display text-body font-semibold text-on-accent disabled:opacity-60"
		disabled={cooking === 'starting'}
		onclick={cookThisVersion}
	>
		{m.thread_cook_this_version()}
	</button>
	{#if cooking === 'started'}
		<p class="mt-2 text-read text-ink-2">{m.thread_cooking_started()}</p>
	{:else if cooking === 'failed'}
		<p class="mt-2 text-read text-accent" role="alert">{m.thread_cooking_failed()}</p>
	{/if}
{/snippet}

{#if openAttempt}
	{@render sheet(whenCooked(openAttempt), false, attemptDetail)}
{/if}
{#snippet attemptDetail()}
	<p class="font-display text-body font-semibold text-ink">
		{whenCooked(openAttempt!)}
	</p>
	{#if openAttempt!.rating}
		<p class="text-body text-ink">{ratingLabel(openAttempt!.rating)}</p>
	{/if}
	<p class="mt-1 text-body text-ink-2">{openAttempt!.note ?? m.thread_attempt_no_note()}</p>
{/snippet}
