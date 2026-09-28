/**
 * What the recipe screen's Language line and sheet need to know about the
 * other Branches of this Lineage (#106, ADR 0006).
 *
 * A Translation is an ordinary Branch, so there is nothing else to ask — no
 * Operation lists *this recipe's translations*, because there is no such
 * object to list. Everything here is read off the Thread's Branches, which the
 * recipe screen already holds.
 *
 * Computed here rather than inside `Language.svelte` so that the component
 * takes plain facts and can be rendered in a test without a Thread.
 */
import type { GetRecipeOutput, GetThreadOutput } from '$lib/api/catalogue';

type Branch = GetThreadOutput['branches'][number];

export interface LanguageFamily {
	/** Every OTHER Branch of this Lineage, which is the whole of what the Language line reads. */
	others: { branch_id: string; language: Branch['language'] }[];
	/**
	 * Whether Unknown is barred here, **in the Core's own terms**: this recipe
	 * translates something, or something translates it —
	 * `set_recipe_language` refuses exactly those two. Not "a sibling is in
	 * another Language", which would wrongly bar a recipe whose only sibling
	 * is a Divergence, and is not the rule being enforced.
	 *
	 * Where the two could still disagree — a Translation held by a Kitchen
	 * this reader does not cook in is absent from the Thread — this errs
	 * toward OFFERING, and the Core's refusal is then shown in its own words.
	 * Wrongly offering costs a sentence; wrongly barring hides a choice behind
	 * a reason that is not true.
	 */
	inAFamily: boolean;
	/**
	 * The Languages not worth offering to translate into: this recipe's own,
	 * and those of the Translations this family already holds. A Divergence is
	 * deliberately NOT counted — somebody else's copy of these words happening
	 * to be in French is no reason to refuse to write a French translation.
	 */
	taken: string[];
}

export function languageFamily(
	branchId: string,
	recipe: Pick<GetRecipeOutput, 'language' | 'translation'> | undefined,
	branches: Branch[],
): LanguageFamily {
	const others = branches
		.filter((each) => each.branch_id !== branchId)
		.map((each) => ({ branch_id: each.branch_id, language: each.language }));
	/*
	 * The Branches that translate THIS one. A Translation names the Version it
	 * renders and `get_thread` answers that pointer per Branch, so this is a
	 * fact rather than an inference from Languages: two Branches in different
	 * Languages are not necessarily a Translation and its source — one may be
	 * a Divergence somebody relabelled.
	 */
	const translationsOfThis = branches.filter(
		(each) => each.branch_id !== branchId && each.translation?.source_branch_id === branchId,
	);
	return {
		others,
		inAFamily: Boolean(recipe?.translation) || translationsOfThis.length > 0,
		taken: [
			...(recipe ? [recipe.language] : []),
			...translationsOfThis.map((each) => each.language),
			...others
				.filter((each) => each.branch_id === recipe?.translation?.source_branch_id)
				.map((each) => each.language),
		],
	};
}
