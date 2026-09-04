/**
 * The screen-seam test: Cooked, the cooking diary, against a stand-in Kamosu
 * (#60).
 *
 * The stand-in checks every answer below against the shape the Catalogue
 * declares before the screen sees it, so a test here can lie about the values
 * and never about the shape — rename a field in `src/catalogue.rs` and this
 * fails in the same commit.
 *
 * What it is really for is the part a behaviour test cannot reach, because
 * none of it is an Operation's answer: that an unfinished cooking is **shown
 * and marked** rather than quietly left out, that the mark tells In Progress
 * apart from one somebody walked away from, that a recipe which has left the
 * shelf still reads under the name it was known by with nothing to tap, and
 * that correcting or deleting a cooking happens here without leaving.
 */

import { describe, expect, it, vi } from 'vitest';
import { screen, fireEvent, within } from '@testing-library/svelte';
import Cooked from './+page.svelte';
import { renderScreen } from '../../testing/render';

/**
 * One Attempt as `edit_attempt` answers it — with everything the Catalogue
 * requires present, and no recipe beside it. The recipe is the diary's own
 * addition, and the seam refuses an answer carrying what the Catalogue does not
 * declare, which is how a test finds out it was about to lie about the shape.
 */
const attempt = (over: Record<string, unknown> = {}) => ({
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
	...over,
});

/** One line of the diary: that same Attempt, and the recipe it was cooked from. */
const entry = (over: Record<string, unknown> = {}) => ({
	...attempt(),
	recipe: { branch_id: 'b_1', title: 'Miso Soup' },
	...over,
});

describe('the cooking diary', () => {
	it('lists the cookings it was given, under the month each one happened in', async () => {
		renderScreen(Cooked, {
			list_attempts: {
				attempts: [
					entry({ id: 'at_2', created_at: '2026-08-20T18:00:00.000Z' }),
					entry({
						id: 'at_1',
						created_at: '2026-07-03T18:00:00.000Z',
						recipe: { branch_id: 'b_2', title: 'Katsu Curry' },
					}),
				],
			},
		});

		expect(await screen.findByText('Miso Soup')).toBeInTheDocument();
		expect(screen.getByText('Katsu Curry')).toBeInTheDocument();
		expect(screen.getByText('2 cookings')).toBeInTheDocument();

		// Filed by date, not by dish: that is the whole reason this screen is
		// not the shelf with a different sort on it.
		const headings = screen.getAllByRole('heading', { level: 2 }).map((h) => h.textContent);
		expect(headings).toEqual(['August 2026', 'July 2026']);
	});

	it('marks a cooking nobody finished, however long ago it was started', async () => {
		renderScreen(Cooked, {
			list_attempts: {
				attempts: [
					entry({
						id: 'at_now',
						recipe: { branch_id: 'b_1', title: 'Pain' },
						finished_at: null,
						resumable: true,
					}),
					entry({
						id: 'at_gone_cold',
						recipe: { branch_id: 'b_2', title: 'Katsu Curry' },
						finished_at: null,
						resumable: false,
					}),
					entry({ id: 'at_done', recipe: { branch_id: 'b_3', title: 'Miso Soup' } }),
				],
			},
		});

		// Starting is what makes a cooking real (ADR 0010) — all three are here.
		expect(await screen.findByText('Pain')).toBeInTheDocument();
		expect(screen.getByText('Katsu Curry')).toBeInTheDocument();
		expect(screen.getByText('Miso Soup')).toBeInTheDocument();

		// The two unfinished ones are marked the SAME. They differ only in
		// `resumable`, which says whether resuming is still offered and never
		// whether the cooking happened (ADR 0010) — and `start_attempt` hands
		// either of them straight back. A mark that flipped at seventy-two
		// hours would be describing Kamosu's prompt, not the evening.
		expect(screen.getAllByText('Still cooking')).toHaveLength(2);

		// And the ordinary case wears no mark at all.
		expect(screen.queryByText(/Never finished/)).not.toBeInTheDocument();
	});

	it('shows the date and the verdict on every entry that has one', async () => {
		renderScreen(Cooked, {
			list_attempts: {
				attempts: [
					entry({ rating: 'again', created_at: '2026-08-20T18:00:00.000Z' }),
					entry({
						id: 'at_2',
						rating: null,
						created_at: '2026-08-14T18:00:00.000Z',
						recipe: { branch_id: 'b_2', title: 'Pain' },
					}),
				],
			},
		});

		// The date the cooking happened, beside the recipe it was of — its own
		// on every line, since two cookings in one month is the ordinary case.
		expect(await screen.findByText('August 20')).toBeInTheDocument();
		expect(screen.getByText('August 14')).toBeInTheDocument();
		expect(screen.getByText('Again')).toBeInTheDocument();

		// A rating is optional and its absence says nothing at all — there is
		// no "unrated" to print (ADR 0015 refuses the arithmetic that would
		// need one).
		expect(screen.getAllByText(/Again|Tweak it|No/)).toHaveLength(1);
	});

	it('still names a recipe that has left the shelf, and offers nothing to tap', async () => {
		renderScreen(Cooked, {
			list_attempts: {
				attempts: [entry({ recipe: { branch_id: null, title: 'Miso Soup' } })],
			},
		});

		await fireEvent.click(await screen.findByRole('button', { name: /Miso Soup/ }));

		expect(screen.getByText(/no longer on your shelf/)).toBeInTheDocument();
		expect(screen.queryByRole('link', { name: 'Open the recipe' })).not.toBeInTheDocument();
	});

	it('opens onto the recipe it was cooked from, while that recipe is still held', async () => {
		renderScreen(Cooked, {
			list_attempts: { attempts: [entry({ recipe: { branch_id: 'b_1', title: 'Miso Soup' } })] },
		});

		await fireEvent.click(await screen.findByRole('button', { name: /Miso Soup/ }));

		expect(screen.getByRole('link', { name: 'Open the recipe' })).toHaveAttribute(
			'href',
			'/recipes/b_1',
		);
	});

	it('corrects a cooking in place — an Attempt is its cook’s to edit', async () => {
		const { kamosu } = renderScreen(Cooked, {
			list_attempts: { attempts: [entry({ rating: 'no', note: 'Trop salé.' })] },
			edit_attempt: attempt({ rating: 'again', note: 'Mieux au dashi.' }),
		});

		await fireEvent.click(await screen.findByRole('button', { name: /Miso Soup/ }));
		await fireEvent.click(screen.getByRole('button', { name: 'Again' }));
		await fireEvent.input(screen.getByRole('textbox'), {
			target: { value: 'Mieux au dashi.' },
		});
		await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

		await vi.waitFor(() => {
			const asked = kamosu.calls.find((call) => call.operation === 'edit_attempt');
			expect(asked?.input).toMatchObject({
				attempt_id: 'at_1',
				rating: 'again',
				note: 'Mieux au dashi.',
			});
		});

		// The correction shows here without a refetch: editing an Attempt makes
		// no Version and there is no history for it to appear in (ADR 0005).
		expect(await screen.findByText('Mieux au dashi.')).toBeInTheDocument();
	});

	it('asks before deleting a cooking, and takes it out of the diary once it has', async () => {
		const { kamosu } = renderScreen(Cooked, {
			list_attempts: {
				attempts: [
					entry({ id: 'at_1', recipe: { branch_id: 'b_1', title: 'Miso Soup' } }),
					entry({ id: 'at_2', recipe: { branch_id: 'b_2', title: 'Katsu Curry' } }),
				],
			},
			delete_attempt: { deleted: true },
		});

		await fireEvent.click(await screen.findByRole('button', { name: /Miso Soup/ }));
		await fireEvent.click(screen.getByRole('button', { name: 'Delete this cooking' }));

		// Deleting a cooking is how a false start is undone, so it is offered
		// plainly — and asked about once, because it does not come back.
		expect(screen.getByText(/gone for good/)).toBeInTheDocument();
		expect(kamosu.calls.some((call) => call.operation === 'delete_attempt')).toBe(false);

		await fireEvent.click(screen.getByRole('button', { name: 'Delete' }));

		await vi.waitFor(() => {
			const asked = kamosu.calls.find((call) => call.operation === 'delete_attempt');
			expect(asked?.input).toMatchObject({ attempt_id: 'at_1' });
		});
		await vi.waitFor(() => {
			expect(screen.queryByText('Miso Soup')).not.toBeInTheDocument();
		});
		expect(screen.getByText('Katsu Curry')).toBeInTheDocument();
	});

	it('says the diary is empty rather than that it failed, when nothing has been cooked', async () => {
		renderScreen(Cooked, { list_attempts: { attempts: [] } });

		expect(await screen.findByText(/Nothing cooked yet/)).toBeInTheDocument();
	});

	it('says so when the diary could not be read', async () => {
		renderScreen(Cooked, { list_attempts: { refuse: 'internal' } });

		const said = await screen.findByRole('alert');
		expect(within(said).getByText(/diary could not be read/)).toBeInTheDocument();
	});
});
