/**
 * The phone adds a Shopping List up exactly as the server does (#77).
 *
 * Every case here was written by `shopping_parity` in `src/shopping.rs`, with
 * the server's own answer beside it. If this fails, the phone's copy of the
 * sum in `shopping.ts` says something the server does not — or the server's
 * sum changed and this copy has not caught up. Either way the fix is here, not
 * in the file: the file is the server speaking.
 */

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import {
	asText,
	fold,
	parseAmount,
	rows,
	wordedVolume,
	yieldScale,
	type Measures,
} from './shopping';

// Read from the repository root: the tests run from `ui/`, and the file is the
// server's, so it lives beside the server's tests rather than under `ui/src`.
const parity = JSON.parse(
	readFileSync(resolve(process.cwd(), '../tests/fixtures/shopping-parity.json'), 'utf8'),
);

describe('the phone adds a Shopping List up as the server does', () => {
	it('folds words the way Kamosu compares them', () => {
		for (const { text, folded } of parity.folds) expect(fold(text), text).toBe(folded);
	});

	it('reads a written amount the way the server reads one', () => {
		for (const { text, value } of parity.amounts) expect(parseAmount(text), text).toBe(value);
	});

	it('scales by a Yield only where the two Yields compare', () => {
		for (const { wanted, written, scale } of parity.yield_scales)
			expect(yieldScale(wanted, written)).toBe(scale);
	});

	it('writes a cup in the singular or plural as each Language does', () => {
		for (const { millilitres, language, worded } of parity.cup_plurals)
			expect(wordedVolume(millilitres, 'us', language), `${millilitres} ml, ${language}`).toBe(
				worded,
			);
	});

	it(`works out the same rows in all ${parity.cases.length} cases`, () => {
		for (const each of parity.cases) {
			expect(
				rows(each.chosen, each.loose, each.measures as Measures, each.language),
				each.name,
			).toEqual(each.rows);
		}
	});

	it('writes the same text to leave with', () => {
		for (const each of parity.texts)
			expect(asText(each.list, each.today, each.language)).toBe(each.text);
	});
});
