/**
 * The screen-seam test: the recipe, and the Divergence, against a stand-in
 * Kamosu. Every answer below is checked against the Catalogue before the screen
 * sees it — a field renamed in `src/catalogue.rs` fails this test in the same
 * commit.
 *
 * What is being tested is ADR 0014's shape: two whole recipes with a switch
 * between them, never a difference. If any assertion here starts describing a
 * comparison view, the screen has gone wrong.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import { tick } from 'svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import type { GetRecipeOutput } from '$lib/api/catalogue';
import RecipeTestHarness from './RecipeTestHarness.svelte';

// Deleting ends by going back to the shelf, because there is nothing left to
// stand on. Where that goes is the router's business, not this screen's.
const went = vi.hoisted(() => vi.fn());
vi.mock('$app/navigation', () => ({ goto: went }));

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

const branch = (
	branch_id: string,
	kitchen_name: string,
	ingredients: { kind: 'section' | 'ingredient'; text: string }[],
	steps: { kind: 'section' | 'step'; text: string; photo: string | null }[],
) => ({
	branch_id,
	kitchen_id: `k_${branch_id}`,
	kitchen_name,
	hand_id: `h_${branch_id}`,
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
		mine: branch('mine', 'Maison Batterman', MY_INGREDIENTS, MY_STEPS),
		theirs: branch('theirs', 'Chez Marc', THEIR_INGREDIENTS, THEIR_STEPS),
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
	name: null,
	change_note: null,
	created_at: '2026-08-09T00:00:00Z',
	translates_version_id: null,
	language: 'en',
});

function forked(extra: Answers = {}) {
	return {
		get_recipe: {
			branch_id: 'mine',
			lineage_id: 'l_1',
			kitchen_id: 'k_mine',
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
			branches: [
				{
					branch_id: 'mine',
					kitchen_id: 'k_mine',
					hand_id: 'h_mine',
					language: 'en',
					head_version_id: 'v_mine',
					translation: null,
				},
				{
					branch_id: 'theirs',
					kitchen_id: 'k_theirs',
					hand_id: 'h_theirs',
					language: 'en',
					head_version_id: 'v_theirs',
					translation: null,
				},
			],
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

function renderRecipe(answers: Answers = forked()) {
	const kamosu = standIn(answers);
	const { rerender } = render(RecipeTestHarness, {
		props: { client: kamosu.client, branchId: 'mine' },
	});
	/** Walk to another recipe, the way tapping through to one does. */
	const goTo = (branchId: string) => rerender({ client: kamosu.client, branchId });
	return { kamosu, goTo };
}

describe('a Divergence', () => {
	it('is two whole recipes with a switch between them, never a difference view', async () => {
		renderRecipe();

		// You are standing in your own Kitchen, reading your own whole recipe.
		expect(await screen.findByText('Maison Batterman')).toBeInTheDocument();
		expect(screen.getByText('1 cup potato starch (or corn starch)')).toBeInTheDocument();
		expect(screen.getByText('¼ cup honey')).toBeInTheDocument();

		// Crossing the threshold puts you in his recipe, whole and cookable —
		// not a comparison, not a column beside yours.
		await fireEvent.click(screen.getByRole('button', { name: /Cross to Chez Marc/ }));
		expect(await screen.findByText('¾ cup potato starch')).toBeInTheDocument();
		expect(screen.getByText('Air fry at 200 C for 18 minutes.')).toBeInTheDocument();
		// Your own Kitchen is now the way back, on the other half of the threshold.
		expect(screen.getByRole('button', { name: /Cross to Maison Batterman/ })).toBeInTheDocument();
	});

	it('carries your tags in your own Branch and none once you have crossed', async () => {
		// A Divergence does not mark Tags (ADR 0019), and the `divergence`
		// Operation therefore carries none — so there is nothing of his to show,
		// and showing yours beside his recipe would say something false about
		// whose filing it is (#104).
		const spicy = {
			id: 't_spicy',
			kitchen_id: 'k_mine',
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

		// The Threshold arrives with the `divergence` answer, which lands after
		// the recipe does — so it is waited for rather than assumed.
		await fireEvent.click(await screen.findByRole('button', { name: /Cross to Chez Marc/ }));
		await screen.findByText('¾ cup potato starch');

		// His recipe, so no Tags row at all — not yours, and not an empty one.
		expect(screen.queryByRole('link', { name: /spicy/ })).not.toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Add a tag' })).not.toBeInTheDocument();
	});

	it('shows a line only one side has as a Ghost, named, in its own position', async () => {
		renderRecipe();

		// Marc's gochugaru is a Ghost on my recipe, and it says whose it is —
		// a struck-through line with no words beside it would read like
		// something crossed off a shopping list.
		const ghost = await screen.findByText('1 tsp gochugaru');
		expect(ghost).toBeInTheDocument();
		expect(screen.getAllByText(/Chez Marc’s — you haven’t got it/i).length).toBeGreaterThan(0);

		// It sits where it sits over there: after the honey, not at the end.
		const texts = Array.from(document.querySelectorAll('ul li')).map((li) => li.textContent ?? '');
		const honey = texts.findIndex((t) => t.includes('¼ cup honey'));
		const gochugaru = texts.findIndex((t) => t.includes('1 tsp gochugaru'));
		expect(gochugaru).toBeGreaterThan(honey);
	});

	it('shows an uncertain Pairing as both lines, unjoined, with nothing labelled a guess', async () => {
		renderRecipe();

		// My rice vinegar and his gochugaru both arrived after we parted and
		// land in the same part of the sauce. Both are on the page, separately.
		expect(await screen.findByText('1 tsp gochugaru')).toBeInTheDocument();
		expect(screen.getByText('1 Tbsp rice vinegar')).toBeInTheDocument();

		// No hedge of any kind, anywhere on the screen.
		const page = document.body.textContent ?? '';
		for (const hedge of ['probably', 'likely', 'uncertain', 'maybe', 'confidence', '%']) {
			expect(page.toLowerCase()).not.toContain(hedge);
		}
	});

	it('puts the marking away and leaves simply the recipe', async () => {
		renderRecipe();
		await screen.findByText('Maison Batterman');

		await fireEvent.click(screen.getByRole('button', { name: /Hide them/i }));

		// Every Ghost goes: a Ghost is a line of the OTHER recipe, and with the
		// marking away it has no business on the page at all.
		expect(screen.queryByText('1 tsp gochugaru')).not.toBeInTheDocument();
		expect(screen.queryByText(/you haven’t got it/i)).not.toBeInTheDocument();
		// What is left is the list you would shop from.
		expect(screen.getByText('1 cup potato starch (or corn starch)')).toBeInTheDocument();
		expect(screen.getByText('¼ cup brown sugar')).toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: /Show them/i }));
		expect(await screen.findByText('1 tsp gochugaru')).toBeInTheDocument();
	});

	it('carries a line across unsaved, and saving writes an ordinary Version with prose', async () => {
		const { kamosu } = renderRecipe(
			forked({
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
			}),
		);
		await screen.findByText('Maison Batterman');

		// The offer lives where you would want a line of his recipe: in it.
		await fireEvent.click(screen.getByRole('button', { name: /Cross to Chez Marc/ }));
		await fireEvent.click(await screen.findByText('¾ cup potato starch'));
		await fireEvent.click(screen.getByRole('button', { name: /Write this into mine/i }));
		// Fold the row shut so only the line itself, at rest, is on the page.
		await fireEvent.click(screen.getAllByText('¾ cup potato starch')[0]);

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
		expect(saved).toBeDefined();
		const input = saved?.input as {
			ingredients: { text: string }[];
			change_note: string;
		};
		// A whole recipe, with his line written in where the Pairing puts it.
		expect(input.ingredients.map((item) => item.text)).toEqual([
			'¾ cup potato starch',
			'¼ cup honey',
			'¼ cup brown sugar',
			'1 Tbsp rice vinegar',
		]);
		expect(input.change_note).toBe('Took ¾ cup potato starch from Chez Marc.');
	});

	it('carries the nutrition figure through a save rather than erasing it', async () => {
		// `save_recipe_version` replaces the whole recipe, so a field this
		// screen forgets to send is a field the save deletes. The nutrition
		// figure is one of the recipe's own words (#72), and taking one line
		// across from another Branch is not a reason to lose it.
		const answers = forked({
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
		});
		const d = (answers as Record<string, unknown>).divergence as ReturnType<typeof divergence>;
		d.mine.content.nutrition = { calories: 308, basis: 'per_serving' };
		const { kamosu } = renderRecipe(answers);
		await screen.findByText('Maison Batterman');

		await fireEvent.click(screen.getByRole('button', { name: /Cross to Chez Marc/ }));
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

	it('marks a single value the two Branches do not agree on', async () => {
		// ADR 0019: "The marking covers the whole recipe, not only the two lists.
		// Title, Yield, Prep Time, Cook Time and Source are single values."
		const answers = forked();
		const d = (answers as Record<string, unknown>).divergence as ReturnType<typeof divergence>;
		d.fields.title = { same: false, mine: 'Korean Fried Chicken', theirs: 'KFC, air fryer' };
		d.fields.cook_time_minutes = { same: false, mine: 30, theirs: 22 };
		renderRecipe(answers);

		expect(await screen.findByText(/Chez Marc has KFC, air fryer/i)).toBeInTheDocument();
		expect(screen.getByText(/Chez Marc has 22/i)).toBeInTheDocument();
	});

	/**
	 * #72 gave `divergence` a `fields.nutrition` and nothing read it, so two
	 * Branches disagreeing about the figure said nothing at all. The mark goes
	 * with the figure, at the foot of the Ingredients, rather than with the
	 * marks at the top: the figure is there, so what the other Kitchen says
	 * about it belongs there too (#84).
	 */
	it('marks the figure the two Branches do not agree on', async () => {
		const answers = forked();
		const d = (answers as Record<string, unknown>).divergence as ReturnType<typeof divergence>;
		d.mine.content.nutrition = { calories: 308, basis: 'per_serving' };
		d.fields.nutrition = {
			same: false,
			mine: { calories: 308, basis: 'per_serving' },
			theirs: { calories: 420, basis: 'per_serving' },
		};
		renderRecipe(answers);

		const mark = await screen.findByText(/Chez Marc has 420 kcal a serving/i);
		expect(mark).toBeInTheDocument();
		// Beside the figure it is about, not up with the Title and the Yield.
		expect(
			mark.compareDocumentPosition(screen.getByText('308 kcal a serving')) &
				Node.DOCUMENT_POSITION_PRECEDING,
		).toBeTruthy();
	});

	it('says so when the other Branch carries no figure where yours does', async () => {
		const answers = forked();
		const d = (answers as Record<string, unknown>).divergence as ReturnType<typeof divergence>;
		d.mine.content.nutrition = { calories: 308, basis: 'per_serving' };
		d.fields.nutrition = {
			same: false,
			mine: { calories: 308, basis: 'per_serving' },
			theirs: null,
		};
		renderRecipe(answers);

		expect(await screen.findByText(/Chez Marc has nothing/i)).toBeInTheDocument();
	});

	it('writes a carried line into your recipe in place, showing what it replaced', async () => {
		renderRecipe();
		await screen.findByText('Maison Batterman');

		// ADR 0014: taking a line "writes it into your recipe and leaves it
		// unsaved, marked in place with what it replaced, until you save".
		await fireEvent.click(await screen.findByText('1 cup potato starch (or corn starch)'));
		await fireEvent.click(screen.getByRole('button', { name: /Write this into mine/i }));
		// Fold the row shut so only the line itself, at rest, is on the page.
		await fireEvent.click(screen.getAllByText('¾ cup potato starch')[0]);

		// At rest, in your own recipe, the line now reads as his words, not yours.
		expect(screen.getByText('¾ cup potato starch')).toBeInTheDocument();
		expect(screen.queryByText('1 cup potato starch (or corn starch)')).not.toBeInTheDocument();
		expect(
			screen.getByText(/replacing 1 cup potato starch \(or corn starch\)/i),
		).toBeInTheDocument();
		expect(screen.getByText(/sitting on your recipe. Not saved/i)).toBeInTheDocument();
	});

	it('says so rather than hiding the others when a third Branch exists', async () => {
		renderRecipe(
			forked({
				get_thread: {
					lineage_id: 'l_1',
					branches: ['mine', 'theirs', 'camille'].map((id) => ({
						branch_id: id,
						kitchen_id: `k_${id}`,
						hand_id: `h_${id}`,
						language: 'en',
						head_version_id: `v_${id}`,
						translation: null,
					})),
					// All three parted from the same root, which is what makes
					// them a crowd rather than unrelated recipes.
					versions: ['mine', 'theirs', 'camille'].flatMap((id) => [
						occurrenceOf(id, 1, 'v_root', null),
						occurrenceOf(id, 2, `v_${id}`, 'v_root'),
					]),
					attempts: [],
				},
			}),
		);

		// ADR 0014: "Two Branches, not five… the switch is the part that will not
		// survive it unchanged." Undesigned is fine; silent is not.
		expect(await screen.findByText(/has 3 Branches on this Kamosu/i)).toBeInTheDocument();
		expect(screen.queryByText('1 tsp gochugaru')).not.toBeInTheDocument();
	});

	it('names the friend’s Kitchen on a Ghost from either side', async () => {
		renderRecipe();
		await screen.findByText('Maison Batterman');

		// Standing in your own recipe: his line, which you haven't got.
		expect(screen.getAllByText(/Chez Marc’s — you haven’t got it/i).length).toBeGreaterThan(0);

		await fireEvent.click(screen.getByRole('button', { name: /Cross to Chez Marc/ }));
		await screen.findByText('¾ cup potato starch');

		// Standing in his: your brown sugar, which HE took out. Crossing over
		// lets you read his recipe; it does not make you him, so the Kitchen
		// named is still his — naming whichever Kitchen you are not in put
		// "Maison Batterman took it out" on your own line.
		expect(screen.getAllByText(/yours — Chez Marc took it out/i).length).toBeGreaterThan(0);
		expect(screen.queryByText(/Maison Batterman took it out/i)).not.toBeInTheDocument();
		// And his own changed line reads as not yours, not as "not Chez Marc's".
		expect(screen.getAllByText(/^not yours$/i).length).toBeGreaterThan(0);
	});

	it('offers no way to take a whole Branch at once', async () => {
		renderRecipe();
		await screen.findByText('Maison Batterman');
		await fireEvent.click(screen.getByRole('button', { name: /Cross to Chez Marc/ }));

		// The absence is the decision (ADR 0014): taking every line one at a
		// time is a person making a recipe; one button doing it is a merge.
		for (const label of [/take all/i, /take everything/i, /accept all/i, /merge/i]) {
			expect(screen.queryByRole('button', { name: label })).not.toBeInTheDocument();
		}
	});

	it('reads a recipe on its own when there is no second Branch', async () => {
		renderRecipe(
			forked({
				get_thread: {
					lineage_id: 'l_1',
					branches: [
						{
							branch_id: 'mine',
							kitchen_id: 'k_mine',
							hand_id: 'h_mine',
							language: 'en',
							head_version_id: 'v_mine',
							translation: null,
						},
					],
					versions: [],
					attempts: [],
				},
			}),
		);

		expect(await screen.findByText('1 cup potato starch (or corn starch)')).toBeInTheDocument();
		// No threshold, because there is nowhere to cross to.
		expect(screen.queryByText(/You are in/i)).not.toBeInTheDocument();
		expect(screen.queryByText('1 tsp gochugaru')).not.toBeInTheDocument();
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
							person_id: 'p_aurelien',
							name: 'Aurélien',
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

		expect(await screen.findByText('Aurélien')).toBeInTheDocument();
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
 * The recipe screen itself (#81): the written Ingredient Line, its Reading,
 * the Sections, correcting a Reading in place, and a recipe wearing none of
 * the optional things.
 *
 * The layout was chosen by Aurélien on 29 August 2026 and the reasoning is on
 * the issue. What is tested here is not how it looks but what ADR 0002 makes
 * true of it: the written Line is the truth of the ingredient, a Reading is
 * subordinate to it, a Reading may be absent, and nothing on the page marks
 * which is which.
 */
describe('the recipe screen', () => {
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
			kitchen_id: 'k_mine',
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
				branches: [
					{
						branch_id: 'mine',
						kitchen_id: 'k_mine',
						hand_id: 'h_mine',
						language: 'en',
						head_version_id: 'v_mine',
						translation: null,
					},
				],
				versions: [],
				attempts: [],
			},
		};
	}

	/**
	 * THE NUTRITION FIGURE CLOSES THE INGREDIENTS (#84). Aurélien chose that
	 * placement on 21 September 2026 against four treatments drawn on both
	 * surfaces: a fourth cell in the meta strip, a line directly under the
	 * strip, this, and a place beside the Source. The list is where what goes
	 * into the dish is already the subject, and the strip keeps the three cells
	 * #81 gave it.
	 *
	 * The figure always says what it counts. 308 on its own says nothing, and
	 * the two bases do not convert into each other without a weight the recipe
	 * does not carry (CONTEXT.md, "Nutrition").
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
		steps: [null, null, null, 'about 375 °F'],
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

	it('offers an oven temperature beside a Step and never inside its sentence', async () => {
		renderRecipe(solo({}, undefined, CONVERTED));

		// The ladder's answer is an addition beside the sentence: a Step's truth
		// is its text, and nothing here rewrites it (ADR 0016).
		const step = await screen.findByText('Deep fry at 190 C until crisp.');
		const converted = screen.getByText('about 375 °F');
		expect(step.closest('li')).toBe(converted.closest('li'));
		expect(converted).toHaveClass('text-read');
		expect(step.textContent).toBe('Deep fry at 190 C until crisp.');

		// The step with no temperature is offered nothing at all.
		const brine = screen.getByText('Brine the chicken overnight.').closest('li') as HTMLElement;
		expect(brine.querySelector('.text-read')).toBeNull();
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
			list_kitchens: {
				kitchens: [
					{
						id: 'k_mine',
						name: 'Maison Batterman',
						nickname: null,
						is_home: true,
						hand_id: 'h_mine',
						members: [],
					},
				],
			},
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
		expect(screen.queryByText(/min prep/i)).not.toBeInTheDocument();
		expect(screen.queryByText(/min cook/i)).not.toBeInTheDocument();
		expect(screen.queryByText(/^From /i)).not.toBeInTheDocument();
	});

	// ── Components (#50, ADR 0008) ───────────────────────────────────────────
	//
	// A Component is an ordinary Ingredient Line whose Reading names a Recipe
	// rather than a Food. Everything the screen shows about one — which recipe,
	// how much of it, and the three sentences for when there is nothing to
	// unfold — is worded in the Core, so what these tests hold is the LAYOUT
	// Aurélien chose on 3 September 2026: B, the annexe.

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
 * to the recipe, not to the marking — but their Branch is theirs (ADR 0007),
 * so it stops at the threshold.
 */
describe('correcting a Reading beside a Divergence', () => {
	it('offers no corrector while you are standing in the other Kitchen’s recipe', async () => {
		renderRecipe();
		await screen.findByText('Maison Batterman');

		// Your own list offers it on every unmarked line.
		expect(screen.getByText('¼ cup honey').closest('button')).toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: /Cross to Chez Marc/ }));
		await screen.findByText('¾ cup potato starch');

		// His is a recipe you read, not one you keep. Nothing here writes into it.
		expect(screen.getByText('¼ cup honey').closest('button')).toBeNull();
		expect(screen.queryByRole('button', { name: /Fix what Kamosu read/i })).not.toBeInTheDocument();
	});

	it('reaches a marked line through its own panel, not through its tap', async () => {
		const { kamosu } = renderRecipe(
			forked({
				set_reading: {
					line_index: 0,
					reading: { amount: '¾', unit: 'cup', target: 'potato starch', lineage_id: null },
					measured: null,
				},
			}),
		);
		await screen.findByText('Maison Batterman');

		// A marked row's tap already means "show me the other side", so the
		// corrector is a second target inside the panel that opens.
		await fireEvent.click(screen.getByText('1 cup potato starch (or corn starch)'));
		await fireEvent.click(screen.getByRole('button', { name: /Fix what Kamosu read/i }));
		await fireEvent.input(screen.getByLabelText(/What it is/i), {
			target: { value: 'potato starch' },
		});
		await fireEvent.click(screen.getByRole('button', { name: /Save the Reading/i }));

		expect(kamosu.calls.find((call) => call.operation === 'set_reading')?.input).toMatchObject({
			branch_id: 'mine',
			line_index: 0,
			target: 'potato starch',
		});
		// Still not an edit to the recipe, marked or not.
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('save_recipe_version');
	});

	it('closes an open corrector when the ground under it moves', async () => {
		renderRecipe();
		await screen.findByText('Maison Batterman');

		await fireEvent.click(screen.getByText('¼ cup honey'));
		expect(screen.getByText('What Kamosu read')).toBeInTheDocument();

		// Both gestures rebuild the list from a different set of rows, so an
		// open panel would reopen on whatever line lands at that index.
		await fireEvent.click(screen.getByRole('button', { name: /Hide them/i }));
		expect(screen.queryByText('What Kamosu read')).not.toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: /Show them/i }));
		await fireEvent.click(await screen.findByText('¼ cup honey'));
		expect(screen.getByText('What Kamosu read')).toBeInTheDocument();
		await fireEvent.click(screen.getByRole('button', { name: /Cross to Chez Marc/ }));
		expect(screen.queryByText('What Kamosu read')).not.toBeInTheDocument();
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
		await screen.findByText('Maison Batterman');

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
		await screen.findByText('Maison Batterman');

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
		await screen.findByText('Maison Batterman');

		await fireEvent.click(await screen.findByRole('button', { name: /Print a sheet/i }));
		expect(await screen.findByRole('alert')).toHaveTextContent(/could not be made/);
		expect(tab.close).toHaveBeenCalled();
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

	it('says offline that a recipe the Kitchen does not hold is a copy, and from when', async () => {
		localStorage.setItem('kamosu.library', JSON.stringify({ held: ['another'] }));
		offlineWithKeptCopy();
		renderRecipe();
		expect(await screen.findByText(`Kept from ${kept.toLocaleDateString()}`)).toBeInTheDocument();
	});

	it('says nothing of the kind before the phone knows what the Kitchen holds', async () => {
		offlineWithKeptCopy();
		renderRecipe();
		await screen.findByText('Maison Batterman');
		expect(screen.queryByText(/Kept from/)).not.toBeInTheDocument();
	});

	it('says nothing of the kind about a recipe the Kitchen holds', async () => {
		localStorage.setItem(
			'kamosu.library',
			JSON.stringify({ filledAt: Date.now(), held: ['mine'] }),
		);
		offlineWithKeptCopy();
		renderRecipe();
		await screen.findByText('Maison Batterman');
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
		shared_by: shared ? 'Aurélien' : null,
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
		expect(sheet).toHaveTextContent(/all 11 times this was cooked keep their ratings/i);
		expect(sheet).toHaveTextContent(/nothing brings it back/i);

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

		expect(sheet).toHaveTextContent(/never been cooked/i);
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
			await screen.findByText(/could not check whether this recipe is shared/i),
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

/**
 * What the recipe page asks for on the side and never waits on, plus the one
 * Kitchen the writing screen needs to know whether a save forks.
 */
const MY_KITCHENS = {
	note_recipe_opened: { lineage_id: 'l_1', opened_at: '2026-09-22T00:00:00Z' },
	shopping_basis: {
		branch_id: 'mine',
		title: 'Korean Fried Chicken',
		written_yield: null,
		lines: [],
	},
	get_shopping_list: { chosen: [], rows: [] },
	list_kitchens: {
		kitchens: [
			{
				id: 'k_mine',
				name: 'Maison Batterman',
				nickname: null,
				is_home: true,
				hand_id: 'h_mine',
				members: [],
			},
		],
	},
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
		...MY_KITCHENS,
		get_thread: {
			lineage_id: 'l_1',
			branches: [
				{
					branch_id: 'mine',
					kitchen_id: 'k_mine',
					hand_id: 'h_mine',
					language: 'en',
					head_version_id: 'v_mine',
					translation: null,
				},
				{
					branch_id: 'b_fr',
					kitchen_id: 'k_mine',
					hand_id: 'h_mine',
					language: 'fr',
					head_version_id: 'v_fr',
					// A Translation renders a Version of this Branch, and its own
					// chain starts fresh — so it shares no Version with it.
					translation: {
						translates_version_id: 'v_mine',
						source_branch_id: 'mine',
						versions_behind: 0,
					},
				},
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
		renderRecipe({ ...forked({ ...MY_KITCHENS, ...savedReading('fr') }) });
		await screen.findByText('Maison Batterman');

		await editAndSave();

		expect(await screen.findByText('Saved.')).toBeInTheDocument();
		expect(
			screen.getByText('This recipe is filed as English, but it reads as French.'),
		).toBeInTheDocument();
	});

	it('says nothing when the save’s text agreed with the Language the recipe carries', async () => {
		renderRecipe({ ...forked({ ...MY_KITCHENS, ...savedReading(null) }) });
		await screen.findByText('Maison Batterman');

		await editAndSave();

		expect(await screen.findByText('Saved.')).toBeInTheDocument();
		expect(screen.queryByText(/but it reads as/)).not.toBeInTheDocument();
	});
});
