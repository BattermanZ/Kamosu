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
	fromIndex: number,
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

/**
 * **Which occurrences said what Language this recipe is in** (#106, ADR 0006),
 * as a map from a row's key to the Language it settled on.
 *
 * Saying a recipe's Language appends an occurrence of the head content
 * carrying the new Language — the same Version, occurring twice, told apart by
 * its sequence. That is what makes the change leave a trace in an append-only
 * history, and it is the one thing separating it from tagging (#104) and
 * relating (#105), neither of which the Thread ever hears about.
 *
 * Without this, that trace reads as a second identical row with nothing to say
 * for itself — the Version is there, but nobody can see WHY. The Language of
 * an occurrence is answered per occurrence precisely so this is answerable, so
 * the comparison is against the previous occurrence ON THE SAME BRANCH rather
 * than against the Branch's Language now, which is only ever the latest one.
 */
export function languageSaidAt(versions: ThreadVersion[]): Map<string, string> {
	const said = new Map<string, string>();
	const carried = new Map<string, string | null>();
	// Oldest first per Branch, which is the order a Thread is read in.
	const inOrder = [...versions].sort(
		(a, b) => a.branch_id.localeCompare(b.branch_id) || a.sequence - b.sequence,
	);
	for (const version of inOrder) {
		const before = carried.get(version.branch_id);
		// The first occurrence on a Branch states a Language rather than
		// changing one: a recipe has always been in some Language, and saying
		// so of its first Version would mark every recipe ever written.
		if (before !== undefined && version.language !== null && version.language !== before) {
			said.set(`${version.branch_id}:${version.sequence}`, version.language);
		}
		carried.set(version.branch_id, version.language);
	}
	return said;
}
