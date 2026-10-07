/**
 * The Thread drawn as a graph (#115, choice "G1"): one row per
 * Version, every row in date order across every Branch, as `git log --graph`
 * reads. A dot per Version sits on a line down the left edge — grey while
 * several Branches still share those Versions, a colour of its own for each
 * Branch from where it splits — and each split is a row of its own.
 *
 * This file only decides the shape: which line a Version sits on, which
 * lines pass each row, where a split happens. It knows nothing about pixels
 * or colours, so the drawing can change without the shape moving, and the
 * shape can be tested without a screen.
 *
 * It is built on `buildGroup` (tree.ts), which already knows where Branches
 * part — by comparing each Branch's own chain position by position, never by
 * Version id alone (ADR 0004).
 */

import type { RenameVersionInput } from '$lib/api/catalogue';
import { buildGroup, chainsByBranch, type ForkGroup, type ThreadVersion } from './tree';

/**
 * Which colour a line is drawn in: `'shared'` while several Branches share it
 * (grey), otherwise the Branch's place in the Thread's list of Branches — the
 * screen cycles its colours over that number.
 */
export type Tone = 'shared' | number;

/**
 * One line passing through a row. `reach` says which part of the row it
 * covers: all of it, only down to the dot (the line ends here), or only from
 * the dot down (the line starts here).
 */
/** A lane and the colour it is drawn in at that moment. */
export interface Lane {
	lane: number;
	tone: Tone;
}

export interface Line extends Lane {
	reach: 'full' | 'above' | 'below' | 'none';
}

/**
 * Where one Version occurs — what `rename_version` targets (#115) — and whose
 * Hand saved that occurrence, since only that Hand may rename it.
 */
export type Occurrence = Pick<RenameVersionInput, 'branch_id' | 'sequence'> & { hand_id: string };

/** How a row is known: one occurrence on one Branch. */
export const rowKey = (version: Pick<ThreadVersion, 'branch_id' | 'sequence'>) =>
	`${version.branch_id}:${version.sequence}`;

export type GraphRow =
	| {
			kind: 'version';
			key: string;
			/** The occurrence the screen reads from — the first of `occurrences`. */
			version: ThreadVersion;
			/**
			 * Every Branch this row stands for holds the Version at the same place
			 * in its chain, each as an occurrence of its own. A shared Version is
			 * drawn once, so renaming it renames it on each of them.
			 */
			occurrences: Occurrence[];
			branchIds: string[];
			lane: number;
			tone: Tone;
			lines: Line[];
			/** Whether the row says which Branch it is on: every row after the first split. */
			labelled: boolean;
	  }
	| {
			kind: 'fork';
			key: string;
			/** The lane that splits. */
			from: number;
			fromTone: Tone;
			/** Each continuation, in the order the Thread lists its Branches. */
			children: (Lane & { branchIds: string[] })[];
			/** Lines that carry straight on past the split. */
			passing: Lane[];
	  };

export interface Graph {
	rows: GraphRow[];
	/** How many lanes the widest moment needs — the gutter's width. */
	lanes: number;
}

/** Which part of a row a line covers, from whether it reaches the top and the bottom. */
const reaches = (top: boolean, bottom: boolean): Line['reach'] =>
	top ? (bottom ? 'full' : 'above') : bottom ? 'below' : 'none';
const hasTop = (reach: Line['reach']) => reach === 'full' || reach === 'above';
const hasBottom = (reach: Line['reach']) => reach === 'full' || reach === 'below';

interface Segment {
	branchIds: string[];
	trunk: ThreadVersion[];
	children: ForkGroup['children'];
	tone: Tone;
	lane: number;
	/** How many of `trunk` have been drawn. */
	next: number;
	/** Whether a line reaches this segment's first row from above. */
	fromAbove: boolean;
	/** The order segments were met in — the tie-break between equal dates. */
	order: number;
}

/**
 * Lay the Thread out. `branchIds` is the Thread's own order of Branches
 * (oldest first), which is also the order colours are handed out in.
 */
export function graphOf(branchIds: string[], versions: ThreadVersion[]): Graph {
	const chains = chainsByBranch(versions);
	const root = buildGroup(branchIds, chains, 0);
	const toneOf = (ids: string[]): Tone => (ids.length === 1 ? branchIds.indexOf(ids[0]) : 'shared');

	const rows: GraphRow[] = [];
	const lanes: (Segment | null)[] = [];
	const ready: Segment[] = [];
	let order = 0;
	let forked = false;
	let widest = 0;

	const place = (segment: Segment, lane?: number) => {
		const at = lane ?? firstFree();
		segment.lane = at;
		lanes[at] = segment;
		widest = Math.max(widest, lanes.length);
	};
	const firstFree = () => {
		const at = lanes.indexOf(null);
		return at === -1 ? lanes.length : at;
	};
	const segmentOf = (group: ForkGroup, ids: string[], fromAbove: boolean): Segment => ({
		branchIds: ids,
		trunk: group.trunk,
		children: group.children,
		tone: toneOf(ids),
		lane: -1,
		next: 0,
		fromAbove,
		order: order++,
	});

	/** Draw what a segment turns into once its own Versions are all drawn. */
	const finish = (segment: Segment) => {
		const children = segment.children;
		if (children.length === 0) {
			lanes[segment.lane] = null;
			trimLanes();
			return;
		}
		if (children.length === 1) {
			// Nothing splits: the one continuation carries on in the same lane.
			const child = segmentOf(children[0].group, children[0].branchIds, true);
			place(child, segment.lane);
			start(child);
			return;
		}
		forked = true;
		const passing = liveLanes().filter((lane) => lane.lane !== segment.lane);
		const made = children.map((child, index) => {
			const next = segmentOf(child.group, child.branchIds, true);
			if (index === 0) place(next, segment.lane);
			return next;
		});
		for (const next of made.slice(1)) place(next);
		rows.push({
			kind: 'fork',
			key: `fork:${segment.branchIds.join(',')}:${segment.trunk.length}:${segment.order}`,
			from: segment.lane,
			fromTone: segment.tone,
			children: made.map((next) => ({
				lane: next.lane,
				tone: next.tone,
				branchIds: next.branchIds,
			})),
			passing,
		});
		for (const next of made) start(next);
	};

	const start = (segment: Segment) => {
		if (segment.trunk.length === 0) finish(segment);
		else ready.push(segment);
	};

	const liveLanes = (): Lane[] =>
		lanes.flatMap((segment) => (segment ? [{ lane: segment.lane, tone: segment.tone }] : []));
	const trimLanes = () => {
		while (lanes.length && lanes[lanes.length - 1] === null) lanes.pop();
	};

	const first = segmentOf(root, branchIds, false);
	place(first, 0);
	start(first);

	while (ready.length) {
		// The segment whose next Version is the oldest; equal dates go in the
		// order the segments were met, so a split's first Branch comes first.
		ready.sort(
			(a, b) =>
				a.trunk[a.next].created_at.localeCompare(b.trunk[b.next].created_at) || a.order - b.order,
		);
		const segment = ready[0];
		const version = segment.trunk[segment.next];
		const isFirst = segment.next === 0;
		const isLast = segment.next === segment.trunk.length - 1;
		const ends = isLast && segment.children.length === 0;
		const above = !isFirst || segment.fromAbove;
		const own = reaches(above, !ends);

		const position = chains.get(segment.branchIds[0])?.indexOf(version) ?? -1;
		const occurrences = segment.branchIds.flatMap((id) => {
			const occurrence = chains.get(id)?.[position];
			return occurrence
				? [{ branch_id: id, sequence: occurrence.sequence, hand_id: occurrence.hand_id }]
				: [];
		});

		rows.push({
			kind: 'version',
			key: rowKey(version),
			version,
			occurrences,
			branchIds: segment.branchIds,
			lane: segment.lane,
			tone: segment.tone,
			lines: liveLanes().map(({ lane, tone }) => ({
				lane,
				tone,
				reach: lane === segment.lane ? own : 'full',
			})),
			labelled: forked,
		});

		segment.next++;
		if (segment.next === segment.trunk.length) {
			ready.shift();
			finish(segment);
		}
	}

	return { rows, lanes: Math.max(widest, 1) };
}

export type VersionRow = Extract<GraphRow, { kind: 'version' }>;

/** A row as the screen shows it: on its own, or folded into a run of quiet saves. */
export type Item =
	{ kind: 'row'; row: GraphRow } | { kind: 'run'; key: string; rows: VersionRow[] };

const RUN_THRESHOLD = 4;

/**
 * Fold runs of quiet saves — four or more in a row, on the same line, with
 * nothing else drawn between them — into one row the cook can open. What
 * counts as quiet is the screen's to say.
 */
export function runsOf(rows: GraphRow[], isQuiet: (row: VersionRow) => boolean): Item[] {
	const items: Item[] = [];
	let run: VersionRow[] = [];
	const flush = () => {
		if (run.length >= RUN_THRESHOLD) {
			items.push({ kind: 'run', key: `run:${run[0].key}`, rows: run });
		} else {
			for (const row of run) items.push({ kind: 'row', row });
		}
		run = [];
	};
	for (const row of rows) {
		if (row.kind === 'version' && isQuiet(row)) {
			if (run.length && run[run.length - 1].lane !== row.lane) flush();
			run.push(row);
			continue;
		}
		flush();
		items.push({ kind: 'row', row });
	}
	flush();
	return items;
}

/**
 * The lines a folded run draws as one row: its own line reaches as far up as
 * the run's first row does and as far down as its last does. Nothing else
 * can start or stop inside a run, since another line only does that on a row
 * of its own, which would have broken the run.
 */
export function foldedLines(rows: VersionRow[]): Line[] {
	const first = rows[0];
	const last = rows[rows.length - 1];
	const lastOwn = last.lines.find((line) => line.lane === last.lane);
	return first.lines.map((line) =>
		line.lane === first.lane
			? { ...line, reach: reaches(hasTop(line.reach), hasBottom(lastOwn?.reach ?? 'none')) }
			: line,
	);
}

/** The lines that carry on below a row, drawn the whole height of a row with no dot. */
export function linesBelow(row: VersionRow): Line[] {
	return row.lines
		.filter((line) => hasBottom(line.reach))
		.map((line) => ({ ...line, reach: 'full' }));
}
