/**
 * **Whether saving onto a recipe writes a Version or starts a Copy, and in
 * which Kitchen** — read off the cook's own Kitchens, the one rule for every
 * screen that edits a recipe: the writing screen (#83), a cooking's picture
 * promoted onto one (#110), and what was cooked kept onto one (#58). Copies
 * of it would be screens able to tell the same cook different things about
 * the same tap.
 *
 * A recipe held by a Kitchen the cook does not cook in cannot take their
 * Version, so saving starts a **Copy** (ADR 0007). Anywhere else the save
 * goes onto the Kitchen that holds it.
 *
 * **Which Kitchen keeps a Copy is asked, never assumed, of a cook in several
 * Kitchens** (#111, spec item 19). Nothing is picked for them — not even the
 * Home Kitchen — because a recipe cannot be moved between Kitchens
 * afterwards, and a default somebody scrolled past is a dish handed to the
 * wrong household. A cook in one Kitchen is never asked: there is nothing to
 * choose between.
 */

import type { ListKitchensOutput } from '$lib/api/catalogue';

export type Kitchen = ListKitchensOutput['kitchens'][number];

/**
 * `chosen` is the cook's answer where they are asked, and nothing until they
 * give one; `into` is then the Kitchen they chose. Where they are not asked it
 * is ignored.
 */
export function whereASaveLands(
	kitchens: Kitchen[],
	holdingKitchenId: string,
	chosen?: string,
): { forking: boolean; asking: boolean; into: Kitchen | undefined } {
	const holding = kitchens.find((kitchen) => kitchen.id === holdingKitchenId);
	if (holding) return { forking: false, asking: false, into: holding };
	return kitchens.length > 1
		? {
				forking: true,
				asking: true,
				into: kitchens.find((kitchen) => kitchen.id === chosen),
			}
		: { forking: true, asking: false, into: kitchens[0] };
}

/**
 * What a save sends about its Kitchen: the one the cook chose, where they
 * were asked, and nothing otherwise — the Core already knows the rest.
 */
export function chosenKitchenInput(lands: { asking: boolean; into: Kitchen | undefined }): {
	kitchen_id?: string;
} {
	return lands.asking && lands.into ? { kitchen_id: lands.into.id } : {};
}

/** The name a cook knows a Kitchen by: their own Nickname, where they set one. */
export function kitchenName(kitchen: Kitchen): string {
	return kitchen.nickname ?? kitchen.name;
}
