/**
 * Bringing a recipe file in, at the screen seam (#93).
 *
 * The file itself travels out of band (ADR 0001), so the test stands in for
 * the upload and checks what happens around it: the id it answers is what
 * `import_bundle` is asked with, and the screen then opens the *recipe* rather
 * than the Import Report — which is the choice option C settled, and the whole
 * difference between one file and a Crouton library.
 *
 * Every Report used here is checked against the Catalogue's declared output by
 * the stand-in before the screen ever sees it, so these tests can lie about
 * values and cannot lie about shape.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import BringInTestHarness from './BringInTestHarness.svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import { OperationError } from '$lib/api/client';
import { forgetArrival } from './arrival.svelte';
import { went } from '../testing/navigation';
import { outlivingTheWait, withTheClockFaked } from '../testing/jobs';

afterEach(() => {
	Reflect.deleteProperty(navigator, 'onLine');
	// The arrival outlives a component on purpose: it has to survive the
	// navigation that shows it. So a test that noted one puts it away.
	forgetArrival();
});

const file = () => new File(['PK'], 'soba-with-walnut-miso.zip', { type: 'application/zip' });

/** One recipe's Report, as `import_bundle` answers it (#67). */
const report = (rows: {
	arrived?: Record<string, unknown>[];
	unreadable?: Record<string, unknown>[];
}) => ({
	import_id: 'i_1',
	kitchen_id: 'k_home',
	source_kind: 'bundle',
	arrived: rows.arrived ?? [],
	offered: [],
	unreadable: rows.unreadable ?? [],
	left_out: [],
	related_candidates: [],
});

const subject = {
	foreign_id: 'b_theirs',
	status: 'created',
	lineage_id: 'l_soba',
	branch_id: 'b_soba',
	title: 'Soba with walnut miso',
	subject: true,
};

const passenger = {
	foreign_id: 'b_theirs_paste',
	status: 'created',
	lineage_id: 'l_paste',
	branch_id: 'b_paste',
	title: 'Toasted walnut paste',
	subject: false,
};

/** The Job this Report is the result of, read back the one way any Job is. */
const finishedWith = (result: ReturnType<typeof report>): Answers => ({
	import_bundle: { job_id: 'j_9' },
	get_job: {
		id: 'j_9',
		operation: 'import_bundle',
		status: 'completed',
		progress: { done: 1, total: 1 },
		error: null,
		errorCode: null,
		created_at: '2026-09-21T10:00:00.000Z',
		updated_at: '2026-09-21T10:00:02.000Z',
		result,
	},
});

/** A Job still running when the screen stops waiting on it (#117). */
const running = (operation: string) => ({
	id: 'j_9',
	operation,
	status: 'running' as const,
	progress: {},
	error: null,
	errorCode: null,
	created_at: '2026-09-21T10:00:00.000Z',
	updated_at: '2026-09-21T10:00:02.000Z',
	result: null,
});

async function bringIn(answers: Answers, upload = vi.fn(async () => 'u_staged')) {
	const kamosu = standIn(answers);
	render(BringInTestHarness, {
		props: { client: kamosu.client, upload, pathname: '/recipes/b_soba' },
	});
	await fireEvent.change(screen.getByLabelText('Add a recipe from a recipe file'), {
		target: { files: [file()] },
	});
	return { kamosu, upload };
}

describe('bringing a recipe file in', () => {
	it('sends the file out of band, names its id to the Operation, and opens the recipe', async () => {
		const { kamosu, upload } = await bringIn(
			finishedWith(report({ arrived: [subject, passenger] })),
		);

		// The recipe, not the Report. A Bundle is one recipe, and the Report was
		// built for 86 at once (#68).
		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/recipes/b_soba'));
		expect(went).not.toHaveBeenCalledWith('/imports/j_9');

		// Sent as its own bytes, then named — never base64 inside the body.
		expect(upload).toHaveBeenCalledOnce();
		expect(kamosu.calls.find((call) => call.operation === 'import_bundle')?.input).toEqual({
			upload_id: 'u_staged',
		});
		// Receiving is slow, so it is a Job, read back the one way any Job is
		// (ADR 0032).
		expect(kamosu.calls.map((call) => call.operation)).toContain('get_job');
	});

	it('says what arrived, and names what travelled inside it', async () => {
		await bringIn(finishedWith(report({ arrived: [subject, passenger] })));

		const said = await screen.findByRole('status', { name: 'Brought in' });
		expect(said).toHaveTextContent('Soba with walnut miso arrived');
		// A Passenger travels because the recipe is made of it (ADR 0008), and
		// saying so is the only way a second recipe appearing is not a surprise.
		expect(said).toHaveTextContent('Toasted walnut paste came with it');
		expect(screen.getByRole('button', { name: 'How it went' })).toBeInTheDocument();
	});

	it('says the same file twice changed nothing, rather than showing a second recipe', async () => {
		await bringIn(finishedWith(report({ arrived: [{ ...subject, status: 'unchanged' }] })));

		const said = await screen.findByRole('status', { name: 'You already had it' });
		expect(said).toHaveTextContent('Nothing was written');
		// It still opens the recipe: what you asked for is to read it.
		expect(went).toHaveBeenCalledWith('/recipes/b_soba');
	});

	it('says a file carried the recipe further when it did', async () => {
		await bringIn(finishedWith(report({ arrived: [{ ...subject, status: 'extended' }] })));
		expect(await screen.findByRole('status', { name: 'Brought up to date' })).toHaveTextContent(
			'Soba with walnut miso was already here',
		);
	});

	it('says a file that is not a recipe file is not one, and opens nothing', async () => {
		await bringIn(
			finishedWith(
				report({
					unreadable: [
						{ foreign_id: null, reason: 'this file is not a zip, so no Bundle could be opened' },
					],
				}),
			),
		);

		// A file nothing can be read from is a completed Job with an empty
		// Report, not a failure — so its reason is the answer, said where the
		// file was chosen.
		expect(await screen.findByRole('alert')).toHaveTextContent('not a zip');
		expect(went).not.toHaveBeenCalled();
	});

	it('opens the recipe a damaged Bundle was kept as, in the words the Core used', async () => {
		const why =
			'Soba with walnut miso was read from its note alone, because this Bundle’s machine ' +
			'half could not be used (the sidecar is unreadable). Its words were kept as a new ' +
			'recipe of your own, with no history, no photographs and no link to the recipe it ' +
			'came from.';
		const kamosu = standIn(
			finishedWith(
				report({
					unreadable: [
						{
							foreign_id: 'Soba with walnut miso.md',
							reason: why,
							kept_as: {
								lineage_id: 'l_kept',
								branch_id: 'b_soba',
								title: 'Soba with walnut miso',
							},
						},
					],
				}),
			),
		);
		render(BringInTestHarness, {
			props: {
				client: kamosu.client,
				upload: vi.fn(async () => 'u_staged'),
				pathname: '/recipes/b_soba',
			},
		});
		await fireEvent.change(screen.getByLabelText('Add a recipe from a recipe file'), {
			target: { files: [file()] },
		});

		// Nothing was inherited, so it is a different sentence — and the Core's
		// own, which already names what was kept and what was lost (#67).
		const said = await screen.findByRole('status', { name: 'Kept as a recipe of your own' });
		expect(said).toHaveTextContent('with no history');
		expect(went).toHaveBeenCalledWith('/recipes/b_soba');
	});

	it('opens the Report, not a guess, when a damaged Bundle kept several recipes', async () => {
		await bringIn(
			finishedWith(
				report({
					unreadable: [
						// Sorted by note filename, which is alphabetical by title and
						// says nothing about which was the Bundle's subject — so the
						// Passenger is first here, exactly as it would really arrive.
						{
							foreign_id: 'Toasted walnut paste.md',
							reason: 'this Bundle’s machine half could not be used',
							kept_as: {
								lineage_id: 'l_kept_paste',
								branch_id: 'b_paste',
								title: 'Toasted walnut paste',
							},
						},
						{
							foreign_id: 'Soba with walnut miso.md',
							reason: 'this Bundle’s machine half could not be used',
							kept_as: {
								lineage_id: 'l_kept_soba',
								branch_id: 'b_soba',
								title: 'Soba with walnut miso',
							},
						},
					],
				}),
			),
		);

		// More happened than a line can name, and which recipe you wanted is not
		// knowable from the Report — so the ledger is the honest answer.
		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/imports/j_9'));
		expect(went).not.toHaveBeenCalledWith('/recipes/b_paste');
		expect(went).not.toHaveBeenCalledWith('/recipes/b_soba');
	});

	it('says why a file was refused before it ever reached an Operation', async () => {
		const kamosu = standIn({});
		const upload = vi.fn(async () => {
			throw new OperationError('upload', 'bad_request', 'the file is larger than 2 GB');
		});
		render(BringInTestHarness, { props: { client: kamosu.client, upload } });
		await fireEvent.change(screen.getByLabelText('Add a recipe from a recipe file'), {
			target: { files: [file()] },
		});

		expect(await screen.findByRole('alert')).toHaveTextContent('larger than 2 GB');
		expect(kamosu.calls.some((call) => call.operation === 'import_bundle')).toBe(false);
	});

	it('says a failed Job rather than going silent', async () => {
		await bringIn({
			import_bundle: { job_id: 'j_9' },
			get_job: {
				id: 'j_9',
				operation: 'import_bundle',
				status: 'failed',
				progress: { done: 0, total: 1 },
				error: 'the upload had gone',
				errorCode: 404,
				created_at: '2026-09-21T10:00:00.000Z',
				updated_at: '2026-09-21T10:00:02.000Z',
				result: null,
			},
		});

		expect(await screen.findByRole('alert')).toHaveTextContent('the upload had gone');
		expect(went).not.toHaveBeenCalled();
	});

	it('says a recipe file still arriving is still arriving, not that it failed (#117)', () =>
		withTheClockFaked(async () => {
			await bringIn({
				import_bundle: { job_id: 'j_9' },
				get_job: outlivingTheWait(running('import_bundle')),
			});

			expect(await screen.findByRole('status')).toHaveTextContent(
				'still going. The recipe will arrive by itself, and Settings › Brought in will show how it went.',
			);
			expect(screen.queryByRole('alert')).not.toBeInTheDocument();
			expect(went).not.toHaveBeenCalled();
		}));

	it('waits for the server offline rather than failing when pressed', async () => {
		Object.defineProperty(navigator, 'onLine', { value: false, configurable: true });
		const kamosu = standIn({});
		render(BringInTestHarness, {
			props: { client: kamosu.client, upload: vi.fn(async () => 'u_staged') },
		});

		// The button stays where it is and says what it is waiting for (#76,
		// ADR 0013) — an offline import queue would be a merge, and Kamosu
		// never merges.
		const waiting = await screen.findByRole('button', {
			name: 'Bringing a recipe in waits for the server',
		});
		expect(waiting).toBeDisabled();
		expect(
			screen.queryByRole('button', { name: 'Bring in a recipe file' }),
		).not.toBeInTheDocument();
	});
});

describe('the quiet line above the shelf', () => {
	it('says the one thing once offline, rather than two greyed acts', async () => {
		Object.defineProperty(navigator, 'onLine', { value: false, configurable: true });
		const kamosu = standIn({});
		render(BringInTestHarness, {
			props: { client: kamosu.client, upload: vi.fn(async () => 'u_staged'), look: 'quiet' },
		});

		// One sentence, not "Add a recipe Importing from a link waits for the
		// server · Bringing a recipe in waits for the server" (#93, seen live).
		expect(
			await screen.findByText('Bringing a recipe in waits for the server'),
		).toBeInTheDocument();
		expect(screen.queryByText('Add a recipe')).not.toBeInTheDocument();
		expect(
			screen.queryByRole('button', { name: 'Add a recipe from a link' }),
		).not.toBeInTheDocument();
		expect(
			screen.queryByRole('button', { name: 'Add a recipe from a recipe file' }),
		).not.toBeInTheDocument();
	});

	it('offers both ways in, each naming the whole act it performs', async () => {
		const kamosu = standIn({});
		render(BringInTestHarness, {
			props: { client: kamosu.client, upload: vi.fn(async () => 'u_staged'), look: 'quiet' },
		});

		// The visible words are a fragment, so the accessible name carries the
		// whole act with the fragment inside it (WCAG 2.5.3).
		const link = screen.getByRole('button', { name: 'Add a recipe from a link' });
		expect(link).toHaveTextContent('from a link');
		expect(
			screen.getByRole('button', { name: 'Add a recipe from a recipe file' }),
		).toHaveTextContent('from a recipe file');

		// And the link offer *does* the thing rather than navigating anywhere
		// (ADR 0027): the address field appears in place.
		await fireEvent.click(link);
		expect(await screen.findByLabelText(/web address/)).toBeInTheDocument();
		expect(went).not.toHaveBeenCalled();
	});
});

describe('importing from a link', () => {
	it('says a page still being read is still being read, not that Kamosu went wrong (#117)', () =>
		withTheClockFaked(async () => {
			const kamosu = standIn({
				import_web_link: { job_id: 'j_9' },
				get_job: outlivingTheWait(running('import_web_link')),
			});
			render(BringInTestHarness, { props: { client: kamosu.client, upload: vi.fn() } });
			await fireEvent.click(screen.getByRole('button', { name: /Import from a link/ }));
			const field = screen.getByLabelText(/web address/);
			await fireEvent.input(field, { target: { value: 'https://example.test/osso-buco' } });
			await fireEvent.submit(field.closest('form')!);

			// Before #117 this escaped uncaught, which a real browser shows as the
			// "Kamosu went wrong" card.
			expect(await screen.findByRole('status')).toHaveTextContent(/still going/);
			expect(screen.queryByRole('alert')).not.toBeInTheDocument();
			expect(went).not.toHaveBeenCalled();
		}));
});
