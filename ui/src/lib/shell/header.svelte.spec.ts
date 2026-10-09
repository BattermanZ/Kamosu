/**
 * The header, at the seam (#102, #218).
 *
 * What this guards is #102's complaint: the header used to be a logo that
 * was secretly a settings button, because the only thing naming its destination
 * was hidden from everyone who can see. Since #218 the way in is a gear with no
 * word, so the sign is the icon and the name is the accessible one; what the
 * eye gets is still asked through `painted()`, which strips `.sr-only` first,
 * so a test cannot stay green on a word only a screen reader hears.
 *
 * The header takes no client. That is not an accident of testing but the design
 * (ADR 0027): nothing about a Kitchen reaches it, so no number of Kitchens can
 * change it. The Kitchen half of the criterion is proved where a Kitchen count
 * is real — on the Settings screen, in `settings.svelte.spec.ts`.
 */

import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import Header from './Header.svelte';
import { m } from '$lib/paraglide/messages';

function header(onSettings = false, bare = false) {
	render(Header, { props: { onSettings, bare } });
}

/** The gear, found the way a screen reader finds it: by the room it opens. */
function gear(): HTMLElement {
	return screen.getByRole('link', { name: m.open_settings() });
}

/**
 * What the eye gets: the element's text with every `sr-only` descendant
 * removed, whitespace flattened. A clone, so the rendered DOM is untouched.
 */
function painted(element: HTMLElement): string {
	const copy = element.cloneNode(true) as HTMLElement;
	copy.querySelectorAll('.sr-only').forEach((hidden) => hidden.remove());
	return (copy.textContent ?? '').replace(/\s+/g, ' ').trim();
}

describe('the header', () => {
	it('holds one way into Settings: a gear with no word, named for a reader who cannot see it', () => {
		header();

		expect(gear()).toHaveAttribute('href', '/settings');
		expect(painted(gear())).toBe('');
		expect(gear().querySelector('svg')).not.toBeNull();
	});

	it('is a 48px target, the size every control a thumb hits is', () => {
		header();

		expect(gear().className).toContain('h-12');
		expect(gear().className).toContain('w-12');
	});

	it('carries the mark, and the mark is not a second door', () => {
		header();

		// One control in the header, not two. The wordmark stopped being a link
		// in #102: it was the thing pretending to be a button.
		expect(screen.getAllByRole('link')).toHaveLength(1);
		expect(painted(screen.getByRole('banner'))).toContain(m.app_name());
	});

	it('paints the name and nothing else', () => {
		header();

		// Pinned whole, which is what forbids a Kitchen's name — or anything
		// else — quietly appearing here later. ADR 0027 rules out a switcher,
		// and filtering by Kitchen lives on the Recipes shelf (#62).
		expect(painted(screen.getByRole('banner'))).toBe(m.app_name());
	});

	it('scrolls away with the page rather than staying at the top (#218)', () => {
		header();

		expect(screen.getByRole('banner').className).not.toContain('sticky');
	});

	it('is the current page on Settings, and drops the expanding name there', () => {
		header(true);

		expect(gear()).toHaveAttribute('aria-current', 'page');
		// Naming both halves of an expanding pair on one screen is not a pair
		// (ADR 0012): on Settings the gear *is* the page.
		expect(gear().getAttribute('style') ?? '').not.toContain('view-transition-name');
	});

	it('is the other half of the expanding pair everywhere else', () => {
		header();

		// The same name the Settings screen puts on its title, which is what
		// makes the gear grow into the page rather than the screen swapping.
		expect(gear().getAttribute('style')).toContain('view-transition-name: settings');
		expect(gear().getAttribute('style')).toContain('view-transition-class: expanding');
		expect(gear()).not.toHaveAttribute('aria-current');
	});

	it('drawn bare, for somebody not yet in the app, holds the name and no door at all', () => {
		header(false, true);

		expect(screen.queryByRole('link')).toBeNull();
		expect(painted(screen.getByRole('banner'))).toBe(m.app_name());
	});
});
