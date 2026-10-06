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
import { fireEvent, render, screen } from '@testing-library/svelte';
import { went } from '../../testing/navigation';
import { standIn, type Answers } from '$lib/api/stand-in';
import type { ListImportsOutput } from '$lib/api/catalogue';
import ImportsTestHarness from './ImportsTestHarness.svelte';
import type { Room } from '$lib/room.svelte';
import { m } from '$lib/paraglide/messages';

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

function draw(answers: Answers, room?: Room) {
	const kamosu = standIn(answers);
	render(ImportsTestHarness, { props: { client: kamosu.client, room } });
	return kamosu;
}

describe('the list of what has been brought in', () => {
	it('names the way back to Settings under its title on the phone', async () => {
		draw({ list_imports: { imports: LIBRARY } });
		await screen.findByRole('link', { name: /Crouton/ });

		expect(screen.getByRole('link', { name: `‹ ${m.imports_back()}` })).toHaveAttribute(
			'href',
			'/settings',
		);
	});

	it('leaves that line out on the wide layout, where the back arrow is the way back (#195)', async () => {
		draw({ list_imports: { imports: LIBRARY } }, 'wide');
		await screen.findByRole('link', { name: /Crouton/ });

		expect(screen.queryByRole('link', { name: `‹ ${m.imports_back()}` })).toBeNull();
	});

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

		expect(await screen.findByText(/Nothing imported yet\./, { exact: false })).toBeInTheDocument();
		expect(screen.queryByRole('link', { name: /Crouton/ })).not.toBeInTheDocument();
	});

	it('says so in the Core’s own words when it cannot read the list', async () => {
		draw({ list_imports: { refuse: 'unauthorized', message: 'this Operation needs a Person' } });

		expect(await screen.findByRole('alert')).toHaveTextContent('this Operation needs a Person');
	});
});

/**
 * Brought in beside the open Report (#200, ADR 0044).
 *
 * On the wide layout a list of every import stays while Reports are opened one
 * after another, as Foods stays beside a Food (#199). The phone keeps the full
 * pages it had, and its list is still the three sources.
 */
describe('on the wide layout', () => {
	function beside(open?: string, room: Room = 'wide') {
		const kamosu = standIn({ list_imports: { imports: LIBRARY } });
		const drawn = render(ImportsTestHarness, { props: { client: kamosu.client, room, open } });
		return { ...kamosu, ...drawn };
	}

	const reports = async () => {
		await screen.findByRole('link', { name: /Crouton/ });
		return screen
			.getAllByRole('link')
			.filter((row) => !row.getAttribute('href')?.startsWith('/imports/from/'));
	};
	const theOpenReport = () => screen.queryByLabelText('the open Report');
	const reads = (drawn: { calls: { operation: string }[] }) =>
		drawn.calls.filter((call) => call.operation === 'list_imports').length;

	it('shows the open Report beside the list, and marks its row', async () => {
		beside('j_b1');

		expect(await reports()).toHaveLength(4);
		expect(theOpenReport()).toBeInTheDocument();
		expect(screen.getByRole('link', { current: 'page' })).toHaveAttribute('href', '/imports/j_b1');
	});

	it('says to choose an import before one is open, and draws none', async () => {
		beside();

		await reports();
		expect(screen.getByText(m.imports_choose())).toBeInTheDocument();
		expect(theOpenReport()).toBeNull();
	});

	it('lists every import, whatever it came from, at the address the phone opens its Report at', async () => {
		beside();

		expect((await reports()).map((row) => row.getAttribute('href'))).toEqual([
			'/imports/j_c2',
			'/imports/j_c1',
			'/imports/j_b1',
			'/imports/j_w1',
		]);
	});

	it('writes a row as when it came in, with what happened under it', async () => {
		beside();

		const [again, first, lost] = await reports();
		expect(first).toHaveTextContent('September 19, 2026');
		expect(first).toHaveTextContent('86 recipes, all new');
		expect(again).toHaveTextContent('86 recipes, already had them all');
		expect(lost).toHaveTextContent('Nothing arrived · 1 could not be read');
	});

	it('heads each source with what it remembers, and leads to its own screen from there', async () => {
		beside();

		const crouton = await screen.findByRole('link', { name: /Crouton/ });
		expect(crouton).toHaveAttribute('href', '/imports/from/crouton');
		expect(crouton).toHaveTextContent('89 recipes remembered · 2 arrivals');
		expect(screen.getByRole('link', { name: /Recipe files/ })).toHaveAttribute(
			'href',
			'/imports/from/bundle',
		);
	});

	it('keeps the list and the reading of it while another Report is opened', async () => {
		const drawn = beside('j_c2');
		const [first] = await reports();

		await drawn.rerender({ open: 'j_w1' });

		expect((await reports())[0]).toBe(first);
		expect(reads(drawn)).toBe(1);
		expect(screen.getByRole('link', { current: 'page' })).toHaveAttribute('href', '/imports/j_w1');
	});

	it('moves down and up the list with the arrows, from one source into the next', async () => {
		const drawn = beside('j_c1');
		await reports();

		await fireEvent.keyDown(document.body, { key: 'ArrowDown' });
		// Replaced, so going back leaves the list in one step rather than walking back up it.
		expect(went).toHaveBeenLastCalledWith('/imports/j_b1', {
			replaceState: true,
			keepFocus: true,
			noScroll: true,
		});

		await drawn.rerender({ open: 'j_b1' });
		await fireEvent.keyDown(document.body, { key: 'ArrowUp' });
		expect(went).toHaveBeenLastCalledWith(
			'/imports/j_c1',
			expect.objectContaining({ replaceState: true }),
		);
	});

	it('opens the first Report on an arrow when none is open', async () => {
		beside();
		await reports();

		await fireEvent.keyDown(document.body, { key: 'ArrowDown' });

		expect(went).toHaveBeenLastCalledWith(
			'/imports/j_c2',
			expect.objectContaining({ replaceState: true }),
		);
	});

	it('stops at the ends of the list', async () => {
		beside('j_w1');
		await reports();

		await fireEvent.keyDown(document.body, { key: 'ArrowDown' });

		expect(went).not.toHaveBeenCalled();
	});

	it('leaves the arrows to a control of the Report that uses them', async () => {
		beside('j_c1');
		await reports();

		await fireEvent.keyDown(screen.getByLabelText('the open Report'), { key: 'ArrowDown' });

		expect(went).not.toHaveBeenCalled();
	});

	it('puts the caret on the row an arrow opened, so Tab goes on into that Report', async () => {
		beside('j_c2');
		const [, second] = await reports();

		await fireEvent.keyDown(document.body, { key: 'ArrowDown' });

		expect(second).toHaveFocus();
	});

	it('lets Tab reach one row of the list and no more, so the Report is one Tab away', async () => {
		beside('j_b1');

		const rows = await reports();
		expect(rows.filter((row) => row.tabIndex === 0).map((row) => row.getAttribute('href'))).toEqual(
			['/imports/j_b1'],
		);
	});

	it('reads the list again when the Report beside it watched its import end', async () => {
		const running: Import = {
			...LIBRARY[2],
			arrivals: [arrival('j_w1', '2026-09-01T10:00:00.000Z', { status: 'running', arrived: 0 })],
		};
		const kamosu = standIn({ list_imports: { imports: [running] } });
		render(ImportsTestHarness, { props: { client: kamosu.client, room: 'wide', open: 'j_w1' } });
		expect(await screen.findByText(m.imports_row_running())).toBeInTheDocument();

		kamosu.answer('list_imports', { imports: [LIBRARY[2]] });
		await fireEvent.click(screen.getByRole('button', { name: 'it ended' }));

		expect(await screen.findByText(m.imports_row_new_one())).toBeInTheDocument();
		expect(screen.queryByText(m.imports_row_running())).toBeNull();
	});

	it('reads the list again when a row that says running is opened, since it may have ended unwatched', async () => {
		const running: Import = {
			...LIBRARY[2],
			arrivals: [arrival('j_w1', '2026-09-01T10:00:00.000Z', { status: 'running', arrived: 0 })],
		};
		const kamosu = standIn({ list_imports: { imports: [running] } });
		const drawn = render(ImportsTestHarness, { props: { client: kamosu.client, room: 'wide' } });
		expect(await screen.findByText(m.imports_row_running())).toBeInTheDocument();

		kamosu.answer('list_imports', { imports: [LIBRARY[2]] });
		await drawn.rerender({ open: 'j_w1' });

		expect(await screen.findByText(m.imports_row_new_one())).toBeInTheDocument();
		expect(reads(kamosu)).toBe(2);
	});

	it('keeps the list that is drawn when reading it again is refused', async () => {
		const drawn = beside('j_w1');
		await reports();

		drawn.answer('list_imports', { refuse: 'internal', message: 'not now' });
		await fireEvent.click(screen.getByRole('button', { name: 'it ended' }));
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(reads(drawn)).toBe(2);
		expect(await reports()).toHaveLength(4);
		expect(screen.queryByRole('alert')).toBeNull();
	});

	it('is side by side where the window is roomy too', async () => {
		beside('j_c1', 'roomy');

		expect(await reports()).toHaveLength(4);
		expect(theOpenReport()).toBeInTheDocument();
	});
});

describe('on the phone layout', () => {
	function phone(open?: string) {
		const kamosu = standIn({ list_imports: { imports: LIBRARY } });
		const drawn = render(ImportsTestHarness, { props: { client: kamosu.client, open } });
		return { ...kamosu, ...drawn };
	}

	it('draws a Report as a full page, with no list beside it and none read', async () => {
		const drawn = phone('j_c1');

		expect(screen.getByLabelText('the open Report')).toBeInTheDocument();
		await new Promise((resolve) => setTimeout(resolve, 0));
		expect(screen.queryAllByRole('link')).toHaveLength(0);
		expect(drawn.calls).toHaveLength(0);
	});

	it('lists the sources and no import, with no sentence about choosing', async () => {
		phone();

		await screen.findByRole('link', { name: /Crouton/ });
		expect(
			screen
				.getAllByRole('link')
				.filter((row) => /^\/imports\/j_/.test(row.getAttribute('href') ?? '')),
		).toHaveLength(0);
		expect(screen.queryByText(m.imports_choose())).toBeNull();
	});

	it('does nothing on an arrow', async () => {
		phone();
		await screen.findByRole('link', { name: /Crouton/ });

		await fireEvent.keyDown(document.body, { key: 'ArrowDown' });

		expect(went).not.toHaveBeenCalled();
	});
});
