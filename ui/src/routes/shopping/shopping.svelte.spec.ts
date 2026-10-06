/**
 * The screen-seam test: Shopping, against a stand-in Kamosu (#73).
 *
 * The stand-in checks every answer below against the shape the Catalogue
 * declares before the screen sees it, so a test here can lie about the values
 * and never about the shape — rename a field in `src/catalogue.rs` and this
 * fails in the same commit.
 *
 * What it is really for is the half a behaviour test cannot reach, because
 * none of it is an Operation's answer: that a row the arithmetic could not
 * close **breaks open into the recipes that fed it** rather than reading
 * `about 30 ml + 4 cloves` (the shape Aurélien chose on 2026-09-01, recorded on
 * #73), that the written lines are one tap away, that a Loose Item shows no
 * amount at all, that a recipe gone away keeps its name and says why, and that
 * **nothing on this screen can be ticked**.
 */

import { describe, expect, it } from 'vitest';
import { screen, within, fireEvent } from '@testing-library/svelte';
import Page from './+page.svelte';
import { renderScreen } from '../../testing/render';
import type { GetShoppingListOutput } from '$lib/api/catalogue';

type Row = GetShoppingListOutput['rows'][number];
type Chosen = GetShoppingListOutput['chosen'][number];

const chosen = (over: Partial<Chosen> = {}): Chosen => ({
	branch_id: 'b_chicken',
	title: 'Korean Fried Chicken',
	gone: false,
	shopping_yield: null,
	written_yield: { amount: '4', noun: 'servings' },
	...over,
});

/** A row that added: one amount, one line. */
const added = (): Row => ({
	id: 'f_soy',
	kind: 'food',
	name: 'soy sauce',
	name_language: 'en',
	parts: [{ kind: 'about', text: 'about 45 ml', sources: ['Korean Fried Chicken', 'Coq au Vin'] }],
	lines: [
		{ branch_id: 'b_chicken', recipe: 'Korean Fried Chicken', text: '2 tbsp soy sauce' },
		{ branch_id: 'b_coq', recipe: 'Coq au Vin', text: '1 tbsp soy sauce' },
	],
	said: null,
});

/** The real corpus case the whole design decision was about. */
const sideBySide = (): Row => ({
	id: 'f_garlic',
	kind: 'food',
	name: 'minced garlic',
	name_language: 'en',
	parts: [
		{ kind: 'about', text: 'about 30 ml', sources: ['Korean Fried Chicken'] },
		{ kind: 'as_written', text: '4 cloves', sources: ['Coq au Vin'] },
	],
	lines: [
		{ branch_id: 'b_chicken', recipe: 'Korean Fried Chicken', text: '2 tbsp minced garlic' },
		{ branch_id: 'b_coq', recipe: 'Coq au Vin', text: '4 cloves minced garlic' },
	],
	said: null,
});

const unstated = (): Row => ({
	id: 'f_oil',
	kind: 'food',
	name: 'olive oil',
	name_language: 'en',
	parts: [{ kind: 'no_amount', text: 'some', sources: ['Coq au Vin'] }],
	lines: [{ branch_id: 'b_coq', recipe: 'Coq au Vin', text: 'olive oil' }],
	said: null,
});

const loose = (): Row => ({
	id: 'i_bags',
	kind: 'loose',
	name: 'bin bags',
	name_language: null,
	parts: [],
	lines: [],
	said: null,
});

/**
 * A Component's written line, kept on the list because Kamosu could not open
 * the recipe it names — the one row that buys nothing and says why (#86).
 */
const unopened = (): Row => ({
	id: 'b_pizza:0',
	kind: 'line',
	name: 'Dough for 2 pizzas',
	name_language: null,
	parts: [],
	lines: [{ branch_id: 'b_pizza', recipe: 'Pizza Margherita', text: 'Dough for 2 pizzas' }],
	said: 'Kamosu does not have this recipe.',
});

const list = (over: Partial<GetShoppingListOutput> = {}): GetShoppingListOutput => ({
	chosen: [chosen()],
	rows: [added(), sideBySide(), unstated(), loose()],
	...over,
});

/** The `<li>` one row is drawn in, found by the name it carries. */
const rowFor = (name: string): HTMLElement => {
	const said = screen.getByText(name);
	const row = said.closest('li');
	if (!row) throw new Error(`no row around ${name}`);
	return row;
};

describe('Shopping', () => {
	it('says what the list is for when there is nothing on it', async () => {
		renderScreen(Page, { get_shopping_list: { chosen: [], rows: [] } });
		expect(await screen.findByText(/Choose the recipes you'll cook/)).toBeInTheDocument();
	});

	it('says a multiplier from the recipe page as one (#109)', async () => {
		renderScreen(Page, {
			get_shopping_list: list({
				chosen: [chosen({ shopping_yield: { amount: '2', noun: '' }, written_yield: null })],
			}),
		});
		expect(await screen.findByText('shopping for ×2')).toBeInTheDocument();
	});

	it('draws a row that added on one line, with its amount', async () => {
		renderScreen(Page, { get_shopping_list: list() });
		await screen.findByText('soy sauce');
		expect(within(rowFor('soy sauce')).getByText('about 45 ml')).toBeInTheDocument();
	});

	it('breaks a row the arithmetic could not close open into the recipes that fed it', async () => {
		renderScreen(Page, { get_shopping_list: list() });
		await screen.findByText('minced garlic');
		const row = within(rowFor('minced garlic'));

		// Both amounts, each under the name of the recipe that wanted it. This
		// is the whole of the chosen shape: `about 30 ml + 4 cloves` on one
		// line would say two amounts and not which dish goes short.
		expect(row.getByText('about 30 ml')).toBeInTheDocument();
		expect(row.getByText('4 cloves')).toBeInTheDocument();
		expect(row.getByText('Korean Fried Chicken')).toBeInTheDocument();
		expect(row.getByText('Coq au Vin')).toBeInTheDocument();
	});

	it('says some where nobody wrote an amount, in the slot every amount uses', async () => {
		renderScreen(Page, { get_shopping_list: list() });
		await screen.findByText('olive oil');
		expect(within(rowFor('olive oil')).getByText('some')).toBeInTheDocument();
	});

	it('shows a Loose Item with no amount at all', async () => {
		renderScreen(Page, { get_shopping_list: list() });
		await screen.findByText('bin bags');
		// Nothing beside it: it was typed, never read, and merges with nothing.
		expect(rowFor('bin bags').textContent?.trim()).toBe('bin bags');
	});

	it('says why a line standing for a recipe Kamosu has not buys nothing', async () => {
		// Without the sentence this row reads exactly like a line Kamosu could
		// not interpret, and nothing on the list tells the shopper that the
		// dough's flour and water are their own problem (ADR 0008, #86).
		renderScreen(Page, { get_shopping_list: list({ rows: [unopened(), added()] }) });
		await screen.findByText('Dough for 2 pizzas');
		const row = rowFor('Dough for 2 pizzas');
		expect(within(row).getByText('Kamosu does not have this recipe.')).toBeInTheDocument();
		// And a row Kamosu did read carries no such sentence.
		expect(rowFor('soy sauce').textContent).not.toContain('Kamosu does not have');
	});

	it('puts the written lines one tap away, whole and under their own recipes', async () => {
		renderScreen(Page, { get_shopping_list: list() });
		await screen.findByText('minced garlic');

		// Nothing is shown until it is asked for.
		expect(screen.queryByText('2 tbsp minced garlic')).not.toBeInTheDocument();

		await fireEvent.click(
			screen.getByRole('button', { name: /written lines behind minced garlic/i }),
		);
		expect(await screen.findByText('2 tbsp minced garlic')).toBeInTheDocument();
		expect(screen.getByText('4 cloves minced garlic')).toBeInTheDocument();

		// One at a time: opening another closes this one. This is a screen read
		// one-handed in a shop, not a page to unfold.
		await fireEvent.click(screen.getByRole('button', { name: /written lines behind soy sauce/i }));
		expect(screen.queryByText('2 tbsp minced garlic')).not.toBeInTheDocument();
		expect(screen.getByText('1 tbsp soy sauce')).toBeInTheDocument();
	});

	it('keeps a recipe that has gone away, with the name it was known by and why', async () => {
		renderScreen(Page, {
			get_shopping_list: list({
				chosen: [
					chosen(),
					chosen({
						branch_id: 'b_pasta',
						title: 'Fresh Pasta with Pistachio–Anchovy Pesto',
						gone: true,
						written_yield: null,
					}),
				],
			}),
		});
		await screen.findByText('Fresh Pasta with Pistachio–Anchovy Pesto');
		expect(screen.getByText('can no longer be read')).toBeInTheDocument();

		// It is not a link: there is nothing to open.
		expect(
			screen.queryByRole('link', { name: 'Fresh Pasta with Pistachio–Anchovy Pesto' }),
		).not.toBeInTheDocument();
	});

	it('takes a recipe off the list and redraws from the one answer that came back', async () => {
		const { kamosu } = renderScreen(Page, {
			get_shopping_list: list(),
			remove_from_shopping_list: { chosen: [], rows: [loose()] },
		});
		await screen.findByText('soy sauce');

		await fireEvent.click(
			screen.getByRole('button', { name: /Take Korean Fried Chicken off the list/i }),
		);
		expect(await screen.findByText('bin bags')).toBeInTheDocument();
		expect(screen.queryByText('soy sauce')).not.toBeInTheDocument();
		expect(kamosu.calls.at(-1)).toEqual({
			operation: 'remove_from_shopping_list',
			input: { branch_id: 'b_chicken' },
		});
	});

	it('types a Loose Item exactly as typed', async () => {
		const { kamosu } = renderScreen(Page, {
			get_shopping_list: list({ chosen: [], rows: [] }),
			add_loose_item: list({ chosen: [], rows: [loose()] }),
		});
		await screen.findByRole('button', { name: /Add something of your own/i });
		await fireEvent.click(screen.getByRole('button', { name: /Add something of your own/i }));

		const field = screen.getByLabelText('What to buy');
		await fireEvent.input(field, { target: { value: '  bin bags  ' } });
		await fireEvent.submit(field.closest('form') as HTMLFormElement);

		// Trimmed of the accident of typing, and otherwise untouched: Kamosu
		// never reads a Loose Item.
		expect(kamosu.calls.at(-1)).toEqual({
			operation: 'add_loose_item',
			input: { text: 'bin bags' },
		});
		expect(await screen.findByText('bin bags')).toBeInTheDocument();
	});

	it('offers nothing to tick', async () => {
		renderScreen(Page, { get_shopping_list: list() });
		await screen.findByText('soy sauce');
		// ADR 0024: a computed row has no name to staple a tick to. The list
		// leaves as text and something else carries it round the shop.
		expect(screen.queryAllByRole('checkbox')).toHaveLength(0);
	});

	it('sends the list out as text and lets go of it', async () => {
		// ADR 0024's other end, and the reason nothing here is ticked: Kamosu
		// decides what to buy, and something else carries it round the shop.
		const written: string[] = [];
		Object.assign(navigator, {
			clipboard: {
				writeText: (text: string) => {
					written.push(text);
					return Promise.resolve();
				},
			},
		});
		const { kamosu } = renderScreen(Page, {
			get_shopping_list: list(),
			shopping_list_as_text: {
				text: '2026-09-01 · Korean Fried Chicken\nsoy sauce — about 45 ml\n',
			},
		});
		await screen.findByText('soy sauce');

		await fireEvent.click(screen.getByRole('button', { name: /Copy the list/i }));
		expect(
			await screen.findByText(/Paste it wherever you keep your shopping/i),
		).toBeInTheDocument();
		expect(written).toEqual(['2026-09-01 · Korean Fried Chicken\nsoy sauce — about 45 ml\n']);
		expect(kamosu.calls.at(-1)).toEqual({ operation: 'shopping_list_as_text', input: {} });

		// **Kamosu offers and does not act.** The list is still whole until the
		// offer is taken: emptying on the way out would be silent and
		// unrecoverable.
		expect(screen.getByText('Empty the list?')).toBeInTheDocument();
		expect(screen.getByText('soy sauce')).toBeInTheDocument();
	});

	it('empties the list only when the offer is taken, and never on the way out', async () => {
		Object.assign(navigator, { clipboard: { writeText: () => Promise.resolve() } });
		const { kamosu } = renderScreen(Page, {
			get_shopping_list: list(),
			shopping_list_as_text: { text: 'a list' },
			empty_shopping_list: { chosen: [], rows: [] },
		});
		await screen.findByText('soy sauce');
		await fireEvent.click(screen.getByRole('button', { name: /Copy the list/i }));

		// Waving the offer off leaves everything where it was, and asks nothing.
		await fireEvent.click(await screen.findByRole('button', { name: 'Kept.' }));
		expect(screen.queryByText('Empty the list?')).not.toBeInTheDocument();
		expect(screen.getByText('soy sauce')).toBeInTheDocument();
		expect(kamosu.calls.at(-1)?.operation).toBe('shopping_list_as_text');

		// Taking it empties the choosing and the typed lines together.
		await fireEvent.click(screen.getByRole('button', { name: /Copy the list/i }));
		await fireEvent.click(await screen.findByRole('button', { name: 'Empty it' }));
		expect(kamosu.calls.at(-1)).toEqual({ operation: 'empty_shopping_list', input: {} });
		expect(await screen.findByText(/Choose the recipes you'll cook/)).toBeInTheDocument();
	});

	it('says how much of a recipe is being shopped for, and lets it be changed', async () => {
		const { kamosu } = renderScreen(Page, {
			get_shopping_list: list(),
			set_shopping_yield: list({
				chosen: [chosen({ shopping_yield: { amount: '8', noun: 'servings' } })],
			}),
		});
		await screen.findByText('soy sauce');

		await fireEvent.click(
			screen.getByRole('button', { name: /How much of Korean Fried Chicken/i }),
		);
		await fireEvent.input(screen.getByLabelText('How many'), { target: { value: '8' } });
		await fireEvent.input(screen.getByLabelText('Of what'), { target: { value: 'servings' } });
		await fireEvent.submit(screen.getByLabelText('How many').closest('form') as HTMLFormElement);

		expect(kamosu.calls.at(-1)).toEqual({
			operation: 'set_shopping_yield',
			input: { branch_id: 'b_chicken', shopping_yield: { amount: '8', noun: 'servings' } },
		});
		expect(await screen.findByText(/shopping for 8 servings/i)).toBeInTheDocument();
	});

	it('takes an emptied amount as the recipe as written, never as zero', async () => {
		const { kamosu } = renderScreen(Page, {
			get_shopping_list: list({
				chosen: [chosen({ shopping_yield: { amount: '8', noun: 'servings' } })],
			}),
			set_shopping_yield: list(),
		});
		await screen.findByText('soy sauce');

		await fireEvent.click(
			screen.getByRole('button', { name: /How much of Korean Fried Chicken/i }),
		);
		await fireEvent.input(screen.getByLabelText('How many'), { target: { value: '  ' } });
		await fireEvent.submit(screen.getByLabelText('How many').closest('form') as HTMLFormElement);

		// `null`, which is what the Operation takes for *as written*. Not "0",
		// and not a guess: a recipe has no zero.
		expect(kamosu.calls.at(-1)).toEqual({
			operation: 'set_shopping_yield',
			input: { branch_id: 'b_chicken', shopping_yield: null },
		});
	});

	it('says so when the list cannot be read', async () => {
		renderScreen(Page, { get_shopping_list: { refuse: 'internal' } });
		expect(await screen.findByRole('alert')).toHaveTextContent(/Couldn't load your list/i);
	});

	it('has no recipes side on the phone until a recipe is chosen', async () => {
		renderScreen(Page, { get_shopping_list: { chosen: [], rows: [loose()] } });
		await screen.findByText('bin bags');
		expect(screen.queryByRole('heading', { name: 'Chosen' })).not.toBeInTheDocument();
	});

	it('draws no − and + on the phone', async () => {
		renderScreen(Page, { get_shopping_list: list() });
		await screen.findByText('soy sauce');
		expect(screen.queryByRole('button', { name: /One more of/ })).not.toBeInTheDocument();
		expect(screen.getByRole('button', { name: /How much of Korean/ })).toHaveTextContent(
			'shopping for 4 servings',
		);
		expect(screen.getByRole('button', { name: /Take Korean Fried Chicken off/ })).toHaveTextContent(
			'Take off the list',
		);
	});
});

/**
 * The wide layout (#202, ADR 0044): the recipes chosen on one side and what to
 * buy on the other. Which column a thing lands in is the stylesheet's and is
 * checked live; what each side holds and does is checked here.
 */
describe('Shopping, on the wide layout', () => {
	const wide = (answers: Parameters<typeof renderScreen>[1]) =>
		renderScreen(Page, answers, undefined, 'wide');

	/** The side one heading stands over. */
	const side = (heading: string): HTMLElement => {
		const section = screen.getByRole('heading', { name: heading }).closest('section');
		if (!section) throw new Error(`no side under ${heading}`);
		return section;
	};

	it('holds the recipes on one side and what to buy on the other', async () => {
		wide({ get_shopping_list: list() });
		await screen.findByText('soy sauce');

		expect(
			within(side('Chosen')).getByRole('link', { name: 'Korean Fried Chicken' }),
		).toBeVisible();
		expect(within(side('Chosen')).queryByText('soy sauce')).not.toBeInTheDocument();
		expect(within(side('To buy')).getByText('soy sauce')).toBeVisible();
		expect(within(side('To buy')).getByText('bin bags')).toBeVisible();
		expect(
			within(side('To buy')).queryByText('Korean Fried Chicken', { selector: 'a' }),
		).toBeNull();
	});

	it('keeps the recipes side standing with nothing chosen, saying what the list is for', async () => {
		wide({ get_shopping_list: { chosen: [], rows: [] } });
		await screen.findByRole('heading', { name: 'To buy' });

		expect(within(side('Chosen')).getByText(/Choose the recipes you'll cook/)).toBeVisible();
		expect(within(side('Chosen')).getByRole('link', { name: 'Add a recipe' })).toBeVisible();
		// Once, and on the recipes' side.
		expect(screen.getAllByText(/Choose the recipes you'll cook/)).toHaveLength(1);
	});

	it('shops for one more of a recipe at a tap on +', async () => {
		const { kamosu } = wide({
			get_shopping_list: list(),
			set_shopping_yield: list({
				chosen: [chosen({ shopping_yield: { amount: '5', noun: 'servings' } })],
			}),
		});
		await screen.findByText('soy sauce');

		await fireEvent.click(screen.getByRole('button', { name: 'One more of Korean Fried Chicken' }));
		expect(kamosu.calls.at(-1)).toEqual({
			operation: 'set_shopping_yield',
			input: { branch_id: 'b_chicken', shopping_yield: { amount: '5', noun: 'servings' } },
		});
		expect(
			await within(side('Chosen')).findByRole('button', { name: /How much of Korean/ }),
		).toHaveTextContent('5 servings');
	});

	it('counts two quick taps on + as two servings', async () => {
		// The second tap lands before the Core has answered the first, and has
		// to step from five, not from the four still in the last answer.
		const { kamosu } = wide({
			get_shopping_list: list(),
			set_shopping_yield: list({
				chosen: [chosen({ shopping_yield: { amount: '6', noun: 'servings' } })],
			}),
		});
		await screen.findByText('soy sauce');

		const more = screen.getByRole('button', { name: 'One more of Korean Fried Chicken' });
		void fireEvent.click(more);
		await fireEvent.click(more);
		expect(
			kamosu.calls.filter((call) => call.operation === 'set_shopping_yield').map((c) => c.input),
		).toEqual([
			{ branch_id: 'b_chicken', shopping_yield: { amount: '5', noun: 'servings' } },
			{ branch_id: 'b_chicken', shopping_yield: { amount: '6', noun: 'servings' } },
		]);
	});

	it('puts the amount back when the Core refuses a step', async () => {
		wide({ get_shopping_list: list(), set_shopping_yield: { refuse: 'internal' } });
		await screen.findByText('soy sauce');
		await fireEvent.click(screen.getByRole('button', { name: 'One more of Korean Fried Chicken' }));
		expect(await screen.findByRole('alert')).toBeVisible();
		expect(screen.getByRole('button', { name: /How much of Korean/ })).toHaveTextContent(
			'4 servings',
		);
	});

	it('goes back to the recipe as written when − lands on what it makes', async () => {
		const { kamosu } = wide({
			get_shopping_list: list({
				chosen: [chosen({ shopping_yield: { amount: '5', noun: 'servings' } })],
			}),
			set_shopping_yield: list(),
		});
		await screen.findByText('soy sauce');
		// A recipe that has moved says where it started.
		expect(screen.getByText('the recipe is written for 4 servings')).toBeVisible();

		await fireEvent.click(
			screen.getByRole('button', { name: 'One fewer of Korean Fried Chicken' }),
		);
		// `null`, the recipe as written, and never "4 servings" stored over it.
		expect(kamosu.calls.at(-1)).toEqual({
			operation: 'set_shopping_yield',
			input: { branch_id: 'b_chicken', shopping_yield: null },
		});
		await screen.findByText('4 servings');
		expect(screen.queryByText(/the recipe is written for/)).not.toBeInTheDocument();
	});

	it('will not step below one', async () => {
		wide({
			get_shopping_list: list({
				chosen: [chosen({ written_yield: { amount: '1', noun: 'servings' } })],
			}),
		});
		await screen.findByText('soy sauce');
		expect(
			screen.getByRole('button', { name: 'One fewer of Korean Fried Chicken' }),
		).toBeDisabled();
		expect(screen.getByRole('button', { name: 'One more of Korean Fried Chicken' })).toBeEnabled();
	});

	it('opens the two boxes at a tap on the amount between − and +', async () => {
		const { kamosu } = wide({
			get_shopping_list: list(),
			set_shopping_yield: list({
				chosen: [chosen({ shopping_yield: { amount: '2', noun: 'trays' } })],
			}),
		});
		await screen.findByText('soy sauce');

		await fireEvent.click(
			screen.getByRole('button', { name: /How much of Korean Fried Chicken/i }),
		);
		await fireEvent.input(screen.getByLabelText('How many'), { target: { value: '2' } });
		await fireEvent.input(screen.getByLabelText('Of what'), { target: { value: 'trays' } });
		await fireEvent.submit(screen.getByLabelText('How many').closest('form') as HTMLFormElement);

		expect(kamosu.calls.at(-1)).toEqual({
			operation: 'set_shopping_yield',
			input: { branch_id: 'b_chicken', shopping_yield: { amount: '2', noun: 'trays' } },
		});
	});

	it('draws no − and + where what is shopped for is counted in another noun', async () => {
		// One more than two trays of a recipe written for four servings would
		// have to say five servings.
		wide({
			get_shopping_list: list({
				chosen: [chosen({ shopping_yield: { amount: '2', noun: 'trays' } })],
			}),
		});
		await screen.findByText('soy sauce');
		expect(screen.queryByRole('button', { name: /One more of/ })).not.toBeInTheDocument();
		expect(screen.getByRole('button', { name: /How much of Korean/ })).toHaveTextContent(
			'shopping for 2 trays',
		);
		expect(screen.getByText('the recipe is written for 4 servings')).toBeVisible();
	});

	it('draws no − and + where there is no whole number to step from', async () => {
		wide({
			get_shopping_list: list({
				chosen: [
					chosen({ written_yield: null }),
					chosen({
						branch_id: 'b_loaf',
						title: 'Banana Loaf',
						written_yield: { amount: '1½', noun: 'loaves' },
					}),
					chosen({ branch_id: 'b_gone', title: 'Lost Soup', gone: true }),
				],
			}),
		});
		await screen.findByText('soy sauce');

		expect(screen.queryByRole('button', { name: /One more of/ })).not.toBeInTheDocument();
		// The sentence that opens the two boxes is still there for both.
		expect(
			screen.getByRole('button', { name: 'How much of Korean Fried Chicken' }),
		).toHaveTextContent('Change how much');
		expect(screen.getByRole('button', { name: 'How much of Banana Loaf' })).toHaveTextContent(
			'shopping for 1½ loaves',
		);
		expect(screen.queryByRole('button', { name: 'How much of Lost Soup' })).toBeNull();
	});

	it('takes a recipe off the list as the phone does, under a shorter word', async () => {
		const { kamosu } = wide({
			get_shopping_list: list(),
			remove_from_shopping_list: { chosen: [], rows: [loose()] },
		});
		await screen.findByText('soy sauce');

		const off = screen.getByRole('button', { name: 'Take Korean Fried Chicken off the list' });
		expect(off.textContent?.trim()).toBe('Take off');
		await fireEvent.click(off);
		expect(kamosu.calls.at(-1)).toEqual({
			operation: 'remove_from_shopping_list',
			input: { branch_id: 'b_chicken' },
		});
		expect(await within(side('To buy')).findByText('bin bags')).toBeVisible();
		expect(screen.queryByText('soy sauce')).not.toBeInTheDocument();
	});

	it('types a Loose Item onto the list side', async () => {
		const { kamosu } = wide({
			get_shopping_list: list({ rows: [added()] }),
			add_loose_item: list({ rows: [added(), loose()] }),
		});
		await screen.findByText('soy sauce');
		await fireEvent.click(
			within(side('To buy')).getByRole('button', { name: /Add something of your own/i }),
		);
		const field = screen.getByLabelText('What to buy');
		await fireEvent.input(field, { target: { value: 'bin bags' } });
		await fireEvent.submit(field.closest('form') as HTMLFormElement);

		expect(kamosu.calls.at(-1)).toEqual({
			operation: 'add_loose_item',
			input: { text: 'bin bags' },
		});
		expect(await within(side('To buy')).findByText('bin bags')).toBeVisible();
	});

	it('opens the written lines behind a row, and still offers nothing to tick', async () => {
		wide({ get_shopping_list: list() });
		await screen.findByText('minced garlic');
		await fireEvent.click(
			screen.getByRole('button', { name: /written lines behind minced garlic/i }),
		);
		expect(await screen.findByText('2 tbsp minced garlic')).toBeVisible();
		expect(screen.queryAllByRole('checkbox')).toHaveLength(0);
	});
});
