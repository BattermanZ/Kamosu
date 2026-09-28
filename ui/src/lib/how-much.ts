/**
 * HOW MUCH OF A RECIPE SOMEBODY MEANS (#109) — the choices a cook or a reader
 * is offered, worked out once for the cooking screen and the recipe page.
 *
 * What is chosen is a Yield: `{amount, noun}` counted in the recipe's own noun,
 * or a multiplier — a Yield with an empty noun, `{amount: '2', noun: ''}` —
 * for a recipe that never said what it makes, or where counting would land
 * between two whole servings. `null` is the recipe as written.
 *
 * NOTHING HERE TOUCHES AN INGREDIENT AMOUNT. The arithmetic below is on the
 * Yield — twelve servings doubled is twenty-four — and the amounts that follow
 * from it come back from the Core, scaled and converted in one line (ADR 0016,
 * #49). A screen shows what `get_recipe` answers and never works a line out.
 */
import { parseAmount, yieldScale } from '$lib/offline/shopping';
import type { GetRecipeOutput } from '$lib/api/catalogue';

export type Written = GetRecipeOutput['versions'][number]['content']['yield'];
export type Wanted = GetRecipeOutput['versions'][number]['scaled_to'];

/**
 * Whether a choice is a multiplier (`×2`) rather than an amount of what the
 * recipe makes — the one place in the browser the empty noun is read as that,
 * as `core::is_multiplier` is on the server.
 */
export function isMultiplier(wanted: Wanted | Written): boolean {
	return wanted !== null && wanted.noun === '';
}

/** The multipliers offered beside the recipe as written, in the order shown. */
const TIMES = [0.5, 1, 2, 3] as const;

/** A multiplier as it is written down and said: ½, 2, 3. */
function timesText(times: number): string {
	return times === 0.5 ? '½' : String(times);
}

/**
 * What the recipe makes, as a count to step from — or null where it cannot be
 * counted: no Yield, or one like `a dozen`.
 */
export function writtenCount(written: Written): number | null {
	const count = written ? parseAmount(written.amount) : null;
	return count !== null && count > 0 ? count : null;
}

/** Whether two choices are the same choice. */
export function same(a: Wanted, b: Wanted): boolean {
	if (a === null || b === null) return a === b;
	return a.amount.trim() === b.amount.trim() && a.noun === b.noun;
}

/**
 * The one choice for `times` the recipe: counted in its own noun wherever that
 * lands on a whole number, and a multiplier otherwise.
 */
export function timesOf(written: Written, times: number): Wanted {
	if (times === 1) return null;
	const count = writtenCount(written);
	const counted = count === null ? null : count * times;
	if (written && counted !== null && Number.isInteger(counted))
		return { amount: String(counted), noun: written.noun };
	return { amount: timesText(times), noun: '' };
}

/** One of the quick choices: what it is, and the two halves of its label. */
export interface Choice {
	wanted: Wanted;
	/** `×2`, `×½`, or null for the recipe as written. */
	times: string | null;
	/** What it comes to in the recipe's own noun, where it counts: `24`. */
	count: string | null;
}

export function choices(written: Written): Choice[] {
	return TIMES.map((times) => {
		const wanted = timesOf(written, times);
		return {
			wanted,
			times: times === 1 ? null : `×${timesText(times)}`,
			count:
				wanted && !isMultiplier(wanted)
					? wanted.amount
					: times === 1
						? (written?.amount ?? null)
						: null,
		};
	});
}

/**
 * The count one step from what is chosen — `null` where that step lands back
 * on the recipe as written, and `undefined` where there is no step to take: a
 * recipe that says nothing, one like `a dozen`, or a step below one.
 *
 * From a multiplier it steps from what the multiplier comes to — half of three
 * servings is one and a half, so − is one serving and + is two.
 */
export function stepped(written: Written, wanted: Wanted, by: 1 | -1): Wanted | undefined {
	const count = writtenCount(written);
	if (!written || count === null || !Number.isInteger(count)) return undefined;
	const times = wanted && isMultiplier(wanted) ? parseAmount(wanted.amount) : null;
	const now =
		times !== null
			? count * times
			: wanted && wanted.noun === written.noun
				? (parseAmount(wanted.amount) ?? count)
				: count;
	const next = Math.floor(now) + (by === -1 && !Number.isInteger(now) ? 0 : by);
	if (next < 1) return undefined;
	return next === count ? null : { amount: String(next), noun: written.noun };
}

/** Whether − and + mean anything for this recipe. */
export function counts(written: Written): boolean {
	const count = writtenCount(written);
	return count !== null && Number.isInteger(count);
}

/**
 * A Yield as a reader says it — `24 servings`, or `×2` for a multiplier — or
 * null for none. What a recipe makes is said the same way.
 */
export function said(wanted: Wanted | Written): string | null {
	if (!wanted) return null;
	return isMultiplier(wanted) ? `×${wanted.amount}` : `${wanted.amount} ${wanted.noun}`;
}

/**
 * Whether an answer from `get_recipe` was scaled to what is chosen now.
 *
 * It asks `yieldScale` — the offline Shopping List's mirror of
 * `core::yield_scale` (#77) — only whether a choice scales at all, which is
 * arithmetic on the Yield and never on an amount (ADR 0016). It is
 * not while the phone has no network (#77): the choice is kept and sent later,
 * and the amounts on screen are still the ones the phone had.
 *
 * A choice the Core would not scale by at all — twelve servings of a recipe
 * for twelve, or loaves against servings from an agent at the other Door —
 * comes back as `null`, and is caught up when it does.
 */
export function caughtUp(scaledTo: Wanted, wanted: Wanted, written: Written): boolean {
	return same(scaledTo, yieldScale(wanted, written) === 1 ? null : wanted);
}

/** A choice as the URL carries it from the recipe page to the cooking (#109). */
export function toSearch(wanted: Wanted): string {
	if (!wanted) return '';
	const params = new URLSearchParams({ amount: wanted.amount, noun: wanted.noun });
	return `?${params}`;
}

export function fromSearch(search: URLSearchParams): Wanted | undefined {
	const amount = search.get('amount');
	const noun = search.get('noun');
	if (amount === null || noun === null) return undefined;
	return { amount, noun };
}
