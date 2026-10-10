/**
 * **What a cooking changed**, read off the Core's own Pairing of an As Cooked
 * against the Version cooked (ADR 0019). The recipe page's offer (#58) and the
 * diary's (#210) both show these lines before a cooking is kept, so they are
 * worked out once.
 *
 * Nothing here matches a line to a line: index arithmetic in a screen would be
 * wrong exactly where a cook dropped a line or added one, which they may.
 */

import type { ListAttemptsOutput } from '$lib/api/catalogue';

type Against = NonNullable<ListAttemptsOutput['attempts'][number]['as_cooked']>['against'];

/** One line a cooking did not leave alone: what it says now, and what the recipe said. */
export type ChangedLine = {
	key: string;
	state: Against['ingredients'][number]['state'];
	now: string | null;
	was: string | null;
};

/**
 * The lines a cooking did not leave alone, in the recipe's own order:
 * rewritten, dropped, or written from nothing. A row the cook left alone is
 * not among them, since what is being confirmed is what would change.
 */
export function changedLines(against: Against | undefined): ChangedLine[] {
	if (!against) return [];
	const of = (rows: Against['ingredients'], list: string): ChangedLine[] =>
		rows
			.filter((row) => row.state !== 'same')
			.map((row, at) => ({
				key: `${list}-${at}`,
				state: row.state,
				now: row.theirs?.text ?? null,
				was: row.mine?.text ?? null,
			}));
	return [...of(against.ingredients, 'i'), ...of(against.steps, 's')];
}
