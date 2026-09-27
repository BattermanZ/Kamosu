import type { ClientInit } from '@sveltejs/kit';
import { holdTheInstallOffer } from '$lib/offline/device.svelte';

/**
 * Runs once, before any screen is loaded: where the app takes over the
 * browser's install offer (#173) from `app.html`, which caught the first one,
 * and listens for the rest itself.
 */
export const init: ClientInit = () => {
	holdTheInstallOffer();
};
