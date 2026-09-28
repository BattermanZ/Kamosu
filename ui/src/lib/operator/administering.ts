/**
 * Does this Person administer the instance? (#103)
 *
 * Kamosu has no Operation answering *who is signed in* — `rename_person`
 * writes a name and nothing reads one back — so the question is put the only
 * way it can be: by asking something only an Operator may ask and seeing
 * whether the Core allows it. `list_accounts` is the cheapest of those, and it
 * is the Operator screen's first call anyway, so asking costs that screen
 * nothing.
 *
 * Written once because two screens ask it — Settings, to decide whether to
 * show the way in, and the Operator screen itself — and written twice it would
 * be two subtly different askings. It already was: both call sites treated
 * *any* refusal as "not an Operator", so an unreadable database showed a real
 * Operator an empty room and hid their own door. Three answers, not two, is
 * the fix, and it belongs in one place.
 */

import { OperationError } from '$lib/api/client';
import type { KamosuClient, ListAccountsOutput } from '$lib/api/catalogue';

export type Administering =
	/** They do, and here is what they administer. */
	| { may: true; accounts: ListAccountsOutput['accounts'] }
	/** They do not. The instance refused, which is the answer. */
	| { may: false }
	/** Kamosu could not say. Not the same as "no", and never shown as one. */
	| { may: 'unknown'; failed: string };

export async function asksWhetherAdministering(kamosu: KamosuClient): Promise<Administering> {
	try {
		return { may: true, accounts: (await kamosu.listAccounts({})).accounts };
	} catch (error) {
		if (!(error instanceof OperationError)) throw error;
		if (error.kind === 'unauthorized') return { may: false };
		return { may: 'unknown', failed: error.message };
	}
}
