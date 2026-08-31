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

	THE SUBORDINATE LINE IS ONE SLOT (#49, ADR 0016). Scaling and conversion are
	one act and share it with the Reading echo, so a row never carries two small
	lines under its written one. What the slot holds, in order: the converted
	amount where this reader needs one — `about 250 g` under `2 cups flour` for
	a metric cook — and otherwise the echo of what Kamosu read. A reader already
	in her own measures gets the echo, because the conversion has nothing to say
	and a blank slot would tell her less than the echo does. Both are quiet, both
	are visibly Kamosu's rather than the cook's, and neither is ever a badge.

	SINCE #71 THE ECHO IS SILENT WHERE IT WOULD ONLY REPEAT THE LINE, which is
	the rule the conversion already obeyed. The echo was chosen while a Reading
	existed only because somebody had typed one — it was the only way to see
	what Kamosu held. Now Kamosu reads every line it can, so an unfiltered echo
	would set `Za’tar` under `Za’tar` on most rows, and would appear on exactly
	the lines Kamosu managed to read: the badge ADR 0002 refuses, arrived at
	sideways. The echo that survives is the one that earns its place — a Reading
	somebody CORRECTED, saying something the written line does not.

	A Step's slot holds the oven temperature in the other system, on the
	conventional ladder — an addition beside the sentence, never written into it.
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

				// Home's *recently opened* shelf, and the whole of what feeds it
				// (#64, ADR 0027). It is a separate Operation rather than
				// something `get_recipe` does on the side, because `get_recipe`
				// must stay a read: a read-only Access Key may read every recipe,
				// and making the reading itself a write would lock it out of the
				// library.
				//
				// Nothing waits on it and nothing is shown if it fails. It is one
				// timestamp that never travels and costs nothing if lost, so it
				// may never be the reason a cook cannot read a recipe — a
				// read-only Key is refused here every time, and reads on.
				void kamosu.noteRecipeOpened({ branch_id: branchId }).catch(() => {});

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
	/**
	 * The one subordinate line the Core worked out for THIS reader — scaling and
	 * conversion in a single slot (#49). Computed there rather than here on
	 * purpose: an agent at the MCP door gets the same answer this screen shows,
	 * and the arithmetic lives in one place beneath both Doors.
	 */
	const measured = $derived(
		here?.measured ?? recipe?.versions.at(-1)?.measured ?? { ingredients: [], steps: [] },
	);

	// ---- correcting a Reading --------------------------------------------

	/** One slot of `readings`: what Kamosu understood of a line, or nothing. */
	type Slot = GetRecipeOutput['versions'][number]['readings'][number];
	/**
	 * A line corrected here: the Reading as it now stands, and the one
	 * subordinate line it now produces. They travel together because
	 * `set_reading` answers with both — the conversion is the Core's, and this
	 * screen only ever displays it.
	 */
	type Fixed = { reading: Slot; measured: string | null };

	/** Which Ingredient Line has the corrector open, by index into the list. */
	let correcting = $state<number | null>(null);
	/**
	 * Readings corrected here, laid over what was fetched. `set_reading` makes
	 * no Version, so there is nothing to refetch and nothing that would show up
	 * in the Thread — the line simply reads differently from now on.
	 */
	let fixed = $state(new Map<number, Fixed>());

	/**
	 * The Reading on one line, with anything corrected here laid over it.
	 *
	 * The overlay is consulted only while standing in your own recipe. It is
	 * keyed by line index, and the two Branches have their own lists — so on
	 * the other side index 2 is a different ingredient entirely, and reading
	 * through the overlay there would put your correction on their line.
	 */
	const readingAt = (index: number): Slot =>
		side === 'mine' && fixed.has(index)
			? (fixed.get(index)?.reading ?? null)
			: (readings[index] ?? null);

	/**
	 * **The one line beneath an Ingredient Line**, and the whole of the rule:
	 * the converted amount where this reader needs one, the echo of what Kamosu
	 * read where she does not, and nothing at all where there is neither. Never
	 * both (#49, ADR 0016).
	 *
	 * **The echo is silent where it would only repeat the line above it**, which
	 * is the same rule the conversion already obeys. It was written when a
	 * Reading existed only because somebody had typed one, so an echo was the
	 * only way to see what Kamosu held; since #71 Kamosu reads every line it
	 * can, and an echo that parrots the line is two things it must not be — a
	 * repetition ADR 0016 says must be absent, and a mark on exactly the lines
	 * Kamosu managed to read, which is the badge ADR 0002 refuses. What is
	 * left is the echo that earns its place: the Reading somebody **corrected**,
	 * which says something the line does not.
	 *
	 * Like `readingAt`, the overlay is consulted only in your own recipe: the
	 * two Branches have their own lists, so index 2 on the other side is a
	 * different ingredient entirely.
	 */
	function beneathLine(index: number): string {
		if (index < 0) return '';
		const converted =
			side === 'mine' && fixed.has(index)
				? (fixed.get(index)?.measured ?? null)
				: (measured.ingredients[index] ?? null);
		if (converted) return converted;
		const echo = reading(readingAt(index));
		const written = content?.ingredients?.[index]?.text ?? '';
		return echo && saysMoreThan(echo, written) ? echo : '';
	}

	/**
	 * Whether an echo is worth showing under the line it was read from: it is,
	 * only where some word of it is not already up there. Compared as bare
	 * letters and digits, because the echo drops the punctuation and the
	 * articles the line keeps — `2 gousses ail` is entirely inside
	 * `2 gousses d’ail` and says nothing new, while a corrected `250 g farine
	 * de blé` under `200 g de farine` says two things.
	 */
	function saysMoreThan(echo: string, line: string): boolean {
		const bare = (text: string) => text.toLowerCase().replace(/[^\p{L}\p{N} ]/gu, '');
		const written = bare(line);
		return bare(echo)
			.split(/\s+/)
			.filter(Boolean)
			.some((word) => !written.includes(word));
	}

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

	function corrected(index: number, reading: Slot, measuredLine: string | null) {
		const next = new Map(fixed);
		next.set(index, { reading, measured: measuredLine });
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
					<div class="pointer-events-none absolute inset-x-0 bottom-0 wash"></div>
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
							onDone={(next, converted) => corrected(at, next, converted)}
							onCancel={() => (correcting = null)}
						/>
					{/if}
				</div>
			</li>
		{/snippet}

		<!--
			The written Line, and beneath it the one subordinate line — smaller,
			quieter, and simply absent where there is nothing to say. Nothing
			here says whether it is a conversion or an echo, and nothing says
			whether Kamosu read the line at all (ADR 0002).
		-->
		{#snippet written(text: string, at: number)}
			<span class="block text-line">{text}</span>
			{#if beneathLine(at)}
				<span class="block text-read text-ink-2">{beneathLine(at)}</span>
			{/if}
		{/snippet}

		<!--
			A Step's own subordinate slot: the oven temperature in this reader's
			measures, on the conventional ladder (ADR 0016). It is an addition
			BESIDE the sentence and is never written into it — a Step's truth is
			its text — and it is absent from the great majority of steps, which
			carry no temperature or already print both.
		-->
		{#snippet beside(at: number)}
			{#if at >= 0 && measured.steps[at]}
				<span class="mt-1 block text-read text-ink-2">{measured.steps[at]}</span>
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
							beneath={own ? beneathLine(own.index) : ''}
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
									onDone={(next, converted) => corrected(own.index, next, converted)}
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
							<div class="min-w-0 flex-1">
								<p class="text-body">{own?.text}</p>
								{@render beside(own?.index ?? -1)}
							</div>
						</li>
					{:else}
						<MarkedRow
							{row}
							{side}
							{otherKitchen}
							number={n}
							beneath={own ? (measured.steps[own.index] ?? '') : ''}
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
							<div class="min-w-0 flex-1">
								<p class="text-body">{item.text}</p>
								{@render beside(index)}
							</div>
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

		<!--
			Into the cooking screen (#61, ADR 0011). It is a link rather than a
			button that starts something: opening the screen IS starting the
			Attempt, and one already In Progress is handed back rather than
			doubled — so there is nothing here to press twice by mistake.
		-->
		<a
			href="/cook/{branchId}"
			class="mx-gutter mt-6 block w-[calc(100%-2*var(--spacing-gutter))] bg-accent p-4 text-center font-display text-body text-on-accent"
		>
			{m.recipe_cook_this()}
		</a>
		<a
			href="/recipes/{branchId}/thread"
			class="mx-gutter mt-2 block border border-rule p-4 text-center font-display text-body text-accent"
		>
			{m.recipe_the_thread()}
		</a>
		<!--
			Into the share screen (#65, ADR 0026). A link rather than a switch
			here on purpose: turning sharing on is one deliberate act taken on a
			screen that says what it means, not a toggle brushed past on the way
			to cooking.
		-->
		<a
			href="/recipes/{branchId}/share"
			class="mx-gutter mt-2 block border border-rule p-4 text-center font-display text-body text-accent"
		>
			{m.share_title()}
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

	/* The wash under the title is `@utility wash` in ui/src/app.css. It moved
	   there in #65, which gave the Share Link page the same hero and the
	   messaging-app card a picture of it: one gradient rendered three ways is
	   exactly what the stylesheet exists to keep from drifting. Its fitted
	   height and stops, and why they are what they are, are recorded beside it.
	*/
</style>
