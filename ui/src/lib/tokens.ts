/**
 * Reading the design tokens back out of the stylesheet the browser loaded.
 *
 * Kamosu's look is one generated stylesheet compiled from `ui/src/app.css`
 * (issue #80). The tokens page shows what is in it — and shows it by *asking
 * the browser*, never by holding a second copy, so the page cannot drift from
 * what every screen is actually drawn with.
 *
 * `@theme static` in app.css is what makes this possible: every token is
 * emitted into `:root` whether or not a utility currently uses it.
 */

/** The palette, in the order the visual identity names it. */
export const COLOUR_ORDER = [
	'--color-ground',
	'--color-ground-2',
	'--color-card',
	'--color-ink',
	'--color-ink-2',
	'--color-rule',
	'--color-accent',
	'--color-accent-2',
	'--color-on-accent',
	'--color-support',
	'--color-support-2',
	'--color-cook-ground',
	'--color-cook-ink',
	'--color-cook-ink-2',
	'--color-cook-rule',
	'--color-cook-panel',
	'--color-cook-accent',
	'--color-cook-on-accent',
] as const;

export const RADIUS_ORDER = ['--radius-sm', '--radius-md', '--radius-lg', '--radius-pill'] as const;

/**
 * The type scale, largest first — the ranking is spec (ADR 0011). Then the
 * cooking screen's sizes on the wide layout, largest first again.
 */
export const TYPE_ORDER = [
	'--text-step',
	'--text-title',
	'--text-panel-figure',
	'--text-shelf-heading',
	'--text-line',
	'--text-list-title',
	'--text-body',
	'--text-tile-title',
	'--text-step-reading',
	'--text-read',
	'--text-label',
	// The cooking screen on the wide layout, read from the counter (#197).
	// Its Step is larger than everything above.
	'--text-step-far',
	'--text-panel-figure-far',
	'--text-foot-far',
	'--text-step-reading-far',
	'--text-line-far',
	'--text-next-step',
	'--text-body-far',
	'--text-read-far',
	'--text-label-far',
] as const;

/** Zero, the seven steps, and the gutter. No other step exists to reach for. */
export const SPACING_ORDER = [
	'--spacing-0',
	'--spacing-1',
	'--spacing-2',
	'--spacing-3',
	'--spacing-4',
	'--spacing-6',
	'--spacing-8',
	'--spacing-12',
	'--spacing-gutter',
] as const;

/** One token's resolved value, as the browser computed it. Empty if undeclared. */
export function readToken(name: string, element: Element = document.documentElement): string {
	return getComputedStyle(element).getPropertyValue(name).trim();
}

export interface TypeStep {
	name: string;
	size: string;
	lineHeight: string;
	letterSpacing: string;
}

/** Every step of the scale, with the line height and tracking declared beside it. */
export function readTypeScale(): TypeStep[] {
	return TYPE_ORDER.map((name) => ({
		name,
		size: readToken(name),
		lineHeight: readToken(`${name}--line-height`),
		letterSpacing: readToken(`${name}--letter-spacing`),
	})).filter((step) => step.size !== '');
}

export function readSpacing(): { name: string; value: string }[] {
	return SPACING_ORDER.map((name) => ({ name, value: readToken(name) })).filter(
		(step) => step.value !== '',
	);
}
