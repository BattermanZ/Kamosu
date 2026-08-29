/**
 * The screen-seam test: the Thread, against a stand-in Kamosu. The answers
 * below are checked against the Catalogue before the screen sees them — a
 * field renamed in `src/catalogue.rs` fails this test in the same commit.
 */

import { describe, expect, it } from 'vitest';
import { render, screen, fireEvent, within } from '@testing-library/svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import ThreadTestHarness from './ThreadTestHarness.svelte';

function renderThread(branchId: string, answers: Answers) {
	const kamosu = standIn(answers);
	render(ThreadTestHarness, { props: { client: kamosu.client, branchId } });
	return { kamosu };
}

describe('the Thread screen', () => {
	it("reads one Branch's Versions oldest first, a quiet save shown plainly, an Attempt hanging off it", async () => {
		const { kamosu } = renderThread('b_mine', {
			get_thread: {
				lineage_id: 'l_1',
				branches: [
					{
						branch_id: 'b_mine',
						kitchen_id: 'k_1',
						hand_id: 'h_aurelien',
						language: 'en',
						head_version_id: 'v_2',
						translation: null
					}
				],
				versions: [
					{
						branch_id: 'b_mine',
						sequence: 1,
						version_id: 'v_1',
						parent_version_id: null,
						hand_id: 'h_aurelien',
						name: null,
						change_note: 'Imported from mykoreankitchen.com',
						created_at: '2026-03-03T00:00:00Z',
						translates_version_id: null,
						language: 'en'
					},
					{
						branch_id: 'b_mine',
						sequence: 2,
						version_id: 'v_2',
						parent_version_id: 'v_1',
						hand_id: 'h_aurelien',
						name: 'Hotter second fry',
						change_note: null,
						created_at: '2026-03-14T00:00:00Z',
						translates_version_id: null,
						language: 'en'
					}
				],
				attempts: [
					{
						id: 'at_1',
						lineage_id: 'l_1',
						person_id: 'p_1',
						version_id: 'v_1',
						current_step_index: 0,
						ticked_ingredients: [],
						cooking_yield: null,
						note: 'Stayed crisp all the way through dinner.',
						rating: 'again' as const,
						finished_at: '2026-03-21T00:00:00Z',
						resumable: false,
						created_at: '2026-03-21T00:00:00Z',
						last_action_at: '2026-03-21T00:00:00Z',
						photographs: []
					}
				]
			}
		});

		expect(await screen.findByText('Imported from mykoreankitchen.com')).toBeInTheDocument();
		expect(screen.getByText('Saved with nothing written down.')).toBeInTheDocument();
		expect(screen.getByText(/Hotter second fry/)).toBeInTheDocument();
		// A rating is the cook's verdict about next time, not a score (#59).
		expect(screen.getByRole('button', { name: /🍲 Again/ })).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).toContain('get_thread');
	});

	it('forks into a column per Branch and names the Branch Point', async () => {
		renderThread('b_mine', {
			get_thread: {
				lineage_id: 'l_1',
				branches: [
					{
						branch_id: 'b_mine',
						kitchen_id: 'k_1',
						hand_id: 'h_aurelien',
						language: 'en',
						head_version_id: 'v_3',
						translation: null
					},
					{
						branch_id: 'b_marc',
						kitchen_id: 'k_2',
						hand_id: 'h_marc',
						language: 'en',
						head_version_id: 'v_4',
						translation: null
					}
				],
				versions: [
					{
						branch_id: 'b_mine',
						sequence: 1,
						version_id: 'v_1',
						parent_version_id: null,
						hand_id: 'h_aurelien',
						name: null,
						change_note: 'Imported',
						created_at: '2026-03-03T00:00:00Z',
						translates_version_id: null,
						language: 'en'
					},
					{
						branch_id: 'b_marc',
						sequence: 1,
						version_id: 'v_1',
						parent_version_id: null,
						hand_id: 'h_aurelien',
						name: null,
						change_note: 'Imported',
						created_at: '2026-03-03T00:00:00Z',
						translates_version_id: null,
						language: 'en'
					},
					{
						branch_id: 'b_mine',
						sequence: 2,
						version_id: 'v_3',
						parent_version_id: 'v_1',
						hand_id: 'h_aurelien',
						name: null,
						change_note: 'Hotter second fry',
						created_at: '2026-08-09T00:00:00Z',
						translates_version_id: null,
						language: 'en'
					},
					{
						branch_id: 'b_marc',
						sequence: 2,
						version_id: 'v_4',
						parent_version_id: 'v_1',
						hand_id: 'h_marc',
						name: null,
						change_note: 'Air fryer',
						created_at: '2026-06-02T00:00:00Z',
						translates_version_id: null,
						language: 'en'
					}
				],
				attempts: []
			}
		});

		expect(await screen.findByText('Splits into 2 Branches here.')).toBeInTheDocument();
		expect(screen.getAllByText('h_aurelien').length).toBeGreaterThan(0);
		expect(screen.getAllByText('h_marc').length).toBeGreaterThan(0);
		expect(screen.getByText('Hotter second fry')).toBeInTheDocument();
		expect(screen.getByText('Air fryer')).toBeInTheDocument();
	});

	it('drops a second-level fork out of the grid — a Branch forking off another Branch, not the trunk', async () => {
		// b_camille forks off b_marc's m1, not off the shared trunk v1 — the
		// exact case #53's decision comment calls out as the one that cannot
		// become a third column, and must render full width below instead.
		renderThread('b_mine', {
			get_thread: {
				lineage_id: 'l_1',
				branches: [
					{
						branch_id: 'b_mine',
						kitchen_id: 'k_1',
						hand_id: 'h_aurelien',
						language: 'en',
						head_version_id: 'v_2',
						translation: null
					},
					{
						branch_id: 'b_marc',
						kitchen_id: 'k_2',
						hand_id: 'h_marc',
						language: 'en',
						head_version_id: 'm_2',
						translation: null
					},
					{
						branch_id: 'b_camille',
						kitchen_id: 'k_3',
						hand_id: 'h_camille',
						language: 'fr',
						head_version_id: 'c_1',
						translation: null
					}
				],
				versions: [
					{
						branch_id: 'b_mine',
						sequence: 1,
						version_id: 'v_1',
						parent_version_id: null,
						hand_id: 'h_aurelien',
						name: null,
						change_note: 'Imported',
						created_at: '2026-03-03T00:00:00Z',
						translates_version_id: null,
						language: 'en'
					},
					{
						branch_id: 'b_marc',
						sequence: 1,
						version_id: 'v_1',
						parent_version_id: null,
						hand_id: 'h_aurelien',
						name: null,
						change_note: 'Imported',
						created_at: '2026-03-03T00:00:00Z',
						translates_version_id: null,
						language: 'en'
					},
					{
						branch_id: 'b_camille',
						sequence: 1,
						version_id: 'v_1',
						parent_version_id: null,
						hand_id: 'h_aurelien',
						name: null,
						change_note: 'Imported',
						created_at: '2026-03-03T00:00:00Z',
						translates_version_id: null,
						language: 'en'
					},
					{
						branch_id: 'b_mine',
						sequence: 2,
						version_id: 'v_2',
						parent_version_id: 'v_1',
						hand_id: 'h_aurelien',
						name: null,
						change_note: 'Hotter second fry',
						created_at: '2026-08-09T00:00:00Z',
						translates_version_id: null,
						language: 'en'
					},
					{
						branch_id: 'b_marc',
						sequence: 2,
						version_id: 'm_1',
						parent_version_id: 'v_1',
						hand_id: 'h_marc',
						name: null,
						change_note: 'Air fryer',
						created_at: '2026-06-02T00:00:00Z',
						translates_version_id: null,
						language: 'en'
					},
					{
						branch_id: 'b_camille',
						sequence: 2,
						version_id: 'm_1',
						parent_version_id: 'v_1',
						hand_id: 'h_marc',
						name: null,
						change_note: 'Air fryer',
						created_at: '2026-06-02T00:00:00Z',
						translates_version_id: null,
						language: 'en'
					},
					{
						branch_id: 'b_marc',
						sequence: 3,
						version_id: 'm_2',
						parent_version_id: 'm_1',
						hand_id: 'h_marc',
						name: 'Kid-friendly',
						change_note: null,
						created_at: '2026-08-10T00:00:00Z',
						translates_version_id: null,
						language: 'en'
					},
					{
						branch_id: 'b_camille',
						sequence: 3,
						version_id: 'c_1',
						parent_version_id: 'm_1',
						hand_id: 'h_camille',
						name: null,
						change_note: 'Translated into French',
						created_at: '2026-08-12T00:00:00Z',
						translates_version_id: null,
						language: 'en'
					}
				],
				attempts: []
			}
		});

		// Two forks happened: v1 splits mine from {marc, camille}, then m1
		// splits marc from camille. Both are two-way, so the marker text
		// repeats — proving there are genuinely two fork sections, not one.
		const forkMarkers = await screen.findAllByText('Splits into 2 Branches here.');
		expect(forkMarkers).toHaveLength(2);

		// "Air fryer" (m_1's change note) must appear exactly once — it was
		// shown inside the marc/camille rail's own trunk, and must not be
		// redrawn when that rail's further fork renders below.
		expect(screen.getAllByText('Air fryer')).toHaveLength(1);

		expect(screen.getByText(/Kid-friendly/)).toBeInTheDocument();
		expect(screen.getByText('Translated into French')).toBeInTheDocument();
	});

	it('opens a past Version in full and cooks from it', async () => {
		const { kamosu } = renderThread('b_mine', {
			get_thread: {
				lineage_id: 'l_1',
				branches: [
					{
						branch_id: 'b_mine',
						kitchen_id: 'k_1',
						hand_id: 'h_aurelien',
						language: 'en',
						head_version_id: 'v_1',
						translation: null
					}
				],
				versions: [
					{
						branch_id: 'b_mine',
						sequence: 1,
						version_id: 'v_1',
						parent_version_id: null,
						hand_id: 'h_aurelien',
						name: null,
						change_note: 'Imported',
						created_at: '2026-03-03T00:00:00Z',
						translates_version_id: null,
						language: 'en'
					}
				],
				attempts: []
			},
			get_recipe: {
				branch_id: 'b_mine',
				lineage_id: 'l_1',
				kitchen_id: 'k_1',
				hand_id: 'h_aurelien',
				language: 'en',
				origin_address: null,
				head_version_id: 'v_1',
				translation: null,
				versions: [
					{
						sequence: 1,
						version_id: 'v_1',
						parent_version_id: null,
						hand_id: 'h_aurelien',
						name: null,
						change_note: 'Imported',
						created_at: '2026-03-03T00:00:00Z',
						translates_version_id: null,
						language: 'en',
						content: {
							title: 'Korean Fried Chicken',
							yield: null,
							prep_time_minutes: null,
							cook_time_minutes: null,
							note: null,
							main_photo: null,
							source: null,
							ingredients: [{ kind: 'ingredient', text: '1.4 kg whole chicken' }],
							steps: [{ kind: 'step', text: 'Marinate the chicken.', photo: null }]
						},
						readings: [null]
					}
				],
				tags: [],
				related_recipes: [],
				cooked: { count: 0, last_cooked_at: null, ratings: [] }
			},
			start_attempt: {
				id: 'at_2',
				lineage_id: 'l_1',
				person_id: 'p_1',
				version_id: 'v_1',
				current_step_index: 0,
				ticked_ingredients: [],
				cooking_yield: null,
				note: null,
				rating: null,
				finished_at: null,
				resumable: true,
				created_at: '2026-08-28T00:00:00Z',
				last_action_at: '2026-08-28T00:00:00Z',
				photographs: []
			}
		});

		await fireEvent.click(await screen.findByText('Imported'));
		expect(await screen.findByText('Korean Fried Chicken')).toBeInTheDocument();
		expect(screen.getByText('1.4 kg whole chicken')).toBeInTheDocument();
		expect(screen.getByText('Marinate the chicken.')).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).toContain('get_recipe');

		await fireEvent.click(screen.getByRole('button', { name: 'Cook this Version' }));
		expect(await screen.findByText(/Started/)).toBeInTheDocument();
		const startCall = kamosu.calls.find((call) => call.operation === 'start_attempt');
		expect(startCall?.input).toEqual({ branch_id: 'b_mine', version_id: 'v_1' });
	});

	it('collapses a run of four or more quiet saves, and expands it on request', async () => {
		const versions = [1, 2, 3, 4].map((n) => ({
			branch_id: 'b_mine',
			sequence: n,
			version_id: `v_${n}`,
			parent_version_id: n === 1 ? null : `v_${n - 1}`,
			hand_id: 'h_aurelien',
			name: null,
			change_note: null,
			created_at: `2026-08-1${n}T00:00:00Z`,
			translates_version_id: null,
			language: 'en'
		}));

		renderThread('b_mine', {
			get_thread: {
				lineage_id: 'l_1',
				branches: [
					{
						branch_id: 'b_mine',
						kitchen_id: 'k_1',
						hand_id: 'h_aurelien',
						language: 'en',
						head_version_id: 'v_4',
						translation: null
					}
				],
				versions,
				attempts: []
			}
		});

		const showButton = await screen.findByRole('button', { name: /4 quiet saves/ });
		expect(within(showButton).getByText('Show')).toBeInTheDocument();

		await fireEvent.click(showButton);
		expect(screen.getAllByText('Saved with nothing written down.')).toHaveLength(4);
		expect(screen.getByRole('button', { name: 'Collapse' })).toBeInTheDocument();
	});
});
