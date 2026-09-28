/**
 * Which of a recipe's versions is yours (#136): the one every other version is
 * marked against, on the recipe page and in the library a phone keeps offline.
 */
import { describe, expect, it } from 'vitest';
import { yoursAmong } from './cookbook';

const version = (branch_id: string, name: string | null, mine = true, arrived = false) => ({
	branch_id,
	name,
	mine,
	arrived,
});

describe('yoursAmong', () => {
	it('is your own unnamed version where you have one', () => {
		const versions = [version('spicy', 'Spicy'), version('main', null)];
		expect(yoursAmong(versions)?.branch_id).toBe('main');
	});

	it('is the one you have kept longest once yours has a name', () => {
		const versions = [version('classic', 'Classic'), version('spicy', 'Spicy')];
		expect(yoursAmong(versions)?.branch_id).toBe('classic');
	});

	it('is never somebody else’s, nor one sent to you', () => {
		const versions = [
			version('marc', null, false),
			version('sent', null, true, true),
			version('spicy', 'Spicy'),
		];
		expect(yoursAmong(versions)?.branch_id).toBe('spicy');
		expect(yoursAmong(versions.slice(0, 2))).toBeUndefined();
	});
});
