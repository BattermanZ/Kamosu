/**
 * A recipe's Related Recipes, on the recipe (#105, #52).
 *
 * Driven in the three states the ticket asks for, because they are three
 * different screens: a recipe carrying no links, which is what every recipe in
 * a fresh library looks like; one carrying several; and one whose link points
 * at a Lineage that has left the shelf.
 *
 * What these guard above everything:
 *
 * - **nothing here is a change to the recipe** (ADR 0035): no Version is
 *   minted and `save_recipe_version` is never called, which is asserted rather
 *   than assumed, because the stand-in records every ask a screen makes;
 * - **a link to a departed Lineage reads as a name, and can still be taken
 *   off** (#105), which is the one case `set_related_recipe` could not address
 *   before, since a deleted recipe has no Branch to name;
 * - **nothing says the two recipes are joined.** A card is a way to walk over
 *   there, and no Lineage is merged by any of this.
 */

import { describe, expect, it } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import type { GetRecipeOutput } from '$lib/api/catalogue';
import RelatedTestHarness from './RelatedTestHarness.svelte';

type Related = GetRecipeOutput['related_recipes'][number];

/** A link to a recipe still on this reader's shelf. */
const link = (id: string, title: string, main_photo: string | null = null): Related => ({
	lineage_id: `l_${id}`,
	branch_id: `b_${id}`,
	title,
	main_photo,
	language: 'en',
	language_fallback: false,
});

/**
 * A link whose Lineage has left the shelf. The Core answers these with the
 * remembered title and no Branch id, deliberately: text is better than a
 * broken pointer (#52).
 */
const DEPARTED: Related = {
	lineage_id: 'l_tonkatsu',
	branch_id: null,
	title: 'Tonkatsu Sauce',
	main_photo: null,
	// No Branch left, so no Language to be in and nothing to mark.
	language: null,
	language_fallback: false,
};

const NAAN = link('naan', 'Naan', 'p_naan');
const RICE = link('rice', 'Avocado Rice');

/** One entry as `search_recipes` answers it, which is what the sheet lists. */
const found = (id: string, title: string) => ({
	branch_id: `b_${id}`,
	lineage_id: `l_${id}`,
	title,
	language: 'en',
	language_fallback: false,
	main_photo: null,
	matched: null,
	yield: null,
});

function draw(related: Related[], answers: Answers = {}) {
	const kamosu = standIn(answers);
	render(RelatedTestHarness, { props: { client: kamosu.client, related } });
	return kamosu;
}

describe('a recipe’s related recipes', () => {
	it('draws the strip on a recipe with no links, rather than hiding the way in', () => {
		// Following the choice made for Tags (#104): a library that arrives with
		// 86 recipes and no links grows none if the only way in is hidden.
		draw([]);

		expect(screen.getByText('Related recipes')).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Relate a recipe' })).toBeInTheDocument();
	});

	it('draws each link as a card that opens that recipe', () => {
		draw([NAAN, RICE]);

		// A related recipe is a dish, so a card is a way to walk over there.
		expect(screen.getByRole('link', { name: 'Naan' })).toHaveAttribute('href', '/recipes/b_naan');
		expect(screen.getByRole('link', { name: 'Avocado Rice' })).toHaveAttribute(
			'href',
			'/recipes/b_rice',
		);
	});

	it('reads a link to a departed Lineage as a name, not a pointer or an error', () => {
		draw([NAAN, DEPARTED]);

		expect(screen.getByText('Tonkatsu Sauce')).toBeInTheDocument();
		expect(screen.getByText('No longer on your shelf')).toBeInTheDocument();
		// No link to follow, and nothing that reads as a failure.
		expect(screen.queryByRole('link', { name: /Tonkatsu Sauce/ })).not.toBeInTheDocument();
		expect(screen.queryByText(/error|failed|broken/i)).not.toBeInTheDocument();
	});

	it('marks a card shown in a Language the reader did not ask for, in the shelf’s words', () => {
		// A preference may never hide a recipe from its owner (ADR 0006), so the
		// recipe is shown and the difference is said. The code is what fits on
		// the card; the sentence is for anyone listening rather than looking.
		draw([{ ...NAAN, language: 'fr', language_fallback: true }]);

		expect(screen.getByText('fr')).toHaveAttribute('aria-hidden', 'true');
		// The shelf's own wording, not a raw code: "In French", never "In fr".
		expect(screen.getByText('In French')).toBeInTheDocument();
	});

	it('marks nothing on a card the Core says is in the reader’s own Language', () => {
		// The screen never compares Languages itself: it draws the mark where
		// `language_fallback` says so and nowhere else.
		draw([NAAN]);

		expect(screen.queryByText(/^In /)).not.toBeInTheDocument();
		expect(screen.queryByText('en')).not.toBeInTheDocument();
	});

	it('explains what a related recipe is where a recipe has none, and says it changes no recipe', async () => {
		// Stubbed with a FULL shelf on purpose. An empty query answers with every
		// recipe, so a library with anything in it never shows an empty list —
		// and gating the one sentence ADR 0035 requires on an empty list is how
		// the first draft of this contrived to say it nowhere.
		draw([], {
			search_recipes: {
				closest: false,
				query: null,
				recipes: [found('naan', 'Naan'), found('rice', 'Avocado Rice')],
			},
		});

		await fireEvent.click(screen.getByRole('button', { name: 'Relate a recipe' }));

		// The one place ADR 0035's promise is spelt out. Said here rather than on
		// every recipe forever, which would be a lecture.
		expect(await screen.findByText(/Neither recipe changes/)).toBeInTheDocument();
		expect(screen.getByText(/Relate recipes that go together/)).toBeInTheDocument();
	});

	it('makes a link by naming the far end’s Branch', async () => {
		const kamosu = draw([], {
			search_recipes: { closest: false, query: null, recipes: [found('naan', 'Naan')] },
			set_related_recipe: { related_recipes: [NAAN] },
		});

		await fireEvent.click(screen.getByRole('button', { name: 'Relate a recipe' }));
		const row = await screen.findByRole('switch', { name: /Naan/ });
		expect(row).toHaveAttribute('aria-checked', 'false');

		await fireEvent.click(row);

		expect(kamosu.calls.find((call) => call.operation === 'set_related_recipe')?.input).toEqual({
			branch_id: 'b_1',
			related_branch_id: 'b_naan',
			related: true,
		});
		// The whole shelf: the Cookbook keeps the link, and its far end may be
		// any recipe this Person can see, in whichever Kitchen (#131).
		expect(kamosu.calls.find((call) => call.operation === 'search_recipes')?.input).toEqual({
			query: null,
			kitchen_id: null,
			mine: false,
		});
		// And nothing that would be a change to the recipe.
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('save_recipe_version');
	});

	it('takes off a link to a departed Lineage by naming the Lineage', async () => {
		// The case that could not be addressed at all before #105: a deleted
		// recipe has no Branch, so there was no id to pass and the row stayed on
		// the shelf for good.
		const kamosu = draw([DEPARTED], {
			search_recipes: { closest: false, query: null, recipes: [] },
			set_related_recipe: { related_recipes: [] },
		});

		await fireEvent.click(screen.getByRole('button', { name: 'Relate a recipe' }));

		// It is listed even though no search could find it, because it is not on
		// the shelf to be found.
		const row = await screen.findByRole('switch', { name: /Tonkatsu Sauce/ });
		expect(row).toHaveAttribute('aria-checked', 'true');

		await fireEvent.click(row);

		expect(kamosu.calls.find((call) => call.operation === 'set_related_recipe')?.input).toEqual({
			branch_id: 'b_1',
			related_lineage_id: 'l_tonkatsu',
			related: false,
		});
	});

	it('does not offer the recipe you are standing on, which cannot be related to itself', async () => {
		draw([], {
			search_recipes: {
				closest: false,
				query: null,
				recipes: [{ ...found('naan', 'Naan') }, { ...found('self', 'This one'), branch_id: 'b_1' }],
			},
		});

		await fireEvent.click(screen.getByRole('button', { name: 'Relate a recipe' }));

		expect(await screen.findByRole('switch', { name: /Naan/ })).toBeInTheDocument();
		expect(screen.queryByRole('switch', { name: /This one/ })).not.toBeInTheDocument();
	});

	it('lists a link once, not twice, when the search also finds it', async () => {
		draw([NAAN], {
			search_recipes: { closest: false, query: null, recipes: [found('naan', 'Naan')] },
		});

		await fireEvent.click(screen.getByRole('button', { name: 'Relate a recipe' }));

		expect(await screen.findAllByRole('switch', { name: /Naan/ })).toHaveLength(1);
	});

	it('says nothing about saving, unsaved changes, a new version, or joining', async () => {
		const kamosu = draw([NAAN], {
			search_recipes: { closest: false, query: null, recipes: [found('rice', 'Avocado Rice')] },
			set_related_recipe: { related_recipes: [NAAN, RICE] },
		});

		await fireEvent.click(screen.getByRole('button', { name: 'Relate a recipe' }));
		await fireEvent.click(await screen.findByRole('switch', { name: /Avocado Rice/ }));

		// Relating is not what a recipe is (ADR 0035), and it never joins two
		// Lineages (#52). Nothing here may imply either.
		expect(screen.queryByText(/unsaved/i)).not.toBeInTheDocument();
		expect(screen.queryByText(/new version/i)).not.toBeInTheDocument();
		expect(screen.queryByText(/\bsaved\b/i)).not.toBeInTheDocument();
		expect(screen.queryByText(/merge|combine|join/i)).not.toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('save_recipe_version');
	});
});
