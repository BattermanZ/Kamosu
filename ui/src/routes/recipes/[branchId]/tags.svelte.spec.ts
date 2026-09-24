/**
 * A recipe's tags, on the recipe (#104).
 *
 * Driven in the three states the ticket asks for, because they are three
 * different screens and only one of them is the pretty one: a library with no
 * tags at all, which is what every new instance and this project's own imported
 * corpus look like; a library with a handful; and one recipe carrying several,
 * one of them named in a Language the reader is not using.
 *
 * What these guard above everything: **no tagging act is ever presented as a
 * change to the recipe** (ADR 0035). No Version is minted, nothing is saved,
 * and `save_recipe_version` is never called — which is asserted rather than
 * assumed, because the stand-in records every ask a screen makes.
 */

import { describe, expect, it } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import type { GetRecipeOutput } from '$lib/api/catalogue';
import TagsTestHarness from './TagsTestHarness.svelte';

type Tag = GetRecipeOutput['tags'][number];

/** One of the Cookbook's tags, with everything the Catalogue requires present. */
const tag = (id: string, name: string, recipes = 0, language = 'en'): Tag => ({
	id,
	cookbook_id: 'c_1',
	name,
	language,
	names: [{ language, name }],
	recipes,
	// The Core decides this, against the Reading Language on the account (#104).
	// A screen never works it out, so a test says what the Core would have said.
	language_fallback: language !== 'en',
});

/** A tag this Cookbook knows only in French, read by somebody reading English. */
const MIJOTE: Tag = {
	id: 't_mijote',
	cookbook_id: 'c_1',
	name: 'mijoté',
	language: 'fr',
	names: [{ language: 'fr', name: 'mijoté' }],
	recipes: 7,
	language_fallback: true,
};

/** A library with a handful of words, as a Cookbook looks after a few months. */
const HANDFUL = [
	tag('t_weeknight', 'weeknight', 21),
	tag('t_batch', 'batch cook', 9),
	tag('t_spicy', 'spicy', 14),
];

/**
 * Every render answers `get_reading_preferences`, because the sheet asks for it:
 * a new tag is named in the Language this Person reads RECIPES in, which lives
 * on the account and is not the interface locale (ADR 0006, ADR 0016).
 */
function draw(tags: Tag[], answers: Answers) {
	const kamosu = standIn({
		get_reading_preferences: { reading_language: 'en', reading_measures: 'us' },
		...answers,
	});
	render(TagsTestHarness, { props: { client: kamosu.client, tags } });
	return kamosu;
}

describe('a recipe’s tags', () => {
	it('draws the row on a recipe with no tags, rather than hiding the way in', async () => {
		// Aurélien's choice, against the quieter option: a library that arrives
		// with 86 recipes and no tags grows none if the only way in is a button
		// at the foot of a long page.
		draw([], { list_tags: { tags: [] } });

		expect(screen.getByText('Tags')).toBeInTheDocument();
		expect(await screen.findByRole('button', { name: 'Add a tag' })).toBeInTheDocument();
	});

	it('explains what a tag is where a Cookbook has none, and says it changes no recipe', async () => {
		draw([], { list_tags: { tags: [] } });

		await fireEvent.click(screen.getByRole('button', { name: 'Add a tag' }));

		// The one place ADR 0035's promise is spelt out. Said here rather than on
		// every recipe forever, which would be a lecture.
		expect(await screen.findByText(/Tagging never changes the recipe/)).toBeInTheDocument();
		expect(screen.getByText(/A tag is your own word for a recipe/)).toBeInTheDocument();
	});

	it('makes a word nobody has used yet and puts it on, in one tap', async () => {
		const made = tag('t_new', 'weeknight', 1);
		const kamosu = draw([], {
			list_tags: { tags: [] },
			create_tag: made,
			set_recipe_tag: { tags: [made] },
		});

		await fireEvent.click(screen.getByRole('button', { name: 'Add a tag' }));
		const field = await screen.findByPlaceholderText('Search tags, or type a new one');
		await fireEvent.input(field, { target: { value: 'weeknight' } });

		await fireEvent.click(await screen.findByRole('button', { name: /Create the tag/ }));

		// Two Operations, in order: the Cookbook gains the word, and this recipe
		// gains the tag.
		const asked = kamosu.calls.map((call) => call.operation);
		expect(asked).toContain('create_tag');
		expect(asked).toContain('set_recipe_tag');
		expect(kamosu.calls.find((call) => call.operation === 'set_recipe_tag')?.input).toEqual({
			branch_id: 'b_1',
			tag_id: 't_new',
			carried: true,
		});

		// Named in the Language this Person reads recipes in, off their account.
		expect(kamosu.calls.find((call) => call.operation === 'create_tag')?.input).toEqual({
			language: 'en',
			name: 'weeknight',
		});

		// And nothing that would be a change to the recipe.
		expect(asked).not.toContain('save_recipe_version');
	});

	it('does not offer to make a word the Cookbook already holds in another Language', async () => {
		// A Cookbook holding *goûter* in French must not be given a second one
		// when somebody types it while reading English (ADR 0006).
		draw([], { list_tags: { tags: [MIJOTE] } });

		await fireEvent.click(screen.getByRole('button', { name: 'Add a tag' }));
		const field = await screen.findByPlaceholderText('Search tags, or type a new one');
		await fireEvent.input(field, { target: { value: 'mijoté' } });

		expect(screen.queryByRole('button', { name: /Create the tag/ })).not.toBeInTheDocument();
	});

	it('ticks what this recipe carries, and takes one back off', async () => {
		const carried = [HANDFUL[1]];
		const kamosu = draw(carried, {
			list_tags: { tags: HANDFUL },
			set_recipe_tag: { tags: [] },
		});

		await fireEvent.click(screen.getByRole('button', { name: 'Add a tag' }));

		// The state is on the switch, where a screen reader reads it — the filled
		// square is only what an eye reads.
		const held = await screen.findByRole('switch', { name: /batch cook/ });
		expect(held).toHaveAttribute('aria-checked', 'true');
		expect(screen.getByRole('switch', { name: /spicy/ })).toHaveAttribute('aria-checked', 'false');

		await fireEvent.click(held);

		expect(kamosu.calls.find((call) => call.operation === 'set_recipe_tag')?.input).toEqual({
			branch_id: 'b_1',
			tag_id: 't_batch',
			carried: false,
		});
	});

	it('reads a tag named in another Language in that Language, and says which', async () => {
		// A preference may never hide what a Cookbook holds (ADR 0006), so the
		// French word is shown as the French word and marked, rather than
		// vanishing or reading as a blank.
		draw([tag('t_batch', 'batch cook', 9), MIJOTE], { list_tags: { tags: HANDFUL } });

		expect(screen.getByText('mijoté')).toBeInTheDocument();
		expect(screen.getByText('batch cook')).toBeInTheDocument();
		// The code is what fits beside the word; the whole sentence is there for
		// anyone listening rather than looking, since "FR" read aloud is not a
		// fallback anyone would understand (`Tile.svelte`'s rule).
		expect(screen.getByText('fr')).toHaveAttribute('aria-hidden', 'true');
		expect(screen.getByText('In French')).toBeInTheDocument();
	});

	it('marks nothing on a tag the Core says is in the reader’s own Language', async () => {
		// The screen never compares Languages itself: it draws the mark where
		// `language_fallback` says so and nowhere else. A reader whose interface
		// is French and whose recipes are English would otherwise see every
		// English tag marked.
		draw([tag('t_batch', 'batch cook', 9)], { list_tags: { tags: HANDFUL } });

		expect(screen.getByText('batch cook')).toBeInTheDocument();
		expect(screen.queryByText('en')).not.toBeInTheDocument();
		expect(screen.queryByText(/^In /)).not.toBeInTheDocument();
	});

	it('sends a tag to the shelf filtered by it, rather than searching for its word', async () => {
		draw([tag('t_spicy', 'spicy', 14)], { list_tags: { tags: HANDFUL } });

		// A real filter and not a search: *spicy* means the recipes tagged so,
		// never the one whose title happens to say spicy (#104, ADR 0027).
		expect(screen.getByRole('link', { name: /spicy/ })).toHaveAttribute(
			'href',
			'/recipes?tag=t_spicy',
		);
	});

	it('says nothing about saving, unsaved changes or a new version', async () => {
		const kamosu = draw([HANDFUL[0]], {
			list_tags: { tags: HANDFUL },
			set_recipe_tag: { tags: HANDFUL.slice(0, 2) },
		});

		await fireEvent.click(screen.getByRole('button', { name: 'Add a tag' }));
		await fireEvent.click(await screen.findByRole('switch', { name: /spicy/ }));

		// Filing is not what a recipe is (ADR 0035). Nothing here may imply
		// otherwise, and the wording is what a person actually reads.
		expect(screen.queryByText(/unsaved/i)).not.toBeInTheDocument();
		expect(screen.queryByText(/new version/i)).not.toBeInTheDocument();
		expect(screen.queryByText(/\bsaved\b/i)).not.toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('save_recipe_version');
	});
});
