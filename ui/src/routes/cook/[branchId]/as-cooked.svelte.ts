/**
 * The **As Cooked** a cook builds at the stove (#58, ADR 0005).
 *
 * What this module holds is a whole recipe, never a list of differences. The
 * Attempt stores one only where the cooking actually differed, and the Core
 * decides that by fingerprint rather than trusting anything here — so a screen
 * that sends the recipe back untouched stores nothing at all.
 *
 * **Why a row carries `from`.** The cooking screen navigates by index: which
 * Step you are on, and which Ingredient Lines that Step uses, are indices into
 * the *Version being cooked*. The moment a cook drops a line or inserts a step,
 * a plain array stops lining up with those indices — `uses: [2]` would start
 * naming a different ingredient. Each row therefore remembers where it came
 * from in the Version, or says it came from nowhere because the cook wrote it.
 * Everything the screen looks up goes through `from`, and nothing goes through
 * an array position.
 *
 * **Why a dropped line is kept.** It stays in the list, marked, so a cook can
 * put it back with the same tap that took it out — nothing vanishes from under
 * a wet finger. It is simply left out when the recipe is serialised.
 *
 * **Nothing here pairs anything** (ADR 0019). Rebuilding this from an As
 * Cooked already stored uses the Core's own reading of it — the same Pairing
 * two Branches are laid over each other with, served on the Attempt. A frontend
 * matching lines by position would be wrong exactly when it matters.
 */
import type { GetRecipeOutput, StartAttemptOutput } from '$lib/api/catalogue';

type Version = GetRecipeOutput['versions'][number];
type Content = Version['content'];
/** The As Cooked as the Core serves it, with its own reading of what differs. */
type Served = NonNullable<StartAttemptOutput['as_cooked']>;

type IngredientKind = Content['ingredients'][number]['kind'];
type StepKind = Content['steps'][number]['kind'];

/** One line of the recipe as this cooking has it. */
export interface Line<K extends string = string> {
	/** 'ingredient' or 'section' in the ingredients; 'step' or 'section' in the steps. */
	kind: K;
	/** The words, as they now read. */
	text: string;
	/** Where this line sits in the Version cooked, or null where the cook wrote it. */
	from: number | null;
	/**
	 * The cook took this line out. Kept so it can be put back; never serialised.
	 *
	 * Present on a step as well as an ingredient, and no control drops a step
	 * today — Aurélien asked for adding and removing Ingredient Lines and for
	 * inserting a Step, and that is what the screen offers. The field is on both
	 * lists because they are one shape, and a second shape differing in one
	 * boolean would cost more than the unused field does. Dropping a step is
	 * where this goes next if *I skipped the marinating* turns out to be a
	 * thing people say.
	 */
	dropped: boolean;
}

/** The two lists a cook may write on. Nothing else about a recipe is editable at the stove. */
export interface AsCooked {
	ingredients: Line<IngredientKind>[];
	steps: Line<StepKind>[];
}

/** The pairing the Core serves on an As Cooked, for one of the two lists. */
type PairedRows = Served['against']['ingredients'];

/**
 * Rebuild the rows of one list from the Core's reading of an As Cooked.
 *
 * `only-mine` is a line the cook dropped — the Version has it, the As Cooked
 * does not. `only-theirs` is one they added. `changed` and `same` are the same
 * line, read differently or not.
 */
function fromPairing<K extends string>(paired: PairedRows): Line<K>[] {
	return paired.map((row) => ({
		// The Core built these rows out of the two lists themselves, so a kind
		// here is one that list already had. The declaration types it as a plain
		// string because the Pairing serves both lists through one shape.
		kind: row.kind as K,
		text: row.theirs?.text ?? row.mine?.text ?? '',
		from: row.mine?.index ?? null,
		dropped: row.theirs === null,
	}));
}

/** The recipe as written, one row per line, nothing changed. */
function fromContent<K extends string>(lines: { kind: K; text: string }[]): Line<K>[] {
	return lines.map((line, from) => ({ kind: line.kind, text: line.text, from, dropped: false }));
}

/**
 * The lines that are actually part of this recipe — what the cook dropped is
 * left out, and so is a line they opened and never wrote in. Shared by
 * `serialise` and `differs` so the two can never disagree about what counts.
 */
function kept<K extends string>(lines: Line<K>[]): Line<K>[] {
	return lines.filter((line) => !line.dropped && line.text.trim() !== '');
}

/**
 * What this cooking starts from: the words the cook already wrote down where
 * they wrote something, and the recipe as it stands where they did not.
 */
export function seed(content: Content, served: Served | null): AsCooked {
	if (served) {
		return {
			ingredients: fromPairing(served.against.ingredients),
			steps: fromPairing(served.against.steps),
		};
	}
	return { ingredients: fromContent(content.ingredients), steps: fromContent(content.steps) };
}

/**
 * The whole recipe as this cooking has it, in the shape `set_as_cooked` takes.
 *
 * Everything the cook cannot edit at the stove — the title, the Yield, the
 * times, the Main Photo — is carried straight through from the Version. A cook
 * standing at a hob is saying *I did this differently*, not renaming the dish.
 */
export function serialise(content: Content, asCooked: AsCooked) {
	const written = <K extends string>(lines: Line<K>[]) =>
		kept(lines).map((line) => ({ kind: line.kind, text: line.text.trim() }));
	return {
		title: content.title,
		yield: content.yield,
		prep_time_minutes: content.prep_time_minutes,
		cook_time_minutes: content.cook_time_minutes,
		note: content.note,
		main_photo: content.main_photo,
		source: content.source,
		nutrition: content.nutrition,
		ingredients: written(asCooked.ingredients),
		steps: written(asCooked.steps),
	};
}

/**
 * Whether this cooking has departed from the recipe at all.
 *
 * Only a hint for the screen — whether to bother sending anything, and whether
 * to mark the noren. The Core decides for real, by fingerprint, and an As Cooked
 * that says yes here but fingerprints to the Version cooked stores nothing.
 */
export function differs(content: Content, asCooked: AsCooked): boolean {
	const same = (lines: Line[], written: readonly { kind: string; text: string }[]) => {
		const mine = kept(lines);
		return (
			mine.length === written.length &&
			mine.every((line, at) => line.text.trim() === written[at]?.text.trim())
		);
	};
	return !(same(asCooked.ingredients, content.ingredients) && same(asCooked.steps, content.steps));
}
