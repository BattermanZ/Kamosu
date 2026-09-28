import { m } from '$lib/paraglide/messages';

/**
 * A recipe's time as a person types and reads it: hours and minutes (#175).
 *
 * What is stored does not change. `prep_time_minutes` and `cook_time_minutes`
 * are whole minutes, as they always were, so no recipe already written moves
 * its id (ADR 0038). Aurélien asked for this on 27 September 2026 while
 * choosing the writing screen's labels: a time can be minutes or hours, and
 * nobody should have to work out that 9 hours is 540 minutes. So the writing
 * screen takes two boxes, and the reading page says `9 h` rather than `540`.
 */

/** A stored time, split into the two boxes the writing screen shows. */
export function split(minutes: number | null): { hours: string; minutes: string } {
	if (minutes === null) return { hours: '', minutes: '' };
	const hours = Math.floor(minutes / 60);
	const rest = minutes % 60;
	return {
		// An empty half stays empty rather than saying 0: `__ h 15 min` reads
		// as fifteen minutes, and `0 h 15 min` reads as a form filled in.
		hours: hours === 0 ? '' : String(hours),
		// Zero minutes on its own is a time somebody typed, so it is kept.
		minutes: rest === 0 && hours > 0 ? '' : String(rest),
	};
}

/** A box holding nothing, a whole number, or something else. */
function whole(value: string): number | null | 'wrong' {
	const typed = value.trim();
	if (typed === '') return null;
	return /^\d+$/.test(typed) ? Number(typed) : 'wrong';
}

/**
 * The two boxes joined back into minutes. Nothing typed in either is no time
 * at all. Anything that is not a whole number is `'wrong'`, so the screen can
 * say so instead of dropping what somebody typed on purpose.
 */
export function joined(hours: string, minutes: string): number | null | 'wrong' {
	const h = whole(hours);
	const m = whole(minutes);
	if (h === 'wrong' || m === 'wrong') return 'wrong';
	if (h === null && m === null) return null;
	return (h ?? 0) * 60 + (m ?? 0);
}

/** One part of a time as the reading page draws it: a number and its unit, if it has one. */
export interface FigurePart {
	value: string;
	unit: 'h' | 'min' | null;
}

/**
 * A time as the reading page draws it, with the unit in the figure: `15 min`,
 * `1 h 30`, `9 h` (#175, reading option 1). The minutes after an hour carry no
 * unit and two digits, the same rule the Sheet's `duration` follows in
 * `src/sheet.rs`, so a printed recipe and the page agree.
 */
export function figureOf(minutes: number): FigurePart[] {
	const hours = Math.floor(minutes / 60);
	const rest = minutes % 60;
	if (hours === 0) return [{ value: String(rest), unit: 'min' }];
	if (rest === 0) return [{ value: String(hours), unit: 'h' }];
	return [
		{ value: String(hours), unit: 'h' },
		{ value: String(rest).padStart(2, '0'), unit: null },
	];
}

/**
 * A time as one line of words, `1 h 30`, where it sits inside a sentence
 * rather than in the strip: the mark that says another Branch's recipe takes
 * a different time (#175), which said a bare `540` before.
 */
export function timeText(minutes: number): string {
	return figureOf(minutes)
		.map((part) => (part.unit ? `${part.value} ${unitWord(part.unit)}` : part.value))
		.join(' ');
}

/** A unit as the Reading Language writes it. */
export function unitWord(unit: 'h' | 'min'): string {
	return unit === 'h' ? m.time_unit_hours() : m.time_unit_minutes();
}
