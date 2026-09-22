/**
 * What the two Food screens share (#107).
 *
 * A Food is an edible thing Kamosu knows about, learnt by itself from the
 * Readings that mention it (CONTEXT.md). It is instance-wide rather than a
 * Kitchen's, which is why every Operation that reads or corrects one is
 * `Permission::Person` and none of them takes a Kitchen.
 */

import { m } from '$lib/paraglide/messages';
import type { GetFoodOutput, ListFoodsOutput, SetFoodNameInput } from '$lib/api/catalogue';

/** A Food as every Operation that answers with one serves it. */
export type FoodRow = ListFoodsOutput['foods'][number];

/** One Food, read whole. The same shape, named for where it is used. */
export type Food = GetFoodOutput;

/**
 * **This list is the Catalogue's, not this file's.** It is derived from the
 * generated client rather than typed out again, for the reason `language.ts`
 * gives in capitals: the client is generated from the Catalogue precisely so
 * the frontend and the Core cannot disagree about an Operation's shape
 * (AGENTS.md, "The interface"). A self-hoster adding a fourth Language to
 * `src/catalogue.rs` regenerates the client and finds this list grew with it,
 * where a hand-written copy would have gone quietly stale with nothing going
 * red.
 */
export type NameLanguage = SetFoodNameInput['language'];
export const NAME_LANGUAGES: NameLanguage[] = ['en', 'fr', 'es'] satisfies NameLanguage[];

/**
 * A word reduced for SEARCHING, which is the only thing it is for.
 *
 * **This is deliberately not a Food Match.** A Food Match holds that "accents
 * and plurals are meaning, so *maïs* is not *mais*" (CONTEXT.md), because it
 * decides whether two words name the same Food. A search box wants the
 * opposite: somebody typing `pecan` on a phone keyboard means `noix de pécan`,
 * and refusing to find it would be pedantry.
 *
 * Nothing downstream of this decides identity. A row found by this fold is
 * followed by its own `id`, which came from `list_foods` — so a generous match
 * can only ever show a Food that might be wanted, never act on the wrong one.
 */
export function foldWord(word: string): string {
	return word
		.normalize('NFD')
		.replace(/\p{Mn}/gu, '')
		.toLowerCase()
		.replace(/[.]/g, ' ')
		.replace(/\s+/g, ' ')
		.trim();
}

/**
 * The name a Food carries in one Language, or nothing.
 *
 * A Food's `names` is the whole truth about what it is called; `name` and
 * `language` are only the Core's pick for this reader. Editing works from
 * `names`, so that changing the French name of a Food shown in English does
 * not first have to work out which one is on screen.
 */
export function nameIn(food: Food, language: NameLanguage): string | null {
	return food.names.find((named) => named.language === language)?.name ?? null;
}

/**
 * **How many Readings point at a Food**, which is the only honest measure of
 * whether correcting it is worth anything. Both screens say it, so both say it
 * the same way — and one line rather than "1 lines", which Kamosu branches at
 * the call site everywhere else (`Diary.svelte`).
 */
export function linesPointingAt(count: number): string {
	return count === 1 ? m.foods_lines_one() : m.foods_lines({ count });
}

/** A Cup Weight as the box holds it: a number, or the empty box meaning none. */
export function cupWeightBox(grams: number | null): string {
	return grams === null ? '' : String(grams);
}
