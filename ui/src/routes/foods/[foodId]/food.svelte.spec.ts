/**
 * One Food's page (#107), with several names in one Language (#179).
 *
 * The states the tickets name are the ones this file is built around: a Food
 * with a single name, where the last name may not be taken off; a Food named
 * in two Languages, where either may; and a Food answering to two words in
 * one Language, "œufs" and "œuf", shown on one row and edited together.
 *
 * What these guard above everything: **nothing here is recipe content** (ADR
 * 0002). No Version is minted and no written line moves, which is asserted
 * rather than assumed — the stand-in records every ask a screen makes, so a
 * screen that quietly called `save_recipe_version` would be caught here.
 */

import { describe, expect, it } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import type { GetFoodOutput } from '$lib/api/catalogue';
import { m } from '$lib/paraglide/messages';
import FoodTestHarness from './FoodTestHarness.svelte';

type Name = GetFoodOutput['names'][number];

/** One Food, with everything the Catalogue requires present. */
const food = (over: Partial<GetFoodOutput> = {}): GetFoodOutput => ({
	id: 'f_caster',
	name: 'caster sugar',
	language: 'en',
	names: [{ language: 'en', name: 'caster sugar' }] as Name[],
	cup_weight_grams: null,
	// #72 deferred per-ingredient nutrition, and all 607 Foods on the real
	// instance answer null. No editor is built for it and none is expected.
	nutrition: null,
	reading_count: 9,
	...over,
});

/** The Food the ticket asks to be shown: one name, nine Readings, no Cup Weight. */
const CASTER = food();

/** A Food answering to the plural and the singular in French (#179). */
const EGGS = food({
	id: 'f_eggs',
	name: 'eggs',
	names: [
		{ language: 'en', name: 'eggs' },
		{ language: 'fr', name: 'œufs' },
		{ language: 'fr', name: 'œuf' },
	],
	reading_count: 12,
});

/** A Food somebody has taught both its words, which is what ADR 0006 is for. */
const SALT = food({
	id: 'f_salt',
	name: 'salt',
	names: [
		{ language: 'en', name: 'salt' },
		{ language: 'fr', name: 'sel' },
	],
	reading_count: 68,
});

function draw(answers: Answers, foodId = 'f_caster') {
	const kamosu = standIn(answers);
	render(FoodTestHarness, { props: { client: kamosu.client, foodId } });
	return kamosu;
}

describe('a Food’s page', () => {
	it('shows what it is called, what a cup weighs and how many lines point at it', async () => {
		draw({ get_food: CASTER });

		expect(await screen.findByRole('heading', { name: 'caster sugar' })).toBeInTheDocument();
		expect(screen.getByText('9 lines point at this')).toBeInTheDocument();
		// The honest state of a Cup Weight nobody has set: an empty box that
		// says so, rather than a zero that would be a lie about a weight.
		expect(screen.getByLabelText('A cup of it weighs')).toHaveValue('');
		expect(screen.getByPlaceholderText('Kamosu has no figure')).toBeInTheDocument();
	});

	it('counts one Reading as a line rather than as “1 lines”', async () => {
		draw({ get_food: food({ reading_count: 1 }) });

		expect(await screen.findByText('1 line points at this')).toBeInTheDocument();
	});

	it('sets a name in a Language the Food has none in, and draws the Core’s answer', async () => {
		const named = food({
			names: [
				{ language: 'en', name: 'caster sugar' },
				{ language: 'fr', name: 'sucre en poudre' },
			],
		});
		const kamosu = draw({ get_food: CASTER, set_food_names: named });

		expect(await screen.findByText('Not named in French yet')).toBeInTheDocument();
		await fireEvent.click(screen.getByRole('button', { name: 'Add a name in French' }));
		await fireEvent.input(screen.getByPlaceholderText('Another name in French'), {
			target: { value: 'sucre en poudre' },
		});
		await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

		await waitFor(() => expect(screen.getByText('sucre en poudre')).toBeInTheDocument());
		const asked = kamosu.calls.find((call) => call.operation === 'set_food_names');
		expect(asked?.input).toEqual({
			food_id: 'f_caster',
			language: 'fr',
			names: ['sucre en poudre'],
		});
	});

	it('shows every name a Language has on its one row', async () => {
		draw({ get_food: EGGS }, 'f_eggs');

		expect(await screen.findByText('œufs · œuf')).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Change the names in French' })).toBeInTheDocument();
	});

	it('adds a second name in a Language that already has one', async () => {
		const kamosu = draw({ get_food: SALT, set_food_names: EGGS }, 'f_salt');

		await screen.findByText('sel');
		await fireEvent.click(screen.getByRole('button', { name: 'Change the names in French' }));
		expect(screen.getByRole('textbox', { name: 'French name 1' })).toHaveValue('sel');
		await fireEvent.input(screen.getByPlaceholderText('Another name in French'), {
			target: { value: 'gros sel' },
		});
		await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

		await waitFor(() =>
			expect(kamosu.calls.find((call) => call.operation === 'set_food_names')?.input).toEqual({
				food_id: 'f_salt',
				language: 'fr',
				names: ['sel', 'gros sel'],
			}),
		);
	});

	it('takes one of two names off a Language, keeping the other', async () => {
		const kamosu = draw({ get_food: EGGS, set_food_names: EGGS }, 'f_eggs');

		await screen.findByText('œufs · œuf');
		await fireEvent.click(screen.getByRole('button', { name: 'Change the names in French' }));
		await fireEvent.click(screen.getByRole('button', { name: 'Take off œuf' }));
		expect(screen.queryByRole('textbox', { name: 'French name 2' })).not.toBeInTheDocument();
		await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

		await waitFor(() =>
			expect(kamosu.calls.find((call) => call.operation === 'set_food_names')?.input).toEqual({
				food_id: 'f_eggs',
				language: 'fr',
				names: ['œufs'],
			}),
		);
	});

	it('takes a whole Language off a Food another Language still names', async () => {
		const kamosu = draw({ get_food: SALT, set_food_names: food({ id: 'f_salt' }) }, 'f_salt');

		await screen.findByText('sel');
		await fireEvent.click(screen.getByRole('button', { name: 'Change the names in French' }));
		await fireEvent.click(screen.getByRole('button', { name: 'Take off sel' }));
		await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

		await waitFor(() =>
			expect(kamosu.calls.find((call) => call.operation === 'set_food_names')?.input).toEqual({
				food_id: 'f_salt',
				language: 'fr',
				names: [],
			}),
		);
	});

	it('offers no way to take off the only name a Food has, and says why', async () => {
		draw({ get_food: CASTER });

		expect(await screen.findByRole('heading', { name: 'caster sugar' })).toBeInTheDocument();
		expect(
			screen.getByText('A Food needs at least one name, so the last one stays.'),
		).toBeInTheDocument();
		await fireEvent.click(screen.getByRole('button', { name: 'Change the names in English' }));
		expect(screen.getByRole('textbox', { name: 'English name 1' })).toHaveValue('caster sugar');
		expect(screen.queryByRole('button', { name: /^Take off/ })).not.toBeInTheDocument();
	});

	it('lets the only name go once a replacement is typed in the empty box', async () => {
		const kamosu = draw({ get_food: CASTER, set_food_names: CASTER });

		await screen.findByRole('heading', { name: 'caster sugar' });
		await fireEvent.click(screen.getByRole('button', { name: 'Change the names in English' }));
		expect(screen.queryByRole('button', { name: /^Take off/ })).not.toBeInTheDocument();
		await fireEvent.input(screen.getByPlaceholderText('Another name in English'), {
			target: { value: 'caster' },
		});
		await fireEvent.click(await screen.findByRole('button', { name: 'Take off caster sugar' }));
		await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

		await waitFor(() =>
			expect(kamosu.calls.find((call) => call.operation === 'set_food_names')?.input).toEqual({
				food_id: 'f_caster',
				language: 'en',
				names: ['caster'],
			}),
		);
	});

	it('says the Core’s refusal as the sentence it arrived as, not as a code', async () => {
		// The Core writes its refusals to be read by a person, so the screen
		// shows the words rather than paraphrasing them into a category.
		draw(
			{
				get_food: SALT,
				set_food_names: {
					refuse: 'bad_request',
					message: 'a Food must keep at least one name',
				},
			},
			'f_salt',
		);

		expect(await screen.findByText('sel')).toBeInTheDocument();
		await fireEvent.click(screen.getByRole('button', { name: 'Change the names in French' }));
		await fireEvent.click(screen.getByRole('button', { name: 'Take off sel' }));
		await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

		expect(await screen.findByRole('alert')).toHaveTextContent(
			'a Food must keep at least one name',
		);
	});

	it('sets a Cup Weight as a number of grams', async () => {
		const kamosu = draw({ get_food: CASTER, set_food_cup_weight: food({ cup_weight_grams: 200 }) });

		await screen.findByRole('heading', { name: 'caster sugar' });
		await fireEvent.input(screen.getByLabelText('A cup of it weighs'), {
			target: { value: '200' },
		});
		await fireEvent.click(screen.getByRole('button', { name: 'Save what a cup weighs' }));

		await waitFor(() =>
			expect(kamosu.calls.find((call) => call.operation === 'set_food_cup_weight')?.input).toEqual({
				food_id: 'f_caster',
				cup_weight_grams: 200,
			}),
		);
	});

	it('reads a comma as a decimal point, because a French keyboard writes 125,5', async () => {
		const kamosu = draw({
			get_food: CASTER,
			set_food_cup_weight: food({ cup_weight_grams: 125.5 }),
		});

		await screen.findByRole('heading', { name: 'caster sugar' });
		await fireEvent.input(screen.getByLabelText('A cup of it weighs'), {
			target: { value: '125,5' },
		});
		await fireEvent.click(screen.getByRole('button', { name: 'Save what a cup weighs' }));

		await waitFor(() =>
			expect(kamosu.calls.find((call) => call.operation === 'set_food_cup_weight')?.input).toEqual({
				food_id: 'f_caster',
				cup_weight_grams: 125.5,
			}),
		);
	});

	it('clears a Cup Weight back to nothing when the box is emptied', async () => {
		// Empty is an ordinary state and the one every Food starts in — it
		// offers millilitres instead of grams — so clearing must be possible and
		// must not be mistaken for a number that failed to parse.
		const kamosu = draw({
			get_food: food({ cup_weight_grams: 200 }),
			set_food_cup_weight: CASTER,
		});

		await waitFor(() => expect(screen.getByLabelText('A cup of it weighs')).toHaveValue('200'));
		await fireEvent.input(screen.getByLabelText('A cup of it weighs'), { target: { value: '' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Save what a cup weighs' }));

		await waitFor(() =>
			expect(kamosu.calls.find((call) => call.operation === 'set_food_cup_weight')?.input).toEqual({
				food_id: 'f_caster',
				cup_weight_grams: null,
			}),
		);
	});

	it('refuses a Cup Weight that is not a weight, without asking Kamosu', async () => {
		const kamosu = draw({ get_food: CASTER });

		await screen.findByRole('heading', { name: 'caster sugar' });
		await fireEvent.input(screen.getByLabelText('A cup of it weighs'), {
			target: { value: 'a handful' },
		});
		await fireEvent.click(screen.getByRole('button', { name: 'Save what a cup weighs' }));

		expect(await screen.findByRole('alert')).toHaveTextContent(
			'A cup weighs a number of grams, more than zero.',
		);
		expect(kamosu.calls.some((call) => call.operation === 'set_food_cup_weight')).toBe(false);
	});

	it('mints no Version and touches no recipe, whatever is corrected', async () => {
		const kamosu = draw({
			get_food: CASTER,
			set_food_cup_weight: food({ cup_weight_grams: 200 }),
		});

		await screen.findByRole('heading', { name: 'caster sugar' });
		await fireEvent.input(screen.getByLabelText('A cup of it weighs'), {
			target: { value: '200' },
		});
		await fireEvent.click(screen.getByRole('button', { name: 'Save what a cup weighs' }));

		await waitFor(() =>
			expect(kamosu.calls.some((call) => call.operation === 'set_food_cup_weight')).toBe(true),
		);
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('save_recipe_version');
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('set_reading');
		// Says so on the page too, because a screen that quietly edits recipes
		// and one that quietly does not look identical.
		expect(
			screen.getByText('Correcting a Food changes no recipe and no line.'),
		).toBeInTheDocument();
	});

	it('builds no nutrition editor, which #72 deferred', async () => {
		draw({ get_food: CASTER });

		await screen.findByRole('heading', { name: 'caster sugar' });
		expect(screen.queryByText(/nutrition/i)).not.toBeInTheDocument();
	});

	it('offers none of the Operator’s powers, which belong to #103', async () => {
		draw({ get_food: SALT }, 'f_salt');

		await screen.findByRole('heading', { name: 'salt' });
		for (const power of [/merge/i, /delete this food/i]) {
			expect(screen.queryByRole('button', { name: power })).not.toBeInTheDocument();
		}
	});

	it('says so plainly when there is no such Food, rather than showing an empty page', async () => {
		draw({ get_food: { refuse: 'not_found', message: 'no such Food' } });

		expect(await screen.findByRole('heading', { name: 'No such Food' })).toBeInTheDocument();
	});
});

/**
 * One Food after another (#199). On the wide layout the Foods list stays
 * beside the open Food, so the page is handed a second Food without being
 * left, which never happened while a Food was only ever a page of its own.
 */
describe('when another Food is opened in place of this one', () => {
	async function fromCasterToEggs(before: () => Promise<void> = async () => {}) {
		const kamosu = standIn({ get_food: CASTER });
		const drawn = render(FoodTestHarness, { props: { client: kamosu.client, foodId: 'f_caster' } });
		await screen.findByRole('heading', { name: 'caster sugar' });
		await before();

		kamosu.answer('get_food', EGGS);
		await drawn.rerender({ foodId: 'f_eggs' });
		await screen.findByRole('heading', { name: 'eggs' });
		return kamosu;
	}

	it('shows the other Food, read by its own id', async () => {
		const kamosu = await fromCasterToEggs();

		expect(screen.getByText('12 lines point at this')).toBeInTheDocument();
		expect(kamosu.calls.at(-1)).toEqual({ operation: 'get_food', input: { food_id: 'f_eggs' } });
	});

	it('never blinks "Loading" between the two, which an arrow held down would do twenty times', async () => {
		let blinked = false;
		const watch = new MutationObserver(() => {
			blinked ||= screen.queryByRole('heading', { name: m.loading() }) !== null;
		});

		// Watched from the first Food on: the page does say so before that one.
		await fromCasterToEggs(async () =>
			watch.observe(document.body, { subtree: true, childList: true, characterData: true }),
		);
		watch.disconnect();

		expect(blinked).toBe(false);
	});

	it('closes the names that were open for the last Food, which are not this one’s', async () => {
		await fromCasterToEggs(async () => {
			await fireEvent.click(screen.getByRole('button', { name: 'Change the names in English' }));
			expect(screen.getByLabelText('English name 1')).toHaveValue('caster sugar');
		});

		expect(screen.queryByLabelText('English name 1')).toBeNull();
	});

	it('forgets what was said about the last Food', async () => {
		await fromCasterToEggs(async () => {
			await fireEvent.input(screen.getByLabelText('A cup of it weighs'), {
				target: { value: 'a lot' },
			});
			await fireEvent.click(screen.getByRole('button', { name: m.food_cup_weight_save() }));
			expect(screen.getByRole('alert')).toBeInTheDocument();
		});

		expect(screen.queryByRole('alert')).toBeNull();
	});

	it('lets the other Food be corrected once it is on screen', async () => {
		await fromCasterToEggs();

		expect(screen.getByRole('button', { name: m.food_cup_weight_save() })).toBeEnabled();
	});

	it('does not draw a save of the last Food that answers after this one is open', async () => {
		const kamosu = standIn({ get_food: CASTER });
		// The save answers when the test says so, and not before.
		let answer: (food: GetFoodOutput) => void = () => {};
		const slow = {
			...kamosu.client,
			setFoodCupWeight: () => new Promise<GetFoodOutput>((resolve) => (answer = resolve)),
		};
		const drawn = render(FoodTestHarness, { props: { client: slow, foodId: 'f_caster' } });
		await screen.findByRole('heading', { name: 'caster sugar' });
		await fireEvent.input(screen.getByLabelText('A cup of it weighs'), {
			target: { value: '200' },
		});
		await fireEvent.click(screen.getByRole('button', { name: m.food_cup_weight_save() }));

		kamosu.answer('get_food', EGGS);
		await drawn.rerender({ foodId: 'f_eggs' });
		await screen.findByRole('heading', { name: 'eggs' });
		answer(food({ cup_weight_grams: 200 }));
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(screen.getByRole('heading', { name: 'eggs' })).toBeInTheDocument();
		expect(screen.getByLabelText('A cup of it weighs')).toHaveValue('');
		await waitFor(() =>
			expect(screen.getByRole('button', { name: m.food_cup_weight_save() })).toBeEnabled(),
		);
	});
});
