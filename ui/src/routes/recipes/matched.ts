/**
 * How a result says what matched (#62, ADR 0027).
 *
 * A result that cannot explain itself is noise: search *chocolate*, get
 * *Chilli con carne*, and without the 50 g of dark chocolate on the card the
 * search looks broken. The Operation answers where it matched and the line
 * itself; this turns the where into the short label set above the quote.
 *
 * A title match is deliberately unlabelled: the title is already on the tile's
 * band, so quoting it back under the tile would say the same thing twice.
 */

import { m } from '$lib/paraglide/messages';
import type { SearchRecipesOutput } from '$lib/api/catalogue';

export type Entry = SearchRecipesOutput['recipes'][number];
export type Matched = NonNullable<Entry['matched']>;

/** The label above a quoted line, or null where the line should not be quoted. */
export function matchedLabel(matched: Matched | null): string | null {
	if (!matched) return null;
	switch (matched.where) {
		case 'title':
			return null;
		case 'tag':
			return m.recipes_matched_tag();
		case 'ingredient':
			return m.recipes_matched_ingredient();
		case 'step':
			// The Operation counts a Step the way the recipe page does, Sections
			// taking no number, so this only has to print it. A Step that
			// somehow arrived without one is still a real match: it keeps the
			// quote and loses only the number.
			return matched.step_number === null
				? m.recipes_matched_step_unnumbered()
				: m.recipes_matched_step({ number: matched.step_number });
		case 'section':
			return m.recipes_matched_section();
		case 'note':
			return m.recipes_matched_note();
		case 'attempt':
			return m.recipes_matched_attempt();
	}
}

/**
 * The mark on a recipe being read in a Language its reader did not ask for,
 * written out in full — "In French". The card shows only the Language's code,
 * which is all that fits; this is what it means, for anyone listening to the
 * screen rather than looking at it.
 *
 * Null when there is nothing to mark — a preference never hides a recipe from
 * its owner, and the card says which Language it fell back to (ADR 0006).
 */
export function fallbackLanguage(entry: Entry): string | null {
	if (!entry.language_fallback) return null;
	const language =
		{
			en: m.recipes_language_en(),
			fr: m.recipes_language_fr(),
			es: m.recipes_language_es(),
			// A Language this build has no word for keeps its own code rather than
			// the recipe quietly losing its mark.
		}[entry.language] ?? entry.language;
	return m.recipes_language_fallback({ language });
}
