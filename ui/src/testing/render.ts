/**
 * Rendering a screen the way the app renders it: inside the provider, holding a
 * client. In the app that client is the real one; here it is the stand-in built
 * from the Catalogue, and the screen cannot tell the difference — which is the
 * entire point of the seam.
 */

import { render } from '@testing-library/svelte';
import type { Component } from 'svelte';
import Harness from './Harness.svelte';
import { standIn, type Answers, type StandIn } from '$lib/api/stand-in';

/** A screen: a route component, which takes no props — its client arrives by context. */
export type ScreenComponent = Component<Record<string, never>>;

export interface Rendered {
	/** The stand-in the screen was given: what it asked for, and what it was told. */
	kamosu: StandIn;
}

/**
 * Render one screen against a Catalogue-derived stand-in.
 *
 * ```ts
 * const { kamosu } = renderScreen(Settings, {
 *   instance_status: { version: '0.1.0', setup_complete: true }
 * });
 * ```
 */
export function renderScreen(component: ScreenComponent, answers: Answers = {}): Rendered {
	const kamosu = standIn(answers);
	render(Harness, { props: { component, client: kamosu.client } });
	return { kamosu };
}
