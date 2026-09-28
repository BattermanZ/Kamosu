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
 * Whether a save lands on the recipe, and why not where it does not: the
 * Core's answer (#132), never an id compared here.
 */
export type Whose = Pick<GetRecipeOutput, 'writes' | 'mine' | 'arrived' | 'cookbook'>;

/**
 * **Why a save starts the reader's own copy, and what stays** (#132): the one
 * sentence the writing screen, Promotion and a cooking's photo all say before
 * the tap, where `writes` is false. It says where the changes go, never that
 * the recipe is not the reader's to change, which read as a telling-off.
 *
 * - Sent to the reader: it sits in their own Cookbook, so the reason is that
 *   it was sent.
 * - In anybody else's Cookbook: the reason is whose Cookbook it is in, and
 *   what stays is its writers'. A named Cookbook is a message of its own
 *   rather than `cookbookCalled` set into one, because French and Spanish put
 *   an unnamed one's writers inside the phrase ("le Carnet de Marie").
 */
export function copySaid(whose: Whose, title: string): string {
	if (whose.mine && whose.arrived) return m.write_said_fork_sent({ title });
	const names = joinedNames(whose.cookbook.authors.map((author) => author.name));
	if (whose.cookbook.name !== null) {
		return m.write_said_fork_theirs_named({ cookbook: whose.cookbook.name, names, title });
	}
	return m.write_said_fork_theirs({ names, title });
}

/**
 * **Which version of a recipe is yours** (#136, Aurélien's choice A of 27
 * September 2026): the one every other version is marked against. It is your
 * own unnamed one where you have one (#131), and once you name it, the one you
 * have kept longest. Never one sent to you, which is somebody else's writing.
 *
 * `versions` comes oldest first, as the Thread answers its Branches. The
 * recipe page and the library a phone keeps offline both ask this, so the two
 * cannot compare against different versions.
 */
export function yoursAmong<T extends Pick<LabelledBranch, 'mine' | 'arrived' | 'name'>>(
	versions: T[],
): T | undefined {
	const own = versions.filter((each) => each.mine && !each.arrived);
	return own.find((each) => each.name === null) ?? own[0];
}

/**
 * **One version of a recipe, as the switch labels it** (#131, screen choice
 * 1): a name, and a line under it saying whose.
 *
 * - The reader's own, the one the others are marked against (`yours`): its
 *   name, or *Yours* while it has none, in *your Cookbook* (#136).
 * - Another of the reader's: its name, *yours too*.
 * - One that arrived from somebody else: the sender's name, *sent to you*.
 * - Anybody else's: its name where it has one, else whose it is.
 */
export function branchLabel(
	branch: LabelledBranch,
	yours: boolean,
): { name: string; whose: string } {
	if (branch.arrived) {
		return {
			name: branch.name ?? branch.hand_name ?? cookbookWhose(branch.cookbook),
			whose: branch.mine ? m.switch_sent_to_you() : cookbookCalled(branch.cookbook),
		};
	}
	if (branch.mine) {
		return {
			name: branch.name ?? m.switch_yours(),
			whose: yours ? m.switch_your_cookbook() : m.switch_yours_too(),
		};
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
