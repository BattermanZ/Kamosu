/**
 * A recipe as `get_recipe` answers it, for the screens that read one without
 * being about it — the sheet that puts a cooking's picture on the recipe
 * (#110) is opened from the diary as well as from the recipe page.
 */

import type { GetRecipeOutput } from '$lib/api/catalogue';

type Content = GetRecipeOutput['versions'][number]['content'];

export const STEPS: Content['steps'] = [
	{ kind: 'section', text: 'The broth', photo: null },
	{ kind: 'step', text: 'Bring the dashi to a simmer.', photo: null },
	{ kind: 'step', text: 'Whisk in the miso off the heat.', photo: 'p_whisked' },
	{ kind: 'step', text: 'Add the tofu and serve.', photo: null },
];

export function recipeAnswer(
	over: { branch_id?: string; kitchen_id?: string; lineage_id?: string } & Partial<Content> = {},
): GetRecipeOutput {
	const { branch_id = 'b_1', kitchen_id = 'k_home', lineage_id = 'l_1', ...content } = over;
	return {
		branch_id,
		lineage_id,
		kitchen_id,
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

/** The cook's Kitchens: their Home Kitchen, which holds `k_home`'s recipes. */
export const MY_KITCHENS = {
	kitchens: [
		{
			id: 'k_home',
			hand_id: 'k_home',
			name: 'Home',
			nickname: null,
			is_home: true,
			members: [{ person_id: 'p_1', name: 'Aurélien' }],
		},
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
