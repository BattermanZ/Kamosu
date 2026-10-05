/**
 * Which navigation the shell draws, by how much room the window has (ADR 0044,
 * #194): the sidebar on the wide layout, the header and tab bar on the phone,
 * and neither where a screen is drawn bare.
 *
 * The two navigations carry the same accessible name, so they are told apart
 * by what only one of them has: the sidebar lists Settings, and the header is
 * the page's banner.
 */

import { beforeEach, describe, expect, it } from 'vitest';
import { render, screen, waitFor, within } from '@testing-library/svelte';
import ShellTestHarness from './ShellTestHarness.svelte';
import { heardWhetherSignedIn, signingIn } from '$lib/shell/signing-in.svelte';
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
