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
import { went } from '../testing/navigation';
import { forgetSlash } from '$lib/shell/slash';
import type { Room } from '$lib/room.svelte';

function shell(pathname: string, room: Room, holds?: 'account' | 'story' | 'shelf') {
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

describe('the tab', () => {
	it("reads the page's name, so history and a bookmark say what they hold (#224)", async () => {
		shell('/recipes', 'phone', 'shelf');

		await waitFor(() => expect(document.title).toBe(`${m.recipes_title()} · ${m.app_name()}`));
	});
});

describe('on the phone layout', () => {
	it('keeps the header and the four tabs, and lists no Settings among them', () => {
		shell('/', 'phone');

		expect(places()).toEqual(TABS);
		expect(screen.getByRole('banner')).toHaveTextContent(m.app_name());
	});

	it('reaches Settings by a gear in the header, named for a reader who cannot see it (#218)', () => {
		shell('/', 'phone');

		const gear = within(screen.getByRole('banner')).getByRole('link', { name: m.open_settings() });
		expect(gear).toHaveAttribute('href', '/settings');
	});

	it("draws the bar in the rail's look, the current place lit (#218)", () => {
		shell('/recipes', 'phone');

		const nav = navigation();
		expect(nav?.className).toContain('bg-accent');
		const current = within(nav!).getByRole('link', { current: 'page' });
		expect(current).toHaveAttribute('href', '/recipes');
		expect(current.className).toContain('bg-cook-ground');
		expect(current.className).toContain('border-t-3');
	});

	it.each([
		['/', 'account' as const],
		['/invite/abc', 'account' as const],
		['/recover/abc', 'account' as const],
		['/cookbook-invite/abc', 'account' as const],
	])(
		'draws no tabs and no gear at %s for somebody not yet in the app, only the name (#218)',
		async (pathname, holds) => {
			heardWhetherSignedIn(false);
			shell(pathname, 'phone', holds);

			await waitFor(() => expect(screen.getByRole('banner')).toBeInTheDocument());
			expect(navigation()).toBeNull();
			expect(screen.getByRole('banner')).toHaveTextContent(m.app_name());
			expect(within(screen.getByRole('banner')).queryByRole('link')).toBeNull();
		},
	);

	it('draws none at / on a device that has heard nothing, so a stranger never sees them', () => {
		signingIn.signedIn = undefined;
		shell('/', 'phone');

		expect(navigation()).toBeNull();
		expect(within(screen.getByRole('banner')).queryByRole('link')).toBeNull();
	});

	it('brings the tabs and the gear back once the form is gone', async () => {
		const { rerender } = render(ShellTestHarness, {
			props: { pathname: '/import', room: 'phone', holds: 'account' },
		});

		await waitFor(() => expect(navigation()).toBeNull());

		await rerender({ pathname: '/import', room: 'phone', holds: undefined });
		await waitFor(() => expect(places()).toEqual(TABS));
		expect(
			within(screen.getByRole('banner')).getByRole('link', { name: m.open_settings() }),
		).toBeInTheDocument();
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
 * The back arrow on a page reached from another page (ADR 0044, #195, #219):
 * on every page the navigation does not list, on both layouts.
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
		['Foods', '/foods', '/settings/kitchens'],
		['Brought in', '/imports', '/settings/instance'],
		['the Operator screen', '/operator', '/settings/instance'],
		["one of Settings' pages", '/settings/account', '/settings'],
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

	it.each([
		['/recipes/b_soba', '/recipes'],
		['/foods/f_flour', '/foods'],
		['/imports/j_1', '/imports'],
		['/imports', '/settings/instance'],
		['/settings/device', '/settings'],
	])(
		"is on the phone layout too at %s, in the page's flow and sticking to the top (#219)",
		(pathname, parent) => {
			shell(pathname, 'phone');

			expect(arrow()).toHaveAttribute('href', parent);
			expect(arrow()?.className).toContain('back-arrow-phone');
			expect(arrow()?.className).not.toContain('fixed');
		},
	);

	it.each([['/'], ['/recipes'], ['/shopping'], ['/cooked'], ['/settings']])(
		"is not on %s on the phone either, one of the bar's places or the gear's",
		(pathname) => {
			shell(pathname, 'phone');

			expect(arrow()).toBeNull();
		},
	);

	it('is not on the phone for somebody not yet in the app', async () => {
		heardWhetherSignedIn(false);
		shell('/invite/abc', 'phone', 'account');

		await waitFor(() => expect(screen.getByRole('banner')).toBeInTheDocument());
		expect(arrow()).toBeNull();
	});

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

describe('the / key', () => {
	beforeEach(forgetSlash);

	const searchBox = () => screen.getByRole('searchbox', { name: m.recipes_search() });
	/** Presses `/` there. False where Kamosu took the key for itself. */
	const slash = (on: Element, how: KeyboardEventInit = {}) =>
		fireEvent.keyDown(on, { key: '/', ...how });

	it.each<Room>(['phone', 'roomy'])(
		'puts the caret in the search box on Recipes, in a %s window',
		async (room) => {
			shell('/recipes', room, 'shelf');
			expect(searchBox()).not.toHaveFocus();

			expect(await slash(document.body)).toBe(false);

			expect(searchBox()).toHaveFocus();
			expect(went).not.toHaveBeenCalled();
		},
	);

	it('opens Recipes from anywhere else, and the search box takes the caret as it arrives', async () => {
		const { rerender } = render(ShellTestHarness, {
			props: { pathname: '/settings', room: 'roomy' },
		});

		expect(await slash(document.body)).toBe(false);
		expect(went).toHaveBeenCalledExactlyOnceWith('/recipes');

		await rerender({ pathname: '/recipes', room: 'roomy', holds: 'shelf' });
		expect(searchBox()).toHaveFocus();
	});

	it('leaves the caret alone on a Recipes that was opened some other way', () => {
		shell('/recipes', 'roomy', 'shelf');

		expect(searchBox()).not.toHaveFocus();
	});

	it('types a slash inside a field, the search box included', async () => {
		shell('/recipes', 'roomy', 'shelf');
		const field = document.createElement('textarea');
		screen.getByRole('main').append(field);

		for (const typedInto of [searchBox(), field]) {
			typedInto.focus();
			expect(await slash(typedInto)).toBe(true);
			expect(typedInto).toHaveFocus();
		}
		expect(went).not.toHaveBeenCalled();
	});

	it('is taken from a tick box, which no slash can be typed into', async () => {
		shell('/settings', 'roomy');
		const tick = document.createElement('input');
		tick.type = 'checkbox';
		screen.getByRole('main').append(tick);

		expect(await slash(tick)).toBe(false);
		expect(went).toHaveBeenCalledExactlyOnceWith('/recipes');
	});

	it('is the key a French keyboard writes with Shift, and no shortcut held with another key', async () => {
		shell('/settings', 'roomy');

		for (const held of [{ ctrlKey: true }, { metaKey: true }, { altKey: true }]) {
			expect(await slash(document.body, held)).toBe(true);
		}
		expect(went).not.toHaveBeenCalled();

		expect(await slash(document.body, { shiftKey: true })).toBe(false);
		expect(went).toHaveBeenCalledExactlyOnceWith('/recipes');
	});

	it('waits while a sheet is open over the page', async () => {
		shell('/settings', 'roomy');
		const sheet = document.createElement('div');
		sheet.setAttribute('role', 'dialog');
		document.body.append(sheet);

		expect(await slash(sheet)).toBe(true);
		sheet.remove();

		expect(went).not.toHaveBeenCalled();
	});

	it.each([
		['the cooking screen', '/cook/b_soba', undefined],
		['the story', '/about', undefined],
		['an Invite', '/invite/abc', undefined],
		['the account form', '/', 'account' as const],
	])('does nothing on %s', async (_where, pathname, holds) => {
		shell(pathname, 'roomy', holds);
		if (holds === 'account') await waitFor(() => expect(navigation()).toBeNull());

		expect(await slash(document.body)).toBe(true);

		expect(went).not.toHaveBeenCalled();
	});
});
