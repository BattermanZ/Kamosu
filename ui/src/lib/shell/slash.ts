/**
 * `/` goes to the recipe search from anywhere (ADR 0044, #203): on Recipes it
 * puts the caret in the search box, and anywhere else it opens Recipes with
 * the caret already there.
 *
 * It is the one key Kamosu takes for itself. ADR 0044 rules out letter
 * shortcuts, and a slash typed into a field stays a slash.
 */

import type { Attachment } from 'svelte/attachments';
import { goto } from '$app/navigation';

/** The inputs a person types words into. A tick box is none of them. */
const NOT_TYPED_INTO = new Set([
	'button',
	'checkbox',
	'color',
	'file',
	'image',
	'radio',
	'range',
	'reset',
	'submit',
]);

/** The search box on Recipes, while Recipes is on screen. */
let box: HTMLInputElement | undefined;

/** `/` was pressed somewhere else, and Recipes has not drawn its box yet. */
let wanted = false;

/**
 * Recipes hands its search box over with this. A box drawn because `/` asked
 * for Recipes takes the caret as it arrives.
 */
export const recipeSearchBox: Attachment<HTMLInputElement> = (node) => {
	box = node;
	if (wanted) node.focus();
	wanted = false;
	return () => {
		if (box === node) box = undefined;
	};
};

/** Whether a slash pressed here belongs to what holds the caret. */
function typesASlash(target: EventTarget | null): boolean {
	if (!(target instanceof HTMLElement)) return false;
	if (target.isContentEditable || target.closest('textarea, select') !== null) return true;
	const input = target.closest('input');
	return input !== null && !NOT_TYPED_INTO.has(input.type);
}

/**
 * The key handler. Only `/` by itself is taken, and never from a field, from
 * a word being composed, or while a sheet is open over the page: a sheet is
 * finished or closed before anything else happens.
 *
 * Shift is allowed through, because a French keyboard writes `/` with it.
 */
export function slashToSearch(event: KeyboardEvent): void {
	if (event.key !== '/' || event.defaultPrevented || event.isComposing || event.repeat) return;
	if (event.altKey || event.ctrlKey || event.metaKey) return;
	if (typesASlash(event.target)) return;
	if (document.querySelector('[role="dialog"]') !== null) return;
	event.preventDefault();
	if (box?.isConnected) {
		box.focus();
		return;
	}
	void openRecipes();
}

/**
 * Recipes, with the caret asked for. A navigation that never arrives must not
 * leave the next visit to Recipes taking the caret for no reason.
 */
async function openRecipes(): Promise<void> {
	wanted = true;
	try {
		await goto('/recipes');
	} catch {
		wanted = false;
	}
}

/** A test starts with no box and nothing asked for. */
export function forgetSlash(): void {
	box = undefined;
	wanted = false;
}
