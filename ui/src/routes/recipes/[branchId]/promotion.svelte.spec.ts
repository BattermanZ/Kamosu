/**
 * Promotion, offered on the recipe (#58, ADR 0005).
 *
 * What these guard, above everything: **a recipe nobody cooked differently
 * carries nothing here**, and **the one irreversible act is not one tap**.
 * Promotion is where an Attempt's freedoms end and the recipe's append-only
 * rules begin, so a thumb brushing past must not spend it.
 */

import { describe, expect, it } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import type { GetRecipeOutput, GetThreadOutput } from '$lib/api/catalogue';
import PromotionTestHarness from './PromotionTestHarness.svelte';

type Attempt = GetThreadOutput['attempts'][number];

const line = (kind: string, text: string, index: number) => ({ kind, text, index });

/**
 * The Core's own reading of an As Cooked against the Version cooked — the same
 * Pairing two Branches are laid over each other with (ADR 0019). The screen
 * displays this and matches nothing itself.
 */
const AGAINST = {
	ingredients: [
		{
			kind: 'ingredient',
			state: 'changed' as const,
			from_branch_point: true,
			mine: line('ingredient', '1 cup panko', 0),
			theirs: line('ingredient', '2 cups panko', 0),
		},
		{
			kind: 'ingredient',
			state: 'same' as const,
			from_branch_point: true,
			mine: line('ingredient', 'a pinch of salt', 1),
			theirs: line('ingredient', 'a pinch of salt', 1),
		},
		{
			kind: 'ingredient',
			state: 'only-mine' as const,
			from_branch_point: true,
			mine: line('ingredient', '800 ml water', 2),
			theirs: null,
		},
		{
			kind: 'ingredient',
			state: 'only-theirs' as const,
			from_branch_point: false,
			mine: null,
			theirs: line('ingredient', '1 bay leaf', 2),
		},
	],
	steps: [
		{
			kind: 'step',
			state: 'same' as const,
			from_branch_point: true,
			mine: line('step', 'Coat the chicken in panko.', 0),
			theirs: line('step', 'Coat the chicken in panko.', 0),
		},
	],
};

const CONTENT = {
	title: 'Chicken Katsu Curry',
	yield: null,
	prep_time_minutes: null,
	cook_time_minutes: null,
	note: null,
	main_photo: null,
	nutrition: null,
	source: null,
	ingredients: [{ kind: 'ingredient' as const, text: '1 cup panko' }],
	steps: [{ kind: 'step' as const, text: 'Coat the chicken in panko.', photo: null }],
};

const attempt = (over: Partial<Attempt> = {}): Attempt => ({
	id: 'at_1',
	lineage_id: 'l_1',
	person_id: 'p_1',
	version_id: 'v_1',
	current_step_index: 0,
	ticked_ingredients: [],
	cooking_yield: null,
	note: null,
	rating: null,
	finished_at: '2026-09-03T19:00:00Z',
	resumable: false,
	created_at: '2026-09-03T18:00:00Z',
	last_action_at: '2026-09-03T19:00:00Z',
	photographs: [],
	as_cooked: {
		version_id: 'v_cooked',
		content: CONTENT,
		against: AGAINST,
		promotion_declined: false,
	},
	...over,
});

const version = (id: string): GetRecipeOutput['versions'][number] => ({
	sequence: 1,
	version_id: id,
	scaled_to: null,
	parent_version_id: null,
	hand_id: 'h_1',
	name: null,
	change_note: null,
	created_at: '2026-09-01T09:00:00Z',
	translates_version_id: null,
	language: 'en',
	components: [],
	content: CONTENT,
	readings: [null],
	measured: { ingredients: [null], steps: [null] },
	cooking: { steps: [{ uses: [], timer_seconds: null }] },
});

function show(attempts: Attempt[], versions: string[], answers: Answers = {}) {
	const kamosu = standIn({
		promote_as_cooked: saved,
		decline_promotion: attempt(),
		...answers,
	});
	render(PromotionTestHarness, {
		props: { client: kamosu.client, attempts, versions: versions.map(version) },
	});
	return kamosu;
}

/** What `promote_as_cooked` answers: an ordinary saved Version. */
const saved = {
	branch_id: 'b_1',
	version_id: 'v_cooked',
	parent_version_id: 'v_1',
	sequence: 2,
	collapsed: false,
	copied: false,
	language: 'en',
	language_offer: null,
	translates_version_id: null,
};

describe('promotion, on the recipe', () => {
	it('is not drawn at all where every cooking followed the recipe', async () => {
		show([attempt({ as_cooked: null })], ['v_1']);
		// The common case, and it costs the recipe page nothing.
		expect(screen.queryByText(/cooked this differently/i)).not.toBeInTheDocument();
	});

	it('is not drawn once those words are already a Version of this recipe', async () => {
		// No flag says "promoted": an As Cooked is named by a fingerprint of its
		// own content, so once kept it simply IS one of the Branch's Versions.
		show([attempt()], ['v_1', 'v_cooked']);
		expect(screen.queryByText(/cooked this differently/i)).not.toBeInTheDocument();
	});

	it('says a cooking departed from these words, and offers to keep them', async () => {
		show([attempt()], ['v_1']);
		expect(await screen.findByText(/you cooked this differently/i)).toBeInTheDocument();
		expect(
			await screen.findByRole('button', { name: /keep it as a new version/i }),
		).toBeInTheDocument();
	});

	it('never mints a Version on one tap', async () => {
		const kamosu = show([attempt()], ['v_1']);
		await fireEvent.click(await screen.findByRole('button', { name: /keep it as a new version/i }));
		// Opening what you would keep is not keeping it. Promotion is where the
		// recipe's append-only rules take over, and a wet thumb must not spend it.
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('promote_as_cooked');

		await fireEvent.click(await screen.findByRole('button', { name: /save as a new version/i }));
		const promoted = kamosu.calls.find((call) => call.operation === 'promote_as_cooked');
		expect(promoted?.input).toEqual({ attempt_id: 'at_1', branch_id: 'b_1' });
	});

	it('shows the Core’s reading of what changed, and matches no line itself', async () => {
		show([attempt()], ['v_1']);
		await fireEvent.click(await screen.findByRole('button', { name: /keep it as a new version/i }));

		// Rewritten, with the recipe's own line under it.
		expect(await screen.findByText('2 cups panko')).toBeInTheDocument();
		expect(await screen.findByText('1 cup panko')).toBeInTheDocument();
		// Dropped, and added — neither expressible as "this line, this amount",
		// which is why an As Cooked holds a whole recipe.
		expect(await screen.findByText('800 ml water')).toBeInTheDocument();
		expect(await screen.findByText('1 bay leaf')).toBeInTheDocument();
		// A line the cook left alone is not part of the question being asked.
		expect(screen.queryByText('a pinch of salt')).not.toBeInTheDocument();
	});

	it('offers to leave it in the diary, and does not ask again', async () => {
		const kamosu = show([attempt()], ['v_1']);
		await fireEvent.click(await screen.findByRole('button', { name: /leave it in the diary/i }));

		// Off the screen at once — a question that lingers while a server thinks
		// reads as one that was not heard.
		expect(screen.queryByText(/you cooked this differently/i)).not.toBeInTheDocument();
		const declined = kamosu.calls.find((call) => call.operation === 'decline_promotion');
		expect(declined?.input).toEqual({ attempt_id: 'at_1', declined: true });
		// And what was cooked is untouched: declining answers the offer, not the
		// cooking record.
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('set_as_cooked');
	});

	it('is not drawn again once the cook has already said no', async () => {
		show(
			[
				attempt({
					as_cooked: {
						version_id: 'v_cooked',
						content: CONTENT,
						against: AGAINST,
						promotion_declined: true,
					},
				}),
			],
			['v_1'],
		);
		// A question already answered, asked twice, is a nag.
		expect(screen.queryByText(/you cooked this differently/i)).not.toBeInTheDocument();
	});

	it('says the recipe has moved on before replacing what it says now', async () => {
		// The Attempt is pinned to v_1; the Branch now stands at v_2. Promotion
		// appends onto the head and never merges (ADR 0004), so the cook is told.
		show([attempt()], ['v_1', 'v_2']);
		await fireEvent.click(await screen.findByRole('button', { name: /keep it as a new version/i }));
		expect(await screen.findByText(/recipe has changed since you cooked it/i)).toBeInTheDocument();
	});

	it('refuses in words rather than failing when there is no server', async () => {
		// Everything on an Attempt's side of Promotion works offline and this
		// does not (ADR 0013) — so the control says what it is waiting for
		// before it is reached for (#76, option C), and no other answer is
		// offered in its place.
		Object.defineProperty(navigator, 'onLine', { value: false, configurable: true });
		try {
			show([attempt()], ['v_1']);
			expect(
				await screen.findByRole('button', { name: /keeping it waits for the server/i }),
			).toBeDisabled();
			expect(screen.queryByRole('button', { name: /keep it as a new version/i })).toBeNull();
			expect(screen.queryByRole('button', { name: /leave it in the diary/i })).toBeNull();
		} finally {
			Reflect.deleteProperty(navigator, 'onLine');
		}
	});
});
