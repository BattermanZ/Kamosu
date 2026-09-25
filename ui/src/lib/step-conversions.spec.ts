import { describe, expect, it } from 'vitest';
import { stepPieces } from './step-conversions';

describe('stepPieces', () => {
	it('cuts the text straight after each written amount, and changes none of it', () => {
		const text = 'Mix 3 oz. Parmesan, finely grated (½ cup), 1 cup panko.';
		const pieces = stepPieces(text, [
			{ written: '3 oz.', measured: 'about 85 g' },
			{ written: '½ cup', measured: 'about 120 ml' },
			{ written: '1 cup', measured: 'about 240 ml' },
		]);
		expect(pieces).toEqual([
			{ text: 'Mix ', written: '3 oz.', measured: 'about 85 g' },
			{ text: ' Parmesan, finely grated (', written: '½ cup', measured: 'about 120 ml' },
			{ text: '), ', written: '1 cup', measured: 'about 240 ml' },
			{ text: ' panko.', written: null, measured: null },
		]);
		expect(pieces.map((piece) => piece.text + (piece.written ?? '')).join('')).toBe(text);
	});

	it('finds the same amount written twice in turn', () => {
		const text = 'Scatter 2 Tbsp. flour. Later, scatter 2 Tbsp. flour again.';
		const pieces = stepPieces(text, [
			{ written: '2 Tbsp.', measured: 'about 16 g' },
			{ written: '2 Tbsp.', measured: 'about 16 g' },
		]);
		expect(pieces.map((piece) => piece.text)).toEqual([
			'Scatter ',
			' flour. Later, scatter ',
			' flour again.',
		]);
	});

	it('leaves out an amount the text no longer has', () => {
		expect(
			stepPieces('Place 2 lb. chicken.', [{ written: '1 lb.', measured: 'about 455 g' }]),
		).toEqual([{ text: 'Place 2 lb. chicken.', written: null, measured: null }]);
	});

	it('draws nothing beside a Step from an answer kept before the shape changed', () => {
		// Before #150 a Step's slot was the oven alone, as a string.
		const kept = 'about 220 °C' as unknown as Parameters<typeof stepPieces>[1];
		expect(stepPieces('Preheat to 425°.', kept)).toEqual([
			{ text: 'Preheat to 425°.', written: null, measured: null },
		]);
		// Anything else that is not a list is refused too, rather than thrown
		// at the Step: found live, a screen newer than its answer crashed here.
		const odd = { temperature: 'about 220 °C' } as unknown as Parameters<typeof stepPieces>[1];
		expect(stepPieces('Preheat to 425°.', odd)).toEqual([
			{ text: 'Preheat to 425°.', written: null, measured: null },
		]);
	});
});
