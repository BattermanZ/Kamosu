<!--
	Shopping: one Person's standing choice of what they are about to cook (#73,
	ADR 0024).

	**The choosing is stored; the rows are computed.** Nothing on this screen is
	worked out here — every row, every amount and every word of an amount
	arrives from the Core, which is what lets an agent asked *what is on my
	list* get the identical answer through the other Door. This file decides
	what a row looks like and nothing about what it says.

	**What a row looks like was chosen by Aurélien on 2026-09-01**, out of four
	treatments of one real list served side by side, and the reasoning is
	recorded on issue #73. The shape is *Three admissions*:

	- A row whose amounts added stays on **one line** — the Food reads left, the
	  amount sits right in its own tabular column.
	- A row the arithmetic could not close **breaks open** into the recipes it
	  came from, each amount under the name of the recipe that wanted it. That
	  is the whole point of it: standing in a shop, `about 30 ml + 4 cloves`
	  tells you two amounts but not which dish goes short if you buy one.
	- A row nobody wrote an amount for says **some** where the figure would be,
	  in the quiet ink and in the same slot every other amount uses.
	- A line kept as typed carries **no amount slot at all**.

	The stated cost, which was put in front of him and accepted: a taller row
	now means *Kamosu could not add this*, and that is a shape appearing only
	sometimes. It survives ADR 0015 for the same reason the recipe screen's
	subordinate line does — the two lines are not a badge *about* the amounts,
	they *are* the amounts.

	**Nothing here is ticked** (ADR 0024, ADR 0019). A row is derived from a
	Food and however many recipes mention it, and it changes shape the moment a
	Yield moves, so there is no name to staple a tick to. The list leaves as
	text and something else carries it round the shop.

	**On the wide layout the two things the list holds stand side by side**
	(#202, ADR 0044): the recipes chosen in two fifths of a page as wide as a
	recipe's, staying in view, and what to buy in the other three. Each recipe
	there has − and + either side of how much of it is being shopped for, so
	one more is one tap, and the amount between them still opens the two
	boxes. Aurélien chose this on 6 October 2026 over a column 300 wide that
	kept the phone's sentence. The phone's screen is what it was.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { GetShoppingListOutput } from '$lib/api/catalogue';
	import Screen from '$lib/shell/Screen.svelte';
	import Section from '$lib/shell/Section.svelte';
	import Empty from '$lib/shell/Empty.svelte';
	import { rereads } from '$lib/offline/device.svelte';
	import { counts, isMultiplier, said, same, stepped } from '$lib/how-much';
	import { useRoom } from '$lib/room.svelte';

	const kamosu = useKamosu();
	const room = useRoom();

	type List = GetShoppingListOutput;
	type Row = List['rows'][number];
	type Chosen = List['chosen'][number];

	let list = $state<List | undefined>(undefined);
	let failed = $state(false);
	/** The row whose written lines are open. One at a time: this is a shop. */
	let opened = $state<string | null>(null);
	let typing = $state(false);
	let typed = $state('');
	/**
	 * The chosen recipe whose Yield is being changed, and the two boxes for it.
	 * One at a time, like the row that is open: this is a screen read
	 * one-handed.
	 */
	let sizing = $state<string | null>(null);
	let sizeAmount = $state('');
	let sizeNoun = $state('');
	/** The entry being sized is a multiplier, which has no noun (#109). */
	let sizingTimes = $state(false);
	/** The list as text, once it has been copied, and how that went. */
	let sent = $state<'no' | 'yes' | 'failed'>('no');
	let offering = $state(false);

	/**
	 * Read the list again once changes written with no network have reached
	 * the server (#77). The last device to write the list wins, so the
	 * server's list may not be the one this phone was showing.
	 */
	const reread = rereads('get_shopping_list');

	$effect(() => {
		void reread.count;
		let current = true;
		kamosu
			.getShoppingList({})
			.then((answered) => {
				if (current) {
					list = answered;
					failed = false;
				}
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				if (current) failed = true;
			});
		return () => {
			current = false;
		};
	});

	/**
	 * Every one of the list's Operations answers the whole list, so acting on it
	 * is one call and one assignment. There is no local copy to keep in step,
	 * which is the same guarantee the Core has: nothing computed is stored.
	 * It answers whether the Operation did.
	 */
	async function act(work: () => Promise<List>): Promise<boolean> {
		try {
			list = await work();
			failed = false;
			return true;
		} catch (error: unknown) {
			if (!(error instanceof OperationError)) throw error;
			failed = true;
			return false;
		}
	}

	/**
	 * Nothing chosen and nothing typed. The sentence still goes up, and so does
	 * the one thing you can do from here — an empty screen is an invitation to
	 * act, not a dead end, and a Loose Item belongs on an empty list as much as
	 * on a full one.
	 */
	const empty = $derived(list !== undefined && list.chosen.length === 0 && list.rows.length === 0);

	/** A row the arithmetic could not close, which is the one that breaks open. */
	const brokenOpen = (row: Row) => row.parts.length > 1;

	/** How much of a recipe is being shopped for, in words. */
	function shoppingFor(entry: Chosen): string {
		const wanted = entry.shopping_yield ?? entry.written_yield;
		if (!wanted) return '';
		// `said`: a multiplier from the recipe page (#109) is `×2`, not `2 `.
		return m.shopping_shopping_for({ amount: said(wanted) ?? '' });
	}

	function typeItem(event: SubmitEvent) {
		event.preventDefault();
		const text = typed.trim();
		if (!text) return;
		typed = '';
		typing = false;
		void act(() => kamosu.addLooseItem({ text }));
	}

	/**
	 * One serving more or fewer of a chosen recipe, from − and + on the wide
	 * layout (#202). The step is worked out where the recipe page's is, and
	 * one that lands on what the recipe is written for goes back to `null`.
	 */
	function step(entry: Chosen, by: 1 | -1) {
		const next = stepped(entry.written_yield, entry.shopping_yield, by);
		if (next === undefined) return;
		sizing = null;
		// Shown at once, before the Core answers, so that a second tap steps
		// from where the first one landed and two quick taps are two servings.
		// The answer replaces the whole list, this with it.
		const before = entry.shopping_yield;
		entry.shopping_yield = next;
		void act(() =>
			kamosu.setShoppingYield({ branch_id: entry.branch_id, shopping_yield: next }),
		).then((done) => {
			if (!done && same(entry.shopping_yield, next)) entry.shopping_yield = before;
		});
	}

	/**
	 * Whether − and + mean anything for a chosen recipe: its Yield is a whole
	 * number, and what is being shopped for is counted in the same noun or is
	 * a multiplier. Two trays of a recipe written for four servings has no
	 * step, since one more would have to say five servings.
	 */
	function steps(entry: Chosen): boolean {
		const wanted = entry.shopping_yield;
		return (
			counts(entry.written_yield) &&
			(wanted === null || isMultiplier(wanted) || wanted.noun === entry.written_yield?.noun)
		);
	}

	/** Open the Yield boxes on one chosen recipe, filled with what it says now. */
	function sizeUp(entry: Chosen) {
		const wanted = entry.shopping_yield ?? entry.written_yield;
		sizeAmount = wanted?.amount ?? '';
		sizeNoun = wanted?.noun ?? '';
		sizingTimes = isMultiplier(wanted);
		sizing = entry.branch_id;
	}

	/**
	 * Say how much of one recipe is being shopped for. An empty amount is the
	 * recipe **as written** — not zero, and not a guess: `null` is what the
	 * Operation takes for it, and every amount the recipe contributes goes back
	 * to what its own Yield says.
	 */
	function setYield(event: SubmitEvent, branch_id: string) {
		event.preventDefault();
		const amount = sizeAmount.trim();
		const noun = sizeNoun.trim();
		sizing = null;
		// A multiplier set on the recipe page (#109) has no noun, and leaving
		// the noun empty keeps it one rather than quietly putting the recipe
		// back to as written.
		const kept =
			amount && noun ? { amount, noun } : amount && sizingTimes ? { amount, noun: '' } : null;
		void act(() => kamosu.setShoppingYield({ branch_id, shopping_yield: kept }));
	}

	/**
	 * **The list leaves as text and Kamosu lets go of it** (ADR 0024). The text
	 * is rendered by the Core, from the same answer this screen drew, so the
	 * note and the screen cannot disagree about a number.
	 *
	 * Copying does not empty the list. Kamosu offers and does not act: a list
	 * that emptied itself on the way out would be silent and unrecoverable, and
	 * the offer sits there until it is taken or waved off.
	 */
	async function send() {
		try {
			const { text } = await kamosu.shoppingListAsText({});
			await navigator.clipboard.writeText(text);
			sent = 'yes';
			offering = true;
		} catch (error: unknown) {
			// A clipboard a browser would not open is the same failure to a
			// shopper as an Operation that would not answer: the list did not
			// leave. Both say so and neither pretends.
			if (error instanceof OperationError || error instanceof Error) {
				sent = 'failed';
				offering = false;
				return;
			}
			throw error;
		}
	}
</script>

<Screen title={m.shopping_title()} blurb={m.shopping_blurb()} columns>
	{#if failed}
		<p class="text-read text-support" role="alert">{m.shopping_failed()}</p>
	{/if}

	{#if empty && !room.wide}
		<Empty>{m.shopping_empty()}</Empty>
	{/if}

	{#if list}
		<!--
			Two columns on the wide layout and one after the other on the
			phone, by the stylesheet alone. The inner box is the one that
			stays in view: on the column itself, which is the grid's own
			item, it would have nothing to stay within.
		-->
		<div class="shopping-columns">
			<!--
			The choosing, in the order it was made. A recipe that has gone away
			is here too, keeping the name it was known by and saying why it
			contributes nothing — a thing that quietly disappears from a
			shopping list is a thing that does not get bought (ADR 0024).
		-->
			<!--
			On the wide layout the recipes' side always stands, since a column
			that came and went would move the list across the page. With
			nothing chosen it says what the list is for.
		-->
			{#if list.chosen.length > 0 || room.wide}
				<div>
					<div class="shopping-chosen">
						<Section heading={m.shopping_chosen()}>
							{#if empty}
								<Empty>{m.shopping_empty()}</Empty>
							{/if}
							<ul>
								{#each list.chosen as entry (entry.branch_id)}
									{@const countable = room.wide && !entry.gone && steps(entry)}
									<li
										class="flex border-b border-rule py-3 {room.wide
											? 'flex-wrap items-center gap-x-3 gap-y-1'
											: 'items-baseline justify-between gap-3'}"
									>
										<div class="min-w-0 {room.wide ? 'w-full' : ''}">
											{#if entry.gone}
												<span class="block text-line text-ink-2">{entry.title}</span>
											{:else}
												<a class="block text-line" href="/recipes/{entry.branch_id}"
													>{entry.title}</a
												>
											{/if}
										</div>
										<div
											class="flex gap-3 {room.wide
												? 'min-w-0 flex-1 flex-wrap items-center gap-y-1'
												: 'shrink-0 items-baseline'}"
										>
											{#if entry.gone}
												<span class="text-read text-support">{m.shopping_gone()}</span>
											{:else if countable}
												{@const fewer = stepped(entry.written_yield, entry.shopping_yield, -1)}
												{@const more = stepped(entry.written_yield, entry.shopping_yield, 1)}
												<!--
										− and + count in the recipe's own noun,
										as the recipe page's do (#109). Drawn
										only where there is a whole number to
										step from: `1½` and a recipe that says
										nothing keep the sentence below.
									-->
												<span class="flex items-center gap-2">
													<button
														type="button"
														class="step-button"
														disabled={fewer === undefined}
														aria-label={m.shopping_fewer_of({ recipe: entry.title })}
														onclick={() => step(entry, -1)}
													>
														−
													</button>
													<button
														type="button"
														class="min-h-12 min-w-[88px] text-center text-read font-medium whitespace-nowrap text-ink"
														aria-label={m.shopping_yield_label({ recipe: entry.title })}
														onclick={() => sizeUp(entry)}
													>
														{said(entry.shopping_yield ?? entry.written_yield)}
													</button>
													<button
														type="button"
														class="step-button"
														disabled={more === undefined}
														aria-label={m.shopping_more_of({ recipe: entry.title })}
														onclick={() => step(entry, 1)}
													>
														+
													</button>
												</span>
											{:else}
												<!--
										**How much of this recipe is being shopped
										for** (ADR 0024). It reads as a sentence
										and opens as two boxes, because the Yield
										is stored as an amount and its noun and
										Kamosu never guesses at either.
									-->
												<button
													type="button"
													class="text-read text-ink-2 underline decoration-rule underline-offset-4"
													aria-label={m.shopping_yield_label({ recipe: entry.title })}
													onclick={() => sizeUp(entry)}
												>
													{shoppingFor(entry) || m.shopping_yield_set()}
												</button>
											{/if}
											<button
												type="button"
												class="text-label text-accent uppercase {room.wide
													? 'ml-auto whitespace-nowrap'
													: ''}"
												aria-label={m.shopping_take_off_named({ name: entry.title })}
												onclick={() =>
													act(() => kamosu.removeFromShoppingList({ branch_id: entry.branch_id }))}
											>
												{room.wide ? m.shopping_take_off_short() : m.shopping_take_off()}
											</button>
										</div>
										{#if room.wide && entry.shopping_yield && entry.written_yield && !same(entry.shopping_yield, entry.written_yield)}
											<!--
									− and + drift from what the cook wrote one
									tap at a time, so a recipe that has moved
									says where it started.
								-->
											<p class="w-full text-read text-ink-2">
												{m.shopping_written_for({ amount: said(entry.written_yield) ?? '' })}
											</p>
										{/if}
									</li>
									{#if sizing === entry.branch_id}
										<!--
								Two boxes and a way out. Clearing the amount is
								*as written* rather than zero — a recipe has no
								zero, and the Yield the cook wrote is always
								somewhere to go back to.
							-->
										<li class="border-b border-rule bg-ground-2 px-3 py-3">
											<form
												class="flex flex-wrap items-end gap-3"
												onsubmit={(e) => setYield(e, entry.branch_id)}
											>
												<span class="min-w-0 flex-1">
													<label class="block text-label text-ink-2 uppercase" for="yield-amount">
														{m.shopping_yield_amount()}
													</label>
													<input
														id="yield-amount"
														class="mt-1 w-full rounded-sm border border-rule bg-card px-3 py-2 text-line"
														bind:value={sizeAmount}
														placeholder={m.shopping_as_written()}
														autocomplete="off"
														inputmode="decimal"
													/>
												</span>
												<span class="min-w-0 flex-1">
													<label class="block text-label text-ink-2 uppercase" for="yield-noun">
														{m.shopping_yield_noun()}
													</label>
													<input
														id="yield-noun"
														class="mt-1 w-full rounded-sm border border-rule bg-card px-3 py-2 text-line"
														bind:value={sizeNoun}
														autocomplete="off"
													/>
												</span>
												<!-- Under the two boxes on the wide layout, whose column is narrow. -->
												<span class="contents wide:flex wide:w-full wide:gap-3">
													<button type="submit" class="py-2 text-label text-accent uppercase">
														{m.shopping_yield_save()}
													</button>
													<button
														type="button"
														class="py-2 text-label text-ink-2 uppercase"
														onclick={() => (sizing = null)}
													>
														{m.shopping_cancel()}
													</button>
												</span>
											</form>
										</li>
									{/if}
								{/each}
							</ul>
							<!--
					A recipe joins the list from its own page, where you can see
					what you are choosing. There is no picker here, because
					choosing a recipe by its title alone is choosing a Branch
					blind — and a Branch is exactly what this list holds.
				-->
							<a class="mt-3 block text-read text-accent" href="/recipes"
								>{m.shopping_add_recipe()}</a
							>
						</Section>
					</div>
				</div>
			{/if}

			<div>
				<Section heading={m.shopping_to_buy()}>
					<ul>
						{#each list.rows as row (row.id)}
							{@const open = opened === row.id}
							<li class="border-b border-rule">
								<button
									type="button"
									class="flex w-full items-start gap-3 py-3 text-left"
									aria-expanded={open}
									aria-label={m.shopping_lines_for({ name: row.name })}
									onclick={() => (opened = open ? null : row.id)}
								>
									<span class="ingredient-marker shrink-0 bg-accent" aria-hidden="true"></span>
									<span class="min-w-0 flex-1">
										{#if brokenOpen(row)}
											<!--
										The row that could not be added: its Food at
										full size, and beneath it one line per
										recipe that wanted it, behind a hairline.
									-->
											<span class="block text-line">{row.name}</span>
											<span class="mt-2 block border-l border-rule pl-3">
												{#each row.parts as part, index (index)}
													<span class="flex items-baseline justify-between gap-3 py-px">
														<span class="text-label text-ink-2 uppercase">
															{part.sources.join(' · ')}
														</span>
														<span
															class="shrink-0 text-read whitespace-nowrap {part.kind === 'no_amount'
																? 'text-ink-2'
																: ''}"
														>
															{part.text}
														</span>
													</span>
												{/each}
											</span>
										{:else}
											<!--
										The ordinary row: one line, the amount right
										in its own column. A row made of somebody's
										own words has no amount slot at all.
									-->
											<span class="flex items-baseline justify-between gap-3">
												<span class="min-w-0 text-line">{row.name}</span>
												{#if row.parts.length === 1}
													<span
														class="shrink-0 text-read {row.parts[0].kind === 'no_amount'
															? 'text-ink-2'
															: ''}"
													>
														{row.parts[0].text}
													</span>
												{/if}
											</span>
										{/if}
										{#if row.said}
											<!--
										**Why a row that buys nothing buys nothing**
										(ADR 0008): this line stands for a recipe
										Kamosu could not open, and without the
										sentence it reads exactly like a line
										Kamosu could not interpret. Worded in the
										Core, so the recipe page says it the same
										way. Quiet, and never a warning: nobody did
										anything wrong.
									-->
											<span class="mt-1 block text-read text-ink-2">{row.said}</span>
										{/if}
									</span>
								</button>

								{#if open}
									<!--
								**The written lines, one tap away** (ADR 0002,
								ADR 0019). Whole, unrewritten, each under the
								recipe it came from — because where a machine
								judgement might mislead, Kamosu shows the
								originals rather than a badge about them.
							-->
									<div class="mb-3 border-t border-rule bg-ground-2 px-3 py-3">
										<p class="mb-3 text-label text-ink-2 uppercase">{m.shopping_made_from()}</p>
										{#if row.kind === 'loose'}
											<p class="text-read text-ink-2">{m.shopping_typed_by_you()}</p>
											<button
												type="button"
												class="mt-3 text-label text-accent uppercase"
												onclick={() => act(() => kamosu.removeLooseItem({ item_id: row.id }))}
											>
												{m.shopping_take_off()}
											</button>
										{:else}
											<ul class="grid gap-3">
												{#each row.lines as line, index (index)}
													<li>
														<span class="block text-label text-ink-2 uppercase">{line.recipe}</span>
														<span class="block text-line">{line.text}</span>
													</li>
												{/each}
											</ul>
										{/if}
									</div>
								{/if}
							</li>
						{/each}

						<!--
					A **Loose Item** is typed straight onto the list and never
					read: it carries no amount and merges with nothing, so
					typing *flour* beside a recipe that wants flour gives two
					lines (ADR 0024).
				-->
						<li class="py-3">
							{#if typing}
								<form class="flex items-center gap-3" onsubmit={typeItem}>
									<label class="sr-only" for="loose-item">{m.shopping_loose_label()}</label>
									<!-- svelte-ignore a11y_autofocus -->
									<input
										id="loose-item"
										class="min-w-0 flex-1 rounded-sm border border-rule bg-card px-3 py-2 text-line"
										bind:value={typed}
										placeholder={m.shopping_loose_placeholder()}
										autocomplete="off"
										autofocus
									/>
									<button type="submit" class="text-label text-accent uppercase">
										{m.shopping_loose_save()}
									</button>
									<button
										type="button"
										class="text-label text-ink-2 uppercase"
										onclick={() => {
											typing = false;
											typed = '';
										}}
									>
										{m.shopping_cancel()}
									</button>
								</form>
							{:else}
								<button type="button" class="text-read text-accent" onclick={() => (typing = true)}>
									{m.shopping_add_loose()}
								</button>
							{/if}
						</li>
					</ul>

					<!--
				**The way out** (ADR 0024). Nothing is ticked inside Kamosu
				because the list leaves and something else — Apple Notes,
				through a Shortcut — carries it round the shop and holds the
				ticks. A list with no door would make the missing tick a
				refusal rather than a boundary, so this is not a convenience:
				it is the other half of the decision.

				**Kamosu offers to empty and never does it.** The offer appears
				after the copy and waits. Emptying on the way out would be
				silent and unrecoverable, and this is the same instinct that
				makes a Merge Suggestion evidence rather than an instruction.
			-->
					{#if list.rows.length > 0}
						<div class="mt-6 border-t border-rule pt-6">
							<button type="button" class="text-body text-accent" onclick={send}>
								{m.shopping_send()}
							</button>

							{#if sent === 'failed'}
								<p class="mt-3 text-read text-support" role="alert">{m.shopping_send_failed()}</p>
							{:else if sent === 'yes'}
								<p class="mt-3 text-read text-ink-2">{m.shopping_sent()}</p>
							{/if}

							{#if offering}
								<div class="mt-3 flex flex-wrap items-baseline gap-3">
									<span class="text-read">{m.shopping_empty_offer()}</span>
									<button
										type="button"
										class="text-label text-accent uppercase"
										onclick={() => {
											offering = false;
											sent = 'no';
											void act(() => kamosu.emptyShoppingList({}));
										}}
									>
										{m.shopping_empty_yes()}
									</button>
									<button
										type="button"
										class="text-label text-ink-2 uppercase"
										onclick={() => (offering = false)}
									>
										{m.shopping_empty_kept()}
									</button>
								</div>
							{/if}
						</div>
					{/if}
				</Section>
			</div>
		</div>
	{/if}
</Screen>
