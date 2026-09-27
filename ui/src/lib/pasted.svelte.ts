/**
 * A recipe pasted as text, read by `read_pasted_recipe` and split where the
 * method starts (#94, #175), or a recipe PDF read into the same shape by
 * `read_recipe_pdf` (#176).
 *
 * Two places take one. The writing screen fills its own fields from it, and
 * the + beside the Recipes search box makes a new recipe from it (#175, both
 * of Aurélien's options). Both read it and split it the same way, so both
 * live here.
 */

import { m } from '$lib/paraglide/messages';
import { OperationError } from '$lib/api/client';
import type { KamosuClient, ReadPastedRecipeOutput } from '$lib/api/catalogue';

/**
 * Pasted text read by `read_pasted_recipe`, or the sentence saying why it
 * could not be: nothing pasted, nothing read from it, or the Core's refusal.
 * Anything else is a Mistake for the layers that catch those, and is thrown.
 */
export async function readPasted(
	kamosu: KamosuClient,
	text: string,
): Promise<ReadPastedRecipeOutput | string> {
	if (text.trim() === '') return m.write_paste_nothing();
	try {
		const answer = await kamosu.readPastedRecipe({ text });
		return answer.lines.length === 0 ? m.write_paste_nothing() : answer;
	} catch (error) {
		if (!(error instanceof OperationError)) throw error;
		return error.message;
	}
}

/**
 * A PDF already staged at `POST /api/uploads`, read by `read_recipe_pdf` into
 * what a paste is read into (#176), or the sentence saying why it could not
 * be. A scan is refused with a reason this says in the reader's own Language
 * (Aurélien's wording, chosen on #176); any other refusal is the Core's.
 */
export async function readPdf(
	kamosu: KamosuClient,
	uploadId: string,
): Promise<ReadPastedRecipeOutput | string> {
	try {
		return await kamosu.readRecipePdf({ upload_id: uploadId });
	} catch (error) {
		if (!(error instanceof OperationError)) throw error;
		return error.reason === 'pdf_has_no_text' ? m.plus_pdf_no_text() : error.message;
	}
}

/**
 * What a paste puts onto the writing screen: a title, if it had one, the two
 * lists, and what it said about the recipe, for its note (#176).
 */
export interface PastedDraft {
	title: string | null;
	note: string | null;
	ingredients: { kind: 'ingredient' | 'section'; text: string }[];
	steps: { kind: 'step' | 'section'; text: string }[];
}

/**
 * The paste split at `boundary`. The two kinds map straight across: a heading
 * is a Section in whichever list it landed in, and every line goes in exactly
 * as it came back (ADR 0002).
 */
export function drafted(pasted: ReadPastedRecipeOutput, boundary: number): PastedDraft {
	const title = pasted.title?.trim() ? pasted.title : null;
	return {
		title,
		note: pasted.note?.trim() ? pasted.note : null,
		ingredients: pasted.lines.slice(0, boundary).map((row) => ({
			kind: row.kind === 'section' ? 'section' : 'ingredient',
			text: row.text,
		})),
		steps: pasted.lines.slice(boundary).map((row) => ({
			kind: row.kind === 'section' ? 'section' : 'step',
			text: row.text,
		})),
	};
}

/**
 * A paste on its way from the + to the writing screen of the recipe it just
 * made. It has to survive one `goto`, so it is held here rather than in the
 * URL, the way a recipe file's arrival line is (`arrival.svelte.ts`). It is
 * NOT saved: the recipe the + made holds only its title, and the pasted lines
 * wait on the writing screen for Save, as they do when pasted there (#83).
 */
let held = $state<{ branchId: string; draft: PastedDraft } | undefined>(undefined);

/** Said by the + just before it opens the recipe it made. */
export function holdPaste(branchId: string, draft: PastedDraft): void {
	held = { branchId, draft };
}

/**
 * Taken once by the recipe page it was held for. A paste held for another
 * recipe is left alone, and taking it forgets it, so a reload an hour later
 * opens the recipe as it is saved.
 */
export function takePaste(branchId: string): PastedDraft | undefined {
	if (held?.branchId !== branchId) return undefined;
	const { draft } = held;
	held = undefined;
	return draft;
}
