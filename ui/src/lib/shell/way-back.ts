/**
 * The way back from a page reached from another page (ADR 0044, #195).
 *
 * The wide layout draws a back arrow on every page the sidebar does not list.
 * It does what the device's own back does, so going back from a recipe
 * restores the search the way #191 makes back do. With nothing of Kamosu
 * behind the page (it was opened from a link, or the app was launched onto
 * it) it goes to the page's parent instead, so it never leaves the app and
 * never does nothing.
 */

import { places, settings } from './places';

/** The parent of every page that has one, the first that matches. */
const PARENTS: [RegExp, string][] = [
	// A recipe, and its Thread and Share with it.
	[/^\/recipes\/./, '/recipes'],
	[/^\/foods\/./, '/foods'],
	// A Report, and what one source brought in.
	[/^\/imports\/./, '/imports'],
	// Reached from the Settings page that lists each of them (#221).
	[/^\/foods$/, '/settings/kitchens'],
	[/^\/(imports|operator)$/, '/settings/instance'],
	// One of Settings' four pages, reached from its menu.
	[/^\/settings\/./, '/settings'],
];

/**
 * A Cookbook Invite is opened from a link somebody sent, and no page of
 * Kamosu leads to it. Its own two buttons are the ways on (#214).
 */
const OPENED_FROM_A_LINK = /^\/cookbook-invite\//;

/**
 * Where a page's back arrow goes when there is nothing behind it. The
 * sidebar's own destinations have no parent and so carry no arrow, and
 * neither does a page opened from a link.
 */
export function parentOf(pathname: string): string | undefined {
	if ([...places, settings].some((place) => place.href === pathname)) return undefined;
	if (OPENED_FROM_A_LINK.test(pathname)) return undefined;
	return PARENTS.find(([page]) => page.test(pathname))?.[1] ?? '/';
}

/**
 * SvelteKit numbers every history entry it makes and keeps the number in the
 * entry's own state: one more for each page opened, the same for an entry it
 * replaces, and the entry's own again when back or forward returns to it. It
 * is the one place that says how far into Kamosu this tab has gone, since a
 * browser tells a page neither where it is in the history nor what is behind
 * it. The name is SvelteKit's own (`HISTORY_INDEX` in its client) and no part
 * of what it documents, so the app reads it here and nowhere else.
 */
export const HISTORY_INDEX = 'sveltekit:history';

/**
 * Where each visit to Kamosu in this tab began: the number of the entry the
 * app was loaded on, one for every fresh load, kept for as long as the tab is.
 *
 * SvelteKit starts its numbering from the clock on every fresh load, so the
 * visits are in order and never overlap, and an entry belongs to the last
 * visit that began at or before it. One arrival would not do. A cook who
 * leaves for another site and comes back by a link has two visits in the tab,
 * and going back to the first page of the second must not walk out to that
 * site.
 */
export const VISITS = 'kamosu:visits-began-at';

/** More than a tab ever holds, and little enough to stay small. */
const KEPT = 20;

/** The number of the history entry on screen, before SvelteKit has set one: none. */
function entryNumber(): number | undefined {
	const number: unknown = (history.state as Record<string, unknown> | null)?.[HISTORY_INDEX];
	return typeof number === 'number' ? number : undefined;
}

function visits(): number[] {
	const kept: unknown = JSON.parse(sessionStorage.getItem(VISITS) ?? '[]');
	return Array.isArray(kept) ? kept.filter((began) => typeof began === 'number') : [];
}

/** Where the visit this entry belongs to began. */
function visitBegan(entry: number): number | undefined {
	const before = visits().filter((began) => began <= entry);
	return before.length ? Math.max(...before) : undefined;
}

/**
 * Remember the entry Kamosu was loaded on. Called once as the app starts.
 *
 * A fresh load begins a visit. A reload, or coming back to the app by back or
 * forward, lands on an entry of a visit already known and begins none. A link
 * opened in a new tab is a fresh load too, which matters because the new tab
 * is handed a copy of the storage of the tab it came from.
 */
export function noteArrival(): void {
	const entry = entryNumber();
	if (entry === undefined) return;
	try {
		const [load] = performance.getEntriesByType('navigation');
		const fresh = (load as PerformanceNavigationTiming | undefined)?.type === 'navigate';
		if (!fresh && visitBegan(entry) !== undefined) return;
		const began = [...visits().filter((other) => other !== entry), entry].slice(-KEPT);
		sessionStorage.setItem(VISITS, JSON.stringify(began));
	} catch {
		// Storage can be refused outright. The arrow then goes to the parent.
	}
}

/** Whether a page of this visit to Kamosu is behind the one on screen, for back to return to. */
export function somethingBehind(): boolean {
	const entry = entryNumber();
	if (entry === undefined) return false;
	try {
		const began = visitBegan(entry);
		return began !== undefined && entry > began;
	} catch {
		return false;
	}
}
