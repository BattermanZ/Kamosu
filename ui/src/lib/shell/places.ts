/**
 * The places Kamosu's navigation names, written once for the two things that
 * draw them: the tab bar on the phone layout and the sidebar on the wide one
 * (ADR 0044, #194).
 */

import { m } from '$lib/paraglide/messages';

export interface Place {
	href: string;
	label: () => string;
	/** Drawn at 24px in the current colour; one path each, no icon library. */
	path: string;
}

/** Home · Recipes · Shopping · Cooked: the four the tab bar has always held. */
export const places: Place[] = [
	{
		href: '/',
		label: () => m.nav_home(),
		path: 'M3 10.5 12 3l9 7.5M5.5 9.5V20h13V9.5',
	},
	{
		href: '/recipes',
		label: () => m.nav_recipes(),
		path: 'M4 4h11a2 2 0 0 1 2 2v14H6a2 2 0 0 1-2-2V4Zm13 0h3v16M8 8h6M8 12h6',
	},
	{
		href: '/shopping',
		label: () => m.nav_shopping(),
		path: 'M4 6h3l2 11h9l2-8H8M10 21h.01M17 21h.01',
	},
	{
		href: '/cooked',
		label: () => m.nav_cooked(),
		path: 'M4 15h16a4 4 0 0 1-4 4H8a4 4 0 0 1-4-4ZM7 11c0-2 1.5-2 1.5-4M12 11c0-2 1.5-2 1.5-4M17 11c0-2 1.5-2 1.5-4',
	},
];

/**
 * Settings, the fifth place. Only the sidebar lists it: on the phone the way
 * in is the *You* card in the header (#102), which the wide layout does not
 * draw.
 */
export const settings: Place = {
	href: '/settings',
	label: () => m.open_settings(),
	path: 'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6ZM19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1Z',
};

/** The place a path belongs to: `/recipes/soba` is still Recipes. */
export function isCurrent(href: string, pathname: string): boolean {
	return href === '/' ? pathname === '/' : pathname === href || pathname.startsWith(`${href}/`);
}
