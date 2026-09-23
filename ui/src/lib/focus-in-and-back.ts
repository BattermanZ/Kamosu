/**
 * A dialog takes the caret on the way in and gives it back on the way out —
 * the attachment `TagSheet` and `LanguageSheet` each carry, written once here
 * for the dialogs that came after them (#110): a keyboard user who closes one
 * must not lose their place on the page behind it.
 *
 * It focuses the dialog itself, which therefore needs `tabindex="-1"`, rather
 * than a control inside it, so the caret never lands on a choice that changes
 * the recipe.
 */

import type { Attachment } from 'svelte/attachments';

export const focusInAndBack: Attachment<HTMLElement> = (node) => {
	const cameFrom = document.activeElement;
	node.focus();
	return () => {
		requestAnimationFrame(() => {
			if (cameFrom instanceof HTMLElement && cameFrom.isConnected) cameFrom.focus();
		});
	};
};
