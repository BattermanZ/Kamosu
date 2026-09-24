import { afterEach, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import Account from './Account.svelte';
import { renderScreen } from '../testing/render';
import Harness from '../testing/Harness.svelte';
import { standIn } from '$lib/api/stand-in';
import type { AuthClient } from '$lib/auth';
import { OperationError } from '$lib/api/client';
import { went } from '../testing/navigation';
import LinkTestHarness from './LinkTestHarness.svelte';
import InviteRoute from './invite/[secret]/+page.svelte';
import RecoverRoute from './recover/[secret]/+page.svelte';

describe('the account screen', () => {
	const user = userEvent.setup();
	it('offers the first visitor account creation', async () => {
		renderScreen(Account, {
			instance_status: { version: '0.1.0', setup_complete: false, password_minimum: 15 },
		});

		expect(await screen.findByRole('heading', { name: 'Set up Kamosu' })).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Create my account' })).toBeInTheDocument();
		expect(screen.getByLabelText('Name')).toBeInTheDocument();
		expect(screen.getByLabelText('Password')).toHaveAttribute('type', 'password');
	});

	it('offers login once the first person exists', async () => {
		renderScreen(Account, {
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
		});

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
					: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
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
					: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
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
					client: standIn({
						instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
					}).client,
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

	// A password being set has a minimum, said before anything is typed; a
	// wrong one at login makes the name wait, and the screen counts it down
	// (#138, choices A and A).
	describe('the password minimum and the wait (#138)', () => {
		afterEach(() => vi.useRealTimers());

		function signInWith(setupComplete: boolean, authenticate: AuthClient['authenticate']) {
			render(Harness, {
				props: {
					component: Account,
					client: standIn({
						instance_status: {
							version: '0.1.0',
							setup_complete: setupComplete,
							password_minimum: 15,
						},
					}).client,
					auth: { authenticate },
				},
			});
		}

		it('says the minimum under the password while one is being set', async () => {
			signInWith(false, async () => {});

			const password = await screen.findByLabelText('Password');
			expect(password).toHaveAccessibleDescription('At least 15 characters.');
		});

		it('says nothing about length at login', async () => {
			signInWith(true, async () => {});

			const password = await screen.findByLabelText('Password');
			expect(password).not.toHaveAccessibleDescription();
			expect(screen.queryByText('At least 15 characters.')).not.toBeInTheDocument();
		});

		it('refuses a password that is too short in words, without asking Kamosu', async () => {
			const authenticate = vi.fn<AuthClient['authenticate']>(async () => {});
			signInWith(false, authenticate);

			await user.type(await screen.findByLabelText('Name'), 'Aurélien');
			await user.type(screen.getByLabelText('Password'), 'fourteen chars');
			await user.click(screen.getByRole('button', { name: 'Create my account' }));

			expect(screen.getByRole('alert')).toHaveTextContent(
				'Your new password needs at least 15 characters.',
			);
			expect(authenticate).not.toHaveBeenCalled();
		});

		it('counts the wait down, and takes the line away at zero', async () => {
			vi.useFakeTimers({ shouldAdvanceTime: true });
			signInWith(true, async () => {
				throw new OperationError(
					'authentication',
					'busy',
					'too many wrong passwords for this name — try again in 2 s',
					{ retryAfterSeconds: 2 },
				);
			});

			await user.type(await screen.findByLabelText('Name'), 'Aurélien');
			await user.type(screen.getByLabelText('Password'), 'a guess');
			await user.click(screen.getByRole('button', { name: 'Log in' }));

			expect(await screen.findByRole('alert')).toHaveTextContent(
				'Too many wrong passwords for this name. Try again in 2 s.',
			);
			// Nothing was checked, so nothing typed is thrown away.
			expect(screen.getByLabelText('Password')).toHaveValue('a guess');
			await vi.advanceTimersByTimeAsync(1000);
			expect(screen.getByRole('alert')).toHaveTextContent('Try again in 1 s.');
			await vi.advanceTimersByTimeAsync(1000);
			expect(screen.queryByRole('alert')).not.toBeInTheDocument();
		});
	});

	// An Invite and a recovery link each open a page of their own. Until #126
	// neither existed, and both addresses opened Not Found: the form knew what to
	// do with a link, but no route ever showed it one.
	describe('opened from a link (#126)', () => {
		/** Open `route` with `secret`, signing in through `authenticate`. */
		function open(
			route: typeof InviteRoute | typeof RecoverRoute,
			secret: string,
			authenticate: AuthClient['authenticate'],
		) {
			render(LinkTestHarness, {
				props: {
					route,
					secret,
					client: standIn({
						instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
					}).client,
					auth: { authenticate },
				},
			});
		}

		it('shows an Invite link the Invite form, and goes Home once it is accepted', async () => {
			const authenticate = vi.fn<AuthClient['authenticate']>(async () => {});
			open(InviteRoute, '8f2c1a94e07b', authenticate);

			expect(await screen.findByRole('heading', { name: 'Join Kamosu' })).toBeInTheDocument();
			await user.type(screen.getByLabelText('Name'), 'Camille');
			await user.type(screen.getByLabelText('Password'), 'a password long enough');
			await user.click(screen.getByRole('button', { name: 'Create my account' }));

			// The whole path, prefix and all: the Core strips `/invite/` itself.
			expect(authenticate).toHaveBeenCalledWith(
				'invite',
				expect.objectContaining({
					link: '/invite/8f2c1a94e07b',
					name: 'Camille',
					password: 'a password long enough',
				}),
			);
			expect(went).toHaveBeenCalledWith('/');
		});

		it('shows a recovery link the recovery form, and goes Home once it is used', async () => {
			const authenticate = vi.fn<AuthClient['authenticate']>(async () => {});
			open(RecoverRoute, '3b71d0ae5c92', authenticate);

			expect(
				await screen.findByRole('heading', { name: 'Choose a new password' }),
			).toBeInTheDocument();
			// The link already says whose password this is.
			expect(screen.queryByLabelText('Name')).not.toBeInTheDocument();
			await user.type(screen.getByLabelText('Password'), 'a new password, long enough');
			await user.click(screen.getByRole('button', { name: 'Set new password' }));

			expect(authenticate).toHaveBeenCalledWith(
				'recover',
				expect.objectContaining({
					link: '/recover/3b71d0ae5c92',
					password: 'a new password, long enough',
				}),
			);
			expect(authenticate).not.toHaveBeenCalledWith(
				'recover',
				expect.objectContaining({ name: expect.anything() }),
			);
			expect(went).toHaveBeenCalledWith('/');
		});

		// Whether a link is still good is learned by using it, not on arrival
		// (decided in triage): the Core's refusal is what the person reads.
		it('says in words that a link is spent, and stays on the form', async () => {
			open(InviteRoute, '8f2c1a94e07b', async () => {
				throw new OperationError(
					'authentication',
					'unauthorized',
					'this Invite has already been spent or revoked',
				);
			});

			await user.type(await screen.findByLabelText('Name'), 'Camille');
			await user.type(screen.getByLabelText('Password'), 'a password long enough');
			await user.click(screen.getByRole('button', { name: 'Create my account' }));

			expect(await screen.findByRole('alert')).toHaveTextContent(
				'this Invite has already been spent or revoked',
			);
			expect(screen.getByRole('heading', { name: 'Join Kamosu' })).toBeInTheDocument();
			expect(went).not.toHaveBeenCalled();
		});
	});
});
