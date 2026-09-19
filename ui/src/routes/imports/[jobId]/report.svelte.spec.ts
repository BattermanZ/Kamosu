/**
 * The screen-seam test: the Import Report for a Crouton library (#69),
 * option A as Aurélien chose it — one page, three parts.
 *
 * `get_job`'s declared result is "anything", so the stand-in cannot hold the
 * Report to its shape here. The generated `ImportCroutonOutput` type does
 * instead: every report below is typed as one, so a field renamed in
 * `src/catalogue.rs` fails svelte-check in the same commit.
 *
 * What these tests are for is what no behaviour test can reach: that the
 * three parts are drawn as three different kinds of thing, that the page is
 * already there while the Job runs, and that ticking relates exactly the pairs
 * ticked and nothing else.
 */

import { describe, expect, it } from 'vitest';
import { render, screen, fireEvent, within } from '@testing-library/svelte';
import ReportTestHarness from './ReportTestHarness.svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import type { GetJobOutput, ImportCroutonOutput } from '$lib/api/catalogue';

const side = (branch: string, title: string, over: Record<string, unknown> = {}) => ({
	lineage_id: `l_${branch}`,
	branch_id: `b_${branch}`,
	title,
	main_photo: null,
	ingredients: 11,
	...over,
});

const arrived = (branch: string, title: string, over: Record<string, unknown> = {}) => ({
	foreign_id: `crouton-${branch}`,
	status: 'created' as const,
	lineage_id: `l_${branch}`,
	branch_id: `b_${branch}`,
	title,
	main_photo: `photo-${branch}`,
	bare: false,
	...over,
});

const report = (over: Partial<ImportCroutonOutput> = {}): ImportCroutonOutput => ({
	import_id: 'i_1',
	kitchen_id: 'k_1',
	source_kind: 'crouton',
	arrived: [
		arrived('beef1', 'Beef Bourguignon'),
		arrived('beef2', 'Beef Bourguignon'),
		arrived('kfc1', 'Korean Fried Chicken'),
		arrived('kfc2', 'Korean Fried Chicken'),
		arrived('dan', 'Dan Dan Noodles', { main_photo: null, bare: true }),
		arrived('ribs', 'Beef Short Ribs'),
	],
	offered: [],
	unreadable: [],
	left_out: [
		{
			foreign_id: 'crouton-kfc1',
			branch_id: 'b_kfc1',
			title: 'Korean Fried Chicken',
			what: 'site_icon',
			count: 1,
			icon: 'data:image/png;base64,iVBORw0KGgo=',
		},
		{
			foreign_id: 'crouton-kfc2',
			branch_id: 'b_kfc2',
			title: 'Korean Fried Chicken',
			what: 'site_icon',
			count: 1,
			icon: 'data:image/png;base64,iVBORw0KGgo=',
		},
		{
			foreign_id: 'crouton-ribs',
			branch_id: 'b_ribs',
			title: 'Beef Short Ribs',
			what: 'extra_photos',
			count: 1,
		},
	],
	related_candidates: [
		{
			recipes: [
				side('beef1', 'Beef Bourguignon'),
				side('beef2', 'Beef Bourguignon', { ingredients: 16 }),
			],
			shared: ['name'],
		},
		{
			recipes: [side('kfc1', 'Korean Fried Chicken'), side('kfc2', 'Korean Fried Chicken')],
			shared: ['name', 'page'],
		},
	],
	...over,
});

const job = (over: Partial<GetJobOutput> = {}): GetJobOutput => ({
	id: 'j_1',
	operation: 'import_crouton',
	status: 'completed',
	progress: { done: 86, total: 86, message: 'finished' },
	result: report(),
	error: null,
	errorCode: null,
	created_at: '2026-09-19T10:00:00.000Z',
	updated_at: '2026-09-19T10:02:00.000Z',
	...over,
});

function open(answers: Answers) {
	localStorage.clear();
	const kamosu = standIn(answers);
	render(ReportTestHarness, { props: { client: kamosu.client, jobId: 'j_1' } });
	return kamosu;
}

describe('the Import Report', () => {
	it('is already a page while the import runs, and becomes the Report when it ends', async () => {
		const kamosu = open({
			get_job: job({
				status: 'running',
				progress: { done: 37, total: 86, message: '37 of 86' },
				result: null,
			}),
		});

		expect(await screen.findByText(/37 of 86 recipes in so far/)).toBeInTheDocument();
		expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '37');
		// The pairs need both halves in, so the work is promised, not shown.
		expect(screen.getByText(/Once everything is in/)).toBeInTheDocument();

		kamosu.answer('get_job', job());
		expect(await screen.findByText(/6 recipes are on your shelf/)).toBeInTheDocument();
		expect(screen.getByText(/2 pairs might be related/)).toBeInTheDocument();
	});

	it('relates exactly the pairs ticked, then says what was linked', async () => {
		const kamosu = open({
			get_job: job(),
			set_related_recipe: { related_recipes: [] },
		});

		const pairs = await screen.findByText(/Tick the ones that are two takes/);
		const card = pairs.closest('div') as HTMLElement;
		const relate = within(card).getByRole('button', { name: 'Relate pairs' });
		expect(relate).toBeDisabled();

		const boxes = within(card).getAllByRole('checkbox');
		expect(boxes).toHaveLength(2);
		expect(within(card).getByText(/same name · same page/)).toBeInTheDocument();
		expect(within(card).getByText(/11 and 16 ingredients/)).toBeInTheDocument();
		await fireEvent.click(boxes[1]);
		await fireEvent.click(within(card).getByRole('button', { name: 'Relate 1 pairs' }));

		expect(
			await screen.findByText(/Linked: Korean Fried Chicken. The other 1 kept apart./),
		).toBeInTheDocument();
		const related = kamosu.calls.filter((call) => call.operation === 'set_related_recipe');
		expect(related.map((call) => call.input)).toEqual([
			{ branch_id: 'b_kfc1', related_branch_id: 'b_kfc2', related: true },
		]);
	});

	it('unlinks a pair again when the answer is changed to "None of them"', async () => {
		const kamosu = open({
			get_job: job(),
			set_related_recipe: { related_recipes: [] },
		});
		const boxes = await screen.findAllByRole('checkbox');
		await fireEvent.click(boxes[0]);
		await fireEvent.click(boxes[1]);
		await fireEvent.click(screen.getByRole('button', { name: 'Relate 2 pairs' }));
		expect(
			await screen.findByText(/Linked: Beef Bourguignon, Korean Fried Chicken\./),
		).toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: 'Change' }));
		await fireEvent.click(screen.getByRole('button', { name: 'None of them' }));

		expect(await screen.findByText(/None linked: all 2 kept apart./)).toBeInTheDocument();
		const answers = kamosu.calls
			.filter((call) => call.operation === 'set_related_recipe')
			.map((call) => (call.input as { related: boolean }).related);
		expect(answers).toEqual([true, true, false, false]);
	});

	it('keeps every pair apart on "None of them" without touching a recipe', async () => {
		const kamosu = open({ get_job: job() });
		await fireEvent.click(await screen.findByRole('button', { name: 'None of them' }));
		expect(await screen.findByText(/None linked: all 2 kept apart./)).toBeInTheDocument();
		expect(kamosu.calls.some((call) => call.operation === 'set_related_recipe')).toBe(false);
	});

	it('names a file it could not read, with what to do about it', async () => {
		open({
			get_job: job({
				result: report({
					unreadable: [
						{
							foreign_id: null,
							name: 'Îles Flottantes.crumb',
							reason: 'the file is damaged; export the library from Crouton again',
						},
					],
				}),
			}),
		});
		expect(await screen.findByText('Îles Flottantes.crumb')).toBeInTheDocument();
		expect(screen.getByText(/export the library from Crouton again/)).toBeInTheDocument();
		expect(screen.getByText(/1 couldn't be read/)).toBeInTheDocument();
	});

	it('says every file was read when none was lost', async () => {
		open({ get_job: job() });
		expect(await screen.findByText('Every file could be read.')).toBeInTheDocument();
	});

	it('says what arrived, what came bare, and what was left out on purpose', async () => {
		open({ get_job: job() });
		expect(await screen.findByText('6 recipes · 5 photos')).toBeInTheDocument();
		expect(
			screen.getByText(/1 came as just a name and a link: Dan Dan Noodles/),
		).toBeInTheDocument();
		expect(screen.getByText(/Left out: 2 website icons/)).toBeInTheDocument();
		// The two recipes share one icon, so it is drawn once.
		expect(document.querySelectorAll('img[src^="data:image/png"]')).toHaveLength(1);
		expect(screen.getByText(/one extra photo from Beef Short Ribs/)).toBeInTheDocument();
		expect(screen.getByText('Show all 6')).toBeInTheDocument();
	});

	it('says why an import stopped', async () => {
		open({
			get_job: job({
				status: 'failed',
				result: null,
				error: 'this zip holds no .crumb files, so it is not a Crouton export',
			}),
		});
		expect(await screen.findByRole('alert')).toHaveTextContent(/not a Crouton export/);
	});
});
