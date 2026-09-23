/**
 * The service worker's rules (#76, ADR 0013), against a cache and a server of
 * the test's own.
 *
 * What these guard: a read is answered from the phone and refreshed behind; a
 * write is never kept or replayed; your own change is never shown undone by an
 * older copy; and what one Session filled is never shown to the next.
 */

import { describe, expect, it } from 'vitest';
import { createWorker } from './worker';
import {
	KEPT_HEADER,
	PHOTOGRAPHS_CACHE,
	READS_CACHE,
	canonical,
	policyFor,
	readKey,
	serverAnswered,
} from './reads';

const ORIGIN = 'https://kamosu.test';
const SHELL = 'kamosu-shell-test';

/** A CacheStorage holding what it was given, keyed by URL as a browser's is. */
function fakeCaches() {
	const stores = new Map<string, Map<string, Response>>();
	const keyOf = (request: RequestInfo | URL) =>
		new URL(
			typeof request === 'string' ? request : request instanceof URL ? request.href : request.url,
			ORIGIN,
		).href;
	const open = async (name: string) => {
		if (!stores.has(name)) stores.set(name, new Map());
		const store = stores.get(name)!;
		return {
			match: async (request: RequestInfo | URL) => store.get(keyOf(request))?.clone(),
			put: async (request: RequestInfo | URL, response: Response) => {
				store.set(keyOf(request), response.clone());
			},
		} as unknown as Cache;
	};
	return {
		stores,
		caches: {
			open,
			delete: async (name: string) => stores.delete(name),
		} as unknown as CacheStorage,
	};
}

type Answer = { status?: number; body: unknown } | 'unreachable';

/** A server answering from a script, and remembering what it was asked. */
function fakeServer(script: (path: string, input: unknown) => Answer) {
	const asked: string[] = [];
	const fetch = async (input: RequestInfo | URL): Promise<Response> => {
		const request = input as Request;
		const path = new URL(request.url).pathname;
		const body = request.method === 'POST' ? await request.clone().json() : undefined;
		asked.push(path);
		const answer = script(path, body);
		if (answer === 'unreachable') throw new TypeError('Failed to fetch');
		return new Response(JSON.stringify(answer.body), {
			status: answer.status ?? 200,
			headers: { 'content-type': 'application/json' },
		});
	};
	return { asked, fetch: fetch as typeof globalThis.fetch };
}

/** The two path lists a worker is given: what was compiled, and what the shell cache holds. */
type BuildLists = { built?: readonly string[]; precached?: readonly string[] };

function world(
	script: (path: string, input: unknown) => Answer,
	shared = { ...fakeCaches(), clock: { at: 1000 } },
	lists: BuildLists = {},
) {
	const { stores, caches, clock } = shared;
	const server = fakeServer(script);
	const announced: string[] = [];
	const reached: boolean[] = [];
	const background: Promise<unknown>[] = [];
	const worker = createWorker({
		caches,
		fetch: server.fetch,
		now: () => clock.at++,
		origin: ORIGIN,
		shell: SHELL,
		built: lists.built ?? [],
		precached: lists.precached ?? [],
		tell: (message) =>
			message.type === 'kamosu:refreshed'
				? announced.push(message.operation)
				: reached.push(message.reached),
	});
	const call = async (operation: string, input: unknown = {}) => {
		const request = new Request(`${ORIGIN}/api/op/${operation}`, {
			method: 'POST',
			body: JSON.stringify(input),
		});
		const response = await worker.handle(request, (work) => background.push(work));
		return (await response!.json()) as { ok: boolean; result?: unknown };
	};
	/** A browser that stopped this worker and started a fresh one over the same caches. */
	const restarted = () => world(script, shared, lists);
	const settle = async () => {
		await Promise.all(background.splice(0));
	};
	return { worker, stores, server, announced, reached, call, settle, restarted };
}

const ok = (result: unknown): Answer => ({ body: { ok: true, result } });

describe('what the phone keeps', () => {
	it('knows a read from a write by the Catalogue alone', () => {
		expect(policyFor('get_recipe')).toBe('phone-first');
		expect(policyFor('get_current_attempt')).toBe('server-first');
		// #108: an import just run must be on the list, and a ledger just
		// forgotten must not still be counted.
		expect(policyFor('list_imports')).toBe('server-first');
		// #111: whether a new recipe asks whose it is depends on the Kitchens
		// you cook in now, not the ones you cooked in last time.
		expect(policyFor('list_kitchens')).toBe('server-first');
		expect(policyFor('get_job')).toBe('never');
		expect(policyFor('promote_as_cooked')).toBeUndefined();
	});

	it('names one input one way, whatever order its keys arrived in', () => {
		expect(canonical({ b: 1, a: { d: [2, { f: 1, e: 0 }], c: null } })).toBe(
			canonical({ a: { c: null, d: [2, { e: 0, f: 1 }] }, b: 1 }),
		);
		expect(readKey(ORIGIN, 'get_recipe', { branch_id: 'b_1' })).not.toBe(
			readKey(ORIGIN, 'get_recipe', { branch_id: 'b_2' }),
		);
	});
});

describe('a read', () => {
	it('is answered from the phone once kept, and refreshed behind', async () => {
		let title = 'Dan Dan Noodles';
		const w = world(() => ok({ title }));

		expect((await w.call('get_recipe', { branch_id: 'b_1' })).result).toEqual({ title });
		await w.settle();

		title = 'Dan Dan Noodles, less chilli';
		// Answered at once from what was kept…
		expect((await w.call('get_recipe', { branch_id: 'b_1' })).result).toEqual({
			title: 'Dan Dan Noodles',
		});
		await w.settle();
		// …while the server was asked behind it, the copy brought level, and the
		// open page told that what it shows has changed.
		expect(w.server.asked).toHaveLength(2);
		expect(w.announced).toEqual(['get_recipe']);
		expect((await w.call('get_recipe', { branch_id: 'b_1' })).result).toEqual({ title });
	});

	it('says nothing when the refresh brought nothing new', async () => {
		const w = world(() => ok({ title: 'Same' }));
		await w.call('get_recipe', { branch_id: 'b_1' });
		await w.settle();
		await w.call('get_recipe', { branch_id: 'b_1' });
		await w.settle();
		expect(w.announced).toEqual([]);
	});

	it('is still answered with no network, from what was kept', async () => {
		let online = true;
		const w = world(() => (online ? ok({ title: 'Korean Fried Chicken' }) : 'unreachable'));
		await w.call('get_recipe', { branch_id: 'b_1' });
		await w.settle();

		online = false;
		expect((await w.call('get_recipe', { branch_id: 'b_1' })).result).toEqual({
			title: 'Korean Fried Chicken',
		});
		await w.settle();
	});

	it('tells the page each time whether the server was reached, so a page opened late still learns', async () => {
		let online = true;
		const w = world(() => (online ? ok({ title: 'x' }) : 'unreachable'));
		await w.call('get_recipe', { branch_id: 'b_1' });
		await w.settle();
		online = false;
		// Answered from the phone, while the refresh behind it finds no server.
		await w.call('get_recipe', { branch_id: 'b_1' });
		await w.settle();
		await w.call('get_recipe', { branch_id: 'b_1' });
		await w.settle();
		online = true;
		await w.call('get_recipe', { branch_id: 'b_1' });
		await w.settle();
		expect(w.reached).toEqual([true, false, false, true]);
	});

	it('fails as the network does when nothing was ever kept', async () => {
		const w = world(() => 'unreachable');
		await expect(w.call('get_recipe', { branch_id: 'b_never' })).rejects.toThrow();
	});

	it('keeps no refusal', async () => {
		let refuse = true;
		const w = world(() =>
			refuse
				? { status: 404, body: { ok: false, error: { kind: 'not_found' } } }
				: ok({ title: 'Found' }),
		);
		await w.call('get_recipe', { branch_id: 'b_1' });
		refuse = false;
		expect((await w.call('get_recipe', { branch_id: 'b_1' })).result).toEqual({ title: 'Found' });
	});

	it('that is about being current asks the server first', async () => {
		let step = 1;
		const w = world(() => ok({ step }));
		await w.call('get_current_attempt');
		step = 4;
		expect((await w.call('get_current_attempt')).result).toEqual({ step: 4 });
	});

	it('that is a Job’s progress is never kept at all', async () => {
		const w = world(() => ok({ done: 1 }));
		await w.call('get_job', { job_id: 'j_1' });
		expect(w.stores.get(READS_CACHE)?.size ?? 0).toBe(0);
	});
});

describe('whether the server was reached', () => {
	it('is judged from what came back', () => {
		const json = { 'content-type': 'application/json' };
		expect(serverAnswered(new TypeError('Failed to fetch'))).toBe(false);
		expect(serverAnswered(new Response('{}', { status: 200, headers: json }))).toBe(true);
		// Kamosu refusing is Kamosu answering.
		expect(
			serverAnswered(
				new Response('{"ok":false}', {
					status: 503,
					headers: { 'content-type': 'application/json' },
				}),
			),
		).toBe(true);
	});

	it("counts anything but Kamosu's envelope as not reached", () => {
		// The proxy in front of a Kamosu that is down…
		expect(serverAnswered(new Response('Bad Gateway', { status: 502 }))).toBe(false);
		expect(serverAnswered(new Response('<html>', { status: 504 }))).toBe(false);
		// …and a hotel wifi's login page, which answers 200.
		expect(
			serverAnswered(new Response('<html>', { headers: { 'content-type': 'text/html' } })),
		).toBe(false);
	});

	it('takes nothing from a cancelled request or an answer the phone kept', () => {
		expect(serverAnswered(new DOMException('gone', 'AbortError'))).toBeUndefined();
		expect(serverAnswered(new Response('{}', { headers: { [KEPT_HEADER]: '1' } }))).toBeUndefined();
	});
});

describe('a write', () => {
	it('goes to the server, and is neither kept nor answered by the phone', async () => {
		let online = true;
		const w = world((path) =>
			!online ? 'unreachable' : path.endsWith('promote_as_cooked') ? ok({}) : ok({ v: 1 }),
		);
		await w.call('promote_as_cooked', { attempt_id: 'a_1' });
		expect(w.stores.get(READS_CACHE)?.size ?? 0).toBe(0);

		// No queue: offline, a write simply does not happen.
		online = false;
		await expect(w.call('promote_as_cooked', { attempt_id: 'a_1' })).rejects.toThrow();
	});

	it('makes the next read ask the server, so your own change is never shown undone', async () => {
		let onList = false;
		const w = world((path) => {
			if (path.endsWith('add_to_shopping_list')) {
				onList = true;
				return ok({});
			}
			return ok({ chosen: onList ? ['b_1'] : [] });
		});
		await w.call('get_shopping_list');
		await w.settle();

		await w.call('add_to_shopping_list', { branch_id: 'b_1' });
		// Even when the browser stopped the worker in between and started a
		// fresh one, which remembers the write only through its notes.
		const again = w.restarted();
		expect((await again.call('get_shopping_list')).result).toEqual({ chosen: ['b_1'] });
		expect((await w.call('get_shopping_list')).result).toEqual({ chosen: ['b_1'] });

		// And once brought level, it is phone-first again.
		const before = w.server.asked.length;
		await w.call('get_shopping_list');
		expect(w.server.asked.length).toBe(before + 1); // the refresh behind, not a wait
		await w.settle();
	});
});

describe('a Session', () => {
	it('that changes empties everything the last one filled', async () => {
		const w = world((path) => (path.startsWith('/auth/') ? ok({}) : ok({ title: 'Mine' })));
		await w.call('get_recipe', { branch_id: 'b_1' });
		await w.worker.handle(new Request(`${ORIGIN}/api/photographs/p_1/card`), () => undefined);
		expect(w.stores.get(READS_CACHE)?.size).toBe(1);
		expect(w.stores.get(PHOTOGRAPHS_CACHE)?.size).toBe(1);

		await w.worker.handle(
			new Request(`${ORIGIN}/auth/login`, { method: 'POST', body: '{}' }),
			() => undefined,
		);
		expect(w.stores.has(READS_CACHE)).toBe(false);
		// Its pictures are its recipes' too.
		expect(w.stores.has(PHOTOGRAPHS_CACHE)).toBe(false);
	});

	it('that is refused empties it too', async () => {
		let signedIn = true;
		const w = world(() =>
			signedIn
				? ok({ title: 'Mine' })
				: { status: 401, body: { ok: false, error: { kind: 'unauthorized' } } },
		);
		await w.call('get_recipe', { branch_id: 'b_1' });
		await w.settle();
		signedIn = false;
		await w.call('get_recipe', { branch_id: 'b_1' });
		await w.settle();
		expect(w.stores.has(READS_CACHE)).toBe(false);
	});

	it('that has ended tells the page it was shown a kept answer, so it asks again', async () => {
		let signedIn = true;
		const w = world(() =>
			signedIn
				? ok({ shelves: [] })
				: { status: 401, body: { ok: false, error: { kind: 'unauthorized' } } },
		);
		await w.call('home_shelves');
		await w.settle();
		signedIn = false;
		// Answered from the phone, and then told otherwise…
		expect((await w.call('home_shelves')).ok).toBe(true);
		await w.settle();
		expect(w.announced).toEqual(['home_shelves']);
		// …so the page's next ask reaches the server and lands on the sign-in form.
		expect((await w.call('home_shelves')).ok).toBe(false);
	});

	it('that is ended empties it', async () => {
		const w = world(() => ok({ title: 'Mine' }));
		await w.call('get_recipe', { branch_id: 'b_1' });
		await w.call('revoke_session', { session_id: 's_1' });
		expect(w.stores.has(READS_CACHE)).toBe(false);
	});
});

describe('a navigation', () => {
	/** What a browser asking for a screen is. A Request cannot be built with that mode. */
	function navigationTo(path: string): Request {
		const request = new Request(`${ORIGIN}${path}`);
		Object.defineProperty(request, 'mode', { value: 'navigate' });
		return request;
	}

	/** A worker whose shell cache holds what install put there, and nothing else. */
	async function installed(lists: BuildLists) {
		const shared = { ...fakeCaches(), clock: { at: 1000 } };
		const store = await shared.caches.open(SHELL);
		for (const path of [...(lists.precached ?? []), '/']) {
			await store.put(path, new Response(`the copy of ${path}`));
		}
		return world(() => ({ body: 'from the server' }), shared, lists);
	}

	it('reaches the server in development, where the cached home page would read 404 (#95)', async () => {
		// `ui/static/` is precached in development too, so a precache list with
		// something in it and no compiled output is exactly the state that used
		// to be mistaken for a build.
		const w = await installed({ built: [], precached: ['/manifest.webmanifest', '/robots.txt'] });
		const answer = await w.worker.handle(navigationTo('/recipes/b_1'), () => undefined);
		expect(await answer!.json()).toBe('from the server');
		expect(w.server.asked).toEqual(['/recipes/b_1']);
	});

	it('is answered from the cached shell once there is a build, which is what #76 is', async () => {
		const w = await installed({
			built: ['/_app/immutable/entry/app.js'],
			precached: ['/_app/immutable/entry/app.js', '/manifest.webmanifest'],
		});
		const answer = await w.worker.handle(navigationTo('/recipes/b_1'), () => undefined);
		expect(await answer!.text()).toBe('the copy of /');
		expect(w.server.asked).toEqual([]);
	});

	it('leaves a precached static file to the shell cache in development too', async () => {
		const w = await installed({ built: [], precached: ['/manifest.webmanifest'] });
		const answer = await w.worker.handle(
			new Request(`${ORIGIN}/manifest.webmanifest`),
			() => undefined,
		);
		expect(await answer!.text()).toBe('the copy of /manifest.webmanifest');
		expect(w.server.asked).toEqual([]);
	});
});

describe('what was kept', () => {
	it('carries when it was kept', async () => {
		const w = world(() => ok({ title: 'x' }));
		await w.call('get_recipe', { branch_id: 'b_1' });
		const kept = w.stores
			.get(READS_CACHE)!
			.get(readKey(ORIGIN, 'get_recipe', { branch_id: 'b_1' }));
		expect(Number(kept?.headers.get(KEPT_HEADER))).toBeGreaterThan(0);
	});

	it('includes a Photograph, kept for good once seen', async () => {
		const w = world(() => ({ body: 'webp' }));
		const ask = () =>
			w.worker.handle(new Request(`${ORIGIN}/api/photographs/p_1/page`), () => undefined);
		await ask();
		await ask();
		expect(w.server.asked).toEqual(['/api/photographs/p_1/page']);
	});

	it('never includes an original, only the Display Copies a screen shows', () => {
		const w = world(() => ({ body: 'webp' }));
		expect(
			w.worker.handle(new Request(`${ORIGIN}/api/photographs/p_1`), () => undefined),
		).toBeUndefined();
		expect(
			w.worker.handle(new Request(`${ORIGIN}/api/photographs/p_1/print`), () => undefined),
		).toBeUndefined();
	});

	it('leaves another origin’s requests to the browser', () => {
		const w = world(() => ok({}));
		expect(w.worker.handle(new Request('https://example.com/x'), () => undefined)).toBe(undefined);
	});
});
