<!--
	Promotion offered on the recipe (#58, ADR 0005): a band above `Cook this`
	saying that a cooking of this dish departed from the words below, and
	offering to keep them.

	IT IS HERE RATHER THAN IN THE DIARY, and that was Aurélien's choice on
	4 September 2026 against a treatment that put it beside the cooking's
	rating. The argument that won: what you are being asked is a question about
	the recipe — should these words become part of it — and this is where the
	recipe's Versions already live. You are standing in the thing you would be
	changing, with the Thread one control away.

	PRESSING IT DOES NOT MINT A VERSION. It opens what you would be keeping,
	and the save is a second, deliberate tap. Promotion is the point at which an
	Attempt's freedoms end and the recipe's append-only rules begin, so the one
	irreversible act on this screen is not something a thumb can spend by
	brushing past it. When the recipe's own editing screen lands (#83) this
	expansion becomes it, pre-filled, and the shape does not change.

	WHAT IT SHOWS IS THE CORE'S READING, never its own. `as_cooked.against` is
	the As Cooked laid over the Version cooked by the same Pairing two Branches
	are laid over each other with (ADR 0019). Nothing here matches a line to a
	line: index arithmetic in a screen would be wrong exactly where a cook
	dropped a line or added one, which they may.

	ALREADY PROMOTED NEEDS NO FLAG. An As Cooked's `version_id` is a fingerprint
	of its own content, so once promoted it simply IS one of the Branch's
	Versions — and the band is not drawn. Nothing stored can disagree with that.

	The fingerprint has one consequence worth knowing, and it is deliberate: a
	cook who happened to cook a recipe into EXACTLY a previous state of its own
	text holds an As Cooked whose id is already in the chain, so the band does
	not offer. The alternative — comparing against the head alone — would ask
	again after every later edit of the recipe, which is a nag rather than a
	question. Those words are in the recipe's history either way, and the
	cooking keeps its own record of them.

	ON A RECIPE NONE OF YOUR KITCHENS HOLDS, KEEPING IS A COPY, and the band
	says so in the writing screen's own words before the tap, by the one rule
	`whereASaveLands` keeps. A cook in several Kitchens is asked which keeps
	it, with the same list (#111). Kept, the Copy is where the cook goes next:
	the recipe on screen did not change, and a "Kept" over it would say it had.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { getLocale } from '$lib/paraglide/runtime';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { goto } from '$app/navigation';
	import type { GetRecipeOutput, GetThreadOutput, ListKitchensOutput } from '$lib/api/catalogue';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';
	import KitchenChoice from '$lib/KitchenChoice.svelte';
	import { chosenKitchenInput, kitchenName, whereASaveLands } from '$lib/where-a-save-lands';
	import { Online } from '$lib/offline/device.svelte';

	type Attempt = GetThreadOutput['attempts'][number];

	interface Props {
		branchId: string;
		/** The Kitchen holding this Branch, which says whether keeping is a Copy. */
		kitchenId: string;
		/** Every Attempt on this Lineage, whatever Branch it cooked. */
		attempts: Attempt[];
		/** This Branch's chain, which says what has already been kept. */
		versions: GetRecipeOutput['versions'];
		/** Re-read the recipe once a Version has joined it. */
		promoted: () => void;
	}

	let { branchId, kitchenId, attempts, versions, promoted }: Props = $props();

	const kamosu = useKamosu();

	let opened = $state(false);
	let saving = $state(false);
	let saved = $state(false);
	let failed = $state(false);
	/** Answered, and answered away — laid over the list without a re-read. */
	let declinedHere = $state(new Set<string>());

	/**
	 * Whether the server is reachable. Promotion is the one thing on this screen
	 * that needs it — everything on an Attempt's side of the line works offline
	 * and this does not (ADR 0013) — so its button says what it is waiting for
	 * rather than failing when pressed (#76). Leaving it in the diary is an
	 * answer too, so it waits for the server with it, and is simply not offered.
	 */
	const online = new Online();

	/**
	 * The cooking being offered: the most recent one that departed from the
	 * recipe and whose words are not already in this Branch. There is at most
	 * one band, because being asked about four afternoons at once is a list to
	 * work through rather than a question to answer.
	 */
	const kept = $derived(new Set(versions.map((version) => version.version_id)));
	const pending = $derived(
		attempts
			.filter(
				(attempt) =>
					attempt.as_cooked &&
					!attempt.as_cooked.promotion_declined &&
					!declinedHere.has(attempt.id) &&
					!kept.has(attempt.as_cooked.version_id),
			)
			.sort((a, b) => b.created_at.localeCompare(a.created_at))
			.at(0),
	);

	/**
	 * The cook's Kitchens, which say whether keeping is a Version or a Copy.
	 * Keeping waits for them, as the writing screen's save does: the band is
	 * about to state the outcome, and must not guess it.
	 */
	let kitchens = $state<ListKitchensOutput['kitchens'] | undefined>(undefined);
	/**
	 * They could not be read. Keeping then goes ahead as it did before #111,
	 * naming no Kitchen and leaving the Core to decide, rather than locking a
	 * button that never needed them for a cook keeping onto their own recipe.
	 */
	let kitchensFailed = $state(false);
	/**
	 * Asked only where there is a band to draw, which is nearly never: every
	 * other opening of a recipe page would pay for a question nobody needed.
	 */
	const offering = $derived(pending !== undefined);
	$effect(() => {
		if (!offering || kitchens) return;
		let current = true;
		kamosu
			.listKitchens({})
			.then((all) => {
				if (current) kitchens = all.kitchens;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				if (current) kitchensFailed = true;
			});
		return () => {
			current = false;
		};
	});
	let chosenKitchen = $state<string | undefined>(undefined);
	const lands = $derived(
		kitchens ? whereASaveLands(kitchens, kitchenId, chosenKitchen) : undefined,
	);
	const forking = $derived(lands?.forking ?? false);
	const askingWhere = $derived(lands?.asking ?? false);
	const savingInto = $derived(lands?.into);
	/** Whether keeping can be pressed: the outcome known, or unknowable. */
	const known = $derived(lands !== undefined || kitchensFailed);

	/**
	 * Whether the recipe has moved since this cooking. Promotion appends onto
	 * wherever the Branch stands now and never merges (ADR 0004), so a cook
	 * about to keep Tuesday's words over Friday's recipe is told so before they
	 * press — the lines struck below are what the recipe said THEN.
	 */
	const movedOn = $derived(
		pending !== undefined && versions.at(-1)?.version_id !== pending.version_id,
	);

	const when = $derived(
		pending
			? new Date(pending.created_at).toLocaleDateString(getLocale(), {
					day: 'numeric',
					month: 'long',
				})
			: '',
	);

	/**
	 * The lines this cooking did not leave alone, in the recipe's own order —
	 * rewritten, dropped, or written from nothing. A row the cook left alone is
	 * not shown: what is being confirmed is what would change.
	 */
	type Marked = { key: string; state: string; now: string | null; was: string | null };
	const marked = $derived.by(() => {
		const against = pending?.as_cooked?.against;
		if (!against) return [];
		const of = (rows: typeof against.ingredients, list: string): Marked[] =>
			rows
				.filter((row) => row.state !== 'same')
				.map((row, at) => ({
					key: `${list}-${at}`,
					state: row.state,
					now: row.theirs?.text ?? null,
					was: row.mine?.text ?? null,
				}));
		return [...of(against.ingredients, 'i'), ...of(against.steps, 's')];
	});

	/**
	 * The diary is where it stays. This answers the offer and touches the As
	 * Cooked not at all — the cooking keeps every word of what was cooked, and
	 * changing your mind later is an ordinary edit of your own Attempt.
	 */
	async function decline() {
		if (!pending) return;
		const id = pending.id;
		// Off the screen before the round trip: the cook has answered, and a
		// question that lingers while a server thinks reads as one not heard.
		declinedHere = new Set([...declinedHere, id]);
		try {
			await kamosu.declinePromotion({ attempt_id: id, declined: true });
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			// It will be asked again next time, which is the honest failure:
			// nothing was lost and nothing was decided.
			declinedHere = new Set([...declinedHere].filter((held) => held !== id));
			failed = true;
		}
	}

	async function promote() {
		if (!pending || !known || (askingWhere && !savingInto)) return;
		saving = true;
		failed = false;
		try {
			const landed = await kamosu.promoteAsCooked({
				attempt_id: pending.id,
				branch_id: branchId,
				...(lands ? chosenKitchenInput(lands) : {}),
			});
			if (landed.copied) {
				await goto(`/recipes/${landed.branch_id}`);
				return;
			}
			saved = true;
			opened = false;
			promoted();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = true;
		} finally {
			saving = false;
		}
	}
</script>

{#if saved}
	<p class="mx-gutter mt-6 text-read text-accent" role="status">{m.recipe_promoted()}</p>
{:else if pending}
	<div class="mx-gutter mt-6 rounded-sm border border-accent p-4">
		<p class="text-label text-accent uppercase">{m.recipe_cooked_differently()}</p>
		<p class="mt-1 font-display text-line">{m.recipe_cooked_differently_on({ when })}</p>

		{#if opened}
			{#if movedOn}
				<p class="mt-3 text-read text-support">{m.recipe_moved_on_since()}</p>
			{/if}
			<h2 class="mt-4 text-label text-ink-2 uppercase">{m.recipe_as_cooked_lines()}</h2>
			<ul>
				{#each marked as line (line.key)}
					<li class="border-b border-rule py-2">
						{#if line.state === 'only-mine'}
							<!-- A line the cook left out. It reads as the recipe has it,
							     struck, because that is what would go. -->
							<span class="block text-label text-ink-2 uppercase">
								{m.recipe_as_cooked_dropped()}
							</span>
							<span class="block font-display text-line text-ink-2 line-through">{line.was}</span>
						{:else}
							{#if line.state === 'only-theirs'}
								<span class="block text-label text-ink-2 uppercase">
									{m.recipe_as_cooked_added()}
								</span>
							{/if}
							<span class="block font-display text-line">{line.now}</span>
							{#if line.was}
								<span class="block text-read text-ink-2 line-through">{line.was}</span>
							{/if}
						{/if}
					</li>
				{/each}
			</ul>

			{#if forking && pending.as_cooked}
				<p class="mt-4 text-label text-support uppercase">{m.write_will_fork()}</p>
				<p class="mt-1 text-read">
					{askingWhere
						? m.write_said_fork_asked({ title: pending.as_cooked.content.title })
						: m.write_said_fork({
								title: pending.as_cooked.content.title,
								kitchen: savingInto ? kitchenName(savingInto) : '',
							})}
				</p>
				{#if askingWhere && kitchens}
					<div class="mt-3">
						<KitchenChoice {kitchens} bind:chosen={chosenKitchen} />
					</div>
				{/if}
			{/if}
			<NeedsServer
				label={!forking
					? m.recipe_save_as_version()
					: askingWhere && savingInto
						? m.write_do_fork_in({ kitchen: kitchenName(savingInto) })
						: m.write_do_fork()}
				waiting={m.offline_waits_keep()}
				disabled={saving || !known || (askingWhere && !savingInto)}
				onclick={promote}
				shapeClass="mt-3 block w-full p-4 text-center font-display text-body"
				lookClass="{forking ? 'bg-support' : 'bg-accent'} text-on-accent disabled:opacity-60"
			/>
			<button
				type="button"
				onclick={() => (opened = false)}
				class="mt-2 block w-full border border-rule p-4 text-center font-display text-body text-ink-2"
			>
				{m.recipe_not_now()}
			</button>
		{:else}
			<NeedsServer
				label={forking ? m.recipe_keep_as_copy() : m.recipe_keep_as_version()}
				waiting={m.offline_waits_keep()}
				onclick={() => (opened = true)}
				shapeClass="mt-3 block w-full p-4 text-center font-display text-body"
				lookClass="bg-accent text-on-accent"
			/>
			{#if online.current}
				<button
					type="button"
					onclick={decline}
					class="mt-2 block w-full border border-rule p-4 text-center font-display text-body text-ink-2"
				>
					{m.recipe_leave_in_diary()}
				</button>
			{/if}
		{/if}

		{#if failed}
			<p class="mt-2 text-read text-support" role="alert">{m.recipe_promote_failed()}</p>
		{/if}
	</div>
{/if}
