/**
 * Settings' four pages (#221, option 2 of the audit of 9 October 2026).
 *
 * Settings was one page 8,000px tall on a phone, with fourteen sections. It
 * is a menu of four rows now, each opening a page of its own that holds the
 * sections it names. The sections' own words did not change; this is where
 * they live, not what they say.
 */

import { m } from '$lib/paraglide/messages';

export type SettingsPage = 'account' | 'device' | 'kitchens' | 'instance';

export interface PageEntry {
	page: SettingsPage;
	href: string;
	title: () => string;
	/** What the page holds, in one line under its name on the menu. */
	inside: () => string;
}

export const PAGES: PageEntry[] = [
	{
		page: 'account',
		href: '/settings/account',
		title: m.settings_account,
		inside: m.settings_account_inside,
	},
	{
		page: 'device',
		href: '/settings/device',
		title: m.settings_device,
		inside: m.settings_device_inside,
	},
	{
		page: 'kitchens',
		href: '/settings/kitchens',
		title: m.settings_kitchens_page,
		inside: m.settings_kitchens_inside,
	},
	{
		page: 'instance',
		href: '/settings/instance',
		title: m.settings_instance,
		inside: m.settings_instance_inside,
	},
];

export const titleOf = (page: SettingsPage): string =>
	PAGES.find((entry) => entry.page === page)!.title();
