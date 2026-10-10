<!--
	The Thread (issue #53): a recipe's whole life on one screen, forking at the
	Branch Point, Attempts hanging off it. Reading, never editing (GLOSSARY.md,
	"Thread") — a past Version opens from here as a page of its own, to be
	read in full and cooked from (#211), never changed. The one thing written here is a Version's name, by the cook
	who saved it (#115, ADR 0015): it is outside the fingerprint, so naming
	moves no id and mints no Version.

	Takes `branchId` as an ordinary prop — rather than reading `$app/state`
	itself — so the screen-seam test can drive it directly, the same way every
	other screen here is tested against the Catalogue-derived stand-in.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
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
	import type { GetThreadOutput } from '$lib/api/catalogue';

	type Attempt = GetThreadOutput['attempts'][number];

	interface Props {
		branchId: string;
		/** Going to another page. A test hands in its own. */
		navigate?: (to: string) => Promise<void>;
	}

	let { branchId, navigate = goto }: Props = $props();

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

	/**
	 * Each cooking under the Version that was eaten (#209), which the Core
	 * names: the As Cooked once it was kept as a Version, otherwise the
	 * Version the cooking started from. A rating is a verdict on what was on
	 * the plate, so filing it by `version_id` would hang it on a recipe nobody
	 * ate that day and leave the Version that earned it looking untried.
	 *
	 * A Thread this phone kept from before the Core said so carries no such
	 * id. Its cookings stay where they were shown until the read is fresh,
	 * rather than dropping off the History for one visit.
	 */
	const attemptsByVersion = $derived.by(() => {
		const map = new Map<string, Attempt[]>();
		for (const attempt of thread?.attempts ?? []) {
			const fromThisPhone: Partial<Attempt> = attempt;
			const eaten = fromThisPhone.eaten_version_id ?? attempt.version_id;
			const list = map.get(eaten) ?? [];
			list.push(attempt);
			map.set(eaten, list);
		}
		return map;
	});

	/**
	 * A past Version opens as a recipe page of its own, with what has changed
	 * since marked on it (#211). The newest Version of a Branch is the recipe,
	 * so it opens the recipe.
	 */
	function openVersion(version: ThreadVersion) {
		const newest = thread?.versions.filter((each) => each.branch_id === version.branch_id).at(-1);
		const recipe = `/recipes/${version.branch_id}`;
		void navigate(
			newest?.version_id === version.version_id
				? recipe
				: `${recipe}/versions/${version.version_id}`,
		);
	}

	let openAttempt = $state<Attempt | undefined>(undefined);

	/** What an Attempt's sheet is called, and what it says first. */
	const whenCooked = (attempt: Attempt) =>
		m.thread_attempt_cooked({ when: aDate(attempt.created_at) });

	function closeSheets() {
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
			onOpenVersion={openVersion}
			onOpenAttempt={(attempt) => (openAttempt = attempt)}
			onRenamed={reread}
		/>
	{/if}
</Screen>

{#snippet sheet(called: string, children: Snippet)}
	<!-- Not one of the twelve #196 counted, since it never called itself a
	     dialog, but a bottom sheet all the same, and ADR 0044 makes every one
	     of those a window. On the phone it keeps its darker dimming, and on
	     both layouts its own card. -->
	<SheetFrame
		label={called}
		dim="ink"
		safe={false}
		class="rounded-sm bg-card p-4"
		onclose={closeSheets}
	>
		<button type="button" class="float-right text-label text-ink-2 uppercase" onclick={closeSheets}>
			{m.thread_close()}
		</button>
		{@render children()}
	</SheetFrame>
{/snippet}

{#if openAttempt}
	{@render sheet(whenCooked(openAttempt), attemptDetail)}
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
