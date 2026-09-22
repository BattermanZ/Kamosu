<!--
	The Foods: every edible thing Kamosu learnt by itself, and the way to one (#107).

	**Why a screen exists at all.** A Food is the most numerous thing Kamosu
	knows — 607 on the dev instance — and until this ticket it was the only one
	with no screen. Four of the five Operations that read and correct a Food had
	no caller anywhere in the interface, so a Cup Weight could never be supplied
	and a Food could never be given its name in a second Language. Both were
	designed to matter and both were unreachable.

	**This list is a way THROUGH, not a thing to read.** Nobody browses 607 rows
	for pleasure, so the search field is the screen and the list under it is the
	result. It opens showing the Foods the most lines point at, because those are
	the ones where correcting anything changes the most.

	**It answers to any of a Food's names.** Searching `sel` finds the Food called
	`salt` where somebody has taught it both, which is the whole mechanism ADR
	0006 built naming-per-Language for. Matching is done here on the plain folded
	word rather than asked of the Core, because `list_foods` already carries every
	name and a round trip per keystroke would be slower and no more correct.

	**`?q=` is how the Reading corrector arrives.** A Reading carries the written
	word and never a Food's id — deliberately, since a Food is instance-local and
	an id would mean nothing in a Bundle (see `reading_schema` in the Catalogue).
	So the corrector hands over the word and this screen resolves it. Where two
	Foods answer to one word, which ADR 0022 permits, both are shown rather than
	one being guessed at: the duplicate is the thing worth seeing.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';
	import Empty from '$lib/shell/Empty.svelte';
	import { foldWord, linesPointingAt, type FoodRow } from '$lib/foods';

	interface Props {
		/** The word the Reading corrector arrived with, where it sent one. */
		q?: string;
	}

	let { q = '' }: Props = $props();

	const kamosu = useKamosu();

	let foods = $state<FoodRow[] | undefined>(undefined);
	/** The Core's own refusal, where one arrived. Its words, not a paraphrase. */
	let said = $state<string | undefined>(undefined);
	/**
	 * Seeded once from the word the corrector sent. The route wraps this
	 * component in `{#key q}`, so arriving with a second word makes a second
	 * component rather than leaving the first one's box behind — which is what
	 * seeding-once would otherwise do, since SvelteKit reuses a component across
	 * two URLs that differ only in their query.
	 */
	// svelte-ignore state_referenced_locally
	let typed = $state(q);

	$effect(() => {
		let current = true;
		kamosu
			.listFoods({})
			.then((answer) => {
				if (current) foods = answer.foods;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				if (current) said = error.message || m.foods_unreachable();
			});
		return () => {
			current = false;
		};
	});

	/**
	 * How many rows are drawn at once. A search narrow enough to be useful is
	 * well under this; a library of 607 drawn in full is a scroll nobody
	 * finishes and a redraw on every keystroke.
	 */
	const SHOWN = 40;

	const wanted = $derived(foldWord(typed));

	/**
	 * The rows to draw. An exact match on any of a Food's names comes first —
	 * that is what the corrector arrived for — then the merely similar, and
	 * within each the Foods the most lines point at.
	 */
	/** How well a Food answers to what was typed. Lower comes first. */
	const ANSWERS_EXACTLY = 0;
	const CONTAINS_IT = 1;
	const NOTHING_TYPED = 2;
	const NO_ANSWER = 3;

	const found = $derived.by(() => {
		const rank = (food: FoodRow) => {
			if (wanted === '') return NOTHING_TYPED;
			const names = food.names.map((named) => foldWord(named.name));
			if (names.some((name) => name === wanted)) return ANSWERS_EXACTLY;
			if (names.some((name) => name.includes(wanted))) return CONTAINS_IT;
			return NO_ANSWER;
		};
		return (foods ?? [])
			.map((food) => ({ food, rank: rank(food) }))
			.filter((row) => row.rank !== NO_ANSWER)
			.sort((a, b) => a.rank - b.rank || b.food.reading_count - a.food.reading_count)
			.map((row) => row.food);
	});

	const drawn = $derived(found.slice(0, SHOWN));
</script>

<Screen title={m.foods_title()} blurb={m.foods_blurb()}>
	<label class="block">
		<span class="block text-label text-ink-2 uppercase">{m.foods_search_label()}</span>
		<input
			bind:value={typed}
			type="search"
			class="mt-1 w-full rounded-sm border border-rule bg-ground p-2 text-body"
			placeholder={m.foods_search_placeholder()}
		/>
	</label>

	{#if said}
		<p class="mt-4 text-body text-support" role="alert">{said}</p>
	{:else if foods === undefined}
		<p class="mt-4 text-body text-ink-2">{m.loading()}</p>
	{:else if foods.length === 0}
		<div class="mt-4"><Empty>{m.foods_none()}</Empty></div>
	{:else if drawn.length === 0}
		<!--
			Not a dead end. The corrector arrives carrying the word WRITTEN on the
			ingredient line, and correcting a Food's name replaces the name rather
			than adding to it — so a Food renamed after its recipe was written no
			longer answers to the word the line still says. That is the one way
			this door can miss, and the way back is the list itself.
		-->
		<div class="mt-4"><Empty>{m.foods_no_match({ word: typed })}</Empty></div>
		<button
			type="button"
			onclick={() => (typed = '')}
			class="mt-3 w-full rounded-sm border border-rule p-2 text-center text-read text-accent"
		>
			{m.foods_show_all()}
		</button>
	{:else}
		<ul class="mt-4">
			{#each drawn as food (food.id)}
				<li>
					<a
						href="/foods/{food.id}"
						class="flex min-h-12 items-center justify-between gap-3 border-b border-rule py-3"
					>
						<span class="min-w-0">
							<span class="block truncate text-body text-ink">{food.name ?? m.food_unnamed()}</span>
							<span class="text-read text-ink-2">
								{linesPointingAt(food.reading_count)}
								{#if food.cup_weight_grams !== null}
									· {m.food_cup_weighs({ grams: food.cup_weight_grams })}
								{/if}
								{#if food.names.length > 1}
									· {food.names.map((named) => named.name).join(' · ')}
								{/if}
							</span>
						</span>
						<span aria-hidden="true" class="text-ink-2">›</span>
					</a>
				</li>
			{/each}
		</ul>
		{#if found.length > drawn.length}
			<p class="mt-3 text-read text-ink-2">
				{found.length - drawn.length === 1
					? m.foods_more_one()
					: m.foods_more({ count: found.length - drawn.length })}
			</p>
		{/if}
	{/if}
</Screen>
