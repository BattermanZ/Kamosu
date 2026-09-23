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
import { keepingForTests, renderScreen } from '../../testing/render';
import { MY_KITCHENS, PROMOTED, recipeAnswer } from '../../testing/recipes';

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
	recipe: { branch_id: 'b_1', title: 'Miso Soup', written_yield: null },
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
						recipe: { branch_id: 'b_2', title: 'Katsu Curry', written_yield: null },
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

	it('says how much was cooked, where it was not the recipe as written (#109)', async () => {
		renderScreen(Cooked, {
			list_attempts: {
				attempts: [
					entry({
						id: 'at_3',
						cooking_yield: { amount: '24', noun: 'servings' },
						recipe: {
							branch_id: 'b_3',
							title: 'Cookies',
							written_yield: { amount: '12', noun: 'servings' },
						},
					}),
					entry({
						id: 'at_2',
						cooking_yield: { amount: '2', noun: '' },
						recipe: { branch_id: 'b_2', title: 'Pain', written_yield: null },
					}),
					entry({
						id: 'at_0',
						cooking_yield: { amount: '½', noun: '' },
						recipe: {
							branch_id: 'b_4',
							title: 'Soupe',
							written_yield: { amount: '3', noun: 'servings' },
						},
					}),
					entry({ id: 'at_1' }),
				],
			},
		});
		expect(
			await screen.findByText('24 servings · the recipe makes 12 servings'),
		).toBeInTheDocument();
		// Halved where counting would land between two servings: still both.
		expect(screen.getByText('×½ · the recipe makes 3 servings')).toBeInTheDocument();
		// A recipe that never said what it makes was multiplied, and says so.
		expect(screen.getByText('×2')).toBeInTheDocument();
		// As written wears nothing: the Miso Soup's line is only its title and date.
		const miso = screen.getByText('Miso Soup').closest('button');
		expect(miso?.textContent).not.toMatch(/servings|×/);
	});

	it('marks a cooking nobody finished, however long ago it was started', async () => {
		renderScreen(Cooked, {
			list_attempts: {
				attempts: [
					entry({
						id: 'at_now',
						recipe: { branch_id: 'b_1', title: 'Pain', written_yield: null },
						finished_at: null,
						resumable: true,
					}),
					entry({
						id: 'at_gone_cold',
						recipe: { branch_id: 'b_2', title: 'Katsu Curry', written_yield: null },
						finished_at: null,
						resumable: false,
					}),
					entry({
						id: 'at_done',
						recipe: { branch_id: 'b_3', title: 'Miso Soup', written_yield: null },
					}),
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
						recipe: { branch_id: 'b_2', title: 'Pain', written_yield: null },
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
				attempts: [entry({ recipe: { branch_id: null, title: 'Miso Soup', written_yield: null } })],
			},
		});

		await fireEvent.click(await screen.findByRole('button', { name: /Miso Soup/ }));

		expect(screen.getByText(/no longer on your shelf/)).toBeInTheDocument();
		expect(screen.queryByRole('link', { name: 'Open the recipe' })).not.toBeInTheDocument();
	});

	it('opens onto the recipe it was cooked from, while that recipe is still held', async () => {
		renderScreen(Cooked, {
			list_attempts: {
				attempts: [
					entry({ recipe: { branch_id: 'b_1', title: 'Miso Soup', written_yield: null } }),
				],
			},
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

	it('shows a cooking’s photographs and adds one beside them (#77)', async () => {
		const keeping = keepingForTests();
		const { kamosu } = renderScreen(
			Cooked,
			{
				list_attempts: { attempts: [entry({ photographs: ['p_plate'] })] },
				edit_attempt: attempt({ photographs: ['p_plate', 'local:0000000000000001'] }),
			},
			keeping,
		);

		await fireEvent.click(await screen.findByRole('button', { name: /Miso Soup/ }));
		expect(screen.getByRole('heading', { name: 'Photographs' })).toBeInTheDocument();
		const shown = await screen.findAllByAltText('A photograph of this cooking');
		expect(shown.map((image) => image.getAttribute('src'))).toEqual([
			'/api/photographs/p_plate/card',
		]);

		const picture = new File(['a plate'], 'plate.jpg', { type: 'image/jpeg' });
		await fireEvent.change(screen.getByLabelText('Add another'), {
			target: { files: [picture] },
		});
		await vi.waitFor(() => {
			const asked = kamosu.calls.find((call) => call.operation === 'edit_attempt');
			expect(asked?.input).toEqual({
				attempt_id: 'at_1',
				add_photographs: ['local:0000000000000001'],
			});
		});
		await vi.waitFor(() =>
			expect(screen.getAllByAltText('A photograph of this cooking')).toHaveLength(2),
		);
	});

	it('asks before deleting a cooking, and takes it out of the diary once it has', async () => {
		const { kamosu } = renderScreen(Cooked, {
			list_attempts: {
				attempts: [
					entry({
						id: 'at_1',
						recipe: { branch_id: 'b_1', title: 'Miso Soup', written_yield: null },
					}),
					entry({
						id: 'at_2',
						recipe: { branch_id: 'b_2', title: 'Katsu Curry', written_yield: null },
					}),
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

/**
 * A cooking's picture made the recipe's (#110, option A): tap it in the diary,
 * say where on the recipe, and read what that does before it is done.
 */
describe('putting a cooking’s picture on the recipe', () => {
	const open = async (answers: Parameters<typeof renderScreen>[1] = {}) => {
		const rendered = renderScreen(
			Cooked,
			{
				list_attempts: { attempts: [entry({ photographs: ['p_plate', 'p_pot'] })] },
				get_recipe: recipeAnswer(),
				list_kitchens: MY_KITCHENS,
				promote_attempt_photograph: PROMOTED,
				...answers,
			},
			keepingForTests(),
		);
		await fireEvent.click(await screen.findByRole('button', { name: /Miso Soup/ }));
		return rendered;
	};
	const sheet = () => screen.findByRole('dialog', { name: 'Use a cooking photo on the recipe' });

	it('makes a tapped photograph the Main Photo, saying first what that does', async () => {
		const { kamosu } = await open();
		expect(screen.getByText('Tap a photo to put it on the recipe.')).toBeInTheDocument();

		await fireEvent.click(
			screen.getAllByRole('button', { name: 'Put this photo on the recipe' })[1],
		);
		const on = within(await sheet());
		// Nothing is said, and nothing can be pressed, until a place is picked.
		expect(
			await on.findByRole('button', {
				name: "The recipe's photo Instead of the Cover it wears now",
			}),
		).toBeInTheDocument();
		expect(on.queryByText(/stops being private/)).not.toBeInTheDocument();
		expect(on.getByRole('button', { name: 'Use this photo' })).toBeDisabled();

		await fireEvent.click(on.getByRole('button', { name: /The recipe's photo/ }));
		// A Version, and a picture that stops being private: both, beside the button.
		expect(on.getByText('This will save')).toBeInTheDocument();
		expect(
			on.getByText(
				'Saving writes a new Version onto your Miso Soup, in Home. It shows in the Thread like any edit.',
			),
		).toBeInTheDocument();
		expect(on.getByText(/The photo stops being private/)).toBeInTheDocument();

		await fireEvent.click(on.getByRole('button', { name: 'Use this photo' }));
		await vi.waitFor(() =>
			expect(
				kamosu.calls.find((call) => call.operation === 'promote_attempt_photograph')?.input,
			).toEqual({
				attempt_id: 'at_1',
				photograph_id: 'p_pot',
				branch_id: 'b_1',
				step_index: null,
			}),
		);
		expect(await screen.findByText('It is on the recipe now.')).toBeInTheDocument();
		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
		// Promoting is not moving: the cooking still holds both pictures.
		expect(screen.getAllByAltText('A photograph of this cooking')).toHaveLength(2);
	});

	it('puts it on a named Step, by that Step’s place in the recipe', async () => {
		const { kamosu } = await open();
		await fireEvent.click(
			screen.getAllByRole('button', { name: 'Put this photo on the recipe' })[0],
		);
		const on = within(await sheet());
		await fireEvent.click(await on.findByText("A step's photo…"));
		// Steps are numbered as the recipe numbers them — the section heading
		// takes no number — and a Step that already has a picture says so.
		const whisk = on.getByRole('button', { name: /Whisk in the miso/ });
		expect(within(whisk).getByText('2')).toBeInTheDocument();
		expect(within(whisk).getByText('has one')).toBeInTheDocument();
		await fireEvent.click(whisk);
		await fireEvent.click(on.getByRole('button', { name: 'Use this photo' }));

		await vi.waitFor(() =>
			expect(
				kamosu.calls.find((call) => call.operation === 'promote_attempt_photograph')?.input,
			).toMatchObject({ photograph_id: 'p_plate', step_index: 2 }),
		);
	});

	it('says it will start your own copy on a recipe another Kitchen holds', async () => {
		const { kamosu } = await open({
			get_recipe: recipeAnswer({ kitchen_id: 'k_marc' }),
			promote_attempt_photograph: { ...PROMOTED, branch_id: 'b_copy', copied: true },
		});
		await fireEvent.click(
			screen.getAllByRole('button', { name: 'Put this photo on the recipe' })[0],
		);
		const on = within(await sheet());
		await fireEvent.click(await on.findByRole('button', { name: /The recipe's photo/ }));
		expect(on.getByText('This will start your own copy')).toBeInTheDocument();
		expect(on.getByText(/Saving does not change it/)).toBeInTheDocument();
		await fireEvent.click(on.getByRole('button', { name: 'Start my own copy' }));

		await vi.waitFor(() =>
			expect(kamosu.calls.some((call) => call.operation === 'promote_attempt_photograph')).toBe(
				true,
			),
		);
		expect(
			await screen.findByText('It is on your own copy of the recipe now.'),
		).toBeInTheDocument();
		expect(screen.getByRole('link', { name: 'Open your copy' })).toHaveAttribute(
			'href',
			'/recipes/b_copy',
		);
	});

	it('says why, and keeps the sheet open, when the Core refuses', async () => {
		await open({
			promote_attempt_photograph: {
				refuse: 'bad_request',
				message: 'that Photograph is not one of this Attempt’s',
			},
		});
		await fireEvent.click(
			screen.getAllByRole('button', { name: 'Put this photo on the recipe' })[0],
		);
		const on = within(await sheet());
		await fireEvent.click(await on.findByRole('button', { name: /The recipe's photo/ }));
		await fireEvent.click(on.getByRole('button', { name: 'Use this photo' }));
		expect(await on.findByRole('alert')).toHaveTextContent(/not one of this Attempt/);
		expect(screen.queryByText('It is on the recipe now.')).not.toBeInTheDocument();
	});

	it('offers nothing on a cooking with no photographs, the ordinary case', async () => {
		await open({ list_attempts: { attempts: [entry()] } });
		expect(screen.queryByText('Tap a photo to put it on the recipe.')).not.toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Put this photo on the recipe' })).toBeNull();
	});

	it('offers nothing while a picture is still only on this phone, or the recipe has gone', async () => {
		await open({
			list_attempts: {
				attempts: [
					entry({ id: 'at_1', photographs: ['local:0000000000000001'] }),
					entry({
						id: 'at_2',
						photographs: ['p_plate'],
						recipe: { branch_id: null, title: 'Gone Soup', written_yield: null },
					}),
				],
			},
		});
		expect(screen.queryByRole('button', { name: 'Put this photo on the recipe' })).toBeNull();
		await fireEvent.click(screen.getByRole('button', { name: /Gone Soup/ }));
		expect(screen.queryByRole('button', { name: 'Put this photo on the recipe' })).toBeNull();
		expect(screen.queryByText('Tap a photo to put it on the recipe.')).not.toBeInTheDocument();
	});
});
