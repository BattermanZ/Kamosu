/**
 * Two questions about the page as it stands, asked by whatever takes a key or
 * a drop for the whole app: the `/` shortcut (#203) and dropped files (#204).
 */

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

/**
 * Whether this is somewhere a person types words. A slash pressed here
 * belongs to what holds the caret, and so does a link dropped here.
 */
export function typedInto(target: EventTarget | null): boolean {
	if (!(target instanceof HTMLElement)) return false;
	if (target.isContentEditable || target.closest('textarea, select') !== null) return true;
	const input = target.closest('input');
	return input !== null && !NOT_TYPED_INTO.has(input.type);
}

/**
 * Whether a sheet is open over the page. A sheet is finished or closed before
 * anything else happens, so what acts on the page behind it waits.
 */
export const underASheet = (): boolean => document.querySelector('[role="dialog"]') !== null;
