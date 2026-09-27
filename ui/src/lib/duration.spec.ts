/**
 * A recipe's time, typed as hours and minutes and stored as minutes (#175).
 * Kamosu keeps `prep_time_minutes` and `cook_time_minutes` as they always were;
 * only the screen splits and joins them.
 */
import { describe, expect, it } from 'vitest';
import { figureOf, joined, split, timeText } from './duration';

describe('a time typed as hours and minutes', () => {
	it('splits what is stored into the two boxes, leaving an empty half empty', () => {
		expect(split(null)).toEqual({ hours: '', minutes: '' });
		expect(split(15)).toEqual({ hours: '', minutes: '15' });
		expect(split(90)).toEqual({ hours: '1', minutes: '30' });
		expect(split(540)).toEqual({ hours: '9', minutes: '' });
		expect(split(0)).toEqual({ hours: '', minutes: '0' });
	});

	it('joins the two boxes back into minutes, so nobody works out 9 hours by hand', () => {
		expect(joined('9', '')).toBe(540);
		expect(joined('1', '30')).toBe(90);
		expect(joined('', '45')).toBe(45);
		expect(joined(' 2 ', ' 5 ')).toBe(125);
		// Ninety minutes typed as minutes is still ninety minutes.
		expect(joined('', '90')).toBe(90);
	});

	it('reads two empty boxes as no time at all', () => {
		expect(joined('', '')).toBeNull();
		expect(joined('  ', '')).toBeNull();
	});

	it('refuses anything that is not a whole number, rather than dropping it', () => {
		expect(joined('1.5', '')).toBe('wrong');
		expect(joined('', 'ten')).toBe('wrong');
		expect(joined('-1', '')).toBe('wrong');
		expect(joined('2h', '')).toBe('wrong');
	});
});

describe('a time on the reading page', () => {
	it('puts the unit in the figure: 15 min, 1 h 30, 9 h', () => {
		expect(figureOf(15)).toEqual([{ value: '15', unit: 'min' }]);
		expect(figureOf(90)).toEqual([
			{ value: '1', unit: 'h' },
			{ value: '30', unit: null },
		]);
		expect(figureOf(540)).toEqual([{ value: '9', unit: 'h' }]);
	});

	it('writes the minutes after an hour with two digits, as a Sheet does', () => {
		expect(figureOf(65)).toEqual([
			{ value: '1', unit: 'h' },
			{ value: '05', unit: null },
		]);
	});

	it('says a time inside a sentence the same way', () => {
		expect(timeText(15)).toBe('15 min');
		expect(timeText(90)).toBe('1 h 30');
		expect(timeText(540)).toBe('9 h');
	});
});
