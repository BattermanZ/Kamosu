/**
 * The screen-seam test: an older Version opened as a recipe page (#211),
 * against a stand-in Kamosu. The answers below are checked against the
 * Catalogue before the screen sees them.
 */

import { describe, expect, it } from 'vitest';
import { render, screen, fireEvent, within } from '@testing-library/svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import type { ChangedSinceOutput } from '$lib/api/catalogue';
import OlderVersionTestHarness from './OlderVersionTestHarness.svelte';
import { cookbookLabel } from '../../../../testing/recipes';

type Side = ChangedSinceOutput['mine'];
type Lines = Side['content']['ingredients'];
type Steps = Side['content']['steps'];

const NOW: Lines = [
	{ kind: 'ingredient', text: '1 cup potato starch' },
	{ kind: 'ingredient', text: '¼ cup honey' },
	{ kind: 'ingredient', text: '1 Tbsp sesame oil' },
];
const THEN: Lines = [
	{ kind: 'ingredient', text: '¾ cup potato starch' },
	{ kind: 'ingredient', text: '¼ cup honey' },
	{ kind: 'ingredient', text: '1 Tbsp rice vinegar' },
];
const STEPS_NOW: Steps = [
	{ kind: 'step', text: 'Coat the chicken and rest it 30 minutes.', photo: 'p_coat' },
	{ kind: 'step', text: 'Deep fry at 175 C until golden.', photo: null },
];
const STEPS_THEN: Steps = [
	{ kind: 'step', text: 'Coat the chicken.', photo: null },
	{ kind: 'step', text: 'Deep fry at 175 C until golden.', photo: null },
];

const side = (version_id: string, ingredients: Lines, steps: Steps): Side => ({
	branch_id: 'b_1',
	cookbook: cookbookLabel('c_1', ['Stéphane']),
	name: null,
	mine: true,
	arrived: false,
	hand_id: 'p_stephane',
	hand_name: 'Stéphane',
	language: 'en',
	head_version_id: version_id,
	content: {
		title: 'Korean Fried Chicken',
		yield: { amount: '4', noun: 'servings' },
		prep_time_minutes: 20,
		cook_time_minutes: 30,
		note: null,
		main_photo: null,
		nutrition: null,
		source: null,
		ingredients,
		steps,
	},
	readings: ingredients.map(() => null),
	measured: { ingredients: ingredients.map(() => null), steps: steps.map(() => null) },
	components: [],
});

const line = (text: string, index: number) => ({ kind: 'ingredient', text, index });
const step = (text: string, index: number) => ({ kind: 'step', text, index });
const field = (value: unknown) => ({ same: true, mine: value, theirs: value });

/** One Branch at two moments: a line changed, one taken out, one written since. */
function compared(over: Partial<ChangedSinceOutput> = {}): ChangedSinceOutput {
	return {
		lineage_id: 'l_1',
		branch_point_version_id: 'v_then',
		version: {
			version_id: 'v_then',
			sequence: 1,
			name: 'Less sweet',
			change_note: null,
			created_at: '2026-08-14T10:00:00Z',
			hand_id: 'p_stephane',
			hand_name: 'Stéphane',
			newest: false,
		},
		writes: true,
		mine: side('v_now', NOW, STEPS_NOW),
		theirs: side('v_then', THEN, STEPS_THEN),
		ingredients: [
			{
				kind: 'ingredient',
				state: 'changed',
				from_branch_point: true,
				mine: line('1 cup potato starch', 0),
				theirs: line('¾ cup potato starch', 0),
			},
			{
				kind: 'ingredient',
				state: 'same',
				from_branch_point: true,
				mine: line('¼ cup honey', 1),
				theirs: line('¼ cup honey', 1),
			},
			{
				kind: 'ingredient',
				state: 'only-theirs',
				from_branch_point: true,
				mine: null,
				theirs: line('1 Tbsp rice vinegar', 2),
			},
			{
				kind: 'ingredient',
				state: 'only-mine',
				from_branch_point: false,
				mine: line('1 Tbsp sesame oil', 2),
				theirs: null,
			},
		],
		steps: [
			{
				kind: 'step',
				state: 'changed',
				from_branch_point: true,
				mine: step('Coat the chicken and rest it 30 minutes.', 0),
				theirs: step('Coat the chicken.', 0),
			},
			{
				kind: 'step',
				state: 'same',
				from_branch_point: true,
				mine: step('Deep fry at 175 C until golden.', 1),
				theirs: step('Deep fry at 175 C until golden.', 1),
			},
		],
		fields: {
			title: field('Korean Fried Chicken'),
			yield: field({ amount: '4', noun: 'servings' }),
			prep_time_minutes: field(20),
			cook_time_minutes: field(30),
			source: field(null),
			note: field(null),
			nutrition: field(null),
			main_photo: field(null),
		},
		...over,
	};
}

/** A cooking of this dish, pinned to one Version. */
const attempt = (version_id: string) => ({
	id: 'at_1',
	lineage_id: 'l_1',
	person_id: 'p_stephane',
	version_id,
	current_step_index: 0,
	ticked_ingredients: [],
	cooking_yield: null,
	note: null,
	rating: null,
	finished_at: null,
	resumable: true,
	created_at: '2026-10-10T00:00:00Z',
	last_action_at: '2026-10-10T00:00:00Z',
	as_cooked: null,
	photographs: [],
});

/** What a save answers. */
const SAVED = {
	branch_id: 'b_1',
	version_id: 'v_next',
	parent_version_id: 'v_now',
	sequence: 3,
	copied: false,
	collapsed: false,
	language: 'en',
	language_offer: null,
	translates_version_id: null,
};

function renderOlder(answers: Answers = {}) {
	const went: string[] = [];
	const kamosu = standIn({ changed_since: compared(), ...answers });
	render(OlderVersionTestHarness, {
		props: {
			client: kamosu.client,
			branchId: 'b_1',
			versionId: 'v_then',
			navigate: async (to: string) => void went.push(to),
		},
	});
	return { kamosu, went };
}

/** The row a line sits in, to read its caption and its panel. */
const rowOf = (text: string) => screen.getByText(text).closest('li')!;

describe('an older Version, opened as a recipe page (#211)', () => {
	it('is the Version whole, and says plainly that it is an older one', async () => {
		const { kamosu } = renderOlder();

		expect(
			await screen.findByRole('heading', { level: 1, name: 'Korean Fried Chicken' }),
		).toBeInTheDocument();
		expect(kamosu.calls.find((call) => call.operation === 'changed_since')?.input).toEqual({
			branch_id: 'b_1',
			version_id: 'v_then',
		});

		// The bar that follows the reader down the page.
		expect(screen.getByText('Older Version')).toBeInTheDocument();
		expect(screen.getByRole('link', { name: 'The recipe now' })).toHaveAttribute(
			'href',
			'/recipes/b_1',
		);
		expect(screen.getByText('Saved 14 Aug 2026 · Stéphane · “Less sweet”')).toBeInTheDocument();

		// Its own words, not the recipe's.
		expect(screen.getByText('¾ cup potato starch')).toBeInTheDocument();
		expect(screen.getByText('¼ cup honey')).toBeInTheDocument();
		expect(screen.getByText('Coat the chicken.')).toBeInTheDocument();
		expect(screen.getByText('4 lines are not as the recipe has them now')).toBeInTheDocument();
	});

	it('marks a changed line in place, and unfolds what the recipe says now', async () => {
		renderOlder();
		const starch = within(
			await screen.findByText('¾ cup potato starch').then(() => rowOf('¾ cup potato starch')),
		);
		expect(starch.getByText('different now')).toBeInTheDocument();
		expect(starch.queryByText('1 cup potato starch')).toBeNull();

		await fireEvent.click(starch.getByText('¾ cup potato starch'));
		expect(starch.getByText('The recipe now')).toBeInTheDocument();
		expect(starch.getByText('1 cup potato starch')).toBeInTheDocument();
		expect(
			starch.getByRole('button', { name: 'Write this line back into the recipe' }),
		).toBeInTheDocument();
	});

	it('shows a line added since as a Ghost: struck through, where the recipe has it', async () => {
		renderOlder();
		await screen.findByText('¾ cup potato starch');
		const oil = within(rowOf('1 Tbsp sesame oil'));
		expect(oil.getByText('added since, not in this Version')).toBeInTheDocument();
		expect(rowOf('1 Tbsp sesame oil')).toHaveClass('mark-ghost');

		await fireEvent.click(oil.getByText('1 Tbsp sesame oil'));
		expect(
			oil.getByText('This line was written after this Version was saved.'),
		).toBeInTheDocument();
		expect(
			oil.getByRole('button', { name: 'Take it out of the recipe again' }),
		).toBeInTheDocument();
	});

	it('marks a line taken out since, which this Version still has', async () => {
		renderOlder();
		await screen.findByText('¾ cup potato starch');
		const vinegar = within(rowOf('1 Tbsp rice vinegar'));
		expect(vinegar.getByText('taken out since')).toBeInTheDocument();
		expect(rowOf('1 Tbsp rice vinegar')).toHaveClass('mark-diff');

		await fireEvent.click(vinegar.getByText('1 Tbsp rice vinegar'));
		expect(
			vinegar.getByText('This line was taken out after this Version was saved.'),
		).toBeInTheDocument();
		expect(
			vinegar.getByRole('button', { name: 'Put this line back into the recipe' }),
		).toBeInTheDocument();
	});

	it('marks a single value the recipe now holds differently', async () => {
		const answer = compared();
		answer.theirs.content.yield = { amount: '2', noun: 'servings' };
		answer.fields.yield = {
			same: false,
			mine: { amount: '4', noun: 'servings' },
			theirs: { amount: '2', noun: 'servings' },
		};
		renderOlder({ changed_since: answer });
		expect(await screen.findByText('Yield now: 4 servings')).toBeInTheDocument();
	});

	it('puts the marks away and brings them back', async () => {
		renderOlder();
		await fireEvent.click(await screen.findByRole('button', { name: 'Hide them' }));
		expect(screen.queryByText('different now')).toBeNull();
		expect(screen.queryByText('1 Tbsp sesame oil')).toBeNull();
		expect(screen.getByText('1 Tbsp rice vinegar')).toBeInTheDocument();
		await fireEvent.click(screen.getByRole('button', { name: 'Show them' }));
		expect(screen.getAllByText('different now')).toHaveLength(2);
	});

	it('leaves to the recipe as it stands everything that is the recipe’s', async () => {
		renderOlder();
		await screen.findByText('¾ cup potato starch');
		for (const gone of ['Tags', 'Related recipes', 'Cooked'])
			expect(screen.queryByRole('heading', { name: gone })).toBeNull();
		for (const gone of [
			'Edit this recipe',
			'Language',
			'Share this recipe',
			'Print a sheet',
			'Add to shopping list',
			'Delete this recipe',
			'Change',
		])
			expect(screen.queryByRole('button', { name: gone })).toBeNull();
		expect(screen.queryByRole('link', { name: 'Share this recipe' })).toBeNull();

		// The foot holds the way back, and the Thread.
		expect(screen.getByRole('link', { name: 'The recipe as it stands' })).toHaveAttribute(
			'href',
			'/recipes/b_1',
		);
		expect(screen.getByRole('link', { name: 'History' })).toHaveAttribute(
			'href',
			'/recipes/b_1/thread',
		);
	});

	it('cooks from this Version: the Attempt is pinned to it', async () => {
		const { kamosu, went } = renderOlder({ start_attempt: attempt('v_then') });
		await fireEvent.click(await screen.findByRole('button', { name: 'Cook this older Version' }));
		await expect.poll(() => went).toEqual(['/cook/b_1']);
		expect(kamosu.calls.find((call) => call.operation === 'start_attempt')?.input).toEqual({
			branch_id: 'b_1',
			version_id: 'v_then',
		});
	});

	it('says so and stays where a cooking from another Version is already under way', async () => {
		const { went } = renderOlder({ start_attempt: attempt('v_now') });
		await fireEvent.click(await screen.findByRole('button', { name: 'Cook this older Version' }));
		expect(await screen.findByRole('alert')).toHaveTextContent(/already under way/);
		expect(screen.getByRole('link', { name: 'Open that cooking' })).toHaveAttribute(
			'href',
			'/cook/b_1',
		);
		expect(went).toEqual([]);
	});

	it('puts a Step back with the Photograph it had', async () => {
		const answer = compared();
		answer.theirs.content.steps = [
			...STEPS_THEN,
			{ kind: 'step', text: 'Rest it.', photo: 'p_rest' },
		];
		answer.theirs.measured.steps = [null, null, null];
		answer.steps.push({
			kind: 'step',
			state: 'only-theirs',
			from_branch_point: true,
			mine: null,
			theirs: step('Rest it.', 2),
		});
		const { kamosu } = renderOlder({ changed_since: answer, save_recipe_version: SAVED });
		await fireEvent.click(await screen.findByText('Rest it.'));
		await fireEvent.click(
			screen.getByRole('button', { name: 'Put this line back into the recipe' }),
		);
		await fireEvent.click(screen.getByRole('button', { name: 'Save a Version' }));
		await fireEvent.click(
			within(await screen.findByRole('dialog')).getByRole('button', { name: 'Save a Version' }),
		);
		await screen.findByText('Saved. The recipe has those lines now.');
		expect(
			kamosu.calls.find((call) => call.operation === 'save_recipe_version')?.input,
		).toMatchObject({
			steps: [STEPS_NOW[0], STEPS_NOW[1], { kind: 'step', text: 'Rest it.', photo: 'p_rest' }],
		});
	});

	it('writes a line back into the recipe as an ordinary Version, once it is saved', async () => {
		const { kamosu } = renderOlder({ save_recipe_version: SAVED });
		await screen.findByText('¾ cup potato starch');

		// The starch as it was, the vinegar back, the sesame oil out, and the
		// first Step as it was.
		await fireEvent.click(screen.getByText('¾ cup potato starch'));
		await fireEvent.click(
			screen.getByRole('button', { name: 'Write this line back into the recipe' }),
		);
		await fireEvent.click(screen.getByText('1 Tbsp rice vinegar'));
		await fireEvent.click(
			screen.getByRole('button', { name: 'Put this line back into the recipe' }),
		);
		await fireEvent.click(screen.getByText('1 Tbsp sesame oil'));
		await fireEvent.click(screen.getByRole('button', { name: 'Take it out of the recipe again' }));
		await fireEvent.click(screen.getByText('Coat the chicken.'));
		await fireEvent.click(
			within(rowOf('Coat the chicken.')).getByRole('button', {
				name: 'Write this line back into the recipe',
			}),
		);

		// Nothing is saved yet, and each row says what a save will do.
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('save_recipe_version');
		expect(
			within(rowOf('1 Tbsp sesame oil')).getByText('leaving the recipe when you save'),
		).toBeInTheDocument();
		expect(
			screen.getByText('4 lines from this Version are set to go into the recipe. Not saved.'),
		).toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: 'Save a Version' }));
		const sheet = within(await screen.findByRole('dialog'));
		expect(sheet.getByRole('textbox')).toHaveValue(
			'Put back ¾ cup potato starch, 1 Tbsp rice vinegar and the step that starts “Coat the chicken.” from the Version of 14 Aug 2026. Took out 1 Tbsp sesame oil, as in the Version of 14 Aug 2026.',
		);
		await fireEvent.click(sheet.getByRole('button', { name: 'Save a Version' }));

		expect(await screen.findByText('Saved. The recipe has those lines now.')).toBeInTheDocument();
		const saved = kamosu.calls.find((call) => call.operation === 'save_recipe_version')?.input;
		expect(saved).toMatchObject({
			branch_id: 'b_1',
			ingredients: [
				{ kind: 'ingredient', text: '¾ cup potato starch' },
				{ kind: 'ingredient', text: '¼ cup honey' },
				{ kind: 'ingredient', text: '1 Tbsp rice vinegar' },
			],
			// The Step is the recipe's own, reworded: it keeps its Photograph.
			steps: [
				{ kind: 'step', text: 'Coat the chicken.', photo: 'p_coat' },
				{ kind: 'step', text: 'Deep fry at 175 C until golden.', photo: null },
			],
		});
		// The recipe has moved, so the two are read again.
		expect(kamosu.calls.filter((call) => call.operation === 'changed_since')).toHaveLength(2);
	});

	it('offers nothing back on a recipe the reader does not write', async () => {
		renderOlder({ changed_since: compared({ writes: false }) });
		await fireEvent.click(await screen.findByText('¾ cup potato starch'));
		expect(screen.getByText('1 cup potato starch')).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: /into the recipe/ })).toBeNull();
	});

	it('sends the newest Version’s address to the recipe, which is what it is', async () => {
		const answer = compared();
		answer.version.newest = true;
		const { went } = renderOlder({ changed_since: answer });
		await expect.poll(() => went).toEqual(['/recipes/b_1']);
		expect(screen.queryByText('Older Version')).toBeNull();
	});

	it('says so when the Version cannot be read', async () => {
		renderOlder({ changed_since: { refuse: 'not_found' } });
		expect(await screen.findByRole('alert')).toHaveTextContent('This Version could not be read.');
	});
});
