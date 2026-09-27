/**
 * The screen-seam test: the share screen, against a stand-in Kamosu. The
 * answers below are checked against the Catalogue before the screen sees them
 * — a field renamed in `src/catalogue.rs` fails this test in the same commit.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import { tick, type ComponentProps } from 'svelte';
import { realFiles } from '$lib/api/files';
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
	cookbook: { id: 'c_1', name: null, authors: [{ person_id: 'p_1', name: 'Aurélien' }] },
	name: null,
	writes: true,
	mine: true,
	arrived: false,
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
			scaled_to: null,
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

function renderShare(
	answers: Answers,
	/** The phone it is on, and how it fetches the file to share (#156). */
	on: Pick<ComponentProps<typeof ShareTestHarness>, 'device' | 'files'> = {},
) {
	const kamosu = standIn({ get_recipe: RECIPE, export_bundle: FILE, ...answers });
	render(ShareTestHarness, { props: { client: kamosu.client, branchId: 'b_1', ...on } });
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
		expect(await screen.findByText(/not copies already sent\./)).toBeInTheDocument();
		expect(screen.getByText(/every version of this recipe/)).toBeInTheDocument();
		// The standing line is on screen before the link's state has even
		// arrived, which is the point of it.
		expect(await screen.findByText(/Only your Kitchen can see it\./)).toBeInTheDocument();
	});

	it('asks for the public address once, at the first link, and shows the link it mints', async () => {
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

		// The link is shown, with no warning that it will vanish: it is shown
		// again every time this screen opens (#171, ADR 0031 as amended).
		expect(await screen.findByText('https://kamosu.example/s/abc')).toBeInTheDocument();
		expect(screen.queryByText(/shown again/)).toBeNull();
		expect(screen.getByText(/Shared by Aurélien/)).toBeInTheDocument();
	});

	it('does not ask for the address again once the instance knows it', async () => {
		renderShare({
			get_share_link: { ...NOT_SHARED, public_address: 'https://kamosu.example' },
		});

		expect(await screen.findByText(/Only your Kitchen can see it\./)).toBeInTheDocument();
		expect(screen.queryByLabelText(/public address/i)).toBeNull();
	});

	it('shows a live link again on a fresh visit, to copy and open, not only at minting', async () => {
		const writes: string[] = [];
		const real = Object.getOwnPropertyDescriptor(navigator, 'clipboard');
		Object.defineProperty(navigator, 'clipboard', {
			configurable: true,
			value: { writeText: async (text: string) => void writes.push(text) },
		});
		try {
			renderShare({
				get_share_link: {
					shared: true,
					share_id: 'sl_1',
					url: 'https://kamosu.example/s/abc',
					shared_by: 'Aurélien',
					created_at: '2026-08-30T00:00:00Z',
					public_address: 'https://kamosu.example',
				},
			});

			expect(await screen.findByText('https://kamosu.example/s/abc')).toBeInTheDocument();
			expect(screen.getByRole('link', { name: /Open the page/ })).toHaveAttribute(
				'href',
				'https://kamosu.example/s/abc',
			);
			expect(screen.queryByText(/made before Kamosu kept its address/)).toBeNull();

			await fireEvent.click(screen.getByRole('button', { name: /Copy the link/ }));
			expect(writes).toEqual(['https://kamosu.example/s/abc']);
			expect(await screen.findByRole('button', { name: /Copied/ })).toBeInTheDocument();
		} finally {
			if (real) Object.defineProperty(navigator, 'clipboard', real);
			else Reflect.deleteProperty(navigator, 'clipboard');
		}
	});

	it('ends a link, and says why one made before Kamosu kept its address has none to copy', async () => {
		const { kamosu } = renderShare({
			get_share_link: {
				shared: true,
				share_id: 'sl_1',
				// Null because this link was minted before #171, when only the
				// Secret's hash was stored: it still opens, and cannot be shown.
				url: null,
				shared_by: 'Aurélien',
				created_at: '2026-08-30T00:00:00Z',
				public_address: 'https://kamosu.example',
			},
			end_share_link: NOT_SHARED,
		});

		expect(await screen.findByText(/Anyone with this link can read/)).toBeInTheDocument();
		expect(screen.getByText(/made before Kamosu kept its address/)).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: /Copy the link/ })).toBeNull();
		expect(screen.queryByRole('link', { name: /Open the page/ })).toBeNull();

		await fireEvent.click(screen.getByRole('button', { name: /End the link/ }));
		const ended = kamosu.calls.find((call) => call.operation === 'end_share_link');
		expect(ended?.input).toEqual({ branch_id: 'b_1' });
		expect(await screen.findByText(/Only your Kitchen can see it\./)).toBeInTheDocument();
	});
	// --- The recipe file (#65, #66, ADR 0020) --------------------------------
	//
	// Described before it is taken, which is the habit this whole screen keeps:
	// the standing line and the line about Components both say what an act
	// means before you do it (option C, Aurélien, 20 September
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
		expect(screen.getByText(/2 photographs are no longer here/)).toBeInTheDocument();
	});

	it('saves the file straight from the server, without holding it in memory', async () => {
		const assigned: string[] = [];
		const real = Object.getOwnPropertyDescriptor(window, 'location');
		Object.defineProperty(window, 'location', {
			configurable: true,
			value: { ...window.location, assign: (to: string) => assigned.push(to) },
		});

		try {
			renderShare({ get_share_link: NOT_SHARED });
			await fireEvent.click(await screen.findByRole('button', { name: /Save it/ }));
			// The address `export_bundle` named, and no second Operation: the
			// bytes come down the Credential-bearing route as a download.
			expect(assigned).toEqual(['/api/bundles/b_1']);
		} finally {
			// The descriptor as it was, not a copy of it: a plain object standing in
			// for the real Location would outlive this file.
			if (real) Object.defineProperty(window, 'location', real);
		}
	});
	it('keeps the file where it is offline, saying what it waits for', async () => {
		// `export_bundle` is answered straight through and never kept on the
		// phone (`$lib/offline/reads`), so off the network there is nothing to
		// describe the file with. The block stays put and greys rather than
		// vanishing from under the reader (#76, option C).
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
			Reflect.deleteProperty(navigator, 'onLine');
		}
	});
});

describe('the recipe file in the app installed on an iPhone (#156)', () => {
	const installedApple = { installed: true, apple: true };

	/**
	 * The share sheet, as iOS offers it to an installed app. `canShare` says
	 * yes to one zip unless a test says otherwise; `share` answers what the
	 * test says.
	 */
	function theShareSheet(answer: () => Promise<void> = async () => {}, takesZips = true) {
		const share = vi.fn<(data: ShareData) => Promise<void>>(() => answer());
		const canShare = vi.fn(
			(data?: ShareData) =>
				takesZips && data?.files?.length === 1 && data.files[0]?.type === 'application/zip',
		);
		Object.defineProperty(navigator, 'share', { value: share, configurable: true });
		Object.defineProperty(navigator, 'canShare', { value: canShare, configurable: true });
		return share;
	}

	/** `/api/bundles/…` as the server answers it, headers and all. */
	function theBundleRoute() {
		const fetch = vi.fn(
			async () =>
				new Response('PK', {
					headers: {
						'content-type': 'application/zip',
						'content-disposition':
							'attachment; filename="Chicken Katsu Curry.zip"; filename*=UTF-8\'\'Chicken%20Katsu%20Curry.zip',
					},
				}),
		);
		return { fetch, files: realFiles(fetch as unknown as typeof globalThis.fetch) };
	}

	/** `window.location.assign`, caught, and put back afterwards. */
	function catchNavigation() {
		const assigned: string[] = [];
		const real = Object.getOwnPropertyDescriptor(window, 'location');
		Object.defineProperty(window, 'location', {
			configurable: true,
			value: { ...window.location, assign: (to: string) => assigned.push(to) },
		});
		return {
			assigned,
			restore: () => {
				if (real) Object.defineProperty(window, 'location', real);
			},
		};
	}

	afterEach(() => {
		Reflect.deleteProperty(navigator, 'share');
		Reflect.deleteProperty(navigator, 'canShare');
	});

	/** Save it tapped and the file fetched: the button now shares it. */
	async function readyToShare(share = theShareSheet()) {
		const route = theBundleRoute();
		renderShare({ get_share_link: NOT_SHARED }, { device: installedApple, files: route.files });
		await fireEvent.click(await screen.findByRole('button', { name: /Save it/ }));
		const button = await screen.findByRole('button', { name: 'Share the file' });
		return { share, route, button };
	}

	it('fetches the file at the first tap, and shares it as one zip named as the server named it', async () => {
		const navigation = catchNavigation();
		try {
			const { share, route, button } = await readyToShare();

			expect(navigation.assigned).toEqual([]);
			expect(route.fetch).toHaveBeenCalledWith('/api/bundles/b_1');
			expect(share).not.toHaveBeenCalled();
			await fireEvent.click(button);

			expect(share).toHaveBeenCalledTimes(1);
			const files = share.mock.calls[0]?.[0].files ?? [];
			expect(files).toHaveLength(1);
			expect(files[0]?.name).toBe('Chicken Katsu Curry.zip');
			expect(files[0]?.type).toBe('application/zip');
		} finally {
			navigation.restore();
		}
	});

	it('shares in the tap itself, with the file already fetched', async () => {
		const { share, route, button } = await readyToShare();
		const fetchedBefore = route.fetch.mock.calls.length;

		// Not awaited: the share sheet must be asked for before the tap's
		// handler gives the browser back, or iOS no longer counts the tap.
		const tapped = fireEvent.click(button);
		expect(share).toHaveBeenCalledTimes(1);
		expect(route.fetch).toHaveBeenCalledTimes(fetchedBefore);
		await tapped;
	});

	it('says it is getting the file while it downloads, and takes no second tap meanwhile', async () => {
		let arrive: (response: Response) => void = () => {};
		const fetch = vi.fn(() => new Promise<Response>((resolve) => (arrive = resolve)));
		theShareSheet();
		renderShare(
			{ get_share_link: NOT_SHARED },
			{ device: installedApple, files: realFiles(fetch as unknown as typeof globalThis.fetch) },
		);

		await fireEvent.click(await screen.findByRole('button', { name: /Save it/ }));

		expect(await screen.findByRole('button', { name: 'Getting the file…' })).toBeDisabled();
		arrive(new Response('PK', { headers: { 'content-type': 'application/zip' } }));
		expect(await screen.findByRole('button', { name: 'Share the file' })).toBeEnabled();
	});

	it('keeps the file ready, and says nothing went wrong, when the share sheet is closed', async () => {
		const share = theShareSheet(async () => {
			throw new DOMException('Share canceled', 'AbortError');
		});
		const { button } = await readyToShare(share);

		await fireEvent.click(button);
		await tick();

		expect(screen.queryByRole('alert')).not.toBeInTheDocument();
		const again = screen.getByRole('button', { name: 'Share the file' });
		expect(again).toBeEnabled();
		await fireEvent.click(again);
		expect(share).toHaveBeenCalledTimes(2);
		expect(share.mock.calls[1]?.[0].files?.[0]).toBe(share.mock.calls[0]?.[0].files?.[0]);
	});

	it('says the file could not be saved when the share sheet refuses it', async () => {
		const share = theShareSheet(async () => {
			throw new DOMException('Not allowed', 'NotAllowedError');
		});
		const { button } = await readyToShare(share);

		await fireEvent.click(button);

		expect(await screen.findByRole('alert')).toHaveTextContent(/could not be saved/);
		expect(screen.getByRole('button', { name: /Save it/ })).toBeEnabled();
	});

	it('says the file could not be saved when the server does not hand it over', async () => {
		theShareSheet();
		const fetch = vi.fn(
			async () =>
				new Response(
					JSON.stringify({ ok: false, error: { kind: 'not_found', message: 'no such recipe' } }),
					{ status: 404, headers: { 'content-type': 'application/json' } },
				),
		);
		renderShare(
			{ get_share_link: NOT_SHARED },
			{ device: installedApple, files: realFiles(fetch as unknown as typeof globalThis.fetch) },
		);

		await fireEvent.click(await screen.findByRole('button', { name: /Save it/ }));

		expect(await screen.findByRole('alert')).toHaveTextContent(/could not be saved/);
		expect(screen.getByRole('button', { name: /Save it/ })).toBeEnabled();
	});

	it('offers to save afresh once the file has been shared', async () => {
		const { button } = await readyToShare();

		await fireEvent.click(button);

		expect(await screen.findByRole('button', { name: /Save it/ })).toBeEnabled();
	});

	it.each([
		['a Safari tab on an iPhone', { installed: false, apple: true }, true],
		['an installed app that is not on Apple', { installed: true, apple: false }, true],
		['an installed iPhone app that will not share a zip', { installed: true, apple: true }, false],
	])('still saves the file straight from the server in %s', async (_, device, takesZips) => {
		const share = theShareSheet(async () => {}, takesZips);
		const route = theBundleRoute();
		const navigation = catchNavigation();
		try {
			renderShare({ get_share_link: NOT_SHARED }, { device, files: route.files });

			await fireEvent.click(await screen.findByRole('button', { name: /Save it/ }));

			expect(navigation.assigned).toEqual(['/api/bundles/b_1']);
			expect(route.fetch).not.toHaveBeenCalled();
			expect(share).not.toHaveBeenCalled();
		} finally {
			navigation.restore();
		}
	});
});
