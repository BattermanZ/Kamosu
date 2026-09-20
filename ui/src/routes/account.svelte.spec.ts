import { describe, expect, it } from 'vitest';
import { screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import Account from './Account.svelte';
import { renderScreen } from '../testing/render';

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
});
