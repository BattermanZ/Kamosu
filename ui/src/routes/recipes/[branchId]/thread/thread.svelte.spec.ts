/**
 * The screen-seam test: the Thread, against a stand-in Kamosu. The answers
 * below are checked against the Catalogue before the screen sees them — a
 * field renamed in `src/catalogue.rs` fails this test in the same commit.
 */

import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent, within } from '@testing-library/svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import ThreadTestHarness from './ThreadTestHarness.svelte';
import { cookbookLabel, threadBranch } from '../../../../testing/recipes';

/** Who is reading. Nobody in these fixtures unless a test says otherwise. */
const NOBODY = { person_id: 'p_nobody', name: 'Nobody' };

function renderThread(branchId: string, answers: Answers) {
	const kamosu = standIn({ get_person: NOBODY, ...answers });
	render(ThreadTestHarness, { props: { client: kamosu.client, branchId } });
	return { kamosu };
}

describe('the Thread screen', () => {
	it('is headed History, the word on the recipe’s button that opens it (#133)', async () => {
		renderThread('b_mine', {
			get_thread: {
				lineage_id: 'l_1',
				branches: [threadBranch('b_mine', { cookbook: cookbookLabel('c_1', ['Aurélien']) })],
				versions: [],
				attempts: [],
			},
		});
		expect(await screen.findByRole('heading', { level: 1, name: 'History' })).toBeInTheDocument();
	});

	it("reads one Branch's Versions oldest first, a quiet save shown plainly, an Attempt hanging off it", async () => {
		const { kamosu } = renderThread('b_mine', {
			get_thread: {
				lineage_id: 'l_1',
				branches: [
					{
						branch_id: 'b_mine',
						cookbook: cookbookLabel('c_1', ['Aurélien']),
						name: null,
						mine: true,
						arrived: false,
						hand_id: 'h_aurelien',
						hand_name: 'Aurélien',
						language: 'en',
						head_version_id: 'v_2',
						translation: null,
					},
				],
				versions: [
					{
						branch_id: 'b_mine',
						sequence: 1,
						version_id: 'v_1',
						parent_version_id: null,
						hand_id: 'h_aurelien',
						hand_name: 'Aurélien',
						name: null,
						change_note: 'Imported from mykoreankitchen.com',
						created_at: '2026-03-03T00:00:00Z',
						translates_version_id: null,
						language: 'en',
					},
					{
						branch_id: 'b_mine',
						sequence: 2,
						version_id: 'v_2',
						parent_version_id: 'v_1',
						hand_id: 'h_aurelien',
						hand_name: 'Aurélien',
						name: 'Hotter second fry',
						change_note: null,
						created_at: '2026-03-14T00:00:00Z',
						translates_version_id: null,
						language: 'en',
					},
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
						as_cooked: null,
						photographs: [],
					},
				],
			},
		});

		expect(await screen.findByText('Imported from mykoreankitchen.com')).toBeInTheDocument();
		expect(screen.getByText('Saved with nothing written down.')).toBeInTheDocument();
		expect(screen.getByText(/Hotter second fry/)).toBeInTheDocument();
		// A rating is the cook's verdict about next time, not a score (#59).
		expect(screen.getByRole('button', { name: /🍲 Again/ })).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).toContain('get_thread');
	});

	it('splits into a line per Branch at the Branch Point, and names each Branch', async () => {
		renderThread('b_mine', {
			get_thread: {
				lineage_id: 'l_1',
				branches: [
					{
						branch_id: 'b_mine',
						cookbook: cookbookLabel('c_1', ['Aurélien']),
						name: null,
						mine: true,
						arrived: false,
						hand_id: 'h_aurelien',
						hand_name: 'Aurélien',
						language: 'en',
						head_version_id: 'v_3',
						translation: null,
					},
					{
						branch_id: 'b_marc',
						cookbook: cookbookLabel('c_marc', ['Marc']),
						name: null,
						mine: false,
						arrived: false,
						hand_id: 'h_marc',
						hand_name: 'Marc',
						language: 'en',
						head_version_id: 'v_4',
						translation: null,
					},
				],
				versions: [
					{
						branch_id: 'b_mine',
						sequence: 1,
						version_id: 'v_1',
						parent_version_id: null,
						hand_id: 'h_aurelien',
						hand_name: 'Aurélien',
						name: null,
						change_note: 'Imported',
						created_at: '2026-03-03T00:00:00Z',
						translates_version_id: null,
						language: 'en',
					},
					{
						branch_id: 'b_marc',
						sequence: 1,
						version_id: 'v_1',
						parent_version_id: null,
						hand_id: 'h_aurelien',
						hand_name: 'Aurélien',
						name: null,
						change_note: 'Imported',
						created_at: '2026-03-03T00:00:00Z',
						translates_version_id: null,
						language: 'en',
					},
					{
						branch_id: 'b_mine',
						sequence: 2,
						version_id: 'v_3',
						parent_version_id: 'v_1',
						hand_id: 'h_aurelien',
						hand_name: 'Aurélien',
						name: null,
						change_note: 'Hotter second fry',
						created_at: '2026-08-09T00:00:00Z',
						translates_version_id: null,
						language: 'en',
					},
					{
						branch_id: 'b_marc',
						sequence: 2,
						version_id: 'v_4',
						parent_version_id: 'v_1',
						hand_id: 'h_marc',
						hand_name: 'Marc',
						name: null,
						change_note: 'Air fryer',
						created_at: '2026-06-02T00:00:00Z',
						translates_version_id: null,
						language: 'en',
					},
				],
				attempts: [],
			},
		});

		expect(await screen.findByText('Splits into 2 Branches here.')).toBeInTheDocument();
		// Each Branch is named as the version switch names it (#131): your own
		// is Yours, another Cookbook's is whose it is. Each Version still says
		// who wrote it.
		expect(screen.getAllByText('Yours').length).toBeGreaterThan(0);
		expect(screen.getAllByText('Marc’s').length).toBeGreaterThan(0);
		expect(screen.getAllByText('Aurélien').length).toBeGreaterThan(0);
		expect(screen.getAllByText('Marc').length).toBeGreaterThan(0);
		// A Hand is shown by its name, never by the id a person cannot read (#113).
		expect(screen.queryByText(/h_aurelien|h_marc/)).not.toBeInTheDocument();
		expect(screen.getByText('Hotter second fry')).toBeInTheDocument();
		expect(screen.getByText('Air fryer')).toBeInTheDocument();
	});

	it('splits again where a Branch forks off another Branch, not the trunk', async () => {
		// b_camille forks off b_marc's m1, not off the shared trunk v1: the
		// case #53's side-by-side lanes could not draw as a third column. As a
		// graph it is one more split, drawn where it happened (#115).
		renderThread('b_mine', {
			get_thread: {
				lineage_id: 'l_1',
				branches: [
					{
						branch_id: 'b_mine',
						cookbook: cookbookLabel('c_1', ['Aurélien']),
						name: null,
						mine: true,
						arrived: false,
						hand_id: 'h_aurelien',
						hand_name: 'Aurélien',
						language: 'en',
						head_version_id: 'v_2',
						translation: null,
					},
					{
						branch_id: 'b_marc',
						cookbook: cookbookLabel('c_marc', ['Marc']),
						name: null,
						mine: false,
						arrived: false,
						hand_id: 'h_marc',
						hand_name: 'Marc',
						language: 'en',
						head_version_id: 'm_2',
						translation: null,
					},
					{
						branch_id: 'b_camille',
						cookbook: cookbookLabel('c_camille', ['Camille']),
						name: null,
						mine: false,
						arrived: false,
						hand_id: 'h_camille',
						hand_name: 'Camille',
						language: 'fr',
						head_version_id: 'c_1',
						translation: null,
					},
				],
				versions: [
					{
						branch_id: 'b_mine',
						sequence: 1,
						version_id: 'v_1',
						parent_version_id: null,
						hand_id: 'h_aurelien',
						hand_name: 'Aurélien',
						name: null,
						change_note: 'Imported',
						created_at: '2026-03-03T00:00:00Z',
						translates_version_id: null,
						language: 'en',
					},
					{
						branch_id: 'b_marc',
						sequence: 1,
						version_id: 'v_1',
						parent_version_id: null,
						hand_id: 'h_aurelien',
						hand_name: 'Aurélien',
						name: null,
						change_note: 'Imported',
						created_at: '2026-03-03T00:00:00Z',
						translates_version_id: null,
						language: 'en',
					},
					{
						branch_id: 'b_camille',
						sequence: 1,
						version_id: 'v_1',
						parent_version_id: null,
						hand_id: 'h_aurelien',
						hand_name: 'Aurélien',
						name: null,
						change_note: 'Imported',
						created_at: '2026-03-03T00:00:00Z',
						translates_version_id: null,
						language: 'en',
					},
					{
						branch_id: 'b_mine',
						sequence: 2,
						version_id: 'v_2',
						parent_version_id: 'v_1',
						hand_id: 'h_aurelien',
						hand_name: 'Aurélien',
						name: null,
						change_note: 'Hotter second fry',
						created_at: '2026-08-09T00:00:00Z',
						translates_version_id: null,
						language: 'en',
					},
					{
						branch_id: 'b_marc',
						sequence: 2,
						version_id: 'm_1',
						parent_version_id: 'v_1',
						hand_id: 'h_marc',
						hand_name: 'Marc',
						name: null,
						change_note: 'Air fryer',
						created_at: '2026-06-02T00:00:00Z',
						translates_version_id: null,
						language: 'en',
					},
					{
						branch_id: 'b_camille',
						sequence: 2,
						version_id: 'm_1',
						parent_version_id: 'v_1',
						hand_id: 'h_marc',
						hand_name: 'Marc',
						name: null,
						change_note: 'Air fryer',
						created_at: '2026-06-02T00:00:00Z',
						translates_version_id: null,
						language: 'en',
					},
					{
						branch_id: 'b_marc',
						sequence: 3,
						version_id: 'm_2',
						parent_version_id: 'm_1',
						hand_id: 'h_marc',
						hand_name: 'Marc',
						name: 'Kid-friendly',
						change_note: null,
						created_at: '2026-08-10T00:00:00Z',
						translates_version_id: null,
						language: 'en',
					},
					{
						branch_id: 'b_camille',
						sequence: 3,
						version_id: 'c_1',
						parent_version_id: 'm_1',
						hand_id: 'h_camille',
						hand_name: 'Camille',
						name: null,
						change_note: 'Translated into French',
						created_at: '2026-08-12T00:00:00Z',
						translates_version_id: null,
						language: 'en',
					},
				],
				attempts: [],
			},
		});

		// Two forks happened: v1 splits mine from {marc, camille}, then m1
		// splits marc from camille. Both are two-way, so the marker text
		// repeats — proving there are genuinely two fork sections, not one.
		const forkMarkers = await screen.findAllByText('Splits into 2 Branches here.');
		expect(forkMarkers).toHaveLength(2);

		// "Air fryer" (m_1's change note) must appear exactly once: Marc and
		// Camille share it, so it is one row, not one per Branch.
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
						cookbook: cookbookLabel('c_1', ['Aurélien']),
						name: null,
						mine: true,
						arrived: false,
						hand_id: 'h_aurelien',
						hand_name: 'Aurélien',
						language: 'en',
						head_version_id: 'v_1',
						translation: null,
					},
				],
				versions: [
					{
						branch_id: 'b_mine',
						sequence: 1,
						version_id: 'v_1',
						parent_version_id: null,
						hand_id: 'h_aurelien',
						hand_name: 'Aurélien',
						name: null,
						change_note: 'Imported',
						created_at: '2026-03-03T00:00:00Z',
						translates_version_id: null,
						language: 'en',
					},
				],
				attempts: [],
			},
			get_recipe: {
				branch_id: 'b_mine',
				lineage_id: 'l_1',
				cookbook: cookbookLabel('c_1', ['Aurélien']),
				name: null,
				writes: true,
				hand_id: 'h_aurelien',
				language: 'en',
				origin_address: null,
				head_version_id: 'v_1',
				translation: null,
				versions: [
					{
						sequence: 1,
						scaled_to: null,
						version_id: 'v_1',
						parent_version_id: null,
						hand_id: 'h_aurelien',
						name: null,
						change_note: 'Imported',
						created_at: '2026-03-03T00:00:00Z',
						translates_version_id: null,
						language: 'en',
						// A recipe that composes nothing, which is nearly all of them (#50).
						components: [],
						content: {
							title: 'Korean Fried Chicken',
							yield: null,
							prep_time_minutes: null,
							cook_time_minutes: null,
							note: null,
							main_photo: null,
							nutrition: null,
							source: null,
							ingredients: [{ kind: 'ingredient', text: '1.4 kg whole chicken' }],
							steps: [{ kind: 'step', text: 'Marinate the chicken.', photo: null }],
						},
						readings: [null],
						measured: { ingredients: [null], steps: [null] },
						cooking: { steps: [{ uses: [], timer_seconds: null }] },
					},
				],
				tags: [],
				related_recipes: [],
				cooked: { count: 0, last_cooked_at: null, ratings: [] },
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
				as_cooked: null,
				photographs: [],
			},
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
			hand_name: 'Aurélien',
			name: null,
			change_note: null,
			created_at: `2026-08-1${n}T00:00:00Z`,
			translates_version_id: null,
			language: 'en',
		}));

		renderThread('b_mine', {
			get_thread: {
				lineage_id: 'l_1',
				branches: [
					{
						branch_id: 'b_mine',
						cookbook: cookbookLabel('c_1', ['Aurélien']),
						name: null,
						mine: true,
						arrived: false,
						hand_id: 'h_aurelien',
						hand_name: 'Aurélien',
						language: 'en',
						head_version_id: 'v_4',
						translation: null,
					},
				],
				versions,
				attempts: [],
			},
		});

		const showButton = await screen.findByRole('button', { name: /4 quiet saves/ });
		expect(within(showButton).getByText('Show')).toBeInTheDocument();

		await fireEvent.click(showButton);
		expect(screen.getAllByText('Saved with nothing written down.')).toHaveLength(4);
		expect(screen.getByRole('button', { name: 'Collapse' })).toBeInTheDocument();
	});
});

describe('a Language said, in the Thread', () => {
	/** One occurrence on one Branch, with everything the Catalogue requires. */
	const occurrence = (sequence: number, version_id: string, language: string, extra = {}) => ({
		branch_id: 'b_mine',
		sequence,
		version_id,
		parent_version_id: sequence === 1 ? null : 'v_1',
		hand_id: 'h_aurelien',
		hand_name: 'Aurélien',
		name: null,
		change_note: null,
		created_at: '2026-09-22T00:00:00Z',
		translates_version_id: null,
		language,
		...extra,
	});

	/**
	 * Saying a recipe's Language appends an occurrence of the head content
	 * carrying the new Language — the SAME Version id, occurring twice, told
	 * apart by its sequence. That is what `set_recipe_language` does.
	 */
	const saidItIsFrench = {
		get_thread: {
			lineage_id: 'l_1',
			branches: [
				{
					branch_id: 'b_mine',
					cookbook: cookbookLabel('c_1', ['Aurélien']),
					name: null,
					mine: true,
					arrived: false,
					hand_id: 'h_aurelien',
					hand_name: 'Aurélien',
					language: 'fr',
					head_version_id: 'v_1',
					translation: null,
				},
			],
			versions: [
				occurrence(1, 'v_1', 'en', { change_note: 'Written down' }),
				occurrence(2, 'v_1', 'fr'),
			],
			attempts: [],
		},
	} as Answers;

	it('says which entry changed the Language, rather than repeating the row', async () => {
		// #106's promise, kept: a Language change MAKES a Version, unlike
		// tagging (#104) or relating (#105), and the sheet says so before it is
		// tapped. Without this the trace it leaves is a second identical row
		// with nothing to say for itself — the Version is there and nobody can
		// see why.
		renderThread('b_mine', saidItIsFrench);

		expect(await screen.findByText('Said this recipe is in French.')).toBeInTheDocument();
		// And it is not passed off as an ordinary save with nothing written down.
		expect(screen.queryByText('Saved with nothing written down.')).not.toBeInTheDocument();
	});

	it('never folds a Language said into a run of quiet saves', async () => {
		// It carries no name and no change note, so the fold would take it —
		// and folding away the one row that explains the duplicate is worse
		// than not drawing it at all.
		const many = {
			get_thread: {
				...(saidItIsFrench.get_thread as Record<string, unknown>),
				versions: [
					occurrence(1, 'v_1', 'en'),
					occurrence(2, 'v_2', 'en'),
					occurrence(3, 'v_3', 'en'),
					occurrence(4, 'v_4', 'en'),
					occurrence(5, 'v_4', 'fr'),
				],
			},
		} as Answers;
		renderThread('b_mine', many);

		expect(await screen.findByText('Said this recipe is in French.')).toBeInTheDocument();
	});
});

describe('who wrote each Version, in the Thread (#113)', () => {
	const version = (sequence: number, hand_name: string | null) => ({
		branch_id: 'b_mine',
		sequence,
		version_id: `v_${sequence}`,
		parent_version_id: sequence === 1 ? null : `v_${sequence - 1}`,
		hand_id: sequence === 1 ? 'p_stranger' : 'p_aurelien',
		hand_name,
		name: null,
		change_note: `Save ${sequence}`,
		created_at: `2026-09-0${sequence}T00:00:00Z`,
		translates_version_id: null,
		language: 'en',
	});

	it('names every Version by the name the server has for its Hand now, and says so when it has none', async () => {
		renderThread('b_mine', {
			get_thread: {
				lineage_id: 'l_1',
				branches: [
					{
						branch_id: 'b_mine',
						cookbook: cookbookLabel('c_1', ['Chez nous']),
						name: null,
						mine: true,
						arrived: false,
						hand_id: 'k_1',
						hand_name: 'Chez nous',
						language: 'en',
						head_version_id: 'v_3',
						translation: null,
					},
				],
				// The server names a Hand live, so the oldest Version of a
				// Person who renamed themselves carries the new name too.
				versions: [version(1, null), version(2, 'Aurélien Dupont'), version(3, 'Aurélien Dupont')],
				attempts: [],
			},
		});

		expect(await screen.findAllByText('Aurélien Dupont')).toHaveLength(2);
		expect(screen.getByText('Someone Kamosu has no name for')).toBeInTheDocument();
		expect(screen.queryByText(/p_stranger|p_aurelien/)).not.toBeInTheDocument();
	});
});

describe('naming a Version from the Thread (#115)', () => {
	const ME = { person_id: 'p_aurelien', name: 'Aurélien' };
	const branch = (branch_id: string, hand_name: string) =>
		threadBranch(branch_id, {
			cookbook: cookbookLabel(`c_${branch_id}`, ['Aurélien', 'Camille'], hand_name),
			hand_id: `c_${branch_id}`,
			hand_name,
			head_version_id: 'v_3',
		});
	const saved = (
		branch_id: string,
		sequence: number,
		hand_id: string,
		extra: Record<string, unknown> = {},
	) => ({
		branch_id,
		sequence,
		version_id: `v_${sequence}`,
		parent_version_id: sequence === 1 ? null : `v_${sequence - 1}`,
		hand_id,
		hand_name: hand_id === 'p_aurelien' ? 'Aurélien' : 'Camille',
		name: null,
		change_note: null,
		created_at: `2026-09-0${sequence}T00:00:00Z`,
		translates_version_id: null,
		language: 'en',
		...extra,
	});

	/**
	 * A Thread the stand-in keeps names for, the way the server does: a
	 * rename lands in it, and the next `get_thread` reads it back.
	 */
	function threadKeepingNames(
		versions: ReturnType<typeof saved>[],
		branches = [branch('b_mine', 'Chez nous')],
	) {
		const names = new Map<string, string | null>(
			versions.map((v) => [`${v.branch_id}:${v.sequence}`, v.name as string | null]),
		);
		return {
			get_person: ME,
			get_thread: () => ({
				lineage_id: 'l_1',
				branches,
				versions: versions.map((v) => ({
					...v,
					name: names.get(`${v.branch_id}:${v.sequence}`) ?? null,
				})),
				attempts: [],
			}),
			rename_version: () => ({ name: null }),
			names,
		};
	}

	/** The Versions drawn, in order: every list item but a split's. */
	const rows = () =>
		screen.getAllByRole('listitem').filter((item) => !item.textContent?.includes('Splits into'));

	it('names your quiet save in place, and offers nothing on a Version another cook saved', async () => {
		const world = threadKeepingNames([
			saved('b_mine', 1, 'p_camille', { change_note: 'Mamie’s way' }),
			saved('b_mine', 2, 'p_aurelien'),
		]);
		const { kamosu } = renderThread('b_mine', {
			...world,
			rename_version: () => {
				world.names.set('b_mine:2', 'Weeknight');
				return { name: 'Weeknight' };
			},
		});

		await screen.findByText('Mamie’s way');
		// Camille's row: readable, nothing to rename.
		expect(within(rows()[0]).queryByRole('button', { name: /Name it|Rename/ })).toBeNull();

		await fireEvent.click(await within(rows()[1]).findByRole('button', { name: 'Name it' }));
		const field = screen.getByRole('textbox', { name: 'Name for this Version' });
		await fireEvent.input(field, { target: { value: 'Weeknight' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

		expect(await screen.findByRole('status')).toHaveTextContent(
			'Named. Nothing else about this Version changed.',
		);
		// The name sits after its writer with a space either side of the dot.
		expect(rows()[1]).toHaveTextContent('Aurélien · “Weeknight”');
		const renames = kamosu.calls.filter((call) => call.operation === 'rename_version');
		expect(renames.map((call) => call.input)).toEqual([
			{ branch_id: 'b_mine', sequence: 2, name: 'Weeknight' },
		]);
		// Naming is not saving: no Version was minted (ADR 0015).
		expect(kamosu.calls.some((call) => call.operation === 'save_recipe_version')).toBe(false);
	});

	it('clears a name as an ordinary answer, never as a mistake', async () => {
		const world = threadKeepingNames([
			saved('b_mine', 1, 'p_aurelien', { name: 'Sunday gratin', change_note: 'Gruyère on top' }),
		]);
		const { kamosu } = renderThread('b_mine', {
			...world,
			rename_version: () => {
				world.names.set('b_mine:1', null);
				return { name: null };
			},
		});

		await fireEvent.click(await screen.findByRole('button', { name: 'Rename' }));
		await fireEvent.click(screen.getByRole('button', { name: 'Remove the name' }));

		expect(await screen.findByRole('status')).toHaveTextContent(
			'Name removed. Nothing else about this Version changed.',
		);
		expect(screen.queryByRole('alert')).toBeNull();
		expect(screen.queryByText(/Sunday gratin/)).toBeNull();
		expect(kamosu.calls.find((call) => call.operation === 'rename_version')?.input).toEqual({
			branch_id: 'b_mine',
			sequence: 1,
			name: null,
		});
		// With no name left, the row offers to name it again.
		expect(screen.getByRole('button', { name: 'Name it' })).toBeInTheDocument();
	});

	it('shows what changed beside the name, and never as something to edit', async () => {
		renderThread(
			'b_mine',
			threadKeepingNames([saved('b_mine', 1, 'p_aurelien', { change_note: 'Gruyère on top' })]),
		);

		await fireEvent.click(await screen.findByRole('button', { name: 'Name it' }));
		// The name is the only field; the note is read, with the reason it stays.
		expect(screen.getAllByRole('textbox')).toHaveLength(1);
		expect(screen.getByText('What changed')).toBeInTheDocument();
		expect(screen.getAllByText('Gruyère on top').length).toBeGreaterThan(0);
		expect(
			screen.getByText('Written when it was saved. It stays as it was written.'),
		).toBeInTheDocument();
	});

	it('names a Version several Branches share on each of them, since it is drawn once', async () => {
		const world = threadKeepingNames(
			[
				saved('b_mine', 1, 'p_aurelien', { change_note: 'Written down' }),
				saved('b_chalet', 1, 'p_aurelien', { change_note: 'Written down' }),
				saved('b_mine', 2, 'p_aurelien', { version_id: 'v_mine', change_note: 'Less salt' }),
				saved('b_chalet', 2, 'p_camille', { version_id: 'v_chalet', change_note: 'More salt' }),
			],
			[branch('b_mine', 'Chez nous'), branch('b_chalet', 'Le Chalet')],
		);
		const { kamosu } = renderThread('b_mine', world);

		expect(await screen.findByText('Splits into 2 Branches here.')).toBeInTheDocument();
		await fireEvent.click(within(rows()[0]).getByRole('button', { name: 'Name it' }));
		await fireEvent.input(screen.getByRole('textbox'), { target: { value: 'The first' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

		await screen.findByRole('status');
		expect(
			kamosu.calls.filter((call) => call.operation === 'rename_version').map((call) => call.input),
		).toEqual([
			{ branch_id: 'b_mine', sequence: 1, name: 'The first' },
			{ branch_id: 'b_chalet', sequence: 1, name: 'The first' },
		]);
		// Camille's own Version on the Chalet's line is hers alone to name.
		expect(within(rows()[2]).queryByRole('button', { name: /Name it|Rename/ })).toBeNull();
	});

	it('offers no rename on a shared Version another cook saved on one of its Branches', async () => {
		// Naming a shared row names every Branch's occurrence, and the server
		// refuses the one that is not yours, so the screen must not offer it.
		renderThread(
			'b_mine',
			threadKeepingNames(
				[
					saved('b_mine', 1, 'p_aurelien', { change_note: 'Written down' }),
					saved('b_chalet', 1, 'p_camille', { change_note: 'Written down' }),
					saved('b_mine', 2, 'p_aurelien', { version_id: 'v_mine', change_note: 'Less salt' }),
					saved('b_chalet', 2, 'p_camille', { version_id: 'v_chalet', change_note: 'More salt' }),
				],
				[branch('b_mine', 'Chez nous'), branch('b_chalet', 'Le Chalet')],
			),
		);

		expect(await screen.findByText('Splits into 2 Branches here.')).toBeInTheDocument();
		expect(within(rows()[0]).queryByRole('button', { name: /Name it|Rename/ })).toBeNull();
		expect(within(rows()[1]).getByRole('button', { name: 'Name it' })).toBeInTheDocument();
	});

	it('says a failed rename failed, and reads the Thread again to show what did land', async () => {
		const world = threadKeepingNames([
			saved('b_mine', 1, 'p_aurelien', { change_note: 'Gruyère' }),
		]);
		const { kamosu } = renderThread('b_mine', {
			...world,
			rename_version: { refuse: 'internal' },
		});

		await fireEvent.click(await screen.findByRole('button', { name: 'Name it' }));
		await fireEvent.input(screen.getByRole('textbox'), { target: { value: 'Sunday' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

		expect(await screen.findByRole('alert')).toHaveTextContent(
			'The name could not be saved. Try again.',
		);
		expect(screen.queryByRole('status')).toBeNull();
		await vi.waitFor(() =>
			expect(kamosu.calls.filter((call) => call.operation === 'get_thread')).toHaveLength(2),
		);
	});
});
