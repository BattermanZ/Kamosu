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
import { carrying } from '../../testing/drops';

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
	unkept: null,
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
		// Day-first, as every date in Kamosu is written since #224.
		expect(await screen.findByText('20 August')).toBeInTheDocument();
		expect(screen.getByText('14 August')).toBeInTheDocument();
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
				'Saving adds a Version to your Miso Soup, in your Cookbook. It shows in the history like any edit.',
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

	it('says it will start your own copy on a recipe somebody else writes', async () => {
		const { kamosu } = await open({
			get_recipe: recipeAnswer({ writes: false }),
			promote_attempt_photograph: { ...PROMOTED, branch_id: 'b_copy', copied: true },
		});
		await fireEvent.click(
			screen.getAllByRole('button', { name: 'Put this photo on the recipe' })[0],
		);
		const on = within(await sheet());
		await fireEvent.click(await on.findByRole('button', { name: /The recipe's photo/ }));
		expect(on.getByText('This will start your own copy')).toBeInTheDocument();
		expect(
			on.getByText(
				'This recipe is in Stéphane’s Cookbook, so your changes go into your own copy of Miso Soup, in your Cookbook. Stéphane’s stays as it is.',
			),
		).toBeInTheDocument();
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

	it('says a copy starts because the recipe was sent to you (#132)', async () => {
		await open({ get_recipe: recipeAnswer({ writes: false, arrived: true }) });
		await fireEvent.click(
			screen.getAllByRole('button', { name: 'Put this photo on the recipe' })[0],
		);
		const on = within(await sheet());
		await fireEvent.click(await on.findByRole('button', { name: /The recipe's photo/ }));
		expect(on.getByText('This will start your own copy')).toBeInTheDocument();
		expect(
			on.getByText(
				'You were sent this recipe, so your changes go into your own copy of Miso Soup, in your Cookbook. The one you were sent stays as it arrived.',
			),
		).toBeInTheDocument();
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

describe('dropping a photograph on a cooking (#205)', () => {
	async function opened() {
		const drawn = renderScreen(
			Cooked,
			{
				list_attempts: { attempts: [entry({ photographs: ['p_plate'] })] },
				edit_attempt: attempt({ photographs: ['p_plate', 'local:0000000000000001'] }),
			},
			keepingForTests(),
		);
		await fireEvent.click(await screen.findByRole('button', { name: /Miso Soup/ }));
		return { ...drawn, place: screen.getByRole('heading', { name: 'Photographs' }) };
	}

	it('adds a dropped photograph to the cooking, as Add another does', async () => {
		const { kamosu, place } = await opened();
		const dataTransfer = carrying({
			files: [new File(['a plate'], 'plate.jpg', { type: 'image/jpeg' })],
		});
		await fireEvent.dragEnter(place, { dataTransfer });
		expect(screen.getByText('Drop to add it')).toBeInTheDocument();
		expect(await fireEvent.dragOver(place, { dataTransfer })).toBe(false);
		await fireEvent.drop(place, { dataTransfer });
		expect(screen.queryByText('Drop to add it')).not.toBeInTheDocument();
		await vi.waitFor(() =>
			expect(kamosu.calls.find((call) => call.operation === 'edit_attempt')?.input).toEqual({
				attempt_id: 'at_1',
				add_photographs: ['local:0000000000000001'],
			}),
		);
	});

	it('refuses anything else in words and changes nothing', async () => {
		const { kamosu, place } = await opened();
		const dataTransfer = carrying({
			files: [new File(['%PDF'], 'tarte.pdf', { type: 'application/pdf' })],
		});
		await fireEvent.dragEnter(place, { dataTransfer });
		expect(screen.getByText('Only a photograph')).toBeInTheDocument();
		await fireEvent.drop(place, { dataTransfer });
		expect(screen.getByRole('alert')).toHaveTextContent(
			"That didn't go anywhere. A cooking's photograph has to be a picture.",
		);
		expect(kamosu.calls.some((call) => call.operation === 'edit_attempt')).toBe(false);
	});
});

describe('keeping a tweaked cooking as a Version, from the diary (#210)', () => {
	const line = (kind: string, text: string, index: number) => ({ kind, text, index });

	/** A cooking with more panko than the recipe says, and the water left out. */
	const AS_COOKED = {
		version_id: 'v_cooked',
		content: {
			title: 'Chicken Katsu Curry',
			yield: null,
			prep_time_minutes: null,
			cook_time_minutes: null,
			note: null,
			main_photo: null,
			nutrition: null,
			source: null,
			ingredients: [{ kind: 'ingredient' as const, text: '2 cups panko' }],
			steps: [{ kind: 'step' as const, text: 'Coat the chicken in panko.', photo: null }],
		},
		against: {
			ingredients: [
				{
					kind: 'ingredient',
					state: 'changed' as const,
					from_branch_point: true,
					mine: line('ingredient', '1 cup panko', 0),
					theirs: line('ingredient', '2 cups panko', 0),
				},
				{
					kind: 'ingredient',
					state: 'only-mine' as const,
					from_branch_point: true,
					mine: line('ingredient', '800 ml water', 1),
					theirs: null,
				},
			],
			steps: [
				{
					kind: 'step',
					state: 'same' as const,
					from_branch_point: true,
					mine: line('step', 'Coat the chicken in panko.', 0),
					theirs: line('step', 'Coat the chicken in panko.', 0),
				},
			],
		},
		promotion_declined: false,
	};

	/** Where the Core says the changes would be kept: the cook's own recipe. */
	const UNKEPT = {
		branch_id: 'b_1',
		moved_on: false,
		writes: true,
		mine: true,
		arrived: false,
		cookbook: { id: 'c_1', name: null, authors: [{ person_id: 'p_1', name: 'Stéphane' }] },
	};

	const tweaked = (over: Record<string, unknown> = {}) =>
		entry({
			recipe: { branch_id: 'b_1', title: 'Chicken Katsu Curry', written_yield: null },
			as_cooked: AS_COOKED,
			unkept: UNKEPT,
			...over,
		});

	const openTweaked = async (answers: Record<string, unknown> = {}, over = {}) => {
		const rendered = renderScreen(Cooked, {
			list_attempts: { attempts: [tweaked(over)] },
			promote_as_cooked: PROMOTED,
			...answers,
		});
		await fireEvent.click(await screen.findByRole('button', { name: /chicken katsu curry/i }));
		return rendered;
	};

	it('shows what the cooking changed, and offers to keep it', async () => {
		await openTweaked();

		const section = screen.getByRole('heading', { name: 'What you changed' }).parentElement!;
		expect(within(section).getByText('2 cups panko')).toBeInTheDocument();
		expect(within(section).getByText('1 cup panko')).toHaveClass('line-through');
		expect(within(section).getByText('You left this out')).toBeInTheDocument();
		expect(within(section).getByText('800 ml water')).toHaveClass('line-through');
		expect(
			within(section).getByRole('button', { name: 'Keep it as a new Version' }),
		).toBeInTheDocument();
	});

	it('keeps nothing on the first tap, and the same Version the recipe page would on the second', async () => {
		const { kamosu } = await openTweaked();

		await fireEvent.click(screen.getByRole('button', { name: 'Keep it as a new Version' }));
		expect(kamosu.calls.some((call) => call.operation === 'promote_as_cooked')).toBe(false);

		await fireEvent.click(screen.getByRole('button', { name: 'Save as a new Version' }));
		await vi.waitFor(() => {
			expect(kamosu.calls.find((call) => call.operation === 'promote_as_cooked')?.input).toEqual({
				attempt_id: 'at_1',
				branch_id: 'b_1',
			});
		});

		// Kept: the entry says so and links to the recipe, and offers nothing more.
		const said = await screen.findByText(/Kept\. It's a new Version of the recipe now\./);
		expect(within(said).getByRole('link', { name: 'Open the recipe' })).toHaveAttribute(
			'href',
			'/recipes/b_1',
		);
		expect(screen.queryByRole('button', { name: 'Keep it as a new Version' })).toBeNull();
		expect(screen.queryByRole('button', { name: 'Save as a new Version' })).toBeNull();
	});

	it('keeps two cookings in one visit, each saying so and neither offered again', async () => {
		const { kamosu } = renderScreen(Cooked, {
			list_attempts: {
				attempts: [
					tweaked({ id: 'at_2', created_at: '2026-08-21T18:00:00.000Z' }),
					tweaked({
						id: 'at_1',
						recipe: { branch_id: 'b_1', title: 'Miso Soup', written_yield: null },
					}),
				],
			},
			promote_as_cooked: PROMOTED,
		});
		for (const name of [/chicken katsu curry/i, /miso soup/i]) {
			await fireEvent.click(await screen.findByRole('button', { name }));
			await fireEvent.click(screen.getByRole('button', { name: 'Keep it as a new Version' }));
			await fireEvent.click(screen.getByRole('button', { name: 'Save as a new Version' }));
			expect(await screen.findByText(/Kept\. It's a new Version/)).toBeInTheDocument();
		}
		expect(
			kamosu.calls
				.filter((call) => call.operation === 'promote_as_cooked')
				.map((call) => (call.input as { attempt_id: string }).attempt_id),
		).toEqual(['at_2', 'at_1']);

		// Back on the first: it still says it was kept, and offers nothing.
		await fireEvent.click(screen.getByRole('button', { name: /chicken katsu curry/i }));
		expect(screen.getByText(/Kept\. It's a new Version/)).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Keep it as a new Version' })).toBeNull();
	});

	it('answers nothing with Not now: the offer is still there (the choice of 10 October 2026)', async () => {
		const { kamosu } = await openTweaked();

		await fireEvent.click(screen.getByRole('button', { name: 'Keep it as a new Version' }));
		await fireEvent.click(screen.getByRole('button', { name: 'Not now' }));

		expect(screen.getByRole('button', { name: 'Keep it as a new Version' })).toBeInTheDocument();
		expect(kamosu.calls.some((call) => call.operation === 'decline_promotion')).toBe(false);
		expect(kamosu.calls.some((call) => call.operation === 'promote_as_cooked')).toBe(false);
	});

	it('still offers a cooking left in the diary from the recipe page', async () => {
		await openTweaked({}, { as_cooked: { ...AS_COOKED, promotion_declined: true } });
		expect(screen.getByRole('button', { name: 'Keep it as a new Version' })).toBeInTheDocument();
	});

	it('says first that the recipe has changed since, where it has', async () => {
		await openTweaked({}, { unkept: { ...UNKEPT, moved_on: true } });
		expect(screen.queryByText(/recipe has changed since you cooked it/i)).toBeNull();

		await fireEvent.click(screen.getByRole('button', { name: 'Keep it as a new Version' }));
		expect(screen.getByText(/recipe has changed since you cooked it/i)).toBeInTheDocument();
	});

	it('starts a copy on a recipe that was sent, saying so first, and links to the copy', async () => {
		const { kamosu } = await openTweaked(
			{ promote_as_cooked: { ...PROMOTED, branch_id: 'b_copy', copied: true } },
			{ unkept: { ...UNKEPT, writes: false, arrived: true } },
		);

		await fireEvent.click(screen.getByRole('button', { name: 'Keep it as your own copy' }));
		expect(screen.getByText(/will start your own copy/i)).toBeInTheDocument();
		expect(screen.getByText(/You were sent this recipe/)).toHaveTextContent('Chicken Katsu Curry');

		await fireEvent.click(screen.getByRole('button', { name: 'Start my own copy' }));
		await vi.waitFor(() => {
			expect(kamosu.calls.some((call) => call.operation === 'promote_as_cooked')).toBe(true);
		});
		const said = await screen.findByText(/Kept, on your own copy of the recipe\./);
		expect(within(said).getByRole('link', { name: 'Open your copy' })).toHaveAttribute(
			'href',
			'/recipes/b_copy',
		);
	});

	it('shows no offer on a cooking that followed the recipe, or whose changes are kept', async () => {
		renderScreen(Cooked, {
			list_attempts: {
				attempts: [
					entry({
						id: 'at_2',
						recipe: { branch_id: 'b_2', title: 'Miso Soup', written_yield: null },
					}),
					// Changed and already a Version: the Core answers nothing unkept.
					tweaked({ unkept: null }),
				],
			},
		});
		for (const name of [/miso soup/i, /chicken katsu curry/i]) {
			await fireEvent.click(await screen.findByRole('button', { name }));
			expect(screen.queryByRole('heading', { name: 'What you changed' })).toBeNull();
			expect(screen.queryByRole('button', { name: /keep it as/i })).toBeNull();
		}
	});

	it('waits for the server in words when there is none', async () => {
		Object.defineProperty(navigator, 'onLine', { value: false, configurable: true });
		try {
			await openTweaked();
			expect(
				screen.getByRole('button', { name: /keeping it waits for the server/i }),
			).toBeDisabled();
			expect(screen.queryByRole('button', { name: 'Keep it as a new Version' })).toBeNull();
			// What was changed is on the phone and is shown all the same.
			expect(screen.getByText('2 cups panko')).toBeInTheDocument();
		} finally {
			Reflect.deleteProperty(navigator, 'onLine');
		}
	});

	it('says so and keeps the offer when the Core refuses', async () => {
		await openTweaked({ promote_as_cooked: { refuse: 'not_found' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Keep it as a new Version' }));
		await fireEvent.click(screen.getByRole('button', { name: 'Save as a new Version' }));

		expect(await screen.findByRole('alert')).toHaveTextContent(/could not keep it/i);
		expect(screen.getByRole('button', { name: 'Save as a new Version' })).toBeInTheDocument();
	});
});
