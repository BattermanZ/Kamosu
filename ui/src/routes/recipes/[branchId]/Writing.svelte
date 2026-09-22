<!--
	Writing a recipe (#83). The other half of #81, which only reads.

	THE PAGE IS THE PAGE. Aurélien chose direction A on 20 September 2026,
	against four full mockups drawn on Dan Dan Noodles out of the real
	86-recipe export, and the reasoning is on #83 rather than repeated here.
	What it settled, and what this file has to keep true:

	  · there is NO SECOND SCREEN. Hero, the facts, Ingredients, and the Method
	    underneath them, in the order #81 puts them, each part becoming a field
	    where it already sits. A separate compose screen was drawn (B) and
	    rejected, so moving any part of this onto its own route undoes the
	    choice;
	  · a long list is ONE RUN OF TYPING. Return at the end of a line makes the
	    next one, so twenty-one lines is not twenty-one taps of *add another*;
	  · both lists carry BOTH controls — a line or a step, and a heading — and
	    each drops the new thing in where the cursor is, so a heading goes into
	    the middle of a list without retyping what follows it.

	One argument was put to Aurélien for the separate screen and it was wrong:
	that a long list gets more room there. The hero scrolls away, so it does
	not. That mistake is recorded on #83 so nobody re-derives it.

	AN INGREDIENT LINE IS ONE FREE-TEXT FIELD (ADR 0002, #43). Never an amount
	box, a unit box and a food box. `2 poignées de farine, environ` goes in and
	comes out as typed, and nothing here rewrites a character of it. What
	Kamosu understands OF that line — the amount, the Unit, the Food — is the
	Reading, and it is corrected on the reading page and is no part of this
	screen (ADR 0021).

	ONE PART OF THE READING IS HERE, AND ONLY ONE (#87): whether the line names
	a RECIPE rather than a Food. Aurélien settled on 20 September 2026 that the
	act of saying *this line is a recipe* is designed once against both screens
	and wears the same small matcha control on both, so it is here as well as in
	the reading page's corrector.

	It is not the exception it looks like. The other three are Kamosu's reading
	of words you wrote, which is why correcting them belongs where you read
	them. This one is not readable from the words at all and never will be:
	ADR 0008 refuses matching `500 g plain flour` against a flour recipe in as
	many words. Only the person typing the line knows, and this is where they
	are.

	WHAT IT COSTS, AND WHY IT IS PAID AFTER THE SAVE. A Reading is keyed to the
	Version it belongs to, and the Version this screen is writing does not exist
	yet — so a pointer cannot be attached until the save lands. It is applied
	immediately afterwards, with `set_reading`, against the Version that just
	landed. The recipe is re-read first for one reason: `set_reading` replaces
	the WHOLE Reading, and the amount and the Unit it would otherwise clear are
	the ones the Core has just worked out of the new line. `500 g pizza dough`
	losing its 500 does not look broken — it silently means the whole dough
	(ADR 0008).

	A SECTION IS A REAL OBJECT in both lists, not a line pretending to be one:
	it is stored as `kind: "section"` and it survives a save as itself.

	A LINE IS DRAGGED, AND THE LIST REARRANGES UNDER IT. You put the line where
	you can see it going, rather than describing the move and watching it
	happen afterwards. This is the handle the mockup drew, and it is the
	control Aurélien approved, so a two-step *move, then choose where* is not
	a substitute for it.

	It is built on pointer events. HTML5 drag-and-drop does not fire on touch
	at all, which on a phone-first app would make rearranging a desktop-only
	feature. The handle captures the pointer, so the drag survives the row
	moving out from under the finger — which happens immediately, because that
	is the point. The arrow keys move a row one place at a time, because a
	control that can only be dragged cannot be reached from a keyboard and
	`svelte-check` counts that as a build failure here (ADR 0012).

	THE SAVE SAYS WHICH OF TWO THINGS IT IS ABOUT TO DO (#54), before it does
	it, in fixed words, and the two are not the same control. Saving a recipe
	one of your Kitchens holds writes a Version on that Branch. Saving one they
	do not forks a Branch of the same Lineage, held by you. Whether this is a
	fork is read off the Kitchens the caller actually cooks in, never guessed.

	WHAT A SAVE DID IS SAID BY THE PAGE, NOT BY THIS SCREEN. Saving closes this
	screen, so a line drawn here would be destroyed before anybody read it —
	which is exactly what happened the first time it was written that way. The
	outcome goes up through `onSaved` and the recipe page says it.

	There are two things worth saying. A second save by the same Hand within
	`COLLAPSE_WINDOW_SECONDS` joins the Version already being shaped instead of
	appending (`src/core.rs`), and a save that silently made no Thread entry
	reads as a save that did not happen. And a Copy lands on a DIFFERENT
	Branch: the answer's `branch_id` is the new one, so the page has to go
	there or the cook is left reading the recipe they did not change.

	OFFLINE THIS SCREEN IS NOT REACHED (ADR 0013, #76). Editing needs the
	server and the button that opens this says so, drawn with `NeedsServer`.
	There is no queue, because an offline edit queue is a merge.

	THERE IS STILL NO DELETE HERE, and now for a better reason. The mockup drew
	one; until #120 Kamosu had no `delete_recipe` Operation at all, and a
	button for an Operation that does not exist is worse than its absence.

	The Operation exists now, and the act lives on the recipe page instead —
	set apart below the actions, where `Recipe.svelte` records why. Keeping it
	off this screen was decided rather than inherited: this screen holds an
	unsaved draft the whole time it is open, so a delete sitting beside Save
	would let one set of buttons ask both "keep what I typed?" and "destroy the
	recipe?". Those are not questions to answer in the same glance.
-->
<script lang="ts">
	import { tick } from 'svelte';
	import type { Attachment } from 'svelte/attachments';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { usePhotograph } from '$lib/api/upload';
	import type {
		GetRecipeOutput,
		ListKitchensOutput,
		ReadPastedRecipeOutput,
	} from '$lib/api/catalogue';
	import type { Nutrition } from './divergence';
	import Cover from '$lib/cover/Cover.svelte';
	import ComponentPicker, { type NamedRecipe } from './ComponentPicker.svelte';

	type Content = GetRecipeOutput['versions'][number]['content'];

	interface Props {
		branchId: string;
		lineageId: string;
		/** The Kitchen holding the Branch being written on. */
		kitchenId: string;
		/** The recipe as it stands, which is what the draft below starts from. */
		content: Content;
		/**
		 * The Components of the recipe as it stands, so a line that already
		 * names a Recipe opens saying so rather than looking like an ordinary
		 * line somebody is about to lose the pointer on. Only the top level:
		 * a Component inside a Component belongs to the inner recipe, and is
		 * edited on the inner recipe's own page.
		 */
		components?: GetRecipeOutput['versions'][number]['components'];
		onCancel: () => void;
		/**
		 * A Version landed. The page re-reads and this screen closes, so what
		 * happened is handed UP rather than said here: a line drawn by a
		 * component that is about to be destroyed is a line nobody reads.
		 *
		 * `branch_id` is the Branch the Version is on, which after a Copy is
		 * the NEW one rather than the one that was open.
		 *
		 * `named` is false where the Version landed but a line naming another
		 * recipe could not be marked (#87). It travels with the rest for the
		 * same reason they do: the screen that knows is closing.
		 */
		onSaved: (landed: {
			branch_id: string;
			collapsed: boolean;
			copied: boolean;
			named: boolean;
		}) => void;
	}

	let {
		branchId,
		lineageId,
		kitchenId,
		content,
		components = [],
		onCancel,
		onSaved,
	}: Props = $props();

	const kamosu = useKamosu();
	const sendPhotograph = usePhotograph();

	/**
	 * A row carries an id of its own because the list is keyed by it. Keying by
	 * index would move focus to a different row the moment anything above the
	 * caret is inserted, removed or moved, which on a twenty-one line list is
	 * the difference between typing and fighting.
	 */
	let nextId = 0;
	const id = () => (nextId += 1);

	interface Line {
		id: number;
		kind: 'ingredient' | 'section';
		text: string;
		/** The Recipe this line names, where it names one (#87, ADR 0008). */
		namedRecipe: NamedRecipe | null;
	}
	interface Step {
		id: number;
		kind: 'step' | 'section';
		text: string;
		photo: string | null;
	}

	// ---- the draft ------------------------------------------------------

	/**
	 * Seeded ONCE from the recipe as it stood when this opened. Deliberately
	 * not derived from the prop: a re-read landing mid-sentence must not
	 * overwrite what somebody is halfway through typing.
	 */
	// svelte-ignore state_referenced_locally
	let title = $state(content.title);
	// svelte-ignore state_referenced_locally
	let mainPhoto = $state<string | null>(content.main_photo);
	// svelte-ignore state_referenced_locally
	let prep = $state(content.prep_time_minutes === null ? '' : String(content.prep_time_minutes));
	// svelte-ignore state_referenced_locally
	let cook = $state(content.cook_time_minutes === null ? '' : String(content.cook_time_minutes));
	// svelte-ignore state_referenced_locally
	let yieldAmount = $state(content.yield?.amount ?? '');
	// svelte-ignore state_referenced_locally
	let yieldNoun = $state(content.yield?.noun ?? '');
	// svelte-ignore state_referenced_locally
	let note = $state(content.note ?? '');
	// svelte-ignore state_referenced_locally
	let sourceText = $state(content.source?.text ?? '');
	// svelte-ignore state_referenced_locally
	let sourceLink = $state(content.source?.link ?? '');
	/**
	 * The Nutrition figure, typed at the foot of the Ingredients — where #84
	 * put it for reading, so writing it is in the same place. The basis is
	 * typed WITH the number rather than being a setting somewhere: 308 says
	 * nothing until it says what it counts, and a serving and 100 g do not
	 * convert into each other without a weight the recipe does not carry.
	 *
	 * An empty number means the recipe has no figure, whatever the basis says.
	 * Most recipes have none, so that is the ordinary path rather than the edge.
	 *
	 * The `per_serving` below is the control's opening position on a recipe
	 * that carries no figure at all, not a guess about one that does: where
	 * there IS a figure its own basis is read off it.
	 */
	// svelte-ignore state_referenced_locally
	let calories = $state(content.nutrition === null ? '' : String(content.nutrition.calories));
	// svelte-ignore state_referenced_locally
	let basis = $state<Nutrition['basis']>(content.nutrition?.basis ?? 'per_serving');
	/**
	 * Which lines already name a Recipe, by index into the written list. Read
	 * off the Components the Core unfolded rather than off the Readings,
	 * because the Component is the one that carries the title.
	 */
	// svelte-ignore state_referenced_locally
	const namedAtOpening = new Map<number, NamedRecipe>(
		components
			.filter((component) => component.path.length === 1)
			.map((component) => [
				component.path[0] as number,
				{ lineageId: component.lineage_id, title: component.title },
			]),
	);
	// svelte-ignore state_referenced_locally
	let lines = $state<Line[]>(
		content.ingredients.map((item, index) => ({
			id: id(),
			kind: item.kind,
			text: item.text,
			namedRecipe: namedAtOpening.get(index) ?? null,
		})),
	);
	// svelte-ignore state_referenced_locally
	let steps = $state<Step[]>(
		content.steps.map((item) => ({
			id: id(),
			kind: item.kind,
			text: item.text,
			photo: item.photo ?? null,
		})),
	);

	/**
	 * The row last written in, so *Add a line* and *Add a heading* put the new
	 * thing beside it rather than always at the foot. It deliberately survives
	 * the field losing focus, because pressing one of those buttons is what
	 * takes focus away and the row it inserts after still has to be known. A
	 * removal or a move clears it, since the index it holds stops meaning
	 * anything then.
	 */
	let cursor = $state<{ list: 'lines' | 'steps'; index: number } | null>(null);
	/** The row under the finger, while one is being dragged. */
	let dragging = $state<{ list: 'lines' | 'steps'; index: number } | null>(null);
	/** The two lists' own elements, which a drag measures the rows of. */
	let linesEl = $state<HTMLElement | undefined>(undefined);
	let stepsEl = $state<HTMLElement | undefined>(undefined);

	let photoFailed = $state(false);
	/**
	 * Which line the picker is open for, by the row's own id rather than its
	 * index (#87) — the same reason the list is keyed by id: an index stops
	 * meaning the row it meant the moment anything above it moves.
	 */
	let picking = $state<number | null>(null);
	/** The row it is open for, or nothing — looked up by id, never by position. */
	const pickingFor = $derived(lines.find((row) => row.id === picking));
	let asking = $state(false);
	let saving = $state(false);
	let failed = $state<string | undefined>(undefined);
	let versionName = $state('');
	let changeNote = $state('');

	// ---- pasting a whole recipe (#94) -----------------------------------
	//
	// C's one advantage, kept as a WAY IN rather than as a way of working
	// (#83): paste a whole recipe, and Kamosu says what it made of it BEFORE
	// anything lands. The engine is `read_pasted_recipe`, an Operation rather
	// than a second parser in a second language — it rests on `reading.rs`,
	// whose Unit vocabulary is `units.rs`'s in three Languages.
	//
	// THE GUESS IS NEVER APPLIED SILENTLY. Measured on the real 86-recipe
	// export, the boundary lands exactly on 77.5% of recipes and within one
	// line on 95%. The 5% is why this sheet exists at all: it names the
	// counts, draws the split, and lets it be moved. A parser right four
	// times in five and silent the fifth is worse than one that says so.
	//
	// NOTHING IS SAVED HERE. Using a paste fills the fields on this page and
	// stops. The save is the ordinary save underneath, which still states
	// which of the two acts it is about to be.

	let pasteOpen = $state(false);
	/** The `⋯` menu, which is how the offer is reached once the page holds something. */
	let moreOpen = $state(false);
	let pasteText = $state('');
	let reading = $state(false);
	let pasteFailed = $state<string | undefined>(undefined);
	/** What came back, or nothing while the paste has not been read yet. */
	let pasted = $state<ReadPastedRecipeOutput | null>(null);
	/**
	 * Where the method starts. Seeded from the answer and then moved by hand,
	 * which RE-SPLITS THE SAME ANSWER: every line came back already read, so
	 * moving this re-reads not one character of any line.
	 */
	let boundary = $state(0);

	/** Whether this recipe already holds something a paste would replace. */
	const holdsSomething = $derived(lines.length > 0 || steps.length > 0);

	/** How many lines the paste holds, readable from a callback that runs later. */
	const pastedLines = $derived(pasted?.lines.length ?? 0);
	const above = $derived(pasted ? pasted.lines.slice(0, boundary) : []);
	const below = $derived(pasted ? pasted.lines.slice(boundary) : []);
	/** The counts the sheet names — a Section is neither an ingredient nor a step. */
	const countOf = (rows: { kind: string }[]) => rows.filter((row) => row.kind === 'line').length;
	const sectionsIn = (rows: { kind: string }[]) =>
		rows.filter((row) => row.kind === 'section').length;

	/**
	 * **What using this paste would overwrite.** A recipe made by
	 * `create_recipe` holds a title and two empty lists, so the lists alone do
	 * not answer this: a paste carrying its own title would silently replace
	 * one somebody typed a moment ago. Said separately because it is a
	 * separate loss.
	 */
	const replacesTitle = $derived(
		title.trim() !== '' && pasted !== null && (pasted.title ?? '').trim() !== '',
	);

	function openPaste() {
		moreOpen = false;
		pasteOpen = true;
		pasteText = '';
		pasted = null;
		pasteFailed = undefined;
	}

	async function readPaste() {
		if (pasteText.trim() === '') {
			pasteFailed = m.write_paste_nothing();
			return;
		}
		reading = true;
		pasteFailed = undefined;
		try {
			const answer = await kamosu.readPastedRecipe({ text: pasteText });
			if (answer.lines.length === 0) {
				pasteFailed = m.write_paste_nothing();
			} else {
				pasted = answer;
				boundary = answer.boundary;
			}
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			pasteFailed = error.message;
		}
		reading = false;
	}

	/**
	 * Put the paste onto the page. The two kinds map straight across — a
	 * heading is a Section in whichever list it landed in — and every line
	 * goes in exactly as it came back (ADR 0002).
	 */
	function usePaste() {
		if (!pasted) return;
		if (pasted.title !== null && pasted.title.trim() !== '') title = pasted.title;
		lines = above.map((row) => ({
			id: id(),
			kind: row.kind === 'section' ? ('section' as const) : ('ingredient' as const),
			text: row.text,
			// A pasted line naming a recipe on the shelf is an ordinary
			// Ingredient Line until somebody says otherwise (ADR 0008).
			namedRecipe: null,
		}));
		steps = below.map((row) => ({
			id: id(),
			kind: row.kind === 'section' ? ('section' as const) : ('step' as const),
			text: row.text,
			photo: null,
		}));
		cursor = null;
		pasteOpen = false;
		pasted = null;
	}

	// ---- which of the two saves this is ---------------------------------

	let kitchens = $state<ListKitchensOutput['kitchens']>([]);
	/**
	 * Whether the Kitchens are known yet. The save is held until they are.
	 *
	 * This is not caution for its own sake: which of the two saves this is, is
	 * read off them. Unknown, `forking` is false, so the sheet would say
	 * *writes a new Version onto your recipe, in* — with no Kitchen named —
	 * and the server would fork anyway. A screen whose whole job at that
	 * moment is to state the outcome must not guess it.
	 */
	let kitchensKnown = $state(false);
	let kitchensFailed = $state(false);
	$effect(() => {
		let current = true;
		kamosu
			.listKitchens({})
			.then((all) => {
				if (!current) return;
				kitchens = all.kitchens;
				kitchensKnown = true;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				if (current) kitchensFailed = true;
			});
		return () => {
			current = false;
		};
	});

	/**
	 * A Copy, not a Version: this Branch is held by a Kitchen the caller does
	 * not cook in. Read off `list_kitchens` rather than guessed, and false
	 * until that answers — the sheet is what states the outcome, and it is not
	 * opened before the answer is in.
	 */
	const forking = $derived(kitchensKnown && !kitchens.some((kitchen) => kitchen.id === kitchenId));
	const landsIn = $derived(kitchens.find((kitchen) => kitchen.is_home) ?? kitchens[0]);
	const holding = $derived(kitchens.find((kitchen) => kitchen.id === kitchenId));
	/** The Kitchen the save writes into, whichever of the two acts it is. */
	const savingInto = $derived(forking ? landsIn : (holding ?? landsIn));

	// ---- the lists ------------------------------------------------------

	/**
	 * A row's number within its own kind, so a heading takes none and the
	 * first ingredient is *line 1* rather than *line 2*. The Step numbers are
	 * also what the page shows, counted the way the reading page counts them.
	 */
	const numbered = (rows: { kind: string }[], kind: string) => {
		let n = 0;
		return rows.map((row) => (row.kind === kind ? (n += 1) : null));
	};
	const stepNumbers = $derived(numbered(steps, 'step'));
	const lineNumbers = $derived(numbered(lines, 'ingredient'));

	/** Where a new row goes: after the row the caret is in, else at the foot. */
	const insertAt = (list: 'lines' | 'steps', length: number) =>
		cursor && cursor.list === list ? cursor.index + 1 : length;

	/**
	 * THE CARET GOES INTO THE NEW ROW. Without this the row appears and the
	 * caret stays where it was, so *Return, type, Return, type* puts the whole
	 * list into the first field — which is exactly what it did the first time
	 * this was tried on a real 21-line recipe. A long list being one run of
	 * typing is the whole of why this direction was chosen, and it rests
	 * entirely on this line.
	 */
	async function writeIn(list: 'lines' | 'steps', index: number) {
		await tick();
		rowsOf(list)[index]?.querySelector('textarea')?.focus();
	}

	function addLine(kind: 'ingredient' | 'section') {
		const at = insertAt('lines', lines.length);
		lines.splice(at, 0, { id: id(), kind, text: '', namedRecipe: null });
		cursor = { list: 'lines', index: at };
		void writeIn('lines', at);
	}

	function addStep(kind: 'step' | 'section') {
		const at = insertAt('steps', steps.length);
		steps.splice(at, 0, { id: id(), kind, text: '', photo: null });
		cursor = { list: 'steps', index: at };
		void writeIn('steps', at);
	}

	/**
	 * Return makes the next row rather than a newline: a list is lines, and a
	 * line that wraps is still one line. Shift and Return are left alone, so a
	 * step that genuinely wants a paragraph can still have one.
	 */
	function onKey(event: KeyboardEvent, list: 'lines' | 'steps', index: number) {
		if (event.key !== 'Enter' || event.shiftKey) return;
		event.preventDefault();
		cursor = { list, index };
		if (list === 'lines') addLine('ingredient');
		else addStep('step');
	}

	function remove(list: 'lines' | 'steps', index: number) {
		if (list === 'lines') lines.splice(index, 1);
		else steps.splice(index, 1);
		cursor = null;
	}

	// ---- rearranging ----------------------------------------------------
	//
	// A row is DRAGGED, and the list rearranges under it as it goes — you put
	// the line where you can see it going, rather than describing the move and
	// watching it happen afterwards.
	//
	// Pointer events rather than HTML5 drag-and-drop, which does not fire on
	// touch at all and would make this a desktop-only control on a phone-first
	// app. The handle captures the pointer, so the drag survives the finger
	// leaving the row it started on — which it does immediately, because the
	// row moves out from under it.
	//
	// The arrow keys do the same thing one row at a time. A control that can
	// only be dragged cannot be reached from a keyboard at all, and
	// `svelte-check` counts that as a build failure here (ADR 0012).

	/** Where the finger is, kept so the scroll tick can re-read it. */
	let pointerAt = 0;
	let ticking = 0;

	const rowsOf = (list: 'lines' | 'steps') =>
		Array.from(
			(list === 'lines' ? linesEl : stepsEl)?.querySelectorAll<HTMLElement>('li[data-row]') ?? [],
		);

	/** Put a row somewhere else in its own list. The whole of the arithmetic. */
	function shift(list: 'lines' | 'steps', from: number, to: number) {
		const rows = list === 'lines' ? lines : steps;
		if (to < 0 || to >= rows.length || to === from) return false;
		const [row] = rows.splice(from, 1);
		rows.splice(to, 0, row as never);
		// The row the caret was in has moved, so where a new line would land
		// stops meaning what it meant.
		cursor = null;
		return true;
	}

	/** Which row the finger is over now, and the list rearranged to match. */
	function settle() {
		if (!dragging) return;
		const rows = rowsOf(dragging.list);
		let to = rows.findIndex((row) => {
			const box = row.getBoundingClientRect();
			return pointerAt < box.top + box.height / 2;
		});
		if (to < 0) to = rows.length - 1;
		if (shift(dragging.list, dragging.index, to)) dragging = { list: dragging.list, index: to };
	}

	/**
	 * Dragging towards the top or the bottom of the screen scrolls the page.
	 * Without it a line can only be moved as far as the screen is tall, and
	 * the list this was built for is twenty-one lines long.
	 */
	function autoScroll() {
		if (!dragging) return;
		const EDGE = 96;
		const above = pointerAt - EDGE;
		const below = window.innerHeight - EDGE - pointerAt;
		if (above < 0) window.scrollBy(0, Math.max(-20, above / 3));
		else if (below < 0) window.scrollBy(0, Math.min(20, -below / 3));
		settle();
		ticking = requestAnimationFrame(autoScroll);
	}

	function startDrag(list: 'lines' | 'steps', index: number, event: PointerEvent) {
		// Left button or a finger, never a right-click or the browser's own
		// text selection starting under the handle.
		if (event.button !== 0) return;
		event.preventDefault();
		dragging = { list, index };
		pointerAt = event.clientY;

		// THE MOVES ARE LISTENED FOR ON THE WINDOW, not on the handle. Pointer
		// capture is the tidier mechanism and it is asked for below, but it is
		// not what this relies on: where the browser refuses the capture the
		// finger leaves the handle on the very first move — the row slides out
		// from under it by design — and every move after that would be
		// delivered somewhere else. Watching the window cannot miss them.
		try {
			(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
		} catch {
			// An id the browser will not capture. The window listeners stand.
		}
		window.addEventListener('pointermove', onDrag);
		window.addEventListener('pointerup', endDrag);
		window.addEventListener('pointercancel', endDrag);
		ticking = requestAnimationFrame(autoScroll);
	}

	function onDrag(event: PointerEvent) {
		if (!dragging) return;
		// Otherwise the phone reads the drag as a page scroll partway through.
		event.preventDefault();
		pointerAt = event.clientY;
		settle();
	}

	function endDrag() {
		dragging = null;
		cancelAnimationFrame(ticking);
		window.removeEventListener('pointermove', onDrag);
		window.removeEventListener('pointerup', endDrag);
		window.removeEventListener('pointercancel', endDrag);
	}

	// A drag interrupted by the screen going away leaves no listeners behind.
	$effect(() => endDrag);

	function onHandleKey(event: KeyboardEvent, list: 'lines' | 'steps', index: number) {
		const by = event.key === 'ArrowUp' ? -1 : event.key === 'ArrowDown' ? 1 : 0;
		if (by === 0) return;
		event.preventDefault();
		if (!shift(list, index, index + by)) return;
		// The focus goes with the row. Without it the next arrow press moves
		// whatever has landed here instead, which is not what anybody holding
		// the key down is asking for.
		void (async () => {
			await tick();
			rowsOf(list)[index + by]?.querySelector<HTMLElement>('[data-handle]')?.focus();
		})();
	}

	// ---- photographs ----------------------------------------------------

	/**
	 * A picture is remade at the door and named by its own bytes (ADR 0017),
	 * so what a recipe stores is the name the server answers with.
	 *
	 * IT GOES TO THE SERVER NOW, not to the outbox. A photograph taken while
	 * cooking is kept on the phone under a `local:…` name and given its real
	 * one when the cooking is finally sent, because the outbox rewrites those
	 * names on the way out. NOTHING rewrites them inside a
	 * `save_recipe_version`, so a recipe that took that path would store a
	 * name no server has ever heard of and draw a broken picture for ever.
	 * This screen needs the server anyway, so there is nothing to keep.
	 */
	async function takePhoto(file: File, onto: (name: string) => void) {
		photoFailed = false;
		try {
			onto(await sendPhotograph(file));
		} catch {
			photoFailed = true;
		}
	}

	const pick = (event: Event, onto: (name: string) => void) => {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		// Cleared so choosing the same file twice in a row still fires.
		input.value = '';
		if (file) void takePhoto(file, onto);
	};

	// ---- saving ---------------------------------------------------------

	/** A field left blank is a field with nothing in it, never an empty string. */
	const orNothing = (value: string) => (value.trim() === '' ? null : value.trim());
	/** Whole minutes, or nothing. A word where a number goes is nothing. */
	const minutes = (value: string) => {
		const n = Number.parseInt(value.trim(), 10);
		return value.trim() === '' || Number.isNaN(n) || n < 0 ? null : n;
	};
	/**
	 * The Nutrition figure in kcal, or nothing. Not whole, unlike minutes: the
	 * Catalogue declares a number rather than an integer, and a source page
	 * stating 154.5 per 100 g is stating a figure Kamosu has no business
	 * rounding.
	 *
	 * `Number.isFinite` rather than `!Number.isNaN`, because `Number` reads
	 * `Infinity` as a number and `JSON.stringify` then writes it as `null` —
	 * which reaches the Core as a shape error instead of the sentence below.
	 */
	const kcal = (value: string) => {
		const n = Number(value.trim());
		return value.trim() === '' || !Number.isFinite(n) || n < 0 ? null : n;
	};

	/**
	 * What cannot be saved as typed, in words. A Yield is one amount AND one
	 * noun — `4` on its own says nothing and the Catalogue will not carry it —
	 * and a time is whole minutes. Both were being dropped in silence, which
	 * loses something somebody typed on purpose.
	 */
	const wrong = $derived.by(() => {
		const amount = yieldAmount.trim();
		const noun = yieldNoun.trim();
		if (title.trim() === '') return m.write_needs_title();
		if ((amount === '') !== (noun === '')) return m.write_needs_both_yield();
		for (const [value, label] of [
			[prep, m.recipe_min_prep()],
			[cook, m.recipe_min_cook()],
		] as const) {
			if (value.trim() !== '' && minutes(value) === null) {
				return m.write_needs_minutes({ field: label });
			}
		}
		if (calories.trim() !== '' && kcal(calories) === null) return m.write_needs_nutrition();
		return undefined;
	});

	/** The rows that are actually lines of the recipe, in the order they save in. */
	const keptLines = () => lines.filter((row) => row.text.trim() !== '');

	/**
	 * **Attach the pointers this screen was given, to the Version that just
	 * landed** (#87). Answers whether every one of them took.
	 *
	 * The recipe is read back first because `set_reading` replaces the whole
	 * Reading: the amount and the Unit have just been worked out of the new
	 * line by the Core, and sending the pointer without them would clear them.
	 * A Component with no quantity is the WHOLE of the inner recipe (ADR 0008),
	 * so `500 g pizza dough` losing its 500 would quietly double the dough
	 * rather than look broken.
	 *
	 * Only the lines whose pointer actually differs from what is now saved are
	 * sent. A line nobody touched already carries its pointer across the save
	 * on its own, so the ordinary edit of a recipe with a Component in it makes
	 * no requests here at all.
	 */
	async function attachNamedRecipes(branch: string): Promise<void> {
		const wanted = keptLines().map((row) => row.namedRecipe?.lineageId ?? null);
		// The common case by a distance: no line on this recipe names another
		// one, and none did when the screen opened. Nothing to reconcile, and
		// no reason to read the recipe back.
		if (wanted.every((pointer) => pointer === null) && namedAtOpening.size === 0) return;

		const recipe = await kamosu.getRecipe({ branch_id: branch });
		const saved = recipe.versions.at(-1)?.readings ?? [];
		for (const [index, pointer] of wanted.entries()) {
			const was = saved[index] ?? null;
			if ((was?.lineage_id ?? null) === pointer) continue;
			await kamosu.setReading({
				branch_id: branch,
				line_index: index,
				amount: was?.amount ?? null,
				unit: was?.unit ?? null,
				// The two are exclusive and the Core refuses a Reading claiming
				// to be both, so naming a Recipe puts the Food down.
				//
				// Un-naming one does NOT pick a Food back up, and cannot: a
				// Component's Reading never held one, and reading the line again
				// is the Core's job and not this screen's (ADR 0021, ADR 0036).
				// The line keeps its quantity and loses only the pointer, which
				// leaves it reading exactly as written — and what it is can be
				// typed into the corrector on the reading page, where every
				// other part of a Reading is corrected.
				target: pointer === null ? (was?.target ?? null) : null,
				lineage_id: pointer,
			});
		}
	}

	function drafted() {
		const amount = yieldAmount.trim();
		const noun = yieldNoun.trim();
		const figure = kcal(calories);
		return {
			branch_id: branchId,
			title: title.trim(),
			// A field holding nothing is no part of the fingerprint (ADR 0038),
			// so nothing here has to invent a default to keep ids still.
			yield: amount === '' || noun === '' ? null : { amount, noun },
			prep_time_minutes: minutes(prep),
			cook_time_minutes: minutes(cook),
			note: orNothing(note),
			main_photo: mainPhoto,
			source:
				sourceText.trim() === '' ? null : { text: sourceText.trim(), link: orNothing(sourceLink) },
			// The figure and what it counts, or nothing at all (#84). An empty
			// number is no figure, whatever the basis beside it says — and a
			// field holding nothing is no part of the fingerprint (ADR 0038).
			nutrition: figure === null ? null : { calories: figure, basis },
			// A row with nothing written in it is not a line of the recipe.
			// The Recipe a line names is no part of the content and never can
			// be: it is a Reading, it is named by no fingerprint, and putting
			// it here would move the id of every recipe holding one (ADR 0004,
			// ADR 0038). It is attached after the save instead.
			ingredients: keptLines().map((row) => ({ kind: row.kind, text: row.text.trim() })),
			steps: steps
				.filter((row) => row.text.trim() !== '')
				.map((row) => ({ kind: row.kind, text: row.text.trim(), photo: row.photo })),
		};
	}

	async function save() {
		// Belt and braces: both controls are already disabled while `wrong` is
		// set, and the reason is on screen beside them.
		if (wrong) return;
		saving = true;
		failed = undefined;
		try {
			const answered = await kamosu.saveRecipeVersion({
				...drafted(),
				...(versionName.trim() === '' ? {} : { name: versionName.trim() }),
				...(changeNote.trim() === '' ? {} : { change_note: changeNote.trim() }),
				...(savingInto ? { kitchen_id: savingInto.id } : {}),
			});
			// The Version has landed. Whatever happens to the pointers now, it
			// has landed — so a failure here is reported beside the save rather
			// than as one, and never as an error that hides what did work.
			let named = true;
			try {
				await attachNamedRecipes(answered.branch_id);
			} catch (error) {
				if (!(error instanceof OperationError)) throw error;
				named = false;
			}
			asking = false;
			saving = false;
			onSaved({
				branch_id: answered.branch_id,
				collapsed: answered.collapsed,
				copied: answered.copied,
				named,
			});
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
			saving = false;
			asking = false;
		}
	}

	/**
	 * A field that grows with what is in it. An attachment rather than an
	 * action because it re-runs when the state it reads changes, which is what
	 * makes a pasted or moved line size itself without a second mechanism.
	 */
	const grows = (text: string): Attachment<HTMLTextAreaElement> => {
		return (node) => {
			const fit = () => {
				node.style.height = 'auto';
				node.style.height = `${node.scrollHeight}px`;
			};
			// Re-runs whenever the text changes, which is what sizes a field as
			// it is typed into, pasted into, or moved.
			void text;
			fit();

			// Measuring once is not enough. At the moment a row mounts the page
			// has not settled, so a line that will wrap to three measures as one
			// and the field stays a line tall with its own words hidden inside
			// it — which is what happened on the real 21-line Dan Dan Noodles
			// the first time this screen was opened. Watching the field's WIDTH
			// catches that, and the later ones a rotated phone and a font
			// arriving late both cause.
			//
			// Width alone, deliberately: fitting changes the height, and a
			// height-sensitive observer would answer its own change for ever.
			if (typeof ResizeObserver === 'undefined') return;
			let was = -1;
			const watch = new ResizeObserver((entries) => {
				const wide = entries[0]?.contentRect.width ?? -1;
				if (wide === was) return;
				was = wide;
				fit();
			});
			watch.observe(node);
			return () => watch.disconnect();
		};
	};

	const FIELD =
		'block w-full resize-none rounded-sm border border-rule bg-card px-2 py-1 text-line text-ink';
	const HEADING_FIELD =
		'block w-full resize-none rounded-sm border border-rule bg-card px-2 py-1 text-label text-ink-2 uppercase';
	const SMALL = 'min-h-12 w-full rounded-sm border border-rule bg-card px-3 text-body text-ink';
	const QUIET = 'rounded-sm border border-rule px-2 py-1 text-read text-ink-2';
	/**
	 * The way in to the library, in matcha — the colour of a Reading that points
	 * at a recipe everywhere else (#50). `Correcting.svelte` wears the same one
	 * on the same act, which is what "one control on both screens" means (#83).
	 */
	const MATCHA = 'rounded-sm border border-support-2 px-2 py-1 text-read text-support-2';
</script>

<div class="mx-auto max-w-2xl pb-tabbar">
	<!--
		The bar that says you are writing. It stays at the top of the page
		rather than following the scroll: this is one page, and a bar pinned
		over it would be the beginning of the second screen A refused.
	-->
	<div class="flex items-center gap-2 border-b border-rule px-gutter py-2">
		<button type="button" class="text-body text-ink-2" onclick={onCancel}>
			{m.write_cancel()}
		</button>
		<span class="flex-1 text-center text-label text-ink-2 uppercase">{m.write_editing()}</span>
		<!--
			The way in to a paste, once the page holds something (#83). Pasting
			over a recipe REPLACES it, so it does not sit in the open beside
			*Add a line*; it sits under `⋯`, and the sheet says what it replaces
			before it does it.
		-->
		{#if holdsSomething}
			<div class="relative">
				<button
					type="button"
					aria-label={m.write_paste_more()}
					aria-expanded={moreOpen}
					class="px-1 text-body text-ink-2"
					onclick={() => (moreOpen = !moreOpen)}
				>
					⋯
				</button>
				{#if moreOpen}
					<div class="w-56 absolute end-0 z-30 mt-1 border border-rule bg-card p-1 text-start">
						<button
							type="button"
							class="block w-full px-2 py-2 text-start text-body text-accent"
							onclick={openPaste}
						>
							{m.write_paste_offer()}
						</button>
					</div>
				{/if}
			</div>
		{/if}
		<button
			type="button"
			class="text-body font-medium {kitchensKnown && !wrong ? 'text-accent' : 'text-ink-2'}"
			disabled={saving || !kitchensKnown || Boolean(wrong)}
			onclick={() => (asking = true)}
		>
			{saving ? m.write_saving() : m.write_save()}
		</button>
	</div>

	<!--
		And in the open on a recipe that holds nothing yet, which is when you
		actually have one as text and there is nothing to lose by pasting it.
	-->
	{#if !holdsSomething}
		<div class="border-b border-rule px-gutter py-2">
			<button type="button" class="{QUIET} w-full text-accent" onclick={openPaste}>
				{m.write_paste_offer()}
			</button>
			<p class="pt-2 text-read text-ink-2">{m.write_paste_hint()}</p>
		</div>
	{/if}

	<!-- The hero, and the title typed onto it — #81's layout, made writable. -->
	<div class="relative overflow-hidden">
		{#if mainPhoto}
			<img
				src="/api/photographs/{mainPhoto}/page"
				alt=""
				class="block w-full object-cover"
				style="height: var(--hero-h)"
			/>
			<div class="pointer-events-none absolute inset-x-0 bottom-0 wash"></div>
		{:else}
			<Cover {lineageId} {title} band={false} />
		{/if}
		<div class="absolute inset-x-0 bottom-0 px-gutter pt-8 pb-4">
			<label class="block">
				<span class="sr-only">{m.write_title_label()}</span>
				<textarea
					bind:value={title}
					rows="1"
					{@attach grows(title)}
					class="block w-full resize-none rounded-sm border border-on-accent bg-card px-2 py-1 font-display text-title font-semibold text-ink"
				></textarea>
			</label>
		</div>
	</div>
	<div class="flex flex-wrap items-center gap-2 border-b border-rule px-gutter py-2">
		<label class="{QUIET} cursor-pointer text-accent">
			{mainPhoto ? m.write_photo_change() : m.write_photo_add()}
			<input
				type="file"
				accept="image/*"
				class="hidden"
				onchange={(event) => pick(event, (name) => (mainPhoto = name))}
			/>
		</label>
		{#if mainPhoto}
			<button type="button" class="{QUIET} text-support" onclick={() => (mainPhoto = null)}>
				{m.write_photo_remove()}
			</button>
		{:else}
			<span class="text-read text-ink-2">{m.write_photo_cover()}</span>
		{/if}
	</div>
	{#if photoFailed}
		<p class="px-gutter pt-2 text-read text-support" role="alert">{m.write_photo_failed()}</p>
	{/if}

	<!-- The facts: #81's one strip of three cells, each a field, each able to
	     stay empty. The reading page's idiom, not a second one. -->
	<div class="flex border-y border-rule">
		<label class="flex-1 border-l border-rule p-2 first:border-l-0">
			<span class="block text-label text-ink-2 uppercase">{m.recipe_min_prep()}</span>
			<input
				bind:value={prep}
				inputmode="numeric"
				aria-label={m.write_prep_label()}
				class={SMALL}
			/>
		</label>
		<label class="flex-1 border-l border-rule p-2 first:border-l-0">
			<span class="block text-label text-ink-2 uppercase">{m.recipe_min_cook()}</span>
			<input
				bind:value={cook}
				inputmode="numeric"
				aria-label={m.write_cook_label()}
				class={SMALL}
			/>
		</label>
		<div class="flex flex-1 flex-col gap-1 border-l border-rule p-2 first:border-l-0">
			<input
				bind:value={yieldAmount}
				inputmode="numeric"
				aria-label={m.write_yield_amount()}
				class={SMALL}
			/>
			<input bind:value={yieldNoun} aria-label={m.write_yield_noun()} class={SMALL} />
		</div>
	</div>

	<!-- Ingredients. -->
	<h2 class="mx-gutter mt-8 mb-2 font-display text-label font-semibold text-accent uppercase">
		{m.recipe_ingredients()}
	</h2>
	<ul bind:this={linesEl}>
		{#each lines as row, index (row.id)}
			<li
				data-row
				class="flex items-start gap-2 border-b border-rule px-gutter py-2 {dragging?.list ===
					'lines' && dragging.index === index
					? 'bg-card'
					: ''}"
			>
				{@render handle('lines', index)}
				{#if row.kind === 'ingredient'}
					<span class="ingredient-marker shrink-0 bg-accent" aria-hidden="true"></span>
				{/if}
				<div class="min-w-0 flex-1">
					<textarea
						bind:value={row.text}
						rows="1"
						{@attach grows(row.text)}
						aria-label={row.kind === 'section'
							? m.write_heading_aria()
							: m.write_line_aria({ number: lineNumbers[index] ?? 0 })}
						class={row.kind === 'section' ? HEADING_FIELD : FIELD}
						onfocus={() => (cursor = { list: 'lines', index })}
						onkeydown={(event) => onKey(event, 'lines', index)}></textarea>
					<div class="flex flex-wrap items-center gap-2 pt-1">
						<!--
							THE ONE PART OF THE READING THAT IS EDITED HERE (#87):
							whether this line names a Recipe rather than a Food.
							Matcha, which is the colour of a Reading that points at a
							recipe everywhere else (#50), and small, because most lines
							are not recipes and this must not shout on all of them.

							A heading carries no Reading at all, so it carries no
							control: `set_reading` refuses one on a section, and a
							button that always fails is worse than no button.
						-->
						{#if row.kind === 'ingredient'}
							{#if row.namedRecipe}
								<button type="button" class={MATCHA} onclick={() => (picking = row.id)}>
									{m.write_line_names({
										title: row.namedRecipe.title ?? m.write_line_recipe(),
									})}
								</button>
								<button type="button" class={QUIET} onclick={() => (row.namedRecipe = null)}>
									{m.write_line_not_recipe()}
								</button>
							{:else}
								<button type="button" class={MATCHA} onclick={() => (picking = row.id)}>
									{m.write_line_recipe()}
								</button>
							{/if}
						{/if}
						<button
							type="button"
							class="{QUIET} text-support"
							onclick={() => remove('lines', index)}
						>
							{m.write_remove()}
						</button>
					</div>
				</div>
			</li>
		{/each}
	</ul>
	{#if lines.length === 0}
		<p class="px-gutter py-2 text-read text-ink-2">{m.write_empty_ingredients()}</p>
	{/if}
	<div class="flex gap-2 px-gutter pt-2">
		<button type="button" class="{QUIET} flex-1 text-accent" onclick={() => addLine('ingredient')}>
			{m.write_add_line()}
		</button>
		<button type="button" class="{QUIET} flex-1 text-accent" onclick={() => addLine('section')}>
			{m.write_add_heading()}
		</button>
	</div>
	<p class="px-gutter pt-2 text-read text-ink-2">{m.write_add_where()}</p>

	<!--
		The Nutrition figure, typed where it is read (#84): at the foot of the
		Ingredients, which is the treatment Aurélien chose on 21 September 2026
		for both surfaces. Reading and writing share it rather than each making
		their own choice.

		The basis sits beside the number rather than being a setting elsewhere,
		because it is part of the figure. Kamosu never works the number out from
		the lines above — a plausible-but-wrong calorie count is worse than an
		empty field — and the hint says so, since a box under a list of
		ingredients otherwise looks like one it would fill in.
	-->
	<div class="mt-4 flex items-end gap-2 border-t border-rule px-gutter pt-3">
		<label class="flex-1">
			<span class="block text-label text-ink-2 uppercase">{m.write_nutrition_label()}</span>
			<!--
				`decimal` rather than `numeric`: a source page stating 154.5 per
				100 g is stating a figure Kamosu does not round, and a
				digits-only keypad has no separator to type it with.
			-->
			<input bind:value={calories} inputmode="decimal" class={SMALL} />
		</label>
		<!--
			No visible label on the basis: its own two options say what it is,
			beside a box that says Calories. The name is there for anyone not
			reading it off the screen.
		-->
		<select bind:value={basis} aria-label={m.write_nutrition_basis()} class="{SMALL} flex-1">
			<option value="per_serving">{m.write_nutrition_per_serving()}</option>
			<option value="per_100g">{m.write_nutrition_per_100g()}</option>
		</select>
	</div>
	<p class="px-gutter pt-2 text-read text-ink-2">{m.write_nutrition_hint()}</p>

	<!-- The Method, underneath the ingredients on the same page. -->
	<h2 class="mx-gutter mt-8 mb-2 font-display text-label font-semibold text-accent uppercase">
		{m.recipe_method()}
	</h2>
	<ol bind:this={stepsEl}>
		{#each steps as row, index (row.id)}
			<li
				data-row
				class="flex items-start gap-2 border-b border-rule px-gutter py-2 {dragging?.list ===
					'steps' && dragging.index === index
					? 'bg-card'
					: ''}"
			>
				{@render handle('steps', index)}
				{#if row.kind === 'step'}
					<span class="w-6 shrink-0 pt-1 font-display text-line font-semibold text-accent">
						{stepNumbers[index]}
					</span>
				{/if}
				<div class="min-w-0 flex-1">
					<textarea
						bind:value={row.text}
						rows="1"
						{@attach grows(row.text)}
						aria-label={row.kind === 'section'
							? m.write_heading_aria()
							: m.write_step_aria({ number: stepNumbers[index] ?? 0 })}
						class={row.kind === 'section' ? HEADING_FIELD : FIELD}
						onfocus={() => (cursor = { list: 'steps', index })}
						onkeydown={(event) => onKey(event, 'steps', index)}></textarea>
					<div class="flex flex-wrap items-center gap-2 pt-1">
						{#if row.kind === 'step'}
							<label class="{QUIET} cursor-pointer text-accent">
								{row.photo ? m.write_photo_change() : m.write_step_photo()}
								<input
									type="file"
									accept="image/*"
									class="hidden"
									onchange={(event) => pick(event, (name) => (row.photo = name))}
								/>
							</label>
							{#if row.photo}
								<button
									type="button"
									class="{QUIET} text-support"
									onclick={() => (row.photo = null)}
								>
									{m.write_step_photo_remove()}
								</button>
							{/if}
						{/if}
						<button
							type="button"
							class="{QUIET} text-support"
							onclick={() => remove('steps', index)}
						>
							{m.write_remove()}
						</button>
					</div>
				</div>
			</li>
		{/each}
	</ol>
	{#if steps.length === 0}
		<p class="px-gutter py-2 text-read text-ink-2">{m.write_empty_steps()}</p>
	{/if}
	<div class="flex gap-2 px-gutter pt-2">
		<button type="button" class="{QUIET} flex-1 text-accent" onclick={() => addStep('step')}>
			{m.write_add_step()}
		</button>
		<button type="button" class="{QUIET} flex-1 text-accent" onclick={() => addStep('section')}>
			{m.write_add_heading()}
		</button>
	</div>
	<p class="px-gutter pt-2 text-read text-ink-2">{m.write_step_rule()}</p>

	<!-- The note, and where the recipe came from. -->
	<label class="mt-6 block px-gutter">
		<span class="block text-label text-ink-2 uppercase">{m.write_note_label()}</span>
		<textarea
			bind:value={note}
			rows="3"
			class="mt-1 block w-full rounded-sm border border-rule bg-card p-2 text-body text-ink"
		></textarea>
	</label>
	<label class="mt-4 block px-gutter">
		<span class="block text-label text-ink-2 uppercase">{m.write_source_text()}</span>
		<input bind:value={sourceText} class="mt-1 {SMALL}" />
	</label>
	<label class="mt-2 block px-gutter">
		<span class="block text-label text-ink-2 uppercase">{m.write_source_link()}</span>
		<input bind:value={sourceLink} type="url" placeholder="https://" class="mt-1 {SMALL}" />
	</label>

	{#if failed}
		<p class="mt-4 px-gutter text-read text-support" role="alert">{failed}</p>
	{/if}

	<div class="px-gutter py-6">
		{#if wrong}
			<p class="mb-2 text-read text-support" role="alert">{wrong}</p>
		{/if}
		<button
			type="button"
			class="block w-full p-4 text-center font-display text-body {kitchensKnown && !wrong
				? forking
					? 'bg-support text-on-accent'
					: 'bg-accent text-on-accent'
				: 'border border-rule text-ink-2'}"
			disabled={saving || !kitchensKnown || Boolean(wrong)}
			onclick={() => (asking = true)}
		>
			{#if !kitchensKnown}
				{kitchensFailed ? m.write_outcome_unknown() : m.loading()}
			{:else}
				{forking ? m.write_do_fork() : m.write_do_save()}
			{/if}
		</button>
	</div>
</div>

<!--
	The handle. It is what you drag, and the list rearranges under it as you go.
	`touch-action: none` is what stops the phone reading the drag as a scroll
	and taking the gesture away before this ever sees it.

	It is a real button, so it takes focus, and the arrow keys move the row one
	place at a time for anybody who is not dragging anything.
-->
<!--
	The library, raised over the page you are standing on (#87) — never a route,
	which on this screen would throw away everything typed since the last save.
-->
{#if pickingFor}
	<ComponentPicker
		line={pickingFor.text}
		onChoose={(chosen) => {
			if (pickingFor) pickingFor.namedRecipe = chosen;
			picking = null;
		}}
		onCancel={() => (picking = null)}
	/>
{/if}

{#snippet handle(list: 'lines' | 'steps', index: number)}
	<button
		type="button"
		data-handle
		aria-label={m.write_move()}
		class="shrink-0 cursor-grab touch-none px-1 pt-1 text-body text-ink-2 select-none"
		onpointerdown={(event) => startDrag(list, index, event)}
		onkeydown={(event) => onHandleKey(event, list, index)}
	>
		⠿
	</button>
{/snippet}

<!--
	THE SHEET THAT SAYS WHAT KAMOSU MADE OF A PASTE, BEFORE IT LANDS (#94).

	It is the whole reason the parser is allowed to guess at all. The boundary
	is found exactly on 77.5% of the real 86-recipe export and within one line
	on 95%, so the ordinary correction is one line — hence the two buttons —
	and the occasional one is a long way, hence every line being able to take
	the split itself.

	Moving it re-splits an answer this screen already holds. No line is read a
	second time and no request is made.
-->
{#if pasteOpen}
	<div class="fixed inset-0 z-40 bg-accent/40"></div>
	<div
		class="py-5 fixed inset-x-0 bottom-0 z-50 mx-auto max-h-[85vh] max-w-2xl overflow-y-auto bg-ground px-gutter pb-safe"
		role="dialog"
		aria-modal="true"
		aria-label={m.write_paste_offer()}
	>
		<p class="text-label text-ink-2 uppercase">{m.write_paste_offer()}</p>

		{#if !pasted}
			<p class="mt-2 text-read text-ink-2">{m.write_paste_hint()}</p>
			<textarea
				bind:value={pasteText}
				rows="10"
				aria-label={m.write_paste_label()}
				class="mt-3 block w-full rounded-sm border border-rule bg-card p-2 text-body text-ink"
			></textarea>
			{#if pasteFailed}
				<p class="mt-2 text-read text-support" role="alert">{pasteFailed}</p>
			{/if}
			<button
				type="button"
				class="mt-3 block w-full bg-accent p-4 text-center font-display text-body text-on-accent"
				disabled={reading}
				onclick={readPaste}
			>
				{reading ? m.write_paste_reading() : m.write_paste_read()}
			</button>
		{:else}
			<!-- What it made of it: the title, the counts, and the split drawn. -->
			<p class="mt-2 text-body">
				{pasted.title === null || pasted.title.trim() === ''
					? m.write_paste_no_title()
					: m.write_paste_found_title({ title: pasted.title })}
			</p>
			<p class="mt-1 text-read text-ink-2">
				{m.write_paste_made({ ingredients: countOf(above), steps: countOf(below) })}
				{#if sectionsIn(pasted.lines) > 0}
					{m.write_paste_headings({ headings: sectionsIn(pasted.lines) })}
				{/if}
			</p>

			<div class="mt-3 flex items-center gap-2">
				<span class="flex-1 text-label text-ink-2 uppercase">{m.write_paste_boundary()}</span>
				<button
					type="button"
					class={QUIET}
					disabled={boundary === 0}
					onclick={() => (boundary = Math.max(0, boundary - 1))}
				>
					{m.write_paste_earlier()}
				</button>
				<button
					type="button"
					class={QUIET}
					disabled={boundary >= pastedLines}
					onclick={() => (boundary = Math.min(pastedLines, boundary + 1))}
				>
					{m.write_paste_later()}
				</button>
			</div>

			<!--
				Every line, in the order it was pasted, on the side it landed.
				Each one takes the split itself, so a boundary eight lines out is
				one tap rather than eight — and the row is a real button, so it
				is reachable from a keyboard.
			-->
			<h3 class="mt-4 font-display text-label font-semibold text-accent uppercase">
				{m.recipe_ingredients()}
			</h3>
			{@render pasteRows(above, 0, m.write_empty_ingredients())}
			<h3
				class="mt-3 border-t border-rule pt-3 font-display text-label font-semibold text-accent uppercase"
			>
				{m.recipe_method()}
			</h3>
			{@render pasteRows(below, boundary, m.write_empty_steps())}

			<p class="mt-4 text-read {holdsSomething || replacesTitle ? 'text-support' : 'text-ink-2'}">
				{#if holdsSomething}
					{m.write_paste_replaces({
						ingredients: lines.filter((row) => row.kind === 'ingredient').length,
						steps: steps.filter((row) => row.kind === 'step').length,
					})}
				{/if}
				{#if replacesTitle}
					{m.write_paste_replaces_title({ title: title.trim() })}
				{/if}
				{#if !holdsSomething && !replacesTitle}
					{m.write_paste_fresh()}
				{/if}
			</p>
			<button
				type="button"
				class="mt-3 block w-full p-4 text-center font-display text-body text-on-accent {holdsSomething ||
				replacesTitle
					? 'bg-support'
					: 'bg-accent'}"
				onclick={usePaste}
			>
				{m.write_paste_use()}
			</button>
		{/if}
		<button
			type="button"
			class="mt-2 block w-full border border-rule p-4 text-center font-display text-body text-ink-2"
			onclick={() => (pasteOpen = false)}
		>
			{m.write_back()}
		</button>
	</div>
{/if}

{#snippet pasteRows(rows: ReadPastedRecipeOutput['lines'], from: number, empty: string)}
	{#if rows.length === 0}
		<p class="py-1 text-read text-ink-2">{empty}</p>
	{/if}
	<ul>
		{#each rows as row, index (from + index)}
			<li>
				<button
					type="button"
					aria-label={m.write_paste_start_here()}
					class="block w-full border-b border-rule py-1 text-start {row.kind === 'section'
						? 'text-label text-ink-2 uppercase'
						: 'text-line text-ink'}"
					onclick={() => (boundary = from + index)}
				>
					{row.text}
				</button>
			</li>
		{/each}
	</ul>
{/snippet}

<!--
	The sheet that says which of the two saves this is, before it happens
	(#54). It names the recipe and the Kitchen, because *Save* and *Save* are
	the same word for two different acts.
-->
{#if asking}
	<div class="fixed inset-0 z-40 bg-accent/40"></div>
	<div
		class="py-5 fixed inset-x-0 bottom-0 z-50 mx-auto max-h-[78vh] max-w-2xl overflow-y-auto bg-ground px-gutter pb-safe"
		role="dialog"
		aria-modal="true"
		aria-label={forking ? m.write_will_fork() : m.write_will_save()}
	>
		<p class="text-label text-ink-2 uppercase">
			{forking ? m.write_will_fork() : m.write_will_save()}
		</p>
		<p class="mt-2 text-body">
			{forking
				? m.write_said_fork({ title: title.trim(), kitchen: savingInto?.name ?? '' })
				: m.write_said_save({ title: title.trim(), kitchen: savingInto?.name ?? '' })}
		</p>
		<label class="mt-4 block">
			<span class="block text-label text-ink-2 uppercase">
				{m.write_name_label()} · {m.write_optional()}
			</span>
			<input bind:value={versionName} class="mt-1 {SMALL}" />
		</label>
		<label class="mt-3 block">
			<span class="block text-label text-ink-2 uppercase">
				{m.write_changed_label()} · {m.write_optional()}
			</span>
			<input bind:value={changeNote} class="mt-1 {SMALL}" />
		</label>
		<p class="mt-1 text-read text-ink-2">{m.write_changed_now()}</p>
		<button
			type="button"
			class="mt-4 block w-full p-4 text-center font-display text-body text-on-accent {forking
				? 'bg-support'
				: 'bg-accent'}"
			disabled={saving}
			onclick={save}
		>
			{forking ? m.write_do_fork() : m.write_do_save()}
		</button>
		<button
			type="button"
			class="mt-2 block w-full border border-rule p-4 text-center font-display text-body text-ink-2"
			onclick={() => (asking = false)}
		>
			{m.write_back()}
		</button>
	</div>
{/if}
