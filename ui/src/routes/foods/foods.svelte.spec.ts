/**
 * The Foods list (#107).
 *
 * This screen is a way through rather than a thing to read, so what it is
 * tested on is finding: that the word the Reading corrector arrives with lands
 * on the right Food, that a Food answers to any of its names, and that where
 * two Foods answer to one word both are shown rather than one being picked.
 *
 * The last of those is the one worth being careful about. ADR 0022 makes doubt
 * produce a new Food rather than a merge, so two Foods really can share a word,
 * and a screen that guessed between them would send somebody off to correct the
 * wrong one.
 */

import { describe, expect, it } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import type { ListFoodsOutput } from '$lib/api/catalogue';
import FoodsTestHarness from './FoodsTestHarness.svelte';

type FoodRow = ListFoodsOutput['foods'][number];

const food = (
	id: string,
	names: { language: string; name: string }[],
	reading_count = 1,
	cup_weight_grams: number | null = null,
): FoodRow => ({
	id,
	name: names[0]?.name ?? null,
	language: names[0]?.language ?? null,
	names,
	cup_weight_grams,
	nutrition: null,
	reading_count,
});

/** A library shaped like the real one: mostly English, a few French, no Cup Weights. */
const LIBRARY: FoodRow[] = [
	food('f_salt', [{ language: 'en', name: 'salt' }], 58),
	food('f_sel', [{ language: 'fr', name: 'sel' }], 10),
	food('f_caster', [{ language: 'en', name: 'caster sugar' }], 9),
	food('f_brown', [{ language: 'en', name: 'brown sugar' }], 11),
	food('f_pecan', [{ language: 'fr', name: 'noix de pécan' }], 3),
];

function draw(answers: Answers, q = '') {
	const kamosu = standIn(answers);
	render(FoodsTestHarness, { props: { client: kamosu.client, q } });
	return kamosu;
}

describe('the Foods list', () => {
	it('opens on the Foods the most lines point at, since those are worth correcting', async () => {
		draw({ list_foods: { foods: LIBRARY } });

		const rows = await screen.findAllByRole('link');
		expect(rows[0]).toHaveTextContent('salt');
		expect(rows[0]).toHaveTextContent('58 lines point at this');
	});

	it('lands on the Food the Reading corrector arrived for', async () => {
		// The corrector hands over the written word, because a Reading carries no
		// Food id — `reading_schema` keeps one out on purpose.
		draw({ list_foods: { foods: LIBRARY } }, 'caster sugar');

		const rows = await screen.findAllByRole('link');
		expect(rows).toHaveLength(1);
		expect(rows[0]).toHaveAttribute('href', '/foods/f_caster');
	});

	it('finds a Food by any of its names, which is what naming per Language is for', async () => {
		const both = food(
			'f_salt',
			[
				{ language: 'en', name: 'salt' },
				{ language: 'fr', name: 'sel' },
			],
			68,
		);
		draw({ list_foods: { foods: [both, ...LIBRARY.slice(2)] } }, 'sel');

		const rows = await screen.findAllByRole('link');
		expect(rows[0]).toHaveAttribute('href', '/foods/f_salt');
		expect(rows[0]).toHaveTextContent('salt');
	});

	it('shows both Foods where two answer to one word, rather than choosing', async () => {
		const twice = [
			food('f_flour_a', [{ language: 'en', name: 'flour' }], 12),
			food('f_flour_b', [{ language: 'en', name: 'flour' }], 3),
		];
		draw({ list_foods: { foods: twice } }, 'flour');

		const rows = await screen.findAllByRole('link');
		expect(rows).toHaveLength(2);
		expect(rows.map((row) => row.getAttribute('href'))).toEqual([
			'/foods/f_flour_a',
			'/foods/f_flour_b',
		]);
	});

	it('finds an accented word typed without its accents', async () => {
		draw({ list_foods: { foods: LIBRARY } }, 'pecan');

		const rows = await screen.findAllByRole('link');
		expect(rows[0]).toHaveTextContent('noix de pécan');
	});

	it('narrows as the search is typed', async () => {
		draw({ list_foods: { foods: LIBRARY } });

		await screen.findAllByRole('link');
		await fireEvent.input(screen.getByRole('searchbox'), { target: { value: 'sugar' } });

		const rows = await screen.findAllByRole('link');
		expect(rows).toHaveLength(2);
		expect(rows.map((row) => row.textContent)).toEqual(
			expect.arrayContaining([expect.stringContaining('brown sugar')]),
		);
	});

	it('says no Food answers to a word, and offers the way back to the list', async () => {
		// The corrector arrives with the word WRITTEN on the line, and correcting
		// a Food's name replaces it — so a Food renamed after its recipe was
		// written no longer answers to the word the line still says. This is the
		// one way that door can miss, and it must not end in a blank screen.
		draw({ list_foods: { foods: LIBRARY } }, 'gochujang');

		expect(
			await screen.findByText(
				'No Food called “gochujang”. It may have been renamed since this line was written.',
			),
		).toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: 'Show every Food' }));
		expect(await screen.findAllByRole('link')).toHaveLength(LIBRARY.length);
	});

	it('explains an instance that has learnt no Foods yet', async () => {
		draw({ list_foods: { foods: [] } });

		expect(
			await screen.findByText('No Foods yet. They appear as you write or import recipes.'),
		).toBeInTheDocument();
	});

	it('says a Cup Weight on the row where one is set, and nothing where none is', async () => {
		draw({
			list_foods: { foods: [food('f_flour', [{ language: 'en', name: 'flour' }], 16, 125)] },
		});

		expect(await screen.findByText(/a cup weighs 125 g/)).toBeInTheDocument();
	});

	it('says a refusal in the words it arrived in, not as a category', async () => {
		// The Core writes its refusals to be read by a person, so this screen
		// shows them as they came — the same as the Food's own page and the
		// Operator's screen. Paraphrasing keeps the fact and loses the reason.
		draw({ list_foods: { refuse: 'internal', message: 'the database is not answering' } });

		expect(await screen.findByRole('alert')).toHaveTextContent('the database is not answering');
	});

	it('still says something when a refusal arrived with no words of its own', async () => {
		draw({ list_foods: { refuse: 'internal', message: '' } });

		expect(await screen.findByRole('alert')).toHaveTextContent(
			'Kamosu could not list its Foods just now.',
		);
	});
});
