<!--
	Writing a recipe (#83). The other half of #81, which only reads.

	THE PAGE IS THE PAGE. Direction A was chosen on 20 September 2026,
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

	One argument was put for the separate screen and it was wrong:
	that a long list gets more room there. The hero scrolls away, so it does
	not. That mistake is recorded on #83 so nobody re-derives it.

	AN INGREDIENT LINE IS ONE FREE-TEXT FIELD (ADR 0002, #43). Never an amount
	box, a unit box and a food box. `2 poignées de farine, environ` goes in and
	comes out as typed, and nothing here rewrites a character of it. What
	Kamosu understands OF that line — the amount, the Unit, the Food — is the
	Reading, and it is corrected on the reading page and is no part of this
	screen (ADR 0021).

	ONE PART OF THE READING IS HERE, AND ONLY ONE (#87): whether the line names
	a RECIPE rather than a Food. It was settled on 20 September 2026 that the
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
	control that was approved, so a two-step *move, then choose where* is not
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
	appending (`src/core/recipes.rs`), and a save that silently made no Thread
	entry reads as a save that did not happen. And a Copy lands on a DIFFERENT
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
	import SheetFrame from '$lib/SheetFrame.svelte';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { usePhotograph } from '$lib/api/upload';
	import type { GetRecipeOutput, ReadPastedRecipeOutput } from '$lib/api/catalogue';
	import type { Nutrition } from './divergence';
	import { languageName, type WrittenLanguage } from '$lib/language';
	import { hostOf } from '$lib/source';
	import { copySaid, type Whose } from '$lib/cookbook';
	import Cover from '$lib/cover/Cover.svelte';
	import { FilePlace, isPicture } from '$lib/drop.svelte';
	import { DRAWN, drawnFor } from '$lib/drop-drawings';
	import DropCover from '$lib/DropCover.svelte';
	import DropHere from '$lib/DropHere.svelte';
	import ComponentPicker, { type NamedRecipe } from './ComponentPicker.svelte';
	import PasteCheck from '$lib/PasteCheck.svelte';
	import { drafted as splitPaste, readPasted } from '$lib/pasted.svelte';
	import { joined, split } from '$lib/duration';

	type Content = GetRecipeOutput['versions'][number]['content'];

	interface Props {
		branchId: string;
		lineageId: string;
		/**
		 * Whether a save lands on this Branch, and why not where it does not:
		 * the Core's answer, from whose Cookbook the Branch is in and whether
		 * it arrived there (ADR 0041, #132). Where it does not, a save starts
		 * the cook's own Branch in their own Cookbook.
		 */
		whose: Whose;
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
		/**
		 * **Translating rather than editing** (#106, ADR 0006): the Language
		 * the draft below is being rendered into. Set, the save calls
		 * `start_translation` and makes an ordinary Branch of the same Lineage
		 * in that Language; the recipe being translated is not touched.
		 *
		 * The screen is otherwise the same screen, which is the point. A
		 * translation is a recipe, so it is written where recipes are written —
		 * there is no translation editor, and the draft starts as the source's
		 * own words because replacing them in place is what translating is.
		 */
		translatingInto?: WrittenLanguage;
		onCancel: () => void;
		/**
		 * A Version landed. The page re-reads and this screen closes, so what
		 * happened is handed UP rather than said here: a line drawn by a
		 * component that is about to be destroyed is a line nobody reads.
		 *
		 * `branch_id` is the Branch the Version is on, which after a Copy is
		 * the NEW one rather than the one that was open. `varied` is the name
		 * of a variation this save started beside the recipe (#131), which the
		 * page says on the one it lands on.
		 *
		 * `named` is false where the Version landed but a line naming another
		 * recipe could not be marked (#87). It travels with the rest for the
		 * same reason they do: the screen that knows is closing.
		 *
		 * `language_offer` is the Language this text reads as where that
		 * disagrees with the one the recipe carries — an offer to be put to
		 * the cook and never a change (#106, ADR 0006). It travels up with the
		 * rest because it belongs to THIS save and to no later read: nothing
		 * stores it, and `get_recipe` does not answer it. A Translation's
		 * first save answers none, since it declared its own Language.
		 */
		onSaved: (landed: {
			branch_id: string;
			collapsed: boolean;
			copied: boolean;
			named: boolean;
			language_offer: string | null;
			varied?: string;
		}) => void;
	}

	let {
		branchId,
		lineageId,
		whose,
		content,
		components = [],
		translatingInto,
		onCancel,
		onSaved,
	}: Props = $props();

	/** Whether this screen is rendering the recipe into another Language rather than editing it. */
	const translating = $derived(translatingInto !== undefined);

	const kamosu = useKamosu();
	const sendPhotograph = usePhotograph();
	const uid = $props.id();
	/** Ties the Cook box to the line beneath the strip that says what goes in it (#118). */
	const cookHintId = `${uid}-cook-hint`;

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
	/**
	 * Each time is two boxes, hours and minutes (#175): a 9-hour prove is `9`
	 * in the hours box, and nobody works out that it is 540 minutes. What is
	 * saved is still whole minutes, joined on the way out.
	 */
	// svelte-ignore state_referenced_locally
	const prep = $state(split(content.prep_time_minutes));
	// svelte-ignore state_referenced_locally
	const cook = $state(split(content.cook_time_minutes));
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
	// whose Unit vocabulary is `units.rs`'s in three Languages. What it made is
	// drawn by `PasteCheck`, which holds why the guess is never applied
	// silently.
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
		reading = true;
		pasteFailed = undefined;
		try {
			const answer = await readPasted(kamosu, pasteText);
			if (typeof answer === 'string') {
				pasteFailed = answer;
			} else {
				pasted = answer;
				boundary = answer.boundary;
			}
		} finally {
			reading = false;
		}
	}

	/** Put the paste onto the page, split where the sheet last put the method's start. */
	function usePaste() {
		if (!pasted) return;
		const draft = splitPaste(pasted, boundary);
		if (draft.title !== null) title = draft.title;
		// What the paste said about the recipe goes to its note (#176), after
		// whatever the note already holds rather than over it: nothing
		// warned that it would be replaced, so nothing is.
		if (draft.note !== null) note = note.trim() === '' ? draft.note : `${note}\n\n${draft.note}`;
		lines = draft.ingredients.map((row) => ({
			id: id(),
			...row,
			// A pasted line naming a recipe on the shelf is an ordinary
			// Ingredient Line until somebody says otherwise (ADR 0008).
			namedRecipe: null,
		}));
		steps = draft.steps.map((row) => ({ id: id(), ...row, photo: null }));
		cursor = null;
		pasteOpen = false;
		pasted = null;
	}

	// ---- which of the two saves this is ---------------------------------

	/**
	 * A Copy, not a Version: the recipe is not the cook's to change — a
	 * Kitchen-mate's, or one that arrived from elsewhere — so the save starts
	 * a Branch of their own in their own Cookbook (ADR 0041). The Core says
	 * which on the recipe itself, so there is nothing to ask first and
	 * nothing to guess.
	 */
	const forking = $derived(!whose.writes);

	/**
	 * **Onto the recipe, or beside it** (#131, screen choice 3). Saving your
	 * own recipe may instead start a variation of it — a second one in your
	 * Cookbook, named, the first left exactly as it was. Offered in the sheet
	 * that already asks what a save is, never on a recipe that is not yours,
	 * where every save starts one of your own anyway.
	 */
	let beside = $state(false);
	let variationName = $state('');

	/** The sheet states the outcome before the save, so every save opens it. */
	function openSheet() {
		beside = false;
		variationName = '';
		asking = true;
	}

	/**
	 * **What this save is, in words** — the one place the three acts are told
	 * apart, so the bar, the button and the sheet cannot drift into saying
	 * three different things about the same tap.
	 *
	 * Translating is checked FIRST and is never a fork: `start_translation`
	 * makes a Branch of the same Lineage in your own Cookbook, whoever writes
	 * the recipe being translated, so #54's two sentences are both wrong for it.
	 */
	const act = $derived.by(() => {
		if (translatingInto) {
			const language = languageName(translatingInto);
			return {
				called: m.write_translation_heading({ language }),
				said: m.write_translation_what({ language }),
				does: m.write_translation_save(),
				/** Beni is reserved for a fork, and a Translation is not one. */
				grave: false,
			};
		}
		if (forking) {
			return {
				called: m.write_will_fork(),
				said: copySaid(whose, title.trim()),
				does: m.write_do_fork(),
				grave: true,
			};
		}
		if (beside) {
			return {
				called: m.write_will_save(),
				said: m.write_where_beside_said({ title: title.trim() }),
				does: m.write_variation_do(),
				grave: false,
			};
		}
		return {
			called: m.write_will_save(),
			said: m.write_said_save({ title: title.trim() }),
			does: m.write_do_save(),
			grave: false,
		};
	});

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

	/** The box the Ingredient Lines stay in view in, on a roomy window (#198). */
	let linesStayEl = $state<HTMLElement | undefined>(undefined);

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
		const by = above < 0 ? Math.max(-20, above / 3) : below < 0 ? Math.min(20, -below / 3) : 0;
		if (by !== 0 && !scrollTheLines(by)) window.scrollBy(0, by);
		settle();
		ticking = requestAnimationFrame(autoScroll);
	}

	/**
	 * Where the window is roomy the Ingredient Lines stay in view in a box that
	 * scrolls on its own once it is taller than the window (#198). A line
	 * dragged to the edge scrolls that box, and the page is left where it is:
	 * the box stays put while the page moves, so scrolling the page would move
	 * nothing under the pointer and run off to the foot of the Method. The
	 * page is scrolled only to bring the rest of the box into the window.
	 *
	 * Everywhere else the box scrolls nothing, this answers no, and the page
	 * scrolls as it always did.
	 */
	function scrollTheLines(by: number) {
		const box = dragging?.list === 'lines' ? linesStayEl : undefined;
		if (!box || getComputedStyle(box).overflowY !== 'auto') return false;
		const before = box.scrollTop;
		box.scrollBy(0, by);
		if (box.scrollTop !== before) return true;
		const { top, bottom } = box.getBoundingClientRect();
		return by > 0 ? bottom <= window.innerHeight : top >= 0;
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

	/**
	 * A photograph dropped where it goes (ADR 0044, #205): onto the recipe's
	 * own, or onto a Step. Each is what the button there does, through
	 * `takePhoto`, so it says what the button says when the picture cannot be
	 * sent. The recipe's photograph is covered while one is held over it, as
	 * Recipes is; a Step is too small for that and is framed instead.
	 */
	const mainPlace = new FilePlace(isPicture, (file) => {
		void takePhoto(file, (name) => (mainPhoto = name));
	});
	const MAIN_SAYS = {
		ok: [m.drop_photo_recipe, m.drop_photo_recipe_then],
		unsure: [m.drop_photo_recipe_unsure, m.drop_photo_recipe_then],
		no: [m.drop_recipes_no, m.drop_photo_recipe_takes],
	};

	/** Each Step's place, made the first time the Step is drawn and kept by the row's id. */
	const stepPlaces = new Map<number, FilePlace>();
	function stepPlace(id: number): FilePlace {
		let place = stepPlaces.get(id);
		if (!place) {
			place = new FilePlace(isPicture, (file) => {
				// Found again when the picture lands: the row may have moved since.
				void takePhoto(file, (name) => {
					const row = steps.find((step) => step.id === id);
					if (row) row.photo = name;
				});
			});
			stepPlaces.set(id, place);
		}
		return place;
	}

	// ---- saving ---------------------------------------------------------

	/** A field left blank is a field with nothing in it, never an empty string. */
	const orNothing = (value: string) => (value.trim() === '' ? null : value.trim());
	/** A time's two boxes as whole minutes, or nothing. `wrong` never gets this far. */
	const minutes = (time: { hours: string; minutes: string }) => {
		const total = joined(time.hours, time.minutes);
		return total === 'wrong' ? null : total;
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
	 * and a time is whole hours and minutes. Both were being dropped in silence, which
	 * loses something somebody typed on purpose.
	 */
	const wrong = $derived.by(() => {
		const amount = yieldAmount.trim();
		const noun = yieldNoun.trim();
		if (title.trim() === '') return m.write_needs_title();
		if ((amount === '') !== (noun === '')) return m.write_needs_both_yield();
		for (const [time, label] of [
			[prep, m.write_prep_time()],
			[cook, m.write_cook_time()],
		] as const) {
			if (joined(time.hours, time.minutes) === 'wrong') {
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

	/** The Source as written, or nothing. A link typed with no name is kept and
	 *  named by its host (#152): the name used to be the only thing that made a
	 *  Source, so a link on its own was thrown away without a word. */
	function draftedSource() {
		const link = orNothing(sourceLink);
		const text = orNothing(sourceText) ?? (link === null ? null : (hostOf(link) ?? link));
		return text === null ? null : { text, link };
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
			source: draftedSource(),
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
		// A variation is named, since nothing else tells the two apart: the
		// button waits for one (ADR 0041).
		if (beside && variationName.trim() === '') return;
		saving = true;
		failed = undefined;
		try {
			if (beside) {
				await startVariation();
				return;
			}
			const common = {
				...drafted(),
				...(versionName.trim() === '' ? {} : { name: versionName.trim() }),
				...(changeNote.trim() === '' ? {} : { change_note: changeNote.trim() }),
			};
			// Two Operations, one screen. Translating makes a Branch of the same
			// Lineage carrying its own Language and pointing at the Version of
			// the source it renders; editing writes a Version on this Branch.
			// Neither is a mode of the other, which is why the answers differ in
			// shape and are read apart rather than merged (#106, ADR 0006).
			const answered = translatingInto
				? {
						...(await kamosu.startTranslation({ ...common, language: translatingInto })),
						// A Translation's first Version is its own first Version:
						// it joins nothing and forks nothing, and it declared the
						// Language it is in, so there is no offer to make.
						collapsed: false,
						copied: false,
						language_offer: null,
					}
				: await kamosu.saveRecipeVersion(common);
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
				language_offer: answered.language_offer,
			});
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
			saving = false;
			asking = false;
		}
	}

	/**
	 * **Start a variation, and put what was written on it** (#131). The
	 * variation starts as the recipe exactly as it stands, which is what
	 * `start_variation` makes; the draft is then saved onto the variation,
	 * never onto the recipe it came from. Saved with no change at all, it is
	 * simply the recipe a second time, under its name.
	 */
	async function startVariation() {
		const name = variationName.trim();
		const started = await kamosu.startVariation({ branch_id: branchId, name });
		// Saved whether or not anything changed: the Core mints nothing for
		// content its head already holds, so an unchanged draft costs nothing.
		const saved = await kamosu.saveRecipeVersion({ ...drafted(), branch_id: started.branch_id });
		// The note follows only where the draft made a Version for it to
		// describe (#165). The variation's head is the recipe's own Version,
		// which the note does not describe and the Core would refuse it on; an
		// unchanged variation is the recipe again, and its name says why.
		if (changeNote.trim() !== '' && saved.version_id !== started.head_version_id) {
			await kamosu.saveRecipeVersion({
				...drafted(),
				branch_id: started.branch_id,
				change_note: changeNote.trim(),
			});
		}
		let named = true;
		try {
			await attachNamedRecipes(started.branch_id);
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			named = false;
		}
		asking = false;
		saving = false;
		onSaved({
			branch_id: started.branch_id,
			collapsed: false,
			copied: true,
			named,
			language_offer: null,
			varied: name,
		});
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
	/** A time's hours or minutes box: SMALL, narrower inside, since two share half a phone. */
	const TIME_BOX =
		'min-h-12 w-full min-w-0 rounded-sm border border-rule bg-card px-2 text-body text-ink';
	const QUIET = 'rounded-sm border border-rule px-2 py-1 text-read text-ink-2';
	/**
	 * A tool beside a line or a Step, as a mark and no word (#225): twenty lines
	 * carried forty words of button. The word is the accessible name, and the
	 * title says it under a pointer. Drawn at 32 and tapped at 48 through
	 * `tap-out` (ADR 0044, 10 October 2026), so the tools in a row stand 16
	 * apart.
	 */
	const TOOL = 'tap-out flex h-8 w-8 shrink-0 items-center justify-center rounded-sm border';
	/**
	 * The way in to the library, in matcha — the colour of a Reading that points
	 * at a recipe everywhere else (#50). `Correcting.svelte` wears the same one
	 * on the same act, which is what "one control on both screens" means (#83).
	 */
	const MATCHA = 'rounded-sm border border-support-2 px-2 py-1 text-read text-support-2';
</script>

<div class="mx-auto recipe-page pb-tabbar">
	<!--
		The bar that says you are writing. It stays at the top of the page
		rather than following the scroll: this is one page, and a bar pinned
		over it would be the beginning of the second screen A refused.
	-->
	<div class="flex items-center gap-2 border-b border-rule px-gutter py-2">
		<button type="button" class="text-body text-ink-2" onclick={onCancel}>
			{m.write_cancel()}
		</button>
		<!--
			What this screen is doing, said where it says it is being written on.
			A translation opens on the source's own words, so without this the
			only difference between translating and editing would be the button
			at the foot — and by then you have retyped the recipe.
		-->
		<span class="flex-1 text-center text-label text-ink-2 uppercase">
			{translating ? act.called : m.write_editing()}
		</span>
		<!--
			The way in to a paste, once the page holds something (#83). Pasting
			over a recipe REPLACES it, so it does not sit in the open beside
			*Add a line*; it sits under `⋯`, and the sheet says what it replaces
			before it does it. Its name says so too (#175): "Replace with
			pasted text", not the offer's own words.
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
							{m.write_paste_replace()}
						</button>
					</div>
				{/if}
			</div>
		{/if}
		<button
			type="button"
			class="text-body font-medium {!wrong ? 'text-accent' : 'text-ink-2'}"
			disabled={saving || Boolean(wrong)}
			onclick={openSheet}
		>
			{saving ? m.write_saving() : m.write_save()}
		</button>
	</div>

	<!--
		And in the open on a recipe that holds nothing yet, which is when you
		actually have one as text and there is nothing to lose by pasting it.
		A card with the page's one filled button (#175, option 1):
		it was a quiet outlined button before, easy to miss, and its name said
		"paste" but not that Kamosu sorts the text out for you.
	-->
	{#if !holdsSomething}
		<div class="mx-gutter mt-3 rounded-sm border border-accent bg-card p-4">
			<h2 class="font-display text-body font-semibold text-ink">{m.write_paste_card_title()}</h2>
			<p class="mt-1 text-read text-ink-2">{m.write_paste_card_body()}</p>
			<button
				type="button"
				class="mt-3 block w-full rounded-sm bg-accent p-4 text-center font-display text-body text-on-accent"
				onclick={openPaste}
			>
				{m.write_paste_offer()}
			</button>
		</div>
		<p class="mt-2 mb-3 text-center text-read text-ink-2">{m.write_paste_or_yourself()}</p>
	{/if}

	<!-- The hero, and the title typed onto it — #81's layout, made writable. -->
	<div class="relative overflow-hidden" {@attach mainPlace.listen}>
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
		{#if mainPlace.held}
			<DropCover
				class="absolute inset-0 z-20"
				icon={drawnFor(mainPlace.held, DRAWN.picture)}
				headline={MAIN_SAYS[mainPlace.held][0]()}
				sentence={MAIN_SAYS[mainPlace.held][1]()}
				refuses={mainPlace.held === 'no'}
			/>
		{/if}
	</div>
	<div class="flex flex-wrap items-center gap-2 border-b border-rule px-gutter py-2">
		<label class="{QUIET} cursor-pointer text-accent">
			{mainPhoto ? m.write_photo_change() : m.write_photo_add()}
			<input
				type="file"
				accept="image/*"
				class="hidden"
				onchange={(event) => {
					mainPlace.refused = false;
					pick(event, (name) => (mainPhoto = name));
				}}
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
	{#if mainPlace.refused}
		<p class="px-gutter pt-2 text-read text-support" role="alert">
			{m.drop_photo_recipe_refused()}
		</p>
	{/if}

	<!--
		The facts (#175, option A): #81's strip, in two rows now so
		real labels fit on a phone. Every field is named by words on the screen,
		not by an `aria-label` only a screen reader hears. The reading page's
		"min prep" read as nonsense here, above an empty box, and the yield was
		two boxes with no name at all (2026-09-26).

		Each time is two boxes, hours and minutes, because a time can be either
		and nobody should work out that 9 hours is 540 minutes. Each box is named
		by the time's label AND its unit, `Prep time h`, both drawn beside it.
	-->
	<div class="border-y border-rule">
		<div class="flex">
			{@render time(prep, m.write_prep_time(), 'prep')}
			{@render time(cook, m.write_cook_time(), 'cook', cookHintId)}
		</div>
		<div class="border-t border-rule p-2">
			<span id="{uid}-makes" class="block text-label text-ink-2 uppercase">{m.write_makes()}</span>
			<div class="mt-1 flex gap-2">
				<div class="w-[5.5rem] shrink-0">
					<input
						bind:value={yieldAmount}
						inputmode="numeric"
						placeholder={m.write_makes_amount_example()}
						aria-labelledby="{uid}-makes {uid}-makes-amount"
						class={SMALL}
					/>
					<span id="{uid}-makes-amount" class="mt-1 block text-read text-ink-2">
						{m.write_makes_amount()}
					</span>
				</div>
				<div class="min-w-0 flex-1">
					<input
						bind:value={yieldNoun}
						placeholder={m.write_makes_noun_example()}
						aria-labelledby="{uid}-makes {uid}-makes-noun"
						class={SMALL}
					/>
					<span id="{uid}-makes-noun" class="mt-1 block text-read text-ink-2">
						{m.write_makes_noun()}
					</span>
				</div>
			</div>
		</div>
	</div>
	<!-- An overnight prove is cook time, or the quick tonight shelf suggests the
	     bread for a Tuesday (#32's item 43). The wording is 1B on #118. -->
	<p id={cookHintId} class="px-gutter pt-2 text-read text-ink-2">{m.write_cook_hint()}</p>

	<!--
		The page's two columns where the window is roomy (#198), since the
		editor is the page (#83). `Recipe.svelte` holds the choice and why the
		part that stays in view is a box inside the left column.
	-->
	<div class="recipe-columns">
		<div>
			<div class="stays-in-view" bind:this={linesStayEl}>
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
								<div class="flex flex-wrap items-center gap-4 pt-1">
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
											<button
												type="button"
												class="{TOOL} border-support-2 text-support-2"
												aria-label={m.write_line_recipe()}
												title={m.write_line_recipe()}
												onclick={() => (picking = row.id)}
											>
												<svg
													viewBox="0 0 24 24"
													aria-hidden="true"
													class="h-4 w-4"
													fill="none"
													stroke="currentColor"
													stroke-width="1.5"
													stroke-linecap="round"
													stroke-linejoin="round"
													><path
														d="M10 13a5 5 0 0 0 7 0l2-2a5 5 0 0 0-7-7l-1 1M14 11a5 5 0 0 0-7 0l-2 2a5 5 0 0 0 7 7l1-1"
													/></svg
												>
											</button>
										{/if}
									{/if}
									<button
										type="button"
										class="{TOOL} border-rule text-support"
										aria-label={m.write_remove()}
										title={m.write_remove()}
										onclick={() => remove('lines', index)}
									>
										<svg
											viewBox="0 0 24 24"
											aria-hidden="true"
											class="h-4 w-4"
											fill="none"
											stroke="currentColor"
											stroke-width="1.75"
											stroke-linecap="round"><path d="M6 6l12 12M18 6 6 18" /></svg
										>
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
					<button
						type="button"
						class="{QUIET} flex-1 text-accent"
						onclick={() => addLine('ingredient')}
					>
						{m.write_add_line()}
					</button>
					<button
						type="button"
						class="{QUIET} flex-1 text-accent"
						onclick={() => addLine('section')}
					>
						{m.write_add_heading()}
					</button>
				</div>
				<p class="px-gutter pt-2 text-read text-ink-2">{m.write_add_where()}</p>

				<!--
		The Nutrition figure, typed where it is read (#84): at the foot of the
		Ingredients, which is the treatment chosen on 21 September 2026
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
			</div>
		</div>
		<div>
			<!-- The Method: underneath the Ingredients, or beside them where the window is roomy. -->
			<h2 class="mx-gutter mt-8 mb-2 font-display text-label font-semibold text-accent uppercase">
				{m.recipe_method()}
			</h2>
			<ol bind:this={stepsEl}>
				{#each steps as row, index (row.id)}
					{@const place = row.kind === 'step' ? stepPlace(row.id) : null}
					<li
						data-row
						class="relative flex items-start gap-2 border-b border-rule px-gutter py-2 {dragging?.list ===
							'steps' && dragging.index === index
							? 'bg-card'
							: ''}"
						{@attach place?.listen}
					>
						{#if place?.held}
							<DropHere part="frame" held={place.held} />
						{/if}
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
							<div class="flex flex-wrap items-center gap-4 pt-1">
								{#if row.kind === 'step'}
									<label
										class="{TOOL} relative cursor-pointer border-rule text-accent"
										title={row.photo ? m.write_photo_change() : m.write_step_photo()}
									>
										{#if place?.held}
											{@const number = stepNumbers[index] ?? 0}
											<DropHere
												part="words"
												held={place.held}
												icon={DRAWN.picture}
												edged
												says={{
													ok: m.drop_photo_step({ number }),
													unsure: m.drop_photo_step_unsure({ number }),
													no: m.drop_photo_step_only(),
												}}
											/>
										{/if}
										<svg
											viewBox="0 0 24 24"
											aria-hidden="true"
											class="h-4 w-4"
											fill="none"
											stroke="currentColor"
											stroke-width="1.5"
											stroke-linecap="round"
											stroke-linejoin="round"
										>
											<path d="M4 7h3l2-2h6l2 2h3v12H4zM12 17a4 4 0 1 0 0-8 4 4 0 0 0 0 8" />
										</svg>
										<span class="sr-only"
											>{row.photo ? m.write_photo_change() : m.write_step_photo()}</span
										>
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
											class="{TOOL} border-rule text-support"
											aria-label={m.write_step_photo_remove()}
											title={m.write_step_photo_remove()}
											onclick={() => (row.photo = null)}
										>
											<svg
												viewBox="0 0 24 24"
												aria-hidden="true"
												class="h-4 w-4"
												fill="none"
												stroke="currentColor"
												stroke-width="1.75"
												stroke-linecap="round"><path d="M6 6l12 12M18 6 6 18" /></svg
											>
										</button>
									{/if}
								{/if}
								<button
									type="button"
									class="{TOOL} border-rule text-support"
									aria-label={m.write_remove()}
									title={m.write_remove()}
									onclick={() => remove('steps', index)}
								>
									<svg
										viewBox="0 0 24 24"
										aria-hidden="true"
										class="h-4 w-4"
										fill="none"
										stroke="currentColor"
										stroke-width="1.75"
										stroke-linecap="round"><path d="M6 6l12 12M18 6 6 18" /></svg
									>
								</button>
							</div>
							{#if place?.refused}
								<p class="pt-1 text-read text-support" role="alert">
									{m.drop_photo_step_refused()}
								</p>
							{/if}
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
		</div>
	</div>

	<!-- The note, and where the recipe came from, back in the one column (#198). -->
	<div class="roomy:mx-auto roomy:max-w-2xl">
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
				class="block w-full p-4 text-center font-display text-body {!wrong
					? act.grave
						? 'bg-support text-on-accent'
						: 'bg-accent text-on-accent'
					: 'border border-rule text-ink-2'}"
				disabled={saving || Boolean(wrong)}
				onclick={openSheet}
			>
				{act.does}
			</button>
		</div>
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
	<SheetFrame
		label={m.write_paste_offer()}
		tall={85}
		class="overflow-y-auto bg-ground px-gutter pt-4"
		onclose={() => (pasteOpen = false)}
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
			<!-- What it made of it: the title, then the counts and the split, drawn by PasteCheck. -->
			<p class="mt-2 text-body">
				{pasted.title === null || pasted.title.trim() === ''
					? m.write_paste_no_title()
					: m.write_paste_found_title({ title: pasted.title })}
			</p>
			<PasteCheck {pasted} bind:boundary />

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
	</SheetFrame>
{/if}

{#snippet time(
	value: { hours: string; minutes: string },
	label: string,
	key: string,
	/** The line beneath the strip that says what goes in this time, if one does. */
	hint?: string,
)}
	<div class="min-w-0 flex-1 border-l border-rule p-2 first:border-l-0">
		<span id="{uid}-{key}" class="block text-label text-ink-2 uppercase">{label}</span>
		<div class="mt-1 flex items-center gap-1">
			<input
				bind:value={value.hours}
				inputmode="numeric"
				aria-labelledby="{uid}-{key} {uid}-{key}-h"
				aria-describedby={hint}
				class={TIME_BOX}
			/>
			<span id="{uid}-{key}-h" class="text-read text-ink-2">{m.time_unit_hours()}</span>
			<input
				bind:value={value.minutes}
				inputmode="numeric"
				aria-labelledby="{uid}-{key} {uid}-{key}-min"
				aria-describedby={hint}
				class={TIME_BOX}
			/>
			<span id="{uid}-{key}-min" class="text-read text-ink-2">{m.time_unit_minutes()}</span>
		</div>
	</div>
{/snippet}

<!--
	The sheet that says which of the saves this is, before it happens (#54):
	onto your recipe, beside it as a variation (#131), or — on a recipe that is
	not yours — your own copy. It names the recipe, because *Save* and *Save*
	are the same word for different acts.
-->
{#if asking}
	<!--
		Enter saves, from the sheet or from one of its fields (#196), under the
		same conditions the button is held to. Only where the sheet asks one
		thing, a copy or a translation: with onto and beside to choose between,
		a key must not choose, which is `Confirm`'s rule as well.
	-->
	<SheetFrame
		label={act.called}
		tall={78}
		class="overflow-y-auto bg-ground px-gutter pt-4"
		onclose={() => {
			if (!saving) asking = false;
		}}
		onconfirm={forking || translating
			? () => {
					if (!saving) void save();
				}
			: undefined}
	>
		<!--
			A translation is neither of the two saves #54 named. It is never a
			fork — it makes a Branch of the same Lineage in your own Cookbook,
			whoever writes the source — and it is never a Version on this Branch. So it says
			its own sentence rather than borrowing the closer of two wrong ones.
		-->
		<p class="text-label text-ink-2 uppercase">{act.called}</p>
		{#if !forking && !translating}
			<!--
				Onto the recipe, or beside it (#131, screen choice 3): the two
				places a save of your own recipe can go, chosen here rather than
				behind a button of their own. Onto is where it starts.
			-->
			<fieldset class="mt-3 grid gap-2">
				<legend class="sr-only">{act.called}</legend>
				{#each [false, true] as choice (choice)}
					<label
						class="flex cursor-pointer items-start gap-3 rounded-sm border bg-card p-3 {beside ===
						choice
							? 'border-accent'
							: 'border-rule'}"
					>
						<input
							type="radio"
							name="{uid}-where"
							checked={beside === choice}
							onchange={() => (beside = choice)}
							class="mt-1 h-4 w-4 shrink-0 accent-accent"
						/>
						<span class="min-w-0">
							<span class="block text-body font-semibold text-ink">
								{choice ? m.write_where_beside() : m.write_where_onto({ title: title.trim() })}
							</span>
							<span class="block text-read text-ink-2">
								{choice
									? m.write_where_beside_said({ title: title.trim() })
									: m.write_where_onto_said()}
							</span>
						</span>
					</label>
				{/each}
			</fieldset>
		{:else}
			<p class="mt-2 text-body">{act.said}</p>
		{/if}
		{#if beside}
			<label class="mt-4 block">
				<span class="block text-label text-ink-2 uppercase">{m.write_variation_name()}</span>
				<input bind:value={variationName} class="mt-1 {SMALL}" />
			</label>
		{:else}
			<label class="mt-4 block">
				<span class="block text-label text-ink-2 uppercase">
					{m.write_name_label()} · {m.write_optional()}
				</span>
				<input bind:value={versionName} class="mt-1 {SMALL}" />
			</label>
		{/if}
		<label class="mt-3 block">
			<span class="block text-label text-ink-2 uppercase">
				{m.write_changed_label()} · {m.write_optional()}
			</span>
			<input bind:value={changeNote} class="mt-1 {SMALL}" />
		</label>
		<p class="mt-1 text-read text-ink-2">{m.write_changed_now()}</p>
		<button
			type="button"
			class="mt-4 block w-full p-4 text-center font-display text-body text-on-accent disabled:opacity-60 {act.grave
				? 'bg-support'
				: 'bg-accent'}"
			disabled={saving || (beside && variationName.trim() === '')}
			onclick={save}
		>
			{act.does}
		</button>
		<button
			type="button"
			class="mt-2 block w-full border border-rule p-4 text-center font-display text-body text-ink-2"
			onclick={() => (asking = false)}
		>
			{m.write_back()}
		</button>
	</SheetFrame>
{/if}
