/**
 * Which Branches of a Lineage are versions of the recipe on screen (#131):
 * the ones that share a history with it, which is every Copy and variation of
 * it and never a Translation.
 *
 * **A Translation is not a Divergence, and cannot be paired with one.** A
 * Divergence is two Branches that parted from a shared Version; a
 * Translation's chain STARTS FRESH, which is exactly what separates it from a
 * Copy (ADR 0006, `start_translation`). So a Translation and the recipe it
 * renders share no Version at all, and asking for a Divergence between them is
 * answered — correctly — with "their chains never converge".
 *
 * **Whether two Branches share a chain is answerable here**, and is not worth
 * a request that would be refused. The Thread carries every Branch's every
 * occurrence, so two Branches part from a shared Version exactly when they
 * have a Version id in common — which two Translations of one recipe never do,
 * and a Branch and its Copy always do.
 */
import type { GetThreadOutput } from '$lib/api/catalogue';

export function sharingAChain(
	thread: Pick<GetThreadOutput, 'branches' | 'versions'>,
	branchId: string,
): GetThreadOutput['branches'] {
	const versionsOf = new Map<string, Set<string>>();
	for (const occurrence of thread.versions) {
		const seen = versionsOf.get(occurrence.branch_id) ?? new Set<string>();
		seen.add(occurrence.version_id);
		versionsOf.set(occurrence.branch_id, seen);
	}
	const onPage = versionsOf.get(branchId) ?? new Set<string>();
	return thread.branches.filter(
		(each) =>
			each.branch_id === branchId ||
			[...(versionsOf.get(each.branch_id) ?? [])].some((id) => onPage.has(id)),
	);
}
