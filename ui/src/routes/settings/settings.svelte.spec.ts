/**
 * The screen-seam test: Settings, against a stand-in Kamosu.
 *
 * What makes this worth having is not that it runs — it is that the answers
 * below are checked against the Catalogue before the screen sees them. Rename
 * `setup_complete` in `src/catalogue.rs` and this test fails, loudly, in the
 * same commit; it cannot quietly keep passing against a shape the Core no
 * longer serves.
 */

import { describe, expect, it } from 'vitest';
import { screen, fireEvent, within } from '@testing-library/svelte';
import Settings from './+page.svelte';
import { renderScreen } from '../../testing/render';
import type { Answers } from '$lib/api/stand-in';

/** Every screen visit asks for Sessions and Access Keys beside instance_status
 * (ADR 0031). Tests unconcerned with Access answer both as an anonymous
 * visitor would be refused — the same shape a stranger meets at either Door. */
const anonymous: Answers = {
	list_sessions: { refuse: 'unauthorized' },
	list_access_keys: { refuse: 'unauthorized' }
};

describe('the settings screen', () => {
	it('asks the instance what it is, and says so', async () => {
		const { kamosu } = renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true },
			...anonymous
		});

		expect(await screen.findByText(/Version 0\.1\.0/)).toBeInTheDocument();
		expect(screen.getByText(/Setup is complete/)).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).toContain('instance_status');
	});

	it('says setup has not happened when it has not', async () => {
		renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: false },
			...anonymous
		});

		expect(await screen.findByText(/Setup has not happened yet/)).toBeInTheDocument();
	});

	it('says so plainly when the instance cannot be reached', async () => {
		renderScreen(Settings, {
			instance_status: { refuse: 'internal', message: 'Kamosu could not be reached.' },
			...anonymous
		});

		expect(await screen.findByText(/could not be reached/)).toBeInTheDocument();
	});

	it('offers every language Paraglide compiled, with the current one pressed', () => {
		renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true },
			...anonymous
		});

		for (const name of ['English', 'Français', 'Español']) {
			expect(screen.getByRole('button', { name })).toBeInTheDocument();
		}
		expect(screen.getByRole('button', { name: 'English' })).toHaveAttribute(
			'aria-pressed',
			'true'
		);
	});

	it('shows nothing about Access when the visitor is not signed in', async () => {
		renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true },
			...anonymous
		});

		await screen.findByText(/Version 0\.1\.0/);
		expect(screen.queryByText('Access')).not.toBeInTheDocument();
	});

	it('lists a signed-in Person\'s Sessions and Access Keys together, each ending on its own', async () => {
		const { kamosu } = renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true },
			list_sessions: {
				sessions: [
					{ id: 's_1', name: 'iPhone', created_at: '2026-01-01T00:00:00Z', last_used_at: null, revoked: false }
				]
			},
			list_access_keys: {
				access_keys: [
					{
						id: 'ak_1',
						name: 'my agent',
						read_only: true,
						created_at: '2026-01-01T00:00:00Z',
						last_used_at: null,
						revoked: false
					}
				]
			},
			revoke_session: { revoked: true },
			revoke_access_key: { revoked: true }
		});

		expect(await screen.findByText('iPhone')).toBeInTheDocument();
		expect(screen.getByText('my agent')).toBeInTheDocument();
		expect(screen.getByText('Read-only')).toBeInTheDocument();

		const sessionRow = screen.getByText('iPhone').closest('li');
		await fireEvent.click(within(sessionRow as HTMLElement).getByRole('button', { name: 'End' }));
		expect(kamosu.calls.map((call) => call.operation)).toContain('revoke_session');

		const keyRow = screen.getByText('my agent').closest('li');
		await fireEvent.click(within(keyRow as HTMLElement).getByRole('button', { name: 'End' }));
		expect(kamosu.calls.map((call) => call.operation)).toContain('revoke_access_key');
	});

	it('shows a minted Access Key\'s secret once, then never again', async () => {
		const { kamosu } = renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true },
			list_sessions: { sessions: [] },
			list_access_keys: { access_keys: [] },
			mint_access_key: {
				id: 'ak_new',
				name: 'a new agent',
				read_only: false,
				secret: 'the-one-time-secret'
			}
		});

		await screen.findByText('No Access Keys yet.');
		await fireEvent.input(screen.getByLabelText('Name'), { target: { value: 'a new agent' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Create Access Key' }));

		expect(await screen.findByText('the-one-time-secret')).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).toContain('mint_access_key');

		await fireEvent.click(screen.getByRole('button', { name: "Done, I copied it" }));
		expect(screen.queryByText('the-one-time-secret')).not.toBeInTheDocument();
	});
});
