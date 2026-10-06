/**
 * Walking a list with the arrow keys, where the list stays beside what is
 * open (ADR 0044, #199). Foods is the first such list.
 */

/**
 * Whether a key was pressed with the caret somewhere that uses it: a field,
 * where the arrows move the caret, or a control that takes them itself.
 */
export function typingInAField(event: KeyboardEvent): boolean {
	const target = event.target;
	if (!(target instanceof HTMLElement)) return false;
	return target.isContentEditable || target.closest('input, textarea, select') !== null;
}

/**
 * Where an arrow leads in a list: the item above or below the open one, the
 * first where none of the list's is open, and the open one again at either
 * end. No answer for any other key, or an arrow held with another key, which
 * is the browser's.
 */
export function stepInList(
	event: KeyboardEvent,
	items: string[],
	open: string | undefined,
): string | undefined {
	const step = event.key === 'ArrowDown' ? 1 : event.key === 'ArrowUp' ? -1 : 0;
	if (step === 0 || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
	if (items.length === 0) return;
	const at = open === undefined ? -1 : items.indexOf(open);
	if (at < 0) return items[0];
	return items[Math.min(items.length - 1, Math.max(0, at + step))];
}
