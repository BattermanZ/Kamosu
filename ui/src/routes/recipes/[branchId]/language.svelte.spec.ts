/**
 * A recipe's Language, and reaching its Translations (#106, ADR 0006).
 *
 * **The fixture these exist for is `language_offer` set to a real Language.**
 * Before #106 the field appeared seven times in the whole frontend and was
 * `null` every time, which is exactly why nothing ever went red while the
 * interface threw the offer away. A test that sets it to null tests the silent
 * case and calls it coverage.
 *
 * What these guard above everything:
 *
 *   · an ordinary recipe says NOTHING about its Language. That is Aurélien's
 *     choice of 22 September 2026 and the reason Option 1 was chosen over a
 *     Languages section drawn on every recipe, so an empty state appearing
 *     here later is a regression and not a polish;
 *   · the offer is an OFFER. Nothing is ever applied without a tap, which is
 *     asserted rather than assumed — the stand-in records every ask a screen
 *     makes, so "never called" is a thing a test can prove;
 *   · **unknown is silent**, permanently. No prompt, no badge, no nag;
 *   · how far behind a Translation has fallen is said in words, and the
 *     unanswerable case is said as unanswerable rather than as zero.
 */

import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import type { GetRecipeOutput } from '$lib/api/catalogue';
import type { BranchLanguage, WrittenLanguage } from '$lib/language';
import LanguageTestHarness from './LanguageTestHarness.svelte';

type Translation = GetRecipeOutput['translation'];

/** The English recipe a French one renders, as `get_thread` lists it. */
const THE_ENGLISH = { branch_id: 'b_en', language: 'en' };

function draw(
	props: {
		language: string;
		translation?: Translation;
		others?: { branch_id: string; language: string }[];
		sheet?: { inAFamily: boolean; taken: string[] };
		offer?: BranchLanguage;
		said?: (landed: { branch_id: string; language: string }) => void;
		translate?: (language: WrittenLanguage) => void;
	},
	answers: Answers = {},
) {
	const kamosu = standIn(answers);
	render(LanguageTestHarness, { props: { client: kamosu.client, ...props } });
	return kamosu;
}

describe('what a recipe says about its Language', () => {
	it('says nothing at all on an ordinary recipe with no Translation', () => {
		// The whole of why Option 1 was chosen. 258 of the dev library's 295
		// recipes are English, read by an English reader: a line telling them
		// so is a line telling them nothing.
		const { container } = render(LanguageTestHarness, {
			props: { client: standIn({}).client, language: 'en' },
		});

		expect(container.textContent?.trim()).toBe('');
	});

	it('draws nothing whatever on a recipe that is honestly more than one Language', () => {
		// ADR 0006: unknown is a permanent, unremarkable state. No prompt, no
		// badge, no nag — which is the entire thing unknown buys, and the thing
		// a screen is most likely to take back by accident.
		const { container } = render(LanguageTestHarness, {
			props: {
				client: standIn({}).client,
				language: 'unknown',
				others: [THE_ENGLISH, { branch_id: 'b_fr', language: 'fr' }],
			},
		});

		expect(container.textContent?.trim()).toBe('');
	});

	it('names the other Languages of its Lineage, each a way into that recipe', () => {
		draw({ language: 'en', others: [{ branch_id: 'b_fr', language: 'fr' }] });

		const link = screen.getByRole('link', { name: 'Also in French.' });
		// A Translation is an ordinary Branch, so walking to it is walking to a
		// recipe — never a switch on this page, and never a screen of its own.
		expect(link).toHaveAttribute('href', '/recipes/b_fr');
	});

	it('does not call another Kitchen’s copy of these words another Language', () => {
		// A second Branch in the SAME Language is a Divergence, which the
		// Threshold above already explains. Calling it a Language would be a
		// screen inventing a fact.
		const { container } = render(LanguageTestHarness, {
			props: {
				client: standIn({}).client,
				language: 'en',
				others: [{ branch_id: 'b_theirs', language: 'en' }],
			},
		});

		expect(container.textContent?.trim()).toBe('');
	});

	it('says what a Translation renders, as a link to the recipe it renders', () => {
		draw({
			language: 'fr',
			translation: { translates_version_id: 'v_3', source_branch_id: 'b_en', versions_behind: 0 },
			others: [THE_ENGLISH],
		});

		const link = screen.getByRole('link', { name: 'Translated from the text in English.' });
		expect(link).toHaveAttribute('href', '/recipes/b_en');

		// Up to date says nothing. Silence is what "nothing to do" looks like,
		// and the source must not also get an "Also in" line of its own: one
		// recipe said twice reads as two recipes.
		expect(screen.queryByText(/has been changed/)).not.toBeInTheDocument();
		expect(screen.queryByText('Also in English.')).not.toBeInTheDocument();
	});

	it('says how far behind a Translation has fallen in words, not in a number', () => {
		// `versions_behind` is exact, and "two versions behind" means nothing to
		// a cook who has not read the source.
		draw({
			language: 'fr',
			translation: { translates_version_id: 'v_1', source_branch_id: 'b_en', versions_behind: 2 },
			others: [THE_ENGLISH],
		});

		expect(
			screen.getByText('The English has changed 2 times since this was translated.'),
		).toBeInTheDocument();
	});

	it('counts one change as once rather than as 1 times', () => {
		draw({
			language: 'fr',
			translation: { translates_version_id: 'v_2', source_branch_id: 'b_en', versions_behind: 1 },
			others: [THE_ENGLISH],
		});

		expect(
			screen.getByText('The English has changed once since this was translated.'),
		).toBeInTheDocument();
	});

	it('does not claim a source is off this Kamosu merely because it is off this shelf', () => {
		// `get_thread` lists only the Branches in Kitchens this reader cooks in,
		// so a source held by a Kitchen they left is absent from `others` while
		// `source_branch_id` is perfectly non-null. Telling that as "not on this
		// Kamosu" is a screen inventing a fact — and it would throw away the
		// exact count the Core DID answer.
		draw({
			language: 'fr',
			translation: {
				translates_version_id: 'v_4',
				source_branch_id: 'b_elsewhere',
				versions_behind: 3,
			},
			others: [],
		});

		expect(
			screen.getByText("Translated from a recipe in this family that isn't on your shelf."),
		).toBeInTheDocument();
		expect(screen.queryByText(/not on this Kamosu/)).not.toBeInTheDocument();
		// The count survives, unnamed — this page never learnt the source's
		// Language, so the sentence does not pretend to know it.
		expect(
			screen.getByText('It has been changed 3 times since this was translated.'),
		).toBeInTheDocument();
	});

	it('says an absent source is unanswerable rather than claiming it is up to date', () => {
		// A Translation may arrive on its own in a Bundle. The Core answers
		// `versions_behind: null` precisely so a screen cannot say zero, and
		// this is the assertion that stops one saying it anyway.
		draw({
			language: 'fr',
			translation: {
				translates_version_id: 'v_9',
				source_branch_id: null,
				versions_behind: null,
			},
		});

		expect(
			screen.getByText(
				"Translated from a recipe not on this Kamosu, so its changes can't be followed.",
			),
		).toBeInTheDocument();
		expect(screen.queryByText(/has been changed/)).not.toBeInTheDocument();
	});
});

describe('the Language offer, after a save', () => {
	it('puts the offer to the cook, and applies nothing on its own', async () => {
		// The fixture #106 exists for: a save that detected French on a recipe
		// filed as English.
		const kamosu = draw({ language: 'en', offer: 'fr' });

		expect(
			await screen.findByText('This recipe is filed as English, but it reads as French.'),
		).toBeInTheDocument();

		// Nothing has been said. An offer that acts on its own is a change, and
		// ADR 0006 is explicit that this is never a change.
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('set_recipe_language');
	});

	it('says that accepting mints a Version, before it is accepted', async () => {
		// What separates this from tagging (#104) and relating (#105), both of
		// which appear nowhere in the Thread.
		draw({ language: 'en', offer: 'fr' });

		expect(
			await screen.findByText('Saying so makes a new Version, the way an edit does.'),
		).toBeInTheDocument();
	});

	it('accepting calls set_recipe_language with the Language that was offered', async () => {
		const said = vi.fn();
		const kamosu = draw(
			{ language: 'en', offer: 'fr', said },
			{ set_recipe_language: { branch_id: 'b_1', language: 'fr', sequence: 4 } },
		);

		await fireEvent.click(await screen.findByRole('button', { name: "Say it's French" }));

		expect(kamosu.calls.find((call) => call.operation === 'set_recipe_language')?.input).toEqual({
			branch_id: 'b_1',
			language: 'fr',
		});
		expect(said).toHaveBeenCalledWith({ branch_id: 'b_1', language: 'fr' });
		// And the question is replaced by what happened, rather than sitting
		// there still asking.
		expect(
			await screen.findByText('Said. This recipe is filed as French now.'),
		).toBeInTheDocument();
	});

	it('declining changes nothing and does not ask again for that save', async () => {
		const kamosu = draw({ language: 'en', offer: 'fr' });

		await fireEvent.click(await screen.findByRole('button', { name: 'Leave it' }));

		expect(kamosu.calls.map((call) => call.operation)).not.toContain('set_recipe_language');
		expect(
			screen.queryByText('This recipe is filed as English, but it reads as French.'),
		).not.toBeInTheDocument();
	});
});

describe('saying what Language a recipe is in, by hand', () => {
	const OPEN = { inAFamily: false, taken: ['en'] };

	it('offers every Language, marks the one this recipe carries, and says it makes a Version', async () => {
		draw({ language: 'en', sheet: OPEN });

		expect(
			await screen.findByText("The recipe's language. Changing it adds a Version to the history."),
		).toBeInTheDocument();
		// The one it already is cannot be tapped: saying a recipe is in the
		// Language it is already in is not an edit, and the Core answers so.
		expect(screen.getByRole('button', { name: /^English/ })).toBeDisabled();
		expect(screen.getByRole('button', { name: 'French' })).toBeEnabled();
	});

	it('sets the Language by hand', async () => {
		const said = vi.fn();
		const kamosu = draw(
			{ language: 'en', sheet: OPEN, said },
			{ set_recipe_language: { branch_id: 'b_1', language: 'fr', sequence: 2 } },
		);

		await fireEvent.click(await screen.findByRole('button', { name: 'French' }));

		expect(kamosu.calls.find((call) => call.operation === 'set_recipe_language')?.input).toEqual({
			branch_id: 'b_1',
			language: 'fr',
		});
		expect(said).toHaveBeenCalledWith({ branch_id: 'b_1', language: 'fr' });
	});

	it('offers unknown, and explains what it is for where it is chosen', async () => {
		const kamosu = draw(
			{ language: 'en', sheet: OPEN },
			{ set_recipe_language: { branch_id: 'b_1', language: 'unknown', sequence: 3 } },
		);

		expect(
			await screen.findByText(
				'For a recipe written in two languages. Kamosu stops guessing, and readers see it marked.',
			),
		).toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: /More than one language/ }));

		expect(kamosu.calls.find((call) => call.operation === 'set_recipe_language')?.input).toEqual({
			branch_id: 'b_1',
			language: 'unknown',
		});
	});

	it('refuses unknown in place, with the reason, on a recipe that has a Translation', async () => {
		// The Core refuses this outright. Offering it and then reporting a
		// refusal would make the cook discover the rule by tripping over it.
		draw({ language: 'en', sheet: { inAFamily: true, taken: ['en', 'fr'] } });

		const row = await screen.findByRole('button', { name: /More than one language/ });
		expect(row).toBeDisabled();
		expect(
			screen.getByText(
				"Not for this recipe. Its family has Translations, and a recipe in two languages can't be part of one.",
			),
		).toBeInTheDocument();
	});

	it('offers to translate into the Languages this recipe does not already exist in', async () => {
		const translate = vi.fn();
		draw({ language: 'en', sheet: { inAFamily: true, taken: ['en', 'fr'] }, translate });

		// French is taken by the Translation that already exists, so it is not
		// offered a second time; Spanish is.
		expect(
			await screen.findByText(
				'A Translation is a recipe of its own, in the same family. Cooking it counts as cooking this dish.',
			),
		).toBeInTheDocument();

		// French is already one of this Lineage's Languages, so it is not
		// offered as somewhere to translate to a second time. Spanish is the
		// only row, and it is named apart from the French row above it.
		const intoRows = screen
			.getAllByRole('button')
			.map((button) => button.textContent?.trim() ?? '')
			.filter((label) => label.startsWith('Into '));
		expect(intoRows).toEqual(['Into Spanish']);

		await fireEvent.click(screen.getByRole('button', { name: 'Into Spanish' }));
		expect(translate).toHaveBeenCalledWith('es');
	});

	it('starting a translation writes nothing on its own', async () => {
		// `start_translation` takes a whole recipe, so this sheet cannot make
		// one — it hands the Language up and the writing screen does the rest.
		const kamosu = draw({ language: 'en', sheet: OPEN, translate: () => {} });

		await fireEvent.click(await screen.findByRole('button', { name: 'Into Spanish' }));

		expect(kamosu.calls.map((call) => call.operation)).not.toContain('start_translation');
	});
});
