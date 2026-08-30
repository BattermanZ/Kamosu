/**
 * The screen-seam test: the share screen, against a stand-in Kamosu. The
 * answers below are checked against the Catalogue before the screen sees them
 * — a field renamed in `src/catalogue.rs` fails this test in the same commit.
 */

import { describe, expect, it } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import ShareTestHarness from './ShareTestHarness.svelte';

function renderShare(answers: Answers) {
	const kamosu = standIn(answers);
	render(ShareTestHarness, { props: { client: kamosu.client, branchId: 'b_1' } });
	return { kamosu };
}

const NOT_SHARED = {
	shared: false,
	share_id: null,
	url: null,
	shared_by: null,
	created_at: null,
	public_address: null,
};

describe('the share screen', () => {
	it('says what a share carries and what ending one cannot reach, before anything is shared', async () => {
		renderShare({ get_share_link: NOT_SHARED });

		// The standing line is there from the first frame: not a dialog, nothing
		// to dismiss, and no first-time special case (ADR 0018).
		expect(await screen.findByText(/It cannot reach a copy already sent\./)).toBeInTheDocument();
		expect(screen.getByText(/back to its first version/)).toBeInTheDocument();
		// The standing line is on screen before the link's state has even
		// arrived, which is the point of it.
		expect(await screen.findByText(/Only your Kitchen can see it\./)).toBeInTheDocument();
	});

	it('asks for the public address once, at the first link, and shows the link exactly once', async () => {
		const { kamosu } = renderShare({
			get_share_link: NOT_SHARED,
			share_recipe: {
				shared: true,
				share_id: 'sl_1',
				url: 'https://kamosu.example/s/abc',
				shared_by: 'Aurélien',
				created_at: '2026-08-30T00:00:00Z',
				public_address: 'https://kamosu.example',
			},
		});

		const address = await screen.findByLabelText(/public address/i);
		await fireEvent.input(address, { target: { value: 'https://kamosu.example' } });
		await fireEvent.click(screen.getByRole('button', { name: /Turn on the link/ }));

		// The address travels with the first share and is stored once.
		const asked = kamosu.calls.find((call) => call.operation === 'share_recipe');
		expect(asked?.input).toEqual({
			branch_id: 'b_1',
			public_address: 'https://kamosu.example',
		});

		// The link is shown, and the screen says plainly that it is shown once —
		// Kamosu keeps only its fingerprint (ADR 0031).
		expect(await screen.findByText('https://kamosu.example/s/abc')).toBeInTheDocument();
		expect(screen.getByText(/cannot show it to you again/)).toBeInTheDocument();
		expect(screen.getByText(/Shared by Aurélien/)).toBeInTheDocument();
	});

	it('does not ask for the address again once the instance knows it', async () => {
		renderShare({
			get_share_link: { ...NOT_SHARED, public_address: 'https://kamosu.example' },
		});

		expect(await screen.findByText(/Only your Kitchen can see it\./)).toBeInTheDocument();
		expect(screen.queryByLabelText(/public address/i)).toBeNull();
	});

	it('ends a link, and cannot reprint one it is only holding the fingerprint of', async () => {
		const { kamosu } = renderShare({
			get_share_link: {
				shared: true,
				share_id: 'sl_1',
				// Null because the Secret was answered at minting and only its
				// hash is stored: a screen opened later knows a link exists and
				// cannot show it.
				url: null,
				shared_by: 'Aurélien',
				created_at: '2026-08-30T00:00:00Z',
				public_address: 'https://kamosu.example',
			},
			end_share_link: NOT_SHARED,
		});

		expect(await screen.findByText(/Anyone with this link can read/)).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: /Copy the link/ })).toBeNull();

		await fireEvent.click(screen.getByRole('button', { name: /End the link/ }));
		const ended = kamosu.calls.find((call) => call.operation === 'end_share_link');
		expect(ended?.input).toEqual({ branch_id: 'b_1' });
		expect(await screen.findByText(/Only your Kitchen can see it\./)).toBeInTheDocument();
	});
});
