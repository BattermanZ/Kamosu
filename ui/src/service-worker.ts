/// <reference no-default-lib="true"/>
/// <reference lib="esnext" />
/// <reference lib="webworker" />
/// <reference types="@sveltejs/kit" />

/**
 * The service worker: what puts the cookbook on the phone (#76, ADR 0013).
 *
 * SvelteKit bundles and registers this file itself. Everything it decides is
 * in `$lib/offline/worker.ts`, where a test can reach it; this file only hands
 * the browser's events over, and fills the app's own cache on install.
 *
 * A browser runs this only on a secure page — `https://`, or `localhost` — so
 * over plain `http://` none of it exists, which is what the app's "This
 * connection isn't secure" card is there to say.
 */

import { build, files, version } from '$service-worker';
import { createWorker } from '$lib/offline/worker';
import { ASSETS_CACHE, READS_CACHE } from '$lib/offline/reads';

const self = globalThis.self as unknown as ServiceWorkerGlobalScope;

/** This build's own files. Named by version, so an upgrade replaces them whole. */
const SHELL = `kamosu-shell-${version}`;
/**
 * What SvelteKit compiled, and nothing else. It is empty in development, which
 * is how the worker knows not to answer a navigation itself (#95).
 */
const BUILT = build;
/**
 * What the shell cache holds: the compiled app, plus `ui/static/`'s files,
 * which are served as they are and are worth having on the phone in either
 * build. That second half is why this cannot stand in for `BUILT`.
 */
const PRECACHED = [...build, ...files];

/**
 * What the binary serves beside the app and every screen needs: the stylesheet
 * (whose fonts are read out of it, so no list of them lives here to fall
 * behind) and the mark in the header.
 */
async function keepAssets(): Promise<void> {
	const store = await caches.open(ASSETS_CACHE);
	const sheet = await fetch('/assets/app.css');
	if (sheet.status !== 200) return;
	const css = await sheet.clone().text();
	await store.put('/assets/app.css', sheet);
	const named = [...css.matchAll(/url\(\s*["']?(\/assets\/[^"')]+)["']?\s*\)/g)].map((m) => m[1]);
	await Promise.allSettled(
		[...new Set([...named, '/assets/img/kamosu-mark.svg'])].map((path) => store.add(path)),
	);
}

self.addEventListener('install', (event) => {
	event.waitUntil(
		(async () => {
			const store = await caches.open(SHELL);
			// The app, all three languages included — Paraglide compiles every
			// phrase into these chunks, so precaching them is precaching fr, en
			// and es alike (ADR 0013).
			await store.addAll([...PRECACHED, '/']);
			// The binary's assets are worth having, but never worth failing the
			// install over.
			await keepAssets().catch(() => undefined);
			await self.skipWaiting();
		})(),
	);
});

self.addEventListener('activate', (event) => {
	event.waitUntil(
		(async () => {
			let upgraded = false;
			for (const key of await caches.keys()) {
				if (key.startsWith('kamosu-shell-') && key !== SHELL) {
					await caches.delete(key);
					upgraded = true;
				}
			}
			// **An upgrade throws the kept reads away.** A kept read is a copy
			// of an answer the Catalogue of *that* build described, and an
			// upgrade is free to change an Operation's shape — #86 moved a
			// Shopping List's lines from an index to a path. Code written for
			// the new shape reading a copy in the old one is the crash that
			// cannot be caught, because nothing about the copy says which
			// build wrote it.
			//
			// Discarding them costs almost nothing and the alternative is a
			// shim per changed field, carried for ever. The database is the
			// truth and this was only ever a copy (ADR 0003); the phone
			// downloaded this worker moments ago, so it has a network, and
			// `missing()` fetches back whatever the library still owes.
			// Photographs are a separate cache and are untouched.
			if (upgraded) await caches.delete(READS_CACHE);
			// Take the page that installed this at once, so a first visit's reads
			// start being kept without waiting for a second one.
			await self.clients.claim();
		})(),
	);
});

/**
 * One for the worker's life: it remembers when this phone last wrote, which
 * decides whether a kept read may still be shown first.
 */
const worker = createWorker({
	caches,
	fetch: (input, init) => fetch(input, init),
	now: () => Date.now(),
	origin: self.location.origin,
	shell: SHELL,
	built: BUILT,
	precached: PRECACHED,
	tell: (message) => {
		void self.clients
			.matchAll({ type: 'window' })
			.then((pages) => pages.forEach((page) => page.postMessage(message)));
	},
});

self.addEventListener('fetch', (event) => {
	const answer = worker.handle(event.request, (work) => event.waitUntil(work));
	if (answer) event.respondWith(answer);
});
