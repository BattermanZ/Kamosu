/**
 * The header, at the seam (#102).
 *
 * What this guards is the ticket's complaint: the header used to be a logo that
 * was secretly a settings button, because the only thing naming its destination
 * was hidden from everyone who can see.
 *
 * That makes `toHaveTextContent` and `getByRole(name)` both useless here, and
 * the trap is worth naming because the first draft of this file fell into it:
 * Tailwind's `sr-only` hides text from the screen while leaving it in the DOM,
 * so `textContent` reads the visible word and the screen-reader word alike. A
 * test written on `textContent` stays green if you move *You* into the
 * `sr-only` span — which is #102, reproduced. Everything below therefore asks
 * `painted()`, which strips `.sr-only` first.
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

function header(onSettings = false) {
	render(Header, { props: { onSettings } });
}

/** The card, found the way a person finds it: by the word written on it. */
function card(): HTMLElement {
	return screen.getByRole('link', { name: new RegExp(m.header_you(), 'i') });
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
	it('names the way into Settings in text a sighted person can read', () => {
		header();

		expect(painted(card())).toBe(m.header_you());
		expect(card()).toHaveAttribute('href', '/settings');
	});

	it('names where the card goes, for a reader who cannot see it is a card', () => {
		header();

		// The visible word is *You*; the accessible name adds the room it opens.
		expect(card()).toHaveAccessibleName(new RegExp(m.open_settings(), 'i'));
		expect(painted(card())).not.toContain(m.open_settings());
	});

	it('carries the mark, and the mark is not a second door', () => {
		header();

		// One control in the header, not two. The wordmark stopped being a link
		// in #102: it was the thing pretending to be a button.
		expect(screen.getAllByRole('link')).toHaveLength(1);
		expect(painted(screen.getByRole('banner'))).toContain(m.app_name());
	});

	it('paints the mark and the card, and nothing else', () => {
		header();

		// Pinned whole, which is what forbids a Kitchen's name — or anything
		// else — quietly appearing here later. ADR 0027 rules out a switcher,
		// and filtering by Kitchen lives on the Recipes shelf (#62).
		expect(painted(screen.getByRole('banner'))).toBe(`${m.app_name()} ${m.header_you()}`);
	});

	it('is the current page on Settings, and drops the expanding name there', () => {
		header(true);

		expect(card()).toHaveAttribute('aria-current', 'page');
		// Naming both halves of an expanding pair on one screen is not a pair
		// (ADR 0012): on Settings the card *is* the page.
		expect(card().getAttribute('style') ?? '').not.toContain('view-transition-name');
	});

	it('is the other half of the expanding pair everywhere else', () => {
		header();

		// The same name the Settings screen puts on its title, which is what
		// makes the card grow into the page rather than the screen swapping.
		expect(card().getAttribute('style')).toContain('view-transition-name: settings');
		expect(card().getAttribute('style')).toContain('view-transition-class: expanding');
		expect(card()).not.toHaveAttribute('aria-current');
	});
});
