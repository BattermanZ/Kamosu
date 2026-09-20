/**
 * The screen-seam test: the share screen, against a stand-in Kamosu. The
 * answers below are checked against the Catalogue before the screen sees them
 * — a field renamed in `src/catalogue.rs` fails this test in the same commit.
 */

import { describe, expect, it } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import ShareTestHarness from './ShareTestHarness.svelte';

/**
 * The recipe behind the share screen. It reads one to name the Components a
 * Share Link would carry, so every test needs an answer for it — without one
 * the stand-in throws into an unhandled rejection, `just test` exits non-zero,
 * and the failure is invisible in the passing count. A recipe that composes
 * nothing is the case nearly every recipe is in (#50).
 */
const RECIPE = {
	branch_id: 'b_1',
	lineage_id: 'l_1',
	kitchen_id: 'k_1',
	hand_id: 'h_1',
	language: 'en',
	origin_address: null,
	head_version_id: 'v_1',
	translation: null,
	tags: [],
	related_recipes: [],
	cooked: { count: 0, last_cooked_at: null, ratings: [] },
	versions: [
		{
			sequence: 1,
			version_id: 'v_1',
			parent_version_id: null,
			hand_id: 'h_1',
			name: null,
			change_note: null,
			created_at: '2026-09-01T09:00:00Z',
			translates_version_id: null,
			language: 'en',
			components: [],
			content: {
				title: 'Chicken Katsu Curry',
				yield: null,
				prep_time_minutes: null,
				cook_time_minutes: null,
				note: null,
				main_photo: null,
				nutrition: null,
				source: null,
				ingredients: [{ kind: 'ingredient' as const, text: '1 cup panko' }],
				steps: [{ kind: 'step' as const, text: 'Coat the chicken in panko.', photo: null }],
			},
			readings: [null],
			measured: { ingredients: [null], steps: [null] },
			cooking: { steps: [{ uses: [], timer_seconds: null }] },
		},
	],
};

/**
 * What the recipe file holds. Read when the screen opens, so every test needs
 * an answer for it for the same reason `get_recipe` does. A recipe carrying
 * nothing and missing nothing is the ordinary case.
 */
const FILE = {
	file_name: 'Chicken Katsu Curry.zip',
	fetch_at: '/api/bundles/b_1',
	subjects: [{ lineage_id: 'l_1', title: 'Chicken Katsu Curry' }],
	passengers: [],
	notes: ['Chicken Katsu Curry.md'],
	photographs: 1,
	missing_photographs: [],
};

function renderShare(answers: Answers) {
	const kamosu = standIn({ get_recipe: RECIPE, export_bundle: FILE, ...answers });
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
	// --- The recipe file (#65, #66, ADR 0020) --------------------------------
	//
	// Described before it is taken, which is the habit this whole screen keeps:
	// the standing line, the line about Components and the shown-once note all
	// say what an act means before you do it (option C, Aurélien, 20 September
	// 2026).

	it('says what the recipe file holds before offering to save it', async () => {
		renderShare({ get_share_link: NOT_SHARED });

		expect(await screen.findByText('Chicken Katsu Curry.zip')).toBeInTheDocument();
		// One note and one photograph is what nearly every recipe is, so it is
		// the case the wording has to read right in (`_one`, house style).
		expect(screen.getByText(/It holds one note and one photograph/)).toBeInTheDocument();
		// The half of the format a zip cannot show for itself (ADR 0020).
		expect(screen.getByText(/another Kamosu takes it in whole/)).toBeInTheDocument();
		expect(screen.getByRole('button', { name: /Save it/ })).toBeInTheDocument();
	});

	it('offers the file whether the link is on or off, because the file is not the link', async () => {
		renderShare({
			get_share_link: {
				shared: true,
				share_id: 'sl_1',
				url: null,
				shared_by: 'Aurélien',
				created_at: '2026-08-30T00:00:00Z',
				public_address: 'https://kamosu.example',
			},
		});

		expect(await screen.findByText(/Anyone with this link can read/)).toBeInTheDocument();
		expect(await screen.findByText('Chicken Katsu Curry.zip')).toBeInTheDocument();
	});

	it('names the recipes travelling inside the file, and the photographs it cannot put in', async () => {
		renderShare({
			get_share_link: NOT_SHARED,
			export_bundle: {
				...FILE,
				passengers: [{ lineage_id: 'l_2', title: 'Katsu Sauce' }],
				notes: ['Chicken Katsu Curry.md', 'Katsu Sauce.md'],
				photographs: 0,
				// Answered as content hashes, so a count is the most that can
				// honestly be said: no screen could name which picture.
				missing_photographs: ['p_aaa', 'p_bbb'],
			},
		});

		expect(await screen.findByText(/Katsu Sauce travels inside it/)).toBeInTheDocument();
		expect(screen.getByText(/2 photographs are no longer on this instance/)).toBeInTheDocument();
	});

	it('saves the file straight from the server, without holding it in memory', async () => {
		const went: string[] = [];
		const assign = window.location.assign;
		Object.defineProperty(window, 'location', {
			configurable: true,
			value: { ...window.location, assign: (to: string) => went.push(to) },
		});

		try {
			renderShare({ get_share_link: NOT_SHARED });
			await fireEvent.click(await screen.findByRole('button', { name: /Save it/ }));
			// The address `export_bundle` named, and no second Operation: the
			// bytes come down the Credential-bearing route as a download.
			expect(went).toEqual(['/api/bundles/b_1']);
		} finally {
			Object.defineProperty(window, 'location', {
				configurable: true,
				value: { ...window.location, assign },
			});
		}
	});
	it('keeps the file where it is offline, saying what it waits for', async () => {
		// `export_bundle` is answered straight through and never kept on the
		// phone (`$lib/offline/reads`), so off the network there is nothing to
		// describe the file with. The block stays put and greys rather than
		// vanishing from under the reader (#76, option C).
		const online = Object.getOwnPropertyDescriptor(Navigator.prototype, 'onLine');
		Object.defineProperty(navigator, 'onLine', { value: false, configurable: true });
		try {
			renderShare({
				get_share_link: NOT_SHARED,
				export_bundle: { refuse: 'internal' as const, message: 'never answered' },
			});

			expect(await screen.findByText(/waits for the server\./)).toBeInTheDocument();
			expect(await screen.findByRole('button', { name: /waits for the server/i })).toBeDisabled();
			// And it does not report the link as broken: the file is not the link.
			expect(screen.queryByText(/The link could not be changed/)).toBeNull();
		} finally {
			if (online) Object.defineProperty(Navigator.prototype, 'onLine', online);
		}
	});
});
