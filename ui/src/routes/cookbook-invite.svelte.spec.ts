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
	id: 'c_aurelien',
	name: null,
	authors: [{ person_id: 'p_1', name: 'Aurélien' }],
	recipe_count: 87,
	kitchens: [],
	invites: [],
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
				their_recipes: 87,
				your_recipes: 12,
				together_recipes: 99,
				already_yours: false,
			},
			accept_cookbook_invite: { ...ALREADY, recipe_count: 99 },
		});

		expect(
			await screen.findByRole('heading', {
				name: 'Aurélien asks you to write one Cookbook together',
			}),
		).toBeInTheDocument();
		expect(
			screen.getByText(
				'Your 12 recipes join their 87. From then on either of you can change any of the 99, and every Kitchen either of you cooks in sees them.',
			),
		).toBeInTheDocument();
		expect(
			screen.getByText(
				'You can leave whenever you like, and you take your own copy of every recipe with you.',
			),
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
		await waitFor(() => expect(went).toHaveBeenCalledWith('/settings'));
	});

	it('leaves the link unspent on Not now', async () => {
		const { kamosu } = open({
			read_cookbook_invite: {
				cookbook: ALREADY,
				their_recipes: 87,
				your_recipes: 12,
				together_recipes: 99,
				already_yours: false,
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
				their_recipes: 87,
				your_recipes: 87,
				together_recipes: 87,
				already_yours: true,
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
							their_recipes: 87,
							your_recipes: 12,
							together_recipes: 99,
							already_yours: false,
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
				name: 'Aurélien asks you to write one Cookbook together',
			}),
		).toBeInTheDocument();
	});
});
