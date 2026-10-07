/**
 * What each way of adding a recipe is drawn with and says while the server is
 * out of reach, written once for the + (#174) and for a recipe dropped onto
 * Recipes (#204), which is the same act by another road.
 */

import { m } from '$lib/paraglide/messages';
import type { Act } from './adding.svelte';

/** Each source's icon, as the path of a 24 by 24 line drawing. */
export const ICON: Record<Act, string> = {
	link: 'M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1',
	file: 'M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8zM14 3v5h5',
	pdf: 'M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8zM14 3v5h5M9 13h6M9 17h4',
	paste:
		'M9 4h6v3H9zM8 5H6a1 1 0 0 0-1 1v14a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V6a1 1 0 0 0-1-1h-2M8 12h8M8 16h5',
	write: 'M4 20h4L19 9l-4-4L4 16zM13.5 6.5l4 4',
};

/** What an act says offline instead of its label (#76). */
export const WAITS_FOR_SERVER: Record<Act, () => string> = {
	link: m.offline_waits_import_link,
	file: m.offline_waits_bring_in,
	pdf: m.offline_waits_pdf,
	write: m.offline_waits_write,
	paste: m.offline_waits_paste,
};
