/**
 * The screen-seam test: the Operator's screen, against a stand-in Kamosu
 * (#103).
 *
 * The answers below are checked against the Catalogue before the screen sees
 * them, so this cannot quietly keep passing against a shape the Core no longer
 * serves. That is not hypothetical here: `list_backups` was declared to answer
 * a Backup's `tier` while the Core has always sent `slot`, and a test written
 * against the declaration passed while the real screen would have rendered a
 * blank. The declaration is now `slot`, and this file asserts the word.
 */

import { describe, expect, it } from 'vitest';
import { screen, fireEvent, within } from '@testing-library/svelte';
import Operator from './+page.svelte';
import { renderScreen } from '../../testing/render';
import type { Answers } from '$lib/api/stand-in';
import type { ListFoodsOutput } from '$lib/api/catalogue';

type Food = ListFoodsOutput['foods'][number];

const food = (over: Partial<Food> & Pick<Food, 'id'>): Food => ({
	name: null,
	language: null,
	names: [],
	cup_weight_grams: null,
	nutrition: null,
	reading_count: 0,
	...over,
});

const flour = food({ id: 'f_flour', name: 'flour', language: 'en', reading_count: 9 });
const farine = food({ id: 'f_farine', name: 'farine', language: 'fr', reading_count: 5 });

/**
 * An instance in the state most instances are in: two accounts, three Backups,
 * an empty worklist, and every Food in use.
 */
const quiet: Answers = {
	list_accounts: {
		accounts: [
			{
				name: 'Aurélien',
				is_operator: true,
				disabled: false,
				is_you: true,
				created_at: '2026-01-04T10:00:00Z',
			},
			{
				name: 'Camille',
				is_operator: false,
				disabled: false,
				is_you: false,
				created_at: '2026-09-21T10:00:00Z',
			},
		],
	},
	list_backups: {
		backups: [
			{
				name: 'kamosu-backup-daily-20260920T175418Z.zip',
				slot: 'daily',
				taken_at: '2026-09-20T17:54:18Z',
				size_bytes: 21_895_835,
			},
		],
	},
	list_merge_suggestions: { suggestions: [] },
	list_foods: { foods: [flour, farine] },
	get_public_address: { public_address: 'https://kamosu.example' },
};

/** The same instance, with one pair of Foods waiting to be looked at. */
const withSuggestion: Answers = {
	...quiet,
	list_merge_suggestions: {
		suggestions: [
			{
				foods: [flour, farine],
				reason: 'arrived_as_one',
				words: [
					{ language: 'en', name: 'flour' },
					{ language: 'fr', name: 'farine' },
				],
				created_at: '2026-09-18T15:20:44Z',
			},
		],
	},
};

/** Move to the Instance room, where People, Backups and the address live. */
async function openInstance() {
	await fireEvent.click(await screen.findByRole('tab', { name: 'Instance' }));
}

describe("the Operator's screen", () => {
	it('shows nothing at all to a Person who is not an Operator', async () => {
		const { kamosu } = renderScreen(Operator, {
			list_accounts: {
				refuse: 'unauthorized',
				message: 'requires a Credential naming an Operator',
			},
		});

		expect(await screen.findByText(/nothing here for you/i)).toBeInTheDocument();
		// Not merely hidden: the room is never asked about. A screen that listed
		// Backups and then declined to show them would still have asked.
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('list_backups');
		expect(screen.queryByRole('tab', { name: 'Worklist' })).not.toBeInTheDocument();
	});

	it('names who holds an account, and which one is you', async () => {
		renderScreen(Operator, quiet);
		await openInstance();

		expect(await screen.findByText('Aurélien')).toBeInTheDocument();
		expect(screen.getByText('Camille')).toBeInTheDocument();
		expect(screen.getByText('You')).toBeInTheDocument();
	});

	it('says the Operator boundary out loud, and is no window onto anyone', async () => {
		renderScreen(Operator, quiet);
		await openInstance();

		expect(await screen.findByText(/courtesy, not a wall/i)).toBeInTheDocument();

		// ADR 0007: no recipe, Attempt or Kitchen of anybody's is reachable here.
		// Asserted on what the screen actually rendered, with the screen-reader
		// text stripped first so a hidden word cannot pass for a visible one.
		const shown = document.body.cloneNode(true) as HTMLElement;
		for (const hidden of shown.querySelectorAll('.sr-only')) hidden.remove();
		const words = (shown.textContent ?? '').toLowerCase();
		for (const forbidden of ['recipe', 'attempt', 'kitchen']) {
			expect(words).not.toContain(forbidden);
		}
	});

	it('shows the empty worklist, which is where most instances live', async () => {
		renderScreen(Operator, quiet);

		expect(await screen.findByText(/nothing to look at/i)).toBeInTheDocument();
		expect(screen.getByText(/never joins two Foods by itself/i)).toBeInTheDocument();
		// Every Food here is in use, so none is offered for deleting.
		expect(screen.getByText(/is used by a recipe/i)).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Delete' })).not.toBeInTheDocument();
	});

	it('announces what a Merge will move before it moves it, and says that figure back', async () => {
		const { kamosu } = renderScreen(Operator, {
			...withSuggestion,
			preview_food_merge: {
				survivor: flour,
				absorbed: farine,
				ingredient_lines: 14,
				readings: 20,
				cup_weight_conflict: false,
			},
			merge_food: { food: flour, ingredient_lines: 14, readings: 20 },
		});

		expect(await screen.findByText(/arrived as one recipe/i)).toBeInTheDocument();
		await fireEvent.click(await screen.findByRole('button', { name: 'Keep flour' }));

		const sheet = await screen.findByRole('dialog');
		expect(within(sheet).getByText('14')).toBeInTheDocument();
		expect(
			within(sheet).getByText(/Ingredient Lines move from farine to flour/),
		).toBeInTheDocument();
		expect(within(sheet).getByText(/no un-merge/i)).toBeInTheDocument();

		// Nothing has moved yet: the announcement is the whole point.
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('merge_food');

		await fireEvent.click(within(sheet).getByRole('button', { name: 'Join them' }));
		const merge = kamosu.calls.find((call) => call.operation === 'merge_food');
		expect(merge?.input).toMatchObject({
			survivor_food_id: 'f_flour',
			absorbed_food_id: 'f_farine',
			// The figure the preview announced, said back — the safety net, not a
			// formality. Recomputing it here would defeat what it is for.
			ingredient_lines: 14,
		});
	});

	it('tells two Foods apart when they carry the same name', async () => {
		// Found in live acceptance: a Merge gives the survivor every name both
		// Foods held, so the next suggestion naming it read "liveflour ·
		// liveflour" over two identical Keep buttons, and neither said which
		// Food it kept. ADR 0022 makes doubt produce a new Food rather than a
		// merge, so same-named Foods are ordinary, not pathological.
		const english = food({ id: 'f_en', name: 'flour', language: 'en' });
		const french = food({ id: 'f_fr', name: 'flour', language: 'fr' });
		renderScreen(Operator, {
			...quiet,
			list_merge_suggestions: {
				suggestions: [
					{
						foods: [english, french],
						reason: 'name_typed_onto_another',
						words: [{ language: 'en', name: 'flour' }],
						created_at: '2026-09-18T15:20:44Z',
					},
				],
			},
		});

		expect(await screen.findByRole('button', { name: 'Keep flour (en)' })).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Keep flour (fr)' })).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Keep flour' })).not.toBeInTheDocument();
	});

	it('still tells them apart when even the language is the same', async () => {
		// The case the first fix missed, caught in live acceptance: both Foods
		// on the dev corpus were English, so naming the language printed the
		// identical label twice. The id always differs.
		renderScreen(Operator, {
			...quiet,
			list_merge_suggestions: {
				suggestions: [
					{
						foods: [
							food({ id: 'f_aaaaaaaa', name: 'flour', language: 'en' }),
							food({ id: 'f_bbbbbbbb', name: 'flour', language: 'en' }),
						],
						reason: 'name_typed_onto_another',
						words: [{ language: 'en', name: 'flour' }],
						created_at: '2026-09-18T15:20:44Z',
					},
				],
			},
		});

		const keeps = await screen.findAllByRole('button', { name: /^Keep flour/ });
		expect(keeps).toHaveLength(2);
		const labels = keeps.map((button) => button.textContent?.trim());
		expect(new Set(labels).size).toBe(2);
	});

	it('asks before deleting an account, and offers the reversible act beside it', async () => {
		const { kamosu } = renderScreen(Operator, { ...quiet, delete_account: { deleted: true } });
		await openInstance();

		const camille = (await screen.findByText('Camille')).closest('li') as HTMLElement;
		await fireEvent.click(within(camille).getByRole('button', { name: 'Delete' }));

		const sheet = await screen.findByRole('dialog');
		expect(within(sheet).getByText('Delete Camille?')).toBeInTheDocument();
		expect(within(sheet).getByText(/can no longer sign in/i)).toBeInTheDocument();
		expect(within(sheet).getByText(/cannot be undone/i)).toBeInTheDocument();
		expect(within(sheet).getByRole('button', { name: 'Disable' })).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('delete_account');

		await fireEvent.click(within(sheet).getByRole('button', { name: 'Delete the account' }));
		expect(kamosu.calls.find((call) => call.operation === 'delete_account')?.input).toMatchObject({
			name: 'Camille',
		});
	});

	it("shows the Core's refusal when the last Operator is the one being ended", async () => {
		const refusal =
			"'Aurélien' is the only Operator this instance has, and deleting the account would leave it with nobody able to administer it and no way to appoint anybody. Make somebody else an Operator first.";
		renderScreen(Operator, {
			...quiet,
			delete_account: { refuse: 'bad_request', message: refusal },
		});
		await openInstance();

		const you = (await screen.findByText('Aurélien')).closest('li') as HTMLElement;
		await fireEvent.click(within(you).getByRole('button', { name: 'Delete' }));
		const sheet = await screen.findByRole('dialog');
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Delete the account' }));

		// In the words it arrived in: the reason is the useful half, and a
		// paraphrase here would keep only the fact.
		expect(await within(sheet).findByText(/only Operator this instance has/)).toBeInTheDocument();
		expect(within(sheet).getByText(/Make somebody else an Operator first/)).toBeInTheDocument();
	});

	it('shows an Invite once, and says it will not be shown again', async () => {
		renderScreen(Operator, { ...quiet, mint_invite: { link: '/invite/8f2c1a94e07b' } });
		await openInstance();

		await fireEvent.click(await screen.findByRole('button', { name: 'Invite someone' }));
		expect(await screen.findByText('/invite/8f2c1a94e07b')).toBeInTheDocument();
		expect(screen.getByText(/shown once and never again/i)).toBeInTheDocument();
	});

	it('raises a Person to Operator and stands one down, by name', async () => {
		const { kamosu } = renderScreen(Operator, {
			...quiet,
			set_operator: { name: 'Camille', is_operator: true },
		});
		await openInstance();

		const camille = (await screen.findByText('Camille')).closest('li') as HTMLElement;
		await fireEvent.click(within(camille).getByRole('button', { name: 'Make an Operator' }));
		expect(kamosu.calls.find((call) => call.operation === 'set_operator')?.input).toEqual({
			name: 'Camille',
			is_operator: true,
		});

		// Somebody who already administers is offered the other direction.
		const you = screen.getByText('Aurélien').closest('li') as HTMLElement;
		expect(within(you).getByRole('button', { name: 'Stand down' })).toBeInTheDocument();
	});

	it('mints an Invite that carries whether they arrive as an Operator', async () => {
		const { kamosu } = renderScreen(Operator, {
			...quiet,
			mint_invite: { link: '/invite/8f2c1a94e07b' },
		});
		await openInstance();

		await fireEvent.click(
			await screen.findByRole('checkbox', { name: 'They arrive as an Operator' }),
		);
		await fireEvent.click(screen.getByRole('button', { name: 'Invite someone' }));
		expect(kamosu.calls.find((call) => call.operation === 'mint_invite')?.input).toEqual({
			is_operator: true,
		});
	});

	it('asks before sweeping, because a swept Photograph does not come back', async () => {
		const { kamosu } = renderScreen(Operator, {
			...quiet,
			sweep_photographs: {
				referenced: 65,
				newly_unreferenced: 0,
				back_in_use: 0,
				swept: 2,
				swept_photograph_ids: ['ph_1', 'ph_2'],
			},
		});

		await fireEvent.click(await screen.findByRole('button', { name: 'Sweep now' }));
		const sheet = await screen.findByRole('dialog');
		expect(within(sheet).getByText(/cannot be undone/i)).toBeInTheDocument();
		expect(within(sheet).getByText(/never touched/i)).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('sweep_photographs');

		await fireEvent.click(within(sheet).getByRole('button', { name: 'Sweep now' }));
		expect(kamosu.calls.map((call) => call.operation)).toContain('sweep_photographs');
		expect(await screen.findByText(/Swept 2\. 65 are still in use\./)).toBeInTheDocument();
	});

	it('does not mistake a broken Kamosu for a room that is not yours', async () => {
		renderScreen(Operator, {
			...quiet,
			list_accounts: { refuse: 'internal', message: 'the database is unreadable' },
		});

		// An Operator whose instance is failing is owed the reason, not an
		// empty room that says the powers were never theirs.
		expect(await screen.findByText('the database is unreadable')).toBeInTheDocument();
		expect(screen.queryByText(/nothing here for you/i)).not.toBeInTheDocument();
	});

	it('lists the Backups that exist, each by its slot', async () => {
		renderScreen(Operator, quiet);
		await openInstance();

		// `slot`, the word the Core has always sent. This assertion is what the
		// declaration's old `tier` would now fail on.
		expect(await screen.findByText(/daily · 21\.9 MB/)).toBeInTheDocument();
		expect(screen.getByRole('link', { name: 'Get' })).toHaveAttribute(
			'href',
			'/api/backups/kamosu-backup-daily-20260920T175418Z.zip',
		);
	});

	it('tells the truth about what changing the public address reaches', async () => {
		renderScreen(Operator, quiet);
		await openInstance();

		expect(await screen.findByText('https://kamosu.example')).toBeInTheDocument();
		const warning = screen.getByText(/Links you have already sent/);
		expect(warning).toHaveTextContent(/keep pointing at the old one/);
		expect(warning).toHaveTextContent(/cannot reissue them/);
		// The false promise this ticket corrected must not come back.
		expect(warning.textContent ?? '').not.toMatch(/follow/i);
	});
});
