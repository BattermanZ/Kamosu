<!--
	The recipe screen, and — when a second Branch of the same Lineage is on this
	instance — the Divergence.

	ADR 0014, which refuses the obvious thing: holding two Branches shows TWO
	WHOLE RECIPES WITH A SWITCH BETWEEN THEM, never a difference. There is no
	comparison screen here, no side-by-side and no object called "the
	difference". You stand inside one recipe or the other — whole, in order,
	cookable — and the threshold at the top moves you between them.

	A line the other Branch has and yours has not is a GHOST: struck through, in
	the position it holds over there, saying whose it is. A removal seen from
	their side and an addition seen from yours are the same object, so one
	mechanism serves both and the page reads the same whichever recipe you are in.

	Which line is which is read against the Branch Point by the `divergence`
	Operation (ADR 0019). Nothing here pairs anything, and no line carries an id.

	THE PAGE'S OWN LAYOUT was chosen by Aurélien on 29 August 2026 against three
	full mockups drawn on real recipes, and the reasoning is on #81 rather than
	repeated here. What it settled:

	  · the hero is the photograph at 320px with the title standing ON it, over
	    an indigo wash — near-solid at the hem, so the title is readable over a
	    photograph whose colours nobody chose;
	  · with no photograph the Cover leads and carries the title the same way,
	    bare, because its dye was chosen and needs no wash (#46). Only the 10.5px
	    Source line cannot clear the contrast bar there, so on a Cover it is set
	    on paper beneath the hero instead;
	  · the meta is one full-bleed ruled strip of three cells;
	  · an Ingredient Line is a hairline-ruled row led by a small indigo square,
	    its Reading subordinate beneath it;
	  · a Step is a number in a narrow column, in indigo, and the step at body
	    size;
	  · a Section, in either list, is the quiet uppercase heading over a rule.

	The Reading is also CORRECTED here, in place, on the line it belongs to (#32
	item 193) — see `Correcting.svelte`, which owns why that makes no Version.

	No mark distinguishes a line Kamosu read from one it did not (ADR 0002): the
	Reading is simply there or it is not. A badge that fires sometimes teaches
	people it fires always.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { DivergenceOutput, GetRecipeOutput } from '$lib/api/catalogue';
	import Cover from '$lib/cover/Cover.svelte';
	import { ratingLabel } from '$lib/rating';
	import Threshold from './Threshold.svelte';
	import MarkedRow from './MarkedRow.svelte';
	import Correcting from './Correcting.svelte';
	import {
		prose,
		draftVersion,
		reading,
		fieldText,
		rowKey,
		type Side,
		type Taken,
	} from './divergence';

	interface Props {
		branchId: string;
	}

	let { branchId }: Props = $props();

	const kamosu = useKamosu();

	let recipe = $state<GetRecipeOutput | undefined>(undefined);
	let divergence = $state<DivergenceOutput | undefined>(undefined);
	/** How many Branches of this Lineage exist, when that is more than the two the switch reads. */
	let crowded = $state(0);
	let failed = $state(false);

	/** Which recipe you are standing in. `mine` is always the Branch in the URL. */
	let side = $state<Side>('mine');
	/** Whether the divergence is marked at all. Off is simply the recipe. */
	let marks = $state(true);
	let open = $state(new Set<string>());
	let taken = $state(new Map<string, Taken>());
	let saving = $state(false);
	let changeNote = $state('');
	let saved = $state<'no' | 'yes' | 'failed'>('no');

	$effect(() => {
		let current = true;
		void (async () => {
			try {
				const read = await kamosu.getRecipe({ branch_id: branchId });
				if (!current) return;
				recipe = read;

				// Any Branch of the Lineage answers the same Thread, so this is how
				// the screen learns a second one exists at all. ADR 0014 designed
				// the switch for exactly two and says a third is not designed for —
				// so the recipe is read on its own, and the cook is told why rather
				// than the other Branches simply vanishing.
				const thread = await kamosu.getThread({ branch_id: branchId });
				if (!current) return;
				const others = thread.branches.filter((each) => each.branch_id !== branchId);
				if (others.length !== 1) {
					if (others.length > 1) crowded = thread.branches.length;
					return;
				}

				divergence = await kamosu.divergence({
					branch_id: branchId,
					other_branch_id: others[0].branch_id,
				});
			} catch (error) {
				if (!(error instanceof OperationError)) throw error;
				if (current) failed = true;
			}
		})();
		return () => {
			current = false;
		};
	});

	// ---- where you are standing ------------------------------------------

	const here = $derived(side === 'mine' ? divergence?.mine : divergence?.theirs);
	const there = $derived(side === 'mine' ? divergence?.theirs : divergence?.mine);
	/**
	 * The friend's Kitchen — `divergence.theirs` — and it does NOT flip when you
	 * cross over. Reading their recipe does not make you them, so a Ghost is
	 * described in terms of them from either side.
	 */
	const otherKitchen = $derived(divergence?.theirs.kitchen_name ?? '');

	const content = $derived(here?.content ?? recipe?.versions.at(-1)?.content);
	const readings = $derived(here?.readings ?? recipe?.versions.at(-1)?.readings ?? []);

	// ---- correcting a Reading --------------------------------------------

	/** One slot of `readings`: what Kamosu understood of a line, or nothing. */
	type Slot = GetRecipeOutput['versions'][number]['readings'][number];

	/** Which Ingredient Line has the corrector open, by index into the list. */
	let correcting = $state<number | null>(null);
	/**
	 * Readings corrected here, laid over what was fetched. `set_reading` makes
	 * no Version, so there is nothing to refetch and nothing that would show up
	 * in the Thread — the line simply reads differently from now on.
	 */
	let fixed = $state(new Map<number, Slot>());

	/**
	 * The Reading on one line, with anything corrected here laid over it.
	 *
	 * The overlay is consulted only while standing in your own recipe. It is
	 * keyed by line index, and the two Branches have their own lists — so on
	 * the other side index 2 is a different ingredient entirely, and reading
	 * through the overlay there would put your correction on their line.
	 */
	const readingAt = (index: number): Slot =>
		side === 'mine' && fixed.has(index) ? (fixed.get(index) ?? null) : (readings[index] ?? null);

	/**
	 * A Reading is corrected only on your own Branch. Standing in the other
	 * Kitchen's recipe you are reading it, not keeping it: their Branch is
	 * theirs and nothing on this screen writes into it (ADR 0007).
	 *
	 * This is the whole of the rule. A marked row reaches the corrector too,
	 * through a second target inside its unfolded panel rather than through its
	 * own tap — see `MarkedRow`'s `onFixReading`.
	 */
	const correctable = $derived(side === 'mine');

	function toggleCorrector(index: number) {
		correcting = correcting === index ? null : index;
	}

	function corrected(index: number, reading: Slot) {
		const next = new Map(fixed);
		next.set(index, reading);
		fixed = next;
		correcting = null;
	}

	const unshared = $derived(
		divergence
			? [...divergence.ingredients, ...divergence.steps].filter((row) => row.state !== 'same')
					.length
			: 0,
	);

	/** Marked rows are only ever drawn when there IS a divergence and it is shown. */
	const marking = $derived(Boolean(divergence) && marks);

	/**
	 * What the other side has for a single value, when the two do not agree.
	 * The marking covers the whole recipe, not only the two lists (ADR 0019):
	 * a Title renamed or a Yield halved is a difference a cook needs to see.
	 */
	function markOf(name: keyof DivergenceOutput['fields']): string | null {
		if (!marking || !divergence) return null;
		const field = divergence.fields[name];
		if (field.same) return null;
		const value = fieldText(side === 'mine' ? field.theirs : field.mine);
		// A Note is a block of prose; "{kitchen} has {value}" reads as nonsense
		// against one, so it is introduced as the note it is.
		return name === 'note'
			? m.divergence_field_note({ kitchen: otherKitchen, value })
			: m.divergence_field_differs({ kitchen: otherKitchen, value });
	}

	// Crossing over, and putting the marking away, both rebuild the list from a
	// different set of rows — so an open panel would reopen on whatever line
	// happens to land at that index. Both gestures close everything first.
	function cross() {
		side = side === 'mine' ? 'theirs' : 'mine';
		open = new Set();
		correcting = null;
	}

	function toggleMarks() {
		marks = !marks;
		open = new Set();
		correcting = null;
	}

	function toggle(key: string) {
		const next = new Set(open);
		if (next.has(key)) next.delete(key);
		else next.add(key);
		open = next;
	}

	function carry(key: string, what: Taken | 'undo') {
		const next = new Map(taken);
		if (what === 'undo') next.delete(key);
		else next.set(key, what);
		taken = next;
		saved = 'no';
	}

	function startSaving() {
		if (!divergence) return;
		changeNote = prose(divergence, taken);
		saving = true;
	}

	async function save() {
		if (!divergence) return;
		try {
			await kamosu.saveRecipeVersion(draftVersion(divergence, taken, changeNote));
			taken = new Map();
			saving = false;
			saved = 'yes';
			// A new Version can move a line's index, and both of these are keyed
			// by index. Neither survives the save.
			correcting = null;
			fixed = new Map();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			saved = 'failed';
		}
	}

	/** A Step's number, counted over the rows so a Ghost step takes none — it is
	 *  not a step of the recipe you are standing in. */
	function numbering() {
		let n = 0;
		return (ghost: boolean) => (ghost ? null : ++n);
	}
</script>

<div class="mx-auto max-w-2xl pb-tabbar" data-side={side}>
	{#if failed}
		<p class="px-gutter py-6 text-body text-support" role="alert">{m.recipe_failed()}</p>
	{:else if !content}
		<p class="px-gutter py-6 text-body text-ink-2">{m.loading()}</p>
	{:else}
		{#if divergence && here && there}
			<Threshold
				hereKitchen={here.kitchen_name}
				thereKitchen={there.kitchen_name}
				{unshared}
				{marks}
				{cross}
				{toggleMarks}
			/>
		{:else if crowded > 2}
			<p class="border-b border-rule bg-ground-2 px-gutter py-3 text-read text-ink-2">
				{m.divergence_too_many({ count: crowded })}
			</p>
		{/if}

		<!--
			What leads the recipe: its Main Photo, or — for the roughly one
			recipe in three that has none — its Cover (#46). A Cover is not a
			placeholder for a missing picture; it is what a recipe without one
			wears. That it is one of these two things is #46's; that the title
			stands on whichever it is, is #81's.
		-->
		{#if recipe}
			<div class="relative overflow-hidden">
				{#if content.main_photo}
					<img
						src="/api/photographs/{content.main_photo}/page"
						alt=""
						class="block w-full object-cover"
						style="height: var(--hero-h)"
					/>
					<!-- The wash. A photograph's colours are nobody's choice, so
					     the title is given ground of its own rather than hoping. -->
					<div class="wash pointer-events-none absolute inset-x-0 bottom-0"></div>
				{:else}
					<Cover lineageId={recipe.lineage_id} title={content.title} band={false} />
				{/if}
				<div class="absolute inset-x-0 bottom-0 px-gutter pt-8 pb-4">
					{#if content.main_photo && content.source}
						<p class="text-label text-on-accent uppercase">
							{m.recipe_from_source({ source: content.source.text })}
						</p>
					{/if}
					<h1 class="mt-1 font-display text-title font-semibold text-on-accent">{content.title}</h1>
				</div>
			</div>
		{/if}

		<!--
			On a Cover the hero carries the title alone: kinari over the pasta
			shape measures 3.9:1, which the 27px title clears and 10.5px text
			does not. So the Source is set here instead, on paper.
		-->
		{#if content.source && !content.main_photo}
			<p class="px-gutter pt-3 text-label text-ink-2 uppercase">
				{m.recipe_from_source({ source: content.source.text })}
			</p>
		{/if}
		{#if markOf('source')}
			<p class="px-gutter pt-2 text-read text-accent">{markOf('source')}</p>
		{/if}
		{#if markOf('title')}
			<p class="px-gutter pt-2 text-read text-accent">{markOf('title')}</p>
		{/if}

		<!-- The meta: one full-bleed strip, three cells, hairlines between. -->
		{#if content.prep_time_minutes !== null || content.cook_time_minutes !== null || content.yield}
			<div class="mt-4 flex border-y border-rule">
				{#if content.prep_time_minutes !== null}
					<div class="flex-1 px-2 py-3 text-center">
						<b class="block font-display text-panel-figure font-semibold">
							{content.prep_time_minutes}
						</b>
						<span class="mt-1 block text-label text-ink-2 uppercase">{m.recipe_min_prep()}</span>
					</div>
				{/if}
				{#if content.cook_time_minutes !== null}
					<div class="flex-1 border-l border-rule px-2 py-3 text-center first:border-l-0">
						<b class="block font-display text-panel-figure font-semibold">
							{content.cook_time_minutes}
						</b>
						<span class="mt-1 block text-label text-ink-2 uppercase">{m.recipe_min_cook()}</span>
					</div>
				{/if}
				{#if content.yield}
					<div class="flex-1 border-l border-rule px-2 py-3 text-center first:border-l-0">
						<b class="block font-display text-panel-figure font-semibold">
							{content.yield.amount}
						</b>
						<span class="mt-1 block text-label text-ink-2 uppercase">{content.yield.noun}</span>
					</div>
				{/if}
			</div>
		{/if}
		{#each ['prep_time_minutes', 'cook_time_minutes', 'yield'] as const as name (name)}
			{#if markOf(name)}
				<p class="mt-1 px-gutter text-read text-accent">{markOf(name)}</p>
			{/if}
		{/each}

		<!--
			One Ingredient Line, marked or not — the same row either way, because
			a list a cook shops from must keep one rhythm whether or not a second
			Branch happens to exist. `at` is the line's index into the written
			list, or -1 for a row with no line of its own to correct.
		-->
		{#snippet ingredientLine(text: string, at: number)}
			{@const readable = correctable && at >= 0}
			<li class="flex gap-3 border-b border-rule py-3">
				<span class="ingredient-marker shrink-0 bg-accent" aria-hidden="true"></span>
				<div class="min-w-0 flex-1">
					{#if readable}
						<button
							type="button"
							class="block w-full text-left"
							aria-expanded={correcting === at}
							onclick={() => toggleCorrector(at)}
						>
							{@render written(text, at)}
						</button>
					{:else}
						{@render written(text, at)}
					{/if}
					{#if readable && correcting === at}
						<Correcting
							{branchId}
							lineIndex={at}
							reading={readingAt(at)}
							onDone={(next) => corrected(at, next)}
							onCancel={() => (correcting = null)}
						/>
					{/if}
				</div>
			</li>
		{/snippet}

		<!--
			The written Line, and beneath it what Kamosu read from it — smaller,
			subordinate, and simply absent where there is no Reading. Nothing
			here says which of the two happened (ADR 0002).
		-->
		{#snippet written(text: string, at: number)}
			<span class="block text-line">{text}</span>
			{#if at >= 0 && reading(readingAt(at))}
				<span class="block text-read text-ink-2">{reading(readingAt(at))}</span>
			{/if}
		{/snippet}

		<!-- Ingredients ------------------------------------------------------ -->
		<h2 class="mx-gutter mt-8 mb-2 font-display text-label font-semibold text-accent uppercase">
			{m.recipe_ingredients()}
		</h2>
		<ul class="px-gutter">
			{#if marking && divergence}
				{#each divergence.ingredients as row, index (rowKey('ingredients', index))}
					{@const key = rowKey('ingredients', index)}
					{@const own = side === 'mine' ? row.mine : row.theirs}
					{#if row.kind === 'section'}
						<li class="border-b border-rule py-4 pb-1">
							<h3 class="font-display text-label text-ink-2 uppercase">
								{(own ?? row.mine ?? row.theirs)?.text}
							</h3>
						</li>
					{:else if row.state === 'same'}
						{@render ingredientLine(own?.text ?? '', own?.index ?? -1)}
					{:else}
						<MarkedRow
							{row}
							{side}
							{otherKitchen}
							readingText={own ? reading(readingAt(own.index)) : ''}
							open={open.has(key)}
							taken={taken.get(key)}
							onToggle={() => toggle(key)}
							onCarry={(what) => carry(key, what)}
							onFixReading={correctable && own ? () => toggleCorrector(own.index) : undefined}
						/>
						{#if correctable && own && correcting === own.index}
							<li class="border-b border-rule pb-3 pl-3">
								<Correcting
									{branchId}
									lineIndex={own.index}
									reading={readingAt(own.index)}
									onDone={(next) => corrected(own.index, next)}
									onCancel={() => (correcting = null)}
								/>
							</li>
						{/if}
					{/if}
				{/each}
			{:else}
				{#each content.ingredients as item, index (index)}
					{#if item.kind === 'section'}
						<li class="border-b border-rule py-4 pb-1">
							<h3 class="font-display text-label text-ink-2 uppercase">{item.text}</h3>
						</li>
					{:else}
						{@render ingredientLine(item.text, index)}
					{/if}
				{/each}
			{/if}
		</ul>

		<!-- Method ----------------------------------------------------------- -->
		<h2 class="mx-gutter mt-8 mb-2 font-display text-label font-semibold text-accent uppercase">
			{m.recipe_method()}
		</h2>
		<ol class="px-gutter">
			{#if marking && divergence}
				{@const number = numbering()}
				{#each divergence.steps as row, index (rowKey('steps', index))}
					{@const key = rowKey('steps', index)}
					{@const own = side === 'mine' ? row.mine : row.theirs}
					{@const n = number(!own)}
					{#if row.kind === 'section'}
						<li class="border-b border-rule py-4 pb-1">
							<h3 class="font-display text-label text-ink-2 uppercase">
								{(own ?? row.mine ?? row.theirs)?.text}
							</h3>
						</li>
					{:else if row.state === 'same'}
						<li class="flex gap-3 border-b border-rule py-3">
							<span class="w-6 shrink-0 font-display text-line font-semibold text-accent">{n}</span>
							<p class="text-body">{own?.text}</p>
						</li>
					{:else}
						<MarkedRow
							{row}
							{side}
							{otherKitchen}
							number={n}
							open={open.has(key)}
							taken={taken.get(key)}
							onToggle={() => toggle(key)}
							onCarry={(what) => carry(key, what)}
						/>
					{/if}
				{/each}
			{:else}
				{@const number = numbering()}
				{#each content.steps as item, index (index)}
					{#if item.kind === 'section'}
						<li class="border-b border-rule py-4 pb-1">
							<h3 class="font-display text-label text-ink-2 uppercase">{item.text}</h3>
						</li>
					{:else}
						<li class="flex gap-3 border-b border-rule py-3">
							<span class="w-6 shrink-0 font-display text-line font-semibold text-accent">
								{number(false)}
							</span>
							<p class="text-body">{item.text}</p>
						</li>
					{/if}
				{/each}
			{/if}
		</ol>

		{#if content.note}
			<div class="mx-gutter mt-6 border-l-2 border-accent py-1 pl-4 text-body whitespace-pre-wrap">
				{content.note}
			</div>
		{/if}
		{#if markOf('note')}
			<p class="mx-gutter mt-1 text-read text-accent">{markOf('note')}</p>
		{/if}

		<!--
			Cooked (#59). How this dish has actually gone: how many times, when
			last, and each Person's most recent verdict with their name.

			There is no average here and no way to build one, which is ADR 0015
			working rather than a rule anybody has to remember — a rating is a
			word, and only the newest one each Person gave ever arrives, so a
			verdict somebody has since superseded cannot drag down a recipe that
			was fixed months ago. A Person who cooked and said nothing simply
			does not appear: silence is not a score of zero.
		-->
		{#if recipe}
			<h2 class="mx-gutter mt-8 mb-2 font-display text-label font-semibold text-accent uppercase">
				{m.recipe_cooked()}
			</h2>
			<div class="px-gutter">
				{#if recipe.cooked.count === 0 || !recipe.cooked.last_cooked_at}
					<p class="text-read text-ink-2">{m.recipe_cooked_never()}</p>
				{:else}
					{@const when = new Date(recipe.cooked.last_cooked_at).toLocaleDateString()}
					<p class="text-read text-ink-2">
						{recipe.cooked.count === 1
							? m.recipe_cooked_once({ when })
							: m.recipe_cooked_times({ count: recipe.cooked.count, when })}
					</p>
					<ul>
						{#each recipe.cooked.ratings as verdict (verdict.person_id)}
							<li class="flex items-baseline justify-between gap-3 border-b border-rule py-2">
								<span class="text-line">{verdict.name}</span>
								<span class="text-read text-accent uppercase">{ratingLabel(verdict.rating)}</span>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		{/if}

		{#if saved === 'yes'}
			<p class="mx-gutter mt-4 text-read text-accent" role="status">{m.divergence_saved()}</p>
		{:else if saved === 'failed'}
			<p class="mx-gutter mt-4 text-read text-support" role="alert">{m.divergence_save_failed()}</p>
		{/if}

		<button
			class="mx-gutter mt-6 block w-[calc(100%-2*var(--spacing-gutter))] bg-accent p-4 text-center font-display text-body text-on-accent"
		>
			{m.recipe_cook_this()}
		</button>
		<a
			href="/recipes/{branchId}/thread"
			class="mx-gutter mt-2 block border border-rule p-4 text-center font-display text-body text-accent"
		>
			{m.recipe_the_thread()}
		</a>
	{/if}

	<!-- Carried across and not yet saved. It becomes real only when an ordinary
	     Version is saved — there is no other kind of save here. -->
	{#if taken.size > 0 && divergence}
		<div
			class="fixed inset-x-0 bottom-tabbar z-30 mx-auto max-w-2xl border-t border-on-accent/25 bg-accent px-gutter py-3 text-on-accent"
		>
			<p class="mb-2 text-read">
				{taken.size === 1
					? m.divergence_unsaved_one({ kitchen: divergence.theirs.kitchen_name })
					: m.divergence_unsaved({
							count: taken.size,
							kitchen: divergence.theirs.kitchen_name,
						})}
			</p>
			<div class="flex gap-2">
				<button
					type="button"
					onclick={startSaving}
					class="flex-1 bg-on-accent p-2 text-center text-read text-accent"
				>
					{m.divergence_save()}
				</button>
				<button
					type="button"
					onclick={() => (taken = new Map())}
					class="flex-1 border border-on-accent/40 p-2 text-center text-read"
				>
					{m.divergence_undo()}
				</button>
			</div>
		</div>
	{/if}
</div>

{#if saving && divergence}
	<div class="fixed inset-0 z-40 bg-accent/40"></div>
	<div
		class="py-5 fixed inset-x-0 bottom-0 z-50 mx-auto max-h-[78vh] max-w-2xl overflow-y-auto bg-ground px-gutter pb-safe"
		role="dialog"
		aria-modal="true"
		aria-label={m.divergence_save()}
	>
		<h3 class="font-display text-title font-semibold">{m.divergence_save()}</h3>
		<label class="mt-4 block text-label text-ink-2 uppercase" for="what-changed">
			{m.divergence_what_changed()}
		</label>
		<textarea
			id="what-changed"
			rows="3"
			bind:value={changeNote}
			class="mt-1 w-full rounded-sm border border-rule bg-card p-3 text-body"></textarea>
		<p class="mt-2 text-read text-ink-2">
			{m.divergence_save_hint({ kitchen: divergence.theirs.kitchen_name })}
		</p>
		<button
			type="button"
			onclick={save}
			class="mt-4 block w-full bg-accent p-4 text-center font-display text-body text-on-accent"
		>
			{m.divergence_save()}
		</button>
		<button
			type="button"
			onclick={() => (saving = false)}
			class="mt-2 block w-full border border-rule p-4 text-center font-display text-body text-accent"
		>
			{m.divergence_cancel()}
		</button>
	</div>
{/if}

<style>
	/* The room you are standing in colours the page: indigo is yours, beni is
	   theirs. It is what lets the marking stay quiet on the lines themselves,
	   and `MarkedRow` reads `--whose` for its margin rule. */
	[data-side='mine'] {
		--whose: var(--color-accent);
	}
	[data-side='theirs'] {
		--whose: var(--color-support);
	}

	/* The wash under the title, in indigo rather than a neutral grey: the app's
	   own colour, and near-solid at the hem so the title is readable over any
	   photograph. Written with color-mix against the token so it cannot drift
	   from the palette the way a pasted hex would.

	   Its height and stops are fitted, not chosen by eye: over the worst
	   backdrop a photograph can be — pure white — kinari must clear 3:1 at the
	   27px title and 4.5:1 at the 10.5px Source line above it. A shorter wash
	   left the Source at 3.5:1. Starting it higher, rather than making it
	   darker, buys the contrast while keeping the fade as gentle at any given
	   point as it already was. */
	.wash {
		height: 200px;
		background: linear-gradient(
			to bottom,
			color-mix(in srgb, var(--color-accent) 0%, transparent) 0%,
			color-mix(in srgb, var(--color-accent) 55%, transparent) 40%,
			color-mix(in srgb, var(--color-accent) 95%, transparent) 100%
		);
	}
</style>
