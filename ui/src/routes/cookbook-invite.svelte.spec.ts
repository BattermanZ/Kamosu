/**
 * The page a Cookbook Invite link opens (#131, screen choice 2): what saying
 * yes would do, said before anything happens, and nothing done until *Write
 * together* is pressed.
 */

import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import type { AuthClient } from '$lib/auth';
import { went } from '../testing/navigation';
import LinkTestHarness from './LinkTestHarness.svelte';
import CookbookInviteRoute from './cookbook-invite/[secret]/+page.svelte';

const ALREADY = {
	id: 'c_stephane',
	name: null,
	authors: [{ person_id: 'p_1', name: 'Stéphane' }],
	recipe_count: 87,
	kitchens: [],
	invites: [],
	joins: [],
};

function open(answers: Answers) {
	const kamosu = standIn({
		instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
		...answers,
	});
	const authenticate = vi.fn<AuthClient['authenticate']>(async () => {});
	render(LinkTestHarness, {
		props: {
			route: CookbookInviteRoute,
			secret: 'the-cookbook-secret',
			client: kamosu.client,
			auth: { authenticate },
		},
	});
	return { kamosu, authenticate };
}

describe('a Cookbook Invite link', () => {
	it('says whose Cookbook it is and how many recipes become one, doing nothing yet', async () => {
		const { kamosu } = open({
			read_cookbook_invite: {
				cookbook: ALREADY,
				invited_by: { person_id: 'p_1', name: 'Stéphane' },
				their_recipes: 87,
				your_recipes: 12,
				together_recipes: 99,
				already_yours: false,
				asks: [],
				waiting: false,
			},
			accept_cookbook_invite: { ...ALREADY, recipe_count: 99 },
		});

		expect(
			await screen.findByRole('heading', {
				name: 'Stéphane asks you to write one Cookbook together',
			}),
		).toBeInTheDocument();
		expect(
			screen.getByText(
				'Your 12 recipes join their 87. Either of you can then change all 99, and your Kitchens see them.',
			),
		).toBeInTheDocument();
		expect(
			screen.getByText('You can leave any time, with a copy of every recipe.'),
		).toBeInTheDocument();
		expect(kamosu.calls).toContainEqual({
			operation: 'read_cookbook_invite',
			input: { secret: 'the-cookbook-secret' },
		});
		// Opening the link spends nothing.
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('accept_cookbook_invite');

		await fireEvent.click(screen.getByRole('button', { name: 'Write together' }));
		await waitFor(() =>
			expect(kamosu.calls).toContainEqual({
				operation: 'accept_cookbook_invite',
				input: { secret: 'the-cookbook-secret' },
			}),
		);
		await waitFor(() => expect(went).toHaveBeenCalledWith('/settings/kitchens'));
	});

	it('names who else will be asked, and once accepted says whom it waits for (#135)', async () => {
		let accepted = false;
		const asks = [
			{ person_id: 'p_4', name: 'Tom' },
			{ person_id: 'p_5', name: 'Bob' },
		];
		const { kamosu } = open({
			read_cookbook_invite: () => ({
				cookbook: ALREADY,
				invited_by: { person_id: 'p_1', name: 'Stéphane' },
				their_recipes: 87,
				your_recipes: 12,
				together_recipes: 99,
				already_yours: false,
				asks,
				waiting: accepted,
			}),
			accept_cookbook_invite: () => {
				accepted = true;
				return {
					...ALREADY,
					id: 'c_camille',
					joins: [
						{
							join_id: 'cj_1',
							state: 'waiting',
							accepted_by: { person_id: 'p_2', name: 'Camille' },
							invited_by: { person_id: 'p_1', name: 'Stéphane' },
							joining: { id: 'c_camille', name: null, authors: [] },
							into: { id: 'c_stephane', name: null, authors: ALREADY.authors },
							together_recipes: 99,
							waiting_on: asks,
							you: 'accepted',
							refused_by: null,
							refused_by_co_author: false,
						},
					],
				};
			},
		});

		expect(
			await screen.findByText(
				'Tom and Bob will be asked too. Nothing changes until everyone says yes.',
			),
		).toBeInTheDocument();
		expect(
			screen.getByText(
				'Your 12 recipes join their 87. Any of you can then change all 99, and your Kitchens see them.',
			),
		).toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: 'Write together' }));
		expect(
			await screen.findByText('You said yes. Waiting for Tom and Bob to say yes too.'),
		).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Write together' })).not.toBeInTheDocument();
		expect(went).not.toHaveBeenCalledWith('/settings/kitchens');
		expect(kamosu.calls.filter((call) => call.operation === 'read_cookbook_invite')).toHaveLength(
			2,
		);
	});

	it('leaves the link unspent on Not now', async () => {
		const { kamosu } = open({
			read_cookbook_invite: {
				cookbook: ALREADY,
				invited_by: { person_id: 'p_1', name: 'Stéphane' },
				their_recipes: 87,
				your_recipes: 12,
				together_recipes: 99,
				already_yours: false,
				asks: [],
				waiting: false,
			},
		});
		const later = await screen.findByRole('link', { name: 'Not now' });
		expect(later).toHaveAttribute('href', '/');
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('accept_cookbook_invite');
	});

	it('says a used or ended link is gone, and asks for a new one', async () => {
		// Refused as a stranger is refused, so the page asks whether you are
		// signed in before it offers the sign-in form.
		open({ read_cookbook_invite: { refuse: 'unauthorized' }, get_cookbook: ALREADY });
		expect(
			await screen.findByText('This invite has been used or ended. Ask for a new one.'),
		).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Write together' })).not.toBeInTheDocument();
	});

	it('says so when the reader already writes that Cookbook', async () => {
		open({
			read_cookbook_invite: {
				cookbook: ALREADY,
				invited_by: { person_id: 'p_1', name: 'Stéphane' },
				their_recipes: 87,
				your_recipes: 87,
				together_recipes: 87,
				already_yours: true,
				asks: [],
				waiting: false,
			},
		});
		expect(await screen.findByText('You already write this Cookbook.')).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Write together' })).not.toBeInTheDocument();
	});

	it('signs a stranger in first, then asks the same question', async () => {
		let signedIn = false;
		const { authenticate } = open({
			read_cookbook_invite: () =>
				signedIn
					? {
							cookbook: ALREADY,
							invited_by: { person_id: 'p_1', name: 'Stéphane' },
							their_recipes: 87,
							your_recipes: 12,
							together_recipes: 99,
							already_yours: false,
							asks: [],
							waiting: false,
						}
					: { refuse: 'unauthorized' },
			get_cookbook: () => (signedIn ? ALREADY : { refuse: 'unauthorized' }),
		});
		authenticate.mockImplementation(async () => {
			signedIn = true;
		});

		expect(await screen.findByRole('heading', { name: 'Welcome back' })).toBeInTheDocument();
		await fireEvent.input(screen.getByLabelText('Name'), { target: { value: 'Camille' } });
		await fireEvent.input(screen.getByLabelText('Password'), { target: { value: 'a password' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Log in' }));

		expect(
			await screen.findByRole('heading', {
				name: 'Stéphane asks you to write one Cookbook together',
			}),
		).toBeInTheDocument();
	});
});
