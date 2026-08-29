<!--
	The Thread (issue #53): a recipe's whole life on one screen, forking at the
	Branch Point, Attempts hanging off it. Reading, never editing (CONTEXT.md,
	"Thread") — a past Version opens here to be read in full and cooked from,
	never changed.

	Takes `branchId` as an ordinary prop — rather than reading `$app/state`
	itself — so the screen-seam test can drive it directly, the same way every
	other screen here is tested against the Catalogue-derived stand-in.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { ratingLabel } from '$lib/rating';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';
	import ThreadGroup from './ThreadGroup.svelte';
	import { buildGroup, chainsByBranch, type ThreadVersion } from './tree';
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

	const branchesById = $derived(new Map(thread?.branches.map((branch) => [branch.branch_id, branch]) ?? []));

	const attemptsByVersion = $derived.by(() => {
		const map = new Map<string, Attempt[]>();
		for (const attempt of thread?.attempts ?? []) {
			const list = map.get(attempt.version_id) ?? [];
			list.push(attempt);
			map.set(attempt.version_id, list);
		}
		return map;
	});

	const rootGroup = $derived.by(() => {
		if (!thread) return undefined;
		const chains = chainsByBranch(thread.versions);
		return buildGroup(thread.branches.map((branch) => branch.branch_id), chains, 0);
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
			await kamosu.startAttempt({ branch_id: openVersion.branch_id, version_id: openVersion.version_id });
			cooking = 'started';
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			cooking = 'failed';
		}
	}

	let openAttempt = $state<Attempt | undefined>(undefined);

	function closeSheets() {
		openVersion = undefined;
		openContent = undefined;
		openAttempt = undefined;
	}
</script>

<Screen title={m.thread_title()} blurb={m.thread_blurb()}>
	{#if failed}
		<p class="text-body text-accent" role="alert">{m.thread_failed()}</p>
	{:else if !thread || !rootGroup}
		<p class="text-body text-ink-2">{m.loading()}</p>
	{:else}
		<ThreadGroup
			group={rootGroup}
			branches={branchesById}
			{attemptsByVersion}
			onOpenVersion={openVersionDetail}
			onOpenAttempt={(attempt) => (openAttempt = attempt)}
		/>
	{/if}
</Screen>

{#snippet sheet(scrollable: boolean, children: Snippet)}
	<div class="fixed inset-0 z-30 flex items-end bg-ink/40">
		<div
			class={`mx-auto w-full max-w-2xl rounded-sm bg-card p-4 ${scrollable ? 'max-h-[70vh] overflow-y-auto' : ''}`}
		>
			<button type="button" class="float-right text-label uppercase text-ink-2" onclick={closeSheets}>
				{m.thread_close()}
			</button>
			{@render children()}
		</div>
	</div>
{/snippet}

{#if openVersion}
	{@render sheet(true, versionDetail)}
{/if}
{#snippet versionDetail()}
	<p class="font-display text-title font-semibold text-ink">
		{openContent?.title ?? m.loading()}
	</p>
	{#if openContent}
		<h2 class="mt-4 text-label uppercase text-accent">{m.thread_ingredients()}</h2>
		<ul class="mt-2 grid gap-1">
			{#each openContent.ingredients as line, index (index)}
				<li class="text-line text-ink">{line.text}</li>
			{/each}
		</ul>
		<h2 class="mt-4 text-label uppercase text-accent">{m.thread_method()}</h2>
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
	{@render sheet(false, attemptDetail)}
{/if}
{#snippet attemptDetail()}
	<p class="font-display text-body font-semibold text-ink">
		{m.thread_attempt_cooked({ when: new Date(openAttempt!.created_at).toLocaleDateString() })}
	</p>
	{#if openAttempt!.rating}
		<p class="text-body text-ink">{ratingLabel(openAttempt!.rating)}</p>
	{/if}
	<p class="mt-1 text-body text-ink-2">{openAttempt!.note ?? m.thread_attempt_no_note()}</p>
{/snippet}
