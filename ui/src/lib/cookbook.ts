/**
 * **Saying whose a recipe is** (ADR 0041, #131). A recipe belongs to a
 * Cookbook, and the Core answers a Cookbook as its id, the name its
 * Co-authors gave it (null until they give one) and who writes it — never as
 * a sentence, because the sentence is in the reader's Language and the Core
 * does not word the interface.
 *
 * One module, so the switch, the writing screens and Settings cannot name the
 * same Cookbook three different ways.
 */

import { m } from '$lib/paraglide/messages';
import { getLocale } from '$lib/paraglide/runtime';
import type { GetRecipeOutput, GetThreadOutput } from '$lib/api/catalogue';

export type CookbookLabel = GetRecipeOutput['cookbook'];
/** What labelling a Branch needs, which a Thread's Branch and each side of a Divergence both carry. */
export type LabelledBranch = Pick<
	GetThreadOutput['branches'][number],
	'cookbook' | 'name' | 'mine' | 'arrived' | 'hand_name'
>;

/** Names joined as the reader's Language joins them: "Aurélien and Camille". */
export function joinedNames(names: string[]): string {
	return new Intl.ListFormat(getLocale(), { type: 'conjunction' }).format(names);
}

/**
 * A Cookbook named plainly, for a sentence that is about it: its own name, or
 * its Co-authors' — "Hélène", "Aurélien and Camille", "Chez nous".
 */
export function cookbookPlainName(cookbook: CookbookLabel): string {
	return cookbook.name ?? joinedNames(cookbook.authors.map((author) => author.name));
}

/**
 * A Cookbook named as a label on a recipe: its own name, or whose it is —
 * "Hélène’s", "Aurélien and Camille’s".
 */
export function cookbookWhose(cookbook: CookbookLabel): string {
	return cookbook.name ?? m.cookbook_whose({ names: cookbookPlainName(cookbook) });
}

/** A Cookbook named as itself: "Hélène’s Cookbook", or the name it was given. */
export function cookbookCalled(cookbook: CookbookLabel): string {
	return cookbook.name ?? m.cookbook_called({ names: cookbookPlainName(cookbook) });
}

/**
 * **One version of a recipe, as the switch labels it** (#131, screen choice
 * 1): a name, and a line under it saying whose.
 *
 * - The reader's own, unnamed: *Yours*, in *your Cookbook*.
 * - A variation of the reader's: its name, *yours too*.
 * - One that arrived from somebody else: the sender's name, *sent to you*.
 * - Anybody else's: its name where it has one, else whose it is.
 */
export function branchLabel(branch: LabelledBranch): { name: string; whose: string } {
	if (branch.arrived) {
		return {
			name: branch.name ?? branch.hand_name ?? cookbookWhose(branch.cookbook),
			whose: branch.mine ? m.switch_sent_to_you() : cookbookCalled(branch.cookbook),
		};
	}
	if (branch.mine) {
		return branch.name
			? { name: branch.name, whose: m.switch_yours_too() }
			: { name: m.switch_yours(), whose: m.switch_your_cookbook() };
	}
	return {
		name: branch.name ?? cookbookWhose(branch.cookbook),
		whose: cookbookCalled(branch.cookbook),
	};
}

/**
 * The name the marks on a recipe use for the version being compared with
 * yours: plain, since the marks set it inside their own sentences ("not
 * Hélène’s", "Took the onions from Hélène").
 */
export function branchPlainName(branch: LabelledBranch): string {
	if (branch.name) return branch.name;
	if (branch.arrived && branch.hand_name) return branch.hand_name;
	return cookbookPlainName(branch.cookbook);
}

/** A Kitchen as the reader knows it: their own Nickname, where they set one. */
export function kitchenName(kitchen: { name: string; nickname: string | null }): string {
	return kitchen.nickname ?? kitchen.name;
}
