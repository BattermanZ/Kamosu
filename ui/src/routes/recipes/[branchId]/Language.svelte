<!--
	What a recipe says about its Language, on the recipe (#106, ADR 0006).

	**It says nothing unless there is something to say**, which is Aurélien's
	choice of 22 September 2026 against a Languages section drawn on every
	recipe and against moving the whole subject onto the Thread. The reasoning
	is his and worth keeping: the shelf already settled that Kamosu marks a
	Language only when it is not the one the reader asked for, and the dev
	library is 258 English recipes out of 295. A section reading
	*English · you are reading this* on nearly every recipe its owner has would
	say something they can already see. The cost accepted is that starting a
	translation sits two taps in, behind the Language control at the foot.

	So an ordinary English recipe with no Translation draws NOTHING HERE — not
	an empty row, not a heading, not a dash. That is the whole point, and a
	later change that gives this component an empty state undoes the choice.

	WHAT IT DRAWS, when it draws:

	  · this recipe is a Translation — which Language it renders, as a link to
	    the recipe it renders, because a Translation is an ordinary Branch and
	    walking to it is walking to a recipe (ADR 0006);
	  · that Translation has fallen behind — said as a SENTENCE and never as a
	    number. `versions_behind` is exact, and "two versions behind" means
	    nothing to a cook who has not read the source. "The English has been
	    changed twice since this was translated" is the same fact in words that
	    tell you what to do about it;
	  · the source is not on this instance — said as unanswerable rather than
	    as zero, which is the one thing the Core is careful about here and the
	    one thing a screen could quietly throw away;
	  · the Lineage has other Languages — one line each, each a link.

	UNKNOWN DRAWS NOTHING, EVER (ADR 0006). A recipe honestly written in two
	Languages is offered nothing, marked nothing and nagged about nothing from
	the moment it is said, and that is the whole of what Unknown buys. It is
	guarded here explicitly rather than left to fall out of the data, because
	falling out of the data is how it comes back.

	NOTHING HERE IS AN ACT. Saying what Language this is, and starting a
	translation, both mint a Version and both live at the foot with the other
	things that change the recipe. This is a line of the recipe's own facts.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { isUnknown, languageName } from '$lib/language';
	import type { GetRecipeOutput } from '$lib/api/catalogue';

	/** One other Branch of this Lineage, as the Thread lists it. */
	export interface OtherBranch {
		branch_id: string;
		language: string;
	}

	interface Props {
		/** The Language this recipe carries. */
		language: string;
		/** How this recipe stands as a Translation, or null — which most recipes are. */
		translation: GetRecipeOutput['translation'];
		/**
		 * Every other Branch of this Lineage the reader can reach, whatever its
		 * Language. Filtered here rather than by the caller so that the one rule
		 * — a Branch in the SAME Language is a divergence and not another
		 * Language — lives beside the sentence it governs.
		 */
		others: OtherBranch[];
	}

	let { language, translation, others }: Props = $props();

	/** One sentence, and where it goes if it goes anywhere. */
	type Line = { key: string; text: string; href?: string; caveat?: boolean };

	/**
	 * The Branch this one translates, where it is on this instance. Held
	 * separately because it is named by the Translation sentence and must not
	 * also get an *Also in* line of its own: one recipe, said twice, reads as
	 * two recipes.
	 */
	const source = $derived(
		translation?.source_branch_id
			? (others.find((each) => each.branch_id === translation.source_branch_id) ?? null)
			: null,
	);

	const lines = $derived.by<Line[]>(() => {
		// Unknown is silent. Not "usually silent" — silent (ADR 0006).
		if (isUnknown(language)) return [];

		const out: Line[] = [];

		if (translation) {
			// Three cases, and they are NOT two. `source_branch_id` being null
			// is the Core saying the source has never reached this instance;
			// the source being absent from `others` only means it is held by a
			// Kitchen this reader does not cook in. Telling the second as the
			// first is a screen inventing a fact, and it throws away the exact
			// `versions_behind` the Core did answer.
			if (source) {
				out.push({
					key: 'translates',
					text: m.recipe_language_translated_from({ language: languageName(source.language) }),
					href: `/recipes/${source.branch_id}`,
				});
			} else if (translation.source_branch_id === null) {
				// Not on this Kamosu at all. How far behind this has fallen is
				// unanswerable, and the Core answers null rather than zero
				// precisely so a screen cannot claim otherwise.
				out.push({ key: 'translates', text: m.recipe_language_source_absent(), caveat: true });
			} else {
				// Here, but not on this reader's shelf. Its Language is unknown
				// to this page — the Thread never listed it — so the sentence
				// cannot name one, and does not pretend to.
				out.push({ key: 'translates', text: m.recipe_language_source_unseen(), caveat: true });
			}

			// Exact, and said in words. Zero is up to date and says nothing:
			// silence is what *nothing to do* looks like. Said in both cases
			// where the Core answered a count, named where the source's
			// Language is known and unnamed where it is not.
			const behind = translation.versions_behind;
			if (behind !== null && behind > 0) {
				const named = source ? languageName(source.language) : null;
				out.push({
					key: 'behind',
					text: named
						? behind === 1
							? m.recipe_language_behind_once({ language: named })
							: m.recipe_language_behind({ language: named, count: behind })
						: behind === 1
							? m.recipe_language_behind_unseen_once()
							: m.recipe_language_behind_unseen({ count: behind }),
					caveat: true,
				});
			}
		}

		for (const other of others) {
			// A Branch in the same Language is a divergence — somebody else's
			// copy of these words — and has nothing to do with Language. The
			// Threshold above says what that is.
			if (other.language === language || isUnknown(other.language)) continue;
			if (source && other.branch_id === source.branch_id) continue;
			out.push({
				key: `also-${other.branch_id}`,
				text: m.recipe_language_also_in({ language: languageName(other.language) }),
				href: `/recipes/${other.branch_id}`,
			});
		}

		return out;
	});
</script>

<!--
	No heading and no wrapper when there is nothing: `{#each}` over an empty
	list draws nothing at all, which is what an ordinary recipe must get.

	The WHOLE SENTENCE is the link rather than one word inside it. On a phone
	that is a target a thumb can hit, and it keeps the phrase in one piece for
	the translator — a link spliced into the middle of a sentence is a phrase
	that cannot be reordered, and French and Spanish both reorder this one.
-->
{#each lines as line (line.key)}
	{#if line.href}
		<a
			href={line.href}
			class="block px-gutter pt-3 text-read text-accent underline underline-offset-2"
		>
			{line.text}
		</a>
	{:else}
		<p class="px-gutter pt-3 text-read {line.caveat ? 'text-support' : 'text-ink-2'}">
			{line.text}
		</p>
	{/if}
{/each}
