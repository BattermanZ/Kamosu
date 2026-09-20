/**
 * The Shopping List's rows, added up on the phone (#77, ADR 0024).
 *
 * **The choosing is stored; the rows are computed**, and until #77 only the
 * server computed them. ADR 0024 says a list works offline, with the rows
 * computed for the recipes the phone holds, and Aurélien chose on #77 to have
 * the phone do that sum itself rather than leave a changed list unadded until
 * the server is back.
 *
 * So this is a second copy of `src/shopping.rs` — deliberately the smallest
 * one that can be. Everything that needs the database or Kamosu's word lists
 * (which Food a line was read as, that Food's name for this reader, which
 * Unit a written word is, what a cup of it weighs) arrives already decided,
 * from `shopping_basis`. What is left here is arithmetic and wording: add
 * what honestly adds, round it for the equipment in the drawer, and say it in
 * the reader's Language.
 *
 * **Two copies of a sum are one sum only while something checks them.**
 * `shopping_parity` in `src/shopping.rs` writes every case it can think of,
 * with the server's answer, to `tests/fixtures/shopping-parity.json`, and
 * `shopping.spec.ts` fails if this file answers any of them differently. When
 * the server's sum changes, both fail until this one agrees.
 */

import type {
	GetShoppingListOutput,
	GetReadingPreferencesOutput,
	ShoppingBasisOutput,
} from '$lib/api/catalogue';

export type Measures = GetReadingPreferencesOutput['reading_measures'];
export type List = GetShoppingListOutput;
export type Row = List['rows'][number];
type Part = Row['parts'][number];
type Line = ShoppingBasisOutput['lines'][number];
type Yield = ShoppingBasisOutput['written_yield'];

// --- Reading words and numbers --------------------------------------------

/**
 * Letters Unicode's default case folding writes differently from lower case.
 * JavaScript has only `toLowerCase`, which leaves these alone, so they are
 * folded by hand to match `units::fold`: `Straße` sorts as `strasse` there.
 */
const FOLDS: Record<string, string> = {
	ß: 'ss',
	ẞ: 'ss',
	ς: 'σ',
	ſ: 's',
	ŉ: 'ʼn',
	ﬀ: 'ff',
	ﬁ: 'fi',
	ﬂ: 'fl',
	ﬃ: 'ffi',
	ﬄ: 'ffl',
	ﬅ: 'st',
	ﬆ: 'st',
};

/**
 * A word as Kamosu compares words: decomposed, case folded, accents kept, and
 * forgiving about spacing and full stops — `units::fold`.
 */
export function fold(word: string): string {
	const cased = [...word.normalize('NFD').toLowerCase()]
		.map((character) => FOLDS[character] ?? character)
		.join('')
		.normalize('NFD');
	let folded = '';
	let wantsSpace = false;
	for (const character of cased) {
		if (/\s/.test(character) || character === '.') {
			wantsSpace = folded !== '';
			continue;
		}
		if (wantsSpace) {
			folded += ' ';
			wantsSpace = false;
		}
		folded += character;
	}
	return folded;
}

/** The vulgar fractions a recipe contains, in the order `units::FRACTIONS` lists them. */
const FRACTIONS: [string, number][] = [
	['½', 0.5],
	['⅓', 1 / 3],
	['⅔', 2 / 3],
	['¼', 0.25],
	['¾', 0.75],
	['⅕', 0.2],
	['⅖', 0.4],
	['⅗', 0.6],
	['⅘', 0.8],
	['⅙', 1 / 6],
	['⅚', 5 / 6],
	['⅛', 0.125],
	['⅜', 0.375],
	['⅝', 0.625],
	['⅞', 0.875],
];

/** A plain number, accepting the French decimal comma. */
function parsePlain(text: string): number | null {
	const plain = text.trim().replaceAll(',', '.');
	if (!/^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?$/.test(plain)) return null;
	const number = Number(plain);
	return Number.isFinite(number) ? number : null;
}

/** `1/2`, and nothing else that contains a slash. */
function parseRatio(text: string): number | null {
	const at = text.indexOf('/');
	if (at === -1) return null;
	const top = parsePlain(text.slice(0, at).trim());
	const bottom = parsePlain(text.slice(at + 1).trim());
	if (top === null || bottom === null || bottom === 0) return null;
	return top / bottom;
}

/** How much a written amount is worth, or null — `units::parse_amount`. */
export function parseAmount(written: string): number | null {
	const text = written.trim();
	if (text === '') return null;
	const last = [...text].at(-1) ?? '';
	const glyph = FRACTIONS.find(([character]) => character === last);
	if (glyph) {
		const whole = text.slice(0, text.length - last.length).trim();
		if (whole === '') return glyph[1];
		const number = parsePlain(whole);
		return number === null ? null : number + glyph[1];
	}
	const space = text.indexOf(' ');
	if (space !== -1) {
		const number = parsePlain(text.slice(0, space));
		const fraction = parseRatio(text.slice(space + 1).trim());
		if (number !== null && fraction !== null) return number + fraction;
	}
	return parseRatio(text) ?? parsePlain(text);
}

/**
 * How far a Yield somebody means is from the Yield as written, or 1 where the
 * two cannot honestly be compared — `core::yield_scale`.
 */
export function yieldScale(wanted: Yield, written: Yield): number {
	const sameNoun = (wanted?.noun ?? null) === (written?.noun ?? null);
	const wantedAmount = wanted ? parseAmount(wanted.amount) : null;
	const writtenAmount = written ? parseAmount(written.amount) : null;
	if (sameNoun && wantedAmount !== null && writtenAmount !== null) {
		if (writtenAmount > 0 && wantedAmount > 0) return wantedAmount / writtenAmount;
	}
	return 1;
}

// --- The closed set of Units, as far as adding up needs it ------------------

type Family = 'mass' | 'volume';
type System = 'metric' | 'customary';
interface Unit {
	id: string;
	family: Family;
	system: System;
	/** Grams or millilitres in one. */
	base: number;
}

const CUP = 236.5882365;

/** `units::UNITS`, without the spellings: `shopping_basis` has already read the word. */
const UNITS: Unit[] = [
	{ id: 'gram', family: 'mass', system: 'metric', base: 1 },
	{ id: 'kilogram', family: 'mass', system: 'metric', base: 1000 },
	{ id: 'ounce', family: 'mass', system: 'customary', base: 28.349523125 },
	{ id: 'pound', family: 'mass', system: 'customary', base: 453.59237 },
	{ id: 'millilitre', family: 'volume', system: 'metric', base: 1 },
	{ id: 'centilitre', family: 'volume', system: 'metric', base: 10 },
	{ id: 'decilitre', family: 'volume', system: 'metric', base: 100 },
	{ id: 'litre', family: 'volume', system: 'metric', base: 1000 },
	{ id: 'teaspoon', family: 'volume', system: 'customary', base: CUP / 48 },
	{ id: 'tablespoon', family: 'volume', system: 'customary', base: CUP / 16 },
	{ id: 'cup', family: 'volume', system: 'customary', base: CUP },
	{ id: 'fluid ounce', family: 'volume', system: 'customary', base: 29.5735295625 },
	{ id: 'imperial tablespoon', family: 'volume', system: 'customary', base: 17.758164 },
	{ id: 'australian tablespoon', family: 'volume', system: 'customary', base: 20 },
];

const unitById = (id: string): Unit | undefined => UNITS.find((unit) => unit.id === id);
const unit = (id: string): Unit => unitById(id)!;

/** `units::in_grams`: which bucket one amount adds into. Null is the volumes. */
function inGrams(
	base: number,
	of: Unit,
	measures: Measures,
	cupWeight: number | null,
): number | null {
	if (of.family === 'mass') return base;
	if (cupWeight === null) return null;
	return of.system === 'customary' && measures === 'metric'
		? (base / unit('cup').base) * cupWeight
		: null;
}

interface Measured {
	quantity: number;
	unit: Unit;
}

const at = (quantity: number, id: string, per: number): Measured => ({
	quantity: quantity / per,
	unit: unit(id),
});

function inMetricMass(grams: number): Measured {
	return grams >= 1000 ? at(grams, 'kilogram', 1000) : at(grams, 'gram', 1);
}

function inMetricVolume(millilitres: number): Measured {
	return millilitres >= 1000 ? at(millilitres, 'litre', 1000) : at(millilitres, 'millilitre', 1);
}

function inCustomaryVolume(millilitres: number): Measured {
	const tablespoon = unit('tablespoon').base;
	const cup = unit('cup').base;
	if (millilitres < tablespoon) return at(millilitres, 'teaspoon', unit('teaspoon').base);
	if (millilitres < cup / 4) return at(millilitres, 'tablespoon', tablespoon);
	return at(millilitres, 'cup', cup);
}

function inCustomaryMass(grams: number): Measured {
	const pound = unit('pound').base;
	return grams < pound ? at(grams, 'ounce', unit('ounce').base) : at(grams, 'pound', pound);
}

/** `units::round_to`: to something a scale or a jug can show. */
function roundTo(quantity: number, id: string | null): number {
	let step = 0.125;
	if (id === 'gram') step = quantity < 100 ? 1 : 5;
	else if (id === 'kilogram' || id === 'litre') step = 0.05;
	else if (id === 'millilitre') step = quantity < 25 ? 1 : quantity < 100 ? 5 : 10;
	else if (id === 'cup') step = 0.125;
	else if (id === 'teaspoon' || id === 'tablespoon' || id === 'ounce' || id === 'pound')
		step = 0.25;
	return roundHalfAway(quantity / step) * step;
}

/** Rust's `f64::round`: halves go away from zero, where `Math.round` sends -0.5 up. */
function roundHalfAway(value: number): number {
	return Math.sign(value) * Math.round(Math.abs(value));
}

/** `units::as_decimal`: `250`, `1.5`, never `1.50`. */
export function asDecimal(quantity: number): string {
	return quantity.toFixed(2).replace(/0+$/, '').replace(/\.$/, '');
}

/** `units::as_fraction`: `4½`, the way the cups in the drawer are marked. */
export function asFraction(quantity: number): string {
	const whole = Math.trunc(quantity);
	const part = quantity - whole;
	const glyph = FRACTIONS.find(([, value]) => Math.abs(value - part) < 0.005)?.[0];
	if (glyph === undefined) return asDecimal(quantity);
	return whole === 0 ? glyph : `${whole.toFixed(0)}${glyph}`;
}

function about(language: string): string {
	return language === 'fr' ? 'environ' : language === 'es' ? 'aprox.' : 'about';
}

/** `units::display`: only the cup has a plural. */
function display(of: Unit, language: string, plural: boolean): string {
	if (of.id === 'cup') {
		if (language === 'fr') return plural ? 'tasses' : 'tasse';
		if (language === 'es') return plural ? 'tazas' : 'taza';
		return plural ? 'cups' : 'cup';
	}
	const fixed: Record<string, string> = {
		gram: 'g',
		kilogram: 'kg',
		millilitre: 'ml',
		centilitre: 'cl',
		decilitre: 'dl',
		litre: 'l',
		ounce: 'oz',
		pound: 'lb',
		'fluid ounce': 'fl oz',
		'imperial tablespoon': 'imperial tbsp',
		'australian tablespoon': 'australian tbsp',
	};
	if (of.id === 'teaspoon')
		return language === 'fr' ? 'c. à c.' : language === 'es' ? 'cdta' : 'tsp';
	if (of.id === 'tablespoon')
		return language === 'fr' ? 'c. à s.' : language === 'es' ? 'cda' : 'tbsp';
	return fixed[of.id] ?? of.id;
}

/** `units::word_it`. */
function wordIt(measured: Measured, language: string): string {
	const quantity = roundTo(measured.quantity, measured.unit.id);
	const number = measured.unit.system === 'customary' ? asFraction(quantity) : asDecimal(quantity);
	return `${about(language)} ${number} ${display(measured.unit, language, quantity !== 1)}`;
}

function wordedMass(grams: number, measures: Measures, language: string): string {
	return wordIt(measures === 'us' ? inCustomaryMass(grams) : inMetricMass(grams), language);
}

function wordedVolume(millilitres: number, measures: Measures, language: string): string {
	return wordIt(
		measures === 'us' ? inCustomaryVolume(millilitres) : inMetricVolume(millilitres),
		language,
	);
}

/** `units::plain_number`: a count, or the figure before a word Kamosu does not know. */
function plainNumber(quantity: number): string {
	return asFraction(roundTo(quantity, null));
}

function noAmountWritten(language: string): string {
	return language === 'fr' ? 'sans quantité' : language === 'es' ? 'sin cantidad' : 'some';
}

// --- Adding up ---------------------------------------------------------------

interface Contribution {
	recipe: string;
	branchId: string;
	text: string;
	/** Already scaled. */
	amount: number | null;
	unit: string | null;
	unitId: string | null;
	unitKey: string | null;
	cupWeight: number | null;
}

function remember(sources: string[], recipe: string): void {
	if (!sources.includes(recipe)) sources.push(recipe);
}

/** `shopping::parts_for`: a Food's contributions laid out as the amounts its row carries. */
function partsFor(contributions: Contribution[], measures: Measures, language: string): Part[] {
	let grams = 0;
	let millilitres = 0;
	let count = 0;
	const asWritten: { key: string; word: string; total: number; sources: string[] }[] = [];
	const massFrom: string[] = [];
	const volumeFrom: string[] = [];
	const countFrom: string[] = [];
	const unstatedFrom: string[] = [];

	for (const contribution of contributions) {
		if (contribution.amount === null) {
			remember(unstatedFrom, contribution.recipe);
			continue;
		}
		const known =
			contribution.unit !== null && measures !== 'as_written' && contribution.unitId !== null
				? unitById(contribution.unitId)
				: undefined;
		if (!known && contribution.unit === null) {
			count += contribution.amount;
			remember(countFrom, contribution.recipe);
		} else if (!known) {
			const key = contribution.unitKey ?? fold(contribution.unit!);
			const held = asWritten.find((entry) => entry.key === key);
			if (held) {
				held.total += contribution.amount;
				remember(held.sources, contribution.recipe);
			} else {
				asWritten.push({
					key,
					word: contribution.unit!,
					total: contribution.amount,
					sources: [contribution.recipe],
				});
			}
		} else {
			const base = contribution.amount * known.base;
			const mass = inGrams(base, known, measures, contribution.cupWeight);
			if (mass !== null) {
				grams += mass;
				remember(massFrom, contribution.recipe);
			} else {
				millilitres += base;
				remember(volumeFrom, contribution.recipe);
			}
		}
	}

	const parts: Part[] = [];
	if (massFrom.length > 0)
		parts.push({ kind: 'about', text: wordedMass(grams, measures, language), sources: massFrom });
	if (volumeFrom.length > 0)
		parts.push({
			kind: 'about',
			text: wordedVolume(millilitres, measures, language),
			sources: volumeFrom,
		});
	if (countFrom.length > 0)
		parts.push({ kind: 'count', text: plainNumber(count), sources: countFrom });
	for (const entry of asWritten)
		parts.push({
			kind: 'as_written',
			text: `${plainNumber(entry.total)} ${entry.word}`,
			sources: entry.sources,
		});
	if (unstatedFrom.length > 0)
		parts.push({ kind: 'no_amount', text: noAmountWritten(language), sources: unstatedFrom });
	return parts;
}

/** One chosen recipe, as the rows add it up: its basis, and how far its Yield scales it. */
export interface Chosen {
	branch_id: string;
	title: string;
	scale: number;
	lines: Line[];
}

/** One comparison for every sort: code point order, as Rust compares strings. */
const byKey = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0);

/**
 * `shopping::line_key`: what names a row that merges with nothing, from the
 * path of line indexes that reaches its line — `3` for the chosen recipe's
 * fourth line, `3.1` for the second line of the dough that line names (#86).
 *
 * A plain index stopped being enough the moment a Component unfolded: the
 * pizza's first line and its dough's first line are two different things to
 * buy, and two rows sharing an id is a list Svelte cannot key.
 */
const lineKey = (path: number[]): string => path.join('.');

/** `shopping::rows`: every row of a list, from the choosing and the Loose Items. */
export function rows(chosen: Chosen[], loose: Row[], measures: Measures, language: string): Row[] {
	const foods = new Map<
		string,
		{ contributions: Contribution[]; food: NonNullable<Line['food']> }
	>();
	const out: Row[] = [];

	for (const entry of chosen) {
		for (const line of entry.lines) {
			// Which recipe the line is really from. A line that arrived by
			// unfolding a Component names the inner recipe — the dough, not the
			// pizza that composes it — so a row that breaks open says which dish
			// wants which, at whatever depth it came from (#86).
			const recipe = line.from?.title ?? entry.title;
			const branchId = line.from?.branch_id ?? entry.branch_id;
			if (!line.food) {
				out.push({
					id: `${entry.branch_id}:${lineKey(line.path)}`,
					kind: 'line',
					name: line.text,
					name_language: null,
					parts: [],
					lines: [{ branch_id: branchId, recipe, text: line.text }],
					// Set only on a line standing for a Component Kamosu could not
					// open, where it says which of the ways that happened.
					said: line.said,
				});
				continue;
			}
			const contribution: Contribution = {
				recipe,
				branchId,
				text: line.text,
				amount: line.food.amount === null ? null : line.food.amount * entry.scale,
				unit: line.food.unit,
				unitId: line.food.unit_id,
				unitKey: line.food.unit_key,
				cupWeight: line.food.cup_weight_grams,
			};
			const gathered = foods.get(line.food.id);
			if (gathered) gathered.contributions.push(contribution);
			else foods.set(line.food.id, { contributions: [contribution], food: line.food });
		}
	}

	for (const [id, { contributions, food }] of foods) {
		out.push({
			id,
			kind: 'food',
			name: food.name ?? contributions[0]?.text ?? '',
			name_language: food.name_language,
			parts: partsFor(contributions, measures, language),
			lines: contributions.map((contribution) => ({
				branch_id: contribution.branchId,
				recipe: contribution.recipe,
				text: contribution.text,
			})),
			// A Food row is every mention of one Food and belongs to no single
			// line, so there is nothing here for a sentence to be about.
			said: null,
		});
	}
	out.push(...loose);

	const keyed = out.map((row, order) => ({ row, order, key: fold(row.name) }));
	keyed.sort((a, b) => byKey(a.key, b.key) || a.order - b.order);
	return keyed.map(({ row }) => row);
}

// --- The text that leaves ------------------------------------------------------

function goneWritten(language: string): string {
	return language === 'fr'
		? 'ne peut plus être lu'
		: language === 'es'
			? 'ya no se puede leer'
			: 'can no longer be read';
}

function forWritten(language: string): string {
	return language === 'fr' ? 'pour' : language === 'es' ? 'para' : 'for';
}

/** `shopping::as_text`: the list as the text a Shortcut hands to Notes. */
export function asText(list: List, today: string, language: string): string {
	const here = list.chosen.filter((entry) => !entry.gone);
	const gone = list.chosen.filter((entry) => entry.gone);
	let out = today;
	if (here.length > 0) out += ` · ${here.map((entry) => entry.title).join(', ')}`;
	for (const entry of gone) out += ` · ${entry.title} ${goneWritten(language)}`;
	for (const row of list.rows) {
		out += `\n- [ ] ${row.name}`;
		if (row.parts.length === 1) out += ` — ${row.parts[0].text}`;
		else if (row.parts.length > 1)
			out += ` — ${row.parts
				.map((part) => `${part.text} ${forWritten(language)} ${part.sources.join(', ')}`)
				.join('; ')}`;
	}
	return `${out}\n`;
}
