/**
 * Which navigation the shell draws, by how much room the window has (ADR 0044,
 * #194): the sidebar on the wide layout, the header and tab bar on the phone,
 * and neither where a screen is drawn bare.
 *
 * The two navigations carry the same accessible name, so they are told apart
 * by what only one of them has: the sidebar lists Settings, and the header is
 * the page's banner.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import ShellTestHarness from './ShellTestHarness.svelte';
import { heardWhetherSignedIn, signingIn } from '$lib/shell/signing-in.svelte';
import { HISTORY_INDEX, VISITS, noteArrival } from '$lib/shell/way-back';
import { m } from '$lib/paraglide/messages';
import type { Room } from '$lib/room.svelte';

function shell(pathname: string, room: Room, holds?: 'account' | 'story') {
	render(ShellTestHarness, { props: { pathname, room, holds } });
}

// A Person is signed in, and this device has heard so, unless a test says otherwise.
beforeEach(() => {
	localStorage.clear();
	heardWhetherSignedIn(true);
});

const navigation = () => screen.queryByRole('navigation', { name: m.nav_sections() });

/** The places the navigation on screen lists, in order, by their addresses. */
function places(): string[] {
	const nav = navigation();
	if (!nav) return [];
	return within(nav)
		.getAllByRole('link')
		.map((link) => link.getAttribute('href') ?? '');
}

const SIDEBAR = ['/', '/recipes', '/shopping', '/cooked', '/settings'];
const TABS = ['/', '/recipes', '/shopping', '/cooked'];

describe('on the wide layout', () => {
	it.each<Room>(['wide', 'roomy'])('a %s window gets the sidebar and no tab bar', (room) => {
		shell('/', room);

		expect(places()).toEqual(SIDEBAR);
		expect(screen.queryByRole('banner')).toBeNull();
		expect(screen.getAllByRole('navigation')).toHaveLength(1);
	});

	it('writes Kamosu on the sidebar, since no header is left to say it', () => {
		shell('/', 'wide');

		expect(navigation()).toHaveTextContent(m.app_name());
	});

	it.each([
		['/', '/'],
		['/recipes', '/recipes'],
		['/recipes/b_soba', '/recipes'],
		['/recipes/b_soba/thread', '/recipes'],
		['/shopping', '/shopping'],
		['/cooked', '/cooked'],
		['/settings', '/settings'],
	])('at %s it marks %s as where you are, and nothing else', (pathname, href) => {
		shell(pathname, 'wide');

		const current = within(navigation()!)
			.getAllByRole('link')
			.filter((link) => link.getAttribute('aria-current') === 'page');
		expect(current.map((link) => link.getAttribute('href'))).toEqual([href]);
	});

	it('labels every place in words a sighted person can read', () => {
		shell('/', 'wide');

		const labels = within(navigation()!)
			.getAllByRole('link')
			.map((link) => link.textContent?.trim());
		expect(labels).toEqual([
			m.nav_home(),
			m.nav_recipes(),
			m.nav_shopping(),
			m.nav_cooked(),
			m.open_settings(),
		]);
	});

	it('draws nothing at all round the cooking screen', () => {
		shell('/cook/b_soba', 'roomy');

		expect(navigation()).toBeNull();
		expect(screen.queryByRole('banner')).toBeNull();
	});

	it.each([['/invite/abc'], ['/recover/abc'], ['/cookbook-invite/abc']])(
		'draws no sidebar at %s, and writes Kamosu above the page with no way into Settings',
		(pathname) => {
			shell(pathname, 'roomy');

			expect(navigation()).toBeNull();
			expect(screen.getByRole('banner')).toHaveTextContent(m.app_name());
			expect(within(screen.getByRole('banner')).queryByRole('link')).toBeNull();
		},
	);

	it('draws no sidebar under the story, wherever the story is shown', async () => {
		shell('/', 'wide', 'story');

		await waitFor(() => expect(navigation()).toBeNull());
		expect(screen.getByRole('main')).toHaveAttribute('data-bare', 'true');
	});

	it('draws none at /about even before the story has said it is showing', () => {
		shell('/about', 'wide');

		expect(navigation()).toBeNull();
		expect(screen.getByRole('main')).toHaveAttribute('data-bare', 'true');
	});

	it('draws no sidebar beside the account form, and brings it back once the form is gone', async () => {
		const { rerender } = render(ShellTestHarness, {
			props: { pathname: '/import', room: 'wide', holds: 'account' },
		});

		await waitFor(() => expect(navigation()).toBeNull());
		expect(screen.getByRole('banner')).toHaveTextContent(m.app_name());

		await rerender({ pathname: '/import', room: 'wide', holds: undefined });
		await waitFor(() => expect(places()).toEqual(SIDEBAR));
	});
});

describe('on the wide layout, before anybody has answered who is here', () => {
	it('draws no sidebar at / on a device that has heard nothing, so a stranger never sees one', () => {
		signingIn.signedIn = undefined;
		shell('/', 'wide');

		expect(navigation()).toBeNull();
	});

	it('draws none at /import on such a device either, where a Share Link sends a stranger', () => {
		signingIn.signedIn = undefined;
		shell('/import', 'wide');

		expect(navigation()).toBeNull();
	});

	it.each([['/'], ['/import']])(
		'draws none at %s on a device last told nobody is signed in',
		(pathname) => {
			heardWhetherSignedIn(false);
			shell(pathname, 'wide');

			expect(navigation()).toBeNull();
		},
	);

	it.each([[undefined], [false]])(
		'draws it on any other page whatever the device heard (%s), so a wide window always has a way to /',
		(heard) => {
			if (heard === false) heardWhetherSignedIn(false);
			else signingIn.signedIn = undefined;
			shell('/recipes/b_soba', 'wide');

			expect(places()).toEqual(SIDEBAR);
		},
	);

	it('remembers the answer between visits', () => {
		heardWhetherSignedIn(false);
		expect(localStorage.getItem('kamosu.signed-in')).toBe('no');
		heardWhetherSignedIn(true);
		expect(localStorage.getItem('kamosu.signed-in')).toBe('yes');
	});
});

describe('on the phone layout', () => {
	it('keeps the header and the four tabs, and lists no Settings among them', () => {
		shell('/', 'phone');

		expect(places()).toEqual(TABS);
		expect(screen.getByRole('banner')).toHaveTextContent(m.header_you());
	});

	it('keeps them on the account form and the link pages, as it always has', async () => {
		heardWhetherSignedIn(false);
		shell('/invite/abc', 'phone', 'account');

		await waitFor(() => expect(screen.getByRole('banner')).toBeInTheDocument());
		expect(places()).toEqual(TABS);
	});

	it('draws neither on the cooking screen', () => {
		shell('/cook/b_soba', 'phone');

		expect(navigation()).toBeNull();
		expect(screen.queryByRole('banner')).toBeNull();
		expect(screen.getByRole('main')).toHaveAttribute('data-bare', 'true');
	});

	it('draws neither under the story', async () => {
		shell('/about', 'phone', 'story');

		await waitFor(() => expect(navigation()).toBeNull());
		expect(screen.queryByRole('banner')).toBeNull();
	});
});

/**
 * The back arrow on a page reached from another page (ADR 0044, #195): on the
 * wide layout only, on every page the sidebar does not list.
 */
describe('the back arrow', () => {
	const arrow = () => screen.queryByRole('link', { name: m.shell_back() });

	/** Put this tab on the history entry SvelteKit numbered `number`. */
	const onEntry = (number: number) => history.replaceState({ [HISTORY_INDEX]: number }, '');

	/**
	 * Tap the arrow, and say whether it took the tap over rather than leaving
	 * it an ordinary link to the parent. The link is not followed either way:
	 * jsdom has nowhere to go.
	 */
	async function tapTakenOver(): Promise<boolean> {
		let taken = false;
		const seen = (event: Event) => {
			taken = event.defaultPrevented;
			event.preventDefault();
		};
		document.addEventListener('click', seen, { once: true });
		await fireEvent.click(arrow()!);
		return taken;
	}

	afterEach(() => {
		history.replaceState(null, '');
		sessionStorage.removeItem(VISITS);
		vi.restoreAllMocks();
	});

	it.each([
		['a recipe', '/recipes/b_soba', '/recipes'],
		["a recipe's Thread", '/recipes/b_soba/thread', '/recipes'],
		["a recipe's Share", '/recipes/b_soba/share', '/recipes'],
		['a Food', '/foods/f_flour', '/foods'],
		['a Report', '/imports/j_1', '/imports'],
		['what one source brought in', '/imports/from/crouton', '/imports'],
		['Foods', '/foods', '/settings'],
		['Brought in', '/imports', '/settings'],
		['the Operator screen', '/operator', '/settings'],
	])('is on %s, and leads to its parent when nothing is behind it', async (_, pathname, parent) => {
		shell(pathname, 'wide');

		expect(arrow()).toHaveAttribute('href', parent);
		expect(await tapTakenOver()).toBe(false);
	});

	it.each([['/'], ['/recipes'], ['/shopping'], ['/cooked'], ['/settings']])(
		"is not on %s, one of the sidebar's own places",
		(pathname) => {
			shell(pathname, 'roomy');

			expect(arrow()).toBeNull();
		},
	);

	it.each([['/recipes/b_soba'], ['/foods/f_flour'], ['/imports/j_1']])(
		'is not on the phone layout at %s',
		(pathname) => {
			shell(pathname, 'phone');

			expect(arrow()).toBeNull();
		},
	);

	it.each([['/cook/b_soba'], ['/invite/abc'], ['/recover/abc'], ['/cookbook-invite/abc']])(
		'is not at %s, where the wide layout draws no sidebar',
		(pathname) => {
			shell(pathname, 'roomy');

			expect(arrow()).toBeNull();
		},
	);

	it('goes back, as the device would, when a page of Kamosu is behind this one', async () => {
		const back = vi.spyOn(history, 'back').mockImplementation(() => {});
		onEntry(7);
		noteArrival();
		// A recipe opened from Recipes: one entry further on.
		onEntry(8);
		shell('/recipes/b_soba', 'wide');

		// Taken over, so the link to the parent is not followed as well.
		expect(await tapTakenOver()).toBe(true);

		expect(back).toHaveBeenCalledOnce();
	});

	it('goes to the parent when Kamosu was opened on this very page', async () => {
		const back = vi.spyOn(history, 'back').mockImplementation(() => {});
		onEntry(7);
		noteArrival();
		shell('/recipes/b_soba', 'wide');

		expect(await tapTakenOver()).toBe(false);

		expect(back).not.toHaveBeenCalled();
	});

	it('still goes back after a reload, which keeps the pages behind it', async () => {
		const back = vi.spyOn(history, 'back').mockImplementation(() => {});
		vi.spyOn(performance, 'getEntriesByType').mockReturnValue([
			{ type: 'reload' } as PerformanceNavigationTiming,
		]);
		onEntry(7);
		noteArrival();
		onEntry(8);
		// The reload starts the app again on the second entry.
		noteArrival();
		shell('/recipes/b_soba', 'wide');

		expect(await tapTakenOver()).toBe(true);

		expect(back).toHaveBeenCalledOnce();
	});

	it('goes to the parent in a tab opened from another, whose visits are not its own', async () => {
		const back = vi.spyOn(history, 'back').mockImplementation(() => {});
		vi.spyOn(performance, 'getEntriesByType').mockReturnValue([
			{ type: 'navigate' } as PerformanceNavigationTiming,
		]);
		// What the new tab was handed: the first tab's visit, begun long before.
		sessionStorage.setItem(VISITS, '[3]');
		onEntry(8);
		noteArrival();
		shell('/recipes/b_soba', 'wide');

		expect(await tapTakenOver()).toBe(false);

		expect(back).not.toHaveBeenCalled();
	});

	it('never goes back out to another site that stands between two visits', async () => {
		const back = vi.spyOn(history, 'back').mockImplementation(() => {});
		const load = vi.spyOn(performance, 'getEntriesByType');
		const loaded = (type: string) =>
			load.mockReturnValue([{ type } as PerformanceNavigationTiming]);
		// A first visit, two pages long. Then another site, and a link back in.
		loaded('navigate');
		onEntry(7);
		noteArrival();
		loaded('navigate');
		onEntry(40);
		noteArrival();
		// Back to the first visit and forward again, as a loaded page each time.
		loaded('back_forward');
		onEntry(8);
		noteArrival();
		onEntry(40);
		noteArrival();
		shell('/recipes/b_soba', 'wide');

		expect(await tapTakenOver()).toBe(false);

		expect(back).not.toHaveBeenCalled();
	});

	it('still goes back inside an earlier visit returned to by the device back', async () => {
		const back = vi.spyOn(history, 'back').mockImplementation(() => {});
		const load = vi.spyOn(performance, 'getEntriesByType');
		load.mockReturnValue([{ type: 'navigate' } as PerformanceNavigationTiming]);
		onEntry(7);
		noteArrival();
		// The app is loaded afresh further on, which begins a second visit.
		onEntry(40);
		noteArrival();
		// Back, to the second page of the first.
		load.mockReturnValue([{ type: 'back_forward' } as PerformanceNavigationTiming]);
		onEntry(8);
		noteArrival();
		shell('/recipes/b_soba', 'wide');

		expect(await tapTakenOver()).toBe(true);

		expect(back).toHaveBeenCalledOnce();
	});
});

describe('as the window changes', () => {
	it('swaps the tab bar for the sidebar without drawing the screen again', async () => {
		const { rerender } = render(ShellTestHarness, {
			props: { pathname: '/recipes', room: 'phone' },
		});
		const main = screen.getByRole('main');

		await rerender({ pathname: '/recipes', room: 'roomy' });

		expect(places()).toEqual(SIDEBAR);
		// The same element, so whatever the cook typed or opened inside it stays.
		expect(screen.getByRole('main')).toBe(main);
	});
});
