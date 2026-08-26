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
import { screen } from '@testing-library/svelte';
import Settings from './+page.svelte';
import { renderScreen } from '../../testing/render';

describe('the settings screen', () => {
	it('asks the instance what it is, and says so', async () => {
		const { kamosu } = renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true }
		});

		expect(await screen.findByText(/Version 0\.1\.0/)).toBeInTheDocument();
		expect(screen.getByText(/Setup is complete/)).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).toEqual(['instance_status']);
	});

	it('says setup has not happened when it has not', async () => {
		renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: false }
		});

		expect(await screen.findByText(/Setup has not happened yet/)).toBeInTheDocument();
	});

	it('says so plainly when the instance cannot be reached', async () => {
		renderScreen(Settings, {
			instance_status: { refuse: 'internal', message: 'Kamosu could not be reached.' }
		});

		expect(await screen.findByText(/could not be reached/)).toBeInTheDocument();
	});

	it('offers every language Paraglide compiled, with the current one pressed', () => {
		renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true }
		});

		for (const name of ['English', 'Français', 'Español']) {
			expect(screen.getByRole('button', { name })).toBeInTheDocument();
		}
		expect(screen.getByRole('button', { name: 'English' })).toHaveAttribute(
			'aria-pressed',
			'true'
		);
	});
});
