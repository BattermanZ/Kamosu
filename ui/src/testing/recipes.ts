/**
 * A recipe as `get_recipe` answers it, for the screens that read one without
 * being about it — the sheet that puts a cooking's picture on the recipe
 * (#110) is opened from the diary as well as from the recipe page.
 */

import type { GetRecipeOutput, GetThreadOutput, ListKitchensOutput } from '$lib/api/catalogue';
import { refreshed } from '$lib/offline/device.svelte';

type Content = GetRecipeOutput['versions'][number]['content'];

export const STEPS: Content['steps'] = [
	{ kind: 'section', text: 'The broth', photo: null },
	{ kind: 'step', text: 'Bring the dashi to a simmer.', photo: null },
	{ kind: 'step', text: 'Whisk in the miso off the heat.', photo: 'p_whisked' },
	{ kind: 'step', text: 'Add the tofu and serve.', photo: null },
];

export function recipeAnswer(
	over: { branch_id?: string; writes?: boolean; lineage_id?: string } & Partial<Content> = {},
): GetRecipeOutput {
	const { branch_id = 'b_1', writes = true, lineage_id = 'l_1', ...content } = over;
	return {
		branch_id,
		lineage_id,
		cookbook: { id: 'c_1', name: null, authors: [{ person_id: 'p_1', name: 'Aurélien' }] },
		name: null,
		writes,
		hand_id: 'h_1',
		language: 'en',
		origin_address: null,
		head_version_id: 'v_1',
		translation: null,
		tags: [],
		related_recipes: [],
		cooked: { count: 1, last_cooked_at: '2026-08-20T19:30:00.000Z', ratings: [] },
		versions: [
			{
				sequence: 1,
				version_id: 'v_1',
				parent_version_id: null,
				hand_id: 'h_1',
				name: null,
				change_note: null,
				created_at: '2026-08-01T09:00:00Z',
				translates_version_id: null,
				scaled_to: null,
				language: 'en',
				components: [],
				content: {
					title: 'Miso Soup',
					yield: null,
					prep_time_minutes: null,
					cook_time_minutes: null,
					note: null,
					main_photo: null,
					nutrition: null,
					source: null,
					ingredients: [],
					...content,
					steps: content.steps ?? STEPS,
				},
				readings: [],
				measured: { ingredients: [], steps: (content.steps ?? STEPS).map(() => null) },
				cooking: {
					steps: (content.steps ?? STEPS).map((row) =>
						row.kind === 'step' ? { uses: [], timer_seconds: null } : null,
					),
				},
			},
		],
	};
}

/** The cook's Kitchens: one, which sees their own Cookbook. */
export const MY_KITCHENS = {
	kitchens: [
		kitchenAnswer('k_home', {
			name: 'Home',
			members: [{ person_id: 'p_1', name: 'Aurélien' }],
		}),
	],
};

/** What `promote_attempt_photograph` answers: the save it made. */
export const PROMOTED = {
	branch_id: 'b_1',
	version_id: 'v_2',
	parent_version_id: 'v_1',
	sequence: 2,
	collapsed: false,
	copied: false,
	language: 'en',
	language_offer: null,
	translates_version_id: null,
};

/** One of the cook's own cookings, as `list_attempts` answers it. */
export const diaryEntry = (over: Record<string, unknown> = {}) => ({
	id: 'at_1',
	lineage_id: 'l_1',
	person_id: 'p_1',
	version_id: 'v_1',
	current_step_index: 0,
	ticked_ingredients: [],
	cooking_yield: null,
	note: null,
	rating: null,
	finished_at: '2026-08-20T19:30:00.000Z',
	resumable: false,
	created_at: '2026-08-20T18:00:00.000Z',
	last_action_at: '2026-08-20T19:30:00.000Z',
	as_cooked: null,
	photographs: [],
	recipe: { branch_id: 'b_1', title: 'Miso Soup', written_yield: null },
	...over,
});

type ThreadBranch = GetThreadOutput['branches'][number];
type Kitchen = ListKitchensOutput['kitchens'][number];

/** A Cookbook as the Core labels one: Aurélien's own, unless a test says whose. */
export function cookbookLabel(
	id = 'c_1',
	authors: string[] = ['Aurélien'],
	name: string | null = null,
): ThreadBranch['cookbook'] {
	return {
		id,
		name,
		authors: authors.map((author, index) => ({ person_id: `p_${id}_${index}`, name: author })),
	};
}

/**
 * One Branch as a Thread names it (#131): the reader's own unnamed one unless
 * a test says otherwise.
 */
export function threadBranch(branch_id: string, over: Partial<ThreadBranch> = {}): ThreadBranch {
	return {
		branch_id,
		cookbook: cookbookLabel(),
		name: null,
		mine: true,
		arrived: false,
		hand_id: `h_${branch_id}`,
		hand_name: null,
		language: 'en',
		head_version_id: `v_${branch_id}`,
		translation: null,
		...over,
	};
}

/** Somebody else's Branch, in their own Cookbook: Hélène's, unless a test says whose. */
export function theirBranch(branch_id: string, whose = 'Hélène', over: Partial<ThreadBranch> = {}) {
	return threadBranch(branch_id, {
		cookbook: cookbookLabel(`c_${whose}`, [whose]),
		mine: false,
		...over,
	});
}

/** One Version as a Thread lists it, on the Branch that holds it. */
export function threadVersion(
	branch_id: string,
	version_id: string,
	sequence = 1,
	parent_version_id: string | null = null,
): GetThreadOutput['versions'][number] {
	return {
		branch_id,
		version_id,
		sequence,
		parent_version_id,
		hand_id: `h_${branch_id}`,
		hand_name: null,
		name: null,
		change_note: null,
		created_at: '2026-08-01T09:00:00Z',
		language: 'en',
		translates_version_id: null,
	};
}

/** A Kitchen as `list_kitchens` answers it: its members, and whose Cookbooks it sees. */
export function kitchenAnswer(id: string, over: Partial<Kitchen> = {}): Kitchen {
	return {
		id,
		name: 'Maison Batterman',
		nickname: null,
		members: [],
		cookbooks: [cookbookLabel()],
		...over,
	};
}

/** One entry as `search_recipes` answers it, with everything the Catalogue requires. */
export const searchEntry = (id: string, title: string) => ({
	branch_id: `b_${id}`,
	lineage_id: `l_${id}`,
	title,
	language: 'en',
	language_fallback: false,
	main_photo: null,
	matched: null,
	yield: null,
});

/** Every `search_recipes` a screen has sent, in order (#121). */
export const searchesSent = (kamosu: { calls: { operation: string; input: unknown }[] }) =>
	kamosu.calls.filter((call) => call.operation === 'search_recipes').map((call) => call.input);

/**
 * What the service worker does when the server answers `search_recipes`
 * otherwise than the phone's cache did (#76): tells every screen reading it.
 */
export const serverAnsweredSearchesOtherwise = () =>
	refreshed.set('search_recipes', (refreshed.get('search_recipes') ?? 0) + 1);
