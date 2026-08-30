import { describe, expect, it } from 'vitest';
import { screen } from '@testing-library/svelte';
import Account from './Account.svelte';
import { renderScreen } from '../testing/render';

describe('the account screen', () => {
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
});
