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
import { m } from '$lib/paraglide/messages';
import type { Room } from '$lib/room.svelte';
import { went } from '../../testing/navigation';
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

/**
 * Foods beside the open Food (#199, ADR 0044).
 *
 * On the wide layout the list stays while Foods are opened one after another,
 * which is what makes tidying open, fix, next. The phone keeps the two full
 * pages it had.
 */
describe('on the wide layout', () => {
	function beside(open?: string, room: Room = 'wide') {
		const kamosu = standIn({ list_foods: { foods: LIBRARY } });
		const drawn = render(FoodsTestHarness, { props: { client: kamosu.client, room, open } });
		return { ...kamosu, ...drawn };
	}

	const rows = () => screen.findAllByRole('link');
	const theOpenFood = () => screen.queryByLabelText('the open Food');
	const search = () => screen.getByRole('searchbox');

	/** What a sighted cook reads on a row: the words kept for a screen reader are taken out. */
	function seen(row: HTMLElement): string {
		const copy = row.cloneNode(true) as HTMLElement;
		copy.querySelectorAll('.sr-only').forEach((hidden) => hidden.remove());
		return (copy.textContent ?? '').replace(/\s+/g, ' ').trim();
	}

	it('shows the open Food beside the list, and marks its row', async () => {
		beside('f_sel');

		const list = await rows();
		expect(list).toHaveLength(LIBRARY.length);
		expect(theOpenFood()).toBeInTheDocument();
		expect(screen.getByRole('link', { current: 'page' })).toHaveTextContent('sel');
	});

	it('says to choose a Food before one is open, and draws none', async () => {
		beside();

		await rows();
		expect(screen.getByText(m.foods_choose())).toBeInTheDocument();
		expect(theOpenFood()).toBeNull();
	});

	it('keeps the list, what was typed in it and the reading of it while another Food is opened', async () => {
		const drawn = beside('f_caster');
		await rows();
		await fireEvent.input(search(), { target: { value: 'sugar' } });
		const list = screen.getByRole('list');

		await drawn.rerender({ open: 'f_brown' });

		expect(search()).toHaveValue('sugar');
		expect(screen.getByRole('list')).toBe(list);
		expect(drawn.calls.filter((call) => call.operation === 'list_foods')).toHaveLength(1);
		expect(screen.getByRole('link', { current: 'page' })).toHaveTextContent('brown sugar');
	});

	it('writes a row as the name with the number of lines beside it', async () => {
		beside();

		const [first] = await rows();
		expect(seen(first)).toBe('salt 58');
		// The number alone says nothing aloud, so the sentence is still there to be read out.
		expect(first).toHaveTextContent('58 lines point at this');
	});

	it('leads to the same address the phone opens a Food at', async () => {
		beside();

		const [first] = await rows();
		expect(first).toHaveAttribute('href', '/foods/f_salt');
	});

	it('moves down and up the list with the arrows, opening each Food in place of the last', async () => {
		const drawn = beside('f_brown');
		await rows();

		await fireEvent.keyDown(document.body, { key: 'ArrowDown' });
		// Replaced, so going back leaves the list in one step rather than walking back up it.
		expect(went).toHaveBeenLastCalledWith('/foods/f_sel', {
			replaceState: true,
			keepFocus: true,
			noScroll: true,
		});

		await drawn.rerender({ open: 'f_sel' });
		await fireEvent.keyDown(document.body, { key: 'ArrowUp' });
		expect(went).toHaveBeenLastCalledWith(
			'/foods/f_brown',
			expect.objectContaining({ replaceState: true }),
		);
	});

	it('opens the first Food on an arrow when none is open', async () => {
		beside();
		await rows();

		await fireEvent.keyDown(document.body, { key: 'ArrowDown' });

		expect(went).toHaveBeenLastCalledWith(
			'/foods/f_salt',
			expect.objectContaining({ replaceState: true }),
		);
	});

	it('stops at the ends of the list', async () => {
		beside('f_salt');
		await rows();

		await fireEvent.keyDown(document.body, { key: 'ArrowUp' });

		expect(went).not.toHaveBeenCalled();
	});

	it('walks only the Foods the search left', async () => {
		beside('f_caster');
		await rows();
		await fireEvent.input(search(), { target: { value: 'sugar' } });

		// brown sugar (11) is above caster sugar (9); salt and sel are gone.
		await fireEvent.keyDown(document.body, { key: 'ArrowUp' });

		expect(went).toHaveBeenLastCalledWith('/foods/f_brown', expect.anything());
	});

	it('puts the caret on the row an arrow opened, so Tab goes on into that Food', async () => {
		beside('f_brown');
		const list = await rows();

		await fireEvent.keyDown(document.body, { key: 'ArrowDown' });

		expect(list[2]).toHaveTextContent('sel');
		expect(list[2]).toHaveFocus();
	});

	it('lets Tab reach one row of the list and no more, so the Food is one Tab away', async () => {
		beside('f_sel');

		const list = await rows();
		expect(list.filter((row) => row.tabIndex === 0)).toEqual([
			screen.getByRole('link', { current: 'page' }),
		]);
	});

	it.each([
		['the search box', search],
		["a field of the open Food's", () => theOpenFood()!],
	])('leaves the arrows to %s while the caret is in it', async (_, field) => {
		beside('f_brown');
		await rows();

		const untouched = await fireEvent.keyDown(field(), { key: 'ArrowDown' });

		expect(went).not.toHaveBeenCalled();
		// Not taken over either, so the caret moves in the field as it always does.
		expect(untouched).toBe(true);
	});

	it('leaves an arrow held with another key to the browser', async () => {
		beside('f_brown');
		await rows();

		await fireEvent.keyDown(document.body, { key: 'ArrowDown', altKey: true });

		expect(went).not.toHaveBeenCalled();
	});

	it('changes a row, and what the search answers to, when its Food is corrected beside it', async () => {
		const kamosu = standIn({ list_foods: { foods: LIBRARY } });
		const renamed = food('f_sel', [{ language: 'fr', name: 'sel fin' }], 10);
		render(FoodsTestHarness, {
			props: { client: kamosu.client, room: 'wide', open: 'f_sel', correction: renamed },
		});
		await rows();

		await fireEvent.click(screen.getByRole('button', { name: 'correct it' }));

		expect(screen.getByRole('link', { current: 'page' })).toHaveTextContent('sel fin');
		await fireEvent.input(search(), { target: { value: 'fin' } });
		expect(await rows()).toHaveLength(1);
		// Told, not read again: the list is the one it was.
		expect(kamosu.calls.filter((call) => call.operation === 'list_foods')).toHaveLength(1);
	});

	it('is side by side where the window is roomy too', async () => {
		beside('f_sel', 'roomy');

		await rows();
		expect(theOpenFood()).toBeInTheDocument();
	});
});

describe('on the phone layout', () => {
	function phone(open?: string) {
		const kamosu = standIn({ list_foods: { foods: LIBRARY } });
		const drawn = render(FoodsTestHarness, { props: { client: kamosu.client, open } });
		return { ...kamosu, ...drawn };
	}

	it('draws a Food as a full page, with no list beside it and none read', async () => {
		const drawn = phone('f_sel');

		expect(screen.getByLabelText('the open Food')).toBeInTheDocument();
		expect(screen.queryByRole('searchbox')).toBeNull();
		await new Promise((resolve) => setTimeout(resolve, 0));
		expect(screen.queryAllByRole('link')).toHaveLength(0);
		expect(drawn.calls).toHaveLength(0);
	});

	it('draws the list as a full page, with no sentence about choosing', async () => {
		phone();

		const [first] = await screen.findAllByRole('link');
		expect(first).toHaveTextContent('58 lines point at this');
		expect(first).not.toHaveAttribute('aria-current');
		expect(screen.queryByText(m.foods_choose())).toBeNull();
	});

	it('does nothing on an arrow', async () => {
		phone();
		await screen.findAllByRole('link');

		await fireEvent.keyDown(document.body, { key: 'ArrowDown' });

		expect(went).not.toHaveBeenCalled();
	});
});

describe('as the window changes', () => {
	it('keeps the open Food, and what was typed in it, when a wide window narrows to a phone', async () => {
		const kamosu = standIn({ list_foods: { foods: LIBRARY } });
		const drawn = render(FoodsTestHarness, {
			props: { client: kamosu.client, room: 'wide', open: 'f_sel' },
		});
		await screen.findAllByRole('link');
		const field = screen.getByLabelText('the open Food');

		await drawn.rerender({ room: 'phone' });

		expect(screen.getByLabelText('the open Food')).toBe(field);
		expect(screen.queryByRole('searchbox')).toBeNull();
	});

	it('keeps what was typed in the search when a phone window widens', async () => {
		const kamosu = standIn({ list_foods: { foods: LIBRARY } });
		const drawn = render(FoodsTestHarness, { props: { client: kamosu.client } });
		await screen.findAllByRole('link');
		await fireEvent.input(screen.getByRole('searchbox'), { target: { value: 'sugar' } });

		await drawn.rerender({ room: 'wide' });

		expect(screen.getByRole('searchbox')).toHaveValue('sugar');
	});
});

describe('arriving with a second word', () => {
	it('puts the second word in the box, where the first one was', async () => {
		const kamosu = standIn({ list_foods: { foods: LIBRARY } });
		const drawn = render(FoodsTestHarness, { props: { client: kamosu.client, q: 'sel' } });
		await screen.findAllByRole('link');

		await drawn.rerender({ q: 'salt' });

		expect(screen.getByRole('searchbox')).toHaveValue('salt');
	});
});
