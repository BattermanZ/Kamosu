/**
 * What the phone keeps, and under which name (#76, ADR 0013).
 *
 * Shared by the service worker, which fills these caches, and the app, which
 * looks in them — to say how old a kept recipe is, and to know what the library
 * fill still owes. One module, so the two can never disagree about a key.
 */

import { READS, type OperationName } from '$lib/api/catalogue';

/** Every Operation answer the phone keeps. Emptied whenever the Session changes. */
export const READS_CACHE = 'kamosu-reads';
/**
 * Display Copies. A Photograph is known by its contents (ADR 0017), so these
 * never go stale — but they are a Session's as much as its reads, and go with it.
 */
export const PHOTOGRAPHS_CACHE = 'kamosu-photographs';
/** The stylesheet, fonts and icons the binary serves beside the app. */
export const ASSETS_CACHE = 'kamosu-assets';
/**
 * The worker's own notes, kept apart from what it answers with: a browser
 * stops an idle worker within a minute, and what it remembers in memory goes
 * with it.
 */
export const NOTES_CACHE = 'kamosu-notes';
/** The header a kept answer carries: when it was kept, in milliseconds. */
export const KEPT_HEADER = 'x-kamosu-kept';

/**
 * How a read is answered.
 *
 * - `phone-first`: from the phone at once, refreshed behind. What makes opening
 *   Kamosu instant, and what nearly every read is.
 * - `server-first`: from the server, the phone only when there is none. For a
 *   read whose whole value is being current.
 * - `never`: straight through, never kept.
 */
export type Policy = 'phone-first' | 'server-first' | 'never';

/**
 * The reads that are not phone-first, each for a reason. Everything else the
 * Catalogue declares a read (`READS`) is phone-first; every write goes straight
 * to the server and is never replayed, which is the whole of ADR 0013's line.
 */
const EXCEPTIONS: Partial<Record<OperationName, Policy>> = {
	// Asked over and over while a Job runs. A kept answer is a progress bar
	// that never moves.
	get_job: 'never',
	list_jobs: 'never',
	// A whole recipe as a zip, asked for once to be saved somewhere else.
	export_bundle: 'never',
	// Where the cook is. With two devices, the last one moved on is where the
	// cook is (ADR 0010), so an old answer would put them back a step.
	get_current_attempt: 'server-first',
	// What has been brought in (#108). Its whole value is being current, and a
	// kept answer is wrong in both directions at the moments it is read: the
	// import you just ran is missing, and a ledger you have just forgotten is
	// still counted beside the very sentence saying it is gone. Found in live
	// acceptance, where a new source did not appear until the second load.
	list_imports: 'server-first',
	// Which Kitchens you cook in. It decides whether a new recipe asks whose
	// it is (#111), and a kept answer is wrong at exactly that moment when
	// the Kitchens changed somewhere else — another device, or another member
	// removing you. (A change made on this phone already goes past the kept
	// answer.) A Kitchen joined elsewhere is missing, so the cook is not asked
	// and the recipe lands in the Home Kitchen silently, which is the thing
	// #111 removed; one left is still offered. Found in live acceptance.
	list_kitchens: 'server-first',
	// Who can get in. Ending a Session on another device must show at once.
	list_sessions: 'server-first',
	list_access_keys: 'server-first',
	get_share_link: 'server-first',
	list_backups: 'server-first',
};

const READ_SET = new Set<string>(READS);

/** How the Operation called `operation` is answered, or `undefined` for a write. */
export function policyFor(operation: string): Policy | undefined {
	if (!READ_SET.has(operation)) return undefined;
	return EXCEPTIONS[operation as OperationName] ?? 'phone-first';
}

/**
 * The same value always written the same way — keys sorted at every depth —
 * so `{a, b}` and `{b, a}` are one answer and not two.
 */
export function canonical(value: unknown): string {
	if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`;
	if (value !== null && typeof value === 'object') {
		const entries = Object.entries(value as Record<string, unknown>)
			.filter(([, v]) => v !== undefined)
			.sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
		return `{${entries.map(([k, v]) => `${JSON.stringify(k)}:${canonical(v)}`).join(',')}}`;
	}
	return JSON.stringify(value ?? null);
}

/**
 * The name an answer is kept under. Operations are POSTs, and a cache keys on
 * GET requests, so each (Operation, input) pair gets a URL of its own that
 * nothing on the server answers.
 */
export function readKey(origin: string, operation: string, input: unknown): string {
	return `${origin}/__kamosu/read/${operation}?${encodeURIComponent(canonical(input ?? {}))}`;
}

/**
 * When an answer was kept on this phone, if it was. The app asks this to say
 * how old a recipe it is showing is; outside a secure page there is no cache
 * to ask and the answer is simply `undefined`.
 */
export async function keptAt(operation: OperationName, input: unknown): Promise<Date | undefined> {
	if (typeof caches === 'undefined') return undefined;
	try {
		const kept = await caches.match(readKey(location.origin, operation, input), {
			cacheName: READS_CACHE,
		});
		const at = Number(kept?.headers.get(KEPT_HEADER));
		return at ? new Date(at) : undefined;
	} catch {
		return undefined;
	}
}

/**
 * Whether one attempt to call an Operation says the server is there (#76).
 * `undefined` when it says nothing either way.
 *
 * Every Operation answers in Kamosu's JSON envelope, refusals included, so
 * that is the test: anything else is something standing in front of Kamosu
 * answering for it — the proxy's 502 for a Kamosu that is down (with HTTPS
 * required, how "the server at home is off" usually looks), or a hotel wifi's
 * login page.
 *
 * - A request the browser cancelled on purpose — the page moved on — says nothing.
 * - An answer the phone kept says nothing about the server.
 */
export function serverAnswered(outcome: unknown): boolean | undefined {
	if (!(outcome instanceof Response)) {
		// By name: a DOMException is not an Error everywhere.
		return (outcome as { name?: unknown } | null)?.name === 'AbortError' ? undefined : false;
	}
	if (outcome.headers.has(KEPT_HEADER)) return undefined;
	return outcome.headers.get('content-type')?.includes('application/json') ?? false;
}
