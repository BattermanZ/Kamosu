import { describe, expect, it } from 'vitest';
import { foldedLines, graphOf, linesBelow, runsOf, type GraphRow } from './graph';
import type { ThreadVersion } from './tree';

let clock = 0;
function v(
	branch_id: string,
	sequence: number,
	version_id: string,
	created_at: string,
	extra: Partial<ThreadVersion> = {},
): ThreadVersion {
	clock++;
	return {
		branch_id,
		sequence,
		version_id,
		parent_version_id: null,
		hand_id: 'h_me',
		hand_name: 'Me',
		name: null,
		change_note: `note ${clock}`,
		created_at,
		translates_version_id: null,
		language: 'en',
		...extra,
	};
}

/** Each version row as `id@lane`, fork rows as `fork lane→[lanes]`. */
function shape(rows: GraphRow[]): string[] {
	return rows.map((row) =>
		row.kind === 'version'
			? `${row.version.version_id}@${row.lane}`
			: `fork ${row.from}→[${row.children.map((child) => child.lane).join(',')}]`,
	);
}

describe('the Thread as a graph (#115)', () => {
	it('draws one Branch as one line in its own colour, starting and ending at a dot', () => {
		const graph = graphOf(
			['b_1'],
			[
				v('b_1', 1, 'a', '2026-01-01'),
				v('b_1', 2, 'b', '2026-01-02'),
				v('b_1', 3, 'c', '2026-01-03'),
			],
		);
		expect(shape(graph.rows)).toEqual(['a@0', 'b@0', 'c@0']);
		expect(graph.lanes).toBe(1);
		const [first, middle, last] = graph.rows as Extract<GraphRow, { kind: 'version' }>[];
		expect(first.lines).toEqual([{ lane: 0, tone: 0, reach: 'below' }]);
		expect(middle.lines).toEqual([{ lane: 0, tone: 0, reach: 'full' }]);
		expect(last.lines).toEqual([{ lane: 0, tone: 0, reach: 'above' }]);
		// A Thread that never splits labels no row with a Branch.
		expect(first.labelled).toBe(false);
	});

	it('greys what several Branches share, splits into a colour each, and interleaves them by date', () => {
		const graph = graphOf(
			['b_papi', 'b_camille', 'b_chalet'],
			[
				v('b_papi', 1, 'root', '2026-08-02'),
				v('b_camille', 1, 'root', '2026-08-02'),
				v('b_chalet', 1, 'root', '2026-08-02'),
				v('b_papi', 2, 'p1', '2026-08-30'),
				v('b_camille', 2, 'c1', '2026-09-05'),
				v('b_papi', 3, 'p2', '2026-09-12'),
				v('b_chalet', 2, 'f1', '2026-09-15'),
				v('b_camille', 3, 'c2', '2026-09-21'),
			],
		);
		expect(shape(graph.rows)).toEqual([
			'root@0',
			'fork 0→[0,1,2]',
			'p1@0',
			'c1@1',
			'p2@0',
			'f1@2',
			'c2@1',
		]);
		expect(graph.lanes).toBe(3);

		const root = graph.rows[0] as Extract<GraphRow, { kind: 'version' }>;
		expect(root.tone).toBe('shared');
		// The shared Version is one row, standing for its occurrence on every Branch.
		expect(root.occurrences).toEqual([
			{ branch_id: 'b_papi', sequence: 1, hand_id: 'h_me' },
			{ branch_id: 'b_camille', sequence: 1, hand_id: 'h_me' },
			{ branch_id: 'b_chalet', sequence: 1, hand_id: 'h_me' },
		]);
		expect(root.labelled).toBe(false);

		const fork = graph.rows[1] as Extract<GraphRow, { kind: 'fork' }>;
		expect(fork.children.map((child) => [child.branchIds, child.tone])).toEqual([
			[['b_papi'], 0],
			[['b_camille'], 1],
			[['b_chalet'], 2],
		]);

		// Camille's first Version: her line runs through, Papi's carries on
		// past it, and the Chalet's has not had a Version yet but is already drawn.
		const c1 = graph.rows[3] as Extract<GraphRow, { kind: 'version' }>;
		expect(c1.tone).toBe(1);
		expect(c1.labelled).toBe(true);
		expect(c1.lines).toEqual([
			{ lane: 0, tone: 0, reach: 'full' },
			{ lane: 1, tone: 1, reach: 'full' },
			{ lane: 2, tone: 2, reach: 'full' },
		]);
		// Papi's last Version ends his line at its dot; the others carry on.
		const p2 = graph.rows[4] as Extract<GraphRow, { kind: 'version' }>;
		expect(p2.lines).toEqual([
			{ lane: 0, tone: 0, reach: 'above' },
			{ lane: 1, tone: 1, reach: 'full' },
			{ lane: 2, tone: 2, reach: 'full' },
		]);
		const c2 = graph.rows[6] as Extract<GraphRow, { kind: 'version' }>;
		expect(c2.lines).toEqual([{ lane: 1, tone: 1, reach: 'above' }]);
	});

	it('never puts a Branch’s Version above the split it comes from, whatever its date says', () => {
		// A Copy's own first save can carry a clock earlier than the Version it
		// was copied from (another device, a wrong clock). The split still comes first.
		const graph = graphOf(
			['b_1', 'b_2'],
			[
				v('b_1', 1, 'root', '2026-05-01'),
				v('b_2', 1, 'root', '2026-05-01'),
				v('b_1', 2, 'x', '2026-04-01'),
				v('b_2', 2, 'y', '2026-06-01'),
			],
		);
		expect(shape(graph.rows)).toEqual(['root@0', 'fork 0→[0,1]', 'x@0', 'y@1']);
	});

	it('forks again off a Branch, reusing a lane another Branch has finished with', () => {
		const graph = graphOf(
			['b_mine', 'b_marc', 'b_camille'],
			[
				v('b_mine', 1, 'v1', '2026-03-03'),
				v('b_marc', 1, 'v1', '2026-03-03'),
				v('b_camille', 1, 'v1', '2026-03-03'),
				v('b_mine', 2, 'v2', '2026-04-01'),
				v('b_marc', 2, 'm1', '2026-06-02'),
				v('b_camille', 2, 'm1', '2026-06-02'),
				v('b_marc', 3, 'm2', '2026-08-10'),
				v('b_camille', 3, 'c1', '2026-08-12'),
			],
		);
		expect(shape(graph.rows)).toEqual([
			'v1@0',
			'fork 0→[0,1]',
			'v2@0',
			'm1@1',
			'fork 1→[1,0]',
			'm2@1',
			'c1@0',
		]);
		// Marc and Camille still share m1, so it is grey and names them both.
		const m1 = graph.rows[3] as Extract<GraphRow, { kind: 'version' }>;
		expect(m1.tone).toBe('shared');
		expect(m1.branchIds).toEqual(['b_marc', 'b_camille']);
		expect(m1.labelled).toBe(true);
		expect(graph.lanes).toBe(2);
	});

	it('folds four or more quiet saves in a row on one line, and leaves a shorter run alone', () => {
		const quiet = (id: string, seq: number, day: number) =>
			v('b_1', seq, id, `2026-01-0${day}`, { change_note: null });
		const graph = graphOf(
			['b_1'],
			[
				v('b_1', 1, 'a', '2026-01-01'),
				quiet('q1', 2, 2),
				quiet('q2', 3, 3),
				quiet('q3', 4, 4),
				quiet('q4', 5, 5),
				v('b_1', 6, 'b', '2026-01-06'),
				quiet('q5', 7, 7),
			],
		);
		const items = runsOf(graph.rows, (row) => !row.version.change_note && !row.version.name);
		expect(
			items.map((item) =>
				item.kind === 'run'
					? `run ${item.rows.map((row) => row.version.version_id).join(',')}`
					: item.row.kind === 'version'
						? item.row.version.version_id
						: 'fork',
			),
		).toEqual(['a', 'run q1,q2,q3,q4', 'b', 'q5']);

		// Folded, the run is one row on a line that carries on through it.
		const run = items[1];
		if (run.kind !== 'run') throw new Error('expected a run');
		expect(foldedLines(run.rows)).toEqual([{ lane: 0, tone: 0, reach: 'full' }]);
		// The last Version ends the line; nothing carries on below it.
		const last = graph.rows[graph.rows.length - 1];
		if (last.kind !== 'version') throw new Error('expected a Version');
		expect(linesBelow(last)).toEqual([]);
	});
});
