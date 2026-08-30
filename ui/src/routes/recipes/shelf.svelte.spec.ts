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

import { describe, expect, it, vi } from 'vitest';
import { screen, fireEvent } from '@testing-library/svelte';
import Recipes from './+page.svelte';
import { renderScreen } from '../../testing/render';
import type { MeaningSearchStatusOutput } from '$lib/api/catalogue';

// Both offers end by opening the recipe they produced. Where that goes is the
// router's business, not this screen's, so it is stubbed and the assertions
// below are about the Operations that ran before it.
const went = vi.hoisted(() => vi.fn());
vi.mock('$app/navigation', () => ({ goto: went }));

const kitchen = {
	id: 'k_home',
	name: 'Maison',
	nickname: null,
	hand_id: 'h_1',
	is_home: true,
	members: [{ person_id: 'p_1', name: 'Aurélien' }],
};

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
	...over,
});

describe('the recipes screen', () => {
	it('shows the shelf it was given, one card per Lineage', async () => {
		renderScreen(Recipes, {
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

	it('explains why nothing matched, and offers the two things you were about to do', async () => {
		renderScreen(Recipes, {
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: { query: 'osso buco', closest: false, recipes: [] },
		});

		// It names what was searched for — from the answer, not from the field.
		expect(await screen.findByText(/Nothing matched .osso buco./)).toBeInTheDocument();

		// And says what it looked through, and why it matched only words. An
		// empty screen would leave a person unable to tell a missing recipe
		// from a broken search.
		expect(screen.getByText(/looked through every title, ingredient and step/)).toBeInTheDocument();
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
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: { query: 'osso buco', closest: false, recipes: [] },
			create_recipe: {
				branch_id: 'b_new',
				lineage_id: 'l_new',
				kitchen_id: 'k_home',
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
		// it is made in the Home Kitchen and opened, with no screen in between.
		await vi.waitFor(() => {
			const asked = kamosu.calls.find((call) => call.operation === 'create_recipe');
			expect(asked?.input).toMatchObject({ kitchen_id: 'k_home', title: 'osso buco' });
		});
	});

	it('reads the link it offered to import, waiting on the Job it is', async () => {
		const { kamosu } = renderScreen(Recipes, {
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
					kitchen_id: 'k_home',
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
			list_kitchens: { kitchens: [kitchen] },
			meaning_search_status: meaningOff(),
			search_recipes: { query: null, closest: false, recipes: [] },
		});

		expect(await screen.findByText(/The shelf is empty/)).toBeInTheDocument();
		expect(screen.queryByText(/Nothing matched/)).not.toBeInTheDocument();
	});

	it('quotes the line that matched, and says where it came from', async () => {
		renderScreen(Recipes, {
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

	it('marks nothing on a recipe whose Language is unknown', async () => {
		renderScreen(Recipes, {
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
			list_kitchens: {
				kitchens: [kitchen, { ...kitchen, id: 'k_marc', name: 'Chez Marc', is_home: false }],
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
});
