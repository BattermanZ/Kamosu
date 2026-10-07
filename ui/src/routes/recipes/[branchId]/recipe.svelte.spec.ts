/**
 * The screen-seam test: the recipe, and the Divergence, against a stand-in
 * Kamosu. Every answer below is checked against the Catalogue before the screen
 * sees it — a field renamed in `src/catalogue.rs` fails this test in the same
 * commit.
 *
 * What is being tested is ADR 0014's shape: whole recipes with a switch
 * between them, never a difference. Since #131 the switch is a strip of every
 * version, and the marks on any version compare it against your own. If any
 * assertion here starts describing a comparison view, the screen has gone
 * wrong.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen, fireEvent, within } from '@testing-library/svelte';
import { tick, type ComponentProps } from 'svelte';
import userEvent from '@testing-library/user-event';
import { standIn, type Answers } from '$lib/api/stand-in';
import { realFiles } from '$lib/api/files';
import { OperationError } from '$lib/api/client';
import type { GetRecipeOutput } from '$lib/api/catalogue';
import RecipeTestHarness from './RecipeTestHarness.svelte';
import { went } from '../../../testing/navigation';
import { outlivingTheWait, withTheClockFaked } from '../../../testing/jobs';
import { cookbookLabel, threadBranch, writtenByMe } from '../../../testing/recipes';
import { holdPaste, takePaste } from '$lib/pasted.svelte';

// Deleting ends by going back to the shelf, because there is nothing left to
// stand on. Where that goes is the router's business, not this screen's, so
// `setup.ts` stubs it and `went` says where the screen asked to go.

/** One slot of a Version's `readings`: what Kamosu understood, or nothing. */
type Slot = GetRecipeOutput['versions'][number]['readings'][number];

/** A recipe's Nutrition figure, or nothing — the Catalogue's own shape (#72). */
type Nutrition = GetRecipeOutput['versions'][number]['content']['nutrition'];

const line = (text: string, index: number) => ({
	kind: 'ingredient',
	text,
	index,
});
const step = (text: string, index: number) => ({ kind: 'step', text, index });

/** Stéphane's own Cookbook, under the name he gave it. */
const MY_COOKBOOK = cookbookLabel('c_mine', ['Stéphane'], 'Maison Batterman');
/** Marc's, which Stéphane sees because they share a Kitchen and never writes. */
const MARCS_COOKBOOK = cookbookLabel('c_marc', ['Marc'], 'Chez Marc');

const branch = (
	branch_id: string,
	cookbook: typeof MY_COOKBOOK,
	ingredients: { kind: 'section' | 'ingredient'; text: string }[],
	steps: { kind: 'section' | 'step'; text: string; photo: string | null }[],
) => ({
	branch_id,
	cookbook,
	name: null,
	mine: cookbook.id === MY_COOKBOOK.id,
	arrived: false,
	hand_id: `h_${branch_id}`,
	hand_name: null,
	language: 'en',
	head_version_id: `v_${branch_id}`,
	content: {
		title: 'Korean Fried Chicken',
		yield: { amount: '4', noun: 'servings' },
		prep_time_minutes: 20,
		cook_time_minutes: 30,
		note: null,
		main_photo: null,
		nutrition: null as Nutrition,
		source: { text: 'mykoreankitchen.com', link: null },
		ingredients,
		steps,
	},
	readings: ingredients.map(() => null),
	// Nothing to say beneath either side's lines: this reader measures the way
	// the recipe is already written (#49).
	measured: { ingredients: ingredients.map(() => null), steps: steps.map(() => null) },
	// Neither side composes anything. Both sides carry their own (#50): ADR
	// 0014 holds the two recipes as equals, so a dough that unfolded in one and
	// not the other would make one of them the lesser.
	components: [],
});

const MY_INGREDIENTS = [
	{ kind: 'ingredient' as const, text: '1 cup potato starch (or corn starch)' },
	{ kind: 'ingredient' as const, text: '¼ cup honey' },
	{ kind: 'ingredient' as const, text: '¼ cup brown sugar' },
	{ kind: 'ingredient' as const, text: '1 Tbsp rice vinegar' },
];
const THEIR_INGREDIENTS = [
	{ kind: 'ingredient' as const, text: '¾ cup potato starch' },
	{ kind: 'ingredient' as const, text: '¼ cup honey' },
	{ kind: 'ingredient' as const, text: '1 tsp gochugaru' },
];
const MY_STEPS = [
	{
		kind: 'step' as const,
		text: 'Deep fry at 190 C until crisp.',
		photo: null,
	},
];
const THEIR_STEPS = [
	{
		kind: 'step' as const,
		text: 'Air fry at 200 C for 18 minutes.',
		photo: null,
	},
	{
		kind: 'step' as const,
		text: 'Let it sit 5 minutes before saucing.',
		photo: null,
	},
];

/**
 * One Lineage, two Branches, one Branch Point — carrying every case at once:
 * a quantity altered and paired, a line the other side removed, two lines that
 * both arrived after the parting and are left unpaired, and a Step rewritten
 * past recognition.
 */
function divergence() {
	return {
		lineage_id: 'l_1',
		branch_point_version_id: 'v_base',
		mine: branch('mine', MY_COOKBOOK, MY_INGREDIENTS, MY_STEPS),
		theirs: branch('theirs', MARCS_COOKBOOK, THEIR_INGREDIENTS, THEIR_STEPS),
		ingredients: [
			{
				kind: 'ingredient',
				state: 'changed' as const,
				from_branch_point: true,
				mine: line('1 cup potato starch (or corn starch)', 0),
				theirs: line('¾ cup potato starch', 0),
			},
			{
				kind: 'ingredient',
				state: 'same' as const,
				from_branch_point: true,
				mine: line('¼ cup honey', 1),
				theirs: line('¼ cup honey', 1),
			},
			{
				// Marc took this out. It was at the Branch Point, so it can be
				// carried across as a removal, not only read.
				kind: 'ingredient',
				state: 'only-mine' as const,
				from_branch_point: true,
				mine: line('¼ cup brown sugar', 2),
				theirs: null,
			},
			{
				// Mine, written after we parted. Nothing of his resembles it.
				kind: 'ingredient',
				state: 'only-mine' as const,
				from_branch_point: false,
				mine: line('1 Tbsp rice vinegar', 3),
				theirs: null,
			},
			{
				// His, written after we parted, landing in the same place. The
				// Pairing declined to join these two, and neither is badged.
				kind: 'ingredient',
				state: 'only-theirs' as const,
				from_branch_point: false,
				mine: null,
				theirs: line('1 tsp gochugaru', 2),
			},
		],
		steps: [
			{
				kind: 'step',
				state: 'only-mine' as const,
				from_branch_point: true,
				mine: step('Deep fry at 190 C until crisp.', 0),
				theirs: null,
			},
			{
				kind: 'step',
				state: 'only-theirs' as const,
				from_branch_point: true,
				mine: null,
				theirs: step('Air fry at 200 C for 18 minutes.', 0),
			},
			{
				kind: 'step',
				state: 'only-theirs' as const,
				from_branch_point: false,
				mine: null,
				theirs: step('Let it sit 5 minutes before saucing.', 1),
			},
		],
		fields: {
			title: {
				same: true,
				mine: 'Korean Fried Chicken',
				theirs: 'Korean Fried Chicken',
			},
			yield: { same: true, mine: null, theirs: null },
			prep_time_minutes: { same: true, mine: 20, theirs: 20 },
			cook_time_minutes: { same: true, mine: 30, theirs: 30 },
			source: { same: true, mine: null, theirs: null },
			note: { same: true, mine: null, theirs: null },
			// Typed, unlike its neighbours: #84 marks this one, so a test has to
			// be able to put a figure on either side of it.
			nutrition: { same: true, mine: null as Nutrition, theirs: null as Nutrition },
			main_photo: { same: true, mine: null, theirs: null },
		},
	};
}

/** One occurrence of one Version on one Branch, as the Thread lists it. */
const occurrenceOf = (
	branch_id: string,
	sequence: number,
	version_id: string,
	parent_version_id: string | null,
) => ({
	branch_id,
	sequence,
	version_id,
	parent_version_id,
	hand_id: `h_${branch_id}`,
	hand_name: `Chez ${branch_id}`,
	name: null,
	change_note: null,
	created_at: '2026-08-09T00:00:00Z',
	translates_version_id: null,
	language: 'en',
});

/** Your own version, as the Thread names it. */
const MINE_IN_THREAD = threadBranch('mine', {
	cookbook: MY_COOKBOOK,
	hand_id: 'h_mine',
	head_version_id: 'v_mine',
});
/** Marc's version of the same recipe, in his own Cookbook. */
const THEIRS_IN_THREAD = threadBranch('theirs', {
	cookbook: MARCS_COOKBOOK,
	mine: false,
	hand_id: 'h_theirs',
	head_version_id: 'v_theirs',
});

function forked(extra: Answers = {}) {
	return {
		get_recipe: {
			branch_id: 'mine',
			lineage_id: 'l_1',
			cookbook: MY_COOKBOOK,
			name: null,
			writes: true,
			mine: true,
			arrived: false,
			hand_id: 'h_mine',
			language: 'en',
			origin_address: null,
			head_version_id: 'v_mine',
			translation: null,
			versions: [
				{
					sequence: 1,
					version_id: 'v_mine',
					parent_version_id: null,
					hand_id: 'h_mine',
					name: null,
					change_note: null,
					created_at: '2026-08-09T00:00:00Z',
					translates_version_id: null,
					scaled_to: null,
					language: 'en',
					// A recipe that composes nothing, which is nearly all of them (#50).
					components: [],
					content: divergence().mine.content,
					readings: MY_INGREDIENTS.map(() => null),
					measured: {
						ingredients: MY_INGREDIENTS.map(() => null),
						steps: MY_STEPS.map(() => null),
					},
					cooking: { steps: MY_STEPS.map(() => ({ uses: [], timer_seconds: null })) },
				},
			],
			tags: [],
			related_recipes: [],
			cooked: { count: 0, last_cooked_at: null, ratings: [] },
		},
		get_thread: {
			lineage_id: 'l_1',
			branches: [MINE_IN_THREAD, THEIRS_IN_THREAD],
			// **A real Divergence shares a Version.** Two Branches that parted
			// have a root in common and their own heads after it, and the page
			// reads exactly that to decide whether a pair can be laid over each
			// other at all — a Translation's chain starts fresh and shares
			// nothing, which is why asking the Core to pair one was refused
			// (#106, ADR 0006). An empty list here would describe a Lineage
			// whose Branches came from nowhere.
			versions: [
				occurrenceOf('mine', 1, 'v_root', null),
				occurrenceOf('mine', 2, 'v_mine', 'v_root'),
				occurrenceOf('theirs', 1, 'v_root', null),
				occurrenceOf('theirs', 2, 'v_theirs', 'v_root'),
			],
			attempts: [],
		},
		divergence: divergence(),
		...extra,
	} as Answers;
}

/**
 * Marc's version as `get_recipe` answers it, opened from the strip: his words,
 * in his Cookbook, which Stéphane may read and cook and not change.
 */
function marcsRecipe(): GetRecipeOutput {
	const mine = forked().get_recipe as GetRecipeOutput;
	const his = divergence().theirs;
	return {
		...mine,
		branch_id: 'theirs',
		cookbook: MARCS_COOKBOOK,
		writes: false,
		mine: false,
		arrived: false,
		hand_id: 'h_theirs',
		head_version_id: 'v_theirs',
		versions: [
			{
				...mine.versions[0]!,
				version_id: 'v_theirs',
				hand_id: 'h_theirs',
				content: his.content,
				readings: THEIR_INGREDIENTS.map(() => null),
				measured: {
					ingredients: THEIR_INGREDIENTS.map(() => null),
					steps: THEIR_STEPS.map(() => null),
				},
				cooking: { steps: THEIR_STEPS.map(() => ({ uses: [], timer_seconds: null })) },
			},
		],
	};
}

/**
 * Wait for the comparison with yours, which lands after his recipe does: his
 * words are on the page a moment before any mark is.
 */
const compared = () => screen.findByText(/you and Chez Marc don’t share/);

/** The page open on Marc's version: every mark compares it against yours. */
function onTheirs(extra: Answers = {}) {
	return forked({ get_recipe: marcsRecipe(), ...extra });
}

function renderRecipe(
	answers: Answers = forked(),
	branchId = 'mine',
	/** The phone it is on, and how it fetches a Sheet to share (#149). */
	on: Pick<ComponentProps<typeof RecipeTestHarness>, 'device' | 'files'> = {},
) {
	// Your own cookings, read for their pictures (#110). Nobody here has
	// cooked anything unless a test says so.
	const kamosu = standIn({ list_attempts: { attempts: [] }, ...answers });
	const { rerender } = render(RecipeTestHarness, {
		props: { client: kamosu.client, branchId, ...on },
	});
	/** Walk to another recipe, the way tapping through to one does. */
	const goTo = (branchId: string) => rerender({ client: kamosu.client, branchId, ...on });
	return { kamosu, goTo };
}

/** A save that landed on your own version, as `save_recipe_version` answers it. */
const SAVED_ONTO_MINE = {
	save_recipe_version: {
		branch_id: 'mine',
		version_id: 'v_new',
		parent_version_id: 'v_mine',
		sequence: 2,
		copied: false,
		collapsed: false,
		language: 'en',
		language_offer: null,
		translates_version_id: null,
	},
} as Answers;

describe('a Divergence', () => {
	it('wears the other colour on his version and your own on your variation (#131)', async () => {
		renderRecipe(onTheirs(), 'theirs');
		await compared();
		expect(document.querySelector('[data-whose]')).toHaveAttribute('data-whose', 'theirs');
		cleanup();

		// Your own variation, read against your main one: still yours.
		const yoursVaried = {
			...marcsRecipe(),
			cookbook: cookbookLabel(),
			writes: true,
			name: 'Spicy',
		};
		renderRecipe(onTheirs({ get_recipe: yoursVaried }), 'theirs');
		await compared();
		expect(document.querySelector('[data-whose]')).toHaveAttribute('data-whose', 'mine');
	});

	it('says every line is the same, and offers nothing to hide, on a version that matches yours', async () => {
		const same = divergence();
		same.ingredients = same.ingredients.filter((row) => row.state === 'same');
		same.steps = [];
		renderRecipe(onTheirs({ divergence: same }), 'theirs');
		expect(await screen.findByText('Every line is the same as in yours.')).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: /Hide them/ })).toBeNull();
	});

	it('is a strip of whole recipes, each its own page, never a difference view', async () => {
		renderRecipe();

		// Your own, whole: nothing marked, and the strip says how to see the others.
		expect(await screen.findByText('This recipe, 2 versions')).toBeInTheDocument();
		expect(screen.getByText('1 cup potato starch (or corn starch)')).toBeInTheDocument();
		expect(screen.getByText('¼ cup honey')).toBeInTheDocument();
		expect(screen.getByText(/Tap another version to read it/)).toBeInTheDocument();
		expect(screen.queryByText('1 tsp gochugaru')).not.toBeInTheDocument();

		// Each version is a link to its own page, so the page is always the
		// recipe it says it is. Yours is the one you are on.
		const yours = screen.getByRole('link', { name: /Yours/ });
		expect(yours).toHaveAttribute('href', '/recipes/mine');
		expect(yours).toHaveAttribute('aria-current', 'page');
		const his = screen.getByRole('link', { name: /Chez Marc/ });
		expect(his).toHaveAttribute('href', '/recipes/theirs');
		expect(his).not.toHaveAttribute('aria-current');
	});

	it('reads Marc’s version whole and cookable, marked against yours', async () => {
		const { kamosu } = renderRecipe(onTheirs(), 'theirs');
		await compared();

		expect(await screen.findByText('¾ cup potato starch')).toBeInTheDocument();
		expect(screen.getByText('Air fry at 200 C for 18 minutes.')).toBeInTheDocument();
		expect(screen.getByRole('link', { name: /Chez Marc/ })).toHaveAttribute('aria-current', 'page');
		// Compared with yours, whichever version is open (#131, screen choice 1).
		expect(kamosu.calls.find((call) => call.operation === 'divergence')?.input).toEqual({
			branch_id: 'mine',
			other_branch_id: 'theirs',
		});
		expect(screen.getByText(/lines you and Chez Marc don’t share/)).toBeInTheDocument();
	});

	it('asks for no comparison on your own version, where there is nothing to mark', async () => {
		const { kamosu } = renderRecipe();
		await screen.findByText('This recipe, 2 versions');
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('divergence');
	});

	it('marks your variation against your own once it has a name (#136)', async () => {
		// Your own named "Classic", and your variation "Spicy" beside it.
		const named = {
			...(forked().get_thread as Record<string, unknown>),
			branches: [
				{ ...MINE_IN_THREAD, name: 'Classic' },
				{ ...THEIRS_IN_THREAD, cookbook: MY_COOKBOOK, mine: true, name: 'Spicy' },
			],
		} as Answers['get_thread'];
		const { kamosu } = renderRecipe(onTheirs({ get_thread: named }), 'theirs');
		await compared();
		expect(kamosu.calls.find((call) => call.operation === 'divergence')?.input).toEqual({
			branch_id: 'mine',
			other_branch_id: 'theirs',
		});
		// The strip says which one the others are measured against.
		expect(screen.getByRole('link', { name: /Classic/ })).toHaveTextContent('your Cookbook');
		expect(screen.getByRole('link', { name: /Spicy/ })).toHaveTextContent('yours too');
		cleanup();

		// On Classic itself: nothing to mark, and the strip says how to see the others.
		const onClassic = renderRecipe(forked({ get_thread: named }));
		expect(await screen.findByText(/Tap another version to read it/)).toBeInTheDocument();
		expect(onClassic.kamosu.calls.map((call) => call.operation)).not.toContain('divergence');
		expect(screen.getByRole('link', { name: /Classic/ })).toHaveTextContent('your Cookbook');
	});

	it('marks nothing where you have no version of your own to compare with', async () => {
		// Two of other people's versions and none of yours: there is no "how
		// is theirs different from mine" to answer.
		const { kamosu } = renderRecipe(
			onTheirs({
				get_thread: {
					...(forked().get_thread as Record<string, unknown>),
					branches: [
						{ ...MINE_IN_THREAD, cookbook: cookbookLabel('c_luc', ['Luc']), mine: false },
						THEIRS_IN_THREAD,
					],
				} as Answers['get_thread'],
			}),
			'theirs',
		);
		expect(await screen.findByText('This recipe, 2 versions')).toBeInTheDocument();
		expect(screen.getByText('¾ cup potato starch')).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('divergence');
		expect(screen.queryByText(/yours — Chez Marc/)).not.toBeInTheDocument();
	});

	it('keeps a photographed Step’s picture beside it where the Step is marked (#110)', async () => {
		// Marc's first Step is marked, since yours has not got it, so it is
		// drawn by the marked row and not the plain list, and still wears its picture.
		const fried = divergence();
		fried.theirs.content.steps = [{ ...THEIR_STEPS[0], photo: 'p_air' }, THEIR_STEPS[1]];
		renderRecipe(onTheirs({ divergence: fried }), 'theirs');
		await compared();
		const open = await screen.findByRole('button', { name: 'Show the photograph of step 1' });
		expect(open.querySelector('img')).toHaveAttribute('src', '/api/photographs/p_air/card');
		// Your deep-frying Step is a Ghost on his page, and a Ghost is not a
		// Step of the recipe being read: one picture, not two.
		expect(screen.getAllByRole('button', { name: /Show the photograph of step/ })).toHaveLength(1);
	});

	it('shows the Tags only where you write the recipe, never your filing on his', async () => {
		// Tags belong to the Cookbook that writes the recipe (#131, question
		// 3). Marc's version is read with his Cookbook's tags, which here are
		// none, and nobody who does not write it is offered a way to add one.
		const spicy = {
			id: 't_spicy',
			cookbook_id: MY_COOKBOOK.id,
			name: 'spicy',
			language: 'en',
			names: [{ language: 'en', name: 'spicy' }],
			recipes: 14,
			// The Core decides this, against the Reading Language (#104).
			language_fallback: false,
		};
		renderRecipe(
			forked({
				list_tags: { tags: [spicy] },
				get_recipe: { ...(forked().get_recipe as GetRecipeOutput), tags: [spicy] },
			}),
		);
		expect(await screen.findByRole('link', { name: /spicy/ })).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Add a tag' })).toBeInTheDocument();
		cleanup();

		renderRecipe(onTheirs(), 'theirs');
		await compared();
		await screen.findByText('¾ cup potato starch');
		expect(screen.queryByRole('link', { name: /spicy/ })).not.toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Add a tag' })).not.toBeInTheDocument();
	});

	it('shows a line of yours his has not got as a Ghost, named, in its own position', async () => {
		renderRecipe(onTheirs(), 'theirs');
		await compared();

		// Your rice vinegar is a Ghost on his recipe, and it says whose it is.
		// A struck-through line with no words beside it would read like
		// something crossed off a shopping list.
		expect(await screen.findByText('1 Tbsp rice vinegar')).toBeInTheDocument();
		expect(screen.getAllByText(/yours, not in Chez Marc’s/i).length).toBeGreaterThan(0);

		// It sits where it sits in yours: after the honey, not at the end.
		const texts = Array.from(document.querySelectorAll('ul li')).map((li) => li.textContent ?? '');
		const honey = texts.findIndex((t) => t.includes('¼ cup honey'));
		const vinegar = texts.findIndex((t) => t.includes('1 Tbsp rice vinegar'));
		expect(vinegar).toBeGreaterThan(honey);
	});

	it('shows an uncertain Pairing as both lines, unjoined, with nothing labelled a guess', async () => {
		renderRecipe(onTheirs(), 'theirs');
		await compared();

		// Your rice vinegar and his gochugaru both arrived after you parted and
		// land in the same part of the sauce. Both are on the page, separately.
		expect(await screen.findByText('1 tsp gochugaru')).toBeInTheDocument();
		expect(screen.getByText('1 Tbsp rice vinegar')).toBeInTheDocument();

		// No hedge of any kind, anywhere on the screen.
		const page = document.body.textContent ?? '';
		for (const hedge of ['probably', 'likely', 'uncertain', 'maybe', 'confidence', '%']) {
			expect(page.toLowerCase()).not.toContain(hedge);
		}
	});

	it('puts the marking away and leaves simply his recipe', async () => {
		renderRecipe(onTheirs(), 'theirs');
		await compared();
		await screen.findByText('1 Tbsp rice vinegar');

		await fireEvent.click(screen.getByRole('button', { name: /Hide them/i }));

		// Every Ghost goes: a Ghost is a line of YOUR recipe, and with the
		// marking away it has no business on his page at all.
		expect(screen.queryByText('1 Tbsp rice vinegar')).not.toBeInTheDocument();
		expect(screen.queryByText(/Chez Marc hasn’t got it/i)).not.toBeInTheDocument();
		// What is left is the list you would shop from, his.
		expect(screen.getByText('¾ cup potato starch')).toBeInTheDocument();
		expect(screen.getByText('1 tsp gochugaru')).toBeInTheDocument();
		expect(screen.getByText(/differences put away/)).toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: /Show them/i }));
		expect(await screen.findByText('1 Tbsp rice vinegar')).toBeInTheDocument();
	});

	it('carries a line of his into yours unsaved, and saving writes an ordinary Version with prose', async () => {
		const { kamosu } = renderRecipe(onTheirs(SAVED_ONTO_MINE), 'theirs');
		await compared();

		// The offer lives where you would want a line of his recipe: in it.
		await fireEvent.click(await screen.findByText('¾ cup potato starch'));
		await fireEvent.click(screen.getByRole('button', { name: /Write this into mine/i }));

		// It is on your recipe and it is not saved. Nothing has been sent yet.
		expect(screen.getByText(/sitting on your recipe. Not saved/i)).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('save_recipe_version');

		// It becomes real only when an ordinary Version is saved, and *what
		// changed* is pre-filled as a sentence rather than a pointer at his Branch.
		await fireEvent.click(screen.getByRole('button', { name: /Save a Version/i }));
		const note = screen.getByLabelText(/What changed/i) as HTMLTextAreaElement;
		expect(note.value).toBe('Took ¾ cup potato starch from Chez Marc.');
		expect(note.value).not.toMatch(/branch|version|v_/i);

		await fireEvent.click(
			screen.getAllByRole('button', { name: /^Save a Version$/i }).at(-1) as HTMLElement,
		);

		const saved = kamosu.calls.find((call) => call.operation === 'save_recipe_version');
		const input = saved?.input as {
			branch_id: string;
			ingredients: { text: string }[];
			change_note: string;
		};
		// Onto YOUR version, even though his is the page you were reading.
		expect(input.branch_id).toBe('mine');
		// A whole recipe, with his line written in where the Pairing puts it.
		expect(input.ingredients.map((item) => item.text)).toEqual([
			'¾ cup potato starch',
			'¼ cup honey',
			'¼ cup brown sugar',
			'1 Tbsp rice vinegar',
		]);
		expect(input.change_note).toBe('Took ¾ cup potato starch from Chez Marc.');
	});

	it('takes a line out of yours as he did, from his page', async () => {
		const { kamosu } = renderRecipe(onTheirs(SAVED_ONTO_MINE), 'theirs');
		await compared();

		// Your brown sugar, which he took out: the Ghost offers the removal.
		await fireEvent.click(await screen.findByText('¼ cup brown sugar'));
		await fireEvent.click(screen.getByRole('button', { name: /Take it out of mine as well/i }));
		await fireEvent.click(screen.getByRole('button', { name: /Save a Version/i }));
		await fireEvent.click(
			screen.getAllByRole('button', { name: /^Save a Version$/i }).at(-1) as HTMLElement,
		);

		const saved = kamosu.calls.find((call) => call.operation === 'save_recipe_version');
		const input = saved?.input as { branch_id: string; ingredients: { text: string }[] };
		expect(input.branch_id).toBe('mine');
		expect(input.ingredients.map((item) => item.text)).not.toContain('¼ cup brown sugar');
	});

	it('carries the nutrition figure through a save rather than erasing it', async () => {
		// `save_recipe_version` replaces the whole recipe, so a field this
		// screen forgets to send is a field the save deletes. The nutrition
		// figure is one of the recipe's own words (#72), and taking one line
		// across from another Branch is not a reason to lose it.
		const answers = onTheirs(SAVED_ONTO_MINE);
		const d = (answers as Record<string, unknown>).divergence as ReturnType<typeof divergence>;
		d.mine.content.nutrition = { calories: 308, basis: 'per_serving' };
		const { kamosu } = renderRecipe(answers, 'theirs');
		await compared();

		await fireEvent.click(await screen.findByText('¾ cup potato starch'));
		await fireEvent.click(screen.getByRole('button', { name: /Write this into mine/i }));
		await fireEvent.click(screen.getByRole('button', { name: /Save a Version/i }));
		await fireEvent.click(
			screen.getAllByRole('button', { name: /^Save a Version$/i }).at(-1) as HTMLElement,
		);

		const saved = kamosu.calls.find((call) => call.operation === 'save_recipe_version');
		expect((saved?.input as { nutrition: unknown }).nutrition).toEqual({
			calories: 308,
			basis: 'per_serving',
		});
	});

	it('marks a single value the two versions do not agree on, with what yours says', async () => {
		// ADR 0019: "The marking covers the whole recipe, not only the two lists.
		// Title, Yield, Prep Time, Cook Time and Source are single values."
		// On his page his values are the page; the mark is what YOURS has.
		const answers = onTheirs();
		const d = (answers as Record<string, unknown>).divergence as ReturnType<typeof divergence>;
		d.fields.title = { same: false, mine: 'Korean Fried Chicken', theirs: 'KFC, air fryer' };
		d.fields.cook_time_minutes = { same: false, mine: 30, theirs: 22 };
		d.theirs.content.title = 'KFC, air fryer';
		d.theirs.content.cook_time_minutes = 22;
		renderRecipe(answers, 'theirs');
		await compared();

		expect(await screen.findByText(/Korean Fried Chicken/)).toBeInTheDocument();
		// Chez Marc has 22, not 30: a mark naming him beside your value says
		// something false about his recipe.
		expect(screen.queryByText(/Chez Marc has 30/)).not.toBeInTheDocument();
		expect(screen.getByText('Yours: Korean Fried Chicken')).toBeInTheDocument();
		// A time reads with its unit, as it does in the strip (#175).
		expect(screen.getByText('Yours: 30 min')).toBeInTheDocument();
		expect(screen.queryByText(/Chez Marc has Korean Fried Chicken/)).not.toBeInTheDocument();
	});

	/**
	 * #72 gave `divergence` a `fields.nutrition` and nothing read it, so two
	 * Branches disagreeing about the figure said nothing at all. The mark goes
	 * with the figure, at the foot of the Ingredients, rather than with the
	 * marks at the top: the figure is there, so what the other version says
	 * about it belongs there too (#84).
	 */
	it('marks the figure the two versions do not agree on, beside the figure', async () => {
		const answers = onTheirs();
		const d = (answers as Record<string, unknown>).divergence as ReturnType<typeof divergence>;
		d.mine.content.nutrition = { calories: 308, basis: 'per_serving' };
		d.theirs.content.nutrition = { calories: 420, basis: 'per_serving' };
		d.fields.nutrition = {
			same: false,
			mine: { calories: 308, basis: 'per_serving' },
			theirs: { calories: 420, basis: 'per_serving' },
		};
		renderRecipe(answers, 'theirs');
		await compared();

		const mark = await screen.findByText('Yours: 308 kcal a serving');
		// Beside the figure it is about, not up with the Title and the Yield.
		expect(
			mark.compareDocumentPosition(screen.getByText('420 kcal a serving')) &
				Node.DOCUMENT_POSITION_PRECEDING,
		).toBeTruthy();
	});

	it('says so when your version carries no figure where his does', async () => {
		const answers = onTheirs();
		const d = (answers as Record<string, unknown>).divergence as ReturnType<typeof divergence>;
		d.theirs.content.nutrition = { calories: 420, basis: 'per_serving' };
		d.fields.nutrition = {
			same: false,
			mine: null,
			theirs: { calories: 420, basis: 'per_serving' },
		};
		renderRecipe(answers, 'theirs');
		await compared();

		expect(await screen.findByText('Yours: nothing')).toBeInTheDocument();
	});

	it('names every version in the strip when there are three', async () => {
		renderRecipe(
			forked({
				get_thread: {
					lineage_id: 'l_1',
					branches: [
						MINE_IN_THREAD,
						THEIRS_IN_THREAD,
						threadBranch('camille', {
							cookbook: cookbookLabel('c_camille', ['Camille']),
							mine: false,
						}),
					],
					// All three parted from the same root, which is what makes
					// them versions of one recipe rather than unrelated recipes.
					versions: ['mine', 'theirs', 'camille'].flatMap((id) => [
						occurrenceOf(id, 1, 'v_root', null),
						occurrenceOf(id, 2, `v_${id}`, 'v_root'),
					]),
					attempts: [],
				},
			}),
		);

		// ADR 0014's switch was for two. Three or four versions of one dish is
		// ordinary under ADR 0041, so every one is a chip.
		expect(await screen.findByText('This recipe, 3 versions')).toBeInTheDocument();
		expect(screen.getByRole('link', { name: /Camille’s/ })).toHaveAttribute(
			'href',
			'/recipes/camille',
		);
		expect(screen.getByRole('link', { name: /Chez Marc/ })).toHaveAttribute(
			'href',
			'/recipes/theirs',
		);
	});

	it('names Marc on a Ghost, and never you', async () => {
		renderRecipe(onTheirs(), 'theirs');
		await compared();
		await screen.findByText('¾ cup potato starch');

		// Your brown sugar, which HE took out. Reading his recipe does not make
		// you him, so the name on the mark is still his. Naming whichever
		// version you were not on once put "Maison Batterman took it out" on
		// your own line.
		expect(screen.getAllByText(/yours, Chez Marc took it out/i).length).toBeGreaterThan(0);
		expect(screen.queryByText(/Maison Batterman took it out/i)).not.toBeInTheDocument();
		// And his own changed line reads as not yours, not as "not Chez Marc's".
		expect(screen.getAllByText(/^not yours$/i).length).toBeGreaterThan(0);
	});

	it('offers no way to take a whole version at once', async () => {
		renderRecipe(onTheirs(), 'theirs');
		await compared();
		await screen.findByText('¾ cup potato starch');

		// The absence is the decision (ADR 0014): taking every line one at a
		// time is a person making a recipe; one button doing it is a merge.
		for (const label of [/take all/i, /take everything/i, /accept all/i, /merge/i]) {
			expect(screen.queryByRole('button', { name: label })).not.toBeInTheDocument();
		}
	});

	it('reads a recipe on its own when there is no second version', async () => {
		renderRecipe(
			forked({
				get_thread: {
					lineage_id: 'l_1',
					branches: [MINE_IN_THREAD],
					versions: [],
					attempts: [],
				},
			}),
		);

		expect(await screen.findByText('1 cup potato starch (or corn starch)')).toBeInTheDocument();
		// No strip, because there is nowhere else to go.
		expect(screen.queryByText(/This recipe, \d+ versions/)).not.toBeInTheDocument();
		expect(screen.queryByText('1 tsp gochugaru')).not.toBeInTheDocument();
	});

	it('shows your own recipe, unmarked, once you walk back to it from his', async () => {
		// The screen is reused from one recipe to the next rather than remade,
		// so walking along the strip must not carry his page's comparison onto yours.
		const kamosu = standIn({ list_attempts: { attempts: [] }, ...onTheirs() });
		const { rerender } = render(RecipeTestHarness, {
			props: { client: kamosu.client, branchId: 'theirs' },
		});
		expect(await screen.findByText('1 Tbsp rice vinegar')).toBeInTheDocument();

		kamosu.answer('get_recipe', forked().get_recipe as GetRecipeOutput);
		await rerender({ client: kamosu.client, branchId: 'mine' });

		expect(await screen.findByText(/Tap another version to read it/)).toBeInTheDocument();
		expect(await screen.findByText('1 cup potato starch (or corn starch)')).toBeInTheDocument();
		expect(screen.queryByText('1 tsp gochugaru')).not.toBeInTheDocument();
		expect(screen.queryByText(/yours — Chez Marc/)).not.toBeInTheDocument();
	});
});

/**
 * Finishing a cook (#59) reaches the recipe as the Cooked section: how often,
 * when last, and each Person's most recent verdict by name.
 */
describe('the Cooked section', () => {
	const cooked = (extra: Partial<{ count: number; last_cooked_at: string | null }> = {}) =>
		forked({
			get_recipe: {
				...(forked().get_recipe as Record<string, unknown>),
				cooked: {
					count: 7,
					last_cooked_at: '2026-08-14T18:30:00Z',
					ratings: [
						{
							person_id: 'p_stephane',
							name: 'Stéphane',
							rating: 'again' as const,
							at: '2026-08-14T18:30:00Z',
						},
						{
							person_id: 'p_marie',
							name: 'Marie',
							rating: 'tweak' as const,
							at: '2026-08-02T19:00:00Z',
						},
					],
					...extra,
				},
			},
		} as Answers);

	it('shows each Person’s own verdict beside their name, and never a score', async () => {
		renderRecipe(cooked());

		expect(await screen.findByText('Stéphane')).toBeInTheDocument();
		expect(screen.getByText('Again')).toBeInTheDocument();
		expect(screen.getByText('Marie')).toBeInTheDocument();
		expect(screen.getByText('Tweak it')).toBeInTheDocument();

		// ADR 0015: no average, mean or aggregate anywhere on the page — and no
		// stars either, which is what a five-point scale would have left behind.
		expect(screen.queryByText(/average/i)).not.toBeInTheDocument();
		expect(screen.queryByText(/★/)).not.toBeInTheDocument();
		expect(screen.queryByText(/out of 5/i)).not.toBeInTheDocument();
	});

	it('says how many times and when last, counting every cooking', async () => {
		renderRecipe(cooked());
		expect(await screen.findByText(/Cooked 7 times/)).toBeInTheDocument();
	});

	it('says plainly that a recipe has never been cooked rather than showing an empty list', async () => {
		renderRecipe(cooked({ count: 0, last_cooked_at: null }));

		expect(await screen.findByText('Not cooked yet.')).toBeInTheDocument();
		expect(screen.queryByText('Again')).not.toBeInTheDocument();
	});
});

/**
 * The way into the Thread (#133, choice A). The button says
 * *History* rather than the Thread's own name, and carries a line built like
 * the Cooked line above it: how many saves are behind it and when the last
 * was. It never says "version", which on this page already means one of the
 * recipe's versions in the strip.
 */
describe('the way into the Thread', () => {
	const at = (occurrence: ReturnType<typeof occurrenceOf>, created_at: string) => ({
		...occurrence,
		created_at,
	});
	const withSaves = (versions: ReturnType<typeof occurrenceOf>[]) =>
		forked({
			get_thread: {
				...(forked().get_thread as Record<string, unknown>),
				versions,
			},
		} as Answers);
	const day = (iso: string) => new Date(iso).toLocaleDateString();
	const button = () => screen.findByRole('link', { name: /^History/ });
	/** The line under the button, once the Thread it counts has been read. */
	const saying = async (text: string) => {
		const line = await screen.findByText(text);
		expect(line.closest('a')).toBe(await button());
	};

	it('says History and leads to the Thread', async () => {
		renderRecipe();
		expect((await button()).getAttribute('href')).toBe('/recipes/mine/thread');
		expect(screen.queryByText(/the thread/i)).not.toBeInTheDocument();
	});

	it('counts the saves on this recipe’s own chain and dates the last', async () => {
		renderRecipe(
			withSaves([
				at(occurrenceOf('mine', 1, 'v_root', null), '2026-08-01T10:00:00Z'),
				at(occurrenceOf('mine', 2, 'v_mine', 'v_root'), '2026-08-09T10:00:00Z'),
				at(occurrenceOf('theirs', 1, 'v_root', null), '2026-08-01T10:00:00Z'),
				// Marc's save is later, and is his: it is neither counted nor dated.
				at(occurrenceOf('theirs', 2, 'v_theirs', 'v_root'), '2026-09-02T10:00:00Z'),
			]),
		);
		await saying(`Saved twice · last on ${day('2026-08-09T10:00:00Z')}`);
	});

	it('says one save plainly, which is every recipe straight from an import', async () => {
		renderRecipe(withSaves([at(occurrenceOf('mine', 1, 'v_mine', null), '2026-09-23T16:00:00Z')]));
		await saying(`Saved once · on ${day('2026-09-23T16:00:00Z')}`);
	});

	it('counts past two in figures', async () => {
		renderRecipe(
			withSaves([
				at(occurrenceOf('mine', 1, 'v_1', null), '2026-08-01T10:00:00Z'),
				at(occurrenceOf('mine', 2, 'v_2', 'v_1'), '2026-08-05T10:00:00Z'),
				at(occurrenceOf('mine', 3, 'v_mine', 'v_2'), '2026-08-20T10:00:00Z'),
			]),
		);
		await saying(`Saved 3 times · last on ${day('2026-08-20T10:00:00Z')}`);
	});
});

/**
 * The recipe screen itself (#81): the written Ingredient Line, its Reading,
 * the Sections, correcting a Reading in place, and a recipe wearing none of
 * the optional things.
 *
 * The layout was chosen on 29 August 2026 and the reasoning is on
 * the issue. What is tested here is not how it looks but what ADR 0002 makes
 * true of it: the written Line is the truth of the ingredient, a Reading is
 * subordinate to it, a Reading may be absent, and nothing on the page marks
 * which is which.
 */
describe('the recipe screen', () => {
	it('sets a named version’s name under its title, and none on an unnamed one (#131)', async () => {
		const named = forked({
			get_recipe: { ...(forked().get_recipe as GetRecipeOutput), name: 'Vegetarian' },
		});
		renderRecipe(named);
		const name = await screen.findByText('Vegetarian');
		expect(
			screen
				.getByRole('heading', { level: 1, name: 'Korean Fried Chicken' })
				.compareDocumentPosition(name) & Node.DOCUMENT_POSITION_FOLLOWING,
		).toBeTruthy();
	});

	const SECTIONED = [
		{ kind: 'section' as const, text: 'Chicken' },
		{ kind: 'ingredient' as const, text: '1.4 kg whole chicken' },
		{ kind: 'ingredient' as const, text: 'Some cooking oil (for deep frying)' },
		{ kind: 'section' as const, text: 'Sauce' },
		{ kind: 'ingredient' as const, text: '3 tbsp Ketchup' },
	];
	const SECTIONED_STEPS = [
		{ kind: 'section' as const, text: 'The day before', photo: null },
		{ kind: 'step' as const, text: 'Brine the chicken overnight.', photo: null },
		{ kind: 'section' as const, text: 'On the day', photo: null },
		{ kind: 'step' as const, text: 'Deep fry at 190 C until crisp.', photo: null },
	];

	/**
	 * One Branch, alone — no Divergence, which is the ordinary case. Two of the
	 * three Ingredient Lines carry a Reading and one does not, because that
	 * mixture is the thing ADR 0002 is about.
	 *
	 * Built as a real `GetRecipeOutput` rather than spread out of `forked()`
	 * and cast: a cast here would be the one place in this file where the
	 * Catalogue's shape stops being checked, which is the whole point of the
	 * stand-in.
	 */
	function solo(
		content: Record<string, unknown> = {},
		readings?: Slot[],
		measured?: GetRecipeOutput['versions'][number]['measured'],
		components?: GetRecipeOutput['versions'][number]['components'],
	): Answers {
		const base = divergence().mine.content;
		const recipe: GetRecipeOutput = {
			branch_id: 'mine',
			lineage_id: 'l_1',
			cookbook: MY_COOKBOOK,
			name: null,
			writes: true,
			mine: true,
			arrived: false,
			hand_id: 'h_mine',
			language: 'en',
			origin_address: null,
			head_version_id: 'v_mine',
			translation: null,
			tags: [],
			related_recipes: [],
			cooked: { count: 0, last_cooked_at: null, ratings: [] },
			versions: [
				{
					sequence: 1,
					scaled_to: null,
					version_id: 'v_mine',
					parent_version_id: null,
					hand_id: 'h_mine',
					name: null,
					change_note: null,
					created_at: '2026-08-09T00:00:00Z',
					translates_version_id: null,
					language: 'en',
					// A recipe that composes nothing, which is nearly all of them (#50).
					components: components ?? [],
					content: {
						...base,
						ingredients: SECTIONED,
						steps: SECTIONED_STEPS,
						...content,
					} as GetRecipeOutput['versions'][number]['content'],
					// A Reading is what Kamosu understood OF the line, not a copy
					// of it: it normalises the unit and names the Food. Sections
					// carry none — `set_reading` refuses one on a section.
					readings: readings ?? [
						null,
						{ amount: '1400', unit: 'g', target: 'chicken', lineage_id: null },
						null,
						null,
						{ amount: '3', unit: 'tbsp', target: 'ketchup', lineage_id: null },
					],
					// The one subordinate line the Core worked out for this
					// reader (#49). Null throughout by default: an American
					// reading a recipe already in her measures is told nothing,
					// and the Reading echo fills the slot instead.
					measured: measured ?? {
						ingredients: SECTIONED.map(() => null),
						steps: SECTIONED_STEPS.map(() => null),
					},
					// What the Core read out of each Step for the cooking screen
					// (#61): the Ingredient Lines it uses, and its timer. The
					// recipe page shows neither — it is here because one Version
					// has one declared shape wherever it is served.
					cooking: {
						steps: SECTIONED_STEPS.map((row) =>
							row.kind === 'step' ? { uses: [], timer_seconds: null } : null,
						),
					},
				},
			],
		};
		return {
			get_recipe: recipe,
			get_thread: {
				lineage_id: 'l_1',
				branches: [MINE_IN_THREAD],
				versions: [],
				attempts: [],
			},
		};
	}

	// ---- how much, for the errands (#109) -----------------------------------

	/**
	 * `solo()` whose recipe comes back scaled to eight servings whenever it is
	 * asked for at an amount — the chicken doubled, the oil with nothing Kamosu
	 * could scale, as the Core answers it.
	 */
	function scalable(extra: Answers = {}) {
		const plain = solo().get_recipe as GetRecipeOutput;
		const held: { kamosu?: ReturnType<typeof standIn> } = {};
		const asked = () =>
			(held.kamosu?.calls ?? [])
				.filter((call) => call.operation === 'get_recipe')
				.map((call) => (call.input as { wanted_yield?: { amount: string } }).wanted_yield)
				.at(-1);
		const answers: Answers = {
			...solo(),
			get_recipe: () => {
				const wanted = asked();
				if (!wanted) return plain;
				return {
					...plain,
					versions: [
						{
							...plain.versions[0],
							scaled_to: { amount: wanted.amount, noun: 'servings' },
							measured: {
								ingredients: [null, 'about 2.8 kg', null, null, 'about 6 tbsp'],
								steps: SECTIONED_STEPS.map(() => null),
							},
						},
					],
				};
			},
			get_shopping_list: { chosen: [], rows: [] },
			add_to_shopping_list: { chosen: [], rows: [] },
			set_shopping_yield: { chosen: [], rows: [] },
			...extra,
		};
		const rendered = renderRecipe(answers);
		held.kamosu = rendered.kamosu;
		return rendered;
	}

	it('reads the recipe at another amount for the errands, from the Core', async () => {
		const { kamosu } = scalable();
		expect(await screen.findByText('Amounts for 4 servings')).toBeInTheDocument();
		await fireEvent.click(screen.getByRole('button', { name: 'Change' }));
		await fireEvent.click(await screen.findByRole('button', { name: '×2 · 8' }));

		await vi.waitFor(() =>
			expect(kamosu.calls.at(-1)).toEqual({
				operation: 'get_recipe',
				input: { branch_id: 'mine', wanted_yield: { amount: '8', noun: 'servings' } },
			}),
		);
		expect(await screen.findByText('Amounts for 8 servings')).toBeInTheDocument();
		// Every figure is the Core's, beneath the recipe's own unchanged line.
		expect(screen.getByText('1.4 kg whole chicken')).toBeInTheDocument();
		expect(screen.getByText('about 2.8 kg')).toBeInTheDocument();
		// The line with nothing to scale says so, rather than standing silent
		// under a heading that says every amount is for eight.
		expect(screen.getByText(/^not scaled/)).toBeInTheDocument();
		// And nothing was written for having looked.
		const wrote = kamosu.calls.map((call) => call.operation);
		expect(wrote).not.toContain('save_recipe_version');
		expect(wrote).not.toContain('set_shopping_yield');
	});

	it('carries the amount onto the list and into the cooking', async () => {
		const { kamosu } = scalable();
		await fireEvent.click(await screen.findByRole('button', { name: 'Change' }));
		await fireEvent.click(await screen.findByRole('button', { name: '×2 · 8' }));
		expect(await screen.findByText('Amounts for 8 servings')).toBeInTheDocument();

		expect(screen.getByRole('link', { name: /Cook this/i })).toHaveAttribute(
			'href',
			'/cook/mine?amount=8&noun=servings',
		);
		await fireEvent.click(screen.getByRole('button', { name: /Add to shopping list/i }));
		await vi.waitFor(() =>
			expect(kamosu.calls.find((call) => call.operation === 'add_to_shopping_list')?.input).toEqual(
				{ branch_id: 'mine', shopping_yield: { amount: '8', noun: 'servings' } },
			),
		);
	});

	it('opens at the Shopping List’s amount, and changing it changes the list', async () => {
		const { kamosu } = scalable({
			get_shopping_list: {
				chosen: [
					{
						branch_id: 'mine',
						title: 'Korean Fried Chicken',
						gone: false,
						shopping_yield: { amount: '8', noun: 'servings' },
						written_yield: { amount: '4', noun: 'servings' },
					},
				],
				rows: [],
			},
		});
		expect(await screen.findByText('Amounts for 8 servings')).toBeInTheDocument();
		await fireEvent.click(screen.getByRole('button', { name: 'Change' }));
		await fireEvent.click(await screen.findByRole('button', { name: '×3 · 12' }));
		await vi.waitFor(() =>
			expect(kamosu.calls.find((call) => call.operation === 'set_shopping_yield')?.input).toEqual({
				branch_id: 'mine',
				shopping_yield: { amount: '12', noun: 'servings' },
			}),
		);
	});

	it('says scaling needs Kamosu, and keeps the amounts it had, with no network', async () => {
		const { kamosu } = scalable();
		expect(await screen.findByText('Amounts for 4 servings')).toBeInTheDocument();
		kamosu.answer('get_recipe', { refuse: 'internal', message: 'Kamosu could not be reached.' });
		await fireEvent.click(screen.getByRole('button', { name: 'Change' }));
		await fireEvent.click(await screen.findByRole('button', { name: '×2 · 8' }));
		expect(await screen.findByText('Scaling needs Kamosu to be reachable.')).toBeInTheDocument();
		expect(screen.getByText('Amounts for 4 servings')).toBeInTheDocument();
		expect(screen.queryByText('about 2.8 kg')).not.toBeInTheDocument();
	});

	/**
	 * THE NUTRITION FIGURE CLOSES THE INGREDIENTS (#84). That
	 * placement was chosen on 21 September 2026 against four treatments drawn on both
	 * surfaces: a fourth cell in the meta strip, a line directly under the
	 * strip, this, and a place beside the Source. The list is where what goes
	 * into the dish is already the subject, and the strip keeps the three cells
	 * #81 gave it.
	 *
	 * The figure always says what it counts. 308 on its own says nothing, and
	 * the two bases do not convert into each other without a weight the recipe
	 * does not carry (GLOSSARY.md, "Nutrition").
	 */
	it('closes the Ingredients with the figure, saying what it counts', async () => {
		renderRecipe(solo({ nutrition: { calories: 308, basis: 'per_serving' } }));

		const figure = await screen.findByText('308 kcal a serving');
		const last = screen.getByText('3 tbsp Ketchup');
		const method = screen.getByText('Method');
		// Under the last Ingredient Line and above the Method: the foot of the
		// list, rather than a fifth thing floating between two sections.
		expect(figure.compareDocumentPosition(last) & Node.DOCUMENT_POSITION_PRECEDING).toBeTruthy();
		expect(figure.compareDocumentPosition(method) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
	});

	it('says per 100 g where that is what the figure counts', async () => {
		renderRecipe(solo({ nutrition: { calories: 154, basis: 'per_100g' } }));

		expect(await screen.findByText('154 kcal per 100 g')).toBeInTheDocument();
	});

	it('shows no gap, no placeholder and no zero where a recipe carries none', async () => {
		// Most recipes carry no figure, and that is an ordinary state rather
		// than a missing one — so there is nothing there at all, not a dash.
		renderRecipe(solo());
		await screen.findByText('1.4 kg whole chicken');

		expect(screen.queryByText(/kcal/i)).not.toBeInTheDocument();
	});

	it('sets the written Line at full size with its Reading subordinate beneath it', async () => {
		renderRecipe(solo());

		// ADR 0002: the written line is the truth of the ingredient. The Reading
		// is Kamosu's understanding OF it and is set beneath, smaller.
		const line = await screen.findByText('1.4 kg whole chicken');
		const read = screen.getByText('1400 g chicken');
		expect(line).toBeInTheDocument();
		expect(read).toHaveClass('text-read');
		// One row, so the Reading belongs to that line and to no other.
		expect(line.closest('li')).toBe(read.closest('li'));
		// Subordinate means literally underneath it in the row, not beside it.
		expect(read.compareDocumentPosition(line) & Node.DOCUMENT_POSITION_PRECEDING).toBeTruthy();
	});

	/**
	 * The converted line, as the Core worked it out for a metric reader of a
	 * recipe written in American measures (#49). Only the two lines that carry
	 * a Reading can produce one; the step is the one with a temperature on it.
	 */
	const CONVERTED = {
		ingredients: [null, null, null, null, 'about 45 ml'],
		steps: [null, null, null, [{ written: '190 C', measured: 'about 375 °F' }]],
	};

	it('gives the converted amount the subordinate slot, and the Reading fills in', async () => {
		renderRecipe(solo({}, undefined, CONVERTED));

		// Scaling and conversion get ONE slot with the Reading echo (ADR 0016),
		// so the ketchup line says what it means for this cook rather than
		// repeating what Kamosu understood.
		const ketchup = await screen.findByText('3 tbsp Ketchup');
		const row = ketchup.closest('li') as HTMLElement;
		const beneath = Array.from(row.querySelectorAll('.text-read')).map((node) =>
			node.textContent?.trim(),
		);
		expect(beneath).toEqual(['about 45 ml']);
		expect(screen.queryByText('3 tbsp ketchup')).not.toBeInTheDocument();

		// The line above is untouched: a conversion is never a rewrite.
		expect(ketchup).toHaveClass('text-line');

		// A line with a Reading and nothing to convert keeps the echo, because
		// a blank slot would tell this cook less than the echo does — and this
		// echo does say something: the line says `1.4 kg`, Kamosu read `1400 g`.
		const chicken = screen.getByText('1.4 kg whole chicken').closest('li') as HTMLElement;
		expect(
			Array.from(chicken.querySelectorAll('.text-read')).map((node) => node.textContent?.trim()),
		).toEqual(['1400 g chicken']);
	});

	it('says nothing beneath a line whose Reading only repeats it', async () => {
		// Since #71 Kamosu reads every line it can, so most Readings are the
		// line again in fewer words. An echo of one would be a repetition
		// ADR 0016 forbids — and, appearing on exactly the lines Kamosu
		// managed to read, the badge ADR 0002 refuses.
		renderRecipe(
			solo(
				{
					ingredients: [
						{ kind: 'ingredient' as const, text: '2 gousses d’ail' },
						{ kind: 'ingredient' as const, text: 'Za’tar' },
						{ kind: 'ingredient' as const, text: '200 g de farine' },
					],
				},
				[
					{ amount: '2', unit: 'gousses', target: 'ail', lineage_id: null },
					{ amount: null, unit: null, target: 'Za’tar', lineage_id: null },
					// The one a person corrected: the line says 200 g of flour,
					// Kamosu has been told it is 250 g of wheat flour.
					{ amount: '250', unit: 'g', target: 'farine de blé', lineage_id: null },
				],
				{ ingredients: [null, null, null], steps: SECTIONED_STEPS.map(() => null) },
			),
		);

		const beneath = async (text: string) => {
			const row = (await screen.findByText(text)).closest('li') as HTMLElement;
			return Array.from(row.querySelectorAll('.text-read')).map((node) => node.textContent?.trim());
		};

		expect(await beneath('2 gousses d’ail')).toEqual([]);
		expect(await beneath('Za’tar')).toEqual([]);
		// The corrected Reading earns its slot: it says what the line does not.
		expect(await beneath('200 g de farine')).toEqual(['250 g farine de blé']);
	});

	it('never puts two small lines under one written line', async () => {
		renderRecipe(solo({}, undefined, CONVERTED));
		await screen.findByText('3 tbsp Ketchup');

		for (const line of Array.from(document.querySelectorAll('ul li'))) {
			expect(line.querySelectorAll('.text-read').length).toBeLessThanOrEqual(1);
		}
	});

	it('offers an oven temperature straight after the one written, and rewrites nothing', async () => {
		renderRecipe(solo({}, undefined, CONVERTED));

		// The ladder's answer sits right after the temperature it converts
		// (#150, option A): an addition drawn between the Step's words, never
		// written into them (ADR 0016).
		const converted = await screen.findByText('(about 375 °F)');
		const step = converted.closest('p') as HTMLElement;
		expect(converted).toHaveClass('text-read');
		expect(step.textContent?.replace(/\s+/g, ' ').trim()).toBe(
			'Deep fry at 190 C (about 375 °F) until crisp.',
		);
		const bare = step.cloneNode(true) as HTMLElement;
		bare.querySelectorAll('.text-read').forEach((span) => span.remove());
		expect(
			bare.textContent
				?.replace(/\u00a0/g, '')
				.replace(/\s+/g, ' ')
				.trim(),
		).toBe('Deep fry at 190 C until crisp.');

		// The step with no temperature is offered nothing at all.
		const brine = screen.getByText('Brine the chicken overnight.').closest('li') as HTMLElement;
		expect(brine.querySelector('.text-read')).toBeNull();
	});

	it('puts each conversion straight after what it converts in a Step, and changes no word', async () => {
		// Option A (#150), chosen over one line of figures beneath the Step.
		const written = 'Place 1 lb. ground chicken on the tray; preheat to 425°.';
		renderRecipe(
			solo(
				{
					steps: [
						{ kind: 'section' as const, text: 'The day before', photo: null },
						{ kind: 'step' as const, text: 'Brine the chicken overnight.', photo: null },
						{ kind: 'section' as const, text: 'On the day', photo: null },
						{ kind: 'step' as const, text: written, photo: null },
					],
				},
				undefined,
				{
					ingredients: SECTIONED.map(() => null),
					steps: [
						null,
						null,
						null,
						[
							{ written: '1 lb.', measured: 'about 455 g' },
							{ written: '425°', measured: 'about 220 °C' },
						],
					],
				},
			),
		);

		const addition = await screen.findByText('(about 455 g)');
		const paragraph = addition.closest('p') as HTMLElement;
		expect(addition).toHaveClass('text-read', 'text-ink-2');
		// The amount and its figure never break across a line (found live: "3"
		// ended one line and "oz. (about 85 g)" began the next).
		const held = addition.parentElement as HTMLElement;
		expect(held).toHaveClass('whitespace-nowrap');
		expect(held.textContent?.replace(/\s+/g, ' ')).toBe('1 lb. (about 455 g)');
		expect(paragraph.textContent?.replace(/\s+/g, ' ').trim()).toBe(
			'Place 1 lb. (about 455 g) ground chicken on the tray; preheat to 425° (about 220 °C).',
		);
		const bare = paragraph.cloneNode(true) as HTMLElement;
		bare.querySelectorAll('.text-read').forEach((span) => span.remove());
		expect(
			bare.textContent
				?.replace(/\u00a0/g, '')
				.replace(/\s+/g, ' ')
				.trim(),
		).toBe(written);

		// The oven sits after its own temperature, with no line of its own.
		expect(paragraph.contains(screen.getByText('(about 220 °C)'))).toBe(true);
		expect(screen.queryByText('about 220 °C')).not.toBeInTheDocument();
	});

	it('redraws the converted line the moment a Reading is corrected', async () => {
		renderRecipe({
			...solo({}, undefined, CONVERTED),
			set_reading: {
				line_index: 2,
				reading: { amount: '500', unit: 'ml', target: 'frying oil', lineage_id: null },
				measured: 'about 2⅛ cups',
			},
		});

		await fireEvent.click(await screen.findByText('Some cooking oil (for deep frying)'));
		await fireEvent.input(screen.getByLabelText(/What it is/i), {
			target: { value: 'frying oil' },
		});
		await fireEvent.input(screen.getByLabelText(/^Amount$/i), { target: { value: '500' } });
		await fireEvent.input(screen.getByLabelText(/^Unit$/i), { target: { value: 'ml' } });
		await fireEvent.click(screen.getByRole('button', { name: /Save the Reading/i }));

		// The Core answered with the line the corrected Reading now produces, so
		// the cook sees the new number rather than waiting for a refetch.
		expect(await screen.findByText('about 2⅛ cups')).toBeInTheDocument();
		expect(screen.queryByText('500 ml frying oil')).not.toBeInTheDocument();
	});

	it('shows a line with no Reading in full, and assumes no quantity anywhere', async () => {
		renderRecipe(solo());

		// "Some cooking oil (for deep frying)" has no amount in it at all, and
		// nothing read from it. ADR 0002: any code path that assumes a quantity
		// exists is a bug — so the line is simply the line.
		const line = await screen.findByText('Some cooking oil (for deep frying)');
		expect(line).toBeInTheDocument();
		const row = line.closest('li') as HTMLElement;
		expect(row.querySelector('.text-read')).toBeNull();
	});

	it('puts no mark on a line Kamosu read, nor on one it did not', async () => {
		renderRecipe(solo());
		await screen.findByText('1.4 kg whole chicken');

		// ADR 0002, and #44's own acceptance: a badge that fires sometimes
		// teaches people it fires always. Read and unread rows differ only in
		// whether a Reading is there to show.
		const read = (screen.getByText('1.4 kg whole chicken') as HTMLElement).closest(
			'li',
		) as HTMLElement;
		const unread = (screen.getByText('Some cooking oil (for deep frying)') as HTMLElement).closest(
			'li',
		) as HTMLElement;
		expect(unread.className).toBe(read.className);

		const page = document.body.textContent ?? '';
		for (const badge of [
			'unread',
			'not read',
			'unrecognised',
			'unknown quantity',
			'needs review',
		]) {
			expect(page.toLowerCase()).not.toContain(badge);
		}
	});

	it('renders a Section in either list as a real heading, not as an ingredient', async () => {
		renderRecipe(solo());

		for (const heading of ['Chicken', 'Sauce', 'The day before', 'On the day']) {
			expect(await screen.findByText(heading)).toBeInTheDocument();
		}

		// A Section is not a line of the list: it carries no marker, no Reading
		// and — in the Method — no step number.
		const sauce = (screen.getByText('Sauce') as HTMLElement).closest('li') as HTMLElement;
		expect(sauce.querySelector('.text-read')).toBeNull();
		// The two real steps are numbered 1 and 2; the Sections take no number.
		const numbers = Array.from(document.querySelectorAll('ol li'))
			.map((li) => li.querySelector('span')?.textContent?.trim())
			.filter((text) => text && /^\d+$/.test(text));
		expect(numbers).toEqual(['1', '2']);
	});

	it('corrects a Reading in place, through set_reading, making no Version', async () => {
		const { kamosu } = renderRecipe({
			...solo(),
			set_reading: {
				line_index: 2,
				reading: { amount: '500', unit: 'ml', target: 'frying oil', lineage_id: null },
				measured: null,
			},
		});

		// #32 item 193: a bad Reading is a tap to fix on screen. The gesture is
		// on the line itself, because that is what the Reading was read from.
		await fireEvent.click(await screen.findByText('Some cooking oil (for deep frying)'));
		const target = screen.getByLabelText(/What it is/i) as HTMLInputElement;
		await fireEvent.input(target, { target: { value: 'frying oil' } });
		await fireEvent.input(screen.getByLabelText(/^Amount$/i), { target: { value: '500' } });
		await fireEvent.input(screen.getByLabelText(/^Unit$/i), { target: { value: 'ml' } });
		await fireEvent.click(screen.getByRole('button', { name: /Save the Reading/i }));

		const sent = kamosu.calls.find((call) => call.operation === 'set_reading');
		expect(sent?.input).toEqual({
			branch_id: 'mine',
			line_index: 2,
			amount: '500',
			unit: 'ml',
			target: 'frying oil',
			lineage_id: null,
		});

		// Correcting a Reading is not an edit to the recipe: no Version is made,
		// so it can never appear in the Thread as a change to the words.
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('save_recipe_version');

		// And you are still on the recipe — the correction landed under the line.
		expect(await screen.findByText('500 ml frying oil')).toBeInTheDocument();
		expect(screen.getByText('Some cooking oil (for deep frying)')).toBeInTheDocument();
	});

	it('saves a Reading on Enter from any of its boxes (#203)', async () => {
		const user = userEvent.setup();
		const { kamosu } = renderRecipe({
			...solo(),
			set_reading: {
				line_index: 2,
				reading: { amount: '500', unit: 'ml', target: null, lineage_id: null },
				measured: null,
			},
		});

		await user.click(await screen.findByText('Some cooking oil (for deep frying)'));
		await user.type(screen.getByLabelText(/^Amount$/i), '500');
		await user.type(screen.getByLabelText(/^Unit$/i), 'ml{Enter}');

		const sent = kamosu.calls.filter((call) => call.operation === 'set_reading');
		expect(sent).toHaveLength(1);
		expect(sent[0].input).toMatchObject({ line_index: 2, amount: '500', unit: 'ml' });
	});

	/**
	 * The first of the two doors onto a Food (#107). It carries the SAVED word
	 * rather than whatever is in the box, and it hands over a word rather than
	 * an id, because a Reading has no Food id to hand — `reading_schema` keeps
	 * one out so that a Reading means the same thing in a Bundle elsewhere.
	 */
	it('offers the way to the Food a line named, carrying the word it was read from', async () => {
		renderRecipe(solo());

		await fireEvent.click(await screen.findByText('1.4 kg whole chicken'));
		const door = screen.getByRole('link', { name: /About chicken/i });
		expect(door).toHaveAttribute('href', '/foods?q=chicken');
	});

	it('offers no way to a Food where Kamosu read no Food, which has none to visit', async () => {
		// A line with no Reading is an ordinary state, not a failure (ADR 0002),
		// and there is no Food behind it to go and look at.
		renderRecipe(solo());

		await fireEvent.click(await screen.findByText('Some cooking oil (for deep frying)'));
		expect(screen.getByText('What Kamosu read')).toBeInTheDocument();
		expect(screen.queryByRole('link', { name: /^About / })).not.toBeInTheDocument();
	});

	it('clears a Reading entirely when Kamosu read nothing there', async () => {
		const { kamosu } = renderRecipe({
			...solo(),
			set_reading: { line_index: 1, reading: null, measured: null },
		});

		await fireEvent.click(await screen.findByText('1.4 kg whole chicken'));
		await fireEvent.click(screen.getByRole('button', { name: /Kamosu read nothing here/i }));

		// All three absent clears it: a line with no Reading is an ordinary
		// state, not a failure.
		expect(kamosu.calls.find((call) => call.operation === 'set_reading')?.input).toEqual({
			branch_id: 'mine',
			line_index: 1,
			amount: null,
			unit: null,
			target: null,
			// The WHOLE Reading, every time — the pointer at a Recipe included
			// (#50, ADR 0008). Leaving it out would not leave it alone: the
			// Operation replaces what is there, so an omitted Lineage is a
			// cleared one, and a Component would quietly become an ordinary
			// ingredient the first time somebody fixed its amount.
			lineage_id: null,
		});
		expect(screen.queryByText('1400 g chicken')).not.toBeInTheDocument();
		expect(screen.getByText('1.4 kg whole chicken')).toBeInTheDocument();
	});

	/**
	 * Making a line a Component from the corrector (#87). Until this there was
	 * no way to make one from the interface at all: `set_reading` took a
	 * `lineage_id` and nothing in the app ever sent one.
	 */
	const SHELF = {
		search_recipes: {
			query: null,
			closest: false,
			recipes: [
				{
					lineage_id: 'l_brine',
					branch_id: 'b_brine',
					title: 'Overnight Brine',
					language: 'en',
					language_fallback: false,
					main_photo: null,
					yield: { amount: '2', noun: 'litres' },
					matched: null,
					...writtenByMe(),
				},
			],
		},
	} as Answers;

	it('makes a line a Component by choosing a recipe, never by matching its words', async () => {
		const { kamosu } = renderRecipe({
			...solo(),
			...SHELF,
			set_reading: {
				line_index: 1,
				reading: { amount: '1.4', unit: 'kg', target: null, lineage_id: 'l_brine' },
				measured: null,
			},
		});

		await fireEvent.click(await screen.findByText('1.4 kg whole chicken'));
		// Nothing here has guessed anything: the corrector offers a way into the
		// library and suggests no recipe at all (ADR 0008).
		expect(screen.queryByText(/Overnight Brine/)).not.toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: /This line is a recipe/i }));
		await fireEvent.click(await screen.findByRole('button', { name: 'Overnight Brine' }));

		// Chosen, the Food field is gone: a Reading names a Food or a Recipe and
		// never both, and the Core refuses a Reading claiming to be both.
		expect(screen.queryByLabelText(/What it is/i)).not.toBeInTheDocument();
		expect(screen.getByText(/will name Overnight Brine/)).toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: /Save the Reading/i }));
		expect(kamosu.calls.find((call) => call.operation === 'set_reading')?.input).toEqual({
			branch_id: 'mine',
			line_index: 1,
			// The amount and the Unit are untouched by naming a recipe — they
			// are what says how MUCH of it is wanted (ADR 0008).
			amount: '1400',
			unit: 'g',
			target: null,
			// The Lineage, never a Branch and never a Version, so it goes on
			// resolving to whatever Branch of the brine its reader holds.
			lineage_id: 'l_brine',
		});
		// And no Version: saying a line names a recipe is a Reading, not an
		// edit to the words above it.
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('save_recipe_version');

		// The recipe IS re-read, though. A Component is not a Reading on this
		// page but the Core's unfolding of one — the title, how much of that
		// recipe is wanted, its scaled lines — and none of that can be worked
		// out here (ADR 0008). Without the re-read the page would go on drawing
		// the line exactly as it was.
		expect(kamosu.calls.filter((call) => call.operation === 'get_recipe').length).toBeGreaterThan(
			1,
		);
	});

	it('un-makes a Component, taking the line back to an ordinary one', async () => {
		const asComponent: Slot[] = [
			null,
			{ amount: '1.4', unit: 'kg', target: null, lineage_id: 'l_brine' },
			null,
			null,
			{ amount: '3', unit: 'tbsp', target: 'ketchup', lineage_id: null },
		];
		const { kamosu } = renderRecipe({
			// The Reading AND the Component the Core unfolds from it — the two
			// always arrive together, and a screen that saw one without the
			// other would be testing a state the server cannot produce.
			...solo({}, asComponent, undefined, [
				{
					path: [1],
					lineage_id: 'l_brine',
					held: true,
					stopped: false,
					branch_id: 'b_brine',
					title: 'Overnight Brine',
					share: 0.7,
					said: 'Overnight Brine · 0.7 of the recipe',
					content: null,
					readings: null,
					measured: null,
				},
			]),
			...SHELF,
			set_reading: {
				line_index: 1,
				reading: { amount: '1.4', unit: 'kg', target: 'whole chicken', lineage_id: null },
				measured: null,
			},
		});

		await fireEvent.click(await screen.findByText('1.4 kg whole chicken'));
		await fireEvent.click(screen.getByRole('button', { name: /This line is not a recipe/i }));

		// The Food field comes back, because the slot the Lineage was in is the
		// slot the Food goes in. Nothing was written to get here: un-making is
		// a draft change like every other field in the box.
		const target = screen.getByLabelText(/What it is/i) as HTMLInputElement;
		await fireEvent.input(target, { target: { value: 'whole chicken' } });
		await fireEvent.click(screen.getByRole('button', { name: /Save the Reading/i }));

		expect(kamosu.calls.find((call) => call.operation === 'set_reading')?.input).toEqual({
			branch_id: 'mine',
			line_index: 1,
			amount: '1.4',
			unit: 'kg',
			target: 'whole chicken',
			lineage_id: null,
		});
		// Un-making re-reads for the same reason making does: the unfolding
		// this line had is the Core's, and only the Core can say it is gone.
		expect(kamosu.calls.filter((call) => call.operation === 'get_recipe').length).toBeGreaterThan(
			1,
		);
	});

	it('re-reads when a Component keeps its recipe but changes how much is wanted', async () => {
		// The pointer is untouched here — only the amount moves. Everything that
		// says how much of the dough is wanted is the Core's, worked out from
		// that amount: the share, the sentence under the line, and the inner
		// recipe's own lines scaled by it (ADR 0008). Comparing pointers alone
		// would leave the row reading 250 g above an unfolding still at 500.
		const { kamosu } = renderRecipe({
			...solo(
				{},
				[
					null,
					{ amount: '500', unit: 'g', target: null, lineage_id: 'l_brine' },
					null,
					null,
					{ amount: '3', unit: 'tbsp', target: 'ketchup', lineage_id: null },
				],
				undefined,
				[
					{
						path: [1],
						lineage_id: 'l_brine',
						held: true,
						stopped: false,
						branch_id: 'b_brine',
						title: 'Overnight Brine',
						share: 0.5,
						said: 'Overnight Brine · ½ of the recipe',
						content: null,
						readings: null,
						measured: null,
					},
				],
			),
			...SHELF,
			set_reading: {
				line_index: 1,
				reading: { amount: '250', unit: 'g', target: null, lineage_id: 'l_brine' },
				measured: null,
			},
		});

		await fireEvent.click(await screen.findByText('1.4 kg whole chicken'));
		await fireEvent.input(screen.getByLabelText(/^Amount$/i), { target: { value: '250' } });
		await fireEvent.click(screen.getByRole('button', { name: /Save the Reading/i }));

		expect(kamosu.calls.filter((call) => call.operation === 'get_recipe').length).toBeGreaterThan(
			1,
		);
	});

	it('leaves what a save did behind when you walk to another recipe', async () => {
		// This screen is reused from one recipe to the next rather than remade,
		// so a banner that is not cleared follows you to a recipe you never
		// wrote on. Harmless for *Saved*; for #87's *a line could not be
		// marked* it is an alert about a defect that is not there.
		const { goTo } = renderRecipe({
			...solo(),
			save_recipe_version: {
				branch_id: 'mine',
				version_id: 'v_2',
				parent_version_id: 'v_mine',
				sequence: 2,
				collapsed: false,
				copied: false,
				language: 'en',
				language_offer: null,
				translates_version_id: null,
			},
			// Walking to a second Branch makes the page ask whether the two have
			// diverged. Nothing here is about a Divergence, so it answers that
			// there is none to read.
			divergence: { refuse: 'not_found' },
		} as Answers);

		await fireEvent.click(await screen.findByRole('button', { name: 'Edit this recipe' }));
		const save = await screen.findAllByRole('button', { name: /Save onto mine/ });
		await fireEvent.click(save[0] as HTMLElement);
		const inSheet = await screen.findAllByRole('button', { name: /Save onto mine/ });
		await fireEvent.click(inSheet[inSheet.length - 1] as HTMLElement);
		expect(await screen.findByText('Saved.')).toBeInTheDocument();

		await goTo('another');
		await tick();
		expect(screen.queryByText('Saved.')).not.toBeInTheDocument();
	});

	it('does not re-read the recipe for a correction that touches no Component', async () => {
		// The ordinary correction, which is nearly every correction. Everything
		// it changes is laid over the Readings on this page already, so a
		// refetch would be a round trip that changes nothing on screen.
		const { kamosu } = renderRecipe({
			...solo(),
			set_reading: {
				line_index: 2,
				reading: { amount: '500', unit: 'ml', target: 'frying oil', lineage_id: null },
				measured: null,
			},
		});
		await fireEvent.click(await screen.findByText('Some cooking oil (for deep frying)'));
		await fireEvent.input(screen.getByLabelText(/What it is/i), {
			target: { value: 'frying oil' },
		});
		await fireEvent.click(screen.getByRole('button', { name: /Save the Reading/i }));
		await screen.findByText('500 ml frying oil');
		expect(kamosu.calls.filter((call) => call.operation === 'get_recipe')).toHaveLength(1);
	});

	// #175, reading option 1: the unit moves into the figure, so a 9-hour
	// prove reads 9 h rather than 540 over "min cook".
	it('writes each time with its unit in the figure, in hours once it passes an hour', async () => {
		renderRecipe(solo({ prep_time_minutes: 90, cook_time_minutes: 540 }));
		const prep = (await screen.findByText('prep')).previousElementSibling;
		const cook = screen.getByText('cook').previousElementSibling;
		expect(prep).toHaveTextContent(/^1\s*h\s*30$/);
		expect(cook).toHaveTextContent(/^9\s*h$/);
		expect(screen.queryByText(/540/)).not.toBeInTheDocument();
	});

	// #175: the + on Recipes makes the recipe from its title and hands the
	// pasted lines here, where they wait for Save like a paste made here does.
	it('opens a recipe the + made from pasted text on its writing screen, filled and unsaved', async () => {
		holdPaste('mine', {
			title: 'Korean Fried Chicken',
			note: 'Best eaten the day it is fried.',
			ingredients: [
				{ kind: 'section', text: 'Brine' },
				{ kind: 'ingredient', text: '1 lemon' },
			],
			steps: [{ kind: 'step', text: 'Squeeze it over.' }],
		});
		const { kamosu } = renderRecipe(solo({ ingredients: [], steps: [] }));

		expect(await screen.findByDisplayValue('1 lemon')).toBeInTheDocument();
		expect(screen.getByDisplayValue('Brine')).toBeInTheDocument();
		expect(screen.getByDisplayValue('Squeeze it over.')).toBeInTheDocument();
		// What the paste said about the recipe waits in its note (#176).
		expect(screen.getByDisplayValue('Best eaten the day it is fried.')).toBeInTheDocument();
		expect(kamosu.calls.some((call) => call.operation === 'save_recipe_version')).toBe(false);
		// Held once: nothing is waiting for this recipe any more.
		expect(takePaste('mine')).toBeUndefined();
	});

	it('leaves a paste held for another recipe where it is', async () => {
		const draft = {
			title: 'Not this one',
			note: null,
			ingredients: [{ kind: 'ingredient' as const, text: 'a stray line' }],
			steps: [],
		};
		holdPaste('someone-else', draft);
		renderRecipe(solo());

		expect(await screen.findByRole('button', { name: 'Edit this recipe' })).toBeInTheDocument();
		expect(screen.queryByDisplayValue('a stray line')).not.toBeInTheDocument();
		expect(takePaste('someone-else')).toEqual(draft);
	});

	it('writes a time under an hour in minutes', async () => {
		renderRecipe(solo({ prep_time_minutes: 20, cook_time_minutes: 5 }));
		expect((await screen.findByText('prep')).previousElementSibling).toHaveTextContent(
			/^20\s*min$/,
		);
		expect(screen.getByText('cook').previousElementSibling).toHaveTextContent(/^5\s*min$/);
	});

	it('reads whole with no photo, no Yield, no times, no Note and no Source', async () => {
		renderRecipe(
			solo({
				main_photo: null,
				nutrition: null,
				source: null,
				note: null,
				yield: null,
				prep_time_minutes: null,
				cook_time_minutes: null,
			}),
		);

		// ADR 0002's degrade-gracefully rule reaches every optional field, not
		// only the quantity: what is left is a whole, cookable recipe.
		expect(await screen.findByText('1.4 kg whole chicken')).toBeInTheDocument();
		expect(screen.getByText('Brine the chicken overnight.')).toBeInTheDocument();
		expect(screen.getByRole('heading', { name: 'Korean Fried Chicken' })).toBeInTheDocument();

		// Nothing is left standing empty where a field used to be.
		expect(screen.queryByText(/^prep$/i)).not.toBeInTheDocument();
		expect(screen.queryByText(/^cook$/i)).not.toBeInTheDocument();
		expect(screen.queryByText(/^From /i)).not.toBeInTheDocument();
	});

	// ── Components (#50, ADR 0008) ───────────────────────────────────────────
	//
	// A Component is an ordinary Ingredient Line whose Reading names a Recipe
	// rather than a Food. Everything the screen shows about one — which recipe,
	// how much of it, and the three sentences for when there is nothing to
	// unfold — is worded in the Core, so what these tests hold is the LAYOUT
	// Chosen on 3 September 2026: B, the annexe.

	/** A recipe whose first line names another recipe. */
	function withComponent(
		component: Partial<GetRecipeOutput['versions'][number]['components'][number]> = {},
	): Answers {
		const answers = solo({}, [
			null,
			{ amount: '500', unit: 'g', target: null, lineage_id: 'l_dough' },
			null,
			null,
			null,
		]);
		const recipe = answers.get_recipe as GetRecipeOutput;
		recipe.versions[0].components = [
			{
				path: [1],
				lineage_id: 'l_dough',
				held: true,
				stopped: false,
				branch_id: 'b_dough',
				title: 'Neapolitan Pizza Dough',
				share: 0.5,
				said: 'Neapolitan Pizza Dough · ½ of the recipe',
				content: {
					title: 'Neapolitan Pizza Dough',
					yield: { amount: '1', noun: 'kg' },
					prep_time_minutes: null,
					cook_time_minutes: null,
					note: null,
					main_photo: null,
					nutrition: null as Nutrition,
					source: null,
					ingredients: [{ kind: 'ingredient', text: '600 g tipo 00 flour' }],
					steps: [{ kind: 'step', text: 'Knead for ten minutes.', photo: null }],
				},
				readings: [{ amount: '600', unit: 'g', target: 'flour', lineage_id: null }],
				measured: { ingredients: ['about 300 g'], steps: [null] },
				...component,
			} as GetRecipeOutput['versions'][number]['components'][number],
		];
		return answers;
	}

	it('leaves a Component closed, saying which recipe it names and how much of it', async () => {
		renderRecipe(withComponent());

		// ADR 0008: closed by default. The row says what is behind it and the
		// recipe itself is a tap away — so the dough's fifteen steps are not
		// sitting between two ingredients nobody asked to move apart.
		const said = await screen.findByRole('button', {
			name: /Neapolitan Pizza Dough · ½ of the recipe/,
		});
		expect(said).toHaveAttribute('aria-expanded', 'false');
		expect(screen.queryByText('600 g tipo 00 flour')).not.toBeInTheDocument();
		expect(screen.queryByText('Knead for ten minutes.')).not.toBeInTheDocument();

		// The written line is untouched and still its own tap, as every line is.
		expect(await screen.findByText(SECTIONED[1].text)).toBeInTheDocument();
	});

	it('unfolds a Component in place and sets its method at the foot of the page', async () => {
		renderRecipe(withComponent());
		await fireEvent.click(
			await screen.findByRole('button', { name: /Neapolitan Pizza Dough · ½ of the recipe/ }),
		);

		// Its INGREDIENT LINES unfold here, already scaled by how much of that
		// recipe this line asks for — so the list stays a list you can shop from.
		const inner = await screen.findByText('600 g tipo 00 flour');
		expect(inner).toBeInTheDocument();
		expect(screen.getByText('about 300 g')).toHaveClass('text-read');

		// Its STEPS are NOT among them. Treatment B puts them at the foot, under
		// a heading of their own, and the row says where they went.
		const method = await screen.findByText('Knead for ten minutes.');
		const heading = screen.getByText(/Neapolitan Pizza Dough · its own method/i);
		expect(inner.compareDocumentPosition(heading) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
		expect(heading.compareDocumentPosition(method) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();

		// The outer Method never grew the dough's Steps: composition says WHAT
		// and never WHEN (ADR 0008).
		const ownMethod = screen.getByText(SECTIONED_STEPS.find((row) => row.kind === 'step')!.text);
		expect(
			ownMethod.compareDocumentPosition(heading) & Node.DOCUMENT_POSITION_FOLLOWING,
		).toBeTruthy();

		// And the row links to it, because the foot of the page is a long way off.
		const link = screen.getByRole('link', { name: /at the foot of the page/i });
		expect(link).toHaveAttribute('href', '#annexe-1');
	});

	it('folds a Component away again, method and all', async () => {
		renderRecipe(withComponent());
		const said = await screen.findByRole('button', {
			name: /Neapolitan Pizza Dough · ½ of the recipe/,
		});
		await fireEvent.click(said);
		expect(await screen.findByText('Knead for ten minutes.')).toBeInTheDocument();

		await fireEvent.click(said);
		// The foot of the page holds exactly what the list above says is open.
		expect(screen.queryByText('Knead for ten minutes.')).not.toBeInTheDocument();
		expect(screen.queryByText('600 g tipo 00 flour')).not.toBeInTheDocument();
	});

	it('leaves a sentence rather than a hole where the recipe is not here', async () => {
		renderRecipe(
			withComponent({
				held: false,
				branch_id: null,
				title: null,
				share: null,
				said: 'Kamosu does not have this recipe.',
				content: null,
				readings: null,
				measured: null,
			}),
		);

		// The written line still reads correctly — it was always the truth
		// (ADR 0002) — and beneath it one sentence, with nothing to tap: a
		// missing recipe is not a door.
		expect(await screen.findByText(SECTIONED[1].text)).toBeInTheDocument();
		const said = await screen.findByText('Kamosu does not have this recipe.');
		expect(said).toHaveClass('text-read');
		expect(said.tagName).toBe('SPAN');
		expect(said.closest('button')).toBeNull();
	});

	it('says plainly where unfolding stopped at a repeat', async () => {
		renderRecipe(
			withComponent({
				stopped: true,
				branch_id: null,
				share: null,
				said: 'Pizza Margherita is already open above — Kamosu stops here.',
				content: null,
				readings: null,
				measured: null,
			}),
		);

		// A cycle is never refused (ADR 0008); it stops, and says so. Nothing to
		// open, because there is nothing behind it.
		const said = await screen.findByText(
			'Pizza Margherita is already open above — Kamosu stops here.',
		);
		expect(said.tagName).toBe('SPAN');
		expect(said.closest('button')).toBeNull();
	});
});

/**
 * Correcting a Reading where a Divergence is on the page. The gesture belongs
 * to the recipe, not to the marking.
 */
describe('correcting a Reading beside a Divergence', () => {
	it('offers the corrector on every line of your own version', async () => {
		renderRecipe();
		expect(await screen.findByText('¼ cup honey')).toBeInTheDocument();
		expect(screen.getByText('¼ cup honey').closest('button')).toBeInTheDocument();
		expect(
			screen.getByText('1 cup potato starch (or corn starch)').closest('button'),
		).toBeInTheDocument();
	});

	it('closes an open panel when the marking is put away', async () => {
		renderRecipe(onTheirs(), 'theirs');
		await compared();

		// Putting the marks away rebuilds the list from a different set of
		// rows, so an open panel would reopen on whatever line lands at that index.
		await fireEvent.click(await screen.findByText('¾ cup potato starch'));
		expect(screen.getByRole('button', { name: /Write this into mine/i })).toBeInTheDocument();
		await fireEvent.click(screen.getByRole('button', { name: /Hide them/i }));
		expect(screen.queryByRole('button', { name: /Write this into mine/i })).not.toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: /Show them/i }));
		expect(await screen.findByText('¾ cup potato starch')).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: /Write this into mine/i })).not.toBeInTheDocument();
	});

	it('puts the recipe on the reader’s Shopping List, and takes it back off', async () => {
		// The list is a Person's, not a Kitchen's (ADR 0024), so the recipe
		// screen asks for it separately rather than reading it off the recipe.
		const { kamosu } = renderRecipe({
			...forked(),
			get_shopping_list: { chosen: [], rows: [] },
			add_to_shopping_list: { chosen: [], rows: [] },
			remove_from_shopping_list: { chosen: [], rows: [] },
		});
		await screen.findByText('This recipe, 2 versions');

		const button = await screen.findByRole('button', { name: /Add to shopping list/i });
		await fireEvent.click(button);
		expect(kamosu.calls.find((call) => call.operation === 'add_to_shopping_list')?.input).toEqual({
			branch_id: 'mine',
		});

		// What it stores is the choosing, so the button now offers the way back.
		const on = await screen.findByRole('button', { name: /On your shopping list/i });
		await fireEvent.click(on);
		expect(
			kamosu.calls.find((call) => call.operation === 'remove_from_shopping_list')?.input,
		).toEqual({ branch_id: 'mine' });
		expect(
			await screen.findByRole('button', { name: /Add to shopping list/i }),
		).toBeInTheDocument();
	});
});

describe('a Sheet (#75)', () => {
	afterEach(() => {
		vi.unstubAllGlobals();
	});

	const sheetJob = {
		id: 'j_sheet',
		operation: 'make_sheet',
		status: 'completed' as const,
		progress: {},
		error: null,
		errorCode: null,
		created_at: '2026-09-19T20:00:00.000Z',
		updated_at: '2026-09-19T20:00:01.000Z',
		result: {
			file_name: 'Korean Fried Chicken.pdf',
			fetch_at: '/api/sheets/j_sheet',
			paper: 'a4',
			pages: 1,
			kept_as: 's_0-1.pdf',
		},
	};

	it('asks the server to set this recipe, waits on the Job, and opens the PDF in a tab', async () => {
		// The tab is opened at the tap and filled once the Job ends: a tab
		// opened from a promise is one a browser may block.
		const tab = { location: { href: '' }, close: vi.fn() };
		const open = vi.fn(() => tab);
		vi.stubGlobal('open', open);
		const { kamosu } = renderRecipe({
			...forked(),
			make_sheet: { job_id: 'j_sheet' },
			get_job: sheetJob,
		});
		await screen.findByText('This recipe, 2 versions');

		await fireEvent.click(await screen.findByRole('button', { name: /Print a sheet/i }));
		expect(open).toHaveBeenCalledWith('', '_blank');
		await vi.waitFor(() => expect(tab.location.href).toBe('/api/sheets/j_sheet'));
		expect(kamosu.calls.find((call) => call.operation === 'make_sheet')?.input).toEqual({
			branch_id: 'mine',
		});
	});

	it('says so when the sheet could not be made, and closes the tab it opened', async () => {
		const tab = { location: { href: '' }, close: vi.fn() };
		vi.stubGlobal(
			'open',
			vi.fn(() => tab),
		);
		renderRecipe({
			...forked(),
			make_sheet: { job_id: 'j_sheet' },
			get_job: { ...sheetJob, status: 'failed', result: null, error: 'no such Branch' },
		});
		await screen.findByText('This recipe, 2 versions');

		await fireEvent.click(await screen.findByRole('button', { name: /Print a sheet/i }));
		expect(await screen.findByRole('alert')).toHaveTextContent(/could not be made/);
		expect(tab.close).toHaveBeenCalled();
	});

	it('says a sheet still being set is still being set, and opens it when it is ready (#117)', () =>
		withTheClockFaked(async () => {
			const tab = { location: { href: '' }, close: vi.fn() };
			vi.stubGlobal(
				'open',
				vi.fn(() => tab),
			);
			renderRecipe({
				...forked(),
				make_sheet: { job_id: 'j_sheet' },
				get_job: outlivingTheWait(
					{ ...sheetJob, status: 'running' as const, result: null },
					sheetJob,
				),
			});
			await screen.findByText('This recipe, 2 versions');

			await fireEvent.click(await screen.findByRole('button', { name: /Print a sheet/i }));
			// Several lines on this screen are statuses, so the sentence is found by
			// its words and then checked for the role.
			expect(await screen.findByText(/The sheet is taking a while/)).toHaveAttribute(
				'role',
				'status',
			);
			expect(screen.queryByText(/could not be made/)).not.toBeInTheDocument();
			// Still being set, so not offered again: a second press is a second tab.
			expect(screen.getByRole('button', { name: /Setting the sheet/ })).toBeDisabled();

			// The wait goes on, and the Sheet lands in the tab that was promised.
			await vi.waitFor(() => expect(tab.location.href).toBe('/api/sheets/j_sheet'));
			expect(tab.close).not.toHaveBeenCalled();
		}));

	/** A Sheet still being set, asked for and outwaited, on the recipe `mine`. */
	async function stillSetting() {
		const tab = { location: { href: '' }, close: vi.fn() };
		vi.stubGlobal(
			'open',
			vi.fn(() => tab),
		);
		const rendered = renderRecipe({
			...forked(),
			make_sheet: { job_id: 'j_sheet' },
			get_job: outlivingTheWait({ ...sheetJob, status: 'running' as const, result: null }),
		});
		await screen.findByText('This recipe, 2 versions');
		await fireEvent.click(await screen.findByRole('button', { name: /Print a sheet/i }));
		await screen.findByText(/The sheet is taking a while/);
		return { tab, ...rendered };
	}

	it('stops waiting on a sheet, and closes its empty tab, when the cook walks to another recipe (#117)', () =>
		withTheClockFaked(async () => {
			const { tab, goTo } = await stillSetting();

			await goTo('theirs');

			await vi.waitFor(() => expect(tab.close).toHaveBeenCalled());
			// The sheet was the last recipe's: this one offers its own.
			expect(screen.queryByText(/The sheet is taking a while/)).not.toBeInTheDocument();
			expect(screen.getByRole('button', { name: /Print a sheet/ })).toBeEnabled();
		}));

	it('stops waiting on a sheet, and closes its empty tab, when the screen closes (#117)', () =>
		withTheClockFaked(async () => {
			const { tab } = await stillSetting();

			cleanup();

			await vi.waitFor(() => expect(tab.close).toHaveBeenCalled());
			expect(tab.location.href).toBe('');
		}));

	describe('in the app installed on an iPhone (#149)', () => {
		const installedApple = { installed: true, apple: true };

		/**
		 * The share sheet, as iOS offers it to an installed app. `canShare` says
		 * yes to one PDF; `share` answers what the test says.
		 */
		function theShareSheet(answer: () => Promise<void> = async () => {}) {
			const share = vi.fn<(data: ShareData) => Promise<void>>(() => answer());
			const canShare = vi.fn(
				(data?: ShareData) =>
					data?.files?.length === 1 && data.files[0]?.type === 'application/pdf',
			);
			Object.defineProperty(navigator, 'share', { value: share, configurable: true });
			Object.defineProperty(navigator, 'canShare', { value: canShare, configurable: true });
			return share;
		}

		/** `/api/sheets/…` as the server answers it, headers and all. */
		function theSheetRoute() {
			const fetch = vi.fn(
				async () =>
					new Response('%PDF-1.7', {
						headers: {
							'content-type': 'application/pdf',
							'content-disposition':
								'inline; filename="Korean Fried Chicken.pdf"; filename*=UTF-8\'\'Korean%20Fried%20Chicken.pdf',
						},
					}),
			);
			return { fetch, files: realFiles(fetch as unknown as typeof globalThis.fetch) };
		}

		afterEach(() => {
			Reflect.deleteProperty(navigator, 'share');
			Reflect.deleteProperty(navigator, 'canShare');
		});

		/** Print tapped and the Sheet ready: the button now shares it. */
		async function readyToShare(
			share = theShareSheet(),
			answers: Answers = { ...forked(), make_sheet: { job_id: 'j_sheet' }, get_job: sheetJob },
		) {
			const open = vi.fn();
			vi.stubGlobal('open', open);
			const route = theSheetRoute();
			const rendered = renderRecipe(answers, 'mine', {
				device: installedApple,
				files: route.files,
			});
			await screen.findByText('This recipe, 2 versions');
			await fireEvent.click(await screen.findByRole('button', { name: /Print a sheet/i }));
			const button = await screen.findByRole('button', { name: 'Share the sheet' });
			return { open, share, route, button, ...rendered };
		}

		it('opens no tab, and shares the Sheet as one PDF named as the server named it', async () => {
			const { open, share, route, button } = await readyToShare();

			expect(open).not.toHaveBeenCalled();
			expect(route.fetch).toHaveBeenCalledWith('/api/sheets/j_sheet');
			await fireEvent.click(button);

			expect(share).toHaveBeenCalledTimes(1);
			const files = share.mock.calls[0]?.[0].files ?? [];
			expect(files).toHaveLength(1);
			expect(files[0]?.name).toBe('Korean Fried Chicken.pdf');
			expect(files[0]?.type).toBe('application/pdf');
		});

		it('shares in the tap itself, with the file already fetched', async () => {
			const { share, route, button } = await readyToShare();
			const fetchedBefore = route.fetch.mock.calls.length;

			// Not awaited: the share sheet must be asked for before the tap's
			// handler gives the browser back, or iOS no longer counts the tap.
			const tapped = fireEvent.click(button);
			expect(share).toHaveBeenCalledTimes(1);
			expect(route.fetch).toHaveBeenCalledTimes(fetchedBefore);
			await tapped;
		});

		it('keeps the Sheet ready, and says nothing went wrong, when the share sheet is closed', async () => {
			const share = theShareSheet(async () => {
				throw new DOMException('Share canceled', 'AbortError');
			});
			const { button } = await readyToShare(share);

			await fireEvent.click(button);
			await tick();

			expect(screen.queryByText(/could not be made/)).not.toBeInTheDocument();
			const again = screen.getByRole('button', { name: 'Share the sheet' });
			expect(again).toBeEnabled();
			await fireEvent.click(again);
			expect(share).toHaveBeenCalledTimes(2);
			expect(share.mock.calls[1]?.[0].files?.[0]).toBe(share.mock.calls[0]?.[0].files?.[0]);
		});

		it('says the sheet could not be made when the share sheet refuses it', async () => {
			const share = theShareSheet(async () => {
				throw new DOMException('Not allowed', 'NotAllowedError');
			});
			const { button } = await readyToShare(share);

			await fireEvent.click(button);

			expect(await screen.findByRole('alert')).toHaveTextContent(/could not be made/);
			expect(screen.getByRole('button', { name: /Print a sheet/i })).toBeEnabled();
		});

		it('offers a fresh sheet once one has been shared', async () => {
			const { button } = await readyToShare();

			await fireEvent.click(button);

			expect(await screen.findByRole('button', { name: /Print a sheet/i })).toBeEnabled();
		});

		it('shares a Sheet already on the phone while the server is out of reach', async () => {
			const { share } = await readyToShare();
			Object.defineProperty(navigator, 'onLine', { value: false, configurable: true });
			try {
				window.dispatchEvent(new Event('offline'));
				await tick();

				await fireEvent.click(screen.getByRole('button', { name: 'Share the sheet' }));
				expect(share).toHaveBeenCalledTimes(1);
			} finally {
				Reflect.deleteProperty(navigator, 'onLine');
			}
		});

		it('drops the Sheet ready to share when the cook walks to another recipe', async () => {
			const { goTo } = await readyToShare();

			await goTo('theirs');

			expect(await screen.findByRole('button', { name: /Print a sheet/i })).toBeEnabled();
			expect(screen.queryByRole('button', { name: 'Share the sheet' })).not.toBeInTheDocument();
		});

		it('says a slow sheet can be shared when it is ready, and not that it opens', () =>
			withTheClockFaked(async () => {
				theShareSheet();
				vi.stubGlobal('open', vi.fn());
				renderRecipe(
					{
						...forked(),
						make_sheet: { job_id: 'j_sheet' },
						get_job: outlivingTheWait(
							{ ...sheetJob, status: 'running' as const, result: null },
							sheetJob,
						),
					},
					'mine',
					{ device: installedApple, files: theSheetRoute().files },
				);
				await screen.findByText('This recipe, 2 versions');
				await fireEvent.click(await screen.findByRole('button', { name: /Print a sheet/i }));

				expect(
					await screen.findByText(/Stay on this page and you can share it when it is ready/),
				).toHaveAttribute('role', 'status');
				expect(await screen.findByRole('button', { name: 'Share the sheet' })).toBeEnabled();
			}));

		it.each([
			['a Safari tab on an iPhone', { installed: false, apple: true }],
			['an installed app that is not on Apple', { installed: true, apple: false }],
		])('still opens the Sheet in a tab in %s', async (_, device) => {
			const share = theShareSheet();
			const tab = { location: { href: '' }, close: vi.fn() };
			const open = vi.fn(() => tab);
			vi.stubGlobal('open', open);
			const route = theSheetRoute();
			renderRecipe({ ...forked(), make_sheet: { job_id: 'j_sheet' }, get_job: sheetJob }, 'mine', {
				device,
				files: route.files,
			});
			await screen.findByText('This recipe, 2 versions');

			await fireEvent.click(await screen.findByRole('button', { name: /Print a sheet/i }));

			expect(open).toHaveBeenCalledWith('', '_blank');
			await vi.waitFor(() => expect(tab.location.href).toBe('/api/sheets/j_sheet'));
			expect(route.fetch).not.toHaveBeenCalled();
			expect(share).not.toHaveBeenCalled();
		});

		it('still opens the Sheet in a tab where the phone cannot share a file', async () => {
			const tab = { location: { href: '' }, close: vi.fn() };
			vi.stubGlobal(
				'open',
				vi.fn(() => tab),
			);
			renderRecipe({ ...forked(), make_sheet: { job_id: 'j_sheet' }, get_job: sheetJob }, 'mine', {
				device: installedApple,
				files: theSheetRoute().files,
			});
			await screen.findByText('This recipe, 2 versions');

			await fireEvent.click(await screen.findByRole('button', { name: /Print a sheet/i }));

			await vi.waitFor(() => expect(tab.location.href).toBe('/api/sheets/j_sheet'));
		});
	});
});

describe('on the phone (#76)', () => {
	const kept = new Date(2026, 8, 12);

	function offlineWithKeptCopy() {
		Object.defineProperty(navigator, 'onLine', { value: false, configurable: true });
		vi.stubGlobal('caches', {
			match: async () =>
				new Response('{}', { headers: { 'x-kamosu-kept': String(kept.getTime()) } }),
		});
	}

	afterEach(() => {
		vi.unstubAllGlobals();
		Reflect.deleteProperty(navigator, 'onLine');
		localStorage.clear();
	});

	it('says offline that a recipe the phone’s library does not hold is a copy, and from when', async () => {
		localStorage.setItem('kamosu.library', JSON.stringify({ held: ['another'] }));
		offlineWithKeptCopy();
		renderRecipe();
		expect(await screen.findByText(`Kept from ${kept.toLocaleDateString()}`)).toBeInTheDocument();
	});

	it('says nothing of the kind before the phone knows what its library holds', async () => {
		offlineWithKeptCopy();
		renderRecipe();
		await screen.findByText('This recipe, 2 versions');
		expect(screen.queryByText(/Kept from/)).not.toBeInTheDocument();
	});

	it('says nothing of the kind about a recipe the library holds', async () => {
		localStorage.setItem(
			'kamosu.library',
			JSON.stringify({ filledAt: Date.now(), held: ['mine'] }),
		);
		offlineWithKeptCopy();
		renderRecipe();
		await screen.findByText('This recipe, 2 versions');
		expect(screen.queryByText(/Kept from/)).not.toBeInTheDocument();
	});
});

/**
 * Deleting this recipe (#120).
 *
 * The three facts the confirmation owes a person are what these assert: which
 * recipe, that the cooking history stays, and the Share Link only when one is
 * live. The fourth thing tested is that nothing happens until it is confirmed.
 */
describe('deleting a recipe', () => {
	/**
	 * What `get_share_link` answers, whole. The stand-in checks every answer
	 * against the Catalogue's output schema before the screen sees it, so a
	 * partial one is refused — which is the guarantee working: an answer
	 * missing `public_address` would have made the screen's catch swallow the
	 * ask and the Share Link line silently never appear.
	 */
	const shareLink = (shared: boolean) => ({
		shared,
		share_id: shared ? 'sh_1' : null,
		url: shared ? 'https://kamosu.example/s/tk_1' : null,
		shared_by: shared ? 'Stéphane' : null,
		created_at: shared ? '2026-09-20T00:00:00Z' : null,
		public_address: 'https://kamosu.example',
	});

	/** Open the confirmation the way a thumb does, and let its ask settle. */
	async function openTheSheet(answers: Answers) {
		const rendered = renderRecipe(answers);
		const affordance = await screen.findByRole('button', { name: 'Delete this recipe' });
		await fireEvent.click(affordance);
		await tick();
		await tick();
		return rendered;
	}

	it('is not offered on a version somebody else writes (#131)', async () => {
		renderRecipe(
			forked({
				get_share_link: shareLink(false),
				get_recipe: { ...(forked().get_recipe as GetRecipeOutput), writes: false },
			}),
		);
		expect(await screen.findByRole('button', { name: /shopping list/i })).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Delete this recipe' })).toBeNull();
	});

	it('is set apart from the actions rather than standing among them', async () => {
		renderRecipe(forked({ get_share_link: shareLink(false) }));
		const affordance = await screen.findByRole('button', { name: 'Delete this recipe' });

		// Every action above it is a full-width row in one column. This one is
		// not shaped like them, which is the whole of why a thumb reaching for
		// `Add to shopping list` cannot land on it.
		const shopping = screen.getByRole('button', { name: /shopping list/i });
		expect(shopping.className).toContain('w-[calc(100%-2*var(--spacing-gutter))]');
		expect(affordance.className).not.toContain('w-[calc(100%-2*var(--spacing-gutter))]');
		expect(affordance.className).toContain('underline');
	});

	it('asks before it does anything, and asking alone deletes nothing', async () => {
		const { kamosu } = await openTheSheet(forked({ get_share_link: shareLink(false) }));

		expect(await screen.findByRole('dialog')).toBeInTheDocument();
		expect(kamosu.calls.some((call) => call.operation === 'delete_recipe')).toBe(false);
	});

	it('names the recipe and promises the cooking history, counted and impersonal', async () => {
		const answers = forked({
			get_share_link: shareLink(false),
		}) as Record<string, unknown>;
		const recipe = answers.get_recipe as { cooked: GetRecipeOutput['cooked'] };
		recipe.cooked = { count: 11, last_cooked_at: '2026-09-14T00:00:00Z', ratings: [] };

		await openTheSheet(answers as Answers);
		const sheet = await screen.findByRole('dialog');

		expect(sheet).toHaveTextContent('Delete Korean Fried Chicken?');
		expect(sheet).toHaveTextContent(/Its 11 cookings stay, with their ratings/i);
		expect(sheet).toHaveTextContent(/This cannot be undone/i);

		// **Never "you".** `cooked.count` is the HOUSEHOLD's tally across the
		// whole Lineage — housemates' cookings and the translation's included —
		// so a sheet saying "you cooked this 11 times" tells somebody who cooked
		// it twice a flat untruth. The Cooked section above is impersonal for
		// the same reason; this must not drift from it.
		expect(sheet).not.toHaveTextContent(/you cooked this/i);
	});

	it('does not invent a cooking history for a recipe nobody has cooked', async () => {
		await openTheSheet(forked({ get_share_link: shareLink(false) }));
		const sheet = await screen.findByRole('dialog');

		expect(sheet).toHaveTextContent(/Never cooked/i);
		expect(sheet).not.toHaveTextContent(/0 times/);
		expect(sheet).not.toHaveTextContent(/you have never/i);
	});

	it('warns about the Share Link only when one is live', async () => {
		await openTheSheet(forked({ get_share_link: shareLink(false) }));
		expect(await screen.findByRole('dialog')).not.toHaveTextContent(/link you shared/i);
	});

	it('warns about the Share Link when one is', async () => {
		await openTheSheet(forked({ get_share_link: shareLink(true) }));
		// Found rather than asserted in one go: the line appears when the ask
		// about the Share Link answers, which is after the sheet is drawn.
		expect(
			await screen.findByText(/link you shared for this recipe will stop working/i),
		).toBeInTheDocument();
	});

	it('says it could not tell, rather than staying quiet, when the ask fails', async () => {
		// Silence here would hide a link that really is about to stop working,
		// and an invented warning would frighten somebody about one they never
		// minted. Saying which of the two it is is the only honest third option.
		await openTheSheet(
			forked({ get_share_link: { refuse: 'internal', message: 'the server fell over' } }),
		);

		const sheet = await screen.findByRole('dialog');
		expect(
			await screen.findByText(/Couldn't check whether this recipe is shared/i),
		).toBeInTheDocument();
		expect(sheet).not.toHaveTextContent(
			/^The link you shared for this recipe will stop working\.$/,
		);
	});

	it('deletes nothing when the ask is turned down', async () => {
		const { kamosu } = await openTheSheet(forked({ get_share_link: shareLink(false) }));

		await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
		await tick();

		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
		expect(kamosu.calls.some((call) => call.operation === 'delete_recipe')).toBe(false);
	});

	it('deletes the Branch in the URL, and goes back to the shelf', async () => {
		went.mockClear();
		const { kamosu } = await openTheSheet(
			forked({
				get_share_link: shareLink(false),
				delete_recipe: { deleted: true },
			}),
		);

		await fireEvent.click(screen.getByRole('button', { name: 'Delete' }));
		await tick();
		await tick();

		const asked = kamosu.calls.find((call) => call.operation === 'delete_recipe');
		expect(asked?.input).toEqual({ branch_id: 'mine' });
		expect(went).toHaveBeenCalledWith('/recipes', { replaceState: true });
	});

	it('says what came back when Kamosu refuses, and stays where it is', async () => {
		went.mockClear();
		const { kamosu } = await openTheSheet(
			forked({
				get_share_link: shareLink(false),
				delete_recipe: { refuse: 'not_found', message: 'no such Branch' },
			}),
		);

		await fireEvent.click(screen.getByRole('button', { name: 'Delete' }));
		await tick();
		await tick();

		expect(await screen.findByRole('alert')).toHaveTextContent('no such Branch');
		expect(screen.getByRole('dialog')).toBeInTheDocument();
		expect(went).not.toHaveBeenCalledWith('/recipes', { replaceState: true });
		expect(kamosu.calls.some((call) => call.operation === 'delete_recipe')).toBe(true);
	});
});

// ---- the Language offer, at the seam it was dropped at (#106, ADR 0006) ---
//
// `Language.svelte`, `LanguageSheet` and `LanguageOffer` are tested on their
// own in `language.svelte.spec.ts`. What is tested HERE is the wiring those
// cannot see: `save_recipe_version` answers `language_offer`, `Writing` hands
// it up, and this page has to put it. That hand-off is the exact shape of the
// original defect — a field the Core answered and the interface dropped — so
// it is tested where the drop would happen.

/** What the recipe page asks for on the side and never waits on. */
const ON_THE_SIDE = {
	note_recipe_opened: { lineage_id: 'l_1', opened_at: '2026-09-22T00:00:00Z' },
	shopping_basis: {
		branch_id: 'mine',
		title: 'Korean Fried Chicken',
		written_yield: null,
		lines: [],
	},
	get_shopping_list: { chosen: [], rows: [] },
} as Answers;

/** A save that landed, and what Language its text read as. */
const savedReading = (language_offer: string | null, extra: Record<string, unknown> = {}) =>
	({
		save_recipe_version: {
			branch_id: 'mine',
			version_id: 'v_new',
			parent_version_id: 'v_mine',
			sequence: 2,
			copied: false,
			collapsed: false,
			language: 'en',
			language_offer,
			translates_version_id: null,
			...extra,
		},
	}) as Answers;

/** Open the writing screen and take the save all the way through its sheet. */
async function editAndSave() {
	await fireEvent.click(await screen.findByRole('button', { name: 'Edit this recipe' }));
	const act = /Save onto mine/;
	await fireEvent.click((await screen.findAllByRole('button', { name: act }))[0] as HTMLElement);
	const inSheet = await screen.findAllByRole('button', { name: act });
	await fireEvent.click(inSheet[inSheet.length - 1] as HTMLElement);
}

/** The Thread as it reads when this recipe has one Translation and no Divergence. */
const withATranslation = (extra: Answers = {}) =>
	forked({
		...ON_THE_SIDE,
		get_thread: {
			lineage_id: 'l_1',
			branches: [
				MINE_IN_THREAD,
				threadBranch('b_fr', {
					cookbook: MY_COOKBOOK,
					hand_id: 'h_mine',
					language: 'fr',
					head_version_id: 'v_fr',
					// A Translation renders a Version of this Branch, and its own
					// chain starts fresh, so it shares no Version with it.
					translation: {
						translates_version_id: 'v_mine',
						source_branch_id: 'mine',
						versions_behind: 0,
					},
				}),
			],
			versions: [],
			attempts: [],
		},
		...extra,
	});

describe('a recipe that has a Translation', () => {
	it('never asks for a Divergence against it, and reads perfectly well', async () => {
		// The defect #106 surfaced: a Lineage holding exactly two Branches used
		// to be paired unconditionally, so the one Lineage shape this ticket
		// creates — a recipe and its Translation — asked the Core to find a
		// shared Version between two chains that start apart. The Core refuses,
		// the refusal is an internal error, and this screen turned that into a
		// blank recipe. It was invisible only because no screen could make a
		// Translation before now.
		const { kamosu } = renderRecipe(withATranslation());

		expect(await screen.findByText('Korean Fried Chicken')).toBeInTheDocument();
		// Awaited, because the Translation arrives with the Thread rather than
		// with the recipe — and this is the assertion that proves the page did
		// not blank out on a refused Divergence.
		expect(await screen.findByRole('link', { name: 'Also in French.' })).toHaveAttribute(
			'href',
			'/recipes/b_fr',
		);
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('divergence');
	});

	it('does not call a Translation a crowd of Branches either', async () => {
		renderRecipe(withATranslation());

		expect(await screen.findByRole('link', { name: 'Also in French.' })).toBeInTheDocument();
		expect(screen.queryByText(/other versions of this recipe|too many/i)).not.toBeInTheDocument();
	});
});

describe('the Language offer on the recipe', () => {
	it('puts the offer where the save already speaks, once a save answers one', async () => {
		renderRecipe({ ...forked({ ...ON_THE_SIDE, ...savedReading('fr') }) });
		await screen.findByText('This recipe, 2 versions');

		await editAndSave();

		expect(await screen.findByText('Saved.')).toBeInTheDocument();
		expect(
			screen.getByText('This recipe is filed as English, but it reads as French.'),
		).toBeInTheDocument();
	});

	it('says nothing when the save’s text agreed with the Language the recipe carries', async () => {
		renderRecipe({ ...forked({ ...ON_THE_SIDE, ...savedReading(null) }) });
		await screen.findByText('This recipe, 2 versions');

		await editAndSave();

		expect(await screen.findByText('Saved.')).toBeInTheDocument();
		expect(screen.queryByText(/but it reads as/)).not.toBeInTheDocument();
	});
});

/**
 * Naming a version of the recipe from its page (#134, choice B): a
 * line among the actions opening a small sheet. A "version" here is a Branch,
 * the thing the strip at the top counts, and never one saved Version.
 */
describe('naming a version (#134)', () => {
	/** The Core's own sentence for clearing a second unnamed version, said as it came. */
	const SECOND_UNNAMED =
		'another version of this recipe in your Cookbook already goes without a name, so this one needs one';

	/**
	 * Your own recipe and a variation of it, both in your Cookbook, with the
	 * page open on one of them. Both parted from one root, so both are versions
	 * in the strip.
	 */
	function withAVariation(
		onPage: 'mine' | 'veg',
		names: { mine: string | null; veg: string | null },
	): Answers {
		const base = forked().get_recipe as GetRecipeOutput;
		return forked({
			get_recipe: { ...base, branch_id: onPage, name: names[onPage] },
			get_thread: {
				lineage_id: 'l_1',
				branches: [
					{ ...MINE_IN_THREAD, name: names.mine },
					threadBranch('veg', { cookbook: MY_COOKBOOK, name: names.veg }),
				],
				versions: [
					occurrenceOf('mine', 1, 'v_root', null),
					occurrenceOf('mine', 2, 'v_mine', 'v_root'),
					occurrenceOf('veg', 1, 'v_root', null),
					occurrenceOf('veg', 2, 'v_veg', 'v_root'),
				],
				attempts: [],
			},
		});
	}

	/** Tap the line among the actions, and wait for the sheet it opens. */
	async function openTheSheet(label: string) {
		await fireEvent.click(await screen.findByRole('button', { name: label }));
		return screen.findByRole('dialog', { name: label });
	}

	/** The name set under the title, which is a paragraph; the chip's is inside a link. */
	const nameUnderTitle = (name: string) =>
		screen.queryAllByText(name).find((element) => element.tagName === 'P');

	it('renames a named version, and both the line under the title and its chip say so', async () => {
		const { kamosu } = renderRecipe(
			withAVariation('veg', { mine: null, veg: 'Vegetarian' }),
			'veg',
		);
		const sheet = await openTheSheet('Rename this version');
		const field = within(sheet).getByLabelText('Name for this version');
		expect(field).toHaveValue('Vegetarian');
		expect(sheet).toHaveTextContent("The recipe and its History don't change.");

		// What the Core holds after the rename, read back rather than patched in.
		const after = withAVariation('veg', { mine: null, veg: 'Veggie' });
		kamosu.answer('rename_branch', { branch_id: 'veg', name: 'Veggie' });
		kamosu.answer('get_recipe', after.get_recipe);
		kamosu.answer('get_thread', after.get_thread);
		await fireEvent.input(field, { target: { value: '  Veggie ' } });
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Save' }));

		await vi.waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
		expect(kamosu.calls.find((call) => call.operation === 'rename_branch')?.input).toEqual({
			branch_id: 'veg',
			name: 'Veggie',
		});
		await vi.waitFor(() => expect(nameUnderTitle('Veggie')).toBeDefined());
		expect(await screen.findByRole('link', { name: /Veggie/ })).toHaveAttribute(
			'href',
			'/recipes/veg',
		);
		expect(nameUnderTitle('Vegetarian')).toBeUndefined();
	});

	it('names your own unnamed version, which then shows its name in both places', async () => {
		const { kamosu } = renderRecipe(withAVariation('mine', { mine: null, veg: 'Vegetarian' }));
		const sheet = await openTheSheet('Name this version');
		const field = within(sheet).getByLabelText('Name for this version');
		expect(field).toHaveValue('');
		// It has no name, so there is none to remove.
		expect(within(sheet).queryByRole('button', { name: 'Remove the name' })).toBeNull();

		const after = withAVariation('mine', { mine: 'Classic', veg: 'Vegetarian' });
		kamosu.answer('rename_branch', { branch_id: 'mine', name: 'Classic' });
		kamosu.answer('get_recipe', after.get_recipe);
		kamosu.answer('get_thread', after.get_thread);
		await fireEvent.input(field, { target: { value: 'Classic' } });
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Save' }));

		await vi.waitFor(() => expect(nameUnderTitle('Classic')).toBeDefined());
		expect(await screen.findByRole('link', { name: /Classic/ })).toHaveAttribute(
			'href',
			'/recipes/mine',
		);
		// Named now, so the line offers a rename rather than a name.
		expect(screen.getByRole('button', { name: 'Rename this version' })).toBeInTheDocument();
	});

	it('clears a name where another version of yours has one', async () => {
		const { kamosu } = renderRecipe(
			withAVariation('veg', { mine: 'Classic', veg: 'Vegetarian' }),
			'veg',
		);
		const sheet = await openTheSheet('Rename this version');

		const after = withAVariation('veg', { mine: 'Classic', veg: null });
		kamosu.answer('rename_branch', { branch_id: 'veg', name: null });
		kamosu.answer('get_recipe', after.get_recipe);
		kamosu.answer('get_thread', after.get_thread);
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Remove the name' }));

		await vi.waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
		expect(kamosu.calls.find((call) => call.operation === 'rename_branch')?.input).toEqual({
			branch_id: 'veg',
			name: null,
		});
		await vi.waitFor(() => expect(nameUnderTitle('Vegetarian')).toBeUndefined());
		expect(await screen.findByRole('link', { name: /Yours/ })).toHaveAttribute(
			'href',
			'/recipes/veg',
		);
	});

	it('saves no name of only spaces, which would quietly clear it', async () => {
		const { kamosu } = renderRecipe(
			withAVariation('veg', { mine: null, veg: 'Vegetarian' }),
			'veg',
		);
		const sheet = await openTheSheet('Rename this version');
		const field = within(sheet).getByLabelText('Name for this version');
		await fireEvent.input(field, { target: { value: '   ' } });

		await fireEvent.submit(field.closest('form')!);

		expect(kamosu.calls.some((call) => call.operation === 'rename_branch')).toBe(false);
		expect(screen.getByRole('dialog', { name: 'Rename this version' })).toBe(sheet);
		expect(field).toHaveValue('');
	});

	it('says the Core’s refusal as it came, keeps what was typed, and changes no name', async () => {
		const { kamosu } = renderRecipe(
			withAVariation('veg', { mine: null, veg: 'Vegetarian' }),
			'veg',
		);
		const sheet = await openTheSheet('Rename this version');
		const field = within(sheet).getByLabelText('Name for this version');
		await fireEvent.input(field, { target: { value: 'Veg' } });
		kamosu.answer('rename_branch', { refuse: 'bad_request', message: SECOND_UNNAMED });
		const reads = kamosu.calls.filter((call) => call.operation === 'get_recipe').length;

		await fireEvent.click(within(sheet).getByRole('button', { name: 'Remove the name' }));

		expect(await within(sheet).findByRole('alert')).toHaveTextContent(SECOND_UNNAMED);
		expect(screen.getByRole('dialog', { name: 'Rename this version' })).toBe(sheet);
		expect(field).toHaveValue('Veg');
		expect(nameUnderTitle('Vegetarian')).toBeDefined();
		expect(kamosu.calls.filter((call) => call.operation === 'get_recipe')).toHaveLength(reads);
	});

	it('says plainly that the name could not be saved when Kamosu was not reached', async () => {
		const { kamosu } = renderRecipe(
			withAVariation('veg', { mine: null, veg: 'Vegetarian' }),
			'veg',
		);
		const sheet = await openTheSheet('Rename this version');
		const field = within(sheet).getByLabelText('Name for this version');
		await fireEvent.input(field, { target: { value: 'Veggie' } });
		kamosu.answer('rename_branch', () => {
			throw new OperationError('rename_branch', 'internal', 'Kamosu could not be reached.', {
				reached: false,
			});
		});

		await fireEvent.click(within(sheet).getByRole('button', { name: 'Save' }));

		expect(await within(sheet).findByRole('alert')).toHaveTextContent(
			'The name could not be saved. Try again.',
		);
		expect(field).toHaveValue('Veggie');
	});

	it('offers no rename on a version you do not write', async () => {
		renderRecipe(onTheirs(), 'theirs');
		await compared();

		expect(screen.queryByRole('button', { name: 'Rename this version' })).toBeNull();
		expect(screen.queryByRole('button', { name: 'Name this version' })).toBeNull();
	});

	it('offers a name only once there is another version to tell it from', async () => {
		const alone = (name: string | null) =>
			forked({
				get_recipe: { ...(forked().get_recipe as GetRecipeOutput), name },
				get_thread: {
					lineage_id: 'l_1',
					branches: [{ ...MINE_IN_THREAD, name }],
					versions: [occurrenceOf('mine', 1, 'v_mine', null)],
					attempts: [],
				},
			});

		renderRecipe(alone(null));
		// The count under History comes from the same read of the Thread that
		// says how many versions there are, so the page knows by then.
		expect(await screen.findByText(/^Saved once/)).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Name this version' })).toBeNull();
		cleanup();

		// A version with a name can always lose or change it, alone or not.
		renderRecipe(alone('Classic'));
		expect(await screen.findByRole('button', { name: 'Rename this version' })).toBeInTheDocument();
	});
});

/**
 * A recipe's Source line, where its link can be tapped (#152,
 * choice A). The line is set on the photograph when there is one and on paper
 * under a Cover when there is not (#81), and the same holds in both places.
 */
describe('the Source line', () => {
	const withSource = (source: { text: string; link: string | null }, main_photo: string | null) => {
		const recipe = forked().get_recipe as GetRecipeOutput;
		const version = recipe.versions[0]!;
		return forked({
			get_recipe: {
				...recipe,
				versions: [{ ...version, content: { ...version.content, source, main_photo } }],
			},
		});
	};

	const LINK = 'https://www.allrecipes.com/recipe/143809/best-steak-marinade/';

	for (const [layout, photo] of [
		['a photograph', 'p_steak'],
		['a Cover', null],
	] as const) {
		it(`makes the line itself the link on ${layout}, opening outside Kamosu`, async () => {
			renderRecipe(withSource({ text: 'Allrecipes', link: LINK }, photo));
			const link = await screen.findByRole('link', { name: /From Allrecipes/ });
			expect(link).toHaveAttribute('href', LINK);
			expect(link).toHaveAttribute('target', '_blank');
			expect(link).toHaveAttribute('rel', 'noopener noreferrer');
			// Not colour alone: on the wash it keeps the line's own colour.
			expect(link.className).toContain('underline');
			// Set apart from the words, and never wrapped onto a line of its own.
			expect(link.textContent).toContain('From Allrecipes\u00a0↗');
			expect(link).toHaveAccessibleName('From Allrecipes, opens the original page');
		});

		it(`draws a Source with no link as the plain line on ${layout}`, async () => {
			renderRecipe(withSource({ text: 'Allrecipes', link: null }, photo));
			const line = await screen.findByText('From Allrecipes');
			expect(line.tagName).toBe('P');
			expect(line.querySelector('a')).toBeNull();
			expect(screen.queryByRole('link', { name: /Allrecipes/ })).not.toBeInTheDocument();
		});

		it(`never offers a link that is not a web address on ${layout}`, async () => {
			renderRecipe(withSource({ text: 'Allrecipes', link: 'javascript:alert(1)' }, photo));
			const line = await screen.findByText('From Allrecipes');
			expect(line.querySelector('a')).toBeNull();
			expect(document.querySelector('a[href^="javascript"]')).toBeNull();
		});
	}
});

describe('two columns where the window is roomy (#198)', () => {
	it('puts the Ingredients in one column, the Method in the other, and the rest under both', async () => {
		renderRecipe();
		const line = await screen.findByText('1 cup potato starch (or corn starch)');
		const step = screen.getByText('Deep fry at 190 C until crisp.');

		const columns = line.closest('.recipe-columns')!;
		expect(columns).not.toBeNull();
		expect(columns.children).toHaveLength(2);
		expect(columns.children[0]).toContainElement(line);
		expect(columns.children[0]).toContainElement(
			screen.getByRole('heading', { name: 'Ingredients' }),
		);
		expect(columns.children[1]).toContainElement(step);
		expect(columns.children[1]).toContainElement(screen.getByRole('heading', { name: 'Method' }));

		// The part that stays in view is a box inside the left column, never
		// the column. As the column it stayed for the whole page and rode over
		// everything under the Method, which was found on the iPad.
		const stays = line.closest('.stays-in-view')!;
		expect(stays.parentElement).toBe(columns.children[0]);
		expect(stays).not.toContainElement(step);

		// What leads the page is above both columns, and what follows is under
		// both: neither is squeezed into one of them.
		for (const outside of [
			screen.getByRole('heading', { name: 'Tags' }),
			screen.getByRole('heading', { name: 'Cooked' }),
			screen.getByRole('button', { name: 'Edit this recipe' }),
			screen.getByRole('link', { name: 'Cook this' }),
		]) {
			expect(columns).not.toContainElement(outside);
		}
	});

	it('keeps a recipe read against another Branch in one column (#192)', async () => {
		renderRecipe(onTheirs(), 'theirs');
		await compared();

		expect(document.querySelector('.recipe-columns')).toBeNull();
		expect(document.querySelector('.stays-in-view')).toBeNull();
		expect(document.querySelector('.recipe-page')).toBeNull();
	});
});
