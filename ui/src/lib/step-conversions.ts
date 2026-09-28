/**
 * A Step's text, cut where each conversion goes (#150).
 *
 * The Core answers each conversion a Step offers as `{ written, measured }`:
 * the oven temperature, or the amount and Unit, exactly as the text has them,
 * and what they come to for this reader. Each addition goes straight after
 * what it converts, "1 lb. (about 455 g) ground chicken" and "preheat to 425°
 * (about 220 °C)": option A, chosen by Aurélien on 25 September 2026 over one
 * line of figures beneath the Step.
 *
 * Found by searching forward from the last one, so two `2 Tbsp.` in one Step
 * land on the first and the second in turn. One not found (the cook rewrote
 * the words) is left out rather than put somewhere it does not belong.
 * The text itself is never changed: joining each piece's `text` and `written`
 * gives it back.
 */
import type { GetRecipeOutput } from '$lib/api/catalogue';

/**
 * A Step's conversions as the Catalogue declares them — its oven and its
 * amounts, in the order written — so a change there breaks the build here.
 */
export type StepConversions = NonNullable<
	GetRecipeOutput['versions'][number]['measured']['steps'][number]
>;

export type StepPiece = { text: string; written: string | null; measured: string | null };

export function stepPieces(text: string, conversions: StepConversions): StepPiece[] {
	const pieces: StepPiece[] = [];
	let from = 0;
	// A phone keeps its answers across an update of Kamosu (#76), so the first
	// read after this one shipped can be an older answer, where a Step's slot
	// was a string. It draws nothing until the refresh behind it arrives.
	for (const { written, measured } of Array.isArray(conversions) ? conversions : []) {
		const at = text.indexOf(written, from);
		if (at < 0) continue;
		pieces.push({ text: text.slice(from, at), written, measured });
		from = at + written.length;
	}
	pieces.push({ text: text.slice(from), written: null, measured: null });
	return pieces;
}
