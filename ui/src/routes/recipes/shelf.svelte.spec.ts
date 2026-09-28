/**
 * The screen-seam test: Recipes, against a stand-in Kamosu (#62).
 *
 * The stand-in checks every answer below against the shape the Catalogue
 * declares before the screen sees it, so a test here can lie about the values
 * and never about the shape — rename a field in `src/catalogue.rs` and this
 * fails in the same commit.
 *
 * What it is really for is the two things a behaviour test cannot reach,
 * because neither is an Operation's answer: that **nothing found explains
 * itself** rather than showing an empty screen, and that a filter is held on
 * this screen and nowhere else.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen, fireEvent } from '@testing-library/svelte';
import Recipes from './+page.svelte';
import { standIn, type StandIn } from '$lib/api/stand-in';
import { renderScreen } from '../../testing/render';
import ShelfTestHarness from './ShelfTestHarness.svelte';
import RouteTestHarness from './RouteTestHarness.svelte';
import type { MeaningSearchStatusOutput, ReadPastedRecipeOutput } from '$lib/api/catalogue';
import {
	cookbookLabel,
	kitchenAnswer,
	searchesSent as searches,
	serverAnsweredSearchesOtherwise,
	writtenByMe,
} from '../../testing/recipes';
import { went } from '../../testing/navigation';
import { takePaste } from '$lib/pasted.svelte';

const kitchen = kitchenAnswer('k_home', {
	name: 'Maison',
	members: [{ person_id: 'p_1', name: 'Aurélien' }],
});

/**
 * Marc's Kitchen, which sees his Cookbook as well as Aurélien's: the shelf's
 * Kitchen filter narrows to whatever Cookbooks the chosen Kitchen sees.
 */
const marcsKitchen = kitchenAnswer('k_marc', {
	name: 'Chez Marc',
	members: [
		{ person_id: 'p_1', name: 'Aurélien' },
		{ person_id: 'p_marc', name: 'Marc' },
	],
	cookbooks: [cookbookLabel(), cookbookLabel('c_marc', ['Marc'])],
});

/**
 * Meaning Search as most instances have it: never asked about, so the offer is
 * live for an Operator and searching matches on words alone (ADR 0029). Every
 * render below needs an answer to this, because the screen asks on open — the
 * tests that are *about* Meaning Search override what they care about.
 */
const meaningOff = (over: Partial<MeaningSearchStatusOutput> = {}): MeaningSearchStatusOutput => ({
	state: 'unasked',
	on: false,
	offer: false,
	may_change: false,
	model: 'EmbeddingGemma-300M (4-bit)',
	terms_url: 'https://ai.google.dev/gemma/terms',
	prohibited_use_policy_url: 'https://ai.google.dev/gemma/prohibited_use_policy',
	terms_version: '2026-04-01',
	accepted_by: null,
	accepted_via_access_key: null,
	accepted_at: null,
	declined_at: null,
	model_present: false,
	indexed_at: null,
	recipes_not_yet_indexed: 0,
	...over,
});

/** One entry on the shelf, with everything the Catalogue requires present. */
const entry = (over: Record<string, unknown> = {}) => ({
	lineage_id: 'l_1',
	branch_id: 'b_1',
	title: 'Miso Soup',
	language: 'en',
	language_fallback: false,
	main_photo: null,
	yield: null,
	matched: null,
	...writtenByMe(),
	...over,
});

/** One of a Cookbook's tags, with everything the Catalogue requires present. */
const shelfTag = (id: string, name: string, recipes: number, cookbook_id = 'c_1') => ({
	id,
	cookbook_id,
	name,
	language: 'en',
	names: [{ language: 'en', name }],
	recipes,
	language_fallback: false,
});

describe('the recipes screen', () => {
	it('shows the shelf it was given, one card per Lineage', async () => {
		renderScreen(Recipes, {
			list_tags: { tags: [] },
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: {
				query: null,
				closest: false,
				recipes: [
					entry({ lineage_id: 'l_1', title: 'Airfried Cauliflower' }),
					entry({ lineage_id: 'l_2', branch_id: 'b_2', title: 'Miso Soup' }),
				],
			},
		});

		expect(await screen.findByText('Airfried Cauliflower')).toBeInTheDocument();
		expect(screen.getByText('Miso Soup')).toBeInTheDocument();
		expect(screen.getByText('2 recipes')).toBeInTheDocument();
	});

	it('keeps the + beside the search box when nothing was found, and says each offer once', async () => {
		renderScreen(Recipes, {
			list_tags: { tags: [] },
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: { query: 'osso buco', closest: false, recipes: [] },
		});

		await screen.findByText(/Nothing matched .osso buco./);
		// The + is small and always in the same place, so it stays; its list is
		// closed, so the three big offers below are the only ones showing (#174).
		expect(screen.getByRole('button', { name: 'Add a recipe' })).toHaveAttribute(
			'aria-expanded',
			'false',
		);
		expect(screen.getAllByRole('button', { name: /Import from a link/ })).toHaveLength(1);
		expect(screen.getAllByRole('button', { name: 'Bring in a Kamosu zip file' })).toHaveLength(1);
	});

	it('explains why nothing matched, and offers the two things you were about to do', async () => {
		renderScreen(Recipes, {
			list_tags: { tags: [] },
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: { query: 'osso buco', closest: false, recipes: [] },
		});

		// It names what was searched for — from the answer, not from the field.
		expect(await screen.findByText(/Nothing matched .osso buco./)).toBeInTheDocument();

		// And says what it looked through, and why it matched only words. An
		// empty screen would leave a person unable to tell a missing recipe
		// from a broken search.
		expect(
			screen.getByText(/searched every title, ingredient, step and cooking note/),
		).toBeInTheDocument();
		expect(screen.getByText(/Meaning search is off/)).toBeInTheDocument();

		expect(
			screen.getByRole('button', { name: /Add a recipe called .osso buco./ }),
		).toBeInTheDocument();
		expect(screen.getByRole('button', { name: /Import from a link/ })).toBeInTheDocument();

		// And no offer to turn Meaning Search on, because this reader cannot:
		// only an Operator can, and an offer somebody cannot act on is worse
		// than none (ADR 0029). Whether to make it is the Operation's answer.
		expect(screen.queryByRole('button', { name: /Turn it on/ })).not.toBeInTheDocument();
	});

	it('offers Meaning Search inside nothing-found, saying what accepting costs', async () => {
		renderScreen(Recipes, {
			list_tags: { tags: [] },
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff({ offer: true }),
			search_recipes: { query: 'osso buco', closest: false, recipes: [] },
		});

		await screen.findByText(/Nothing matched .osso buco./);

		// The offer says what is actually being agreed to before the button
		// rather than after it: Kamosu ships no weights, so this downloads
		// somebody else's under somebody else's terms (ADR 0029).
		expect(screen.getByText(/EmbeddingGemma/)).toBeInTheDocument();
		expect(screen.getByRole('link', { name: /Gemma terms/ })).toHaveAttribute(
			'href',
			'https://ai.google.dev/gemma/terms',
		);
		expect(screen.getByRole('link', { name: /prohibited use policy/ })).toHaveAttribute(
			'href',
			'https://ai.google.dev/gemma/prohibited_use_policy',
		);
		expect(screen.getByRole('button', { name: /Turn it on/ })).toBeInTheDocument();
	});

	it('declines Meaning Search through the Operation, so it is never offered again', async () => {
		const { kamosu } = renderScreen(Recipes, {
			list_tags: { tags: [] },
			list_kitchens: { kitchens: [kitchen] },
			// Asked once, then answered: the second reading is what the screen
			// sees after declining, and it carries no offer.
			meaning_search_status: (() => {
				let asked = 0;
				return () =>
					asked++ === 0
						? meaningOff({ offer: true })
						: meaningOff({ state: 'declined', declined_at: '2026-08-30T00:00:00Z' });
			})(),
			decline_meaning_search: { state: 'declined' },
			search_recipes: { query: 'osso buco', closest: false, recipes: [] },
		});

		await fireEvent.click(await screen.findByRole('button', { name: /No thanks/ }));

		await vi.waitFor(() => {
			expect(kamosu.calls.map((call) => call.operation)).toContain('decline_meaning_search');
		});
		await vi.waitFor(() => {
			expect(screen.queryByRole('button', { name: /Turn it on/ })).not.toBeInTheDocument();
		});
	});

	it('shows the closest anyway, under exactly that label, rather than an empty screen', async () => {
		renderScreen(Recipes, {
			list_tags: { tags: [] },
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff({ state: 'on', on: true, model_present: true }),
			search_recipes: {
				query: 'something with squid',
				// Meaning-matching always has a nearest neighbour, so "nothing
				// found" means "nothing close enough". Kamosu says exactly that
				// and shows the closest under that label — never letting a weak
				// match pass as a good one (ADR 0027).
				closest: true,
				recipes: [
					entry({
						title: 'Calamari',
						matched: {
							where: 'ingredient',
							line: '500 g calamari',
							step_number: null,
							by: 'meaning',
						},
					}),
				],
			},
		});

		expect(await screen.findByText(/Nothing matched .something with squid./)).toBeInTheDocument();
		expect(screen.getByText('The closest anyway')).toBeInTheDocument();
		expect(screen.getByText('Calamari')).toBeInTheDocument();
		// The quoted line does not carry the words that were typed — that is
		// the whole point of a meaning match — so the card says which half of
		// searching found it, or it reads as a search gone wrong.
		expect(screen.getByText('Close in meaning')).toBeInTheDocument();
		expect(screen.getByText('500 g calamari')).toBeInTheDocument();
	});

	it('adds the recipe it offered to add, rather than pointing at a screen to add it on', async () => {
		const { kamosu } = renderScreen(Recipes, {
			list_tags: { tags: [] },
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: { query: 'osso buco', closest: false, recipes: [] },
			create_recipe: {
				branch_id: 'b_new',
				lineage_id: 'l_new',
				cookbook: cookbookLabel(),
				name: null,
				writes: true,
				mine: true,
				arrived: false,
				hand_id: 'h_1',
				language: 'en',
				origin_address: null,
				head_version_id: 'v_new',
				versions: [],
				translation: null,
				tags: [],
				related_recipes: [],
				cooked: { count: 0, last_cooked_at: null, ratings: [] },
			},
		});

		await fireEvent.click(
			await screen.findByRole('button', { name: /Add a recipe called .osso buco./ }),
		);

		// A recipe needs only a title (#6), so the query is the whole recipe:
		// it is made in the writer's own Cookbook and opened, with no screen
		// in between.
		await vi.waitFor(() => {
			const asked = kamosu.calls.find((call) => call.operation === 'create_recipe');
			expect(asked?.input).toEqual({ title: 'osso buco' });
		});
	});

	it('asks nothing of a cook in several Kitchens: a new recipe is always their own (#131)', async () => {
		// Before #131 a cook in several Kitchens was asked whose recipe it was
		// (#111). A recipe now lives in the Cookbook of whoever writes it, and
		// every Kitchen they cook in sees that, so there is nothing to ask.
		const { kamosu } = renderScreen(Recipes, {
			list_tags: { tags: [] },
			list_kitchens: { kitchens: [kitchen, marcsKitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: { query: 'osso buco', closest: false, recipes: [] },
			create_recipe: {
				branch_id: 'b_new',
				lineage_id: 'l_new',
				cookbook: cookbookLabel(),
				name: null,
				writes: true,
				mine: true,
				arrived: false,
				hand_id: 'h_1',
				language: 'en',
				origin_address: null,
				head_version_id: 'v_new',
				versions: [],
				translation: null,
				tags: [],
				related_recipes: [],
				cooked: { count: 0, last_cooked_at: null, ratings: [] },
			},
		});
		await fireEvent.click(
			await screen.findByRole('button', { name: /Add a recipe called .osso buco./ }),
		);
		expect(screen.queryByRole('dialog')).toBeNull();
		expect(screen.queryAllByRole('radio')).toHaveLength(0);
		await vi.waitFor(() => {
			const asked = kamosu.calls.find((call) => call.operation === 'create_recipe');
			expect(asked?.input).toEqual({ title: 'osso buco' });
		});
	});

	it('reads the link it offered to import, waiting on the Job it is', async () => {
		const { kamosu } = renderScreen(Recipes, {
			list_tags: { tags: [] },
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: { query: 'osso buco', closest: false, recipes: [] },
			import_web_link: { job_id: 'j_1' },
			get_job: {
				id: 'j_1',
				operation: 'import_web_link',
				status: 'completed',
				progress: {},
				error: null,
				errorCode: null,
				created_at: '2026-08-28T20:00:00.000Z',
				updated_at: '2026-08-28T20:00:01.000Z',
				result: {
					import_id: 'i_1',
					cookbook_id: 'c_1',
					source_kind: 'web',
					arrived: [
						{
							branch_id: 'b_landed',
							lineage_id: 'l_landed',
							foreign_id: 'https://example.test/osso-buco',
							status: 'created',
							title: 'Osso Buco',
						},
					],
					offered: [],
					unreadable: [],
				},
			},
		});

		await fireEvent.click(await screen.findByRole('button', { name: /Import from a link/ }));
		const field = screen.getByLabelText(/web address/);
		await fireEvent.input(field, { target: { value: 'https://example.test/osso-buco' } });
		await fireEvent.submit(field.closest('form')!);

		await vi.waitFor(() => {
			const asked = kamosu.calls.find((call) => call.operation === 'import_web_link');
			expect(asked?.input).toMatchObject({ url: 'https://example.test/osso-buco' });
		});
		// Reading a page is slow, so it is a Job, and a Job is read back the one
		// way any Job ever is (ADR 0032).
		await vi.waitFor(() => {
			expect(kamosu.calls.map((call) => call.operation)).toContain('get_job');
		});
	});

	it('says the shelf is empty rather than that a search failed, when nothing was searched for', async () => {
		renderScreen(Recipes, {
			list_tags: { tags: [] },
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: { query: null, closest: false, recipes: [] },
		});

		expect(await screen.findByText(/The shelf is empty/)).toBeInTheDocument();
		expect(screen.queryByText(/Nothing matched/)).not.toBeInTheDocument();
	});

	it('quotes the line that matched, and says where it came from', async () => {
		renderScreen(Recipes, {
			list_tags: { tags: [] },
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: {
				query: 'chocolate',
				closest: false,
				recipes: [
					entry({
						title: 'Chilli con carne',
						matched: {
							where: 'ingredient',
							line: '50 g dark chocolate',
							step_number: null,
							by: 'words',
						},
					}),
					entry({
						lineage_id: 'l_2',
						branch_id: 'b_2',
						title: 'Braised Beef',
						matched: {
							where: 'step',
							line: 'Stir in a square of chocolate.',
							step_number: 2,
							by: 'words',
						},
					}),
				],
			},
		});

		expect(await screen.findByText('50 g dark chocolate')).toBeInTheDocument();
		expect(screen.getByText('Ingredient')).toBeInTheDocument();
		// The number is the one the recipe page shows, Sections taking none.
		expect(screen.getByText('Step 2')).toBeInTheDocument();
	});

	it('does not quote a title match — the title is already on the card', async () => {
		renderScreen(Recipes, {
			list_tags: { tags: [] },
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: {
				query: 'miso',
				closest: false,
				recipes: [
					entry({
						matched: { where: 'title', line: 'Miso Soup', step_number: null, by: 'words' },
					}),
				],
			},
		});

		expect(await screen.findByText('Miso Soup')).toBeInTheDocument();
		expect(screen.getAllByText('Miso Soup')).toHaveLength(1);
	});

	it('marks a recipe shown in a Language its reader did not ask for', async () => {
		renderScreen(Recipes, {
			list_tags: { tags: [] },
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: {
				query: null,
				closest: false,
				recipes: [
					entry({ title: 'Îles Flottantes', language: 'fr', language_fallback: true }),
					entry({ lineage_id: 'l_2', branch_id: 'b_2', title: 'Miso Soup' }),
				],
			},
		});

		// Shown, not hidden: a Language preference never hides a recipe from
		// its owner (ADR 0006). The mark says which Language it fell back to.
		expect(await screen.findByText('Îles Flottantes')).toBeInTheDocument();
		expect(screen.getByText('fr')).toBeInTheDocument();
		expect(screen.getByText('In French')).toBeInTheDocument();
		expect(screen.queryByText('In English')).not.toBeInTheDocument();
	});

	it('marks the English recipes, and not the French, for somebody reading in French (#112)', async () => {
		// What the shelf looks like the moment a cook sets their Reading
		// Language to French: the Core has already chosen each title and said
		// which fell back, and the screen marks exactly those, recipes and Tags
		// alike, in words rather than codes.
		renderScreen(Recipes, {
			list_tags: {
				tags: [
					{
						id: 't_weeknight',
						cookbook_id: 'c_1',
						name: 'weeknight',
						language: 'en',
						names: [{ language: 'en', name: 'weeknight' }],
						recipes: 12,
						language_fallback: true,
					},
				],
			},
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: {
				query: null,
				closest: false,
				recipes: [
					entry({ title: 'Poulet frit coréen', language: 'fr' }),
					entry({
						lineage_id: 'l_2',
						branch_id: 'b_2',
						title: 'Miso Soup',
						language: 'en',
						language_fallback: true,
					}),
				],
			},
		});

		expect(await screen.findByText('Poulet frit coréen')).toBeInTheDocument();
		expect(screen.queryByText('In French')).not.toBeInTheDocument();
		// One mark on the card, one on the Tag chip: both say the word.
		expect(screen.getAllByText('In English')).toHaveLength(2);
		expect(screen.queryByText('In en')).not.toBeInTheDocument();
	});

	it('marks nothing on a recipe whose Language is unknown', async () => {
		renderScreen(Recipes, {
			list_tags: { tags: [] },
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: {
				query: null,
				closest: false,
				recipes: [
					entry({
						title: 'Sukiyaki Udon',
						language: 'unknown',
						language_fallback: false,
					}),
				],
			},
		});

		// A recipe honestly written in two Languages is not in a Language its
		// reader failed to ask for — it is in no single one. So it shows to
		// everybody exactly as it is: no badge, no nag (ADR 0006).
		expect(await screen.findByText('Sukiyaki Udon')).toBeInTheDocument();
		expect(screen.queryByText('unknown')).not.toBeInTheDocument();
		expect(screen.queryByText(/^In /)).not.toBeInTheDocument();
	});

	it('names no Kitchen on a card, and offers a Kitchen filter only where there are several', async () => {
		renderScreen(Recipes, {
			list_tags: { tags: [] },
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: { query: null, closest: false, recipes: [entry()] },
		});

		expect(await screen.findByText('Miso Soup')).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Maison' })).not.toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'All' })).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'My recipes' })).toBeInTheDocument();
	});

	it('offers one filter per Kitchen where the Person cooks in several, and asks with it', async () => {
		const { kamosu } = renderScreen(Recipes, {
			list_tags: { tags: [] },
			list_kitchens: {
				kitchens: [kitchen, marcsKitchen],
			},
			meaning_search_status: meaningOff(),
			search_recipes: { query: null, closest: false, recipes: [entry()] },
		});

		const marc = await screen.findByRole('button', { name: 'Chez Marc' });
		await fireEvent.click(marc);

		await vi.waitFor(() => {
			const asked = kamosu.calls.filter((call) => call.operation === 'search_recipes');
			expect(asked.at(-1)?.input).toMatchObject({ kitchen_id: 'k_marc' });
		});

		// Held on the screen and nowhere else: nothing was written to the URL,
		// so a filter cannot greet you tomorrow or survive the back button.
		expect(window.location.search).toBe('');
	});

	// --- Browsing by tag (#104) ---------------------------------------------
	//
	// The half of Aurélien's choice that makes a tag something you get around
	// by rather than a word a search happens to match. The three states are
	// covered between these: a library with no tags, which is every new
	// instance and was this project's own on the day it imported 86 recipes; a
	// handful; and a full shelf with one tag named in another Language.

	it('draws no tag row at all where no Cookbook files by anything yet', async () => {
		renderScreen(Recipes, {
			list_tags: { tags: [] },
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: { query: null, closest: false, recipes: [entry()] },
		});

		expect(await screen.findByText('Miso Soup')).toBeInTheDocument();
		// A heading over nothing is worse than no heading. The two filters that
		// always exist are untouched.
		expect(screen.queryByText('Tags')).not.toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'All' })).toBeInTheDocument();
	});

	it('narrows the shelf by a tag, through the Operation, and counts by that tag', async () => {
		const { kamosu } = renderScreen(Recipes, {
			list_tags: {
				tags: [
					shelfTag('t_spicy', 'spicy', 14),
					shelfTag('t_batch', 'batch cook', 9),
					// Named only in French, read by somebody reading English.
					{
						id: 't_mijote',
						cookbook_id: 'c_1',
						name: 'mijoté',
						language: 'fr',
						names: [{ language: 'fr', name: 'mijoté' }],
						recipes: 7,
						// The Core says this, not the screen (#104).
						language_fallback: true,
					},
				],
			},
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: { query: null, closest: false, recipes: [entry()] },
		});

		// Alphabetical by the word the reader sees, which is the only order a
		// person can predict.
		const chip = await screen.findByRole('button', { name: /spicy/ });
		await fireEvent.click(chip);

		await vi.waitFor(() => {
			const asked = kamosu.calls.filter((call) => call.operation === 'search_recipes');
			expect(asked.at(-1)?.input).toMatchObject({ tag_id: 't_spicy' });
		});

		// The count is said by the tag rather than by the shelf, because that is
		// the question the reader just asked.
		expect(await screen.findByText('1 tagged spicy')).toBeInTheDocument();
		expect(chip).toHaveAttribute('aria-pressed', 'true');

		// A tag named in another Language keeps its own word and says which
		// Language that is (ADR 0006).
		expect(screen.getByRole('button', { name: /mijoté/ })).toBeInTheDocument();
		expect(screen.getByText('fr')).toBeInTheDocument();
		expect(screen.getByText('In French')).toBeInTheDocument();

		// Tapping the lit chip again lets it go, which is the only way back to
		// the whole shelf without leaving the screen.
		await fireEvent.click(chip);
		await vi.waitFor(() => {
			const asked = kamosu.calls.filter((call) => call.operation === 'search_recipes');
			expect(asked.at(-1)?.input).toMatchObject({ tag_id: null });
		});
	});

	it('says which tag found nothing, rather than that the library is empty', async () => {
		const { kamosu } = renderScreen(Recipes, {
			list_tags: { tags: [shelfTag('t_try', 'to try', 0)] },
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: { query: null, closest: false, recipes: [] },
		});

		await fireEvent.click(await screen.findByRole('button', { name: /to try/ }));

		// "You have no recipes" would be a lie about the other eighty-five.
		await vi.waitFor(() => {
			expect(kamosu.calls.at(-1)?.input).toMatchObject({ tag_id: 't_try' });
		});
		expect(await screen.findByText('Nothing is tagged to try yet.')).toBeInTheDocument();
	});

	it('offers each Kitchen the words its own Cookbooks file by, and lets the narrowing go with them', async () => {
		// The chips follow the Kitchen chip: a word no Cookbook in that Kitchen
		// files by can only ever find nothing, and worse, left lit it narrows
		// the shelf to nothing with no chip on screen to turn off. Each
		// Kitchen's words are its own list, read once (#131): the one-per-word
		// list keeps ONE Tag per word, and that Tag may belong to a Cookbook the
		// Kitchen does not see.
		const ines = kitchenAnswer('k_ines', {
			name: 'Chez Inès',
			cookbooks: [cookbookLabel(), cookbookLabel('c_ines', ['Inès'])],
		});
		const everywhere = [
			shelfTag('t_spicy', 'spicy', 14),
			shelfTag('t_marc', 'from Marc', 3, 'c_marc'),
			shelfTag('t_ines', 'from Inès', 2, 'c_ines'),
		];
		const kamosu = standIn({
			list_tags: () => {
				const asked = kamosu.calls.filter((call) => call.operation === 'list_tags').at(-1)
					?.input as { kitchen_id?: string } | undefined;
				if (asked?.kitchen_id === 'k_marc') {
					return { tags: [everywhere[0]!, everywhere[1]!] };
				}
				if (asked?.kitchen_id) return { tags: [everywhere[0]!, everywhere[2]!] };
				return { tags: everywhere };
			},
			list_kitchens: { kitchens: [kitchen, marcsKitchen, ines] },
			meaning_search_status: meaningOff(),
			search_recipes: { query: null, closest: false, recipes: [entry()] },
		});
		render(ShelfTestHarness, { props: { client: kamosu.client, tag: null } });

		// Every word to begin with, because the shelf is every Kitchen.
		await fireEvent.click(await screen.findByRole('button', { name: /from Inès/ }));
		await vi.waitFor(() => {
			const asked = kamosu.calls.filter((call) => call.operation === 'search_recipes');
			expect(asked.at(-1)?.input).toMatchObject({ tag_id: 't_ines' });
		});
		expect(screen.getByRole('button', { name: /from Marc/ })).toBeInTheDocument();
		expect(kamosu.calls.filter((call) => call.operation === 'list_tags')).toHaveLength(4);

		// Filtering to Marc's Kitchen offers his Kitchen's words: Inès's goes,
		// and the narrowing goes with it rather than stranding an invisible
		// filter. Aurélien's own words stay: his Cookbook is seen there too.
		await fireEvent.click(screen.getByRole('button', { name: 'Chez Marc' }));

		await vi.waitFor(() => {
			expect(screen.queryByRole('button', { name: /from Inès/ })).not.toBeInTheDocument();
		});
		expect(screen.getByRole('button', { name: /from Marc/ })).toBeInTheDocument();
		expect(screen.getByRole('button', { name: /spicy/ })).toBeInTheDocument();
		await vi.waitFor(() => {
			const asked = kamosu.calls.filter((call) => call.operation === 'search_recipes');
			expect(asked.at(-1)?.input).toMatchObject({ kitchen_id: 'k_marc', tag_id: null });
		});
	});

	it('asks once for every word on the shelf, and once for each Kitchen’s (#131)', async () => {
		const { kamosu } = renderScreen(Recipes, {
			list_tags: { tags: [shelfTag('t_spicy', 'spicy', 14)] },
			list_kitchens: { kitchens: [kitchen, marcsKitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: { query: null, closest: false, recipes: [entry()] },
		});
		expect(await screen.findByRole('button', { name: /spicy/ })).toBeInTheDocument();
		await vi.waitFor(() =>
			expect(kamosu.calls.filter((call) => call.operation === 'list_tags')).toHaveLength(3),
		);
		const asked = kamosu.calls
			.filter((call) => call.operation === 'list_tags')
			.map((call) => call.input);
		expect(asked).toEqual([
			{ everywhere: true },
			{ kitchen_id: kitchen.id },
			{ kitchen_id: 'k_marc' },
		]);
	});

	it('shows the shelf rather than a failure when ?tag= names a tag that is not here', async () => {
		// Stale, or somebody else's: `search_recipes` answers not-found (ADR
		// 0040). Left standing, that wore the whole screen as a failure with no
		// chip lit to turn the filter off.
		const kamosu = standIn({
			list_tags: { tags: [shelfTag('t_spicy', 'spicy', 14)] },
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: () => {
				const asked = kamosu.calls.filter((call) => call.operation === 'search_recipes');
				return asked.some((call) => (call.input as { tag_id?: string }).tag_id === 't_gone')
					? { refuse: 'not_found' as const, message: 'no such Tag' }
					: { query: null, closest: false, recipes: [entry()] };
			},
		});
		render(ShelfTestHarness, { props: { client: kamosu.client, tag: 't_gone' } });

		// The shelf, not "Your recipes could not be searched just now."
		expect(await screen.findByText('Miso Soup')).toBeInTheDocument();
		expect(screen.queryByText(/could not be searched/)).not.toBeInTheDocument();
		await vi.waitFor(() => {
			const asked = kamosu.calls.filter((call) => call.operation === 'search_recipes');
			expect(asked.at(-1)?.input).toMatchObject({ tag_id: null });
		});
	});

	it('opens already narrowed when a chip on a recipe sent the reader here', async () => {
		// `?tag=` is a message from the recipe page, read by the route into a
		// prop. Nothing on this screen ever writes it, which is why arriving
		// tagged and arriving by the tab bar are two different renders rather
		// than one screen with a memory.
		const kamosu = standIn({
			list_tags: { tags: [shelfTag('t_spicy', 'spicy', 14)] },
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: { query: null, closest: false, recipes: [entry()] },
		});
		render(ShelfTestHarness, { props: { client: kamosu.client, tag: 't_spicy' } });

		// The first ask already carries the tag: the reader does not watch the
		// whole shelf draw and then collapse.
		await vi.waitFor(() => {
			const asked = kamosu.calls.filter((call) => call.operation === 'search_recipes');
			expect(asked.at(0)?.input).toMatchObject({ tag_id: 't_spicy' });
		});
		expect(await screen.findByText('1 tagged spicy')).toBeInTheDocument();
		expect(await screen.findByRole('button', { name: /spicy/ })).toHaveAttribute(
			'aria-pressed',
			'true',
		);
	});

	// --- Searching as you type (#121) ---------------------------------------
	//
	// The shelf, the Component picker and the Related sheet share one helper for
	// the waiting and the dropping of stale answers. Svelte re-asks only when a
	// value it SAW read changes, so a helper that reads the search box in the
	// wrong place still works on first open and then silently stops following
	// the thumb. These drive the real inputs, which is the only place that fails.

	it('follows what is typed, and asks again when a filter or a tag changes', async () => {
		const { kamosu } = renderScreen(Recipes, {
			list_tags: { tags: [shelfTag('t_spicy', 'spicy', 1)] },
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: ({ query }) =>
				query === 'curry'
					? {
							query,
							closest: false,
							recipes: [entry({ lineage_id: 'l_curry', title: 'Katsu Curry' })],
						}
					: { query: query ?? null, closest: false, recipes: [entry()] },
		});
		expect(await screen.findByText('Miso Soup')).toBeInTheDocument();

		await fireEvent.input(screen.getByRole('searchbox'), { target: { value: 'curry' } });

		expect(await screen.findByText('Katsu Curry')).toBeInTheDocument();
		expect(screen.queryByText('Miso Soup')).not.toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: 'My recipes' }));
		await vi.waitFor(() => {
			expect(searches(kamosu).at(-1)).toMatchObject({ query: 'curry', mine: true });
		});

		await fireEvent.click(screen.getByRole('button', { name: /spicy/ }));
		await vi.waitFor(() => {
			expect(searches(kamosu).at(-1)).toMatchObject({
				query: 'curry',
				mine: true,
				tag_id: 't_spicy',
			});
		});
	});

	it('asks the same search again once Meaning Search is on, so what it finds appears', async () => {
		let on = false;
		const { kamosu } = renderScreen(Recipes, {
			list_tags: { tags: [] },
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: () =>
				on ? meaningOff({ state: 'on', on: true }) : meaningOff({ offer: true }),
			accept_meaning_search_terms: { state: 'accepted' },
			download_meaning_model: { job_id: 'j_model' },
			build_meaning_index: () => {
				on = true;
				return { job_id: 'j_index' };
			},
			get_job: ({ job_id }) => ({
				id: job_id,
				operation: job_id === 'j_model' ? 'download_meaning_model' : 'build_meaning_index',
				status: 'completed',
				progress: {},
				error: null,
				errorCode: null,
				created_at: '2026-09-26T10:00:00.000Z',
				updated_at: '2026-09-26T10:00:01.000Z',
				result: {},
			}),
			// Words alone find nothing; by meaning, the stew is close enough.
			search_recipes: ({ query }) => ({
				query: query ?? null,
				closest: false,
				recipes: on ? [entry({ lineage_id: 'l_stew', title: 'Veal Stew' })] : [],
			}),
		});

		await fireEvent.input(screen.getByRole('searchbox'), { target: { value: 'osso buco' } });
		await fireEvent.click(await screen.findByRole('button', { name: /Turn it on/ }));

		expect(await screen.findByText('Veal Stew')).toBeInTheDocument();
		expect(searches(kamosu).at(-1)).toMatchObject({ query: 'osso buco' });
	});

	it('asks its current search again when the server answers otherwise than the phone did (#76)', async () => {
		let served = 'phone';
		const { kamosu } = renderScreen(Recipes, {
			list_tags: { tags: [] },
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: ({ query }) => ({
				query: query ?? null,
				closest: false,
				recipes: [
					served === 'phone' ? entry() : entry({ lineage_id: 'l_curry', title: 'Katsu Curry' }),
				],
			}),
		});
		await fireEvent.input(screen.getByRole('searchbox'), { target: { value: 'soup' } });
		await vi.waitFor(() => expect(searches(kamosu).at(-1)).toMatchObject({ query: 'soup' }));
		expect(await screen.findByText('Miso Soup')).toBeInTheDocument();

		served = 'server';
		serverAnsweredSearchesOtherwise();

		expect(await screen.findByText('Katsu Curry')).toBeInTheDocument();
		expect(searches(kamosu).at(-1)).toMatchObject({ query: 'soup' });
	});
});

describe('the + for every new recipe (#174)', () => {
	afterEach(() => {
		Reflect.deleteProperty(navigator, 'onLine');
	});

	/** A full shelf that already holds a chicken curry, which is the whole case. */
	const fullShelf = {
		list_tags: { tags: [] },
		list_kitchens: { kitchens: [kitchen] },
		meaning_search_status: meaningOff(),
		search_recipes: {
			query: null,
			closest: false,
			recipes: [
				entry({ title: 'Chicken katsu curry' }),
				entry({ lineage_id: 'l_2', branch_id: 'b_2', title: 'Miso Soup' }),
			],
		},
	};

	const made = {
		branch_id: 'b_new',
		lineage_id: 'l_new',
		cookbook: cookbookLabel(),
		name: null,
		writes: true,
		mine: true,
		arrived: false,
		hand_id: 'h_1',
		language: 'en',
		origin_address: null,
		head_version_id: 'v_new',
		versions: [],
		translation: null,
		tags: [],
		related_recipes: [],
		cooked: { count: 0, last_cooked_at: null, ratings: [] },
	};

	async function openThePlus() {
		const plus = await screen.findByRole('button', { name: 'Add a recipe' });
		expect(plus).toHaveAttribute('aria-expanded', 'false');
		await fireEvent.click(plus);
		expect(plus).toHaveAttribute('aria-expanded', 'true');
		return plus;
	}

	it('is there with a full library and no search, and offers the five sources', async () => {
		renderScreen(Recipes, fullShelf);
		await screen.findByText('Chicken katsu curry');

		await openThePlus();
		expect(screen.getByRole('button', { name: /From a link/ })).toBeInTheDocument();
		expect(screen.getByRole('button', { name: /From a Kamosu zip file/ })).toBeInTheDocument();
		// A PDF has a row of its own, apart from the zip file (#176, settled on #174).
		expect(
			screen.getByRole('button', { name: /From a PDF.*A recipe printed or saved as a PDF/ }),
		).toBeInTheDocument();
		expect(screen.getByRole('button', { name: /Write it yourself/ })).toBeInTheDocument();
		// The fourth, from #175: pasted text, before any recipe page exists.
		expect(
			screen.getByRole('button', {
				name: /From pasted text.*Kamosu sorts it into title, ingredients and method/,
			}),
		).toBeInTheDocument();
		// The quiet row #93 put above the shelf is gone, replaced by this.
		expect(screen.queryByText('Add a recipe from a link')).not.toBeInTheDocument();
	});

	it('writes a recipe whose title shares words with one already here, and opens it', async () => {
		const { kamosu } = renderScreen(Recipes, { ...fullShelf, create_recipe: made });

		await openThePlus();
		await fireEvent.click(screen.getByRole('button', { name: /Write it yourself/ }));
		// The list closes and the title is asked for in place (ADR 0027).
		expect(screen.queryByRole('button', { name: /From a link/ })).not.toBeInTheDocument();
		const field = screen.getByLabelText('What is it called?');
		await fireEvent.input(field, { target: { value: 'Chicken curry' } });
		await fireEvent.submit(field.closest('form')!);

		// A search for these words finds the katsu curry and never offers to add
		// one. The + asks for no search, and a title is a whole recipe (#6), made
		// in the writer's own Cookbook with nothing asked (ADR 0041).
		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/recipes/b_new'));
		const asked = kamosu.calls.find((call) => call.operation === 'create_recipe');
		expect(asked?.input).toEqual({ title: 'Chicken curry' });
	});

	/** A lemon loaf as `read_pasted_recipe` answers it: a title, a heading, and the split. */
	const LEMON_LOAF: ReadPastedRecipeOutput = {
		title: 'Lemon drizzle loaf',
		lines: [
			{ kind: 'line', text: '225g butter, softened' },
			{ kind: 'line', text: '4 eggs' },
			{ kind: 'section', text: 'For the drizzle' },
			{ kind: 'line', text: '1½ lemons, juiced' },
			{ kind: 'line', text: 'Beat the butter and sugar until pale.' },
			{ kind: 'line', text: 'Pour the drizzle over the warm cake.' },
		],
		boundary: 4,
		note: null,
	};

	async function pasteThroughThePlus(text = 'Lemon drizzle loaf\n225g butter…') {
		await openThePlus();
		await fireEvent.click(screen.getByRole('button', { name: /From pasted text/ }));
		const field = screen.getByLabelText('The recipe, pasted as text');
		await fireEvent.input(field, { target: { value: text } });
		await fireEvent.submit(field.closest('form')!);
		return screen.findByRole('dialog', { name: 'From pasted text' });
	}

	// #175, Aurélien's choice: pasted text is a fourth source, so the recipe
	// is made from the text rather than from a title somebody makes up first.
	it('reads pasted text, shows what it made, and makes the recipe from it unsaved', async () => {
		const { kamosu } = renderScreen(Recipes, {
			...fullShelf,
			read_pasted_recipe: LEMON_LOAF,
			create_recipe: made,
		});

		const sheet = await pasteThroughThePlus();
		expect(kamosu.calls.find((call) => call.operation === 'read_pasted_recipe')?.input).toEqual({
			text: 'Lemon drizzle loaf\n225g butter…',
		});
		expect(sheet).toHaveTextContent('3 ingredients and 2 steps.');
		expect(screen.getByLabelText('Title')).toHaveValue('Lemon drizzle loaf');
		expect(sheet).toHaveTextContent('Nothing is saved until you save.');

		await fireEvent.click(screen.getByRole('button', { name: 'Make this recipe' }));
		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/recipes/b_new'));
		// Made from its title alone; the lines wait on the writing screen for Save.
		expect(kamosu.calls.find((call) => call.operation === 'create_recipe')?.input).toEqual({
			title: 'Lemon drizzle loaf',
		});
		expect(kamosu.calls.some((call) => call.operation === 'save_recipe_version')).toBe(false);
		expect(takePaste('b_new')).toEqual({
			title: 'Lemon drizzle loaf',
			note: null,
			ingredients: [
				{ kind: 'ingredient', text: '225g butter, softened' },
				{ kind: 'ingredient', text: '4 eggs' },
				{ kind: 'section', text: 'For the drizzle' },
				{ kind: 'ingredient', text: '1½ lemons, juiced' },
			],
			steps: [
				{ kind: 'step', text: 'Beat the butter and sugar until pale.' },
				{ kind: 'step', text: 'Pour the drizzle over the warm cake.' },
			],
		});
		// Taken once: a reload later opens the recipe as it is saved.
		expect(takePaste('b_new')).toBeUndefined();
	});

	it('asks for a title when the paste carried none, before anything is made', async () => {
		const { kamosu } = renderScreen(Recipes, {
			...fullShelf,
			read_pasted_recipe: { ...LEMON_LOAF, title: null },
			create_recipe: made,
		});

		const sheet = await pasteThroughThePlus();
		expect(sheet).toHaveTextContent('No title in it, so give it one here.');
		const make = screen.getByRole('button', { name: 'Make this recipe' });
		expect(make).toBeDisabled();

		await fireEvent.input(screen.getByLabelText('Title'), { target: { value: 'Lemon loaf' } });
		await fireEvent.click(make);
		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/recipes/b_new'));
		expect(kamosu.calls.find((call) => call.operation === 'create_recipe')?.input).toEqual({
			title: 'Lemon loaf',
		});
		expect(takePaste('b_new')?.title).toBe('Lemon loaf');
	});

	it('says why the paste could not be read, where it was pasted', async () => {
		renderScreen(Recipes, {
			...fullShelf,
			read_pasted_recipe: { refuse: 'bad_request', message: 'too long' },
		});
		await openThePlus();
		await fireEvent.click(screen.getByRole('button', { name: /From pasted text/ }));
		const field = screen.getByLabelText('The recipe, pasted as text');
		await fireEvent.input(field, { target: { value: 'something' } });
		await fireEvent.submit(field.closest('form')!);
		expect(await screen.findByRole('alert')).toHaveTextContent('too long');
		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('says why writing was refused, where the title was typed', async () => {
		renderScreen(Recipes, {
			...fullShelf,
			create_recipe: { refuse: 'bad_request', message: 'a recipe needs a title' },
		});

		await openThePlus();
		await fireEvent.click(screen.getByRole('button', { name: /Write it yourself/ }));
		const field = screen.getByLabelText('What is it called?');
		await fireEvent.input(field, { target: { value: 'Chicken curry' } });
		await fireEvent.submit(field.closest('form')!);

		expect(await screen.findByRole('alert')).toHaveTextContent('a recipe needs a title');
		expect(went).not.toHaveBeenCalled();
	});

	it('reads a link in place, waiting on the Job it is', async () => {
		const { kamosu } = renderScreen(Recipes, {
			...fullShelf,
			import_web_link: { job_id: 'j_1' },
			get_job: {
				id: 'j_1',
				operation: 'import_web_link',
				status: 'completed',
				progress: {},
				error: null,
				errorCode: null,
				created_at: '2026-09-27T10:00:00.000Z',
				updated_at: '2026-09-27T10:00:01.000Z',
				result: {
					import_id: 'i_1',
					cookbook_id: 'c_1',
					source_kind: 'web',
					arrived: [
						{
							branch_id: 'b_landed',
							lineage_id: 'l_landed',
							foreign_id: 'https://example.test/chicken-curry',
							status: 'created',
							title: 'Chicken curry',
						},
					],
					offered: [],
					unreadable: [],
				},
			},
		});

		await openThePlus();
		await fireEvent.click(screen.getByRole('button', { name: /From a link/ }));
		const field = screen.getByLabelText(/web address/);
		await fireEvent.input(field, { target: { value: 'https://example.test/chicken-curry' } });
		await fireEvent.submit(field.closest('form')!);

		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/recipes/b_landed'));
		expect(kamosu.calls.find((call) => call.operation === 'import_web_link')?.input).toEqual({
			url: 'https://example.test/chicken-curry',
		});
	});

	it('opens the phone’s own picker for a Kamosu zip file, and brings the file in', async () => {
		const upload = vi.fn(async () => 'u_staged');
		const kamosu = standIn({
			...fullShelf,
			import_bundle: { job_id: 'j_9' },
			get_job: {
				id: 'j_9',
				operation: 'import_bundle',
				status: 'completed',
				progress: { done: 1, total: 1 },
				error: null,
				errorCode: null,
				created_at: '2026-09-27T10:00:00.000Z',
				updated_at: '2026-09-27T10:00:02.000Z',
				result: {
					import_id: 'i_1',
					kitchen_id: 'k_home',
					source_kind: 'bundle',
					arrived: [
						{
							foreign_id: 'b_theirs',
							status: 'created',
							lineage_id: 'l_soba',
							branch_id: 'b_soba',
							title: 'Soba with walnut miso',
							subject: true,
						},
					],
					offered: [],
					unreadable: [],
					left_out: [],
					related_candidates: [],
				},
			},
		});
		render(ShelfTestHarness, { props: { client: kamosu.client, upload } });
		const picked = vi.spyOn(HTMLInputElement.prototype, 'click');

		await openThePlus();
		await fireEvent.click(screen.getByRole('button', { name: /From a Kamosu zip file/ }));
		// No field of its own: the tap is the picker (ADR 0027).
		const field = screen.getByLabelText('Add a recipe from a Kamosu zip file');
		expect(picked.mock.contexts).toContain(field);
		picked.mockRestore();

		await fireEvent.change(field, {
			target: { files: [new File(['PK'], 'soba.zip', { type: 'application/zip' })] },
		});
		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/recipes/b_soba'));
		expect(kamosu.calls.find((call) => call.operation === 'import_bundle')?.input).toEqual({
			upload_id: 'u_staged',
		});
	});

	/** Biscuit de Savoie as `read_recipe_pdf` answers it: headings, the split, and a note. */
	const BISCUIT: ReadPastedRecipeOutput = {
		title: 'Biscuit de Savoie',
		lines: [
			{ kind: 'section', text: 'Ingrédients :' },
			{ kind: 'line', text: 'Trois gros œufs' },
			{ kind: 'line', text: '150 g de sucre' },
			{ kind: 'section', text: 'Préparation' },
			{ kind: 'line', text: '1. Préparer les jaunes : on ajoute le sucre aux jaunes.' },
			{ kind: 'line', text: '2. Incorporer les poudres : la farine, puis la fécule.' },
		],
		boundary: 3,
		note: 'Ce gros gâteau est incroyablement léger.\n\nSuggestions de service : une salade de fruits.',
	};

	/** Pick a PDF from the + the way a phone does: the tap is the picker. */
	async function pickAPdf() {
		const picked = vi.spyOn(HTMLInputElement.prototype, 'click');
		await openThePlus();
		await fireEvent.click(screen.getByRole('button', { name: /From a PDF/ }));
		const field = screen.getByLabelText('Add a recipe from a PDF');
		expect(picked.mock.contexts).toContain(field);
		picked.mockRestore();
		expect(field).toHaveAttribute('accept', '.pdf,application/pdf');
		await fireEvent.change(field, {
			target: { files: [new File(['%PDF-1.4'], 'biscuit.pdf', { type: 'application/pdf' })] },
		});
	}

	// #176: a PDF lands on the sheet a paste is checked on, with the reader's
	// answer, and is made from there the way a paste is.
	it('reads a picked PDF onto the paste sheet, where the split moves and the recipe is made', async () => {
		const upload = vi.fn(async () => 'u_pdf');
		const kamosu = standIn({ ...fullShelf, read_recipe_pdf: BISCUIT, create_recipe: made });
		render(ShelfTestHarness, { props: { client: kamosu.client, upload } });

		await pickAPdf();
		const sheet = await screen.findByRole('dialog', { name: 'From a PDF' });
		expect(kamosu.calls.find((call) => call.operation === 'read_recipe_pdf')?.input).toEqual({
			upload_id: 'u_pdf',
		});
		expect(screen.getByLabelText('Title')).toHaveValue('Biscuit de Savoie');
		expect(sheet).toHaveTextContent('2 ingredients and 2 steps.');
		// What it said about the recipe is shown, in neither list.
		expect(sheet).toHaveTextContent('Ce gros gâteau est incroyablement léger.');

		// The split moves, as it does for a paste.
		await fireEvent.click(screen.getByRole('button', { name: 'A line earlier' }));
		expect(sheet).toHaveTextContent('1 ingredients and 3 steps.');
		await fireEvent.click(screen.getByRole('button', { name: 'A line later' }));
		expect(sheet).toHaveTextContent('2 ingredients and 2 steps.');

		await fireEvent.click(screen.getByRole('button', { name: 'Make this recipe' }));
		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/recipes/b_new'));
		expect(kamosu.calls.find((call) => call.operation === 'create_recipe')?.input).toEqual({
			title: 'Biscuit de Savoie',
		});
		expect(kamosu.calls.some((call) => call.operation === 'save_recipe_version')).toBe(false);
		expect(takePaste('b_new')).toEqual({
			title: 'Biscuit de Savoie',
			note: BISCUIT.note,
			ingredients: [
				{ kind: 'section', text: 'Ingrédients :' },
				{ kind: 'ingredient', text: 'Trois gros œufs' },
				{ kind: 'ingredient', text: '150 g de sucre' },
			],
			steps: [
				{ kind: 'section', text: 'Préparation' },
				{ kind: 'step', text: '1. Préparer les jaunes : on ajoute le sucre aux jaunes.' },
				{ kind: 'step', text: '2. Incorporer les poudres : la farine, puis la fécule.' },
			],
		});
	});

	it('says in its own words why a scanned PDF could not be read, and makes nothing', async () => {
		const upload = vi.fn(async () => 'u_scan');
		const kamosu = standIn({
			...fullShelf,
			read_recipe_pdf: {
				refuse: 'bad_request',
				message: 'the Core says it in English',
				reason: 'pdf_has_no_text',
			},
		});
		render(ShelfTestHarness, { props: { client: kamosu.client, upload } });

		await pickAPdf();
		// Aurélien's wording, chosen on #176: why, and what to do instead.
		expect(await screen.findByRole('alert')).toHaveTextContent(
			"This PDF is a scan or a photo of a page, so it has no text to read. If it came from a website, use From a link instead, or copy the recipe's text and use From pasted text.",
		);
		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
		expect(kamosu.calls.some((call) => call.operation === 'create_recipe')).toBe(false);
	});

	it('says any other refusal of a PDF in the words it was refused with', async () => {
		const upload = vi.fn(async () => 'u_bad');
		const kamosu = standIn({
			...fullShelf,
			read_recipe_pdf: { refuse: 'bad_request', message: 'that file is not a PDF' },
		});
		render(ShelfTestHarness, { props: { client: kamosu.client, upload } });

		await pickAPdf();
		expect(await screen.findByRole('alert')).toHaveTextContent('that file is not a PDF');
	});

	it('closes its list on Escape, and when the + is tapped again', async () => {
		renderScreen(Recipes, fullShelf);

		const plus = await openThePlus();
		await fireEvent.keyDown(window, { key: 'Escape' });
		expect(plus).toHaveAttribute('aria-expanded', 'false');
		expect(screen.queryByRole('button', { name: /From a link/ })).not.toBeInTheDocument();

		await fireEvent.click(plus);
		await fireEvent.click(plus);
		expect(plus).toHaveAttribute('aria-expanded', 'false');
	});

	it('says offline that each source waits for the server', async () => {
		Object.defineProperty(navigator, 'onLine', { value: false, configurable: true });
		renderScreen(Recipes, fullShelf);

		await openThePlus();
		// None of the three is queued: an offline import queue would be a
		// merge, and Kamosu never merges (#76, ADR 0013). The outbox keeps no
		// new recipe either, so each row stays and says what it waits for.
		expect(
			screen.getByRole('button', { name: 'Importing from a link waits for the server' }),
		).toBeDisabled();
		expect(
			screen.getByRole('button', { name: 'Bringing a recipe in waits for the server' }),
		).toBeDisabled();
		expect(
			screen.getByRole('button', { name: 'Writing a recipe waits for the server' }),
		).toBeDisabled();
	});
});

describe('going back to the shelf (#191)', () => {
	afterEach(() => {
		// `scrollTo` is spied on below, and test files share one window.
		vi.restoreAllMocks();
	});

	/**
	 * A shelf that answers what it was asked: the search box's words find the
	 * one recipe with them in its title, and the filters are read off the ask,
	 * so a test can see which of them came back.
	 */
	const answering = (kitchens = [kitchen, marcsKitchen]) =>
		standIn({
			list_tags: { tags: [shelfTag('t_spicy', 'spicy', 1)] },
			list_kitchens: { kitchens },
			meaning_search_status: meaningOff(),
			search_recipes: (input) => {
				const everything = [
					entry({ lineage_id: 'l_1', title: 'Chicken Katsu' }),
					entry({ lineage_id: 'l_2', branch_id: 'b_2', title: 'Miso Soup' }),
				];
				const query = input.query ?? null;
				return {
					query,
					closest: false,
					recipes: query
						? everything.filter((one) => one.title.toLowerCase().includes(query))
						: everything,
				};
			},
		});

	type Route = ReturnType<typeof render<typeof RouteTestHarness>>;

	/** Opening a recipe: SvelteKit keeps the shelf, then the screen is taken down. */
	const leave = (route: Route) => {
		const left = route.component.snapshot().capture();
		cleanup();
		return left;
	};

	/** Going back: a new screen, handed what was kept, as SvelteKit does on back. */
	const goBack = (kamosu: StandIn, left: unknown) => {
		const route = render(RouteTestHarness, { props: { client: kamosu.client } });
		route.component.snapshot().restore(left);
		return route;
	};

	const katsuAtMarcsSpicy = { query: 'katsu', kitchen_id: 'k_marc', tag_id: 't_spicy' };

	it('comes back with the words, the filters, the results and the place in the list', async () => {
		const scrolled = vi.spyOn(window, 'scrollTo').mockImplementation(() => {});
		const kamosu = answering();
		const first = render(RouteTestHarness, { props: { client: kamosu.client } });

		await fireEvent.input(await screen.findByRole('searchbox'), { target: { value: 'katsu' } });
		await fireEvent.click(await screen.findByRole('button', { name: 'Chez Marc' }));
		await fireEvent.click(await screen.findByRole('button', { name: /spicy/ }));
		await vi.waitFor(() => expect(searches(kamosu).at(-1)).toMatchObject(katsuAtMarcsSpicy));
		await screen.findByText('1 found');
		vi.stubGlobal('scrollY', 640);

		const left = leave(first);
		vi.unstubAllGlobals();
		goBack(kamosu, left);

		await vi.waitFor(() => expect(screen.getByRole('searchbox')).toHaveValue('katsu'));
		expect(screen.getByRole('button', { name: 'Chez Marc' })).toHaveAttribute(
			'aria-pressed',
			'true',
		);
		expect(await screen.findByRole('button', { name: /spicy/ })).toHaveAttribute(
			'aria-pressed',
			'true',
		);
		expect(await screen.findByText('Chicken Katsu')).toBeInTheDocument();
		expect(screen.queryByText('Miso Soup')).not.toBeInTheDocument();
		expect(searches(kamosu).at(-1)).toMatchObject(katsuAtMarcsSpicy);
		// Where the list was, once the list is there to scroll.
		await vi.waitFor(() => expect(scrolled).toHaveBeenCalledWith(0, 640));
		expect(scrolled).toHaveBeenCalledTimes(1);
	});

	it('comes back to a Tag held with no words typed', async () => {
		vi.spyOn(window, 'scrollTo').mockImplementation(() => {});
		const kamosu = answering();
		const first = render(RouteTestHarness, { props: { client: kamosu.client } });

		await fireEvent.click(await screen.findByRole('button', { name: /spicy/ }));
		await vi.waitFor(() => expect(searches(kamosu).at(-1)).toMatchObject({ tag_id: 't_spicy' }));

		goBack(kamosu, leave(first));

		await vi.waitFor(() =>
			expect(screen.getByRole('button', { name: /spicy/ })).toHaveAttribute('aria-pressed', 'true'),
		);
		expect(screen.getByRole('searchbox')).toHaveValue('');
		expect(screen.getByRole('button', { name: 'All' })).toHaveAttribute('aria-pressed', 'true');
		await vi.waitFor(() =>
			expect(searches(kamosu).at(-1)).toMatchObject({
				query: null,
				kitchen_id: null,
				mine: false,
				tag_id: 't_spicy',
			}),
		);
	});

	it('lets go of a Kitchen filter it can no longer offer a chip for', async () => {
		vi.spyOn(window, 'scrollTo').mockImplementation(() => {});
		const kamosu = answering();
		const first = render(RouteTestHarness, { props: { client: kamosu.client } });

		await fireEvent.click(await screen.findByRole('button', { name: 'Chez Marc' }));
		await vi.waitFor(() => expect(searches(kamosu).at(-1)).toMatchObject({ kitchen_id: 'k_marc' }));
		const left = leave(first);

		// Left Marc's Kitchen since: one Kitchen is no row of Kitchen chips,
		// so a filter held on his would have nothing to turn it off.
		const fewer = answering([kitchen]);
		goBack(fewer, left);

		await vi.waitFor(() =>
			expect(searches(fewer).at(-1)).toMatchObject({ kitchen_id: null, mine: false }),
		);
		expect(screen.getByRole('button', { name: 'All' })).toHaveAttribute('aria-pressed', 'true');
		expect(screen.queryByRole('button', { name: 'Chez Marc' })).not.toBeInTheDocument();
	});

	it('forgets the place in the list when the search it came back to fails', async () => {
		const scrolled = vi.spyOn(window, 'scrollTo').mockImplementation(() => {});
		const kamosu = answering();
		const first = render(RouteTestHarness, { props: { client: kamosu.client } });
		await fireEvent.input(await screen.findByRole('searchbox'), { target: { value: 'katsu' } });
		await vi.waitFor(() => expect(searches(kamosu).at(-1)).toMatchObject({ query: 'katsu' }));
		vi.stubGlobal('scrollY', 640);
		const left = leave(first);
		vi.unstubAllGlobals();

		// Back with the server gone quiet, then a new search once it answers.
		let down = true;
		const flaky = standIn({
			list_tags: { tags: [] },
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: () =>
				down
					? { refuse: 'busy' as const, message: 'the server is away' }
					: { query: 'miso', closest: false, recipes: [entry()] },
		});
		goBack(flaky, left);
		await screen.findByText(/shelf could not be read/);

		down = false;
		await fireEvent.input(screen.getByRole('searchbox'), { target: { value: 'miso' } });
		expect(await screen.findByText('Miso Soup')).toBeInTheDocument();
		// The reader is somewhere else now; jumping them to 640 would be a lurch.
		expect(scrolled).not.toHaveBeenCalled();
	});

	it('starts a whole shelf from something kept that is not a shelf', async () => {
		const kamosu = answering();
		// Written by an older Kamosu into this tab's storage, say.
		goBack(kamosu, { query: 'katsu', kitchen: 'k_marc' });

		expect(await screen.findByText('Miso Soup')).toBeInTheDocument();
		expect(screen.getByRole('searchbox')).toHaveValue('');
		expect(searches(kamosu).at(-1)).toMatchObject({ query: null, kitchen_id: null, tag_id: null });
	});

	it('starts a fresh shelf when Recipes is arrived at rather than gone back to', async () => {
		const kamosu = answering();
		const first = render(RouteTestHarness, { props: { client: kamosu.client } });

		await fireEvent.input(await screen.findByRole('searchbox'), { target: { value: 'katsu' } });
		await fireEvent.click(await screen.findByRole('button', { name: 'Chez Marc' }));
		await vi.waitFor(() => expect(searches(kamosu).at(-1)).toMatchObject({ query: 'katsu' }));

		// The tab bar is a new visit: SvelteKit restores nothing into it.
		leave(first);
		render(RouteTestHarness, { props: { client: kamosu.client } });

		expect(await screen.findByText('Miso Soup')).toBeInTheDocument();
		expect(screen.getByRole('searchbox')).toHaveValue('');
		expect(screen.getByRole('button', { name: 'All' })).toHaveAttribute('aria-pressed', 'true');
		expect(searches(kamosu).at(-1)).toMatchObject({
			query: null,
			kitchen_id: null,
			mine: false,
			tag_id: null,
		});
	});
});
