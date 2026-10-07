/**
 * The screen-seam test: Home, against a stand-in Kamosu (#64).
 *
 * The stand-in checks every answer below against the shape the Catalogue
 * declares before the screen sees it, so a test here can lie about the values
 * and never about the shape — rename a field in `src/catalogue.rs` and this
 * fails in the same commit.
 *
 * What it is really for is the part a behaviour test cannot reach, because
 * none of it is an Operation's answer: that `/` is Home to a Person and the
 * login form to anybody else, that an omitted shelf leaves no heading behind,
 * that the shelf which says *under 30 minutes* takes the 30 from the Core
 * rather than from a phrase file, and that a brand-new instance is told what to
 * do rather than shown four empty rows.
 */

import { describe, expect, it, vi } from 'vitest';
import { screen, within, fireEvent } from '@testing-library/svelte';
import Page from './+page.svelte';
import { renderScreen } from '../testing/render';
import { writtenByMe } from '../testing/recipes';
import type { GetCookbookOutput, HomeShelvesOutput } from '$lib/api/catalogue';

type Shelf = HomeShelvesOutput['shelves'][number];

/** One card, with everything the Catalogue requires present. */
const card = (over: Record<string, unknown> = {}) => ({
	lineage_id: 'l_1',
	branch_id: 'b_1',
	title: 'Miso Soup',
	language: 'en',
	language_fallback: false,
	main_photo: null,
	yield: null,
	// Nothing was searched for on Home, so nothing matched — the same field the
	// library's shelf fills in, left empty here by the same Operation family.
	matched: null,
	...writtenByMe(),
	...over,
});

/** The reader's Cookbook with no join waiting on them: Home asks for it (#135). */
const ALONE: GetCookbookOutput = {
	id: 'c_1',
	name: null,
	authors: [{ person_id: 'p_1', name: 'Tom' }],
	recipe_count: 3,
	kitchens: [],
	invites: [],
	joins: [],
};

type Join = GetCookbookOutput['joins'][number];

/** Camille, who writes with Tom, said yes to Stéphane's Invite; Stéphane writes with Bob. */
const joinOf = (over: Partial<Join> = {}): Join => ({
	join_id: 'cj_1',
	state: 'waiting',
	accepted_by: { person_id: 'p_2', name: 'Camille' },
	invited_by: { person_id: 'p_3', name: 'Stéphane' },
	joining: {
		id: 'c_1',
		name: null,
		authors: [
			{ person_id: 'p_2', name: 'Camille' },
			{ person_id: 'p_1', name: 'Tom' },
		],
	},
	into: {
		id: 'c_2',
		name: null,
		authors: [
			{ person_id: 'p_3', name: 'Stéphane' },
			{ person_id: 'p_4', name: 'Bob' },
		],
	},
	together_recipes: 42,
	waiting_on: [
		{ person_id: 'p_1', name: 'Tom' },
		{ person_id: 'p_4', name: 'Bob' },
	],
	you: 'asked',
	refused_by: null,
	refused_by_co_author: false,
	...over,
});

const home = (shelves: Shelf[], minutes = 30): HomeShelvesOutput => ({
	quick_tonight_minutes: minutes,
	shelves,
});

describe('a Cookbook join waiting on you (#135)', () => {
	const shelves = home([{ name: 'cooked_most', recipes: [card({ title: 'Katsu Curry' })] }]);

	it('is asked above the shelves, and a yes answers it by the Operation', async () => {
		let answered = false;
		const { kamosu } = renderScreen(Page, {
			home_shelves: shelves,
			get_cookbook: () => (answered ? ALONE : { ...ALONE, joins: [joinOf()] }),
			answer_cookbook_join: () => {
				answered = true;
				return ALONE;
			},
		});

		expect(
			await screen.findByRole('heading', { name: 'Write one Cookbook with Stéphane and Bob?' }),
		).toBeInTheDocument();
		expect(screen.getByText('Stéphane invited Camille, who said yes.')).toBeInTheDocument();
		expect(
			screen.getByText(
				'42 recipes in one Cookbook, which all of you can change. Nothing changes until everyone says yes.',
			),
		).toBeInTheDocument();
		expect(screen.getByText('Katsu Curry')).toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: 'Write together' }));
		expect(kamosu.calls).toContainEqual({
			operation: 'answer_cookbook_join',
			input: { join_id: 'cj_1', yes: true },
		});
		await vi.waitFor(() =>
			expect(
				screen.queryByRole('heading', { name: 'Write one Cookbook with Stéphane and Bob?' }),
			).not.toBeInTheDocument(),
		);
	});

	it('answers no by the Operation too', async () => {
		const { kamosu } = renderScreen(Page, {
			home_shelves: shelves,
			get_cookbook: { ...ALONE, joins: [joinOf()] },
			answer_cookbook_join: ALONE,
		});
		await fireEvent.click(await screen.findByRole('button', { name: 'No' }));
		expect(kamosu.calls).toContainEqual({
			operation: 'answer_cookbook_join',
			input: { join_id: 'cj_1', yes: false },
		});
	});

	it('shows only the question among joins that also wait on others', async () => {
		renderScreen(Page, {
			home_shelves: shelves,
			get_cookbook: {
				...ALONE,
				joins: [joinOf(), joinOf({ join_id: 'cj_2', you: 'invited' })],
			},
		});
		expect(
			await screen.findByRole('heading', { name: 'Write one Cookbook with Stéphane and Bob?' }),
		).toBeInTheDocument();
		expect(screen.queryByText(/^Waiting for/)).not.toBeInTheDocument();
	});

	it('says nothing on Home about a join that is not waiting on you', async () => {
		renderScreen(Page, {
			home_shelves: shelves,
			get_cookbook: { ...ALONE, joins: [joinOf({ you: 'answered' })] },
		});
		expect(await screen.findByText('Katsu Curry')).toBeInTheDocument();
		await vi.waitFor(() =>
			expect(screen.queryByText(/Waiting for|Write one Cookbook/)).not.toBeInTheDocument(),
		);
	});
});

describe('Home', () => {
	it('shows each shelf the Core sent, under its own heading', async () => {
		renderScreen(Page, {
			get_cookbook: ALONE,
			home_shelves: home([
				{ name: 'cooked_most', recipes: [card({ title: 'Katsu Curry' })] },
				{
					name: 'quick_tonight',
					recipes: [card({ lineage_id: 'l_2', branch_id: 'b_2', title: 'Omelette' })],
				},
			]),
		});

		expect(await screen.findByRole('heading', { name: 'Cooked most' })).toBeInTheDocument();
		expect(screen.getByRole('heading', { name: 'Quick tonight' })).toBeInTheDocument();
		expect(screen.getByText('Katsu Curry')).toBeInTheDocument();
		expect(screen.getByText('Omelette')).toBeInTheDocument();
	});

	it('leaves no heading behind for a shelf the Core did not send', async () => {
		renderScreen(Page, {
			get_cookbook: ALONE,
			home_shelves: home([{ name: 'never_cooked', recipes: [card()] }]),
		});

		expect(await screen.findByRole('heading', { name: 'Never cooked' })).toBeInTheDocument();
		// An empty shelf is omitted rather than shown empty. The Core decides
		// that, so the screen has no case where a heading draws itself over
		// nothing — and this is what proves the absence is a real absence.
		expect(screen.queryByRole('heading', { name: 'Cooked most' })).not.toBeInTheDocument();
		expect(screen.queryByRole('heading', { name: 'Quick tonight' })).not.toBeInTheDocument();
		expect(screen.queryByRole('heading', { name: 'Recently opened' })).not.toBeInTheDocument();
		expect(screen.queryByRole('heading', { name: 'Recently added' })).not.toBeInTheDocument();
	});

	it('leads with what was recently added, newest first, in the order the Core sent', async () => {
		renderScreen(Page, {
			get_cookbook: ALONE,
			home_shelves: home([
				{
					name: 'recently_added',
					recipes: [
						card({ lineage_id: 'l_2', branch_id: 'b_2', title: 'Tarte Tatin' }),
						card({ title: 'Miso Soup' }),
					],
				},
				{ name: 'never_cooked', recipes: [card({ title: 'Miso Soup' })] },
			]),
		});

		const heading = await screen.findByRole('heading', { name: 'Recently added' });
		// First on the screen, where #151 put it. The Core decides
		// the order, and the screen draws what it was sent without resorting.
		expect(screen.getAllByRole('heading', { level: 2 }).map((h) => h.textContent?.trim())).toEqual([
			'Recently added',
			'Never cooked',
		]);
		const section = heading.closest('section') as HTMLElement;
		expect(within(section).getByText('What just came in')).toBeInTheDocument();
		expect(
			within(section)
				.getAllByRole('listitem')
				.map((item) => item.textContent),
		).toEqual([expect.stringContaining('Tarte Tatin'), expect.stringContaining('Miso Soup')]);
	});

	it('takes the line quick tonight is drawn at from the Core, never from its own words', async () => {
		renderScreen(Page, {
			get_cookbook: ALONE,
			home_shelves: home([{ name: 'quick_tonight', recipes: [card()] }], 20),
		});

		// The Core said twenty, so the screen says twenty. A number written into
		// the phrase file would go on saying thirty after the Core moved.
		expect(await screen.findByText('On the table inside 20 minutes')).toBeInTheDocument();
		expect(screen.queryByText('On the table inside 30 minutes')).not.toBeInTheDocument();
	});

	it('counts what is on the rail, and does not offer a filtered library that does not exist', async () => {
		renderScreen(Page, {
			get_cookbook: ALONE,
			home_shelves: home([
				{
					name: 'cooked_most',
					recipes: [card(), card({ lineage_id: 'l_2', branch_id: 'b_2', title: 'Pain' })],
				},
			]),
		});

		const heading = await screen.findByRole('heading', { name: 'Cooked most' });
		const section = heading.closest('section') as HTMLElement;
		const cards = within(section).getAllByRole('listitem');

		// The number beside the heading is the number of cards under it — not a
		// total the Core never sent. The Core answers a shelf's worth, cut to
		// length, so a figure claiming to be *all* of them would be wrong the
		// moment a shelf was longer than one screenful.
		expect(within(section).getByText(String(cards.length))).toBeInTheDocument();
		expect(cards).toHaveLength(2);

		// And it is not a link. Recipes filters by Kitchen and by *mine*, never
		// by *cooked most*, so a link there would promise this shelf and deliver
		// the whole library.
		const links = within(section).getAllByRole('link');
		expect(links.every((link) => link.getAttribute('href')?.startsWith('/recipes/'))).toBe(true);
	});

	it('tells a brand-new instance what to do instead of showing four empty rows', async () => {
		renderScreen(Page, {
			get_cookbook: ALONE,
			home_shelves: home([]),
		});

		expect(await screen.findByRole('heading', { name: 'Your shelf is empty' })).toBeInTheDocument();
		// The two things that were going to happen next anyway — and both of
		// them *do* the thing. A link to Recipes would land the first person who
		// ever opened Kamosu on a second empty screen.
		expect(screen.getByLabelText('What is it called?')).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Write a recipe' })).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Import from a link' })).toBeInTheDocument();
		expect(screen.queryByRole('link', { name: 'Write a recipe' })).not.toBeInTheDocument();
		// And not one heading for a shelf that is not there.
		expect(screen.queryByRole('heading', { name: 'Never cooked' })).not.toBeInTheDocument();
	});

	it('writes the recipe from the empty state rather than pointing at a screen to write it on', async () => {
		const { kamosu } = renderScreen(Page, {
			get_cookbook: ALONE,
			home_shelves: home([]),
			create_recipe: {
				branch_id: 'b_new',
				lineage_id: 'l_new',
				cookbook: { id: 'c_1', name: null, authors: [{ person_id: 'p_1', name: 'Stéphane' }] },
				name: null,
				writes: true,
				mine: true,
				arrived: false,
				hand_id: 'h_1',
				language: 'en',
				origin_address: null,
				head_version_id: 'v_new',
				versions: [],
				translation: null,
				tags: [],
				related_recipes: [],
				cooked: { count: 0, last_cooked_at: null, ratings: [] },
			},
		});

		const field = await screen.findByLabelText('What is it called?');
		await fireEvent.input(field, { target: { value: 'Tarte Tatin' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Write a recipe' }));

		// It lands in the writer's own Cookbook, the only place a new recipe
		// can (#131), under the name that was typed: nothing is asked.
		await vi.waitFor(() => {
			const asked = kamosu.calls.find((call) => call.operation === 'create_recipe');
			expect(asked?.input).toEqual({ title: 'Tarte Tatin' });
		});
	});

	it('stands the login form at / for anybody who is not a Person yet', async () => {
		// Refused for want of a Credential is not a failure — it is the answer,
		// and the answer is the form. There is no cookie read anywhere here.
		renderScreen(Page, {
			get_cookbook: ALONE,
			home_shelves: { refuse: 'unauthorized' },
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
		});

		expect(await screen.findByRole('heading', { name: 'Welcome back' })).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Log in' })).toBeInTheDocument();
		expect(screen.queryByRole('heading', { name: 'Home' })).not.toBeInTheDocument();
	});

	it('says so when Kamosu could not be reached, rather than asking you to log in again', async () => {
		// A refusal that is not about a Credential must never be answered with
		// the login form: telling a signed-in cook to sign in again is the worst
		// possible reading of a server that is simply down.
		renderScreen(Page, {
			get_cookbook: ALONE,
			home_shelves: { refuse: 'internal', message: 'boom' },
		});

		expect(
			await screen.findByText('Kamosu could not work out your shelves just now.'),
		).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Log in' })).not.toBeInTheDocument();
	});
});
