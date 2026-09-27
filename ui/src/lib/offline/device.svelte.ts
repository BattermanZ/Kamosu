/**
 * What this phone can and cannot do right now (#76).
 *
 * Each fact is read from the browser the same way everywhere, so the cards,
 * Settings and the buttons that reword themselves can never disagree about it.
 */

import { untrack } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import type { KamosuClient } from '$lib/api/catalogue';
import { putAway } from './put-away';
import { serverAnswered } from './reads';
import type { WorkerMessage } from './worker';

/** The facts the cards are chosen from. A test hands in its own. */
export interface Device {
	/**
	 * `https://` or `localhost`. Without it a browser runs no service worker,
	 * so nothing can be kept for offline at all.
	 */
	secure: boolean;
	/** Opened from the Home Screen rather than in a browser tab. */
	installed: boolean;
	/** An iPhone or iPad, where adding to the Home Screen is Share, then Add. */
	apple: boolean;
	/**
	 * Whether the phone is on wifi. `undefined` where the browser will not say,
	 * which is every iPhone: Safari never shipped the API for it, so there the
	 * first fill waits for a tap rather than for wifi.
	 */
	wifi: () => boolean | undefined;
}

interface NetworkInformation extends EventTarget {
	type?: string;
}

const network = (): NetworkInformation | undefined =>
	typeof navigator === 'undefined'
		? undefined
		: (navigator as Navigator & { connection?: NetworkInformation }).connection;

/** This browser, read once. */
export function thisDevice(): Device {
	const apple =
		/iPhone|iPad|iPod/.test(navigator.userAgent) ||
		// iPadOS asks for the desktop site and says it is a Mac.
		(navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1);
	return {
		secure: window.isSecureContext === true,
		installed:
			(typeof matchMedia === 'function' && matchMedia('(display-mode: standalone)').matches) ||
			(navigator as Navigator & { standalone?: boolean }).standalone === true,
		apple,
		wifi: () => {
			const type = network()?.type;
			return type === undefined ? undefined : type === 'wifi' || type === 'ethernet';
		},
	};
}

/**
 * A Chromium browser's offer to install Kamosu (#173): Chrome, Edge and
 * Samsung Internet send one when the page could be installed. No iPhone
 * browser does, and neither does Firefox.
 */
export interface InstallOffer extends Event {
	/** Opens the browser's own install dialog. Works once per offer. */
	prompt(): Promise<unknown>;
	userChoice: Promise<{ outcome: 'accepted' | 'dismissed' }>;
}

/**
 * The offer being held, and whether Kamosu was installed from this page. The
 * offer is gone once it has been used, since its dialog opens only once, and
 * once Kamosu is installed.
 */
export const install: { offer: InstallOffer | undefined; accepted: boolean } = $state({
	offer: undefined,
	accepted: false,
});

/**
 * Hold the browser's install offer from the moment the app starts. The
 * browser sends it once, usually before the app has even started, so
 * `app.html` catches it first and this takes it from there; a card listening
 * for itself would miss it. Keeping it also stops Chrome's own install bar,
 * which would compete with the card. Returns what stops listening and forgets
 * the offer; the app never stops, so only the tests call it, to start each one
 * clean.
 */
export function holdTheInstallOffer(): () => void {
	const offered = (event: Event) => {
		event.preventDefault();
		// Chrome offers again the moment its dialog is turned down. The person
		// has just said no, so they keep the written steps, not the button.
		if (!turnedDown) install.offer = event as InstallOffer;
	};
	if (window.kamosuInstallOffer) offered(window.kamosuInstallOffer);
	delete window.kamosuInstallOffer;
	addEventListener('beforeinstallprompt', offered);
	addEventListener('appinstalled', installed);
	return () => {
		removeEventListener('beforeinstallprompt', offered);
		removeEventListener('appinstalled', installed);
		install.offer = undefined;
		install.accepted = false;
		turnedDown = false;
	};
}

/** The browser's install dialog was turned down on this page. */
let turnedDown = false;

/**
 * Kamosu was installed, from the card, from Settings or from the browser's own
 * menu. The card is put away as "Not now" puts it away, so it stays away in
 * this tab after a reload too, and Settings still says how to do it again.
 */
function installed(): void {
	install.offer = undefined;
	install.accepted = true;
	putAway('install');
}

/**
 * Open the browser's install dialog, from a tap. Either way the offer is used
 * up, so a person who turns it down is left with the written steps.
 */
export async function installKamosu(): Promise<void> {
	const offer = install.offer;
	install.offer = undefined;
	if (!offer) return;
	try {
		await offer.prompt();
		if ((await offer.userChoice).outcome === 'accepted') installed();
		else turnedDown = true;
	} catch {
		// Already used, or the browser changed its mind: the steps are still there.
	}
}

/** Call `then` whenever the phone moves between networks, where it says so. */
export function onNetworkChange(then: () => void): () => void {
	const info = network();
	info?.addEventListener('change', then);
	return () => info?.removeEventListener('change', then);
}

/**
 * Whether the server answered the last time anything asked it. The browser's
 * own `navigator.onLine` is only a hint — true on a wifi with no internet
 * behind it, and true when the phone has signal but the Kamosu at home is
 * down — so every Operation's outcome is judged: the service worker reports
 * each one it sends, and the page's transport each one it sees.
 */
export const reach = $state({ server: true, lost: 0 });

/**
 * Note whether the server answered. The one place `reach` is written, from
 * the worker's reports and the page's own requests alike. Counts each time the
 * server is lost, where it is lost.
 */
export function reached(server: boolean): void {
	if (reach.server && !server) reach.lost += 1;
	reach.server = server;
}

/** What the page's own transport saw, judged the way the worker judges it. */
export function noteReach(outcome: unknown): void {
	const answered = serverAnswered(outcome);
	if (answered !== undefined) reached(answered);
}

/**
 * Whether the server can be reached: the browser thinks so, and the last
 * request did. Made inside a component, and follows both for as long as that
 * component lives.
 *
 * Every wording built on it is about what waits for the server rather than a
 * claim about the network, because even the two together are a judgement.
 */
export class Online {
	#browser = $state(true);
	#browserLost = $state(0);
	readonly current = $derived(this.#browser && reach.server);
	/**
	 * Counts each time the server went out of reach, so "this spell" has a
	 * name. Counted where it happens rather than by watching `current`, which
	 * would miss a drop and a return that both land before it is looked at.
	 */
	readonly spell = $derived(this.#browserLost + reach.lost);

	constructor() {
		$effect(() => {
			const read = () => {
				if (this.#browser && !navigator.onLine) this.#browserLost += 1;
				this.#browser = navigator.onLine;
			};
			read();
			addEventListener('online', read);
			addEventListener('offline', read);
			return () => {
				removeEventListener('online', read);
				removeEventListener('offline', read);
			};
		});
	}
}

/**
 * How many times each read has come back different from what a page was
 * shown. The service worker answers from the phone first and asks the server
 * behind; when the server's answer differs it says so, and a screen reading
 * `refreshed.get('get_recipe')` in an effect reads again.
 */
export const refreshed = new SvelteMap<string, number>();

/**
 * Listen to the service worker, once, from the root layout: for reads that
 * have changed, and for whether the server is being reached.
 */
export function listenToTheWorker(): () => void {
	if (typeof navigator === 'undefined' || !('serviceWorker' in navigator)) return () => {};
	const heard = (event: MessageEvent) => {
		const message = event.data as Partial<WorkerMessage> | undefined;
		if (message?.type === 'kamosu:refreshed' && message.operation) {
			refreshed.set(message.operation, (refreshed.get(message.operation) ?? 0) + 1);
		} else if (message?.type === 'kamosu:reach' && typeof message.reached === 'boolean') {
			reached(message.reached);
		}
	};
	navigator.serviceWorker.addEventListener('message', heard);
	return () => navigator.serviceWorker.removeEventListener('message', heard);
}

/** While the server is out of reach, how often to ask whether it is back. */
const RETRY_MS = 20_000;

/**
 * While the server is out of reach, ask it something cheap now and then, so
 * the page learns it is back without waiting for the person to press a button
 * that says it cannot work. Through the client, with or without a worker, so
 * the answer is judged like any other.
 */
export function retryWhileUnreachable(kamosu: KamosuClient): () => void {
	const retry = setInterval(() => {
		if (reach.server || !navigator.onLine) return;
		void kamosu.instanceStatus().catch(() => {});
	}, RETRY_MS);
	return () => clearInterval(retry);
}

/**
 * A number that goes up each time any of these reads comes back different
 * from what a page was shown — the service worker's refresh behind, or the
 * outbox having sent what it held (#77). A screen reads it in the effect that
 * loads it, so it loads again. It does not go up the first time it is read.
 */
export function rereads(...operations: string[]): { readonly count: number } {
	const total = () =>
		operations.reduce((sum, operation) => sum + (refreshed.get(operation) ?? 0), 0);
	const from = untrack(total);
	return {
		get count() {
			return total() - from;
		},
	};
}
