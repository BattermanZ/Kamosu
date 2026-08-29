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

import { describe, expect, it } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import RecipeTestHarness from './RecipeTestHarness.svelte';

const line = (text: string, index: number) => ({
	kind: 'ingredient',
	text,
	index
});
const step = (text: string, index: number) => ({ kind: 'step', text, index });

const branch = (
	branch_id: string,
	kitchen_name: string,
	ingredients: { kind: 'section' | 'ingredient'; text: string }[],
	steps: { kind: 'section' | 'step'; text: string; photo: string | null }[]
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
		source: { text: 'mykoreankitchen.com', link: null },
		ingredients,
		steps
	},
	readings: ingredients.map(() => null)
});

const MY_INGREDIENTS = [
	{ kind: 'ingredient' as const, text: '1 cup potato starch (or corn starch)' },
	{ kind: 'ingredient' as const, text: '¼ cup honey' },
	{ kind: 'ingredient' as const, text: '¼ cup brown sugar' },
	{ kind: 'ingredient' as const, text: '1 Tbsp rice vinegar' }
];
const THEIR_INGREDIENTS = [
	{ kind: 'ingredient' as const, text: '¾ cup potato starch' },
	{ kind: 'ingredient' as const, text: '¼ cup honey' },
	{ kind: 'ingredient' as const, text: '1 tsp gochugaru' }
];
const MY_STEPS = [
	{
		kind: 'step' as const,
		text: 'Deep fry at 190 C until crisp.',
		photo: null
	}
];
const THEIR_STEPS = [
	{
		kind: 'step' as const,
		text: 'Air fry at 200 C for 18 minutes.',
		photo: null
	},
	{
		kind: 'step' as const,
		text: 'Let it sit 5 minutes before saucing.',
		photo: null
	}
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
				theirs: line('¾ cup potato starch', 0)
			},
			{
				kind: 'ingredient',
				state: 'same' as const,
				from_branch_point: true,
				mine: line('¼ cup honey', 1),
				theirs: line('¼ cup honey', 1)
			},
			{
				// Marc took this out. It was at the Branch Point, so it can be
				// carried across as a removal, not only read.
				kind: 'ingredient',
				state: 'only-mine' as const,
				from_branch_point: true,
				mine: line('¼ cup brown sugar', 2),
				theirs: null
			},
			{
				// Mine, written after we parted. Nothing of his resembles it.
				kind: 'ingredient',
				state: 'only-mine' as const,
				from_branch_point: false,
				mine: line('1 Tbsp rice vinegar', 3),
				theirs: null
			},
			{
				// His, written after we parted, landing in the same place. The
				// Pairing declined to join these two, and neither is badged.
				kind: 'ingredient',
				state: 'only-theirs' as const,
				from_branch_point: false,
				mine: null,
				theirs: line('1 tsp gochugaru', 2)
			}
		],
		steps: [
			{
				kind: 'step',
				state: 'only-mine' as const,
				from_branch_point: true,
				mine: step('Deep fry at 190 C until crisp.', 0),
				theirs: null
			},
			{
				kind: 'step',
				state: 'only-theirs' as const,
				from_branch_point: true,
				mine: null,
				theirs: step('Air fry at 200 C for 18 minutes.', 0)
			},
			{
				kind: 'step',
				state: 'only-theirs' as const,
				from_branch_point: false,
				mine: null,
				theirs: step('Let it sit 5 minutes before saucing.', 1)
			}
		],
		fields: {
			title: {
				same: true,
				mine: 'Korean Fried Chicken',
				theirs: 'Korean Fried Chicken'
			},
			yield: { same: true, mine: null, theirs: null },
			prep_time_minutes: { same: true, mine: 20, theirs: 20 },
			cook_time_minutes: { same: true, mine: 30, theirs: 30 },
			source: { same: true, mine: null, theirs: null },
			note: { same: true, mine: null, theirs: null },
			main_photo: { same: true, mine: null, theirs: null }
		}
	};
}

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
					content: divergence().mine.content,
					readings: MY_INGREDIENTS.map(() => null)
				}
			],
			tags: [],
			related_recipes: [],
			cooked: { count: 0, last_cooked_at: null, ratings: [] }
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
					translation: null
				},
				{
					branch_id: 'theirs',
					kitchen_id: 'k_theirs',
					hand_id: 'h_theirs',
					language: 'en',
					head_version_id: 'v_theirs',
					translation: null
				}
			],
			versions: [],
			attempts: []
		},
		divergence: divergence(),
		...extra
	} as Answers;
}

function renderRecipe(answers: Answers = forked()) {
	const kamosu = standIn(answers);
	render(RecipeTestHarness, {
		props: { client: kamosu.client, branchId: 'mine' }
	});
	return { kamosu };
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
					translates_version_id: null
				}
			})
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
			screen.getAllByRole('button', { name: /^Save a Version$/i }).at(-1) as HTMLElement
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
			'1 Tbsp rice vinegar'
		]);
		expect(input.change_note).toBe('Took ¾ cup potato starch from Chez Marc.');
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
			screen.getByText(/replacing 1 cup potato starch \(or corn starch\)/i)
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
						translation: null
					})),
					versions: [],
					attempts: []
				}
			})
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
							translation: null
						}
					],
					versions: [],
					attempts: []
				}
			})
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
							at: '2026-08-14T18:30:00Z'
						},
						{
							person_id: 'p_marie',
							name: 'Marie',
							rating: 'tweak' as const,
							at: '2026-08-02T19:00:00Z'
						}
					],
					...extra
				}
			}
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
