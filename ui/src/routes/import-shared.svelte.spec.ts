/**
 * The page a Share Link's *Import this recipe* opens (#170), at the screen
 * seam. Aurélien's choices of 26 September 2026: a confirm screen before
 * anything is written, one "where do you keep your recipes?" screen for a
 * reader who is not signed in here, and a confirm screen that already knows
 * what the reader holds.
 *
 * Every answer is checked against the Catalogue's declared output by the
 * stand-in first, so these tests can lie about values and cannot lie about
 * shape.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { standIn, type Answers, type StandIn } from '$lib/api/stand-in';
import type { AuthClient } from '$lib/auth';
import type { PreviewSharedRecipeOutput } from '$lib/api/catalogue';
import { forgetArrival, noteArrival, theArrival } from '$lib/arrival.svelte';
import { getLocale } from '$lib/paraglide/runtime';
import { outlivingTheWait, withTheClockFaked } from '../testing/jobs';
import { went } from '../testing/navigation';
import ImportSharedTestHarness from './import/ImportSharedTestHarness.svelte';

afterEach(() => {
	forgetArrival();
});

const ORIGIN = 'https://recipes.aurelien.example';
const TOKEN = 'a'.repeat(64);
const LINK = `/s/${TOKEN}`;
const WHOLE = `${ORIGIN}/s/${TOKEN}`;

const PREVIEW: PreviewSharedRecipeOutput = {
	upload_id: 'u_0123456789abcdef0123456789abcdef',
	title: 'Yogurt Flatbread',
	shared_by: 'Aurélien',
	written_by: 'Aurélien',
	source: {
		text: 'feelgoodfoodie.net',
		link: 'https://feelgoodfoodie.net/recipe/yogurt-flatbread/',
	},
	versions: 3,
	photo: 'data:image/webp;base64,UklGRg==',
	held: null,
};

const job = (id: string, operation: string, result: unknown) => ({
	id,
	operation,
	status: 'completed' as const,
	progress: { done: 1, total: 1 },
	error: null,
	errorCode: null,
	created_at: '2026-09-26T10:00:00.000Z',
	updated_at: '2026-09-26T10:00:01.000Z',
	result,
});

const REPORT = {
	import_id: 'i_1',
	cookbook_id: 'c_nadia',
	source_kind: 'bundle',
	arrived: [
		{
			foreign_id: 'b_theirs',
			status: 'created',
			lineage_id: 'l_flatbread',
			branch_id: 'b_here',
			title: 'Yogurt Flatbread',
			subject: true,
		},
	],
	offered: [],
	unreadable: [],
	left_out: [],
	related_candidates: [],
};

/**
 * The page, with `preview_shared_recipe` answering `preview` and
 * `import_bundle` answering `REPORT`, each read back through `get_job` the
 * one way any Job is.
 */
function open(options: {
	preview?: PreviewSharedRecipeOutput;
	answers?: Answers;
	link?: string | null;
}) {
	let kamosu: StandIn | undefined = undefined;
	const lastJob = () =>
		kamosu?.calls.filter((call) => call.operation === 'get_job').at(-1)?.input as
			{ job_id: string } | undefined;
	kamosu = standIn({
		instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
		preview_shared_recipe: { job_id: 'j_preview' },
		import_bundle: { job_id: 'j_import' },
		get_job: () =>
			lastJob()?.job_id === 'j_import'
				? job('j_import', 'import_bundle', REPORT)
				: job('j_preview', 'preview_shared_recipe', options.preview ?? PREVIEW),
		...options.answers,
	});
	const leave = vi.fn();
	const authenticate = vi.fn<AuthClient['authenticate']>(async () => {});
	render(ImportSharedTestHarness, {
		props: {
			client: kamosu.client,
			auth: { authenticate },
			link: options.link === undefined ? LINK : options.link,
			origin: ORIGIN,
			leave,
		},
	});
	return { kamosu, leave, authenticate };
}

/** The held copy's date, as the app writes a date in its own Language. */
const ON_THE_20TH = new Date('2026-09-20T09:00:00.000Z').toLocaleDateString(getLocale(), {
	day: 'numeric',
	month: 'long',
	year: 'numeric',
});

const operations = (kamosu: StandIn) => kamosu.calls.map((call) => call.operation);

describe('importing a shared recipe, signed in', () => {
	it('says what importing does before anything is written, then imports the file it read', async () => {
		const { kamosu } = open({});

		expect(await screen.findByRole('heading', { name: 'Import this recipe?' })).toBeInTheDocument();
		expect(screen.getByText('Yogurt Flatbread')).toBeInTheDocument();
		expect(screen.getByText('Shared by Aurélien · from feelgoodfoodie.net')).toBeInTheDocument();
		expect(
			screen.getByText(
				'It goes into your Cookbook, with every version behind it (3) and where it came from.',
			),
		).toBeInTheDocument();
		expect(
			screen.getByText(
				'Aurélien stays named as the one who wrote it. Your first change starts your own version.',
			),
		).toBeInTheDocument();
		// Asked with the whole address, read against where the reader is.
		expect(kamosu.calls).toContainEqual({
			operation: 'preview_shared_recipe',
			input: { url: WHOLE },
		});
		// Opening the page imports nothing.
		expect(operations(kamosu)).not.toContain('import_bundle');

		await fireEvent.click(screen.getByRole('button', { name: 'Import it' }));
		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/recipes/b_here'));
		// The file the preview staged, not a second fetch of the link.
		expect(kamosu.calls).toContainEqual({
			operation: 'import_bundle',
			input: { upload_id: PREVIEW.upload_id },
		});
		expect(operations(kamosu).filter((name) => name === 'preview_shared_recipe')).toHaveLength(1);
		// And the line above the recipe says it came from a link, not a file.
		expect(theArrival()).toMatchObject({
			branchId: 'b_here',
			status: 'created',
			from: 'link',
			writer: 'Aurélien',
		});
	});

	it('offers your copy instead when you already hold it and nothing is new', async () => {
		const { kamosu } = open({
			preview: {
				...PREVIEW,
				held: {
					branch_id: 'b_mine',
					arrived: true,
					since: '2026-09-20T09:00:00.000Z',
					newer: 0,
					diverged: false,
				},
			},
		});

		expect(
			await screen.findByRole('heading', { name: 'Already in your library' }),
		).toBeInTheDocument();
		const date = ON_THE_20TH;
		expect(
			screen.getByText(`You imported this recipe on ${date}. Nothing new has been written since.`),
		).toBeInTheDocument();
		expect(screen.getByRole('link', { name: 'Open your copy' })).toHaveAttribute(
			'href',
			'/recipes/b_mine',
		);
		expect(screen.queryByRole('button', { name: 'Import it' })).not.toBeInTheDocument();
		expect(operations(kamosu)).not.toContain('import_bundle');
	});

	it('says a recipe your own Cookbook writes has nothing to import', async () => {
		open({
			preview: {
				...PREVIEW,
				held: {
					branch_id: 'b_mine',
					arrived: false,
					since: '2026-09-20T09:00:00.000Z',
					newer: 0,
					diverged: false,
				},
			},
		});

		expect(
			await screen.findByText('Your Cookbook writes this recipe, so there is nothing to import.'),
		).toBeInTheDocument();
	});

	it('counts the newer versions and imports only those', async () => {
		const { kamosu } = open({
			preview: {
				...PREVIEW,
				held: {
					branch_id: 'b_mine',
					arrived: true,
					since: '2026-09-20T09:00:00.000Z',
					newer: 2,
					diverged: false,
				},
			},
		});

		expect(
			await screen.findByRole('heading', { name: 'Bring it up to date?' }),
		).toBeInTheDocument();
		const date = ON_THE_20TH;
		expect(
			screen.getByText(
				`You imported this recipe on ${date}. Aurélien has saved 2 newer versions since.`,
			),
		).toBeInTheDocument();
		expect(
			screen.getByText('Importing adds them to its History. Nothing you wrote changes.'),
		).toBeInTheDocument();
		await fireEvent.click(screen.getByRole('button', { name: 'Import the 2 new versions' }));
		await waitFor(() => expect(operations(kamosu)).toContain('import_bundle'));
	});

	it('says the one newer version in the singular', async () => {
		open({
			preview: {
				...PREVIEW,
				held: {
					branch_id: 'b_mine',
					arrived: true,
					since: '2026-09-20T09:00:00.000Z',
					newer: 1,
					diverged: false,
				},
			},
		});

		expect(
			await screen.findByRole('button', { name: 'Import the new version' }),
		).toBeInTheDocument();
	});

	it('offers your copy, and no import, when the sender rewrote what you hold', async () => {
		const { kamosu } = open({
			preview: {
				...PREVIEW,
				held: {
					branch_id: 'b_mine',
					arrived: true,
					since: '2026-09-20T09:00:00.000Z',
					newer: 0,
					diverged: true,
				},
			},
		});

		expect(
			await screen.findByRole('heading', { name: 'Already in your library' }),
		).toBeInTheDocument();
		expect(
			screen.getByText(
				`You imported this recipe on ${ON_THE_20TH}. Aurélien has since rewritten it in a way your copy cannot take in, so yours stays as it is.`,
			),
		).toBeInTheDocument();
		expect(screen.getByRole('link', { name: 'Open your copy' })).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: /^Import/ })).not.toBeInTheDocument();
		expect(operations(kamosu)).not.toContain('import_bundle');
	});

	it('says an import that outlasts the wait is still going, and offers no second one', async () => {
		await withTheClockFaked(async () => {
			let importing = false;
			const stillImporting = outlivingTheWait({
				...job('j_import', 'import_bundle', null),
				status: 'running' as const,
				progress: {},
			});
			open({
				answers: {
					import_bundle: () => {
						importing = true;
						return { job_id: 'j_import' };
					},
					get_job: () =>
						importing ? stillImporting() : job('j_preview', 'preview_shared_recipe', PREVIEW),
				},
			});

			await fireEvent.click(await screen.findByRole('button', { name: 'Import it' }));
			expect(await screen.findByRole('status')).toHaveTextContent('still going');
			expect(screen.queryByRole('button', { name: 'Try again' })).not.toBeInTheDocument();
			expect(screen.queryByRole('button', { name: 'Import it' })).not.toBeInTheDocument();
		});
	});

	it('says why a link that hands over nothing could not be imported', async () => {
		open({
			answers: {
				get_job: {
					id: 'j_preview',
					operation: 'preview_shared_recipe',
					status: 'failed',
					progress: {},
					error: 'this Share Link was ended by the person who shared it',
					errorCode: null,
					created_at: '2026-09-26T10:00:00.000Z',
					updated_at: '2026-09-26T10:00:01.000Z',
					result: null,
				},
			},
		});

		expect(
			await screen.findByRole('heading', { name: 'This recipe could not be imported' }),
		).toBeInTheDocument();
		expect(screen.getByRole('alert')).toHaveTextContent('this Share Link was ended');
		expect(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
	});

	it('says so when opened with no Share Link at all, and asks nothing', async () => {
		const { kamosu } = open({ link: null });

		expect(
			await screen.findByRole('heading', { name: 'This recipe could not be imported' }),
		).toBeInTheDocument();
		expect(operations(kamosu)).not.toContain('preview_shared_recipe');
		expect(screen.queryByRole('button', { name: 'Try again' })).not.toBeInTheDocument();
	});
});

describe('importing a shared recipe, signed out', () => {
	const SHARED = {
		ended: false,
		public_address: ORIGIN,
		share_id: 'sl_1',
		shared_by: 'Aurélien',
		thread: [],
		translations: [],
		recipe: {
			branch_id: 'b_theirs',
			components: [],
			content: {
				title: 'Yogurt Flatbread',
				cook_time_minutes: 5,
				prep_time_minutes: 15,
				ingredients: [],
				steps: [],
				main_photo: null,
				note: null,
				nutrition: null,
				source: null,
				yield: null,
			},
			language: 'en',
			lineage_id: 'l_flatbread',
			readings: [],
			version_id: 'v_1',
		},
	};

	const signedOut: Answers = {
		preview_shared_recipe: { refuse: 'unauthorized' },
		read_shared_recipe: SHARED,
	};

	it('asks where you keep your recipes: log in here, or name your own Kamosu', async () => {
		const { kamosu } = open({ answers: signedOut });

		expect(
			await screen.findByRole('heading', { name: 'Import Yogurt Flatbread' }),
		).toBeInTheDocument();
		expect(
			screen.getByText('Shared by Aurélien. Where do you keep your recipes?'),
		).toBeInTheDocument();
		expect(screen.getByRole('heading', { name: 'Here, on this Kamosu' })).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Log in and import it' })).toBeInTheDocument();
		expect(screen.getByRole('heading', { name: 'In another Kamosu' })).toBeInTheDocument();
		expect(screen.getByRole('link', { name: 'Download the recipe file' })).toHaveAttribute(
			'href',
			`${ORIGIN}/s/${TOKEN}/bundle`,
		);
		expect(kamosu.calls).toContainEqual({
			operation: 'read_shared_recipe',
			input: { token: TOKEN },
		});
	});

	it('sends you to your own Kamosu with the link in hand', async () => {
		const { leave } = open({ answers: signedOut });

		await fireEvent.input(await screen.findByLabelText('Your Kamosu’s address'), {
			target: { value: ' kamosu.martin-family.fr/ ' },
		});
		await fireEvent.click(screen.getByRole('button', { name: 'Import it there' }));
		expect(leave).toHaveBeenCalledWith(
			`https://kamosu.martin-family.fr/import?link=${encodeURIComponent(WHOLE)}`,
		);
	});

	it('keeps a plain http address plain, and refuses this Kamosu as the other one', async () => {
		const { leave } = open({ answers: signedOut });
		const field = await screen.findByLabelText('Your Kamosu’s address');

		await fireEvent.input(field, { target: { value: ORIGIN } });
		await fireEvent.click(screen.getByRole('button', { name: 'Import it there' }));
		expect(screen.getByRole('alert')).toHaveTextContent('That is this Kamosu.');
		expect(leave).not.toHaveBeenCalled();

		await fireEvent.input(field, { target: { value: 'http://192.168.1.20:5266' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Import it there' }));
		expect(leave).toHaveBeenCalledWith(
			`http://192.168.1.20:5266/import?link=${encodeURIComponent(WHOLE)}`,
		);
	});

	it('asks again once you log in, and then shows the confirm screen', async () => {
		let signedIn = false;
		const { kamosu, authenticate } = open({
			answers: {
				preview_shared_recipe: () =>
					signedIn ? { job_id: 'j_preview' } : { refuse: 'unauthorized' },
				read_shared_recipe: SHARED,
			},
		});
		authenticate.mockImplementation(async () => {
			signedIn = true;
		});

		await fireEvent.input(await screen.findByLabelText('Name'), { target: { value: 'Nadia' } });
		await fireEvent.input(screen.getByLabelText('Password'), {
			target: { value: 'a long enough password' },
		});
		await fireEvent.click(screen.getByRole('button', { name: 'Log in and import it' }));

		expect(await screen.findByRole('heading', { name: 'Import this recipe?' })).toBeInTheDocument();
		expect(operations(kamosu).filter((name) => name === 'preview_shared_recipe')).toHaveLength(2);
	});

	it('names no recipe it cannot read, when the link is another Kamosu’s', async () => {
		const { kamosu } = open({
			answers: signedOut,
			link: `https://far.example/s/${TOKEN}`,
		});

		expect(await screen.findByRole('heading', { name: 'Import this recipe' })).toBeInTheDocument();
		expect(screen.getByText('Where do you keep your recipes?')).toBeInTheDocument();
		expect(operations(kamosu)).not.toContain('read_shared_recipe');
	});
});

describe('the line above a recipe imported from a Share Link', () => {
	it('says whose newer versions came, and how many', async () => {
		noteArrival({
			branchId: 'b_here',
			title: 'Yogurt Flatbread',
			how: 'arrived',
			status: 'extended',
			passengers: [],
			jobId: 'j_import',
			from: 'link',
			writer: 'Aurélien',
			added: 2,
		});
		render(ImportSharedTestHarness, {
			props: {
				client: standIn({}).client,
				auth: { authenticate: vi.fn() },
				link: null,
				origin: ORIGIN,
				leave: vi.fn(),
				pathname: '/recipes/b_here',
			},
		});

		const said = await screen.findByRole('status', { name: 'Brought up to date' });
		expect(said).toHaveTextContent(
			'Aurélien’s 2 newer versions of Yogurt Flatbread are in its History now.',
		);
	});

	it('names no file', async () => {
		noteArrival({
			branchId: 'b_here',
			title: 'Yogurt Flatbread',
			how: 'arrived',
			status: 'created',
			passengers: [],
			jobId: 'j_import',
			from: 'link',
		});
		render(ImportSharedTestHarness, {
			props: {
				client: standIn({}).client,
				auth: { authenticate: vi.fn() },
				link: null,
				origin: ORIGIN,
				leave: vi.fn(),
				pathname: '/recipes/b_here',
			},
		});

		const said = await screen.findByRole('status', { name: 'Imported into your Cookbook' });
		expect(said).toHaveTextContent(
			'Yogurt Flatbread is yours now, with its whole History and where it came from.',
		);
	});
});
