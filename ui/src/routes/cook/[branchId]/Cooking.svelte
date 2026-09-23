<!--
	The cooking screen: ONE Step filling the phone, with the amounts for that
	step above it (ADR 0011).

	It is used at arm's length, with wet hands, on a phone propped against
	something — which is the whole of why it is shaped like this:

	  · the Step's text is the LARGEST TYPE IN THE APP (`text-step`, 26px), and
	    it is the only thing on screen that scrolls;
	  · above it, ONLY the Ingredients this step uses. Not the list, not a
	    filter over the list — the two or three lines that matter now, on the
	    room's own ground with a hairline under them. A step that adds nothing
	    says so in words rather than showing an empty box;
	  · the ground turns indigo. Standing at the stove is another room, and it
	    is the one place in Kamosu where that is true;
	  · THERE IS NO TAB BAR HERE. The layout hides it on this route, so no tap
	    near the bottom of a wet phone can land you in Shopping. The foot of
	    the screen is Back and Next and NOTHING ELSE (#88), so the only thing a
	    wet thumb can reach there is a way through the recipe.

	HOW MUCH ROOM ANY OF IT GETS WAS SETTLED BY MEASUREMENT, NOT BY EYE (#88).
	The screen is used in Safari with its URL bar on show, which leaves about
	700px on the phone this was fitted to — not the 844px a headless browser
	reports. Everything about this room's spacing was re-fitted against the real
	579 Steps of the Crouton export at that height: 53% of Steps naming two
	amounts used to fit without scrolling, and 92% do now. Before moving a
	number here, measure it at 700px against real Steps.

	EVERY CONTROL A WET THUMB MUST HIT IS 48px OF TOUCH. Some of them are only
	32px of ink — `tap-out` grows the hit box with a pseudo-element instead of
	with height, because a row this screen does not need is a line of the Step.

	WHICH INGREDIENTS THIS STEP USES IS NOT DECIDED HERE. The Core works it out
	from their Readings and serves it as `version.cooking.steps[i].uses` — a
	list of indices into the recipe's own Ingredient Lines — so an agent at the
	MCP door asked to read out the next step names the same amounts this screen
	is showing (ADR 0001). The same is true of the duration a timer is offered
	for and of the temperature in the other system: both are read out of the
	Step's own text, server-side, at display time, and neither is stored.

	THE AMOUNTS ARE THE WRITTEN LINES, at full size, with the one subordinate
	line beneath them (#49, ADR 0016) — already converted to this cook's
	measures, because `get_recipe` works `measured` out for this reader. A line
	Kamosu could not read carries no subordinate line and is shown whole rather
	than guessed at (ADR 0002); at the Yield as written nothing marks it as
	unread, because a badge that fires sometimes teaches people it fires always.

	HOW MUCH IS BEING COOKED IS ASKED BEFORE A FRESH COOKING STARTS (#109, option
	B, Aurélien's choice of 23 September 2026). The question stands where the
	amounts and the Step will, and the foot's own right-hand button starts the
	cooking — there is no second button for it. Afterwards the Yield on the row
	above the hairline brings the question back, and the foot then reads *Back
	to step N*. Once the amounts are scaled THE SCALED AMOUNT LEADS: the Core's
	subordinate line at full size, the recipe's own line small beneath it. At
	the stove the number to measure is the one that matters, and the written
	line is still there to say what it is a number of. A line that could not be
	scaled keeps its own words and says it was not scaled — once a cook has
	asked for twice the recipe, a line left alone is exactly what they need
	telling. Kamosu scaled nothing here: every figure came from the Core.
-->
<script lang="ts">
	import { onDestroy, untrack } from 'svelte';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { GetRecipeOutput, StartAttemptOutput } from '$lib/api/catalogue';
	import { timer, clock } from './timer.svelte';
	import { wakeLock } from './wake-lock.svelte';
	import {
		seed,
		serialise,
		differs,
		keepDraft,
		readDraft,
		forgetDraft,
		type AsCooked,
		type Line,
	} from './as-cooked.svelte';
	import { useKeeping } from '$lib/offline/outbox';
	import { photographCooking, pickedFile } from '$lib/offline/photograph';
	import { rereads } from '$lib/offline/device.svelte';
	import { page } from '$app/state';
	import { replaceState } from '$app/navigation';
	import HowMuch from '$lib/HowMuch.svelte';
	import StepPhoto from '$lib/StepPhoto.svelte';
	import { fresh } from './fresh';
	import { caughtUp, fromSearch, said, same, type Wanted } from '$lib/how-much';

	interface Props {
		branchId: string;
	}

	let { branchId }: Props = $props();

	const kamosu = useKamosu();
	const keeping = useKeeping();
	const countdown = timer();
	const awake = wakeLock();

	let attempt = $state<StartAttemptOutput | undefined>(undefined);
	let recipe = $state<GetRecipeOutput | undefined>(undefined);
	let failed = $state(false);
	let finished = $state(false);

	// ---- saying you did it differently (#58, ADR 0005) --------------------
	//
	// The recipe as THIS cooking has it, which is the recipe itself until the
	// cook writes on it. It is held here rather than sent on every keystroke:
	// a whole recipe crossing the wire per character typed at a stove would be
	// absurd, and `Done` is a moment the cook already understands. What is sent
	// is the whole state, and the Core decides by fingerprint whether it is a
	// deviation at all.
	let asCooked = $state<AsCooked | undefined>(undefined);
	/** The step in front of the cook has become fields, in place. */
	let writing = $state(false);
	/** The rest of the Ingredient Lines are open beneath this step's own. */
	let wholeList = $state(false);
	/** Written since the last save. */
	let unsaved = $state(false);
	let writeFailed = $state(false);
	let discarding = $state(false);
	let discarded = $state(false);
	/** A photograph that could not be kept, even on the phone. */
	let photoFailed = $state(false);

	/** How much is being cooked is being asked (#109). */
	let asking = $state(false);
	/**
	 * It was asked as the cooking opened, so the foot starts the cooking rather
	 * than going back to a step.
	 */
	let atStart = $state(false);
	/** The last change of how much did not reach even the phone's own keeping. */
	let howMuchFailed = $state(false);

	/**
	 * The Step's own element, and whether its text is taller than the box it was
	 * given. Read from the DOM rather than predicted, because how many lines a
	 * sentence takes depends on the font, the width and the phone (#88).
	 */
	let stepEl = $state<HTMLElement | undefined>(undefined);
	let stepScrolls = $state(false);

	/**
	 * The false-start dialog, so that opening it moves the focus into it.
	 *
	 * It has to: #88 moved the control that opens it up into the header, while
	 * the dialog itself is still the last thing in the document. Without this a
	 * keyboard or a screen reader would leave the trigger, walk the amounts, the
	 * Step, Back and Next, and only then arrive at a confirmation it had already
	 * asked for.
	 */
	let discardDialog = $state<HTMLElement | undefined>(undefined);
	$effect(() => {
		if (discarding) discardDialog?.focus();
	});

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
				// A cooking nothing has happened to yet opens on the question
				// (#109) — already set to what the recipe page was scaled to,
				// where the cook came from one that was (the errands).
				const carried = untrack(() => fromSearch(page.url.searchParams));
				if (fresh(started)) {
					asking = true;
					atStart = true;
					if (carried !== undefined && !same(carried, started.cooking_yield)) await choose(carried);
				} else if (carried !== undefined && !same(carried, started.cooking_yield)) {
					// Sent from a recipe page scaled to another amount, into a
					// cooking already under way at its own. The cooking's amount
					// is a fact about this afternoon and is not overwritten from
					// another screen — but the cook is shown the question rather
					// than left to find the difference in the amounts.
					asking = true;
				}
				// Read once and let go of: left in the address, a reload later in
				// the cooking would offer the recipe page's amount again over the
				// one the cook has since chosen (found in live acceptance, #109).
				// Shallow: a navigation here would run this start a second time,
				// racing the amount just sent.
				if (carried !== undefined) replaceState(`/cook/${branchId}`, page.state);
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

	/**
	 * Whether the Step overflows the box it was given.
	 *
	 * The text is read as a dependency and not just the element: Svelte reuses
	 * the same paragraph from one step to the next, so an effect watching only
	 * the element would measure the first Step and then never look again. What
	 * changes the answer is the sentence, whether the screen is in writing mode,
	 * and how tall the amounts above it are — hence all three are read — plus
	 * turning the phone, which `resize` covers.
	 *
	 * Deliberately NOT a ResizeObserver. The screen tests run in jsdom, whose
	 * setup stubs nothing a screen could come to rely on by accident, and a fade
	 * is an enhancement: it must not be the reason this component cannot mount.
	 */
	$effect(() => {
		const element = stepEl;
		// read, so the effect re-runs when any of them moves
		void here?.row.text;
		void writing;
		void amounts.length;
		void writeFailed;
		if (!element) {
			stepScrolls = false;
			return;
		}
		const measure = () => {
			// A few pixels of slack: sub-pixel line heights make a Step that fits
			// report a pixel of overflow, and a fade over a Step that fits is
			// exactly the greyed-out last line this exists to avoid.
			stepScrolls = element.scrollHeight > element.clientHeight + 8;
		};
		measure();
		window.addEventListener('resize', measure);
		return () => window.removeEventListener('resize', measure);
	});

	onDestroy(() => countdown.dispose());

	/**
	 * The recipe again, whenever what the phone showed has been overtaken: the
	 * service worker's refresh behind a kept answer, or the outbox having sent
	 * a change of how much it held while the network was gone (#77). The
	 * scaled amounts are the Core's, so they arrive by reading, never by being
	 * worked out here.
	 */
	const reread = rereads('get_recipe');
	$effect(() => {
		if (reread.count === 0) return;
		let current = true;
		void kamosu
			.getRecipe({ branch_id: branchId })
			.then((read) => {
				if (current) recipe = read;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
			});
		return () => {
			current = false;
		};
	});

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
	 * The recipe this cooking is actually walking through — the Version, until
	 * the cook writes on it, and their own words from then on. Seeded once the
	 * Attempt and its Version are both in hand, and never re-seeded underneath
	 * somebody who is typing.
	 */
	$effect(() => {
		if (asCooked || !content || !attempt) return;
		// Words written with no network are the cook's until the server has
		// them (#77): the phone keeps them here, because what the server makes
		// of them — which line changed, which was added — is the server's to
		// read (ADR 0019), and it has not read them yet.
		const waiting = keeping.holds(attempt.id);
		if (!waiting) forgetDraft(attempt.id);
		asCooked = (waiting ? readDraft(attempt.id) : undefined) ?? seed(content, attempt.as_cooked);
	});

	/**
	 * The rows a cook actually stands on, in order — Sections are headings over
	 * the list, not places, and a line the cook dropped is not a place either.
	 *
	 * `at` indexes the AS COOKED's steps rather than the Version's, and that is what
	 * `current_step_index` means from #58 onwards: the cooking screen is the As
	 * Cooked once there is one, so a step the cook inserted is a step they stand
	 * on, time and all. Where the cook has written nothing this is the Version line for
	 * line, so nothing about an ordinary cooking moved.
	 */
	const stops = $derived(
		(asCooked?.steps ?? [])
			.map((row, index) => ({ row, index }))
			.filter((each) => each.row.kind === 'step' && !each.row.dropped),
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
		const above = (asCooked?.steps ?? []).slice(0, here.index).reverse();
		return above.find((row) => row.kind === 'section' && !row.dropped)?.text ?? null;
	});

	/**
	 * What the Core read out of this Step's own text: the Ingredient Lines it
	 * uses, and the duration it offers as a timer. Deliberately not called a
	 * Reading — that word is taken, and means what Kamosu understood of an
	 * INGREDIENT LINE (CONTEXT.md).
	 */
	/**
	 * Looked up by where the step CAME FROM in the Version, never by where it
	 * sits now — inserting a step shifts every position after it, and `uses` and
	 * `measured` are the Core's arrays over the Version. A step the cook wrote
	 * has no such reading, and says nothing rather than borrowing its
	 * neighbour's: Kamosu has not read that sentence.
	 */
	const outOfText = $derived(
		here?.row.from === null || here === undefined
			? null
			: (version?.cooking.steps[here.row.from] ?? null),
	);
	/** The recipe's photograph of this Step, if it has one (#110). */
	const stepPhoto = $derived(
		here?.row.from === null || here === undefined
			? null
			: (content?.steps[here.row.from]?.photo ?? null),
	);
	/** The subordinate line under the Step — the oven temperature in this cook's measures. */
	const beneathStep = $derived(
		here?.row.from === null || here === undefined
			? null
			: (version?.measured.steps[here.row.from] ?? null),
	);

	/**
	 * The amounts for this step: the written Ingredient Line at full size and
	 * the one subordinate line beneath it, in the recipe's own order — as this
	 * cooking has them, which is as the recipe has them until somebody writes.
	 *
	 * A line whose words the cook changed loses its subordinate line, because
	 * that reading describes the amount the recipe asked for and not the one
	 * that went in. A line they dropped stays on screen, struck: it was in front
	 * of them a moment ago and vanishing furniture is how a screen loses a cook.
	 */
	const amounts = $derived(
		(outOfText?.uses ?? []).flatMap((from) => {
			const at = (asCooked?.ingredients ?? []).findIndex((row) => row.from === from);
			const row = asCooked?.ingredients[at];
			if (!row || row.kind !== 'ingredient') return [];
			const written = content?.ingredients[from]?.text ?? row.text;
			const changed = row.text.trim() !== written.trim();
			return [
				{
					at,
					row,
					written,
					changed,
					beneath: changed || row.dropped ? null : (version?.measured.ingredients[from] ?? null),
				},
			];
		}),
	);

	/**
	 * Whether this cooking's words for a step are not the recipe's — rewritten,
	 * or written from nothing. What the noren stripes, and what puts the bar
	 * down the side of the Step.
	 */
	/**
	 * Whether a line is ticked. Ticking is keyed on the line's index in the
	 * Version, which is what the server stores and what two devices agree on, so
	 * a line the cook added has no index and is never ticked.
	 */
	function isTicked(line: Line): boolean {
		return line.from !== null && ticked.has(line.from);
	}

	function wroteStep(line: Line): boolean {
		if (line.dropped) return false;
		if (line.from === null) return true;
		return line.text.trim() !== (content?.steps[line.from]?.text ?? '').trim();
	}

	/** Every other Ingredient Line, for the steps — most of them — that name none. */
	const otherLines = $derived(
		(asCooked?.ingredients ?? []).flatMap((row, at) => {
			if (row.kind !== 'ingredient') return [];
			if (amounts.some((amount) => amount.at === at)) return [];
			const written = row.from === null ? null : (content?.ingredients[row.from]?.text ?? null);
			return [
				{ at, row, written, changed: written !== null && row.text.trim() !== written.trim() },
			];
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
		writing = false;
		wholeList = false;
		void save();
		void move({ ...attempt, current_step_index: stop.index }, { current_step_index: stop.index });
	}

	// ---- writing what you did --------------------------------------------
	//
	// Every one of these changes what is held here and nothing else. Nothing reaches the
	// server until `save`, which runs when the cook leaves writing mode or the
	// step — the two moments they have already decided they are done.

	/** Rewrite one line, in either list. */
	function write(list: 'ingredients' | 'steps', at: number, text: string) {
		const row = asCooked?.[list][at];
		if (!row) return;
		row.text = text;
		unsaved = true;
	}

	/**
	 * Take a line out, or put it back. It stays here either way, marked
	 * — so the tap that dropped it is the tap that undoes it, and nothing
	 * disappears from under a wet finger. `serialise` is what leaves it out.
	 */
	function drop(at: number) {
		const row = asCooked?.ingredients[at];
		if (!row) return;
		row.dropped = !row.dropped;
		unsaved = true;
	}

	/** A line the cook added: theirs, from nowhere in the recipe. */
	function addLine() {
		if (!asCooked) return;
		asCooked.ingredients.push({ kind: 'ingredient', text: '', from: null, dropped: false });
		unsaved = true;
	}

	/**
	 * Insert a step before the one the cook is standing on, and leave them
	 * standing on the new one — they are writing down what they have just done,
	 * so that is where they want to be. Inserting before shifts every later step
	 * up by one, and `current_step_index` therefore names the new step without
	 * anything being written: the stored number did not move, the list did.
	 */
	function insertStep() {
		if (!asCooked || !here) return;
		asCooked.steps.splice(here.index, 0, { kind: 'step', text: '', from: null, dropped: false });
		writing = true;
		unsaved = true;
	}

	/**
	 * Send the whole recipe as this cooking has it — or nothing at all, where
	 * the cook has ended up back where they started. The Core decides for real
	 * by fingerprint; this only saves a round trip on the common case.
	 */
	async function save() {
		if (!attempt || !asCooked || !content || !unsaved) return;
		const as_cooked = differs(content, asCooked) ? serialise(content, asCooked) : null;
		try {
			const answer = await kamosu.setAsCooked({ attempt_id: attempt.id, as_cooked });
			// ONLY the As Cooked is taken from the answer. It carries a whole
			// Attempt, including the `current_step_index` as it stood when the
			// request left — and this runs alongside `advance_attempt`, so
			// replacing the Attempt wholesale would throw the cook back a step
			// whenever this one answered second.
			if (attempt) attempt = { ...attempt, as_cooked: answer.as_cooked };
			if (attempt && keeping.holds(attempt.id)) keepDraft(attempt.id, asCooked);
			else if (attempt) forgetDraft(attempt.id);
			unsaved = false;
			writeFailed = false;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			// What they wrote stays on screen. Losing a cook's own words because
			// a network blinked is the one failure this screen must not have.
			writeFailed = true;
		}
	}

	/**
	 * Ticking is keyed on the line's index in the VERSION, which is what the
	 * server stores and what two devices agree on. A line the cook added has no
	 * such index and cannot be ticked — it is a note to themselves about what
	 * went in, not a thing to check off.
	 */
	function tick(at: number | null) {
		if (!attempt || at === null) return;
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
			forgetDraft(attempt.id);
			discarding = false;
			discarded = true;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = true;
		}
	}

	/**
	 * A picture of the cooking, taken at the stove (#77, option A). It is kept
	 * on the phone before anything else, so it is never lost to a network that
	 * blinked, and put on the cooking beside the others. Only the pictures are
	 * taken from the answer: the rest of the Attempt it carries left before the
	 * step the cook may have moved to since.
	 */
	async function takePhoto(event: Event) {
		const picture = pickedFile(event);
		if (!attempt || !picture) return;
		const id = attempt.id;
		try {
			const photographs = await photographCooking(kamosu, keeping, id, picture);
			if (attempt?.id === id) attempt = { ...attempt, photographs };
			photoFailed = false;
		} catch (error) {
			if (!(error instanceof OperationError) && !(error instanceof DOMException)) throw error;
			photoFailed = true;
		}
	}

	async function finish() {
		if (!attempt) return;
		writing = false;
		await save();
		try {
			await kamosu.finishAttempt({ attempt_id: attempt.id });
			finished = true;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = true;
		}
	}

	/**
	 * HOW MUCH is being cooked — the Attempt's own Yield or multiplier where one
	 * was set, and otherwise the recipe as written: its Yield, or *As written*
	 * for the third of the library that never said what it makes (#109).
	 *
	 * This used to carry the recipe's title in front of it. #88 took the title
	 * off this screen: at 430px it truncated to `Katsu Curry (Japanese Curry
	 * with Chicken C…`, which told a cook nothing they did not already know, and
	 * the row it freed is what let the false start and the screen's sleep leave
	 * the bottom edge. The Yield stayed, because it is short and because it is
	 * the number every amount on this screen was scaled to — dropping it would
	 * leave `800 ml` on screen with nothing saying 800 ml of what quantity of
	 * dish. Built as one string so the spacing cannot depend on how a formatter
	 * happened to break the lines.
	 */
	const written = $derived(content?.yield ?? null);
	const wanted = $derived(attempt?.cooking_yield ?? null);
	const howMuch = $derived(said(wanted) ?? said(written) ?? m.how_much_as_written());

	/**
	 * Whether the amounts on screen were scaled to what is being cooked. They
	 * are not while a change made with no network waits to be sent (#77): the
	 * phone holds the choice, and only the Core can scale.
	 */
	// `?? null`: a recipe the phone kept before #109 has no `scaled_to` at all,
	// and was as written.
	const current = $derived(version ? caughtUp(version.scaled_to ?? null, wanted, written) : true);
	/** The amounts on screen are not the recipe's as written. */
	const scaled = $derived(current && (version?.scaled_to ?? null) !== null);

	/**
	 * Say how much is being cooked. It is shown at once and sent like every
	 * other move on this screen; the amounts follow when the recipe is read
	 * again, scaled by the Core. It stores nothing on the recipe and mints no
	 * Version (ADR 0002): it is a fact about this afternoon.
	 */
	async function choose(chosen: Wanted) {
		if (!attempt) return;
		const before = attempt;
		attempt = { ...attempt, cooking_yield: chosen };
		try {
			const answer = await kamosu.advanceAttempt({ attempt_id: before.id, cooking_yield: chosen });
			// Only the Yield: the rest of the answer left before any step the
			// cook may have taken since (`save` has the same rule).
			if (attempt) attempt = { ...attempt, cooking_yield: answer.cooking_yield };
			howMuchFailed = false;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			if (attempt) attempt = { ...attempt, cooking_yield: before.cooking_yield };
			howMuchFailed = true;
			return;
		}
		try {
			recipe = await kamosu.getRecipe({ branch_id: branchId });
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
		}
	}

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

<div
	class="fixed inset-0 z-30 flex flex-col bg-cook-ground px-gutter pt-safe pb-safe text-cook-ink"
>
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
				<!--
					A strip is STRIPED where this cooking's words are not the
					recipe's — a step rewritten, or one the cook inserted. It costs
					the noren no height and no number to say so, which is the whole
					of why the noren is shaped the way it is.
				-->
				<i
					style="height: var(--noren-h){wroteStep(stop.row)
						? '; background-image: repeating-linear-gradient(135deg, var(--color-cook-accent) 0 3px, transparent 3px 6px)'
						: ''}"
					class="flex-1 rounded-sm
						{index < position ? 'bg-cook-accent opacity-45' : ''}
						{index === position ? 'bg-cook-accent' : ''}
						{index > position ? 'bg-cook-rule' : ''}"
				></i>
			{/each}
		</div>

		<!--
			EVERYTHING ABOUT THE SESSION IS ON THIS ONE ROW (#88), and the recipe's
			name is not: at 430px it was truncated to `Katsu Curry (Japanese Curry
			with Chicken C…`, which told a cook nothing they did not already know.
			Giving up those characters is what lets the false start and the screen's
			sleep come up here — and that removes a whole 48px row from the foot of
			the screen, where a wet thumb was landing on them.

			THREE WAYS TO STOP, and they mean different things. PAUSE leaves the
			cooking exactly as it is — stopping is just stopping (ADR 0010), and
			Kamosu offers to resume for three days; it is a filled block because it
			is how you leave, and every other control that does something is one.
			NOT REALLY COOKING undoes a false start, which is a real cooking record
			being thrown away, so it asks first. FINISH COOKING is at the foot, with
			the step it ends on.

			It WRAPS rather than truncating. In English and French the row fits; in
			Spanish `No estoy cocinando` and `La pantalla puede apagarse` are 44
			characters between them and take a second line. A screen that silently
			clipped `La pantalla puede apagar…` would be worse than one that is a row
			taller in one language.
		-->
		<header
			class="flex shrink-0 flex-wrap items-center justify-between gap-x-3 gap-y-1 pb-3 text-read text-cook-ink-2"
		>
			<a
				href="/recipes/{branchId}"
				class="min-h-12 shrink-0 content-center rounded-sm bg-cook-panel px-4 text-body text-cook-ink"
			>
				{m.cook_leave()}
			</a>
			<!--
				WHERE YOU ARE, second on the row and not last. This header wraps
				(see above), and with the count last a wrap left it alone on a line
				of its own with everything else above it — which looked like a
				fault. Second, it shares the first line with Pause whatever
				happens, and the two quiet controls wrap together beneath them.
			-->
			<span class="shrink-0 font-display text-body font-semibold text-cook-ink">
				{position + 1}<span class="text-read opacity-55">/{stops.length}</span>
				<span class="sr-only">
					— {m.cook_step_of({ n: position + 1, total: stops.length })}
				</span>
			</span>
			<!--
				The Wake Lock, and its off switch. The real API and nothing else — a
				browser without it says the screen may sleep, which is true, rather
				than a hidden video pretending otherwise.

				It stands BETWEEN Pause and the false start deliberately. All three
				are 48px targets on one row and two of them are ways to stop, but
				only one throws a cooking away — so the harmless one sits between
				them, and a thumb that misses Pause lands on a toggle rather than
				on a confirmation it did not mean to open.
			-->
			<button
				type="button"
				class="min-h-12 shrink-0 disabled:opacity-60"
				disabled={!awake.available}
				aria-pressed={awake.wanted}
				onclick={() => awake.toggle()}
			>
				{awake.held ? m.cook_awake_on() : m.cook_awake_off()}
			</button>
			<button type="button" class="min-h-12 shrink-0" onclick={() => (discarding = true)}>
				{m.cook_false_start()}
			</button>
		</header>

		{#if section}
			<p class="shrink-0 pb-2 font-display text-body font-semibold text-cook-accent">{section}</p>
		{/if}

		<!--
			THE AMOUNTS FOR THIS STEP, and what buys the cook not being sent back
			to the ingredient list on most steps.

			THEY ARE NOT A BOX ANY MORE (#88). The fill, the border and the
			`FOR THIS STEP` label row went together: 56px of a 700px screen spent
			saying what the position above the Step already says. What is left is
			the lines themselves, on the room's own ground, with a hairline under
			them — and the Step, which this room exists for, got the height back.

			AND THEY NO LONGER WIN THE SQUEEZE. They used to be `shrink-0` against
			a `flex-1` Step, so a Step naming many Ingredients pushed the Step,
			the timer and both buttons clean off the bottom of the phone: on
			`Best Steak Marinade In Existence` — ten lines on one Step, from the
			real corpus — the screen showed the amounts and NOTHING else, with no
			way to read the instruction and no way to reach the next step. They
			are capped and scroll inside that cap ALWAYS now, which is the same
			mechanism #58 used only while the whole list was open — and at ONE
			number, not two. A roomier cap for the whole-list case was tried and
			taken out: at 60% the Step's own field was squeezed to a line too
			short to type in, which is the bug at the other end of this one.
			Twenty-one fields do not need more room, they need to scroll.
		-->
		{#if asking}
			<!--
				HOW MUCH ARE YOU MAKING (#109), in the place the amounts and the
				Step will stand. The foot's right-hand button is what leaves it —
				Start cooking as the cooking opens, Back to step N afterwards —
				so there is no button in here to confuse with it.
			-->
			<div
				class="flex min-h-0 flex-1 flex-col justify-center overflow-y-auto border-t border-cook-rule pt-3"
			>
				<p class="font-display text-title font-semibold">
					{atStart ? m.cook_before_you_start() : m.cook_change_how_much()}
				</p>
				{#if written}
					<p class="pt-1 pb-4 text-read text-cook-ink-2">
						{m.how_much_makes({ made: said(written) ?? '' })}
					</p>
				{/if}
				<HowMuch {written} {wanted} room="cook" onchoose={(chosen) => void choose(chosen)} />
				<p class="pt-3 text-read text-cook-ink-2">{m.how_much_amounts_follow()}</p>
				{#if howMuchFailed}
					<p class="pt-2 text-read text-cook-ink" role="alert">{m.how_much_failed()}</p>
				{/if}
			</div>
		{:else}
			<div class="max-h-[45%] min-h-0 shrink overflow-y-auto">
				<!--
				A save that did not land. What the cook wrote is still on screen and
				still held here, and the next `Done` sends it again — losing
				somebody's own words because a network blinked is the one failure
				this screen must not have.
			-->
				{#if writeFailed}
					<p class="pb-2 text-read text-cook-ink-2" role="alert">{m.cook_write_failed()}</p>
				{/if}
				{#if photoFailed}
					<p class="pb-2 text-read text-cook-ink-2" role="alert">{m.cook_photo_failed()}</p>
				{/if}
				<!--
				A change of how much made with no network (#77): the phone holds
				it and the Core has not scaled it yet, so the amounts below are the
				ones the phone had, and say nothing beneath them rather than a
				figure for the wrong amount.
			-->
				{#if !current}
					<p class="pb-2 text-read text-cook-ink-2" role="status">
						{m.cook_scaling_waits({ amount: howMuch })}
					</p>
				{/if}
				{#if writing}
					<!--
					The amounts, as fields, in the place they already occupied and at
					the size they already had. The tick boxes go while writing: a wet
					thumb aiming at a field must not be able to tick a line by
					missing it.
				-->
					{#if amounts.length === 0}
						<p class="text-body opacity-70">{m.cook_nothing_new()}</p>
					{:else}
						<ul class="flex flex-col gap-2">
							{#each amounts as amount (amount.at)}
								<li class="flex items-start gap-2">
									<input
										value={amount.row.text}
										disabled={amount.row.dropped}
										oninput={(event) => write('ingredients', amount.at, event.currentTarget.value)}
										class="min-h-12 min-w-0 flex-1 rounded-sm border-b border-cook-accent bg-transparent py-2 font-display text-panel-figure font-semibold text-cook-ink disabled:line-through disabled:opacity-45"
									/>
									<button
										type="button"
										class="min-h-12 shrink-0 text-label text-cook-ink-2 uppercase"
										onclick={() => drop(amount.at)}
									>
										{amount.row.dropped ? m.cook_keep_line() : m.cook_drop_line()}
									</button>
								</li>
								<!--
								The recipe's own line, offered back on a tap. It lives here,
								inside writing mode, where *what did it say?* is the question
								being asked — and not on the screen a cook reads at the stove.
							-->
								{#if amount.changed && !amount.row.dropped}
									<li>
										<button
											type="button"
											class="min-h-12 text-left text-read text-cook-ink-2 line-through"
											onclick={() => write('ingredients', amount.at, amount.written)}
										>
											{amount.written}
											<span class="sr-only">— {m.cook_as_written()}</span>
										</button>
									</li>
								{/if}
							{/each}
						</ul>
					{/if}

					<!--
					THE REST OF THE LIST. Across the real corpus a step names an
					Ingredient Line only 42% of the time — 2 steps of 11 on Dan Dan
					Noodles — so on most steps the panel above is empty and the salt
					this cook actually used less of is not in it. Aurélien found that
					standing in the room, and both this and adding a line answer it:
					the whole list opens INSIDE the panel, which scrolls, so the screen
					still never hands the cook a second surface to be in.
				-->
					<button
						type="button"
						class="mt-3 flex min-h-12 w-full items-center justify-between border-t border-cook-rule pt-2 text-label uppercase
						{wholeList ? 'font-semibold text-cook-accent' : 'text-cook-ink-2'}"
						onclick={() => (wholeList = !wholeList)}
					>
						<span>{wholeList ? m.cook_every_line() : m.cook_other_lines()}</span>
						<span>{wholeList ? '−' : `+${otherLines.length}`}</span>
					</button>

					{#if wholeList}
						<ul>
							{#each otherLines as line (line.at)}
								<li class="flex items-start gap-2">
									<input
										value={line.row.text}
										disabled={line.row.dropped}
										placeholder={line.row.from === null ? m.cook_added_line() : undefined}
										oninput={(event) => write('ingredients', line.at, event.currentTarget.value)}
										class="min-h-12 min-w-0 flex-1 rounded-sm border-b border-cook-rule bg-transparent py-2 font-display text-body text-cook-ink disabled:line-through disabled:opacity-45"
									/>
									<button
										type="button"
										class="min-h-12 shrink-0 text-label text-cook-ink-2 uppercase"
										onclick={() => drop(line.at)}
									>
										{line.row.dropped ? m.cook_keep_line() : m.cook_drop_line()}
									</button>
								</li>
							{/each}
						</ul>
						<button
							type="button"
							class="min-h-12 w-full text-left text-label text-cook-accent uppercase"
							onclick={addLine}
						>
							{m.cook_add_a_line()}
						</button>
					{/if}
				{:else if amounts.length === 0}
					<p class="text-body opacity-70">{m.cook_nothing_new()}</p>
				{:else}
					<ul class="flex flex-col gap-2">
						{#each amounts as amount (amount.at)}
							<li>
								<button
									type="button"
									class="flex min-h-12 w-full items-baseline gap-3 text-left"
									aria-pressed={isTicked(amount.row)}
									onclick={() => tick(amount.row.from)}
								>
									<span
										class="mt-1 h-4 w-4 shrink-0 self-start rounded-sm border
										{isTicked(amount.row) ? 'border-cook-accent bg-cook-accent' : 'border-cook-rule'}"
										aria-hidden="true"
									></span>
									<span
										class="min-w-0 flex-1 {isTicked(amount.row) ? 'opacity-45' : ''}
										{amount.changed || amount.row.dropped ? 'border-l-2 border-cook-accent pl-2' : ''}"
									>
										{#if scaled && amount.beneath && !amount.changed && !amount.row.dropped}
											<!--
											Scaled, so the scaled amount leads and the recipe's own
											line sits beneath to say what it is an amount of (#109).
										-->
											<span class="block font-display text-panel-figure font-semibold">
												{amount.beneath}
											</span>
											<span class="block text-read text-cook-ink-2">
												{m.cook_recipe_line({ line: amount.row.text })}
											</span>
										{:else}
											<span
												class="block font-display text-panel-figure font-semibold
												{amount.row.dropped ? 'line-through opacity-45' : ''}"
											>
												{amount.row.text}
											</span>
											{#if scaled && !amount.changed && !amount.row.dropped}
												<span class="block text-read text-cook-ink-2"
													>{m.how_much_not_scaled()}</span
												>
											{:else if amount.beneath && current}
												<span class="block text-read text-cook-ink-2">{amount.beneath}</span>
											{/if}
										{/if}
										<!--
										What the recipe asked for, kept under what went in. One
										short line, and *what did it say?* is a real question with
										a hot pan in hand — unlike a whole struck paragraph under
										the Step, which would be a difference view (ADR 0014).
									-->
										{#if amount.changed && !amount.row.dropped}
											<span class="block text-read text-cook-ink-2 line-through">
												{amount.written}
											</span>
										{/if}
									</span>
									<span class="sr-only">{m.cook_tick({ line: amount.row.text })}</span>
								</button>
							</li>
						{/each}
					</ul>
				{/if}
			</div>

			<!--
			THE ROW ABOVE THE HAIRLINE, which exists whatever is on it, and is why
			everything on it is free (#88).

			THE TIMER LIVES HERE NOW. It used to sit under the Step, inside the
			Step's own scrolling column — so on a long Step a RUNNING timer
			scrolled out of sight, which is the one thing a timer must never do.
			Up here it is always visible, and it costs the room no height at all,
			because the row was already being paid for by `Changed it`.

			32px of ink over a 48px tap area, both of them: `tap-out` grows the
			hit box with a pseudo-element rather than with height, so a control a
			wet thumb must be able to hit does not spend a whole row saying one
			word. THE GAPS AROUND THEM ARE WHAT MAKES THAT SAFE — the overhang is
			8px on every side and invisible, and it wins any hit test it covers.
			`gap-6` leaves 8px of clearance between the two; `pt-3` above and the
			hairline's `mt-3` below leave 4px each. Narrow any of them and this
			row starts stealing taps from its neighbours in silence.
		-->
			<div class="flex shrink-0 items-center gap-6 pt-3">
				<!--
				HOW MUCH is being cooked, beside the amounts rather than up in the
				header. It belongs here: every figure above it was scaled to this
				number, so `800 ml` with nothing saying 800 ml towards what is half
				a fact. It was in the header until it made that row wrap (#88).
			-->
				<!--
				It gives way first when the row is full — a timer's offer and the
				Photo word beside it are things to press, and this is a thing to
				read — so it shrinks and ellipsises rather than pushing them off
				the edge of a narrow phone (#77).
			-->
				<!--
				It is also what asks again (#109): a cook three steps in who decides
				to stretch it to six taps the number they are cooking to. Outlined,
				like the timer beside it, because it does something; ellipsised
				before either of its neighbours, because it is also a thing to read.
			-->
				{#if !writing}
					<button
						type="button"
						class="tap-out h-8 min-w-0 truncate rounded-sm border border-cook-rule px-3 text-read text-cook-ink"
						aria-label={m.cook_how_much_is({ amount: howMuch })}
						onclick={() => {
							atStart = false;
							asking = true;
						}}
					>
						{howMuch}
					</button>
				{/if}
				{#if writing}
					<p class="min-w-0 truncate text-label font-semibold text-cook-accent uppercase">
						{m.cook_writing()}
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
						class="tap-out h-8 shrink-0 rounded-sm border px-3 text-read font-semibold
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
						class="tap-out h-8 shrink-0 rounded-sm border border-cook-accent px-3 text-read font-semibold text-cook-accent"
						onclick={() => countdown.start(duration)}
					>
						{m.cook_timer_start({ duration: offer })}
					</button>
				{/if}
				<!--
				ONE WORD, and the whole of how a cook says *I did it differently*
				(#58). It opens no sheet and covers nothing: the amounts and the
				Step become fields where they already stand, so the cook never
				leaves the step they are on. Aurélien chose this over a sheet on
				4 September 2026, against both built and running; the reasoning is
				on #58.

				A cooking that deviated from nothing pays this word and nothing
				else — no row, no panel, no badge, and no stored state.
			-->
				<!--
				A PICTURE OF THE COOKING, one quiet word beside "Changed it" and the
				same size, at the stove where the dish is (#77). Aurélien chose this
				over offering it once the cooking is finished, on 19 September 2026:
				the dough at step 4 is photographed at step 4. It opens the phone's
				camera, the cook comes back to the step they were on, and the word
				counts what has been taken.

				A label around the file picker rather than a button that clicks
				one: a label is the one way iOS Safari always opens a picker, and
				the picker inside stays reachable for a keyboard. It goes while the
				step is being written on, where the row belongs to Done.
			-->
				{#if !writing}
					<label
						class="tap-out ms-auto flex h-8 shrink-0 cursor-pointer items-center text-read text-cook-ink-2"
					>
						{attempt.photographs.length > 0
							? m.cook_photo_count({ count: attempt.photographs.length })
							: m.cook_photo()}
						<input
							type="file"
							accept="image/*"
							capture="environment"
							class="sr-only"
							aria-label={m.cook_photo_take()}
							onchange={takePhoto}
						/>
					</label>
				{/if}
				<button
					type="button"
					class="tap-out h-8 shrink-0 text-read {writing
						? 'ms-auto font-semibold text-cook-accent'
						: 'text-cook-ink-2'}"
					onclick={() => {
						if (writing) void save();
						writing = !writing;
					}}
				>
					{writing ? m.cook_writing_done() : m.cook_changed_it()}
				</button>
			</div>
			<!--
			The Step. The largest type in the app, and the only thing here that
			scrolls — everything else is fixed, so the cook's eye lands in the
			same place on every step.

			The text is NOT given the free space: it takes the height it needs and
			scrolls only once there is no more, so the temperature stays against
			the sentence it belongs to rather than being pushed to the foot of the
			screen. A Step's conversion is an addition BESIDE it (CONTEXT.md), and
			half a screen away is not beside. The timer used to be under here too
			and is not any more — see the row above the hairline (#88).
		-->
			<div class="mt-3 flex min-h-0 flex-1 flex-col items-start border-t border-cook-rule pt-3">
				{#if writing}
					<!--
					The Step, still the largest type in the app, still in the same
					place — now a field. The cook never left the step, because there
					was never anywhere else to be.
				-->
					<textarea
						value={here.row.text}
						placeholder={here.row.from === null ? m.cook_added_step() : undefined}
						oninput={(event) => write('steps', here.index, event.currentTarget.value)}
						class="min-h-0 w-full flex-1 resize-none rounded-sm border border-cook-accent bg-transparent p-2 font-display text-step font-semibold text-cook-ink"
					></textarea>
					{#if wroteStep(here.row) && here.row.from !== null}
						{@const written = content.steps[here.row.from]?.text ?? ''}
						<button
							type="button"
							class="min-h-12 shrink-0 text-left text-read text-cook-ink-2 line-through"
							onclick={() => write('steps', here.index, written)}
						>
							{written}
							<span class="sr-only">— {m.cook_as_written()}</span>
						</button>
					{/if}
					<!--
					A method that grew a stage. Inserting BEFORE and staying put is
					Aurélien's wording and the right shape: a cook writes the step
					down having just done it, so the new one is where they now are.
				-->
					<button
						type="button"
						class="min-h-12 shrink-0 text-label text-cook-accent uppercase"
						onclick={insertStep}
					>
						{m.cook_insert_step()}
					</button>
				{:else}
					<!--
					A rewritten Step wears the bar and nothing else. An amount keeps
					its old line under it, but a whole struck paragraph under the
					largest type in the app is a difference view, and Kamosu does not
					draw one (ADR 0014). The recipe's words are one tap away inside
					writing mode.
				-->
					<!--
					A Step longer than its box is about one in twelve of the corpus
					once #88's sizes landed, and a line sliced in half by the box's
					edge reads as a rendering fault rather than as an invitation to
					scroll. The last 28px fade into the room's own ground, and only
					when there is actually more below — a fade over a Step that fits
					would grey out its last line for nothing. The identity record of
					26 August asked for exactly this: scrolling a Step has to feel
					intended rather than be pretended away.
				-->
					<!--
					The Step's photograph, where the recipe gives it one (#110): a
					small square at the top right that the words wrap around, and a
					tap shows it across the screen. Aurélien's choice (K3) over a
					picture under the Step, which cut a long Step's words off to make
					room, and over a button that hid it. A `div` rather than a `p`
					because the picture's full-screen view cannot sit inside a
					paragraph.

					By where the step CAME FROM, like everything else read off the
					Version: a step the cook wrote has no photograph, and a rewritten
					one keeps the recipe's.
				-->
					<div class="relative min-h-0 w-full">
						<div
							bind:this={stepEl}
							class="max-h-full overflow-y-auto font-display text-step font-semibold
							{wroteStep(here.row) ? 'border-l-[3px] border-cook-accent pl-3' : ''}"
						>
							{#if stepPhoto}
								<StepPhoto
									photograph={stepPhoto}
									number={position + 1}
									shapeClass="float-right mt-1 mb-2 ml-3 h-[var(--step-photo)] w-[var(--step-photo)]"
								/>
							{/if}
							{here.row.text}
						</div>
						{#if stepScrolls}
							<div
								class="pointer-events-none absolute inset-x-0 bottom-0 step-fade"
								aria-hidden="true"
							></div>
						{/if}
					</div>
					<!--
				A Step's own subordinate line: the oven temperature in this
				cook's measures, on the conventional ladder. Beside the
				sentence, never written into it.
			-->
					{#if beneathStep}
						<p class="shrink-0 pt-3 text-read text-cook-ink-2">{beneathStep}</p>
					{/if}
				{/if}
			</div>
		{/if}

		<!--
			THE TWO WAYS THROUGH THE RECIPE, and now the only things at the foot of
			the phone (#88). Back was a 112px box against a full-width Next, which
			made the control you reach for when you missed something the smallest
			one on screen; they are halves of the same row now, and both are real
			blocks. Neither wears `py-4` on top of `min-h-12`: a button sized twice
			is a button 57px tall, and this screen has no 9px to spare.

			Nothing else is down here. The false start and the screen's sleep were
			10.5px words at the very bottom edge, where a wet thumb lands — they
			are in the header now, and this row is the whole of what the bottom of
			the screen can do.
		-->
		<div class="flex shrink-0 gap-3 pt-3">
			<button
				type="button"
				class="min-h-12 flex-1 rounded-sm border border-cook-accent font-semibold text-cook-accent disabled:opacity-40"
				disabled={position === 0 || asking}
				onclick={() => step(position - 1)}
			>
				{m.cook_back()}
			</button>
			{#if asking}
				<button
					type="button"
					class="min-h-12 flex-1 rounded-sm bg-cook-accent font-semibold text-cook-on-accent"
					onclick={() => {
						asking = false;
						atStart = false;
					}}
				>
					{atStart ? m.cook_start_cooking() : m.cook_back_to_step({ n: position + 1 })}
				</button>
			{:else if last}
				<button
					type="button"
					class="min-h-12 flex-1 rounded-sm bg-cook-accent font-semibold text-cook-on-accent"
					onclick={finish}
				>
					{m.cook_finish()}
				</button>
			{:else}
				<button
					type="button"
					class="min-h-12 flex-1 rounded-sm bg-cook-accent font-semibold text-cook-on-accent"
					onclick={() => step(position + 1)}
				>
					{m.cook_next()}
				</button>
			{/if}
		</div>

		{#if discarding}
			<div
				bind:this={discardDialog}
				tabindex="-1"
				class="fixed inset-x-0 bottom-0 z-40 mx-auto max-w-2xl bg-cook-panel px-gutter pt-6 pb-safe"
				role="dialog"
				aria-modal="true"
				aria-label={m.cook_false_start()}
			>
				<p class="text-body text-cook-ink">{m.cook_false_start_confirm()}</p>
				<button
					type="button"
					class="mt-4 min-h-12 w-full rounded-sm bg-cook-accent font-semibold text-cook-on-accent"
					onclick={discard}
				>
					{m.cook_false_start_yes()}
				</button>
				<button
					type="button"
					class="mt-3 min-h-12 w-full rounded-sm border border-cook-rule text-body text-cook-ink"
					onclick={() => (discarding = false)}
				>
					{m.cook_false_start_no()}
				</button>
			</div>
		{/if}
	{/if}
</div>
