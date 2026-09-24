/**
 * The screen-seam test for a recipe's photographs (#110): the cook's own
 * cooking pictures of the dish, offered on the recipe page to become the
 * recipe's, and a Step's photograph drawn beside that Step.
 */

import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent, within } from '@testing-library/svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import RecipeTestHarness from './RecipeTestHarness.svelte';
import {
	MY_KITCHENS,
	PROMOTED,
	STEPS,
	diaryEntry,
	recipeAnswer,
	threadBranch,
} from '../../../testing/recipes';

function show(over: Answers = {}) {
	const kamosu = standIn({
		get_recipe: recipeAnswer(),
		get_thread: {
			lineage_id: 'l_1',
			branches: [threadBranch('b_1', { hand_id: 'h_1', head_version_id: 'v_1' })],
			versions: [],
			attempts: [],
		},
		list_attempts: { attempts: [] },
		list_kitchens: MY_KITCHENS,
		// What the page asks for on the side and never waits on.
		note_recipe_opened: { lineage_id: 'l_1', opened_at: '2026-09-22T00:00:00Z' },
		shopping_basis: { branch_id: 'b_1', title: 'Miso Soup', written_yield: null, lines: [] },
		get_shopping_list: { chosen: [], rows: [] },
		promote_attempt_photograph: PROMOTED,
		...over,
	});
	render(RecipeTestHarness, { props: { client: kamosu.client, branchId: 'b_1' } });
	return kamosu;
}

describe('your cooking photographs on the recipe page', () => {
	it('offers every picture from your cookings of this dish, and only this dish', async () => {
		const kamosu = show({
			list_attempts: {
				attempts: [
					diaryEntry({ id: 'at_2', photographs: ['p_second'] }),
					diaryEntry({ id: 'at_1', photographs: ['p_first', 'local:0000000000000001'] }),
					// Another dish: never on this page.
					diaryEntry({ id: 'at_9', lineage_id: 'l_other', photographs: ['p_other'] }),
				],
			},
		});
		expect(
			await screen.findByRole('heading', { name: 'Your photos from cooking this' }),
		).toBeInTheDocument();
		// Two sent pictures of this dish; the one still on the phone waits.
		const shown = screen.getAllByAltText(/^Taken /).map((image) => image.getAttribute('src'));
		expect(shown).toEqual(['/api/photographs/p_second/card', '/api/photographs/p_first/card']);

		await fireEvent.click(screen.getByRole('button', { name: 'Use one on the recipe…' }));
		const on = within(
			await screen.findByRole('dialog', { name: 'Use a cooking photo on the recipe' }),
		);
		// Several on offer, so the sheet asks which; nothing is picked for you.
		expect(await on.findByRole('heading', { name: 'Which photo' })).toBeInTheDocument();
		await fireEvent.click(on.getByRole('button', { name: /The recipe's photo/ }));
		expect(on.getByRole('button', { name: 'Use this photo' })).toBeDisabled();

		await fireEvent.click(on.getAllByRole('button', { name: /^Taken / })[1]);
		await fireEvent.click(on.getByRole('button', { name: 'Use this photo' }));
		await vi.waitFor(() =>
			expect(
				kamosu.calls.find((call) => call.operation === 'promote_attempt_photograph')?.input,
			).toEqual({
				attempt_id: 'at_1',
				photograph_id: 'p_first',
				branch_id: 'b_1',
				step_index: null,
			}),
		);
		// An ordinary save: the page says so and reads the recipe again.
		expect(await screen.findByText('Saved.')).toBeInTheDocument();
		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
		expect(kamosu.calls.filter((call) => call.operation === 'get_recipe').length).toBeGreaterThan(
			2,
		);
	});

	it('takes the caret in, gives it back, and cannot be escaped mid-save', async () => {
		let release: () => void = () => {};
		const kamosu = show({
			list_attempts: { attempts: [diaryEntry({ photographs: ['p_first'] })] },
			// A save still on its way until the test lets it land.
			promote_attempt_photograph: () => PROMOTED,
		});
		const opener = await screen.findByRole('button', { name: 'Use one on the recipe…' });
		opener.focus();
		await fireEvent.click(opener);
		const dialog = await screen.findByRole('dialog', { name: 'Use a cooking photo on the recipe' });
		expect(document.activeElement).toBe(dialog);

		// Escape with nothing in flight closes it, and the caret goes back.
		await fireEvent.keyDown(window, { key: 'Escape' });
		expect(screen.queryByRole('dialog')).toBeNull();
		await vi.waitFor(() => expect(document.activeElement).toBe(opener));

		// Mid-save, Escape and Cancel are both held off.
		const client = kamosu.client;
		const real = client.promoteAttemptPhotograph.bind(client);
		client.promoteAttemptPhotograph = (input) =>
			new Promise((done) => {
				release = () => void real(input).then(done);
			});
		await fireEvent.click(opener);
		const on = within(await screen.findByRole('dialog'));
		await fireEvent.click(await on.findByRole('button', { name: /The recipe's photo/ }));
		await fireEvent.click(on.getByRole('button', { name: 'Use this photo' }));
		expect(on.getByRole('button', { name: 'Cancel' })).toBeDisabled();
		await fireEvent.keyDown(window, { key: 'Escape' });
		expect(screen.getByRole('dialog')).toBeInTheDocument();
		release();
		expect(await screen.findByText('Saved.')).toBeInTheDocument();
	});

	it('shows nothing when you have no pictures of this dish, the ordinary case', async () => {
		const kamosu = show({ list_attempts: { attempts: [diaryEntry()] } });
		// Asked, and answered with a cooking that took no pictures.
		await vi.waitFor(() =>
			expect(kamosu.calls.some((call) => call.operation === 'list_attempts')).toBe(true),
		);
		expect(await screen.findByText('Miso Soup')).toBeInTheDocument();
		expect(screen.queryByRole('heading', { name: 'Your photos from cooking this' })).toBeNull();
		expect(screen.queryByRole('button', { name: 'Use one on the recipe…' })).toBeNull();
	});
});

describe('a cooking photo that makes a Copy (#111)', () => {
	/** The recipe on screen is written by somebody else's Cookbook, not the cook's. */
	async function openOnSomeoneElses() {
		const kamosu = show({
			get_recipe: recipeAnswer({ writes: false }),
			list_attempts: { attempts: [diaryEntry({ photographs: ['p_first'] })] },
			promote_attempt_photograph: { ...PROMOTED, branch_id: 'b_copy', copied: true },
		});
		await fireEvent.click(await screen.findByRole('button', { name: 'Use one on the recipe…' }));
		const on = within(await screen.findByRole('dialog'));
		await fireEvent.click(await on.findByRole('button', { name: /The recipe's photo/ }));
		return { kamosu, on };
	}
	const sentTo = (kamosu: ReturnType<typeof show>) =>
		kamosu.calls.find((call) => call.operation === 'promote_attempt_photograph')?.input;

	it('starts the Copy in your own Cookbook and asks nothing about where (#131)', async () => {
		const { kamosu, on } = await openOnSomeoneElses();
		expect(on.getByText(/your own Miso Soup in your Cookbook/)).toBeInTheDocument();
		expect(on.queryAllByRole('radio')).toHaveLength(0);
		await fireEvent.click(on.getByRole('button', { name: 'Start my own copy' }));
		await vi.waitFor(() => expect(sentTo(kamosu)).toMatchObject({ photograph_id: 'p_first' }));
		expect(sentTo(kamosu)).not.toHaveProperty('kitchen_id');
	});
});

describe('a Step’s photograph on the recipe page', () => {
	it('sits beside its Step as a small square, and opens across the screen', async () => {
		show();
		// Step 2 carries one (the section heading takes no number); the others do not.
		const open = await screen.findByRole('button', { name: 'Show the photograph of step 2' });
		// The square's picture says nothing of its own: the button names it.
		expect(open.querySelector('img')).toHaveAttribute('src', '/api/photographs/p_whisked/card');
		expect(screen.getAllByRole('button', { name: /Show the photograph of step/ })).toHaveLength(1);

		await fireEvent.click(open);
		const large = await screen.findByRole('dialog', { name: 'A photograph of step 2' });
		expect(within(large).getByAltText('A photograph of step 2')).toHaveAttribute(
			'src',
			'/api/photographs/p_whisked/page',
		);
		expect(document.activeElement).toBe(large);
		await fireEvent.click(within(large).getByRole('button', { name: 'Close' }));
		expect(screen.queryByRole('dialog')).toBeNull();
		// Escape puts it away too.
		await fireEvent.click(open);
		await fireEvent.keyDown(window, { key: 'Escape' });
		expect(screen.queryByRole('dialog')).toBeNull();
	});

	it('draws none on a recipe whose Steps have no photographs', async () => {
		show({ get_recipe: recipeAnswer({ steps: STEPS.map((row) => ({ ...row, photo: null })) }) });
		expect(await screen.findByText('Whisk in the miso off the heat.')).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: /Show the photograph of step/ })).toBeNull();
	});
});
