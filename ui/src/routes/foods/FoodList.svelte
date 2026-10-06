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

	**On the wide layout this is the list beside the open Food** (#199, ADR
	0044), drawn by `Foods.svelte`. It is 300 wide and stays where it is while
	one Food after another is opened: its title and search box at the top, its
	rows scrolling under them. A row there is the name with the number of lines
	at its right, and the open Food's row is marked. Aurélien chose this on 6
	October 2026 over the rows the phone has, which need a list 420 wide.

	**The arrows walk it.** Up and down open the Food above or below the open
	one, in place of it: the history entry is replaced, so going back leaves
	the list in one step. A click on a row is an ordinary link and adds an
	entry. Tab reaches one row of the list and no more, and the caret stays on
	a row that was clicked or walked to, so the open Food is one Tab from its
	row.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { untrack } from 'svelte';
	import { useKamosu } from '$lib/kamosu';
	import { useRoom } from '$lib/room.svelte';
	import { openInPlace, stepInList, typingInAField } from '$lib/shell/walk';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';
	import Empty from '$lib/shell/Empty.svelte';
	import { foldWord, linesPointingAt, type FoodRow } from '$lib/foods';
	import { useCorrected } from './corrected.svelte';

	interface Props {
		/** The word the Reading corrector arrived with, where it sent one. */
		q?: string;
		/** The Food that is open beside this list, on the wide layout. */
		open?: string;
	}

	let { q = '', open }: Props = $props();

	const kamosu = useKamosu();
	const room = useRoom();
	const corrected = useCorrected();

	/** Where a Food's page is, on every layout. */
	const addressOf = (id: string) => `/foods/${id}`;

	let foods = $state<FoodRow[] | undefined>(undefined);
	/** The Core's own refusal, where one arrived. Its words, not a paraphrase. */
	let said = $state<string | undefined>(undefined);
	/**
	 * Seeded from the word the corrector sent, and then it belongs to whoever
	 * is typing in it. Arriving at the list with a second word puts that word
	 * in the box, since SvelteKit keeps one component across two addresses
	 * that differ only in their query. An open Food's address carries no word,
	 * and that is no second arrival: the box keeps what it holds.
	 */
	// svelte-ignore state_referenced_locally
	let typed = $state(q);
	// svelte-ignore state_referenced_locally
	let seeded = q;
	$effect(() => {
		if (open !== undefined || q === seeded) return;
		seeded = q;
		typed = q;
	});

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

	// A Food corrected on the page beside this list takes its row's place, so
	// the row and the search answer to what it is called now.
	$effect(() => {
		const food = corrected?.latest;
		if (food === undefined) return;
		untrack(() => {
			foods = foods?.map((row) => (row.id === food.id ? food : row));
		});
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

	/** The room a line of words keeps from the list's edges, which a row keeps for itself. */
	const pad = $derived(room.wide ? 'px-gutter' : undefined);

	/** The row Tab stops on: the open Food's, or the first where none of these is open. */
	const tabStop = $derived(drawn.some((food) => food.id === open) ? open : drawn[0]?.id);

	let rows = $state<HTMLElement | undefined>(undefined);

	function walk(event: KeyboardEvent) {
		if (!room.wide || typingInAField(event)) return;
		const to = stepInList(
			event,
			drawn.map((food) => food.id),
			open,
		);
		if (to === undefined) return;
		event.preventDefault();
		if (to === open) return;
		const row = rows?.querySelector<HTMLElement>(`a[href="${addressOf(to)}"]`);
		// The caret goes to the row, so Tab goes on into the Food it opened.
		row?.focus({ preventScroll: true });
		row?.scrollIntoView?.({ block: 'nearest' });
		void openInPlace(addressOf(to));
	}
</script>

<svelte:window onkeydown={walk} />

{#snippet search()}
	<label class="block">
		<span class="block text-label text-ink-2 uppercase">{m.foods_search_label()}</span>
		<input
			bind:value={typed}
			type="search"
			class="mt-1 w-full rounded-sm border border-rule bg-ground p-2 text-body"
			placeholder={m.foods_search_placeholder()}
		/>
	</label>
{/snippet}

{#snippet results()}
	{#if said}
		<p class={['mt-4 text-body text-support', pad]} role="alert">{said}</p>
	{:else if foods === undefined}
		<p class={['mt-4 text-body text-ink-2', pad]}>{m.loading()}</p>
	{:else if foods.length === 0}
		<div class={['mt-4', pad]}><Empty>{m.foods_none()}</Empty></div>
	{:else if drawn.length === 0}
		<!--
			Not a dead end. The corrector arrives carrying the word WRITTEN on the
			ingredient line, and correcting a Food's name replaces the name rather
			than adding to it — so a Food renamed after its recipe was written no
			longer answers to the word the line still says. That is the one way
			this door can miss, and the way back is the list itself.
		-->
		<div class={['mt-4', pad]}>
			<Empty>{m.foods_no_match({ word: typed })}</Empty>
			<button
				type="button"
				onclick={() => (typed = '')}
				class="mt-3 w-full rounded-sm border border-rule p-2 text-center text-read text-accent"
			>
				{m.foods_show_all()}
			</button>
		</div>
	{:else}
		<ul class={[!room.wide && 'mt-4']} bind:this={rows}>
			{#each drawn as food (food.id)}
				<li>
					{#if room.wide}
						<a
							href={addressOf(food.id)}
							data-sveltekit-keepfocus
							aria-current={food.id === open ? 'page' : undefined}
							tabindex={food.id === tabStop ? 0 : -1}
							class={[
								'flex min-h-12 items-center justify-between gap-3 border-b border-l-3 border-rule px-gutter py-3',
								food.id === open ? 'border-l-accent bg-card' : 'border-l-transparent',
							]}
						>
							<span class="min-w-0 truncate text-body text-ink"
								>{food.name ?? m.food_unnamed()}</span
							>
							<!-- The number is for the eye. Read aloud it is the sentence the phone's row writes. -->
							<span aria-hidden="true" class="text-read text-ink-2 tabular-nums"
								>{food.reading_count}</span
							>
							<span class="sr-only">{linesPointingAt(food.reading_count)}</span>
						</a>
					{:else}
						<a
							href={addressOf(food.id)}
							class="flex min-h-12 items-center justify-between gap-3 border-b border-rule py-3"
						>
							<span class="min-w-0">
								<span class="block truncate text-body text-ink"
									>{food.name ?? m.food_unnamed()}</span
								>
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
					{/if}
				</li>
			{/each}
		</ul>
		{#if found.length > drawn.length}
			<p class={['mt-3 text-read text-ink-2', pad]}>
				{found.length - drawn.length === 1
					? m.foods_more_one()
					: m.foods_more({ count: found.length - drawn.length })}
			</p>
		{/if}
	{/if}
{/snippet}

{#if room.wide}
	<div class="list-column">
		<div class="border-b border-rule px-gutter pb-3">
			<!-- The page's title until a Food is open, whose name is the title then. -->
			<svelte:element
				this={open === undefined ? 'h1' : 'h2'}
				class="beside-back-arrow flex min-h-12 items-center font-display text-title font-semibold"
			>
				{m.foods_title()}
			</svelte:element>
			<div class="mt-3">{@render search()}</div>
		</div>
		<div class="min-h-0 flex-1 overflow-y-auto pb-6">{@render results()}</div>
	</div>
{:else}
	<Screen title={m.foods_title()} blurb={m.foods_blurb()}>
		{@render search()}
		{@render results()}
	</Screen>
{/if}
