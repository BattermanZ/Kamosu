/**
 * Every date in the Language the interface is set to (#224). The audit of
 * 9 October 2026 found some screens writing dates in the browser's locale
 * and some in the app's, five shapes on one visit; this pins the helper that
 * every screen now calls, in the three Languages Kamosu speaks.
 */

import { afterEach, describe, expect, it } from 'vitest';
import { setLocale } from '$lib/paraglide/runtime';
import { aDate, aDay, aMoment, aMonth } from './dates';

const noon = '2026-09-25T12:05:00Z';

// The Language is set the way the app sets it, kept in localStorage; back to
// English after each, so no test reads another's choice.
afterEach(() => setLocale('en', { reload: false }));

describe("a date, in the app's own Language", () => {
	it('writes English day first, never the American month first', () => {
		expect(aDate(noon)).toBe('25 Sept 2026');
		expect(aDay(noon)).toBe('25 September');
		expect(aMonth(noon)).toBe('September 2026');
	});

	it('writes French in French, whatever the browser speaks', () => {
		setLocale('fr', { reload: false });
		expect(aDate(noon)).toBe('25 sept. 2026');
		expect(aDay(noon)).toBe('25 septembre');
	});

	it('writes Spanish in Spanish', () => {
		setLocale('es', { reload: false });
		expect(aDay(noon)).toBe('25 de septiembre');
	});

	it('writes a moment with its minute and no seconds, on a 24-hour clock', () => {
		const written = aMoment(noon);
		expect(written).toMatch(/^25 Sept 2026, \d\d:05$/);
		expect(written).not.toMatch(/AM|PM/);
	});

	it('takes a Date as readily as the string the Core writes', () => {
		expect(aDate(new Date(noon))).toBe(aDate(noon));
	});
});
