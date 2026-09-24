/**
 * One source and its arrivals, with forgetting (#108).
 *
 * The forget confirmation is the part worth being careful about. `forget_import`
 * reads as destructive and is not: no recipe is deleted, renamed or marked
 * (ADR 0025). Its real cost is invisible until the next time the same file is
 * brought in. So what is tested is that the sheet says both — what is kept and
 * what is lost — before anything happens, that nothing is asked of the Core
 * until the sheet is answered, and that answering it does not take the arrivals
 * away with the ledger.
 */

import { describe, expect, it } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import type { ListImportsOutput } from '$lib/api/catalogue';
import SourceTestHarness from './SourceTestHarness.svelte';

type Import = ListImportsOutput['imports'][number];
type Arrival = Import['arrivals'][number];

const arrival = (job_id: string, created_at: string, rest: Partial<Arrival> = {}): Arrival => ({
	job_id,
	created_at,
	status: 'completed',
	arrived: 1,
	created: 1,
	offered: 0,
	unreadable: 0,
	...rest,
});

const CROUTON: Import = {
	import_id: 'imp_crouton',
	source_kind: 'crouton',
	created_at: '2026-08-29T16:16:00.000Z',
	remembered: 89,
	arrivals: [
		arrival('j_c2', '2026-09-19T12:55:00.000Z', { arrived: 86, created: 0 }),
		arrival('j_c1', '2026-09-19T12:53:00.000Z', { arrived: 86, created: 86 }),
		arrival('j_c0', '2026-08-29T16:16:00.000Z', { arrived: 3, created: 3 }),
	],
};

function draw(answers: Answers, sourceKind = 'crouton') {
	const kamosu = standIn(answers);
	render(SourceTestHarness, { props: { client: kamosu.client, sourceKind } });
	return kamosu;
}

describe('one source and what has arrived through it', () => {
	it('lists every arrival, each leading to the Report that already exists', async () => {
		draw({ list_imports: { imports: [CROUTON] } });

		// Waited for by a line only one arrival says: two of the three landed on
		// the same day, and the way back to the list is a link too.
		await screen.findByText('86 recipes, already had them all');
		const reports = screen
			.getAllByRole('link')
			.filter((row) => /^\/imports\/j_/.test(row.getAttribute('href') ?? ''));
		expect(reports.map((row) => row.getAttribute('href'))).toEqual([
			'/imports/j_c2',
			'/imports/j_c1',
			'/imports/j_c0',
		]);
	});

	it('says what each arrival did without anyone opening its Report', async () => {
		draw({
			list_imports: {
				imports: [
					{
						...CROUTON,
						arrivals: [
							arrival('j_none', '2026-09-21T11:43:00.000Z', {
								arrived: 0,
								created: 0,
								unreadable: 1,
							}),
							arrival('j_held', '2026-09-19T12:55:00.000Z', { arrived: 86, created: 0 }),
							arrival('j_new', '2026-09-19T12:53:00.000Z', { arrived: 86, created: 86 }),
							arrival('j_some', '2026-09-18T15:20:00.000Z', {
								arrived: 5,
								created: 2,
								offered: 1,
							}),
							arrival('j_running', '2026-09-22T09:00:00.000Z', { status: 'running' }),
						],
					},
				],
			},
		});

		expect(await screen.findByText('Nothing arrived · 1 could not be read')).toBeInTheDocument();
		expect(screen.getByText('86 recipes, already had them all')).toBeInTheDocument();
		expect(screen.getByText('86 recipes, all new')).toBeInTheDocument();
		expect(
			screen.getByText('5 recipes, 2 new · 1 changed and waiting for you'),
		).toBeInTheDocument();
		expect(screen.getByText('Bringing them in now')).toBeInTheDocument();
	});

	it('says what forgetting keeps and what it costs, before it does anything', async () => {
		const kamosu = draw({ list_imports: { imports: [CROUTON] } });

		await fireEvent.click(
			await screen.findByRole('button', { name: 'Forget where these came from' }),
		);

		const sheet = await screen.findByRole('dialog');
		expect(sheet).toHaveTextContent('Forget that these recipes came from Crouton?');
		// What is KEPT leads, and Confirm puts the count in the largest type on
		// the sheet — here the reassuring fact rather than the frightening one.
		expect(sheet).toHaveTextContent('89');
		expect(sheet).toHaveTextContent('recipes stay exactly as they are');
		expect(sheet).toHaveTextContent('No recipe is deleted, renamed or changed');
		// And then what it costs, named as the act that will pay it.
		expect(sheet).toHaveTextContent('The cost comes later.');
		expect(sheet).toHaveTextContent('all 89 arrive again, as 89 new recipes beside yours');

		// Nothing has been asked of the Core: the sheet is a question.
		expect(kamosu.calls.map((call) => call.operation)).toEqual(['list_imports']);
	});

	it('words the cost for a source that remembers exactly one recipe', async () => {
		// One recipe from one web page is an ordinary import, and a single phrase
		// carrying the count produced "all 1 arrive a second time, as 1 new
		// recipes" — caught in live acceptance, not by any earlier test.
		draw(
			{
				list_imports: {
					imports: [{ ...CROUTON, source_kind: 'web', import_id: 'imp_web', remembered: 1 }],
				},
			},
			'web',
		);

		await fireEvent.click(
			await screen.findByRole('button', { name: 'Forget where these came from' }),
		);

		const sheet = await screen.findByRole('dialog');
		expect(sheet).toHaveTextContent('recipe stays exactly as it is');
		expect(sheet).toHaveTextContent(
			'the one it remembers arrives again, as a new recipe beside yours',
		);
		expect(sheet).not.toHaveTextContent('1 new recipes');
		expect(sheet).not.toHaveTextContent('all 1 arrive');
	});

	it('keeps the memory when the sheet is answered that way', async () => {
		const kamosu = draw({ list_imports: { imports: [CROUTON] } });

		await fireEvent.click(
			await screen.findByRole('button', { name: 'Forget where these came from' }),
		);
		await fireEvent.click(screen.getByRole('button', { name: 'Keep the memory' }));

		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
		expect(kamosu.calls.some((call) => call.operation === 'forget_import')).toBe(false);
	});

	it('forgets the ledger and leaves the arrivals where they were', async () => {
		const kamosu = draw({
			list_imports: { imports: [CROUTON] },
			forget_import: { import_id: 'imp_crouton', forgotten: 89 },
		});

		await fireEvent.click(
			await screen.findByRole('button', { name: 'Forget where these came from' }),
		);
		// Once forgotten the Core answers a source with no ledger and its
		// arrivals intact — the Jobs outlive the ledger, and so do their Reports.
		kamosu.answer('list_imports', {
			imports: [{ ...CROUTON, import_id: null, remembered: 0 }],
		});
		await fireEvent.click(screen.getByRole('button', { name: 'Forget' }));

		expect(
			await screen.findByText(/Forgotten\. Your recipes and this history stay as they were/),
		).toBeInTheDocument();
		expect(kamosu.calls.find((call) => call.operation === 'forget_import')?.input).toEqual({
			import_id: 'imp_crouton',
		});

		// Every Report is still one tap away, which is the whole point.
		const reports = screen
			.getAllByRole('link')
			.filter((row) => /^\/imports\/j_/.test(row.getAttribute('href') ?? ''));
		expect(reports).toHaveLength(3);
		// And there is nothing left to forget.
		expect(
			screen.queryByRole('button', { name: 'Forget where these came from' }),
		).not.toBeInTheDocument();
	});

	it('offers no forgetting for a source whose ledger is already gone', async () => {
		draw({ list_imports: { imports: [{ ...CROUTON, import_id: null, remembered: 0 }] } });

		await screen.findByText('86 recipes, already had them all');
		expect(
			screen.queryByRole('button', { name: 'Forget where these came from' }),
		).not.toBeInTheDocument();
	});

	it('offers no forgetting for recipe files, whose Import never keeps a ledger', async () => {
		// The Import row exists; its ledger is empty and always will be, because
		// a Bundle is matched on its travelling Branch id (ADR 0020). Forgetting
		// it would be a frightening button that throws away nothing.
		draw(
			{ list_imports: { imports: [{ ...CROUTON, source_kind: 'bundle', remembered: 0 }] } },
			'bundle',
		);

		await screen.findByText('86 recipes, already had them all');
		expect(
			screen.queryByRole('button', { name: 'Forget where these came from' }),
		).not.toBeInTheDocument();
	});

	it('does not tell a full library that it has imported nothing', async () => {
		// Reachable only by a stale or hand-typed URL. "Nothing has been brought
		// in yet" would be a plain lie here: three sources exist, just not this.
		draw({ list_imports: { imports: [CROUTON] } }, 'notebook');

		expect(await screen.findByText(/Nothing has come from here/)).toBeInTheDocument();
		expect(screen.queryByText(/Nothing imported yet/)).not.toBeInTheDocument();
	});
});
