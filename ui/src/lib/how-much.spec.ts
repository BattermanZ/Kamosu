/**
 * The choices a cook or a reader is offered for how much of a recipe they
 * mean (#109). Arithmetic on the Yield only — the amounts that follow are the
 * Core's, and nothing here could produce one.
 */
import { describe, expect, it } from 'vitest';
import { caughtUp, choices, fromSearch, said, stepped, timesOf, toSearch } from './how-much';

const twelve = { amount: '12', noun: 'servings' };

describe('how much', () => {
	it('counts the quick choices in the recipe’s own noun', () => {
		expect(choices(twelve).map((choice) => [choice.times, choice.wanted])).toEqual([
			['×½', { amount: '6', noun: 'servings' }],
			[null, null],
			['×2', { amount: '24', noun: 'servings' }],
			['×3', { amount: '36', noun: 'servings' }],
		]);
	});

	it('falls back to a multiplier where counting would land between two servings', () => {
		expect(timesOf({ amount: '3', noun: 'servings' }, 0.5)).toEqual({ amount: '½', noun: '' });
		// A Yield like `a dozen` cannot be counted from at all.
		expect(timesOf({ amount: 'a dozen', noun: 'cookies' }, 2)).toEqual({ amount: '2', noun: '' });
	});

	it('offers a recipe that never said what it makes only multipliers', () => {
		expect(choices(null).map((choice) => choice.wanted)).toEqual([
			{ amount: '½', noun: '' },
			null,
			{ amount: '2', noun: '' },
			{ amount: '3', noun: '' },
		]);
		expect(stepped(null, null, 1)).toBeUndefined();
	});

	it('steps one serving at a time, back to as written, and never below one', () => {
		expect(stepped(twelve, null, 1)).toEqual({ amount: '13', noun: 'servings' });
		expect(stepped(twelve, { amount: '13', noun: 'servings' }, -1)).toBeNull();
		expect(stepped({ amount: '1', noun: 'servings' }, null, -1)).toBeUndefined();
		// From half of three servings — one and a half — down is one, up is two.
		const three = { amount: '3', noun: 'servings' };
		const half = { amount: '½', noun: '' };
		expect(stepped(three, half, -1)).toEqual({ amount: '1', noun: 'servings' });
		expect(stepped(three, half, 1)).toEqual({ amount: '2', noun: 'servings' });
	});

	it('says a choice the way a reader would', () => {
		expect(said({ amount: '24', noun: 'servings' })).toBe('24 servings');
		expect(said({ amount: '2', noun: '' })).toBe('×2');
		expect(said(null)).toBeNull();
	});

	it('knows an answer kept from before a change from one scaled to it', () => {
		const eight = { amount: '8', noun: 'servings' };
		const four = { amount: '4', noun: 'servings' };
		expect(caughtUp(eight, eight, four)).toBe(true);
		// Chosen with no network: the phone still has the recipe as written.
		expect(caughtUp(null, eight, four)).toBe(false);
		// A choice the Core would not scale by comes back as written, and is level.
		expect(caughtUp(null, { amount: '2', noun: 'loaves' }, four)).toBe(true);
	});

	it('carries a choice from the recipe page to the cooking through the address', () => {
		const eight = { amount: '8', noun: 'servings' };
		expect(fromSearch(new URLSearchParams(toSearch(eight)))).toEqual(eight);
		const twice = { amount: '2', noun: '' };
		expect(fromSearch(new URLSearchParams(toSearch(twice)))).toEqual(twice);
		expect(toSearch(null)).toBe('');
		expect(fromSearch(new URLSearchParams(''))).toBeUndefined();
	});
});
