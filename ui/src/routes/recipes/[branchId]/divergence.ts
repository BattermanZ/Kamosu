/**
 * Reading a Divergence from wherever you are standing, and carrying a line
 * across.
 *
 * The Operation answers rows that are symmetric: `mine` is always the reader's
 * own recipe (#131), and reading anybody else's version is reading those same
 * rows from the other side. Everything here is that flip, plus the words each
 * row says once you tap it.
 *
 * There is deliberately nothing in this file that takes a whole Branch. The
 * absence is the decision (ADR 0014): taking every line one at a time is a
 * person making a recipe, whereas one button doing it is a merge with extra
 * steps.
 */

import { m } from '$lib/paraglide/messages';
import { branchPlainName } from '$lib/cookbook';
import type { DivergenceOutput, GetRecipeOutput, SaveRecipeVersionInput } from '$lib/api/catalogue';

/**
 * A recipe's Nutrition figure, taken from the Catalogue rather than written
 * out here: `basis` is a union of exactly two values, and spelling one of them
 * wrong has to be a build error rather than a figure that quietly says the
 * wrong thing (ADR 0012).
 */
export type Nutrition = NonNullable<GetRecipeOutput['versions'][number]['content']['nutrition']>;

export type Row = DivergenceOutput['ingredients'][number];
export type Side = 'mine' | 'theirs';
/** What a line was carried across as. `remove` mirrors a line they took out. */
export type Taken = 'write' | 'remove';

/** Which list a row belongs to, so a row has a stable name across re-renders. */
export type ListName = 'ingredients' | 'steps';

export const rowKey = (list: ListName, index: number): string => `${list}:${index}`;

/** One row, seen from the recipe you are standing in. */
export interface Seen {
	/** The line the recipe you are IN has here. Null makes this row a Ghost. */
	own: Row['mine'];
	/** The line the other recipe has here. */
	other: Row['mine'];
	ghost: boolean;
	/** Whether this line was already there at the Branch Point. */
	shared: boolean;
	state: Row['state'];
}

export function see(row: Row, side: Side): Seen {
	const own = side === 'mine' ? row.mine : row.theirs;
	const other = side === 'mine' ? row.theirs : row.mine;
	return {
		own,
		other,
		ghost: !own,
		shared: row.from_branch_point,
		state: row.state,
	};
}

/**
 * What a marked line or a Ghost says at rest, in words.
 *
 * A struck-through line with nothing beside it reads like something crossed off
 * a shopping list rather than a real line of a real recipe, so every one of them
 * is named.
 *
 * `kitchen` is ALWAYS the other version's name — `divergence.theirs`, whose it
 * is — never the one you happen to be standing in. Reading their recipe does
 * not make you them. Naming whichever you are not in instead put the wrong
 * name on a Ghost the moment you crossed: your own brown sugar, on their page,
 * read "Maison Batterman took it out" when Chez Marc had.
 */
export function caption(seen: Seen, side: Side, kitchen: string): string {
	if (!seen.ghost) {
		// A line both have, worded differently: on your page it is not theirs,
		// on theirs it is not yours.
		return side === 'mine' ? m.divergence_not_theirs({ kitchen }) : m.divergence_not_yours();
	}
	if (side === 'mine') return m.divergence_ghost_theirs({ kitchen });
	return seen.shared
		? m.divergence_ghost_removed({ kitchen })
		: m.divergence_ghost_never_had({ kitchen });
}

/** What Kamosu says about a Ghost when it is asked. It never hedges: by the
 *  time this is read, both texts are already on the page. `kitchen` is the
 *  other version's name, for the same reason as above. */
export function sentence(seen: Seen, side: Side, kitchen: string): string {
	if (!seen.ghost) return seen.other?.text ?? '';
	if (side === 'mine') return m.divergence_said_added({ kitchen });
	return seen.shared
		? m.divergence_said_removed({ kitchen })
		: m.divergence_said_yours_only({ kitchen });
}

/**
 * The offer under a row, and where it lives: only while you are standing in the
 * other person's recipe, because that is where you would want a line of it.
 * Their Branch is their Cookbook's, so nothing here writes to it (ADR 0041) —
 * every offer writes into YOUR recipe, and leaves it unsaved.
 */
export function offer(row: Row, taken: Taken | undefined): Taken | 'undo' | null {
	if (taken) return 'undo';
	// Read from the row rather than from where you happen to be standing: a
	// difference is visible from either Branch (ADR 0014), and so is the gesture.
	// Carrying always writes into YOUR recipe — `row.mine` — whichever recipe you
	// are reading when you decide to.
	if (row.state === 'changed' || row.state === 'only-theirs') return 'write';
	// A line of yours they haven't got. Only offerable where they actually took
	// it out; one you wrote after parting has nothing to carry across.
	if (row.state === 'only-mine') return row.from_branch_point ? 'remove' : null;
	return null;
}

/**
 * A line you have carried across, as it now reads on YOUR recipe — the new
 * words in place, and what they replaced (ADR 0014: "writes it into your recipe
 * and leaves it unsaved, marked in place with what it replaced, until you
 * save"). Only ever shown while standing in your own recipe: carrying changes
 * nothing about theirs.
 */
export function carriedInPlace(
	row: Row,
	taken: Taken | undefined,
): { text: string; replaced: string | null; leaving: boolean } | null {
	if (!taken) return null;
	if (taken === 'remove') {
		return row.mine ? { text: row.mine.text, replaced: null, leaving: true } : null;
	}
	return row.theirs
		? { text: row.theirs.text, replaced: row.mine?.text ?? null, leaving: false }
		: null;
}

/** One single value — Title, Yield, a time, the Source, the Note — on both sides. */
export interface Field {
	same: boolean;
	mine: unknown;
	theirs: unknown;
}

/**
 * How a single value reads when the two Branches do not agree on it. The
 * marking covers the whole recipe, not only the two lists (ADR 0019): a Title
 * renamed or a Yield halved is a difference a cook needs to see.
 */
export function fieldMark(field: Field | undefined, side: Side): string | null {
	if (!field || field.same) return null;
	const theirs = side === 'mine' ? field.theirs : field.mine;
	return String(theirs ?? '') || null;
}

/**
 * **The Nutrition figure as the one phrase it reads as** — the number and what
 * it counts, together and never apart. 308 says nothing until it says whether
 * it counts a serving or 100 g, and the two do not convert into each other
 * without a weight the recipe does not carry (CONTEXT.md, "Nutrition").
 *
 * Written once here because both places that show it — the foot of the
 * Ingredients on the recipe page, and the divergence mark beside it (#84) —
 * must say it in the same words.
 */
export function nutritionText(figure: Nutrition | null | undefined): string | null {
	if (!figure) return null;
	const calories = String(figure.calories);
	// Two bases, two phrases, and no third arm to fall through: the basis is
	// never guessed, because a figure that says the wrong one is worse than a
	// figure nobody shows.
	return figure.basis === 'per_100g'
		? m.recipe_kcal_100g({ calories })
		: m.recipe_kcal_serving({ calories });
}

/**
 * A `divergence` field read back as a Nutrition figure, or nothing. The rows
 * come off the wire as `unknown`, so this is the one place the shape is
 * checked — and a basis the Catalogue does not declare is not a figure this
 * build knows how to word, so it says nothing rather than picking one.
 */
function asNutrition(value: object): Nutrition | null {
	const figure = value as Partial<Nutrition>;
	if (typeof figure.calories !== 'number') return null;
	if (figure.basis !== 'per_serving' && figure.basis !== 'per_100g') return null;
	return { calories: figure.calories, basis: figure.basis };
}

/** A Yield, a Source, a time or a Nutrition figure, as the phrase it reads as. */
export function fieldText(value: unknown): string {
	if (value === null || value === undefined) return m.divergence_field_none();
	if (typeof value === 'object') {
		const shape = value as { amount?: string; noun?: string; text?: string };
		if (shape.text !== undefined) return shape.text;
		if (shape.amount !== undefined) return `${shape.amount} ${shape.noun ?? ''}`.trim();
		const figure = asNutrition(value);
		if (figure) return nutritionText(figure) ?? '';
	}
	return String(value);
}

/**
 * Your recipe with the lines you have carried across written into it, rebuilt
 * from the rows so a taken line lands where the Pairing puts it rather than at
 * the bottom of the list.
 *
 * Untouched, this reproduces your own list exactly, which is what makes it safe
 * to send as a whole Version: `save_recipe_version` replaces what is on the
 * Branch, as it does for every other edit.
 */
export function draftList(
	rows: Row[],
	list: ListName,
	taken: Map<string, Taken>,
	/** Your own lines, so a Step you keep keeps its Photograph. */
	ownSteps: { photo: string | null }[] = [],
) {
	const out: {
		kind: 'section' | 'ingredient' | 'step';
		text: string;
		photo: string | null;
	}[] = [];
	rows.forEach((row, index) => {
		const decision = taken.get(rowKey(list, index));
		const kind = row.kind as 'section' | 'ingredient' | 'step';
		if (!decision) {
			// A Photograph is known by its own contents (ADR 0017), so a Step you
			// are not changing keeps the one it had.
			if (row.mine) {
				out.push({
					kind,
					text: row.mine.text,
					photo: ownSteps[row.mine.index]?.photo ?? null,
				});
			}
			return;
		}
		// Carrying a line across carries its WORDS. Their Step's Photograph is
		// held by their Cookbook and is not yours to write into your recipe.
		if (decision === 'write' && row.theirs) out.push({ kind, text: row.theirs.text, photo: null });
		// `remove` writes nothing: that is what taking a removal across means.
	});
	return out;
}

/** Everything a save needs: your whole recipe as it now reads. */
export function draftVersion(
	divergence: DivergenceOutput,
	taken: Map<string, Taken>,
	changeNote: string,
): SaveRecipeVersionInput {
	const content = divergence.mine.content;
	return {
		branch_id: divergence.mine.branch_id,
		title: content.title,
		// Sent as written, nulls and all. `save_recipe_version` replaces the whole
		// recipe, so a field dropped here would be a field erased — and a null is
		// a real value the Catalogue declares, not an absence to tidy away.
		yield: content.yield,
		prep_time_minutes: content.prep_time_minutes,
		cook_time_minutes: content.cook_time_minutes,
		note: content.note,
		main_photo: content.main_photo,
		source: content.source,
		nutrition: content.nutrition,
		ingredients: draftList(divergence.ingredients, 'ingredients', taken).map(({ kind, text }) => ({
			kind,
			text,
		})) as { kind: 'section' | 'ingredient'; text: string }[],
		steps: draftList(divergence.steps, 'steps', taken, content.steps) as {
			kind: 'section' | 'step';
			text: string;
			photo: string | null;
		}[],
		change_note: changeNote,
	};
}

/**
 * How a Step is named in a sentence: by its own opening words.
 *
 * An earlier pass classified the step — `marinade`, `frying`, `resting` — by
 * matching English keywords. That was wrong twice over: it read only English,
 * so a French Branch saved a French recipe with an English label in its history;
 * and it put a name of Kamosu's invention where the cook's own words would do.
 * ADR 0019 already settled the same argument about a rewritten Step: the text
 * is better evidence than a label about the text.
 */
function openingOf(text: string): string {
	const words = text.trim().split(/\s+/).slice(0, 5).join(' ');
	return words.length < text.trim().length ? `${words}…` : words;
}

interface Carried {
	kind: Taken;
	text: string;
	isStep: boolean;
}

/** The lines carried across, in the order they sit in the recipe. */
export function carried(divergence: DivergenceOutput, taken: Map<string, Taken>): Carried[] {
	const out: Carried[] = [];
	const walk = (rows: Row[], list: ListName) =>
		rows.forEach((row, index) => {
			const kind = taken.get(rowKey(list, index));
			if (!kind) return;
			// On a `remove` the line is the one leaving YOUR recipe; on a `write`
			// it is theirs arriving. On a Ghost, `mine` is null by definition.
			const line = kind === 'remove' ? row.mine : (row.theirs ?? row.mine);
			if (line) out.push({ kind, text: line.text, isStep: row.kind === 'step' });
		});
	walk(divergence.ingredients, 'ingredients');
	walk(divergence.steps, 'steps');
	return out;
}

/**
 * The *what changed* line a save is pre-filled with — prose, never a structured
 * pointer at the other recipe. Their Branch could be gone tomorrow and this
 * sentence would still mean something, which is the whole reason it is a
 * sentence (ADR 0004).
 */
export function prose(divergence: DivergenceOutput, taken: Map<string, Taken>): string {
	const kitchen = branchPlainName(divergence.theirs);
	const items = carried(divergence, taken);
	const say = (item: Carried) =>
		item.isStep
			? m.divergence_prose_step({ opening: openingOf(item.text) })
			: item.text.length > 42
				? `${item.text.slice(0, 39).trimEnd()}…`
				: item.text;
	const join = (parts: string[]) =>
		parts.length === 1
			? parts[0]
			: m.divergence_prose_and({
					list: parts.slice(0, -1).join(', '),
					last: parts.at(-1) ?? '',
				});

	const wrote = items.filter((item) => item.kind === 'write').map(say);
	const removed = items.filter((item) => item.kind === 'remove').map(say);

	const sentences: string[] = [];
	if (wrote.length) sentences.push(m.divergence_prose_took({ what: join(wrote), kitchen }));
	if (removed.length) sentences.push(m.divergence_prose_took_out({ what: join(removed), kitchen }));
	return sentences.join(' ');
}

/** How a Reading reads under its line: what Kamosu understood, and nothing more. */
export function reading(
	slot: {
		amount: string | null;
		unit: string | null;
		target: string | null;
	} | null,
): string {
	if (!slot) return '';
	return [slot.amount, slot.unit, slot.target].filter(Boolean).join(' ');
}
