/**
 * How a Tag's word is drawn, in one place (#104).
 *
 * Four screens show a Tag — the row on a recipe, the sheet that puts one on,
 * the shelf's filter and the Settings list — and every one of them needs the
 * same two facts: which word to print, and whether to mark it as being in a
 * Language the reader did not ask for. Written once here because the four had
 * drifted into three functions and one inlined copy of the same idea, and
 * because the version they had drifted into was wrong.
 *
 * **Neither fact is decided here.** The Core chooses the word, against the
 * Reading Language held on the account, and says whether it fell back
 * (`language_fallback`, the same word a shelf entry uses). A screen that
 * compared `tag.language` against the *interface* locale — which is what these
 * four did before — marks every Tag wrongly for anybody reading Kamosu in one
 * Language and their recipes in another, and those are two separate settings.
 */

import { m } from '$lib/paraglide/messages';
import { languageName } from '$lib/language';
import type { GetReadingPreferencesOutput, ListTagsOutput } from '$lib/api/catalogue';

/** The three Languages Kamosu holds a name in, as the Catalogue declares them. */
export type ReadingLanguage = GetReadingPreferencesOutput['reading_language'];

const READING_LANGUAGES: readonly ReadingLanguage[] = ['en', 'fr', 'es'];

/**
 * One of the three, from the looser `string | null` a Tag's `language` is
 * served as. A **check** rather than a cast: `tag_schema` shares its
 * reader-facing shape with a Food's, so tightening the declaration there would
 * make the two diverge for a reason that has nothing to do with Foods — and the
 * compiler is the reviewer this frontend does not have (ADR 0012), so the place
 * to be honest about a widening is here, once, with a stated fallback.
 *
 * Nothing in Kamosu writes a fourth Language: every path that names a Tag takes
 * the same three-way enum. The fallback exists for the shape, not for a case.
 */
export function oneOfThree(code: string | null, fallback: ReadingLanguage): ReadingLanguage {
	return READING_LANGUAGES.find((language) => language === code) ?? fallback;
}

/** A Tag as every Operation that answers with one serves it. */
export type Tag = ListTagsOutput['tags'][number];

export interface TagWord {
	/** The word to print. */
	name: string;
	/**
	 * The Language code to mark, or `null` where the word is in the one the
	 * reader asked for and there is nothing to say.
	 */
	elsewhere: string | null;
	/**
	 * The whole sentence, for anyone listening to the screen rather than
	 * looking at it — `Tile.svelte`'s rule, because "FR" read aloud is not a
	 * fallback anyone would understand. `null` when there is no mark.
	 */
	said: string | null;
}

/**
 * A Tag holding no name at all is possible in the Catalogue's shape and by
 * nothing any Operation does: `create_tag` demands a word and nothing removes a
 * single name. It is drawn rather than crashed on, because "true by omission"
 * is not the same as "true".
 */
export function tagWord(tag: Tag): TagWord {
	if (tag.name === null) return { name: m.tags_unnamed(), elsewhere: null, said: null };
	if (!tag.language_fallback || tag.language === null) {
		return { name: tag.name, elsewhere: null, said: null };
	}
	return {
		name: tag.name,
		elsewhere: tag.language,
		// The word, never the code: "In en" read aloud is not a sentence (#112).
		said: m.settings_tags_in_language({ language: languageName(tag.language) }),
	};
}

/**
 * Every Tag in the order a reader can predict: by the word they see.
 * `localeCompare` because *é* belongs beside *e* to a person and nowhere near
 * it by code point.
 */
export function byWord(tags: Tag[]): Tag[] {
	return tags.toSorted((a, b) => tagWord(a).name.localeCompare(tagWord(b).name));
}
