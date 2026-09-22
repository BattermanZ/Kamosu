/**
 * Brought in: the list of sources (#108).
 *
 * What this screen is tested on is reachability, because unreachability is the
 * bug it exists to fix. Before it, one Report out of forty-eight could be
 * opened. So: every source is listed whichever importer made it, a source whose
 * ledger has been forgotten is still listed, and an instance that has imported
 * nothing says so rather than showing an empty box.
 */

import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import type { ListImportsOutput } from '$lib/api/catalogue';
import ImportsTestHarness from './ImportsTestHarness.svelte';

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

/** The dev instance's shape: three sources, most arrivals under one of them. */
const LIBRARY: Import[] = [
	{
		import_id: 'imp_crouton',
		source_kind: 'crouton',
		created_at: '2026-08-29T16:16:00.000Z',
		remembered: 89,
		arrivals: [
			arrival('j_c2', '2026-09-19T12:55:00.000Z', { arrived: 86, created: 0 }),
			arrival('j_c1', '2026-09-19T12:53:00.000Z', { arrived: 86, created: 86 }),
		],
	},
	{
		import_id: 'imp_bundle',
		source_kind: 'bundle',
		created_at: '2026-09-18T15:19:00.000Z',
		remembered: 42,
		arrivals: [
			arrival('j_b1', '2026-09-21T11:43:00.000Z', { arrived: 0, created: 0, unreadable: 1 }),
		],
	},
	{
		import_id: 'imp_web',
		source_kind: 'web',
		created_at: '2026-09-01T09:52:00.000Z',
		remembered: 3,
		arrivals: [arrival('j_w1', '2026-09-01T10:00:00.000Z')],
	},
];

function draw(answers: Answers) {
	const kamosu = standIn(answers);
	render(ImportsTestHarness, { props: { client: kamosu.client } });
	return kamosu;
}

describe('the list of what has been brought in', () => {
	it('lists every source, whichever importer made it', async () => {
		draw({ list_imports: { imports: LIBRARY } });

		// Waited for by name, not by count: the way back to Settings is a link
		// too and it is on screen before the list has been read.
		await screen.findByRole('link', { name: /Crouton/ });
		const sources = screen
			.getAllByRole('link')
			.filter((row) => row.getAttribute('href')?.startsWith('/imports/from/'));
		expect(sources.map((row) => row.getAttribute('href'))).toEqual([
			'/imports/from/crouton',
			'/imports/from/bundle',
			'/imports/from/web',
		]);
		expect(sources[0]).toHaveTextContent('Crouton');
		expect(sources[1]).toHaveTextContent('Recipe files');
		expect(sources[2]).toHaveTextContent('Web links');
	});

	it('says how much a source remembers and how often it has been used', async () => {
		draw({ list_imports: { imports: LIBRARY } });

		const crouton = await screen.findByRole('link', { name: /Crouton/ });
		expect(crouton).toHaveTextContent('89 recipes remembered');
		expect(crouton).toHaveTextContent('2 arrivals');
		// The newest arrival, not the date the channel was opened in August.
		expect(crouton).toHaveTextContent('Last September 19, 2026');
	});

	it('still lists a source whose ledger has been forgotten', async () => {
		// Forgetting throws away which recipe became which — never the record of
		// what happened. Hiding these rows would put their Reports back out of
		// reach, which is the whole bug this screen closes.
		draw({
			list_imports: {
				imports: [{ ...LIBRARY[0], import_id: null, remembered: 0 }],
			},
		});

		const crouton = await screen.findByRole('link', { name: /Crouton/ });
		expect(crouton).toHaveTextContent('2 arrivals');
		// An empty ledger is left unsaid, not counted at zero: "Nothing
		// remembered" beside two arrivals reads as damage.
		expect(crouton).not.toHaveTextContent('remembered');
	});

	it('counts no ledger for recipe files, which never keep one', async () => {
		// A Bundle carries the sender's travelling Branch id and is matched on
		// that (ADR 0020), so `import_bundle` writes no ledger row at all. The
		// dev instance has 40 such arrivals and remembers none of them.
		draw({ list_imports: { imports: [{ ...LIBRARY[1], remembered: 0 }] } });

		const files = await screen.findByRole('link', { name: /Recipe files/ });
		expect(files).toHaveTextContent('1 arrival');
		expect(files).not.toHaveTextContent('remembered');
	});

	it('names a source an agent invented rather than inventing a name for it', async () => {
		// `import` takes whatever `source_kind` an agent passes, so the set is
		// open and Paraglide has no phrase for a kind nobody foresaw.
		draw({
			list_imports: {
				imports: [{ ...LIBRARY[2], source_kind: 'notebook', import_id: 'imp_n' }],
			},
		});

		expect(await screen.findByRole('link', { name: /notebook/ })).toHaveAttribute(
			'href',
			'/imports/from/notebook',
		);
	});

	it('explains an instance that has never imported anything', async () => {
		draw({ list_imports: { imports: [] } });

		expect(
			await screen.findByText(/Nothing has been brought in yet\./, { exact: false }),
		).toBeInTheDocument();
		expect(screen.queryByRole('link', { name: /Crouton/ })).not.toBeInTheDocument();
	});

	it('says so in the Core’s own words when it cannot read the list', async () => {
		draw({ list_imports: { refuse: 'unauthorized', message: 'this Operation needs a Person' } });

		expect(await screen.findByRole('alert')).toHaveTextContent('this Operation needs a Person');
	});
});
