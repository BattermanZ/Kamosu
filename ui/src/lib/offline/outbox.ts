/**
 * What a phone with no network may still write, kept until it can be sent
 * (#77, ADR 0013).
 *
 * **Everything on an Attempt's side of Promotion works offline; nothing on the
 * recipe's side does.** Starting a cooking, moving through it, ticking,
 * writing down what you did differently, photographing it, finishing, rating
 * and noting it are facts about one person on one afternoon, and so is the
 * Shopping List (ADR 0024). This keeps exactly those writes when the server
 * cannot be reached, answers the screen as though they had landed, and sends
 * them in order once it can. Every other write still goes straight to the
 * server and nowhere else: an offline edit queue for a recipe would be a
 * merge, and Kamosu never merges.
 *
 * **Nothing here asks anybody anything when it sends.** Each write carries
 * when it was really made, and the server decides from that (`written_at`):
 * a move older than where the cook now stands changes nothing, and so does a
 * change to a Shopping List older than the list's last one. The last one to
 * move on wins, which is what those things already meant (ADR 0010, ADR 0024).
 * A cooking finished on another device is final: a move or a finish the
 * phone sends it afterwards changes nothing and fails nothing.
 *
 * **The screens do not know this exists.** It sits in the transport every
 * Operation already goes through, so a screen that advances a Step calls
 * `advanceAttempt` exactly as it always did and gets an Attempt back either
 * way. The only new things a screen asks of it are about photographs, because
 * a picture is bytes before it is an id.
 *
 * **What it keeps belongs to the Session that wrote it**, like everything else
 * the phone keeps (#76): a new Session begins with nothing held.
 */

import { CATALOGUE } from '$lib/api/catalogue';
import type {
	GetCurrentAttemptOutput,
	GetRecipeOutput,
	GetReadingPreferencesOutput,
	GetShoppingListOutput,
	ListAttemptsOutput,
	OperationName,
	ShoppingBasisOutput,
	StartAttemptOutput,
} from '$lib/api/catalogue';
import { getContext, setContext } from 'svelte';
import { OperationError, httpTransport, type Transport } from '$lib/api/client';
import { realPhotographUpload } from '$lib/api/upload';
import { noteReach, refreshed } from './device.svelte';
import { asText, rows, yieldScale, type List, type Row } from './shopping';

type Attempt = StartAttemptOutput;
type DiaryEntry = ListAttemptsOutput['attempts'][number];
type Input = Record<string, unknown>;

/** The writes on an Attempt's side of Promotion (ADR 0013). */
const ATTEMPT_WRITES = new Set<OperationName>([
	'start_attempt',
	'advance_attempt',
	'set_as_cooked',
	'finish_attempt',
	'edit_attempt',
	'delete_attempt',
]);

/** The Shopping List's writes, which ADR 0024 put on the same side. */
const SHOPPING_WRITES = new Set<OperationName>([
	'add_to_shopping_list',
	'remove_from_shopping_list',
	'set_shopping_yield',
	'add_loose_item',
	'remove_loose_item',
	'empty_shopping_list',
]);

/**
 * The writes that say when they were made — read off the Catalogue, which
 * declares `written_at` on exactly those (#77), so this cannot fall out of
 * step with what the server accepts. `start_attempt` says it as `started_at`.
 */
const SAYS_WHEN = new Set<OperationName>(
	CATALOGUE.filter(
		(declaration) =>
			'written_at' in ((declaration.input_schema as { properties?: object }).properties ?? {}),
	).map((declaration) => declaration.name as OperationName),
);

/** How many cookings that are over the phone remembers (`#settle`). */
const SETTLED_KEPT = 20;

/** A picture kept on the phone is named this until the server names it by its bytes. */
const KEPT_PICTURE = 'local:';

/**
 * Whether a cooking's picture is still only on this phone, under its kept
 * name. The server knows nothing by that name, so nothing can be done with it
 * there — promoting it to a recipe included (#110) — until it has been sent.
 */
export const onlyOnThisPhone = (photograph: string): boolean => photograph.startsWith(KEPT_PICTURE);

/** One write not yet sent, as it will be sent. */
interface Pending {
	/**
	 * Which write this is, so the one the server has just answered is the one
	 * taken out — not whatever happens to be first by then.
	 */
	id: string;
	operation: OperationName;
	input: Input;
	/** When it was made, which is what the server judges it by. */
	written_at: string;
}

/**
 * A cooking as this phone last had it: one it wrote to offline, one the
 * server answered with, or one that is over — finished or thrown away — and
 * remembered only so a stale read cannot hand it back (`#settle`).
 */
interface Held {
	attempt: Attempt;
	/** What the diary calls it, for a cooking the server has never heard of. */
	recipe: DiaryEntry['recipe'];
	/** Thrown away, here or online. */
	gone?: boolean;
}

/** Still going: neither finished nor thrown away. */
const live = (held: Held) => !held.gone && held.attempt.finished_at === null;

/** Everything this keeps, as one value, so it can be saved and read back whole. */
export interface Kept {
	queue: Pending[];
	/**
	 * Every cooking the phone knows, by id: what it answers for while writes
	 * wait, what it resumes when the network goes mid-cook, and what it will
	 * not resume because it is over.
	 */
	attempts: Record<string, Held>;
	/**
	 * The Shopping List as the phone has it, while changes to it are waiting.
	 * Null when nothing is: the list is then the server's.
	 */
	shopping: { chosen: List['chosen']; loose: { id: string; text: string }[] } | null;
	/**
	 * A cooking started here that the server answered with another it already
	 * had In Progress (ADR 0010): everything written to the first goes to the
	 * second.
	 */
	renamed: Record<string, string>;
	/** A picture kept here, and the Photograph the server made of it. */
	photographs: Record<string, string>;
	/**
	 * The waiting write on its way to the server, while it is. Kept here
	 * rather than in memory so that another open tab, deleting a cooking
	 * whose start is being sent, sees it too.
	 */
	sending?: string;
}

const nothingKept = (): Kept => ({
	queue: [],
	attempts: {},
	shopping: null,
	renamed: {},
	photographs: {},
});

/** Where the queue is saved between visits. A test hands in its own. */
export interface Storage {
	load(): Kept | undefined;
	save(kept: Kept): void;
}

/** Where pictures wait for the server. A test hands in its own. */
export interface Pictures {
	keep(name: string, picture: Blob): Promise<void>;
	read(name: string): Promise<Blob | undefined>;
	forget(name: string): Promise<void>;
}

export interface World {
	/** The transport that actually reaches Kamosu — through the service worker, which answers reads from the phone. */
	inner: Transport;
	storage: Storage;
	pictures: Pictures;
	/** Send one picture's bytes, answering the id the server gives the Photograph it makes of them. */
	upload: (picture: Blob) => Promise<string>;
	now: () => Date;
	/** Sixteen lower-case hex digits, fresh each time: the shape Kamosu mints its own ids in. */
	mint: () => string;
	/** Run the sending alone, across every open tab. */
	lock?: (work: () => Promise<void>) => Promise<void>;
	/** Tell the screens that these reads may now say something different. */
	told?: (operations: OperationName[]) => void;
}

/** What a screen may ask of it: only about photographs, and whether a cooking is still waiting. */
export interface Keeping {
	/** Whether this phone holds writes to this cooking that it has not sent yet. */
	holds(attemptId: string): boolean;
	/** Keep a picture on the phone. Answers the name to put on the cooking. */
	keepPhotograph(picture: Blob): Promise<string>;
	/** Where to draw one of a cooking's photographs from, whether or not it has reached the server. */
	photographSrc(id: string, size: 'card' | 'page'): Promise<string | undefined>;
}

const isUnreached = (error: unknown): error is OperationError =>
	error instanceof OperationError && !error.reached;

export class Outbox implements Keeping {
	#world: World;
	#sending: Promise<void> | undefined;
	#again = false;
	/**
	 * Every change to what is kept, one after another. Holding a write reads
	 * the queue, may wait on the phone for a recipe or a list, and writes the
	 * queue back; two of those interleaved — two taps, or a tap and the sender
	 * finishing a write — would each write back a queue missing the other's
	 * change.
	 */
	#turn: Promise<unknown> = Promise.resolve();
	/** Writes being held right now, not yet in the queue a direct write checks. */
	#holding = 0;
	/** Pictures drawn from the phone this visit, so each is read from storage once. */
	#drawn = new Map<string, string>();

	constructor(world: World) {
		this.#world = world;
	}

	/** Read afresh every time: another tab, or a new Session, may have changed it. */
	#read(): Kept {
		return { ...nothingKept(), ...(this.#world.storage.load() ?? {}) };
	}

	#write(kept: Kept): void {
		this.#world.storage.save(kept);
	}

	/** Run `change` once every change before it has finished. */
	#exclusive<T>(change: () => Promise<T> | T): Promise<T> {
		const turn = this.#turn.then(change, change);
		this.#turn = turn.catch(() => undefined);
		return turn;
	}

	/** The transport screens use: the one they had, with this in front of it. */
	readonly transport: Transport = (operation, input) =>
		this.#call(operation, (input ?? {}) as Input);

	async #call(operation: OperationName, input: Input): Promise<unknown> {
		if (ATTEMPT_WRITES.has(operation) || SHOPPING_WRITES.has(operation)) {
			return this.#writeOne(operation, input);
		}
		const kept = this.#read();
		switch (operation) {
			case 'get_current_attempt': {
				const held = this.#heldFor(kept, input.lineage_id as string);
				if (held) return { attempt: live(held) ? held.attempt : null };
				const read = (await this.#world.inner(operation, input)) as GetCurrentAttemptOutput;
				// A cooking the phone knows is over is over, whatever an older
				// copy of this read says: the server never makes one In
				// Progress again once it has finished or gone.
				const known = read.attempt && kept.attempts[this.#renamed(kept, read.attempt.id)];
				if (known && !live(known)) return { attempt: null };
				this.#notice(operation, read);
				return read;
			}
			case 'list_attempts':
				if (this.#waiting(kept).size > 0) return this.#diary(kept);
				break;
			case 'get_shopping_list':
				if (kept.shopping) return this.#list(kept);
				break;
			case 'shopping_list_as_text':
				if (kept.shopping) {
					const list = await this.#list(kept);
					const { reading_language } = await this.#preferences();
					// The server's date is the instance's day in UTC; so is this.
					const today = this.#world.now().toISOString().slice(0, 10);
					return { text: asText(list, today, reading_language) };
				}
				break;
		}
		const answer = await this.#world.inner(operation, input);
		this.#notice(operation, answer);
		if (operation === 'list_attempts' || operation === 'get_thread') {
			this.#freshen((answer as { attempts: Attempt[] }).attempts);
		}
		return answer;
	}

	/**
	 * Mark as finished any of the phone's copies of cookings that a read says
	 * have finished — the diary and a recipe's Thread both carry each
	 * cooking's own state — so one finished on the iPad is known to be over
	 * here, and is not resumed the next time the network goes. A cooking
	 * thrown away elsewhere is not learned this way, and one resumed from a
	 * copy that old has its moves ignored by the server, which is the cost.
	 */
	#freshen(fresh: Attempt[]): void {
		const kept = this.#read();
		const waiting = this.#waiting(kept);
		const said = new Map(fresh.map((attempt) => [attempt.id, attempt]));
		let changed = false;
		for (const [id, held] of Object.entries(kept.attempts)) {
			if (waiting.has(id) || !live(held)) continue;
			const now = said.get(id);
			if (now && now.finished_at !== null) {
				held.attempt = { ...held.attempt, finished_at: now.finished_at, resumable: false };
				changed = true;
			}
		}
		if (changed) {
			this.#settle(kept);
			this.#write(kept);
		}
	}

	// ---- writing --------------------------------------------------------------

	async #writeOne(operation: OperationName, given: Input): Promise<unknown> {
		const input = { ...given };
		// Named here, always, so that the write means the same thing whether it
		// is sent now or in an hour, and sending it twice is sending it once.
		if (operation === 'start_attempt' && input.attempt_id === undefined) {
			input.attempt_id = `at_${this.#world.mint()}`;
		}
		if (operation === 'add_loose_item' && input.item_id === undefined) {
			input.item_id = `i_${this.#world.mint()}`;
		}

		// When the tap was made, which is what the server judges it by — not
		// when a request that timed out gave up, or when this write's turn came.
		const written_at = this.#world.now().toISOString();

		// Nothing waiting, so nothing this could overtake: try the server.
		if (this.#holding === 0 && this.#read().queue.length === 0) {
			try {
				const answer = await this.#send(operation, input);
				this.#notice(operation, answer, input);
				return answer;
			} catch (error) {
				if (!isUnreached(error)) throw error;
				return this.#hold(operation, input, written_at, error);
			}
		}
		return this.#hold(operation, input, written_at, undefined);
	}

	/**
	 * Keep one write and answer as though it had landed. `unreached` is the
	 * failure that sent it here, rethrown where the phone cannot answer for the
	 * server either — a recipe it has never held cannot be cooked from it.
	 */
	async #hold(
		operation: OperationName,
		input: Input,
		written_at: string,
		unreached: OperationError | undefined,
	): Promise<unknown> {
		this.#holding += 1;
		try {
			return await this.#exclusive(() => this.#holdNow(operation, input, written_at, unreached));
		} finally {
			this.#holding -= 1;
			void this.flush();
		}
	}

	async #holdNow(
		operation: OperationName,
		input: Input,
		written_at: string,
		unreached: OperationError | undefined,
	): Promise<unknown> {
		const kept = this.#read();
		const cannot = () =>
			unreached ??
			new OperationError(operation, 'internal', 'Kamosu could not be reached.', {
				reached: false,
			});
		const answer = SHOPPING_WRITES.has(operation)
			? await this.#holdShopping(kept, operation, input, cannot)
			: await this.#holdAttempt(kept, operation, input, written_at, cannot);
		if (answer.send)
			kept.queue.push({ id: this.#world.mint(), operation, input: answer.send, written_at });
		// Whatever the sender marked meanwhile. In this tab it cannot have —
		// the sender marks inside the same `#exclusive` turn — but a sender in
		// another open tab can.
		kept.sending = this.#read().sending;
		this.#write(kept);
		return answer.value;
	}

	async #holdAttempt(
		kept: Kept,
		operation: OperationName,
		input: Input,
		written_at: string,
		cannot: () => OperationError,
	): Promise<{ value: unknown; send?: Input }> {
		if (operation === 'start_attempt') return this.#holdStart(kept, input, written_at, cannot);

		const id = this.#renamed(kept, input.attempt_id as string);
		const held = kept.attempts[id] ?? (await this.#fromDiary(kept, id));
		if (!held || held.gone) throw cannot();
		const attempt = { ...held.attempt, last_action_at: written_at };

		switch (operation) {
			case 'advance_attempt':
				if (input.current_step_index !== undefined)
					attempt.current_step_index = input.current_step_index as number;
				if (input.ticked_ingredients !== undefined)
					attempt.ticked_ingredients = input.ticked_ingredients as number[];
				if (input.cooking_yield !== undefined)
					attempt.cooking_yield = input.cooking_yield as Attempt['cooking_yield'];
				break;
			case 'set_as_cooked':
				// The cook's own words stay with the cooking screen, which keeps
				// them while this cooking is held (`holds`). What the server
				// reads out of them — which line changed, which was added — is
				// the server's to say once it has them (ADR 0019): nothing on
				// the phone pairs one list against another.
				if (input.as_cooked === null) attempt.as_cooked = null;
				break;
			case 'finish_attempt':
				attempt.finished_at = written_at;
				attempt.resumable = false;
				judge(attempt, input);
				break;
			case 'edit_attempt':
				judge(attempt, input);
				break;
			case 'delete_attempt': {
				// A cooking the server has never heard of goes without a word to
				// it: every write to it is simply dropped.
				// Unless that start is on its way to the server right now: then the
				// server will hear of it, and is told of the deletion after it.
				const unsent = kept.queue.some(
					(pending) =>
						pending.operation === 'start_attempt' &&
						pending.input.attempt_id === id &&
						pending.id !== kept.sending,
				);
				if (unsent) {
					kept.queue = kept.queue.filter((pending) => pending.input.attempt_id !== id);
					delete kept.attempts[id];
					return { value: { deleted: true } };
				}
				held.gone = true;
				return { value: { deleted: true }, send: { attempt_id: id } };
			}
		}
		held.attempt = attempt;
		return { value: attempt, send: { ...input, attempt_id: id } };
	}

	/**
	 * Start cooking with no network. Where this phone already knows a cooking
	 * of this dish In Progress, that is the one: starting twice is the same
	 * Attempt seen twice (ADR 0010). Otherwise the phone begins one, pinned to
	 * the Version it has in front of it, and the server hears of it later.
	 */
	async #holdStart(
		kept: Kept,
		input: Input,
		written_at: string,
		cannot: () => OperationError,
	): Promise<{ value: unknown; send?: Input }> {
		const recipe = (await this.#world
			.inner('get_recipe', { branch_id: input.branch_id })
			.catch(() => {
				throw cannot();
			})) as GetRecipeOutput;
		// Any cooking of this dish the phone knows In Progress — one it wrote
		// to offline first, or one it was answered with online before the
		// network went, which is the usual way a cook ends up here.
		const waiting = this.#waiting(kept);
		const inProgress = Object.values(kept.attempts).filter(
			(held) => held.attempt.lineage_id === recipe.lineage_id && live(held),
		);
		const known = inProgress.find((held) => waiting.has(held.attempt.id)) ?? inProgress[0];
		if (known) return { value: known.attempt };
		const current = (await this.#world
			.inner('get_current_attempt', { lineage_id: recipe.lineage_id })
			.catch(() => ({ attempt: null }))) as GetCurrentAttemptOutput;
		const title = recipe.versions.at(-1)?.content.title ?? '';
		/** What the Version cooked says it makes, for the diary (#109). */
		const writtenYield = (versionId: string) =>
			(recipe.versions.find((each) => each.version_id === versionId) ?? recipe.versions.at(-1))
				?.content.yield ?? null;
		// A cooking the read names that the phone already knows is one of
		// those over, or it would have been resumed above: however the server
		// described it before the network went, it is not one to resume.
		const onPhone = current.attempt && kept.attempts[this.#renamed(kept, current.attempt.id)];
		if (current.attempt && !onPhone) {
			kept.attempts[current.attempt.id] = {
				attempt: current.attempt,
				recipe: {
					branch_id: recipe.branch_id,
					title,
					written_yield: writtenYield(current.attempt.version_id),
				},
			};
			return { value: current.attempt };
		}

		const id = input.attempt_id as string;
		const version_id = (input.version_id as string | undefined) ?? recipe.head_version_id;
		const someone = Object.values(kept.attempts)[0]?.attempt.person_id ?? '';
		const attempt: Attempt = {
			id,
			lineage_id: recipe.lineage_id,
			person_id: someone,
			version_id,
			current_step_index: 0,
			ticked_ingredients: [],
			cooking_yield: null,
			note: null,
			rating: null,
			finished_at: null,
			resumable: true,
			created_at: written_at,
			last_action_at: written_at,
			photographs: [],
			as_cooked: null,
		};
		kept.attempts[id] = {
			attempt,
			recipe: { branch_id: recipe.branch_id, title, written_yield: writtenYield(version_id) },
		};
		// Pinned to what the phone had, which a recipe edited meanwhile at home
		// does not change: the Version cooked still exists, so this lands
		// exactly as it was cooked (ADR 0013).
		return { value: attempt, send: { ...input, version_id, started_at: written_at } };
	}

	async #holdShopping(
		kept: Kept,
		operation: OperationName,
		input: Input,
		cannot: () => OperationError,
	): Promise<{ value: unknown; send?: Input }> {
		const state = kept.shopping ?? (await this.#shoppingFromServer());
		const branchId = input.branch_id as string | undefined;
		const wanted = (input.shopping_yield ?? null) as List['chosen'][number]['shopping_yield'];
		switch (operation) {
			case 'add_to_shopping_list': {
				const chosen = state.chosen.find((entry) => entry.branch_id === branchId);
				if (chosen) {
					chosen.shopping_yield = wanted;
					break;
				}
				const basis = await this.#basis(branchId!);
				const recipe = basis
					? undefined
					: ((await this.#world
							.inner('get_recipe', { branch_id: branchId })
							.catch(() => undefined)) as GetRecipeOutput | undefined);
				const content = recipe?.versions.at(-1)?.content;
				if (!basis && !content) throw cannot();
				state.chosen.push({
					branch_id: branchId!,
					title: basis?.title ?? content!.title,
					gone: false,
					shopping_yield: wanted,
					written_yield: basis?.written_yield ?? content!.yield,
				});
				break;
			}
			case 'remove_from_shopping_list':
				state.chosen = state.chosen.filter((entry) => entry.branch_id !== branchId);
				break;
			case 'set_shopping_yield': {
				const chosen = state.chosen.find((entry) => entry.branch_id === branchId);
				if (chosen) chosen.shopping_yield = wanted;
				break;
			}
			case 'add_loose_item':
				if (!state.loose.some((item) => item.id === input.item_id))
					state.loose.push({ id: input.item_id as string, text: (input.text as string).trim() });
				break;
			case 'remove_loose_item':
				state.loose = state.loose.filter((item) => item.id !== input.item_id);
				break;
			case 'empty_shopping_list':
				state.chosen = [];
				state.loose = [];
				break;
		}
		kept.shopping = state;
		return { value: await this.#list(kept), send: input };
	}

	/**
	 * A finished cooking, read off the diary the phone last kept, so its
	 * rating and note can be written with no network: that is where they are
	 * usually written, a day after the cooking (#59, #60).
	 */
	async #fromDiary(kept: Kept, id: string): Promise<Held | undefined> {
		try {
			const diary = (await this.#world.inner('list_attempts', {})) as ListAttemptsOutput;
			const entry = diary.attempts.find((each) => each.id === id);
			if (!entry) return undefined;
			const { recipe, ...attempt } = entry;
			kept.attempts[id] = { attempt, recipe };
			return kept.attempts[id];
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			return undefined;
		}
	}

	/** The list as the phone last read it, to change. Empty where it never read one. */
	async #shoppingFromServer(): Promise<NonNullable<Kept['shopping']>> {
		try {
			const list = (await this.#world.inner('get_shopping_list', {})) as GetShoppingListOutput;
			return {
				chosen: list.chosen.map((entry) => ({ ...entry })),
				loose: list.rows
					.filter((row) => row.kind === 'loose')
					.map((row) => ({ id: row.id, text: row.name })),
			};
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			return { chosen: [], loose: [] };
		}
	}

	// ---- reading, while something is held ------------------------------------

	/** The phone's Shopping List: its choosing, and the rows added up here (`shopping.ts`). */
	async #list(kept: Kept): Promise<List> {
		const state = kept.shopping!;
		const { reading_language, reading_measures } = await this.#preferences();
		const chosen = [];
		for (const entry of state.chosen) {
			if (entry.gone) continue;
			const basis = await this.#basis(entry.branch_id);
			// A recipe the phone does not hold contributes nothing until the
			// server adds it up, rather than a guess (ADR 0024).
			if (!basis) continue;
			chosen.push({
				branch_id: entry.branch_id,
				title: entry.title,
				scale: yieldScale(entry.shopping_yield, entry.written_yield),
				lines: basis.lines,
			});
		}
		const loose: Row[] = state.loose.map((item) => ({
			id: item.id,
			kind: 'loose',
			name: item.text,
			name_language: null,
			parts: [],
			lines: [],
			// A Loose Item is never interpreted, deliberately (ADR 0024), so
			// Kamosu has nothing to say about it.
			said: null,
		}));
		return {
			chosen: state.chosen,
			rows: rows(chosen, loose, reading_measures, reading_language),
		};
	}

	async #basis(branchId: string): Promise<ShoppingBasisOutput | undefined> {
		try {
			return (await this.#world.inner('shopping_basis', {
				branch_id: branchId,
			})) as ShoppingBasisOutput;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			return undefined;
		}
	}

	async #preferences(): Promise<GetReadingPreferencesOutput> {
		try {
			return (await this.#world.inner(
				'get_reading_preferences',
				{},
			)) as GetReadingPreferencesOutput;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			// The stated default, as the server's own is (ADR 0016).
			return { reading_language: 'en', reading_measures: 'us' };
		}
	}

	/** The diary as the server last said it, with this phone's waiting cookings laid over it. */
	async #diary(kept: Kept): Promise<ListAttemptsOutput> {
		let entries: DiaryEntry[] = [];
		try {
			const diary = (await this.#world.inner('list_attempts', {})) as ListAttemptsOutput;
			entries = diary.attempts;
		} catch (error) {
			if (!isUnreached(error)) throw error;
		}
		const waiting = this.#waiting(kept);
		const byId = new Map(entries.map((entry) => [entry.id, entry]));
		for (const id of waiting) {
			const held = kept.attempts[id];
			if (!held) continue;
			if (held.gone) {
				byId.delete(id);
				continue;
			}
			byId.set(id, {
				...held.attempt,
				recipe: byId.get(id)?.recipe ?? (await this.#named(held.recipe, held.attempt.version_id)),
				// Whether there are changes still to keep is the server's to
				// say (#210), and keeping them needs it. Until it has spoken
				// about this cooking, nothing is offered.
				unkept: byId.get(id)?.unkept ?? null,
			});
		}
		const attempts = [...byId.values()].sort((a, b) =>
			a.created_at === b.created_at ? (a.id < b.id ? 1 : -1) : a.created_at < b.created_at ? 1 : -1,
		);
		return { attempts };
	}

	/**
	 * A diary entry's recipe with its name. A cooking the phone only learned of
	 * from the server's answer to starting it knows which recipe, not what it
	 * is called, and the recipe itself is on the phone to say. One learned
	 * some other way knows neither and stays nameless; no screen reaches that
	 * today, because a cooking corrected from the diary is named by the diary.
	 *
	 * What the Version cooked says it makes comes from the same copy (#109), so
	 * the diary can say how much was cooked beside it.
	 */
	async #named(recipe: Held['recipe'], versionId: string): Promise<Held['recipe']> {
		if (recipe.title !== '' || recipe.branch_id === null) return recipe;
		try {
			const read = (await this.#world.inner('get_recipe', {
				branch_id: recipe.branch_id,
			})) as GetRecipeOutput;
			const cooked =
				read.versions.find((each) => each.version_id === versionId) ?? read.versions.at(-1);
			return {
				...recipe,
				title: read.versions.at(-1)?.content.title ?? '',
				written_yield: cooked?.content.yield ?? null,
			};
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			return recipe;
		}
	}

	/** The cooking this phone holds for a dish, if it holds writes to one. */
	#heldFor(kept: Kept, lineageId: string): Held | undefined {
		const waiting = this.#waiting(kept);
		const ofThisDish = Object.values(kept.attempts).filter(
			(held) => held.attempt.lineage_id === lineageId && waiting.has(held.attempt.id),
		);
		// The one still In Progress, where there is one: a dish cooked and
		// finished this afternoon and started again is two cookings, and the
		// second is the one the cook is standing in (ADR 0010).
		return ofThisDish.find(live) ?? ofThisDish[0];
	}

	/** Every cooking with a write still waiting. */
	#waiting(kept: Kept): Set<string> {
		return new Set(
			kept.queue
				.filter((pending) => ATTEMPT_WRITES.has(pending.operation))
				.map((pending) => this.#renamed(kept, pending.input.attempt_id as string)),
		);
	}

	#renamed(kept: Kept, id: string): string {
		return kept.renamed[id] ?? id;
	}

	/**
	 * Remember the cookings the server answers with, so a move made after the
	 * network goes has something to move, and remember which are over, so a
	 * stale read cannot resume one — but never over a cooking this phone is
	 * still holding writes to, which the server has not heard yet.
	 */
	#notice(operation: OperationName, answer: unknown, input: Input = {}): void {
		const kept = this.#read();
		if (operation === 'delete_attempt') {
			// Thrown away online: remembered as thrown away, so a read the phone
			// kept from before cannot hand it back to a cook with no network.
			const held = kept.attempts[this.#renamed(kept, input.attempt_id as string)];
			if (held) held.gone = true;
			this.#settle(kept);
			this.#write(kept);
			return;
		}
		const attempt =
			operation === 'get_current_attempt'
				? (answer as GetCurrentAttemptOutput).attempt
				: ATTEMPT_WRITES.has(operation)
					? (answer as Attempt)
					: null;
		const waiting = this.#waiting(kept);
		if (!attempt || waiting.has(attempt.id)) return;
		// One In Progress per Lineage (ADR 0010): the server answering with
		// this one means any other the phone had for the dish is over.
		if (attempt.finished_at === null) {
			for (const [id, held] of Object.entries(kept.attempts)) {
				if (
					id !== attempt.id &&
					held.attempt.lineage_id === attempt.lineage_id &&
					live(held) &&
					!waiting.has(id)
				)
					// Marked over rather than forgotten, so an older copy of a
					// read cannot hand it back (`#settle` keeps a few).
					held.gone = true;
			}
		}
		// A finished one is kept too, for the same reason as a thrown-away one.
		const before = kept.attempts[attempt.id]?.recipe;
		kept.attempts[attempt.id] = {
			attempt,
			recipe: {
				branch_id: before?.branch_id ?? (input.branch_id as string | undefined) ?? null,
				title: before?.title ?? '',
				written_yield: before?.written_yield ?? null,
			},
		};
		this.#settle(kept);
		this.#write(kept);
	}

	/**
	 * Keep only the newest cookings that are over — finished or thrown away,
	 * with nothing left to send. They are remembered only so that a stale read
	 * cannot resume them, and a few are all that could be.
	 */
	#settle(kept: Kept): void {
		const waiting = this.#waiting(kept);
		const over = Object.values(kept.attempts)
			.filter((held) => !waiting.has(held.attempt.id) && !live(held))
			.sort((a, b) => (a.attempt.last_action_at < b.attempt.last_action_at ? 1 : -1));
		for (const held of over.slice(SETTLED_KEPT)) delete kept.attempts[held.attempt.id];
	}

	// ---- sending ----------------------------------------------------------

	/** One write to the server, with the phone's names swapped for the server's. */
	async #send(operation: OperationName, input: Input): Promise<unknown> {
		const kept = this.#read();
		const sent = { ...input };
		if (typeof sent.attempt_id === 'string' && operation !== 'start_attempt') {
			sent.attempt_id = this.#renamed(kept, sent.attempt_id);
		}
		for (const field of ['photographs', 'add_photographs']) {
			const names = sent[field];
			if (Array.isArray(names)) sent[field] = await this.#photographed(names as string[]);
		}
		return this.#world.inner(operation, sent);
	}

	/**
	 * The server's names for a cooking's pictures, sending any it has not had
	 * yet. This is where a photograph taken with no network reaches the door,
	 * is remade there, and is named by its bytes (ADR 0017).
	 */
	async #photographed(names: string[]): Promise<string[]> {
		const out = [];
		for (const name of names) {
			if (!name.startsWith(KEPT_PICTURE)) {
				out.push(name);
				continue;
			}
			const kept = this.#read();
			let id = kept.photographs[name];
			if (!id) {
				const picture = await this.#world.pictures.read(name);
				// Gone from the phone — its storage cleared — so there is nothing
				// left to send, and a name pointing at nothing is refused.
				if (!picture) continue;
				id = await this.#world.upload(picture);
				const now = this.#read();
				now.photographs[name] = id;
				this.#write(now);
			}
			out.push(id);
		}
		return out;
	}

	/**
	 * Send everything waiting, in the order it was written, until the server
	 * stops answering. A write the server refuses is dropped: it would be
	 * refused again, and holding it would hold everything behind it. Nothing
	 * is sent while the Session has ended: what it wrote waits for it.
	 */
	flush(): Promise<void> {
		// Asked again while sending — the server just came back, or a write
		// arrived after the sender last looked — so go round once more when
		// this pass ends rather than trusting a pass that may have given up.
		if (this.#sending) {
			this.#again = true;
			return this.#sending;
		}
		this.#sending = (async () => {
			try {
				for (;;) {
					this.#again = false;
					let stopped = false;
					await (this.#world.lock ?? ((work) => work()))(async () => {
						stopped = await this.#flush();
					});
					const more = !stopped && this.#read().queue.length > 0;
					if (!this.#again && !more) break;
				}
			} finally {
				this.#sending = undefined;
			}
		})();
		return this.#sending;
	}

	/** Send what is waiting. Answers whether the server stopped answering. */
	async #flush(): Promise<boolean> {
		let sentAny = false;
		for (;;) {
			// Chosen and marked on its way in one turn of `#exclusive`, so a
			// write being held — a deletion of this very cooking, say — sees the
			// queue either before this leaves or after it is marked, never
			// halfway. Held writes carry the mark forward.
			const next = await this.#exclusive(() => {
				const kept = this.#read();
				const first = kept.queue[0];
				if (first) this.#write({ ...kept, sending: first.id });
				return first;
			});
			if (!next) break;
			const input = SAYS_WHEN.has(next.operation)
				? { ...next.input, written_at: next.written_at }
				: next.input;
			let answer: unknown;
			try {
				answer = await this.#send(next.operation, input);
			} catch (error) {
				await this.#exclusive(() => this.#write({ ...this.#read(), sending: undefined }));
				if (isUnreached(error)) return true;
				// A Session that has ended cannot send anything; what it wrote
				// waits for it rather than being thrown away one write at a time.
				if (error instanceof OperationError && error.kind === 'unauthorized') return true;
				console.warn(`Kamosu refused ${next.operation} written offline`, error);
			}
			await this.#exclusive(() => this.#sent(next, answer));
			sentAny = true;
		}
		if (!sentAny) return false;
		return this.#exclusive(() => this.#level());
	}

	/** Note that a waiting write has gone, and what the server named in answer. */
	#sent(next: Pending, answer: unknown): void {
		const after = this.#read();
		if (next.operation === 'start_attempt' && answer) {
			const given = next.input.attempt_id as string;
			if ((answer as Attempt).id !== given) rename(after, given, answer as Attempt);
		}
		after.queue = after.queue.filter((pending) => pending.id !== next.id);
		after.sending = undefined;
		this.#write(after);
	}

	/**
	 * Level with the server: what the phone was holding is now the server's
	 * to say, whatever it said about it. Unless something was written while
	 * the last write was on its way, which the next pass sends.
	 */
	async #level(): Promise<boolean> {
		const done = this.#read();
		if (done.queue.length > 0) return false;
		done.shopping = null;
		this.#settle(done);
		// Which cooking the server answered for one begun here, and which
		// Photograph it made of each picture, stay known for the whole Session:
		// a screen still showing the phone's name for either goes on sending
		// and drawing it, and neither is a thing to be wrong about.
		const pictures = Object.keys(done.photographs);
		this.#write(done);
		await Promise.all(pictures.map((name) => this.#world.pictures.forget(name)));
		this.#world.told?.([
			'get_shopping_list',
			'shopping_list_as_text',
			'list_attempts',
			'get_current_attempt',
			'get_recipe',
		]);
		return false;
	}

	// ---- what a screen may ask ------------------------------------------------

	holds(attemptId: string): boolean {
		const kept = this.#read();
		return this.#waiting(kept).has(this.#renamed(kept, attemptId));
	}

	async keepPhotograph(picture: Blob): Promise<string> {
		const name = `${KEPT_PICTURE}${this.#world.mint()}`;
		await this.#world.pictures.keep(name, picture);
		return name;
	}

	async photographSrc(id: string, size: 'card' | 'page'): Promise<string | undefined> {
		const known = id.startsWith(KEPT_PICTURE) ? this.#read().photographs[id] : id;
		if (known) return `/api/photographs/${known}/${size}`;
		const drawn = this.#drawn.get(id);
		if (drawn) return drawn;
		const picture = await this.#world.pictures.read(id);
		if (!picture) return undefined;
		const url = URL.createObjectURL(picture);
		this.#drawn.set(id, url);
		return url;
	}
}

/**
 * Everything written to the cooking the phone called `was` goes to `to` from
 * now on, and the phone's copy moves across: the server answered a start with
 * a cooking it already had In Progress (ADR 0010).
 */
function rename(kept: Kept, was: string, to: Attempt): void {
	kept.renamed[was] = to.id;
	const held = kept.attempts[was];
	if (held) kept.attempts[to.id] = { ...held, attempt: { ...held.attempt, id: to.id } };
	delete kept.attempts[was];
}

/** A rating, a note and pictures, laid over a cooking as the server lays them. */
function judge(attempt: Attempt, input: Input): void {
	if (input.note !== undefined) attempt.note = (input.note as string | null)?.trim() || null;
	if (input.rating !== undefined) attempt.rating = input.rating as Attempt['rating'];
	if (input.photographs !== undefined)
		attempt.photographs = (input.photographs as string[] | null) ?? [];
	if (Array.isArray(input.add_photographs)) {
		const held = [...attempt.photographs];
		for (const name of input.add_photographs as string[]) if (!held.includes(name)) held.push(name);
		attempt.photographs = held;
	}
}

// ---- the browser's own storage --------------------------------------------------

const KEY = 'kamosu.outbox';
const PICTURES_CACHE = 'kamosu-outbox-pictures';

/** The queue, in `localStorage`: small, and read by every tab alike. */
export const browserStorage: Storage = {
	load() {
		try {
			const raw = localStorage.getItem(KEY);
			return raw ? (JSON.parse(raw) as Kept) : undefined;
		} catch {
			return undefined;
		}
	},
	save(kept) {
		try {
			localStorage.setItem(KEY, JSON.stringify(kept));
		} catch {
			// Storage refused. What was written still goes to the server when it
			// can be reached; it just is not kept across a reload.
		}
	},
};

/**
 * Pictures, in a cache of their own, which the service worker never empties.
 * Outside a secure page there is no cache at all, and a picture waits in
 * memory for as long as the page is open.
 */
export function browserPictures(): Pictures {
	const memory = new Map<string, Blob>();
	const url = (name: string) => `${location.origin}/__kamosu/picture/${encodeURIComponent(name)}`;
	const store = () => (typeof caches === 'undefined' ? undefined : caches.open(PICTURES_CACHE));
	return {
		async keep(name, picture) {
			const cache = await store();
			if (cache) await cache.put(url(name), new Response(picture));
			else memory.set(name, picture);
		},
		async read(name) {
			const cache = await store();
			const kept = await cache?.match(url(name));
			return kept ? await kept.blob() : memory.get(name);
		},
		async forget(name) {
			const cache = await store();
			await cache?.delete(url(name));
			memory.delete(name);
		},
	};
}

/** Forget everything held, for a new Session: none of it is theirs. */
export async function forgetEverythingHeld(): Promise<void> {
	try {
		localStorage.removeItem(KEY);
		// And a cooking's own words, kept while it waited (`as-cooked.svelte.ts`).
		for (const key of Object.keys(localStorage)) {
			if (key.startsWith('kamosu.as-cooked.')) localStorage.removeItem(key);
		}
	} catch {
		// Nothing kept, then nothing to forget.
	}
	if (typeof caches !== 'undefined') await caches.delete(PICTURES_CACHE).catch(() => false);
}

/** Sixteen lower-case hex digits. */
export function mintHex(): string {
	const bytes = crypto.getRandomValues(new Uint8Array(8));
	return [...bytes].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

/** One sender at a time across every tab, where the browser can arrange it. */
export function browserLock(work: () => Promise<void>): Promise<void> {
	const locks = (navigator as Navigator & { locks?: LockManager }).locks;
	return locks ? locks.request('kamosu-outbox', work) : work();
}

/** The one outbox the app runs on, made once by the root layout. */
export function realOutbox(): Outbox {
	return new Outbox({
		inner: httpTransport({ observe: noteReach }),
		storage: browserStorage,
		pictures: browserPictures(),
		upload: realPhotographUpload(),
		now: () => new Date(),
		mint: mintHex,
		lock: browserLock,
		told: (operations) => {
			for (const operation of operations)
				refreshed.set(operation, (refreshed.get(operation) ?? 0) + 1);
		},
	});
}

const KEEPING = Symbol('keeping');

export function provideKeeping(keeping: () => Keeping): void {
	setContext(KEEPING, keeping);
}

/** What a screen may ask the outbox: about photographs, and whether a cooking is still waiting. */
export function useKeeping(): Keeping {
	const keeping = getContext<(() => Keeping) | undefined>(KEEPING)?.();
	if (!keeping) throw new Error('no outbox in context — render inside <Kamosu>');
	return keeping;
}
