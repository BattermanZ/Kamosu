/**
 * The line under the recipe screen's History button (#133): the Versions on
 * this Branch's own chain, counted and dated, so a cook can tell before
 * tapping whether anything is behind it.
 *
 * A Variation's chain includes the Versions it started from, since those are
 * its past too, and never another Branch's later ones. Filtered by the id on
 * screen rather than reset on leaving, so the last recipe's count can never
 * label this one's button.
 */
import { m } from '$lib/paraglide/messages';
import { aDate } from '$lib/dates';
import type { GetThreadOutput } from '$lib/api/catalogue';

export function threadLine(versions: GetThreadOutput['versions'], branchId: string): string | null {
	const own = versions.filter((each) => each.branch_id === branchId);
	if (own.length === 0) return null;
	const last = own.reduce(
		(latest, each) => (each.created_at > latest ? each.created_at : latest),
		own[0]!.created_at,
	);
	const when = aDate(last);
	if (own.length === 1) return m.recipe_thread_saved_once({ when });
	if (own.length === 2) return m.recipe_thread_saved_twice({ when });
	return m.recipe_thread_saved_times({ count: own.length, when });
}
