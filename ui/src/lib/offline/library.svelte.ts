/**
 * The library on the phone (#76, ADR 0013).
 *
 * *Your cookbook is on your phone, the rest of the instance is on the server.*
 * Every recipe of every Kitchen the Person cooks in is read once, the way the
 * recipe screen reads it, with its pictures — and because the service worker
 * keeps every read that passes through it, reading is all filling takes.
 *
 * The first fill waits: a library arriving over mobile data by surprise is
 * the thing ADR 0013 rules out. Where the browser can tell it is on wifi it
 * starts by itself; an iPhone cannot tell, so there it waits for a tap. After
 * that, each start only fetches what the phone does not hold yet.
 */

import { getContext, setContext } from 'svelte';
import { SvelteSet } from 'svelte/reactivity';
import type { KamosuClient } from '$lib/api/catalogue';
import { OperationError } from '$lib/api/client';
import { PHOTOGRAPHS_CACHE, keptAt } from './reads';
import { forgetEverythingHeld } from './outbox';

/**
 * What one recipe costs the phone, measured on the dev library (2026-09-19,
 * 12 recipes): a card Display Copy averaged 34 KB and a page one 70 KB, and a
 * recipe with its Thread 5 KB. Only used to say *about* how much a first fill
 * is before it starts; nothing is decided by it.
 */
const BYTES_PER_PHOTOGRAPH = 104_000;
const BYTES_PER_RECIPE = 6_000;

/** How many recipes are read at once. Enough to be quick, few enough to be polite. */
const AT_ONCE = 3;

/** What survives on the device between visits. */
interface Kept {
	/** When the whole library last finished arriving; absent before the first fill has. */
	filledAt?: number;
	/**
	 * Which recipes are the Person's Kitchens', as last asked — saved the moment
	 * it is known rather than when a fill ends, because offline it is the only
	 * way to tell a recipe of theirs from one they only opened.
	 */
	held: string[];
}

const KEY = 'kamosu.library';

function load(): Kept | undefined {
	try {
		const raw = localStorage.getItem(KEY);
		return raw ? (JSON.parse(raw) as Kept) : undefined;
	} catch {
		return undefined;
	}
}

function save(kept: Kept): void {
	try {
		localStorage.setItem(KEY, JSON.stringify(kept));
	} catch {
		// Private browsing, or storage refused. The library still arrives; the
		// phone simply asks again next time.
	}
}

/**
 * - `unknown`: not yet looked at, this visit.
 * - `unavailable`: nothing can be kept here — not a secure page, no service
 *   worker, or nobody signed in.
 * - `absent`: never filled; the first fill is waiting.
 * - `filling`: arriving now.
 * - `kept`: on the phone.
 */
export type Phase = 'unknown' | 'unavailable' | 'absent' | 'filling' | 'kept';

type Entry = { branch_id: string; main_photo: string | null };

export class Library {
	phase = $state<Phase>('unknown');
	/** Recipes in the Person's Kitchens. */
	count = $state(0);
	/** About how much a fill costs, in bytes. */
	bytes = $state(0);
	done = $state(0);
	total = $state(0);
	filledAt = $state<Date | undefined>(undefined);
	/** Branches held by the Person's Kitchens, as last known. */
	readonly held = new SvelteSet<string>();
	/** Whether `held` has ever been learned on this device. Until it has, nothing is claimed about a recipe. */
	known = $state(false);

	#kamosu: KamosuClient;
	#entries: Entry[] = [];
	#running: Promise<void> | undefined;

	constructor(kamosu: KamosuClient) {
		this.#kamosu = kamosu;
		this.#recall();
	}

	/** What the device remembers of the library, which a new Session wipes. */
	#recall(): void {
		const kept = load();
		this.filledAt = kept?.filledAt ? new Date(kept.filledAt) : undefined;
		this.known = kept !== undefined;
		this.held.clear();
		for (const id of kept?.held ?? []) this.held.add(id);
	}

	#remember(): void {
		save({ filledAt: this.filledAt?.getTime(), held: [...this.held] });
		this.known = true;
	}

	/**
	 * Whether the page is showing a recipe the Person only opened — a copy from
	 * that day — rather than one of their Kitchens', which the library keeps
	 * whole. Nothing is claimed until the device has learned which are which.
	 */
	onlyOpened(branchId: string): boolean {
		return this.known && !this.held.has(branchId);
	}

	/**
	 * Find out what the library is, and bring what the phone holds level with
	 * it: silently if it was filled before, on wifi if the browser can say, and
	 * otherwise by waiting for `fill()`.
	 */
	async start(wifi: boolean | undefined): Promise<void> {
		this.#recall();
		if (!(await controlled())) {
			this.phase = 'unavailable';
			return;
		}
		try {
			// Exactly what the Recipes shelf asks on first open, so surveying the
			// library is also what puts that shelf on the phone.
			const found = await this.#kamosu.searchRecipes({
				query: null,
				kitchen_id: null,
				mine: false,
			});
			this.#entries = found.recipes;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			// Signed out, or no server: nothing to fill, and nothing to offer.
			this.phase = this.filledAt ? 'kept' : 'unavailable';
			return;
		}
		this.count = this.#entries.length;
		const photographs = new Set(this.#entries.flatMap((e) => (e.main_photo ? [e.main_photo] : [])));
		this.bytes = photographs.size * BYTES_PER_PHOTOGRAPH + this.count * BYTES_PER_RECIPE;
		// The shelf shows one entry per Lineage; a second Branch of it in another
		// of the Person's Kitchens joins `held` when the fill reads its Thread.
		// Added to rather than replaced, because a top-up reads no Thread and
		// would otherwise forget those; a new Session starts it afresh.
		for (const entry of this.#entries) this.held.add(entry.branch_id);
		this.#remember();

		if (this.filledAt) {
			this.phase = 'kept';
			await this.fill({ owed: true });
		} else if (wifi === true) {
			await this.fill();
		} else {
			this.phase = 'absent';
		}
	}

	/** Bring the whole library onto the phone now. `owed` fetches only what is missing. */
	fill(options: { owed?: boolean } = {}): Promise<void> {
		this.#running ??= this.#fill(options.owed ?? false).finally(() => (this.#running = undefined));
		return this.#running;
	}

	async #fill(owed: boolean): Promise<void> {
		const was = this.phase;
		const queue = [...this.#entries];
		this.total = queue.length;
		this.done = 0;
		if (!owed) this.phase = 'filling';
		void persist();

		let failed = false;
		const reader = async () => {
			for (let entry = queue.shift(); entry && !failed; entry = queue.shift()) {
				try {
					if (!owed || (await missing(entry))) {
						for (const sibling of await this.#read(entry)) {
							if (this.held.has(sibling.branch_id)) continue;
							this.held.add(sibling.branch_id);
							queue.push(sibling);
							this.total += 1;
						}
					}
				} catch (error) {
					if (!(error instanceof OperationError) && !(error instanceof TypeError)) throw error;
					// The network went, or the Session did: stop, and let the next
					// start pick up the rest. A single recipe refused — deleted
					// while this ran — is simply skipped.
					failed = error instanceof OperationError ? unreachable(error) : true;
				}
				this.done += 1;
			}
		};
		await Promise.all(Array.from({ length: AT_ONCE }, reader));

		if (failed) {
			this.phase = was === 'kept' ? 'kept' : 'absent';
			return;
		}
		this.filledAt = new Date();
		this.#remember();
		this.phase = 'kept';
	}

	/**
	 * One recipe, read as the recipe screen reads it on opening — the recipe,
	 * its pictures at the two sizes the shelf and the page show, its Thread,
	 * and every other version of it laid over the reader's own, which is what
	 * the switch compares against (#131) — so that opening any of them with no
	 * network finds every answer that screen asks for.
	 *
	 * Answers the other versions of the recipe the Person may see, which the
	 * shelf's one-entry-per-Lineage never named. The Thread answers only what
	 * they may see, so every one it names is theirs to keep.
	 */
	async #read(entry: Entry): Promise<Entry[]> {
		const recipe = await this.#kamosu.getRecipe({ branch_id: entry.branch_id });
		// What it puts on a Shopping List, so the list's rows can be worked out
		// here with no network (#77, ADR 0024).
		// Refused, the list simply waits for the server to add this recipe up;
		// only a network gone stops the fill.
		await this.#kamosu.shoppingBasis({ branch_id: entry.branch_id }).catch((error: unknown) => {
			if (!(error instanceof OperationError) || unreachable(error)) throw error;
		});
		const page = recipe.versions.at(-1)?.content.main_photo;
		await Promise.all([
			entry.main_photo ? picture(`/api/photographs/${entry.main_photo}/card`) : undefined,
			page ? picture(`/api/photographs/${page}/page`) : undefined,
		]);
		const thread = await this.#kamosu.getThread({ branch_id: entry.branch_id });
		// The page's own rule for which versions go on the switch: those that
		// share a Version with this one, which no Translation does.
		const held = new Map<string, Set<string>>();
		for (const occurrence of thread.versions) {
			const seen = held.get(occurrence.branch_id) ?? new Set<string>();
			seen.add(occurrence.version_id);
			held.set(occurrence.branch_id, seen);
		}
		const own = held.get(entry.branch_id) ?? new Set<string>();
		const family = thread.branches.filter((each) =>
			[...(held.get(each.branch_id) ?? [])].some((id) => own.has(id)),
		);
		const yours = family.find((each) => each.mine && !each.arrived && each.name === null);
		// Every version is filled, Translations included: the Thread names only
		// what this Person may see, and the page's language switch opens those.
		const others = thread.branches.filter((each) => each.branch_id !== entry.branch_id);
		for (const other of family) {
			if (!yours || other.branch_id === yours.branch_id) continue;
			try {
				await this.#kamosu.divergence({
					branch_id: yours.branch_id,
					other_branch_id: other.branch_id,
				});
			} catch (error) {
				// Refused, the page will be refused the same, and nothing else
				// about this recipe depends on it. Only a network gone stops the fill.
				if (!(error instanceof OperationError) || unreachable(error)) throw error;
			}
		}
		// No card picture: the shelf shows one card per Lineage, and it is this one's.
		return others.map((each) => ({ branch_id: each.branch_id, main_photo: null }));
	}
}

/** The network is gone, or the Session is: no point asking for the next recipe. */
const unreachable = (error: OperationError) => !error.reached || error.kind === 'unauthorized';

/** Whether a service worker is answering this page's requests, so reading is keeping. */
async function controlled(): Promise<boolean> {
	if (typeof navigator === 'undefined' || !('serviceWorker' in navigator) || !isSecureContext) {
		return false;
	}
	const workers = navigator.serviceWorker;
	if (workers.controller) return true;
	// A first visit: the worker installs, then takes this page (`clients.claim`).
	return new Promise((resolve) => {
		const timer = setTimeout(() => resolve(false), 10_000);
		workers.addEventListener(
			'controllerchange',
			() => {
				clearTimeout(timer);
				resolve(true);
			},
			{ once: true },
		);
	});
}

/** Whether the phone still owes this recipe. */
async function missing(entry: Entry): Promise<boolean> {
	if (!(await keptAt('get_recipe', { branch_id: entry.branch_id }))) return true;
	if (!(await keptAt('shopping_basis', { branch_id: entry.branch_id }))) return true;
	if (!entry.main_photo) return false;
	const cached = await caches.match(`/api/photographs/${entry.main_photo}/card`, {
		cacheName: PHOTOGRAPHS_CACHE,
	});
	return cached === undefined;
}

async function picture(url: string): Promise<void> {
	const response = await fetch(url);
	// Read to the end, so the worker has finished keeping it.
	await response.arrayBuffer();
}

/**
 * Ask the browser not to clear what is kept here when space runs short. It may
 * say no; the library is kept either way, just less firmly.
 */
async function persist(): Promise<void> {
	try {
		await navigator.storage?.persist?.();
	} catch {
		// Nothing to do: this is a request, not a requirement.
	}
}

/**
 * Bumped whenever a Session begins on this device. The service worker empties
 * what it kept for the last one; this forgets that the library was ever
 * filled, so the next person's first fill waits for wifi like anyone's, and
 * the library is looked at again for them.
 */
export const sessions = $state({ began: 0 });

export function sessionBegan(): void {
	try {
		localStorage.removeItem(KEY);
	} catch {
		// Nothing kept, then nothing to forget.
	}
	// And nothing the last person wrote offline is sent as this one (#77).
	void forgetEverythingHeld();
	sessions.began += 1;
}

const LIBRARY = Symbol('library');

export function provideLibrary(library: () => Library): void {
	setContext(LIBRARY, library);
}

export function useLibrary(): Library {
	const library = getContext<(() => Library) | undefined>(LIBRARY)?.();
	if (!library) throw new Error('no Library in context — render inside <Kamosu>');
	return library;
}

/** A size a person reads: "13 MB", never "13,107,200 bytes". */
export function readableSize(bytes: number): string {
	if (bytes < 1_000_000) return `${Math.max(1, Math.round(bytes / 1000))} KB`;
	return `${Math.round(bytes / 1_000_000)} MB`;
}
