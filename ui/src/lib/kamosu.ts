/**
 * How a screen gets hold of Kamosu.
 *
 * The layout puts one client into context and every screen takes it out again.
 * That single indirection is the screen seam: in the app the client is the real
 * one, in a test it is the Catalogue-derived stand-in, and a screen cannot tell
 * — nor should it be able to.
 */

import { getContext, setContext } from 'svelte';
import type { KamosuClient } from './api/catalogue';
import { createClient, httpTransport } from './api/client';
import { noteReach } from './offline/device.svelte';

const KEY = Symbol('kamosu');

/** How the client is held: an accessor, so context always reads the current one. */
type Provided = () => KamosuClient;

/** Put a client into context. The layout does this once, at the root. */
export function provideKamosu(client: Provided): void {
	setContext(KEY, client);
}

/**
 * The client for this screen. Tests render a screen inside a provider holding
 * a stand-in; the app renders it inside the layout holding the real one.
 */
export function useKamosu(): KamosuClient {
	const client = getContext<Provided | undefined>(KEY)?.();
	if (!client) {
		throw new Error(
			'no Kamosu client in context — a screen must be rendered inside the root layout, or inside <Kamosu client={standIn().client}> in a test',
		);
	}
	return client;
}

/**
 * The real client: same origin, no configuration. In development vite proxies
 * `/api` to the binary on 5266, and in the built app the binary serves both.
 */
export const realKamosu = (): KamosuClient => createClient(httpTransport({ observe: noteReach }));
