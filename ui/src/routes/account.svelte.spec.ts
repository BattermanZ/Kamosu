import { afterEach, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import Account from './Account.svelte';
import { renderScreen } from '../testing/render';
import Harness from '../testing/Harness.svelte';
import { standIn } from '$lib/api/stand-in';
import type { AuthClient } from '$lib/auth';

describe('the account screen', () => {
	const user = userEvent.setup();
	it('offers the first visitor account creation', async () => {
		renderScreen(Account, { instance_status: { version: '0.1.0', setup_complete: false } });

		expect(await screen.findByRole('heading', { name: 'Set up Kamosu' })).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Create my account' })).toBeInTheDocument();
		expect(screen.getByLabelText('Name')).toBeInTheDocument();
		expect(screen.getByLabelText('Password')).toHaveAttribute('type', 'password');
	});

	it('offers login once the first person exists', async () => {
		renderScreen(Account, { instance_status: { version: '0.1.0', setup_complete: true } });

		expect(await screen.findByRole('heading', { name: 'Welcome back' })).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Log in' })).toBeInTheDocument();
	});

	// A browser whose Session has ended keeps presenting the dead cookie, and
	// that is what makes even this Public Operation come back refused. The
	// refusal expires the cookie, so the second ask is answered (#91).
	it('asks again when a dead Session refuses the first ask, and reaches the form', async () => {
		let asked = 0;
		const { kamosu } = renderScreen(Account, {
			instance_status: () =>
				++asked === 1
					? { refuse: 'unauthorized', message: 'this Credential does not name anyone' }
					: { version: '0.1.0', setup_complete: true },
		});

		expect(await screen.findByRole('heading', { name: 'Welcome back' })).toBeInTheDocument();
		expect(kamosu.calls.filter((call) => call.operation === 'instance_status')).toHaveLength(2);
	});

	// The bug this screen had was silence: it caught the refusal into a variable
	// it only rendered in the other branch, and waited on the loading title
	// forever. It never waits without saying so now.
	it('says so, rather than waiting, when it cannot learn the instance state', async () => {
		renderScreen(Account, {
			instance_status: { refuse: 'internal', message: 'the database is unreadable' },
		});

		expect(
			await screen.findByRole('heading', { name: 'Kamosu could not be reached.' }),
		).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
		expect(screen.queryByRole('heading', { name: 'Loading Kamosu…' })).not.toBeInTheDocument();
	});

	// Two refusals in a row mean the first one did not fix anything. It gives up
	// rather than asking forever.
	it('gives up after one retry when the refusal keeps coming back', async () => {
		const { kamosu } = renderScreen(Account, {
			instance_status: { refuse: 'unauthorized', message: 'this Credential does not name anyone' },
		});

		expect(
			await screen.findByRole('heading', { name: 'Kamosu could not be reached.' }),
		).toBeInTheDocument();
		expect(kamosu.calls.filter((call) => call.operation === 'instance_status')).toHaveLength(2);
	});

	// Pressing it asks again from scratch, which is the whole of the way out
	// when the first pair of asks happened while the kitchen had no signal.
	it('asks again when Try again is pressed', async () => {
		let asked = 0;
		const { kamosu } = renderScreen(Account, {
			instance_status: () =>
				++asked === 1
					? { refuse: 'internal', message: 'the database is unreadable' }
					: { version: '0.1.0', setup_complete: true },
		});

		await user.click(await screen.findByRole('button', { name: 'Try again' }));

		expect(await screen.findByRole('heading', { name: 'Welcome back' })).toBeInTheDocument();
		// Once on arrival, once for the press. An `internal` refusal is not the
		// one worth asking twice for, so neither ask was doubled.
		expect(kamosu.calls.filter((call) => call.operation === 'instance_status')).toHaveLength(2);
	});

	// Whatever went wrong, the one thing this screen must never do again is go
	// quiet: an error that is not a refusal at all still reaches a person.
	it('says so even when what went wrong is not a refusal', async () => {
		renderScreen(Account, {
			instance_status: () => {
				throw new TypeError('the generated client could not read the answer');
			},
		});

		expect(
			await screen.findByRole('heading', { name: 'Kamosu could not be reached.' }),
		).toBeInTheDocument();
	});

	describe('naming the Session it signs in (#114)', () => {
		afterEach(() => vi.unstubAllGlobals());

		/** Sign in on a browser that describes itself as `userAgent`. */
		async function signInAs(userAgent: string, maxTouchPoints: number) {
			vi.stubGlobal('navigator', { ...navigator, userAgent, maxTouchPoints });
			const authenticate = vi.fn<AuthClient['authenticate']>(async () => {});
			render(Harness, {
				props: {
					component: Account,
					client: standIn({ instance_status: { version: '0.1.0', setup_complete: true } }).client,
					auth: { authenticate },
				},
			});
			await user.type(await screen.findByLabelText('Name'), 'Aurélien');
			await user.type(screen.getByLabelText('Password'), 'a password');
			await user.click(screen.getByRole('button', { name: 'Log in' }));
			return authenticate.mock.calls[0][1].session_name;
		}

		it('names it for the browser and device it is on', async () => {
			expect(
				await signInAs(
					'Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.5 Mobile/15E148 Safari/604.1',
					5,
				),
			).toBe('Safari · iPhone');
		});

		it('still names it something when the browser says nothing useful', async () => {
			expect(await signInAs('curl/8.5.0', 0)).toBe('A browser');
		});
	});
});
