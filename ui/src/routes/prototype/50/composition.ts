/**
 * PROTOTYPE — #50 Components. Throwaway. Not shipped, not tested, not linted
 * against the real standards. It exists to put two treatments in front of
 * Aurélien and is deleted the moment he has chosen.
 *
 * Components do not exist in the Core yet — that is what #50 builds. So this
 * module fabricates the one thing the backend cannot yet answer: which
 * Ingredient Line's Reading points at a Lineage rather than a Food. Everything
 * else on the screen is real — the recipes are read through the real
 * `get_recipe` Operation from the dev instance's own library, the written
 * lines, the Readings, the Yields, the photographs and the design tokens are
 * all as they stand today.
 *
 * The arithmetic here mirrors what `src/units.rs` already does for scaling, so
 * the factors shown are the factors Kamosu would print.
 */

import type { GetRecipeOutput } from '$lib/api/catalogue';

/** One Reading slot, as `get_recipe` hands it over. */
export type Slot = { amount: string | null; unit: string | null; target: string | null } | null;

/** A recipe as this rig holds it: what `get_recipe` answered, flattened. */
export type Held = {
	branchId: string;
	lineageId: string;
	title: string;
	yield: { amount: string; noun: string } | null;
	prep: number | null;
	cook: number | null;
	note: string | null;
	photo: string | null;
	ingredients: { kind: string; text: string }[];
	steps: { kind: string; text: string }[];
	readings: Slot[];
};

export function held(recipe: GetRecipeOutput): Held {
	const version = recipe.versions.at(-1)!;
	const content = version.content;
	return {
		branchId: recipe.branch_id,
		lineageId: recipe.lineage_id,
		title: content.title,
		yield: content.yield ?? null,
		prep: content.prep_time_minutes ?? null,
		cook: content.cook_time_minutes ?? null,
		note: content.note ?? null,
		photo: content.main_photo ?? null,
		ingredients: content.ingredients,
		steps: content.steps,
		readings: version.readings as Slot[],
	};
}

/**
 * The fabricated half: for one Branch, which line indexes carry a Reading
 * pointing at a Lineage. `lineage` is what the Reading names; it resolves to a
 * recipe or it does not, which is the whole of ADR 0008's degradation story.
 */
export type Composition = Record<string, Record<number, string>>;

// ---- the arithmetic ------------------------------------------------------

/**
 * The closed set of Units, copied from `src/units.rs` — the same ids, the same
 * base values, the same families and systems. A word outside it is no less a
 * Unit; it simply never converts (ADR 0016).
 */
type Kind = { id: string; family: 'mass' | 'volume'; metric: boolean; base: number };
const CUP = 236.5882365;
const UNITS: Record<string, Kind> = {};
const register = (kind: Kind, spellings: string[]) => {
	for (const word of spellings) UNITS[word] = kind;
};
register({ id: 'gram', family: 'mass', metric: true, base: 1 }, [
	'g',
	'gr',
	'gm',
	'gram',
	'grams',
	'gramme',
	'grammes',
]);
register({ id: 'kilogram', family: 'mass', metric: true, base: 1000 }, [
	'kg',
	'kgs',
	'kilo',
	'kilos',
	'kilogram',
	'kilograms',
]);
register({ id: 'ounce', family: 'mass', metric: false, base: 28.349523125 }, [
	'oz',
	'ounce',
	'ounces',
]);
register({ id: 'pound', family: 'mass', metric: false, base: 453.59237 }, [
	'lb',
	'lbs',
	'pound',
	'pounds',
]);
register({ id: 'millilitre', family: 'volume', metric: true, base: 1 }, [
	'ml',
	'mls',
	'millilitre',
	'millilitres',
	'milliliter',
]);
register({ id: 'centilitre', family: 'volume', metric: true, base: 10 }, [
	'cl',
	'centilitre',
	'centilitres',
]);
register({ id: 'decilitre', family: 'volume', metric: true, base: 100 }, [
	'dl',
	'decilitre',
	'decilitres',
]);
register({ id: 'litre', family: 'volume', metric: true, base: 1000 }, [
	'l',
	'lt',
	'litre',
	'litres',
	'liter',
	'liters',
]);
register({ id: 'teaspoon', family: 'volume', metric: false, base: CUP / 48 }, [
	'tsp',
	'tsps',
	'teaspoon',
	'teaspoons',
]);
register({ id: 'tablespoon', family: 'volume', metric: false, base: CUP / 16 }, [
	'tbsp',
	'tbsps',
	'tablespoon',
	'tablespoons',
]);
register({ id: 'cup', family: 'volume', metric: false, base: CUP }, ['cup', 'cups']);
register({ id: 'fluid ounce', family: 'volume', metric: false, base: 29.5735295625 }, [
	'fl oz',
	'floz',
	'fluid ounce',
	'fluid ounces',
]);

const bare = (word: string) => word.trim().toLowerCase().replace(/\.$/, '');
const recognise = (word: string | null | undefined): Kind | null =>
	word ? (UNITS[bare(word)] ?? null) : null;
/** `servings` and `serving` are the same noun; so are `pizzas` and `pizza`. */
const singular = (word: string) => bare(word).replace(/s$/, '');

/**
 * **How much of the inner recipe is wanted**, worked out from the Reading over
 * the line and the inner recipe's Yield — and stored nowhere (ADR 0008).
 *
 * `null` means Kamosu could not work it out, which is an ordinary answer and
 * not a failure: the inner recipe is then shown as written.
 */
export function factor(reading: Slot, innerYield: Held['yield']): number | null {
	// No quantity means the whole recipe. 28% of real ingredient rows carry
	// none, so this is the common case rather than the edge one.
	if (!reading?.amount) return 1;
	const amount = Number(reading.amount);
	if (!Number.isFinite(amount) || amount <= 0) return null;
	// An inner recipe with no Yield gives nothing to divide by.
	if (!innerYield) return null;
	const yieldAmount = Number(innerYield.amount);
	if (!Number.isFinite(yieldAmount) || yieldAmount <= 0) return null;

	const unit = recognise(reading.unit);
	const noun = recognise(innerYield.noun);

	// Mass against mass, volume against volume — through the same base values
	// `src/units.rs` converts on, so `500 g` of a dough yielding `1 kg` is ×0.5
	// and `3 tbsp` of an oil yielding `250 ml` is ×0.18.
	if (unit && noun && unit.family === noun.family) {
		return (amount * unit.base) / (yieldAmount * noun.base);
	}
	// The Yield's own noun used as a unit: `2 servings` of a recipe that yields
	// `4 servings`. A line with a bare number takes the Yield's noun.
	if (!reading.unit || singular(reading.unit) === singular(innerYield.noun)) {
		return amount / yieldAmount;
	}
	return null;
}

/** Nice fractions, so a factor reads as a quantity rather than as a number. */
const FRACTIONS: [number, string][] = [
	[1, 'the whole recipe'],
	[0.75, 'three quarters of the recipe'],
	[0.6667, 'two thirds of the recipe'],
	[0.5, 'half the recipe'],
	[0.3333, 'a third of the recipe'],
	[0.25, 'a quarter of the recipe'],
	[0.2, 'a fifth of the recipe'],
	[0.125, 'an eighth of the recipe'],
];

/** How much of the inner recipe, said in words. */
export function share(value: number): string {
	for (const [near, words] of FRACTIONS) {
		if (Math.abs(value - near) < 0.02) return words;
		if (Math.abs(value - near) < near * 0.15) return `about ${words.replace(/^the /, 'the ')}`;
	}
	if (value > 1) return `${trim(value)} times the recipe`;
	return `about ${Math.round(value * 100)}% of the recipe`;
}

/**
 * **The one subordinate line under a Component's inner Ingredient Line**: the
 * written amount scaled by the factor, in this reader's own measures.
 *
 * This mirrors `units::measured_line` for a metric reader — scale, then
 * convert, then round once at the end (ADR 0016) — so the amounts here are the
 * amounts Kamosu prints. A Unit outside the closed set keeps the cook's own
 * word and only the quantity multiplies. The Food is left off, because the
 * slot is silent where it would only repeat the line above it (#71).
 */
export function scaled(reading: Slot, by: number | null): string {
	if (by === null || !reading?.amount) return '';
	const amount = Number(reading.amount);
	if (!Number.isFinite(amount)) return '';
	const source = recognise(reading.unit);
	const value = amount * by;

	// A Unit Kamosu does not know still scales; its word is untouched.
	if (!source) {
		return by === 1 ? '' : `about ${decimal(round(value, null))} ${reading.unit ?? ''}`.trim();
	}
	// Already in a metric reader's own system: scale, do not re-express.
	if (source.metric) {
		if (by === 1) return '';
		return `about ${decimal(round(value, source.id))} ${short(source.id, value)}`;
	}
	// A customary measure crosses into metric.
	const base = value * source.base;
	const id =
		source.family === 'mass'
			? base >= 1000
				? 'kilogram'
				: 'gram'
			: base >= 1000
				? 'litre'
				: 'millilitre';
	const per = id === 'kilogram' || id === 'litre' ? 1000 : 1;
	const quantity = base / per;
	return `about ${decimal(round(quantity, id))} ${short(id, quantity)}`;
}

/** `round_to` in `src/units.rs`: round last, once, to the step the Unit deserves. */
function round(quantity: number, id: string | null): number {
	let step = 0.125;
	if (id === 'gram') step = quantity < 100 ? 1 : 5;
	else if (id === 'kilogram' || id === 'litre') step = 0.05;
	else if (id === 'millilitre') step = quantity < 25 ? 1 : quantity < 100 ? 5 : 10;
	else if (id === 'cup') step = 0.125;
	else if (['teaspoon', 'tablespoon', 'ounce', 'pound'].includes(id ?? '')) step = 0.25;
	return Math.round(quantity / step) * step;
}

const SHORT: Record<string, string> = {
	gram: 'g',
	kilogram: 'kg',
	millilitre: 'ml',
	litre: 'l',
	centilitre: 'cl',
	decilitre: 'dl',
};
const short = (id: string, quantity: number) => SHORT[id] ?? (quantity === 1 ? id : `${id}s`);

/** Metric stays whole numbers — two decimals at most, no trailing zeroes. */
function decimal(value: number): string {
	return String(Math.round(value * 100) / 100);
}

/** Two decimals at most, and no trailing zeroes — `src/units.rs`'s habit. */
function trim(value: number): string {
	const rounded = value >= 10 ? Math.round(value) : Math.round(value * 100) / 100;
	return String(rounded);
}
