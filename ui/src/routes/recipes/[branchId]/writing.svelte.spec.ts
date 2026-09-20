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
import { render, screen, fireEvent, within } from '@testing-library/svelte';
import { tick } from 'svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import type { GetRecipeOutput } from '$lib/api/catalogue';
import WritingTestHarness from './WritingTestHarness.svelte';

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

const KITCHENS = {
	list_kitchens: {
		kitchens: [
			{
				id: 'k_mine',
				name: 'Chez Aurélien',
				nickname: null,
				is_home: true,
				hand_id: 'h_mine',
				members: [],
			},
		],
	},
} as Answers;

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

function renderWriting(answers: Answers = {}, kitchenId = 'k_mine') {
	const onSaved = vi.fn();
	const onCancel = vi.fn();
	const kamosu = standIn({ ...KITCHENS, ...SAVED, ...answers });
	render(WritingTestHarness, {
		props: { client: kamosu.client, content: content(), kitchenId, onSaved, onCancel },
	});
	return { kamosu, onSaved, onCancel };
}

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

/** Walk the save through the sheet that states the outcome. */
async function saveThrough(name: RegExp) {
	await fireEvent.click(screen.getAllByRole('button', { name })[0] as HTMLElement);
	const inSheet = screen.getAllByRole('button', { name });
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
		const kamosu = standIn({ ...KITCHENS, ...SAVED });
		render(WritingTestHarness, {
			props: { client: kamosu.client, content: content(), kitchenId: 'k_mine', photograph },
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
		const kamosu = standIn({ ...KITCHENS, ...SAVED });
		const bare = content();
		bare.ingredients = [];
		bare.steps = [];
		bare.yield = null;
		bare.prep_time_minutes = null;
		bare.cook_time_minutes = null;
		bare.source = null;
		render(WritingTestHarness, {
			props: { client: kamosu.client, content: bare, kitchenId: 'k_mine' },
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

	it('carries the Nutrition figure through untouched, since it is #84’s to edit', async () => {
		const onSaved = vi.fn();
		const kamosu = standIn({ ...KITCHENS, ...SAVED });
		const held = content();
		held.nutrition = { calories: 308, basis: 'per_serving' };
		render(WritingTestHarness, {
			props: { client: kamosu.client, content: held, kitchenId: 'k_mine', onSaved },
		});

		await screen.findByRole('textbox', { name: 'Ingredient line 1' });
		await saveThrough(/Save onto mine/);
		expect(sent(kamosu)?.nutrition).toEqual({ calories: 308, basis: 'per_serving' });
	});

	it('says it is writing a Version onto yours, and asks the two optional things', async () => {
		const { kamosu } = renderWriting();
		await screen.findByRole('textbox', { name: 'Ingredient line 1' });

		await fireEvent.click(screen.getAllByRole('button', { name: /Save onto mine/ })[0]!);
		// The outcome is stated before it happens, naming the recipe and the
		// Kitchen — "Save" and "Save" are the same word for two different acts.
		expect(
			screen.getByText(/writes a new Version onto your Dan Dan Noodles, in Chez Aurélien/),
		).toBeInTheDocument();

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

	it('says a save onto a Kitchen you do not cook in will fork, and is not the same control', async () => {
		renderWriting({}, 'k_someone_else');
		await screen.findByRole('textbox', { name: 'Ingredient line 1' });

		// The forking save wears its own words. Finding *Save onto mine* here
		// would mean the two acts had been collapsed into one button.
		expect(screen.queryByRole('button', { name: /Save onto mine/ })).not.toBeInTheDocument();
		await fireEvent.click(screen.getAllByRole('button', { name: /Start my own copy/ })[0]!);
		expect(screen.getByText(/is not in one of your Kitchens/)).toBeInTheDocument();
		expect(screen.getByText(/the original stays where it is/)).toBeInTheDocument();
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
			'k_someone_else',
		);

		await screen.findByRole('textbox', { name: 'Ingredient line 1' });
		await saveThrough(/Start my own copy/);
		expect(onSaved).toHaveBeenCalledWith({
			branch_id: 'b_my_copy',
			collapsed: false,
			copied: true,
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

	it('holds the save until it knows which of the two it would be', async () => {
		// `forking` is read off the caller's Kitchens. Unknown, the sheet would
		// name no Kitchen and the server would fork anyway, so the screen says
		// nothing rather than guessing.
		renderWriting({ list_kitchens: { refuse: 'internal' } } as Answers);
		await screen.findByRole('textbox', { name: 'Ingredient line 1' });
		expect(
			await screen.findByRole('button', { name: /could not tell whose Kitchen/ }),
		).toBeDisabled();
		expect(screen.queryByRole('button', { name: /Save onto mine/ })).not.toBeInTheDocument();
		expect(screen.queryByRole('button', { name: /Start my own copy/ })).not.toBeInTheDocument();
	});
});
