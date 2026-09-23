/**
 * **Whether saving onto a recipe writes a Version or starts a Copy, and in
 * which Kitchen** — read off the cook's own Kitchens, the one rule for every
 * screen that edits a recipe: the writing screen (#83) and a cooking's picture
 * promoted onto one (#110). Two copies of it would be two screens able to tell
 * the same cook two different things about the same tap.
 *
 * A recipe held by a Kitchen the cook does not cook in cannot take their
 * Version, so it forks into their Home Kitchen (ADR 0007). Anywhere else the
 * save goes onto the Kitchen that holds it.
 */

import type { ListKitchensOutput } from '$lib/api/catalogue';

type Kitchen = ListKitchensOutput['kitchens'][number];

export function whereASaveLands(
	kitchens: Kitchen[],
	holdingKitchenId: string,
): { forking: boolean; into: Kitchen | undefined } {
	const home = kitchens.find((kitchen) => kitchen.is_home) ?? kitchens[0];
	const holding = kitchens.find((kitchen) => kitchen.id === holdingKitchenId);
	return holding ? { forking: false, into: holding } : { forking: true, into: home };
}
