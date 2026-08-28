/**
 * The Thread's shape (CONTEXT.md, "Thread"): every Branch's own chain, oldest
 * first, split into a shared trunk and the Branches that continue past it —
 * recursively, so a Branch forking off another Branch (a Copy of a Copy, or
 * a Translation of a Copy) reads the same way a fork off the original trunk
 * does.
 *
 * A Version id is never trusted to be unique across Branches — two Branches
 * that reach identical content hold the same id (ADR 0004) — so a fork is
 * found by comparing each Branch's own chain position by position within the
 * group being split, never by id alone.
 */

import type { GetThreadOutput } from '$lib/api/catalogue';

export type ThreadVersion = GetThreadOutput['versions'][number];
export type ThreadBranch = GetThreadOutput['branches'][number];

export interface ForkGroup {
	/** The Versions this whole group shares, oldest first. */
	trunk: ThreadVersion[];
	/** Where the group splits: each child holds the Branches that keep this
	 *  particular continuation, and its own further shape past this point. */
	children: { branchIds: string[]; group: ForkGroup }[];
}

/** Every Branch's own chain, oldest first — the raw material `buildGroup` splits. */
export function chainsByBranch(versions: ThreadVersion[]): Map<string, ThreadVersion[]> {
	const map = new Map<string, ThreadVersion[]>();
	for (const version of versions) {
		const list = map.get(version.branch_id) ?? [];
		list.push(version);
		map.set(version.branch_id, list);
	}
	for (const list of map.values()) list.sort((a, b) => a.sequence - b.sequence);
	return map;
}

export function buildGroup(
	branchIds: string[],
	chains: Map<string, ThreadVersion[]>,
	fromIndex: number
): ForkGroup {
	let depth = fromIndex;
	for (;;) {
		const values = branchIds.map((id) => chains.get(id)?.[depth]?.version_id);
		if (values.some((value) => value === undefined)) break;
		if (new Set(values).size > 1) break;
		depth++;
	}

	const trunk = chains.get(branchIds[0])?.slice(fromIndex, depth) ?? [];

	const groups = new Map<string, string[]>();
	for (const id of branchIds) {
		// A Branch that has nothing past `depth` gets its own unique key, so it
		// is never mistaken for sharing a continuation with another Branch that
		// also happens to end here.
		const key = chains.get(id)?.[depth]?.version_id ?? `__end_${id}`;
		const list = groups.get(key) ?? [];
		list.push(id);
		groups.set(key, list);
	}

	const children = [...groups.values()]
		.filter((ids) => !(ids.length === 1 && (chains.get(ids[0])?.length ?? 0) <= depth))
		.map((ids) => ({ branchIds: ids, group: buildGroup(ids, chains, depth) }));

	return { trunk, children };
}
