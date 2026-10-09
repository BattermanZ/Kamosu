/**
 * Every date Kamosu writes, in the Language the interface is set to (#224).
 *
 * The audit of 9 October 2026 found five ways of writing a date on one visit:
 * some calls passed the app's locale and the rest took the browser's, so a
 * French interface could show an English date, and a Session's time carried
 * seconds. Four styles, named for what they are for, and nothing else calls
 * `toLocaleDateString` directly.
 */

import { getLocale } from '$lib/paraglide/runtime';

/**
 * Kamosu's English is the day-first kind, `25 Sept 2026`. Intl reads a bare
 * `en` as American and writes `Sep 25, 2026`, so English is asked for as
 * British; French and Spanish are day-first already.
 */
const locale = (): string => (getLocale() === 'en' ? 'en-GB' : getLocale());

const at = (when: string | Date): Date => (when instanceof Date ? when : new Date(when));

/** A day in the year, the way the diary names a cooking: `20 August`. */
export const aDay = (when: string | Date): string =>
	at(when).toLocaleDateString(locale(), { day: 'numeric', month: 'long' });

/** A date: `25 Sept 2026`. */
export const aDate = (when: string | Date): string =>
	at(when).toLocaleDateString(locale(), { day: 'numeric', month: 'short', year: 'numeric' });

/** A date with its minute, as an arrival or a Session's last use is: `7 Oct 2026, 01:58`. */
export const aMoment = (when: string | Date): string =>
	at(when).toLocaleString(locale(), {
		day: 'numeric',
		month: 'short',
		year: 'numeric',
		hour: '2-digit',
		minute: '2-digit',
		hourCycle: 'h23',
	});

/** A month, as the diary heads one: `September 2026`. */
export const aMonth = (when: string | Date): string =>
	at(when).toLocaleDateString(locale(), { month: 'long', year: 'numeric' });
