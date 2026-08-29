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
import type { GetRecipeOutput } from '$lib/api/catalogue';
import RecipeTestHarness from './RecipeTestHarness.svelte';

/** One slot of a Version's `readings`: what Kamosu understood, or nothing. */
type Slot = GetRecipeOutput['versions'][number]['readings'][number];

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
		source: { text: 'mykoreankitchen.com', link: null },
		ingredients,
		steps,
	},
	readings: ingredients.map(() => null),
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
			main_photo: { same: true, mine: null, theirs: null },
		},
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
					readings: MY_INGREDIENTS.map(() => null),
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
			versions: [],
			attempts: [],
		},
		divergence: divergence(),
		...extra,
	} as Answers;
}

function renderRecipe(answers: Answers = forked()) {
	const kamosu = standIn(answers);
	render(RecipeTestHarness, {
		props: { client: kamosu.client, branchId: 'mine' },
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
					versions: [],
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
	function solo(content: Record<string, unknown> = {}, readings?: Slot[]): Answers {
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
						{ amount: '1400', unit: 'g', target: 'chicken' },
						null,
						null,
						{ amount: '3', unit: 'tbsp', target: 'ketchup' },
					],
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
				reading: { amount: '500', unit: 'ml', target: 'frying oil' },
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
			set_reading: { line_index: 1, reading: null },
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
		});
		expect(screen.queryByText('1400 g chicken')).not.toBeInTheDocument();
		expect(screen.getByText('1.4 kg whole chicken')).toBeInTheDocument();
	});

	it('reads whole with no photo, no Yield, no times, no Note and no Source', async () => {
		renderRecipe(
			solo({
				main_photo: null,
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
					reading: { amount: '¾', unit: 'cup', target: 'potato starch' },
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
});
