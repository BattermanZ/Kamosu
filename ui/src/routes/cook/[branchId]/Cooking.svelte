<!--
	The cooking screen: ONE Step filling the phone, with the amounts for that
	step above it (ADR 0011).

	It is used at arm's length, with wet hands, on a phone propped against
	something — which is the whole of why it is shaped like this:

	  · the Step's text is the LARGEST TYPE IN THE APP (`text-step`, 33px), and
	    it is the only thing on screen that scrolls;
	  · above it, a panel carrying ONLY the Ingredients this step uses. Not the
	    list, not a filter over the list — the two or three lines that matter
	    now. A step that adds nothing says so in words rather than showing an
	    empty box;
	  · the ground turns indigo. Standing at the stove is another room, and it
	    is the one place in Kamosu where that is true;
	  · THERE IS NO TAB BAR HERE. The layout hides it on this route, so no tap
	    near the bottom of a wet phone can land you in Shopping. Leaving is one
	    deliberate control, at the top, and it costs nothing — stopping is just
	    stopping (ADR 0010), the Attempt stays In Progress and Kamosu offers to
	    resume for three days.

	WHICH INGREDIENTS THIS STEP USES IS NOT DECIDED HERE. The Core works it out
	from their Readings and serves it as `version.cooking.steps[i].uses` — a
	list of indices into the recipe's own Ingredient Lines — so an agent at the
	MCP door asked to read out the next step names the same amounts this screen
	is showing (ADR 0001). The same is true of the duration a timer is offered
	for and of the temperature in the other system: both are read out of the
	Step's own text, server-side, at display time, and neither is stored.

	THE AMOUNTS ARE THE WRITTEN LINES, at full size, with the one subordinate
	line beneath them (#49, ADR 0016) — already scaled to the Yield this Attempt
	is cooking to and already converted to this cook's measures, because
	`get_recipe` reads the Attempt's own Yield when it works `measured` out. A
	line Kamosu could not read carries no subordinate line and is shown whole
	rather than guessed at (ADR 0002); nothing marks it as unread, because a
	badge that fires sometimes teaches people it fires always.
-->
<script lang="ts">
	import { onDestroy } from 'svelte';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { GetRecipeOutput, StartAttemptOutput } from '$lib/api/catalogue';
	import { timer, clock } from './timer.svelte';
	import { wakeLock } from './wake-lock.svelte';

	interface Props {
		branchId: string;
	}

	let { branchId }: Props = $props();

	const kamosu = useKamosu();
	const countdown = timer();
	const awake = wakeLock();

	let attempt = $state<StartAttemptOutput | undefined>(undefined);
	let recipe = $state<GetRecipeOutput | undefined>(undefined);
	let failed = $state(false);
	let finished = $state(false);
	let discarding = $state(false);
	let discarded = $state(false);

	$effect(() => {
		let current = true;
		void (async () => {
			try {
				// Starting IS the Attempt (ADR 0010): opening this screen twice
				// hands back the one already In Progress rather than making a
				// second, so there is no "resume" branch to write here.
				const started = await kamosu.startAttempt({ branch_id: branchId });
				if (!current) return;
				attempt = started;
				const read = await kamosu.getRecipe({ branch_id: branchId });
				if (!current) return;
				recipe = read;
			} catch (error) {
				if (!(error instanceof OperationError)) throw error;
				if (current) failed = true;
			}
		})();
		return () => {
			current = false;
		};
	});

	$effect(() => {
		awake.start();
		return () => awake.stop();
	});

	onDestroy(() => countdown.dispose());

	// ---- where the cook is standing --------------------------------------

	/**
	 * The Version this Attempt is pinned to, which is not necessarily the head:
	 * an Attempt started from the Thread cooks an older Version, and a recipe
	 * edited mid-cook must not change under the cook's hands (ADR 0005).
	 */
	const version = $derived(
		recipe?.versions.find((each) => each.version_id === attempt?.version_id) ??
			recipe?.versions.at(-1),
	);
	const content = $derived(version?.content);

	/**
	 * The rows a cook actually stands on, in order, by their index into the
	 * Version's own `steps` — Sections are headings over the list, not places.
	 * The index is what the server stores, so the two never drift.
	 */
	const stops = $derived(
		(content?.steps ?? [])
			.map((row, index) => ({ row, index }))
			.filter((each) => each.row.kind === 'step'),
	);

	/**
	 * Where in `stops` the cook is. `current_step_index` is an index into the
	 * whole list and may land on a Section — the stored 0 of a recipe that opens
	 * with one — so the first stop at or after it is where they are standing.
	 * Read rather than corrected: writing on load would make merely opening the
	 * screen an action, and two devices are meant to agree without either
	 * nudging the other (ADR 0010).
	 */
	const position = $derived.by(() => {
		const at = attempt?.current_step_index ?? 0;
		const found = stops.findIndex((stop) => stop.index >= at);
		return found === -1 ? Math.max(0, stops.length - 1) : found;
	});
	const here = $derived(stops[position]);
	const last = $derived(position >= stops.length - 1);

	/** The Section this step falls under, where the recipe has any. */
	const section = $derived.by(() => {
		if (!here) return null;
		const above = (content?.steps ?? []).slice(0, here.index).reverse();
		return above.find((row) => row.kind === 'section')?.text ?? null;
	});

	/**
	 * What the Core read out of this Step's own text: the Ingredient Lines it
	 * uses, and the duration it offers as a timer. Deliberately not called a
	 * Reading — that word is taken, and means what Kamosu understood of an
	 * INGREDIENT LINE (CONTEXT.md).
	 */
	const outOfText = $derived(here ? (version?.cooking.steps[here.index] ?? null) : null);

	/**
	 * The amounts for this step: the written Ingredient Line at full size and
	 * the one subordinate line beneath it, in the recipe's own order.
	 */
	const amounts = $derived(
		(outOfText?.uses ?? []).flatMap((at) => {
			const line = content?.ingredients[at];
			if (!line || line.kind !== 'ingredient') return [];
			return [{ at, text: line.text, beneath: version?.measured.ingredients[at] ?? null }];
		}),
	);

	const ticked = $derived(new Set(attempt?.ticked_ingredients ?? []));

	// ---- moving, ticking, finishing --------------------------------------

	/**
	 * Every write on this screen is the same one Operation, and every one of
	 * them shows on screen before it is sent: at the stove a tap that waits for
	 * a round trip reads as a tap that did not register, and the cook taps
	 * again. The answer replaces what was laid down optimistically; a refusal
	 * puts the Attempt back exactly as the server has it.
	 */
	async function move(
		optimistic: StartAttemptOutput,
		input: { current_step_index?: number; ticked_ingredients?: number[] },
	) {
		const before = attempt;
		attempt = optimistic;
		try {
			attempt = await kamosu.advanceAttempt({ attempt_id: optimistic.id, ...input });
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			attempt = before;
		}
	}

	function step(to: number) {
		const stop = stops[to];
		if (!attempt || !stop) return;
		void move({ ...attempt, current_step_index: stop.index }, { current_step_index: stop.index });
	}

	function tick(at: number) {
		if (!attempt) return;
		const next = new Set(attempt.ticked_ingredients);
		if (next.has(at)) next.delete(at);
		else next.add(at);
		const ticked_ingredients = [...next].sort((a, b) => a - b);
		void move({ ...attempt, ticked_ingredients }, { ticked_ingredients });
	}

	/**
	 * Finishing stops the Attempt being In Progress and nothing more. The
	 * rating and the note are optional and live in the diary (#59, ADR 0010):
	 * a cook who says nothing still cooked, and being asked to file paperwork
	 * with a hot pan in hand is how a screen makes people lie to it.
	 *
	 * It does not navigate anywhere by itself. A screen that jumps somewhere
	 * else the moment a tap lands is a tap the cook did not intend spent in
	 * reverse — they look up and are somewhere they did not ask to be. Both
	 * ways out are offered instead, and neither is pressed for them.
	 */
	/**
	 * Undoing a false start (ADR 0010). Because starting a cook is what makes
	 * the cooking real, opening these steps out of curiosity would otherwise
	 * register as one — so the counterweight is here, at the stove, where the
	 * false start happened, rather than only in the diary a week later.
	 *
	 * It is behind a confirmation because it is the one control on this screen
	 * that destroys something, and a wet thumb must not be able to spend it.
	 */
	async function discard() {
		if (!attempt) return;
		try {
			await kamosu.deleteAttempt({ attempt_id: attempt.id });
			discarding = false;
			discarded = true;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = true;
		}
	}

	async function finish() {
		if (!attempt) return;
		try {
			await kamosu.finishAttempt({ attempt_id: attempt.id });
			finished = true;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = true;
		}
	}

	/**
	 * What is being cooked, and how much of it — the Attempt's own Yield where
	 * one was set, and otherwise the recipe as written. Built as one string
	 * rather than inline markup so the spacing around the separator cannot
	 * depend on how a formatter happened to break the lines.
	 */
	const heading = $derived.by(() => {
		const title = content?.title ?? '';
		const cooking = attempt?.cooking_yield ?? content?.yield ?? null;
		return cooking ? `${title} · ${cooking.amount} ${cooking.noun}` : title;
	});

	// ---- the timer --------------------------------------------------------

	const duration = $derived(outOfText?.timer_seconds ?? null);
	/**
	 * The duration as a cook reads it before starting it — `45 min`, `1 h 30 min`,
	 * `30 s`. Whole units only: an offer of `7 min 0 s` is noise, and the number
	 * on the button is there to be recognised at arm's length.
	 */
	const offer = $derived.by(() => {
		if (duration === null) return null;
		if (duration < 60) return m.cook_seconds({ seconds: duration });
		const wholeMinutes = Math.round(duration / 60);
		if (wholeMinutes < 60) return m.cook_minutes({ minutes: wholeMinutes });
		const hours = Math.floor(wholeMinutes / 60);
		const minutes = wholeMinutes % 60;
		return minutes === 0 ? m.cook_hours({ hours }) : m.cook_hours_minutes({ hours, minutes });
	});
</script>

<div class="fixed inset-0 z-30 flex flex-col bg-cook-ground px-gutter pb-safe text-cook-ink">
	{#if failed}
		<p class="pt-8 text-body text-cook-ink-2" role="alert">{m.cook_failed()}</p>
	{:else if !content || !attempt}
		<p class="pt-8 text-body text-cook-ink-2">{m.loading()}</p>
	{:else if !here}
		<p class="pt-8 text-body text-cook-ink-2">{m.cook_no_steps()}</p>
	{:else if discarded}
		<div class="flex min-h-0 flex-1 flex-col justify-center gap-4">
			<p class="font-display text-title font-semibold" role="status">{m.cook_thrown_away()}</p>
			<a
				href="/recipes/{branchId}"
				class="min-h-12 rounded-sm bg-cook-accent px-4 py-4 text-center font-semibold text-cook-on-accent"
			>
				{m.cook_back_to_recipe()}
			</a>
		</div>
	{:else if finished}
		<!--
			Cooked. The cooking is already real and was from the moment it
			started (ADR 0010); this only stopped it being In Progress. Both
			ways out, and no rating asked for at the stove.
		-->
		<div class="flex min-h-0 flex-1 flex-col justify-center gap-4">
			<p class="font-display text-title font-semibold" role="status">{m.cook_finished()}</p>
			<a
				href="/cooked"
				class="min-h-12 rounded-sm bg-cook-accent px-4 py-4 text-center font-semibold text-cook-on-accent"
			>
				{m.cook_see_diary()}
			</a>
			<a
				href="/recipes/{branchId}"
				class="min-h-12 rounded-sm border border-cook-rule px-4 py-4 text-center text-body text-cook-ink"
			>
				{m.cook_back_to_recipe()}
			</a>
		</div>
	{:else}
		<!--
			The noren: one hanging strip per step, the one you are on lit. It is
			a progress bar that never needs a number read off it, which is what
			a glance from two steps away buys.
		-->
		<div class="flex shrink-0 gap-1 pt-2 pb-3" aria-hidden="true">
			{#each stops as stop, index (stop.index)}
				<i
					style="height: var(--noren-h)"
					class="flex-1 rounded-sm
						{index < position ? 'bg-cook-accent opacity-45' : ''}
						{index === position ? 'bg-cook-accent' : ''}
						{index > position ? 'bg-cook-rule' : ''}"
				></i>
			{/each}
		</div>

		<header class="flex shrink-0 items-center gap-3 pb-4 text-read text-cook-ink-2">
			<!--
				Two ways to stop, and they mean different things. PAUSE leaves the
				cooking exactly as it is — stopping is just stopping (ADR 0010),
				and Kamosu offers to resume for three days. NOT REALLY COOKING
				undoes a false start, which is a real cooking record being thrown
				away, so it asks first.
			-->
			<a href="/recipes/{branchId}" class="min-h-12 shrink-0 content-center">{m.cook_leave()}</a>
			<span class="flex-1 truncate text-center">{heading}</span>
			<span class="shrink-0 font-display text-body font-semibold text-cook-ink">
				{position + 1}<span class="text-read opacity-55">/{stops.length}</span>
				<span class="sr-only">
					— {m.cook_step_of({ n: position + 1, total: stops.length })}
				</span>
			</span>
		</header>

		{#if section}
			<p class="shrink-0 pb-2 font-display text-body font-semibold text-cook-accent">{section}</p>
		{/if}

		<!--
			The amounts for THIS step. A fifth of the screen, and what buys the
			cook not being sent back to the ingredient list on most steps.
		-->
		<div class="shrink-0 rounded-sm bg-cook-panel px-4 py-3">
			<p class="pb-2 text-label font-semibold text-cook-accent uppercase opacity-90">
				{m.cook_for_this_step()}
			</p>
			{#if amounts.length === 0}
				<p class="text-body opacity-70">{m.cook_nothing_new()}</p>
			{:else}
				<ul class="flex flex-col gap-2">
					{#each amounts as amount (amount.at)}
						<li>
							<button
								type="button"
								class="flex min-h-12 w-full items-baseline gap-3 text-left"
								aria-pressed={ticked.has(amount.at)}
								onclick={() => tick(amount.at)}
							>
								<span
									class="mt-1 h-4 w-4 shrink-0 self-start rounded-sm border
										{ticked.has(amount.at) ? 'border-cook-accent bg-cook-accent' : 'border-cook-rule'}"
									aria-hidden="true"
								></span>
								<span class="min-w-0 flex-1 {ticked.has(amount.at) ? 'opacity-45' : ''}">
									<span class="block font-display text-panel-figure font-semibold">
										{amount.text}
									</span>
									{#if amount.beneath}
										<span class="block text-read text-cook-ink-2">{amount.beneath}</span>
									{/if}
								</span>
								<span class="sr-only">{m.cook_tick({ line: amount.text })}</span>
							</button>
						</li>
					{/each}
				</ul>
			{/if}
		</div>

		<!--
			The Step. The largest type in the app, and the only thing here that
			scrolls — everything else is fixed, so the cook's eye lands in the
			same place on every step.
		-->
		<!--
			The Step, and the two things that belong directly under it. The text
			is NOT given the free space — it takes the height it needs and
			scrolls only once there is no more, so the temperature and the timer
			stay against the sentence they belong to rather than being pushed to
			the foot of the screen. A Step's conversion is an addition BESIDE it
			(CONTEXT.md), and half a screen away is not beside.
		-->
		<div class="flex min-h-0 flex-1 flex-col items-start pt-6">
			<p class="min-h-0 overflow-y-auto font-display text-step font-semibold">
				{here.row.text}
			</p>
			<!--
				A Step's own subordinate line: the oven temperature in this
				cook's measures, on the conventional ladder. Beside the
				sentence, never written into it.
			-->
			{#if version?.measured.steps[here.index]}
				<p class="shrink-0 pt-3 text-read text-cook-ink-2">
					{version.measured.steps[here.index]}
				</p>
			{/if}

			<!--
				The timer. One tap, nothing typed, nothing stored — and it is the
				cook's rather than the step's, so it follows them forward. While
				one runs it stands where the offer was: two timers at once is a
				thing to keep track of, and this screen exists to stop the cook
				keeping track of things.
			-->
			{#if countdown.remaining !== null}
				<button
					type="button"
					class="mt-4 min-h-12 shrink-0 rounded-sm border-2 px-4 font-semibold
						{countdown.rung
						? 'border-cook-accent bg-cook-accent text-cook-on-accent'
						: 'border-cook-accent text-cook-accent'}"
					onclick={() => countdown.clear()}
				>
					{countdown.rung
						? m.cook_timer_done()
						: m.cook_timer_running({ clock: clock(countdown.remaining) })}
					<span class="sr-only">— {m.cook_timer_stop()}</span>
				</button>
			{:else if duration !== null && offer}
				<button
					type="button"
					class="mt-4 min-h-12 shrink-0 rounded-sm border-2 border-cook-accent px-4 font-semibold text-cook-accent"
					onclick={() => countdown.start(duration)}
				>
					{m.cook_timer_start({ duration: offer })}
				</button>
			{/if}
		</div>

		<div class="flex shrink-0 gap-3 pt-4">
			<button
				type="button"
				class="w-28 min-h-12 shrink-0 rounded-sm border border-cook-rule py-4 text-body text-cook-ink disabled:opacity-40"
				disabled={position === 0}
				onclick={() => step(position - 1)}
			>
				{m.cook_back()}
			</button>
			{#if last}
				<button
					type="button"
					class="min-h-12 flex-1 rounded-sm bg-cook-accent py-4 font-semibold text-cook-on-accent"
					onclick={finish}
				>
					{m.cook_finish()}
				</button>
			{:else}
				<button
					type="button"
					class="min-h-12 flex-1 rounded-sm bg-cook-accent py-4 font-semibold text-cook-on-accent"
					onclick={() => step(position + 1)}
				>
					{m.cook_next()}
				</button>
			{/if}
		</div>

		<!--
			The Wake Lock, and its off switch. The real API and nothing else —
			a browser without it says the screen may sleep, which is true, rather
			than a hidden video pretending otherwise.
		-->
		<div class="flex shrink-0 items-center justify-between gap-3">
			<button
				type="button"
				class="min-h-12 text-label text-cook-ink-2 uppercase"
				onclick={() => (discarding = true)}
			>
				{m.cook_false_start()}
			</button>
			<button
				type="button"
				class="min-h-12 text-label text-cook-ink-2 uppercase disabled:opacity-60"
				disabled={!awake.available}
				aria-pressed={awake.wanted}
				onclick={() => awake.toggle()}
			>
				{awake.held ? m.cook_awake_on() : m.cook_awake_off()}
			</button>
		</div>

		{#if discarding}
			<div
				class="py-5 fixed inset-x-0 bottom-0 z-40 mx-auto max-w-2xl bg-cook-panel px-gutter pb-safe"
				role="dialog"
				aria-modal="true"
				aria-label={m.cook_false_start()}
			>
				<p class="text-body text-cook-ink">{m.cook_false_start_confirm()}</p>
				<button
					type="button"
					class="mt-4 min-h-12 w-full rounded-sm bg-cook-accent py-4 font-semibold text-cook-on-accent"
					onclick={discard}
				>
					{m.cook_false_start_yes()}
				</button>
				<button
					type="button"
					class="mt-2 min-h-12 w-full rounded-sm border border-cook-rule py-4 text-body text-cook-ink"
					onclick={() => (discarding = false)}
				>
					{m.cook_false_start_no()}
				</button>
			</div>
		{/if}
	{/if}
</div>
