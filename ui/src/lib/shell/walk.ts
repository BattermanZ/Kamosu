/**
 * Walking a list with the arrow keys, where the list stays beside what is
 * open (ADR 0044, #199). Foods is the first such list and Brought in the
 * second (#200).
 */

import { goto } from '$app/navigation';

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

/**
 * Open what an arrow led to, in place of what was open: the history entry is
 * replaced, so going back leaves the list in one step, and the caret and the
 * scroll stay where they are.
 *
 * An arrow pressed again before the last one has finished opening ends that
 * navigation, and SvelteKit rejects its promise. That is a key held down and
 * no Mistake, so it is caught here and never reaches the net under the app
 * (#98), which showed the *Kamosu went wrong* card for it.
 */
export async function openInPlace(address: string): Promise<void> {
	try {
		await goto(address, { replaceState: true, keepFocus: true, noScroll: true });
	} catch {
		// Overtaken by the next arrow.
	}
}
