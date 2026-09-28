/**
 * A Language, said in words (ADR 0006, #106).
 *
 * A Language reaches a screen as a code — `en`, `fr`, `es`, or `unknown` — and
 * no screen may print one. "In fr" is not a sentence, and a code read aloud by
 * a screen reader is worse than one looked at.
 *
 * This is the single mapping from code to word. `matched.ts` held the first
 * copy, for the shelf's fallback mark, and its own comment says why there must
 * never be a second: that is how "In French" becomes "In fr" on one screen and
 * nobody notices. It now reads this one.
 *
 * **THE TWO LISTS BELOW ARE THE CATALOGUE'S, NOT THIS FILE'S.** They are
 * derived from the generated client rather than typed out again, because the
 * client is generated from the Catalogue precisely so the frontend and the
 * Core cannot disagree about an Operation's shape (AGENTS.md, "The
 * interface"). A self-hoster adding a fourth Language to `src/catalogue.rs`
 * regenerates the client, and a hand-written copy here would go quietly stale
 * with nothing going red. `LANGUAGE_WORDS` below is the one place that then
 * fails to compile, which is the right place to be told.
 *
 * The two differ by exactly one member, and that difference is the Core's rule
 * rather than this file's opinion: a recipe may carry Unknown, but a
 * Translation may never be written into it — a recipe honestly written in two
 * Languages can neither be a Translation nor have one.
 */

import { m } from '$lib/paraglide/messages';
import type { SetRecipeLanguageInput, StartTranslationInput } from '$lib/api/catalogue';

/** The Languages a recipe can be written in — what a Translation may render into. */
export type WrittenLanguage = StartTranslationInput['language'];
/** Those, and the answer that says a recipe is honestly more than one. */
export type BranchLanguage = SetRecipeLanguageInput['language'];

/**
 * The word for each Language, keyed by the Catalogue's own enum — so a
 * Language added to `src/catalogue.rs` fails to compile here until somebody
 * says what it is called, rather than rendering as its own code for ever.
 */
const LANGUAGE_WORDS: Record<BranchLanguage, () => string> = {
	en: m.recipes_language_en,
	fr: m.recipes_language_fr,
	es: m.recipes_language_es,
	unknown: m.recipes_language_unknown,
};

/** Every Language a recipe may carry, in the order the Catalogue declares them. */
export const BRANCH_LANGUAGES = Object.keys(LANGUAGE_WORDS) as BranchLanguage[];
/** Every Language a Translation may be written into — the same, less Unknown. */
export const WRITTEN_LANGUAGES = BRANCH_LANGUAGES.filter(
	(language): language is WrittenLanguage => language !== 'unknown',
);

/** Whether a Language code is the permanent, unremarkable *more than one* (ADR 0006). */
export function isUnknown(language: string): boolean {
	return language === 'unknown';
}

/**
 * A Language code this build can actually act on, or null.
 *
 * `language_offer` is declared as a plain string rather than an enum, so a
 * save's answer is not typed down to the four this build knows — and a Bundle
 * or a newer server can carry a fifth. This is the one place that narrows it,
 * and it narrows against the Catalogue's own list rather than a written-out
 * copy. A Language this build has no word for is not offered, which is better
 * than offering a choice it could not carry out.
 */
export function asBranchLanguage(language: string): BranchLanguage | null {
	return (BRANCH_LANGUAGES as string[]).includes(language) ? (language as BranchLanguage) : null;
}

/**
 * One Language, in this reader's interface words — "French".
 *
 * Takes a plain string rather than a `BranchLanguage`, because what arrives on
 * a recipe is whatever was written into it: a Bundle from a newer Kamosu, or
 * from a self-hoster who dropped in a fourth language file, can carry a
 * Language this build has no word for. That one keeps its own code rather than
 * the recipe quietly losing its mark.
 */
export function languageName(language: string): string {
	return LANGUAGE_WORDS[language as BranchLanguage]?.() ?? language;
}
