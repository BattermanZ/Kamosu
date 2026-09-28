/**
 * The Component picker's search (#87, #121).
 *
 * The picker shares the shelf's search-as-you-type helper. The helper has no
 * test of its own on purpose: a test that hands it values cannot see whether
 * the values it tracks are the ones a person types. So this types into the
 * sheet's real field, and checks that a search the phone answered from its
 * cache is asked again when the server says otherwise.
 */

import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import ComponentPickerTestHarness from './ComponentPickerTestHarness.svelte';
import {
	searchEntry as found,
	searchesSent as searches,
	serverAnsweredSearchesOtherwise,
} from '../../../testing/recipes';

function draw(answers: Answers) {
	const kamosu = standIn(answers);
	render(ComponentPickerTestHarness, { props: { client: kamosu.client } });
	return kamosu;
}

describe('the Component picker’s search', () => {
	it('follows what is typed, across the whole library', async () => {
		const kamosu = draw({
			search_recipes: ({ query }) => ({
				query: query ?? null,
				closest: false,
				recipes: query === 'dough' ? [found('dough', 'Pizza Dough')] : [found('naan', 'Naan')],
			}),
		});
		expect(await screen.findByText('Naan')).toBeInTheDocument();

		await fireEvent.input(screen.getByRole('searchbox'), { target: { value: 'dough' } });

		expect(await screen.findByText('Pizza Dough')).toBeInTheDocument();
		expect(screen.queryByText('Naan')).not.toBeInTheDocument();
		expect(searches(kamosu).at(-1)).toEqual({ query: 'dough', kitchen_id: null, mine: false });
	});

	it('asks its current search again when the server answers otherwise than the phone did (#76)', async () => {
		let served = 'phone';
		const kamosu = draw({
			search_recipes: ({ query }) => ({
				query: query ?? null,
				closest: false,
				recipes: [served === 'phone' ? found('naan', 'Naan') : found('dough', 'Pizza Dough')],
			}),
		});
		await fireEvent.input(screen.getByRole('searchbox'), { target: { value: 'bread' } });
		await vi.waitFor(() => expect(searches(kamosu).at(-1)).toMatchObject({ query: 'bread' }));
		expect(await screen.findByText('Naan')).toBeInTheDocument();

		served = 'server';
		serverAnsweredSearchesOtherwise();

		expect(await screen.findByText('Pizza Dough')).toBeInTheDocument();
		expect(searches(kamosu).at(-1)).toMatchObject({ query: 'bread' });
	});
});
