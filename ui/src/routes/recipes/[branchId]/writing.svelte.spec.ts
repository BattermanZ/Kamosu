/**
 * The screen-seam test for writing a recipe (#83), against a stand-in Kamosu.
 * Every answer below is checked against the Catalogue before the screen sees
 * it, and so is every input the screen sends — a field renamed in
 * `src/catalogue.rs` fails this test in the same commit.
 *
 * What is being tested is the shape Aurélien chose on 20 September 2026:
 * ONE PAGE, both lists on it, an Ingredient Line as one free-text field, and
 * a save that says which of two things it is about to do. If an assertion here
 * starts describing a second screen, or a line split into amount and unit
 * boxes, the screen has gone wrong.
 *
 * The recipe underneath is the awkward one on purpose: headings in BOTH lists,
 * and lines with no quantity at all — 239 of the 863 Ingredient Lines in the
 * real corpus carry none, so it is the ordinary case rather than the edge.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { tick } from 'svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import type { GetRecipeOutput } from '$lib/api/catalogue';
import WritingTestHarness from './WritingTestHarness.svelte';
import { recipeAnswer } from '../../../testing/recipes';

type Content = GetRecipeOutput['versions'][number]['content'];

const content = (): Content => ({
	title: 'Dan Dan Noodles',
	yield: { amount: '4', noun: 'servings' },
	prep_time_minutes: 15,
	cook_time_minutes: 15,
	note: null,
	main_photo: null,
	nutrition: null,
	source: { text: 'recipetineats.com', link: null },
	ingredients: [
		{ kind: 'section', text: 'Dan Dan Sauce' },
		{ kind: 'ingredient', text: '2 tbsp Chinese sesame paste' },
		// No quantity at all, which is 28% of the real corpus.
		{ kind: 'ingredient', text: 'chilli oil, to taste' },
		{ kind: 'section', text: 'To serve' },
		{ kind: 'ingredient', text: '3 tbsp peanuts (finely chopped)' },
	],
	steps: [
		{ kind: 'section', text: 'Dan Dan Sauce', photo: null },
		{ kind: 'step', text: 'Mix everything except the oil.', photo: null },
		{ kind: 'section', text: 'Assemble', photo: null },
		{ kind: 'step', text: 'Pile in noodles and top with pork.', photo: null },
	],
});

const SAVED = {
	save_recipe_version: {
		branch_id: 'mine',
		version_id: 'v_2',
		parent_version_id: 'v_1',
		sequence: 2,
		collapsed: false,
		copied: false,
		language: 'en',
		language_offer: null,
		translates_version_id: null,
	},
} as Answers;

type Components = GetRecipeOutput['versions'][number]['components'];

function renderWriting(
	answers: Answers = {},
	/** Whether the reader writes the recipe's Cookbook; a save makes a Copy where not. */
	writes = true,
	components: Components = [],
	/** The recipe underneath, where a test needs one that is not Dan Dan Noodles. */
	start: Content = content(),
	/** Translating into this Language rather than editing (#106). */
	translatingInto?: 'en' | 'fr' | 'es',
) {
	const onSaved = vi.fn();
	const onCancel = vi.fn();
	const kamosu = standIn({ ...SAVED, ...answers });
	render(WritingTestHarness, {
		props: {
			client: kamosu.client,
			content: start,
			writes,
			components,
			translatingInto,
			onSaved,
			onCancel,
		},
	});
	return { kamosu, onSaved, onCancel };
}

/** A recipe that holds nothing yet — a new one, which is where a paste lands. */
const empty = (): Content => ({ ...content(), title: '', ingredients: [], steps: [] });

// ---- naming another recipe from a line (#87) -----------------------------
//
// The library the picker searches, and the recipe read back after a save so
// the pointer can be attached without clearing the amount the Core has just
// worked out of the line.

const SHELF = {
	search_recipes: {
		query: null,
		closest: false,
		recipes: [
			{
				lineage_id: 'l_chilli',
				branch_id: 'b_chilli',
				title: 'Chilli Oil',
				language: 'en',
				language_fallback: false,
				main_photo: null,
				yield: { amount: '1', noun: 'jar' },
				matched: null,
			},
		],
	},
} as Answers;

/**
 * The recipe as it stands the instant after the save: the Core has read
 * `2 tbsp Chinese sesame paste` into an amount, a Unit and a Food of its own.
 * Those are exactly what a pointer must not throw away.
 */
const READ_BACK = {
	get_recipe: {
		branch_id: 'mine',
		lineage_id: 'l_1',
		cookbook: { id: 'c_1', name: null, authors: [{ person_id: 'p_1', name: 'Aurélien' }] },
		name: null,
		writes: true,
		hand_id: 'h_mine',
		language: 'en',
		origin_address: null,
		head_version_id: 'v_2',
		translation: null,
		versions: [
			{
				sequence: 2,
				version_id: 'v_2',
				parent_version_id: 'v_1',
				hand_id: 'h_mine',
				name: null,
				change_note: null,
				created_at: '2026-09-21T00:00:00Z',
				translates_version_id: null,
				scaled_to: null,
				language: 'en',
				components: [],
				content: content(),
				readings: [
					null,
					{ amount: '2', unit: 'tbsp', target: 'Chinese sesame paste', lineage_id: null },
					{ amount: null, unit: null, target: 'chilli oil', lineage_id: null },
					null,
					{ amount: '3', unit: 'tbsp', target: 'peanuts', lineage_id: null },
				],
				measured: {
					ingredients: content().ingredients.map(() => null),
					steps: content().steps.map(() => null),
				},
				cooking: { steps: content().steps.map(() => ({ uses: [], timer_seconds: null })) },
			},
		],
		tags: [],
		related_recipes: [],
		cooked: { count: 0, last_cooked_at: null, ratings: [] },
	},
	set_reading: { line_index: 1, reading: null, measured: null },
} as Answers;

/**
 * What `start_translation` answers: a whole recipe, because what it makes is a
 * recipe (#106, ADR 0006). There is no Translation object for it to answer.
 */
const TRANSLATED = {
	start_translation: {
		...(READ_BACK.get_recipe as Record<string, unknown>),
		branch_id: 'b_fr',
		language: 'fr',
		translation: {
			translates_version_id: 'v_2',
			source_branch_id: 'mine',
			versions_behind: 0,
		},
	},
} as Answers;

/** Open the picker on one line and choose the only recipe the shelf holds. */
async function nameARecipe(line: string) {
	const row = (await screen.findByRole('textbox', { name: line })).closest('li') as HTMLElement;
	await fireEvent.click(within(row).getByRole('button', { name: 'Recipe' }));
	// The library is asked on the shelf's own short delay, so the entry is
	// waited for rather than expected to be there already.
	await fireEvent.click(await screen.findByRole('button', { name: 'Chilli Oil' }));
}

/** What the screen sent to `set_reading`, in order. */
const readings = (kamosu: ReturnType<typeof standIn>) =>
	kamosu.calls
		.filter((call) => call.operation === 'set_reading')
		.map((call) => call.input as Record<string, unknown>);

/** What the screen actually sent, once it has sent it. */
const sent = (kamosu: ReturnType<typeof standIn>) =>
	kamosu.calls.find((call) => call.operation === 'save_recipe_version')?.input as
		Record<string, unknown> | undefined;

/**
 * The fields that are rows of a list, in document order. Scoped by their
 * accessible names, because the note and the source link are textboxes too
 * and neither is a row of anything.
 */
const rowFields = () =>
	screen
		.getAllByRole('textbox')
		.filter((field) =>
			/^(Ingredient line \d+|Step \d+|Heading)$/.test(field.getAttribute('aria-label') ?? ''),
		) as HTMLTextAreaElement[];

/**
 * Walk the save through the sheet that states the outcome.
 *
 * The control is waited for rather than assumed present. Reading it
 * synchronously happened to work while an answer landed on a particular tick,
 * and broke the moment anything upstream of the client took one more (#98).
 */
async function saveThrough(name: RegExp) {
	await fireEvent.click((await screen.findAllByRole('button', { name }))[0] as HTMLElement);
	const inSheet = await screen.findAllByRole('button', { name });
	await fireEvent.click(inSheet[inSheet.length - 1] as HTMLElement);
}

afterEach(() => {
	Reflect.deleteProperty(navigator, 'onLine');
});

describe('writing a recipe', () => {
	it('is one page: both lists are on it, with their headings as real headings', async () => {
		renderWriting();

		// The ingredients and the method are both here, on one page. A separate
		// compose screen was drawn and rejected; finding only one list here
		// would mean the other had been moved off somewhere.
		const fields = await screen.findAllByRole('textbox');
		const values = fields.map((field) => (field as HTMLTextAreaElement).value);
		expect(values).toContain('2 tbsp Chinese sesame paste');
		expect(values).toContain('Mix everything except the oil.');

		// A heading in each list, editable as itself rather than as a line
		// pretending to be one.
		const headings = screen.getAllByRole('textbox', { name: 'Heading' });
		expect(headings).toHaveLength(4);
		expect(headings.map((h) => (h as HTMLTextAreaElement).value)).toEqual([
			'Dan Dan Sauce',
			'To serve',
			'Dan Dan Sauce',
			'Assemble',
		]);
	});

	it('writes an Ingredient Line as one free-text field and changes not a character of it', async () => {
		const { kamosu } = renderWriting();

		// One field for the whole line. If this screen ever grows an amount box
		// and a unit box, this is where it fails (ADR 0002, #43).
		const line = await screen.findByRole('textbox', { name: 'Ingredient line 1' });
		expect((line as HTMLTextAreaElement).value).toBe('2 tbsp Chinese sesame paste');
		expect(screen.queryByRole('textbox', { name: /amount/i })).not.toBeInTheDocument();
		expect(screen.queryByRole('textbox', { name: /unit/i })).not.toBeInTheDocument();

		await fireEvent.input(line, { target: { value: '2 poignées de farine, environ' } });
		await saveThrough(/Save onto mine/);

		const input = sent(kamosu);
		expect(input?.ingredients).toEqual([
			{ kind: 'section', text: 'Dan Dan Sauce' },
			{ kind: 'ingredient', text: '2 poignées de farine, environ' },
			{ kind: 'ingredient', text: 'chilli oil, to taste' },
			{ kind: 'section', text: 'To serve' },
			{ kind: 'ingredient', text: '3 tbsp peanuts (finely chopped)' },
		]);
	});

	it('puts the caret in the row Return just made, so a long list is one run of typing', async () => {
		renderWriting();

		// The bug this pins: the row appeared and the caret stayed put, so
		// every line after the first went into the first field. Found on a
		// real 21-line recipe, not here, which is why it is here now.
		const second = await screen.findByRole('textbox', { name: 'Ingredient line 1' });
		second.focus();
		await fireEvent.keyDown(second, { key: 'Enter' });
		await tick();

		const active = document.activeElement as HTMLTextAreaElement;
		expect(active.tagName).toBe('TEXTAREA');
		expect(active.value).toBe('');
		expect(active).not.toBe(second);
		// And it is the row directly under the one Return was pressed in.
		const order = rowFields();
		expect(order.indexOf(active)).toBe(order.indexOf(second as HTMLTextAreaElement) + 1);
	});

	it('adds a line and a heading to each list, and removes one', async () => {
		const { kamosu } = renderWriting();

		await fireEvent.click(await screen.findByRole('button', { name: 'Add a line' }));
		await fireEvent.click(screen.getByRole('button', { name: 'Add a step' }));
		await fireEvent.click(screen.getAllByRole('button', { name: 'Add a heading' })[1]!);

		// Three new rows, each empty and each waiting to be written in.
		const empties = rowFields().filter((field) => field.value === '');
		expect(empties.length).toBe(3);
		await fireEvent.input(empties[0]!, { target: { value: '1 tbsp black vinegar' } });
		await fireEvent.input(empties[1]!, { target: { value: 'Toast the peanuts.' } });
		await fireEvent.input(empties[2]!, { target: { value: 'At the table' } });

		// And a line taken out is gone from what is sent.
		await fireEvent.click(screen.getAllByRole('button', { name: 'Remove' })[2]!);

		await saveThrough(/Save onto mine/);
		const input = sent(kamosu);
		expect(input?.ingredients).toEqual([
			{ kind: 'section', text: 'Dan Dan Sauce' },
			{ kind: 'ingredient', text: '2 tbsp Chinese sesame paste' },
			{ kind: 'section', text: 'To serve' },
			{ kind: 'ingredient', text: '3 tbsp peanuts (finely chopped)' },
			{ kind: 'ingredient', text: '1 tbsp black vinegar' },
		]);
		expect(input?.steps).toEqual([
			{ kind: 'section', text: 'Dan Dan Sauce', photo: null },
			{ kind: 'step', text: 'Mix everything except the oil.', photo: null },
			{ kind: 'section', text: 'Assemble', photo: null },
			{ kind: 'step', text: 'Pile in noodles and top with pork.', photo: null },
			{ kind: 'step', text: 'Toast the peanuts.', photo: null },
			{ kind: 'section', text: 'At the table', photo: null },
		]);
	});

	it('rearranges a list by its handle, and from the keyboard', async () => {
		const { kamosu } = renderWriting();

		// The peanuts are stranded at the foot under the wrong heading. The
		// handle is dragged to move them; the arrow keys walk the same code one
		// row at a time, and are what a test can drive without a pointer.
		await screen.findByRole('textbox', { name: 'Ingredient line 1' });
		const lines = rowFields().filter((field) =>
			(field.getAttribute('aria-label') ?? '').startsWith('Ingredient line'),
		);
		const peanuts = lines[lines.length - 1]!.closest('li')!;
		const grip = within(peanuts).getByRole('button', { name: /Drag to move/ });

		// Up three: past the heading it does not belong under, and up into the
		// sauce where it does. The row is found again each time by the words in
		// it, because a bound field holds its text as a value rather than as
		// anything the row's own textContent can see.
		const rowHolding = (text: string) =>
			rowFields()
				.find((f) => f.value === text)!
				.closest('li')!;
		for (let press = 0; press < 3; press += 1) {
			const here = rowHolding('3 tbsp peanuts (finely chopped)');
			await fireEvent.keyDown(within(here).getByRole('button', { name: /Drag to move/ }), {
				key: 'ArrowUp',
			});
		}
		// The same handle throughout: the row is moved, never rebuilt, which is
		// what keeps a drag alive across a rearrange.
		expect(grip).toBe(
			within(rowHolding('3 tbsp peanuts (finely chopped)')).getByRole('button', {
				name: /Drag to move/,
			}),
		);

		// Three places up, across a heading on the way: the list it lands in is
		// the one it was dragged through, with everything else in order.
		await saveThrough(/Save onto mine/);
		expect(sent(kamosu)?.ingredients).toEqual([
			{ kind: 'section', text: 'Dan Dan Sauce' },
			{ kind: 'ingredient', text: '3 tbsp peanuts (finely chopped)' },
			{ kind: 'ingredient', text: '2 tbsp Chinese sesame paste' },
			{ kind: 'ingredient', text: 'chilli oil, to taste' },
			{ kind: 'section', text: 'To serve' },
		]);
	});

	it('stops a row at the end of its own list rather than sliding it into the other', async () => {
		const { kamosu } = renderWriting();
		await screen.findByRole('textbox', { name: 'Ingredient line 1' });

		// The first row, pushed up. There is nowhere above it, and the method
		// below is a different list — a row must not fall out of its own.
		const first = rowFields()[0]!.closest('li')!;
		await fireEvent.keyDown(within(first).getByRole('button', { name: /Drag to move/ }), {
			key: 'ArrowUp',
		});

		await saveThrough(/Save onto mine/);
		const input = sent(kamosu);
		expect((input?.ingredients as { text: string }[])[0]!.text).toBe('Dan Dan Sauce');
		expect(input?.steps).toHaveLength(4);
	});

	it('renames and reorders a heading in the METHOD too, not only in the list', async () => {
		// The spec asks for all four verbs in BOTH lists. Adding and removing
		// are covered above; these two were only ever proved on the
		// ingredients, which is where a second list quietly goes wrong.
		const { kamosu } = renderWriting();

		const assemble = (await screen.findAllByRole('textbox', { name: 'Heading' }))[3]!;
		expect((assemble as HTMLTextAreaElement).value).toBe('Assemble');
		await fireEvent.input(assemble, { target: { value: 'To finish' } });

		// And move it up past the step above it.
		await fireEvent.keyDown(
			within(assemble.closest('li')!).getByRole('button', { name: /Drag to move/ }),
			{
				key: 'ArrowUp',
			},
		);

		await saveThrough(/Save onto mine/);
		expect(sent(kamosu)?.steps).toEqual([
			{ kind: 'section', text: 'Dan Dan Sauce', photo: null },
			{ kind: 'section', text: 'To finish', photo: null },
			{ kind: 'step', text: 'Mix everything except the oil.', photo: null },
			{ kind: 'step', text: 'Pile in noodles and top with pork.', photo: null },
		]);
	});

	it('sends a Step photograph to the server, never a name only this phone knows', async () => {
		// A photograph taken while cooking is kept under a `local:…` name and
		// given its real one when the cooking is sent. Nothing rewrites those
		// inside a save_recipe_version, so a recipe that took that path would
		// store a name no server has ever heard of.
		const photograph = vi.fn(async () => 'p_realname');
		const kamosu = standIn({ ...SAVED });
		render(WritingTestHarness, {
			props: { client: kamosu.client, content: content(), photograph },
		});

		const picture = new File([new Uint8Array([1, 2, 3])], 'plate.jpg', { type: 'image/jpeg' });
		const choosers = document.querySelectorAll<HTMLInputElement>('input[type="file"]');
		await fireEvent.change(choosers[choosers.length - 1]!, { target: { files: [picture] } });
		await vi.waitFor(() => expect(photograph).toHaveBeenCalledOnce());

		await saveThrough(/Save onto mine/);
		const steps = sent(kamosu)?.steps as { photo: string | null }[];
		expect(steps.some((step) => step.photo === 'p_realname')).toBe(true);
		expect(steps.every((step) => !step.photo?.startsWith('local:'))).toBe(true);
	});

	it('opens on a recipe with nothing in it, which is what a new one is', async () => {
		// A new recipe is the same page, empty (#83's decision). Both lists
		// say so rather than being absent, and all four controls are there.
		const kamosu = standIn({ ...SAVED });
		const bare = content();
		bare.ingredients = [];
		bare.steps = [];
		bare.yield = null;
		bare.prep_time_minutes = null;
		bare.cook_time_minutes = null;
		bare.source = null;
		render(WritingTestHarness, {
			props: { client: kamosu.client, content: bare },
		});

		expect(await screen.findByText('No ingredients yet.')).toBeInTheDocument();
		expect(screen.getByText('No method yet.')).toBeInTheDocument();
		for (const name of ['Add a line', 'Add a step']) {
			expect(screen.getByRole('button', { name })).toBeInTheDocument();
		}
		expect(screen.getAllByRole('button', { name: 'Add a heading' })).toHaveLength(2);

		// Nothing empty is an error state: the save is offered straight away.
		await saveThrough(/Save onto mine/);
		const input = sent(kamosu);
		expect(input?.ingredients).toEqual([]);
		expect(input?.steps).toEqual([]);
		expect(input?.yield).toBeNull();
	});

	it('has no timer, temperature or duration anywhere on it', async () => {
		renderWriting();
		await screen.findByRole('textbox', { name: 'Step 1' });

		// #43 and ADR 0011: a Step is a sentence, and the amounts for it are
		// worked out from the Readings rather than typed into fields here.
		for (const forbidden of [/timer/i, /temperature/i, /duration/i, /degrees/i]) {
			expect(screen.queryByRole('textbox', { name: forbidden })).not.toBeInTheDocument();
			expect(screen.queryByRole('spinbutton', { name: forbidden })).not.toBeInTheDocument();
		}
	});

	// #118 and #32's item 43: an overnight prove is cook time, and the quick
	// tonight shelf is only as right as that number.
	it('says that resting, proving, marinating and chilling belong in Cook', async () => {
		renderWriting();

		const cook = await screen.findByRole('textbox', { name: 'Cook, in whole minutes' });
		expect(cook).toHaveAccessibleDescription(
			'Include time spent resting, proving, marinating or chilling.',
		);
		expect(
			screen.getByText('Include time spent resting, proving, marinating or chilling.'),
		).toBeVisible();
	});

	it('lets every fact be emptied, and sends nothing rather than a blank', async () => {
		const { kamosu } = renderWriting();

		for (const name of ['Prep, in whole minutes', 'Cook, in whole minutes']) {
			await fireEvent.input(await screen.findByRole('textbox', { name }), {
				target: { value: '' },
			});
		}
		for (const name of ['Yield, how many', 'Yield, of what']) {
			await fireEvent.input(screen.getByRole('textbox', { name }), { target: { value: '' } });
		}
		await fireEvent.input(screen.getByRole('textbox', { name: 'Source' }), {
			target: { value: '' },
		});

		await saveThrough(/Save onto mine/);
		const input = sent(kamosu);
		expect(input?.prep_time_minutes).toBeNull();
		expect(input?.cook_time_minutes).toBeNull();
		expect(input?.yield).toBeNull();
		expect(input?.source).toBeNull();
		expect(input?.note).toBeNull();
	});

	/**
	 * THE FIGURE IS TYPED WHERE IT IS READ (#84): at the foot of the
	 * Ingredients, which is the treatment Aurélien chose on 21 September 2026
	 * for both surfaces. Reading and writing share it rather than each making
	 * their own choice.
	 *
	 * The basis is typed with the number and is not a setting. 308 says nothing
	 * until it says what it counts, and the two do not convert into each other
	 * without a weight a recipe does not carry (CONTEXT.md, "Nutrition").
	 */
	it('types the figure at the foot of the Ingredients, with what it counts', async () => {
		const { kamosu } = renderWriting();

		await fireEvent.input(await screen.findByRole('textbox', { name: 'Nutrition, in kcal' }), {
			target: { value: '308' },
		});
		await fireEvent.change(screen.getByRole('combobox', { name: 'What the figure counts' }), {
			target: { value: 'per_100g' },
		});

		await saveThrough(/Save onto mine/);
		expect(sent(kamosu)?.nutrition).toEqual({ calories: 308, basis: 'per_100g' });
	});

	it('opens on the figure the recipe already carries', async () => {
		const onSaved = vi.fn();
		const kamosu = standIn({ ...SAVED });
		const held = content();
		held.nutrition = { calories: 308, basis: 'per_serving' };
		render(WritingTestHarness, {
			props: { client: kamosu.client, content: held, onSaved },
		});

		expect(await screen.findByRole('textbox', { name: 'Nutrition, in kcal' })).toHaveValue('308');
		expect(screen.getByRole('combobox', { name: 'What the figure counts' })).toHaveValue(
			'per_serving',
		);
		// Opening the screen and saving changes nothing it did not touch.
		await saveThrough(/Save onto mine/);
		expect(sent(kamosu)?.nutrition).toEqual({ calories: 308, basis: 'per_serving' });
	});

	it('sends nothing rather than a zero where the figure is left blank', async () => {
		// A field holding nothing is no part of the fingerprint (ADR 0038), and
		// a zero would be a claim this recipe has no calories in it.
		const { kamosu } = renderWriting();

		await screen.findByRole('textbox', { name: 'Nutrition, in kcal' });
		await saveThrough(/Save onto mine/);
		expect(sent(kamosu)?.nutrition).toBeNull();
	});

	it('empties a figure the recipe carried when it is cleared', async () => {
		const onSaved = vi.fn();
		const kamosu = standIn({ ...SAVED });
		const held = content();
		held.nutrition = { calories: 308, basis: 'per_serving' };
		render(WritingTestHarness, {
			props: { client: kamosu.client, content: held, onSaved },
		});

		await fireEvent.input(await screen.findByRole('textbox', { name: 'Nutrition, in kcal' }), {
			target: { value: '' },
		});
		await saveThrough(/Save onto mine/);
		expect(sent(kamosu)?.nutrition).toBeNull();
	});

	it('will not let a Nutrition figure that is not a number be dropped in silence', async () => {
		const { kamosu } = renderWriting();

		await fireEvent.input(await screen.findByRole('textbox', { name: 'Nutrition, in kcal' }), {
			target: { value: 'quite a lot' },
		});

		expect(await screen.findByText(/Nutrition figure has to be a number/i)).toBeInTheDocument();
		expect(sent(kamosu)).toBeUndefined();
	});

	it('says it is writing a Version onto yours, and asks the two optional things', async () => {
		const { kamosu } = renderWriting();
		await screen.findByRole('textbox', { name: 'Ingredient line 1' });

		await fireEvent.click((await screen.findAllByRole('button', { name: /Save onto mine/ }))[0]!);
		// The outcome is stated before it happens, naming the recipe — "Save"
		// and "Save" are the same word for two different acts. Onto the recipe
		// is where it starts; beside it is the other choice (#131).
		expect(screen.getByRole('radio', { name: /Onto your Dan Dan Noodles/ })).toBeChecked();
		expect(screen.getByText('A new Version of it, in your Cookbook.')).toBeInTheDocument();

		await fireEvent.input(screen.getByRole('textbox', { name: /Name this version/ }), {
			target: { value: 'Less chilli' },
		});
		await fireEvent.input(screen.getByRole('textbox', { name: /What changed/ }), {
			target: { value: 'Moved the vinegar into the sauce' },
		});
		const buttons = screen.getAllByRole('button', { name: /Save onto mine/ });
		await fireEvent.click(buttons[buttons.length - 1]!);

		const input = sent(kamosu);
		expect(input?.name).toBe('Less chilli');
		expect(input?.change_note).toBe('Moved the vinegar into the sauce');
	});

	it('says a save onto a recipe somebody else writes will fork, and is not the same control', async () => {
		renderWriting({}, false);
		await screen.findByRole('textbox', { name: 'Ingredient line 1' });

		// The forking save wears its own words. Finding *Save onto mine* here
		// would mean the two acts had been collapsed into one button.
		expect(screen.queryByRole('button', { name: /Save onto mine/ })).not.toBeInTheDocument();
		await fireEvent.click(
			(await screen.findAllByRole('button', { name: /Start my own copy/ }))[0]!,
		);
		expect(screen.getByText(/is not yours to change/)).toBeInTheDocument();
		expect(screen.getByText(/the original stays as it is/)).toBeInTheDocument();
		// Where it goes is not a question: your own Cookbook, always (#131).
		expect(screen.queryAllByRole('radio')).toHaveLength(0);
	});

	it('hands a collapse up to the page, which is what is still on screen afterwards', async () => {
		// Saving closes this screen, so what happened has to be handed up: a
		// line drawn here would be destroyed before anybody read it. The
		// recipe page draws it.
		const { kamosu, onSaved } = renderWriting({
			save_recipe_version: {
				branch_id: 'mine',
				version_id: 'v_1',
				parent_version_id: null,
				sequence: 1,
				collapsed: true,
				copied: false,
				language: 'en',
				language_offer: null,
				translates_version_id: null,
			},
		} as Answers);

		await screen.findByRole('textbox', { name: 'Ingredient line 1' });
		await saveThrough(/Save onto mine/);
		expect(sent(kamosu)).toBeDefined();
		expect(onSaved).toHaveBeenCalledWith({
			branch_id: 'mine',
			collapsed: true,
			copied: false,
			named: true,
			// The save's Language offer travels up with the rest (#106): this
			// save's text agreed with the Language the recipe carries, so there
			// is nothing to put to the cook.
			language_offer: null,
		});
	});

	it('hands up the NEW Branch a Copy landed on, not the one that was open', async () => {
		// The Version went somewhere else. A page that stayed put would leave
		// the cook reading the recipe they deliberately did not change.
		const { onSaved } = renderWriting(
			{
				save_recipe_version: {
					branch_id: 'b_my_copy',
					version_id: 'v_9',
					parent_version_id: 'v_1',
					sequence: 1,
					collapsed: false,
					copied: true,
					language: 'en',
					language_offer: null,
					translates_version_id: null,
				},
			} as Answers,
			false,
		);

		await screen.findByRole('textbox', { name: 'Ingredient line 1' });
		await saveThrough(/Start my own copy/);
		expect(onSaved).toHaveBeenCalledWith({
			branch_id: 'b_my_copy',
			collapsed: false,
			copied: true,
			named: true,
			language_offer: null,
		});
	});

	it('will not offer a save it cannot make, and says why', async () => {
		const { kamosu } = renderWriting();

		// A title is all a recipe needs (#6), so a recipe without one is the
		// one thing this screen refuses. The save is not offered rather than
		// offered and then rejected.
		await fireEvent.input(await screen.findByRole('textbox', { name: 'Title' }), {
			target: { value: '   ' },
		});
		expect(await screen.findByText('A recipe needs a title.')).toBeInTheDocument();
		for (const button of screen.getAllByRole('button', { name: /Save/ })) {
			expect(button).toBeDisabled();
		}
		expect(sent(kamosu)).toBeUndefined();
	});

	it('will not let half a Yield be dropped in silence', async () => {
		// A Yield is a number AND a word for what it makes; the Catalogue
		// carries neither alone. Typing `4` and no noun used to be thrown away
		// on the way out.
		const { kamosu } = renderWriting();
		await fireEvent.input(await screen.findByRole('textbox', { name: 'Yield, of what' }), {
			target: { value: '' },
		});
		expect(
			await screen.findByText('A Yield needs both a number and a word for what it makes.'),
		).toBeInTheDocument();
		expect(sent(kamosu)).toBeUndefined();
	});

	it('will not let a time that is not minutes be dropped in silence', async () => {
		const { kamosu } = renderWriting();
		await fireEvent.input(await screen.findByRole('textbox', { name: 'Prep, in whole minutes' }), {
			target: { value: 'about an hour' },
		});
		expect(await screen.findByText(/must be whole minutes/)).toBeInTheDocument();
		expect(sent(kamosu)).toBeUndefined();
	});

	it('starts a variation beside the recipe, under the name it is given', async () => {
		// Screen choice 3 of 24 September 2026: a variation is chosen inside
		// the save sheet, and the draft lands on it rather than on the recipe.
		const { kamosu, onSaved } = renderWriting({
			start_variation: recipeAnswer({ branch_id: 'b_veg' }),
		} as Answers);
		await screen.findByRole('textbox', { name: 'Ingredient line 1' });
		await fireEvent.click((await screen.findAllByRole('button', { name: /Save onto mine/ }))[0]!);
		const sheet = within(await screen.findByRole('dialog'));
		await fireEvent.click(sheet.getByRole('radio', { name: /As a variation beside it/ }));
		expect(sheet.getByText(/A second Dan Dan Noodles/)).toBeInTheDocument();

		// Nothing tells two unnamed recipes apart, so the button waits for a name.
		const start = sheet.getByRole('button', { name: 'Start the variation' });
		expect(start).toBeDisabled();
		await fireEvent.input(sheet.getByRole('textbox', { name: /Call the variation/ }), {
			target: { value: 'Vegetarian' },
		});
		await fireEvent.click(start);

		await waitFor(() => expect(onSaved).toHaveBeenCalled());
		expect(kamosu.calls.find((call) => call.operation === 'start_variation')?.input).toEqual({
			branch_id: 'mine',
			name: 'Vegetarian',
		});
		// The draft goes onto the variation, never onto the recipe it came from.
		expect(sent(kamosu)?.branch_id).toBe('b_veg');
		expect(onSaved).toHaveBeenCalledWith(
			expect.objectContaining({ branch_id: 'b_veg', varied: 'Vegetarian' }),
		);
	});
});

/**
 * Saying *this line is a recipe* (#87) — the act that makes a Component, and
 * the one part of a Reading this screen edits.
 *
 * ADR 0008 refuses guessing which recipe a line means, so every test here
 * checks that the recipe was CHOSEN. Nothing on this screen may ever arrive at
 * a pointer by matching words.
 */
describe('where a Copy goes (#111, #131)', () => {
	it('never asks: a Copy is started in your own Cookbook', async () => {
		const { kamosu } = renderWriting({}, false);
		await screen.findByRole('textbox', { name: 'Ingredient line 1' });
		await fireEvent.click(
			(await screen.findAllByRole('button', { name: /Start my own copy/ }))[0]!,
		);

		const sheet = await screen.findByRole('dialog');
		expect(within(sheet).queryAllByRole('radio')).toHaveLength(0);
		expect(
			within(sheet).getByText(/your own Dan Dan Noodles in your Cookbook/),
		).toBeInTheDocument();
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Start my own copy' }));
		await waitFor(() => expect(sent(kamosu)).toBeDefined());
		expect(sent(kamosu)).not.toHaveProperty('kitchen_id');
	});

	it('asks nothing about where a Translation goes either', async () => {
		const { kamosu } = renderWriting({ ...TRANSLATED } as Answers, false, [], content(), 'fr');
		await screen.findByRole('textbox', { name: 'Ingredient line 1' });
		await fireEvent.click(
			(await screen.findAllByRole('button', { name: /Save the translation/ }))[0] as HTMLElement,
		);
		const sheet = await screen.findByRole('dialog');
		expect(within(sheet).queryAllByRole('radio')).toHaveLength(0);
		await fireEvent.click(within(sheet).getByRole('button', { name: /Save the translation/ }));
		await waitFor(() => {
			const input = kamosu.calls.find((call) => call.operation === 'start_translation')?.input;
			expect(input).toBeDefined();
			expect(input).not.toHaveProperty('kitchen_id');
		});
	});
});

describe('naming another recipe from a line', () => {
	it('offers no recipe until one is searched for, and pre-selects nothing', async () => {
		renderWriting({ ...SHELF, ...READ_BACK });
		const row = (
			await screen.findByRole('textbox', {
				name: 'Ingredient line 1',
			})
		).closest('li') as HTMLElement;

		// Closed, the row says nothing about recipes: the control is an offer,
		// never a suggestion that this line looks like one.
		expect(within(row).queryByText(/Names/)).not.toBeInTheDocument();

		await fireEvent.click(within(row).getByRole('button', { name: 'Recipe' }));
		// Open, the library is a list to pick from and nothing in it is chosen.
		// `aria-pressed` would be the tell of a pre-selection; there is none.
		const chosen = await screen.findByRole('button', { name: 'Chilli Oil' });
		expect(chosen).not.toHaveAttribute('aria-pressed', 'true');
		expect(screen.getByText(/For the line: 2 tbsp Chinese sesame paste/)).toBeInTheDocument();
	});

	it('attaches the pointer after the save, keeping the amount the Core just read', async () => {
		const { kamosu, onSaved } = renderWriting({ ...SHELF, ...READ_BACK });
		await nameARecipe('Ingredient line 1');
		await saveThrough(/Save onto mine/);

		// The written line is untouched by any of this: naming a recipe is a
		// Reading, and a Reading never rewrites the words above it (ADR 0002).
		const saved = sent(kamosu) as { ingredients: { text: string }[] };
		expect(saved.ingredients[1]?.text).toBe('2 tbsp Chinese sesame paste');

		// One line changed, so one Reading was set — and it carries the amount
		// and the Unit the Core worked out of the line during the save. Sending
		// the pointer alone would have cleared them, and a Component with no
		// quantity is the WHOLE of the inner recipe (ADR 0008).
		expect(readings(kamosu)).toEqual([
			{
				branch_id: 'mine',
				line_index: 1,
				amount: '2',
				unit: 'tbsp',
				// The two are exclusive, so naming a Recipe puts the Food down.
				target: null,
				lineage_id: 'l_chilli',
			},
		]);
		// Handed up only after the read-back and the Reading have landed, which
		// is three asks deep — waited for rather than counted in ticks.
		await waitFor(() =>
			expect(onSaved).toHaveBeenCalledWith(
				expect.objectContaining({ branch_id: 'mine', named: true }),
			),
		);
	});

	it('takes a line that names a recipe back to an ordinary one', async () => {
		// The save carried the pointer forward on its own, because the line was
		// not edited — so what the read-back finds is a Component, and taking it
		// back to an ordinary line is a change this screen has to send.
		const carried = {
			get_recipe: {
				...(READ_BACK.get_recipe as Record<string, unknown>),
				versions: [
					{
						...((READ_BACK.get_recipe as { versions: Record<string, unknown>[] })
							.versions[0] as Record<string, unknown>),
						readings: [
							null,
							// A Component's Reading names a Lineage and no Food:
							// the two are one slot (ADR 0008).
							{ amount: '2', unit: 'tbsp', target: null, lineage_id: 'l_chilli' },
							{ amount: null, unit: null, target: 'chilli oil', lineage_id: null },
							null,
							{ amount: '3', unit: 'tbsp', target: 'peanuts', lineage_id: null },
						],
					},
				],
			},
		} as Answers;
		const { kamosu } = renderWriting({ ...SHELF, ...READ_BACK, ...carried }, true, [
			{
				path: [1],
				lineage_id: 'l_chilli',
				held: true,
				stopped: false,
				branch_id: 'b_chilli',
				title: 'Chilli Oil',
				share: null,
				said: 'Chilli Oil',
				content: null,
				readings: null,
				measured: null,
			},
		]);

		// It opens SAYING SO, rather than looking like an ordinary line whose
		// pointer somebody is about to lose without noticing.
		const row = (
			await screen.findByRole('textbox', {
				name: 'Ingredient line 1',
			})
		).closest('li') as HTMLElement;
		expect(within(row).getByRole('button', { name: 'Names Chilli Oil' })).toBeInTheDocument();

		await fireEvent.click(within(row).getByRole('button', { name: 'Not a recipe' }));
		expect(within(row).getByRole('button', { name: 'Recipe' })).toBeInTheDocument();
		await saveThrough(/Save onto mine/);

		// The pointer is dropped and the amount is left standing. No Food takes
		// its place, because a Component never had one to put back — the target
		// and the Lineage are one slot, and it held the Lineage. The line reads
		// exactly as it always did, and what it is can be typed in the
		// corrector on the reading page.
		expect(readings(kamosu)).toEqual([
			{
				branch_id: 'mine',
				line_index: 1,
				amount: '2',
				unit: 'tbsp',
				target: null,
				lineage_id: null,
			},
		]);
	});

	it('sends nothing at all when no line names a recipe', async () => {
		// The ordinary save, which is nearly every save. A recipe that composes
		// nothing must not pay a read-back and a round of Readings for a
		// feature it does not use.
		const { kamosu } = renderWriting({ ...SHELF, ...READ_BACK });
		await screen.findByRole('textbox', { name: 'Ingredient line 1' });
		await saveThrough(/Save onto mine/);
		expect(readings(kamosu)).toEqual([]);
		expect(kamosu.calls.some((call) => call.operation === 'get_recipe')).toBe(false);
	});

	it('says the Version landed even when the pointer did not', async () => {
		// The save SUCCEEDED. A line that reads correctly but is not a
		// Component is exactly the failure nobody would otherwise notice, so it
		// is reported beside the save rather than instead of it.
		const { onSaved } = renderWriting({
			...SHELF,
			...READ_BACK,
			set_reading: { refuse: 'internal' },
		} as Answers);
		await nameARecipe('Ingredient line 1');
		await saveThrough(/Save onto mine/);
		// Handed up only after the read-back and the Reading have landed, which
		// is three asks deep — waited for rather than counted in ticks.
		await waitFor(() =>
			expect(onSaved).toHaveBeenCalledWith(
				expect.objectContaining({ branch_id: 'mine', collapsed: false, named: false }),
			),
		);
	});
});

// ---- pasting a whole recipe (#94) ----------------------------------------
//
// The screen seam, not the parser: what the Operation answers is the
// stand-in's to say, and what is tested here is that the screen SHOWS it
// before anything lands, splits by the boundary it was given, re-splits when
// that boundary is moved WITHOUT asking again, and says what it is about to
// replace.
//
// The parser itself is measured in Rust, against the real 86-recipe export
// (`tests/pasting_corpus.rs`).

/** A paste with Sections in BOTH blocks — the awkward case, on purpose. */
const WITH_HEADINGS = {
	read_pasted_recipe: {
		title: 'Dan Dan Noodles',
		lines: [
			{ kind: 'section', text: 'Dan Dan Sauce:' },
			{ kind: 'line', text: '2 tbsp Chinese sesame paste' },
			{ kind: 'line', text: 'chilli oil, to taste' },
			{ kind: 'section', text: 'Assemble:' },
			{ kind: 'line', text: 'Mix the sauce and pour it over the noodles.' },
		],
		boundary: 3,
	},
} as Answers;

/** And one with no Section at all, which is most of them. */
const NO_HEADINGS = {
	read_pasted_recipe: {
		title: null,
		lines: [
			{ kind: 'line', text: '200 g plain flour' },
			{ kind: 'line', text: '2 poignées de farine, environ' },
			{ kind: 'line', text: 'Heat the oven and butter a tin thoroughly.' },
		],
		boundary: 2,
	},
} as Answers;

/** Open the paste sheet, put text in it, and have it read. */
async function pasteIn(text = 'anything, since the stand-in answers') {
	await fireEvent.click(screen.getByRole('button', { name: 'Paste a whole recipe' }));
	const field = await screen.findByRole('textbox', { name: 'The recipe, pasted as text' });
	await fireEvent.input(field, { target: { value: text } });
	await fireEvent.click(screen.getByRole('button', { name: 'Read it' }));
	return screen.findByRole('button', { name: 'Use it' });
}

/**
 * How many rows the sheet draws on each side of the split, read off the page
 * rather than off the answer — Sections included, since they are what moving
 * the boundary most often carries across.
 */
function pasteSides(): [number, number] {
	const headings = screen.getAllByRole('heading', { level: 3 });
	const rows = screen.getAllByRole('button', { name: 'Start the method at this line' });
	const method = headings[1] as HTMLElement;
	const below = rows.filter(
		(row) => method.compareDocumentPosition(row) & Node.DOCUMENT_POSITION_FOLLOWING,
	);
	return [rows.length - below.length, below.length];
}

/** How many times the paste was actually sent to be read. */
const reads = (kamosu: ReturnType<typeof standIn>) =>
	kamosu.calls.filter((call) => call.operation === 'read_pasted_recipe');

describe('pasting a whole recipe', () => {
	it('becomes a title, an ingredient list and a method, headings kept as Sections', async () => {
		renderWriting(WITH_HEADINGS, true, [], empty());
		await pasteIn();
		await fireEvent.click(screen.getByRole('button', { name: 'Use it' }));

		// The title it found is on the page, in the title field.
		const title = (await screen.findByRole('textbox', { name: 'Title' })) as HTMLTextAreaElement;
		expect(title.value).toBe('Dan Dan Noodles');

		// Split at the boundary it was given: two ingredients above, one step
		// below, and a heading in each — as real Sections, not lines wearing a
		// colon.
		expect(rowFields().map((field) => field.value)).toEqual([
			'Dan Dan Sauce:',
			'2 tbsp Chinese sesame paste',
			'chilli oil, to taste',
			'Assemble:',
			'Mix the sauce and pour it over the noodles.',
		]);
		const headings = screen.getAllByRole('textbox', { name: 'Heading' });
		expect(headings.map((h) => (h as HTMLTextAreaElement).value)).toEqual([
			'Dan Dan Sauce:',
			'Assemble:',
		]);
		expect(screen.getAllByRole('textbox', { name: /^Ingredient line/ })).toHaveLength(2);
		expect(screen.getAllByRole('textbox', { name: /^Step/ })).toHaveLength(1);
	});

	it('takes a paste with no headings at all, exactly as it was pasted', async () => {
		renderWriting(NO_HEADINGS, true, [], empty());
		await pasteIn();
		await fireEvent.click(screen.getByRole('button', { name: 'Use it' }));

		// No heading anywhere, and every line unchanged — no amount lifted out
		// of `2 poignées de farine, environ`, no rewording (ADR 0002).
		expect(screen.queryAllByRole('textbox', { name: 'Heading' })).toHaveLength(0);
		expect(rowFields().map((field) => field.value)).toEqual([
			'200 g plain flour',
			'2 poignées de farine, environ',
			'Heat the oven and butter a tin thoroughly.',
		]);
	});

	it('says what it made of the paste before anything lands', async () => {
		renderWriting(WITH_HEADINGS, true, [], empty());
		await pasteIn();

		// The counts and the title are on screen, and the page underneath is
		// still empty: nothing lands until *Use it*.
		expect(screen.getByText(/2 ingredients and 1 steps/)).toBeInTheDocument();
		expect(screen.getByText(/Dan Dan Noodles/)).toBeInTheDocument();
		expect(rowFields()).toHaveLength(0);
	});

	it('re-splits when the boundary is moved, without reading a line again', async () => {
		const { kamosu } = renderWriting(WITH_HEADINGS, true, [], empty());
		await pasteIn();
		expect(reads(kamosu)).toHaveLength(1);

		// One line later: `Assemble:` was the first row of the method and is
		// now the last row of the list. Asserted on WHICH SIDE it is drawn on,
		// because the counts alone would not have moved — a Section is neither
		// an ingredient nor a step, so a count-only assertion here witnesses
		// nothing at all.
		expect(pasteSides()).toEqual([3, 2]);
		await fireEvent.click(screen.getByRole('button', { name: 'A line later' }));
		expect(pasteSides()).toEqual([4, 1]);

		// All the way to the end: the method empties, and says so in its own
		// words rather than in the ingredients'.
		await fireEvent.click(screen.getByRole('button', { name: 'A line later' }));
		expect(pasteSides()).toEqual([5, 0]);
		// Scoped to the sheet: the page underneath is an empty recipe, so it is
		// saying the same thing for its own reasons.
		expect(within(screen.getByRole('dialog')).getByText('No method yet.')).toBeInTheDocument();

		// And a long way, in one tap, by naming the line the method starts at:
		// back from four to two, which a screen offering only the two buttons
		// would have taken two taps to do.
		const rows = screen.getAllByRole('button', { name: 'Start the method at this line' });
		await fireEvent.click(rows[2] as HTMLElement);
		expect(pasteSides()).toEqual([2, 3]);
		await fireEvent.click(screen.getByRole('button', { name: 'Use it' }));

		// Every line is still there, in the order it was pasted. Only where the
		// two lists part has moved.
		expect(rowFields().map((field) => field.value)).toEqual([
			'Dan Dan Sauce:',
			'2 tbsp Chinese sesame paste',
			'chilli oil, to taste',
			'Assemble:',
			'Mix the sauce and pour it over the noodles.',
		]);
		expect(screen.getAllByRole('textbox', { name: /^Ingredient line/ })).toHaveLength(1);
		expect(screen.getAllByRole('textbox', { name: /^Step/ })).toHaveLength(2);

		// THE WHOLE POINT: moving it never asked again.
		expect(reads(kamosu)).toHaveLength(1);
	});

	it('says what it replaces on a recipe that already holds something', async () => {
		// Reached through `⋯` rather than in the open, because pasting over a
		// recipe replaces it.
		const { kamosu } = renderWriting(WITH_HEADINGS);
		await screen.findByRole('textbox', { name: 'Ingredient line 1' });
		await fireEvent.click(screen.getByRole('button', { name: 'More' }));
		await pasteIn();

		// The three ingredients and two steps of Dan Dan Noodles, named before
		// they go.
		expect(
			screen.getByText(/replaces the 3 ingredients and 2 steps already here/),
		).toBeInTheDocument();
		expect(reads(kamosu)).toHaveLength(1);
	});

	it('says the title goes too, on a recipe that is only a title', async () => {
		// `create_recipe` takes a title and nothing else, so this is the state
		// every brand-new recipe is in. The two lists are empty, so the offer
		// is still in the open — but a paste carrying its own title would
		// replace one somebody typed a moment ago, and that is a loss of its
		// own rather than part of the lists'.
		renderWriting(WITH_HEADINGS, true, [], { ...empty(), title: 'Tuesday supper' });
		await pasteIn();
		expect(screen.getByText(/also replaces the title Tuesday supper/)).toBeInTheDocument();
		expect(screen.queryByText(/ingredients and .* steps already here/)).not.toBeInTheDocument();
	});

	it('says nothing about replacing anything when the recipe is empty', async () => {
		renderWriting(WITH_HEADINGS, true, [], empty());
		await pasteIn();
		expect(screen.queryByText(/replaces/)).not.toBeInTheDocument();
		expect(screen.getByText('Nothing is saved until you save.')).toBeInTheDocument();
	});

	it('saves nothing by itself — the paste fills the page and stops', async () => {
		const { kamosu } = renderWriting(WITH_HEADINGS, true, [], empty());
		await pasteIn();
		await fireEvent.click(screen.getByRole('button', { name: 'Use it' }));
		expect(kamosu.calls.some((call) => call.operation === 'save_recipe_version')).toBe(false);
		expect(kamosu.calls.some((call) => call.operation === 'create_recipe')).toBe(false);
	});

	it('says so when the paste could not be read', async () => {
		renderWriting(
			{ read_pasted_recipe: { refuse: 'bad_request', message: 'too long' } } as Answers,
			true,
			[],
			empty(),
		);
		await fireEvent.click(screen.getByRole('button', { name: 'Paste a whole recipe' }));
		const field = await screen.findByRole('textbox', { name: 'The recipe, pasted as text' });
		await fireEvent.input(field, { target: { value: 'something' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Read it' }));
		// By its text rather than by `role="alert"`: an empty recipe is already
		// showing one, since it has no title yet.
		expect(await screen.findByText('too long')).toBeInTheDocument();
	});
});

// ---- translating (#106, ADR 0006) ---------------------------------------

describe('translating a recipe', () => {
	it('is the same screen, saying what it is doing and what the save will make', async () => {
		// A translation is a recipe, so it is written where recipes are written.
		// The draft opens on the source's own words, which is what a person
		// replaces — so without the screen saying so, the only difference from
		// an ordinary edit would be the button, by which point they have
		// retyped the recipe.
		renderWriting({ ...TRANSLATED }, true, [], content(), 'fr');

		expect(await screen.findByText('Translating into French')).toBeInTheDocument();
		await fireEvent.click(
			(await screen.findAllByRole('button', { name: /Save the translation/ }))[0] as HTMLElement,
		);
		expect(
			await screen.findByText(
				'This makes a new recipe in the same family, written in French. The one you are translating is untouched.',
			),
		).toBeInTheDocument();
	});

	it('calls start_translation with the Language, and never save_recipe_version', async () => {
		const { kamosu, onSaved } = renderWriting({ ...TRANSLATED }, true, [], content(), 'fr');

		await screen.findByRole('textbox', { name: 'Ingredient line 1' });
		await saveThrough(/Save the translation/);

		const asked = kamosu.calls.map((call) => call.operation);
		expect(asked).toContain('start_translation');
		// The recipe being translated is not touched. A Version on it would be
		// an edit nobody asked for.
		expect(asked).not.toContain('save_recipe_version');

		const input = kamosu.calls.find((call) => call.operation === 'start_translation')
			?.input as Record<string, unknown>;
		expect(input.language).toBe('fr');
		expect(input.branch_id).toBe('mine');

		// It lands on a Branch of its own, and answers no offer: it declared
		// the Language it is written in, so there is nothing to put to the cook.
		expect(onSaved).toHaveBeenCalledWith({
			branch_id: 'b_fr',
			collapsed: false,
			copied: false,
			named: true,
			language_offer: null,
		});
	});

	it('is never dressed as a fork, whoever writes the recipe being translated', async () => {
		// `start_translation` makes a Branch of the same Lineage in your own
		// Cookbook whoever writes the source, so #54's two sentences — save
		// onto this one, start my own copy — are both wrong here.
		renderWriting({ ...TRANSLATED }, false, [], content(), 'fr');

		expect(await screen.findByText('Translating into French')).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: /Start my own copy/ })).not.toBeInTheDocument();
	});
});

describe('the Language offer a save answers', () => {
	it('hands the offered Language up, rather than dropping it', async () => {
		// The whole of #106's cause: the Core answers this on every save and
		// no production code in ui/ read it, so the offered half of ADR 0006
		// never happened for anybody using a browser.
		const { onSaved } = renderWriting({
			save_recipe_version: {
				branch_id: 'mine',
				version_id: 'v_2',
				parent_version_id: 'v_1',
				sequence: 2,
				collapsed: false,
				copied: false,
				language: 'en',
				language_offer: 'fr',
				translates_version_id: null,
			},
		} as Answers);

		await screen.findByRole('textbox', { name: 'Ingredient line 1' });
		await saveThrough(/Save onto mine/);

		expect(onSaved).toHaveBeenCalledWith(expect.objectContaining({ language_offer: 'fr' }));
	});
});
