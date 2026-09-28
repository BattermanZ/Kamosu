/**
 * What a phone with no network may still write (#77, ADR 0013), against a
 * pretend server that can be switched off and a pretend phone that keeps
 * what it last read — which is what the service worker is.
 */

import { describe, expect, it, vi } from 'vitest';
import type {
	GetRecipeOutput,
	OperationName,
	ShoppingBasisOutput,
	StartAttemptOutput,
} from '$lib/api/catalogue';
import { OperationError } from '$lib/api/client';
import { Outbox, type Kept, type Pictures } from './outbox';

const attempt = (over: Partial<StartAttemptOutput> = {}): StartAttemptOutput => ({
	id: 'at_server000000001',
	lineage_id: 'l_1',
	person_id: 'p_1',
	version_id: 'v_1',
	current_step_index: 0,
	ticked_ingredients: [],
	cooking_yield: null,
	note: null,
	rating: null,
	finished_at: null,
	resumable: true,
	created_at: '2026-09-19T10:00:00.000Z',
	last_action_at: '2026-09-19T10:00:00.000Z',
	photographs: [],
	as_cooked: null,
	...over,
});

const recipe = {
	branch_id: 'b_1',
	lineage_id: 'l_1',
	head_version_id: 'v_2',
	versions: [{ content: { title: 'Katsu Curry', yield: { amount: '4', noun: 'servings' } } }],
} as unknown as GetRecipeOutput;

const basis = (branch_id: string, title: string, grams: number): ShoppingBasisOutput => ({
	branch_id,
	title,
	written_yield: { amount: '4', noun: 'servings' },
	lines: [
		{
			path: [0],
			from: null,
			said: null,
			text: `${grams} g flour`,
			food: {
				id: 'f_flour',
				name: 'flour',
				name_language: 'en',
				amount: grams,
				unit: 'g',
				unit_id: 'gram',
				unit_key: 'g',
				cup_weight_grams: 125,
			},
		},
	],
});

/**
 * A server that can be switched off, behind a phone that answers the reads
 * it has already read when the server cannot — the service worker's rule.
 */
function world(
	answers: Partial<Record<OperationName, (input: Record<string, unknown>) => unknown>>,
) {
	let online = true;
	const sent: { operation: OperationName; input: Record<string, unknown> }[] = [];
	const kept = new Map<string, unknown>();
	let stored: Kept | undefined;
	const pictures = new Map<string, Blob>();
	const uploaded: Blob[] = [];
	let minted = 0;
	let clock = Date.parse('2026-09-19T12:00:00.000Z');

	const inner = async (operation: OperationName, input: unknown) => {
		const key = `${operation} ${JSON.stringify(input)}`;
		if (!online) {
			if (kept.has(key)) return structuredClone(kept.get(key));
			throw new OperationError(operation, 'internal', 'Kamosu could not be reached.', {
				reached: false,
			});
		}
		sent.push({ operation, input: input as Record<string, unknown> });
		// An answer may be a promise, so a test can hold a request on its way.
		const answer = await answers[operation]?.(input as Record<string, unknown>);
		if (answer instanceof OperationError) throw answer;
		kept.set(key, answer);
		return structuredClone(answer);
	};
	const picturesStore: Pictures = {
		keep: async (name, picture) => void pictures.set(name, picture),
		read: async (name) => pictures.get(name),
		forget: async (name) => void pictures.delete(name),
	};
	const told = vi.fn();
	const outbox = new Outbox({
		inner,
		storage: {
			load: () => structuredClone(stored),
			save: (value) => (stored = structuredClone(value)),
		},
		pictures: picturesStore,
		upload: async (picture) => {
			if (!online)
				throw new OperationError('upload', 'internal', 'Kamosu could not be reached.', {
					reached: false,
				});
			uploaded.push(picture);
			return `p_${uploaded.length}`;
		},
		now: () => new Date((clock += 60_000)),
		mint: () => (minted += 1).toString(16).padStart(16, '0'),
		told,
	});
	return {
		outbox,
		// Loose on purpose: each Operation answers its own shape, and every test
		// below checks the fields it is about rather than the type.
		call: (operation: OperationName, input: Record<string, unknown> = {}) =>
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			outbox.transport(operation, input) as Promise<any>,
		sent,
		pictures,
		uploaded,
		told,
		kept: () => stored,
		offline: () => (online = false),
		online: () => (online = true),
	};
}

describe('writing with no network', () => {
	it('sends straight to the server when it can, and keeps nothing', async () => {
		const at = world({
			advance_attempt: (input) =>
				attempt({ current_step_index: input.current_step_index as number }),
		});
		const moved = await at.call('advance_attempt', {
			attempt_id: 'at_server000000001',
			current_step_index: 1,
		});
		expect(moved.current_step_index).toBe(1);
		expect(at.sent).toHaveLength(1);
		expect(at.kept()?.queue ?? []).toEqual([]);
	});

	it('passes a refusal straight back: the server answered, and would again', async () => {
		const at = world({
			advance_attempt: () => new OperationError('advance_attempt', 'bad_request', 'finished'),
		});
		await expect(
			at.call('advance_attempt', { attempt_id: 'at_server000000001', current_step_index: 1 }),
		).rejects.toMatchObject({ kind: 'bad_request' });
		expect(at.kept()?.queue ?? []).toEqual([]);
	});

	it('cooks a whole dish with the server off, then sends it all in order, each saying when', async () => {
		const at = world({
			get_recipe: () => recipe,
			get_current_attempt: () => ({ attempt: null }),
			start_attempt: (input) =>
				attempt({ id: input.attempt_id as string, version_id: input.version_id as string }),
			advance_attempt: (input) => attempt({ id: input.attempt_id as string }),
			set_as_cooked: (input) => attempt({ id: input.attempt_id as string }),
			finish_attempt: (input) => attempt({ id: input.attempt_id as string, finished_at: 'x' }),
			edit_attempt: (input) => attempt({ id: input.attempt_id as string, finished_at: 'x' }),
			list_attempts: () => ({ attempts: [] }),
		});
		// Read once while online, as opening the recipe does: now the phone has it.
		await at.call('get_recipe', { branch_id: 'b_1' });
		await at.call('get_current_attempt', { lineage_id: 'l_1' });
		await at.call('list_attempts');
		at.offline();

		const started = await at.call('start_attempt', { branch_id: 'b_1' });
		expect(started.id).toBe('at_0000000000000001');
		expect(started.version_id).toBe('v_2');
		const moved = await at.call('advance_attempt', {
			attempt_id: started.id,
			current_step_index: 2,
			ticked_ingredients: [0],
		});
		expect(moved).toMatchObject({ current_step_index: 2, ticked_ingredients: [0] });
		await at.call('set_as_cooked', { attempt_id: started.id, as_cooked: { title: 'Katsu Curry' } });
		const finished = await at.call('finish_attempt', { attempt_id: started.id });
		expect(finished.finished_at).not.toBeNull();
		await at.call('edit_attempt', {
			attempt_id: started.id,
			rating: 'again',
			note: 'Chez mes parents',
		});

		// Read back with the server still off: the phone answers from what it holds.
		expect(at.outbox.holds(started.id)).toBe(true);
		const current = await at.call('get_current_attempt', { lineage_id: 'l_1' });
		expect(current.attempt).toBeNull();
		const diary = await at.call('list_attempts');
		expect(diary.attempts).toHaveLength(1);
		expect(diary.attempts[0]).toMatchObject({
			id: started.id,
			rating: 'again',
			note: 'Chez mes parents',
			recipe: { branch_id: 'b_1', title: 'Katsu Curry' },
		});

		const before = at.sent.length;
		at.online();
		await at.outbox.flush();
		const sent = at.sent.slice(before);
		expect(sent.map((each) => each.operation)).toEqual([
			'start_attempt',
			'advance_attempt',
			'set_as_cooked',
			'finish_attempt',
			'edit_attempt',
		]);
		expect(sent[0].input).toMatchObject({
			attempt_id: started.id,
			version_id: 'v_2',
			started_at: started.created_at,
		});
		expect(sent[0].input.written_at).toBeUndefined();
		const whens = sent.slice(1).map((each) => each.input.written_at as string);
		expect(whens.every(Boolean)).toBe(true);
		expect([...whens].sort()).toEqual(whens);
		expect(at.kept()?.queue).toEqual([]);
		expect(at.outbox.holds(started.id)).toBe(false);
		expect(at.told).toHaveBeenCalledWith(expect.arrayContaining(['list_attempts']));
	});

	it('keeps how much a cooking started offline is making, and sends it once there is a network (#109)', async () => {
		const at = world({
			get_recipe: () => recipe,
			get_current_attempt: () => ({ attempt: null }),
			start_attempt: (input) =>
				attempt({ id: input.attempt_id as string, version_id: input.version_id as string }),
			advance_attempt: (input) =>
				attempt({
					id: input.attempt_id as string,
					cooking_yield: input.cooking_yield as { amount: string; noun: string },
				}),
		});
		await at.call('get_recipe', { branch_id: 'b_1' });
		at.offline();

		const started = await at.call('start_attempt', { branch_id: 'b_1' });
		const twice = { amount: '2', noun: '' };
		const held = await at.call('advance_attempt', {
			attempt_id: started.id,
			cooking_yield: twice,
		});
		// The phone says it at once, and the screen shows it chosen.
		expect(held.cooking_yield).toEqual(twice);

		at.online();
		await at.outbox.flush();
		const sent = at.sent.filter((each) => each.operation === 'advance_attempt');
		expect(sent).toHaveLength(1);
		expect(sent[0].input).toMatchObject({ attempt_id: started.id, cooking_yield: twice });
		expect(sent[0].input.written_at).toBeTruthy();
		// And the recipe is read again, which is how the amounts arrive scaled.
		expect(at.told).toHaveBeenCalledWith(expect.arrayContaining(['get_recipe']));
	});

	it('follows the cooking another device began, whatever the phone called its own', async () => {
		const at = world({
			get_recipe: () => recipe,
			get_current_attempt: () => ({ attempt: null }),
			start_attempt: () => attempt({ id: 'at_fromtheipad0001' }),
			advance_attempt: (input) => attempt({ id: input.attempt_id as string }),
		});
		await at.call('get_recipe', { branch_id: 'b_1' });
		await at.call('get_current_attempt', { lineage_id: 'l_1' });
		at.offline();
		const started = await at.call('start_attempt', { branch_id: 'b_1' });
		await at.call('advance_attempt', { attempt_id: started.id, current_step_index: 1 });
		at.online();
		await at.outbox.flush();
		expect(at.sent.at(-1)).toMatchObject({
			operation: 'advance_attempt',
			input: { attempt_id: 'at_fromtheipad0001' },
		});
	});

	it('drops a write the server refuses and sends the ones behind it', async () => {
		const at = world({
			advance_attempt: () => new OperationError('advance_attempt', 'bad_request', 'out of range'),
			add_loose_item: () => ({ chosen: [], rows: [] }),
			get_shopping_list: () => ({ chosen: [], rows: [] }),
			get_reading_preferences: () => ({ reading_language: 'en', reading_measures: 'metric' }),
			get_current_attempt: () => ({ attempt: attempt() }),
		});
		await at.call('get_current_attempt', { lineage_id: 'l_1' });
		await at.call('get_shopping_list');
		await at.call('get_reading_preferences');
		at.offline();
		await at.call('advance_attempt', { attempt_id: 'at_server000000001', current_step_index: 1 });
		await at.call('add_loose_item', { text: 'bin bags' });
		at.online();
		const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
		await at.outbox.flush();
		warn.mockRestore();
		expect(at.sent.map((each) => each.operation).slice(-2)).toEqual([
			'advance_attempt',
			'add_loose_item',
		]);
		expect(at.kept()?.queue).toEqual([]);
	});

	it('forgets an unsent cooking outright when it is thrown away', async () => {
		const at = world({ get_recipe: () => recipe, get_current_attempt: () => ({ attempt: null }) });
		await at.call('get_recipe', { branch_id: 'b_1' });
		await at.call('get_current_attempt', { lineage_id: 'l_1' });
		at.offline();
		const started = await at.call('start_attempt', { branch_id: 'b_1' });
		await at.call('advance_attempt', { attempt_id: started.id, current_step_index: 1 });
		expect(await at.call('delete_attempt', { attempt_id: started.id })).toEqual({ deleted: true });
		expect(at.kept()?.queue).toEqual([]);
	});

	it('cannot cook a recipe the phone has never held', async () => {
		const at = world({});
		at.offline();
		await expect(at.call('start_attempt', { branch_id: 'b_1' })).rejects.toMatchObject({
			reached: false,
		});
	});
});

describe('a photograph taken with no network', () => {
	it('waits on the phone, is shown from there, and is named by the server when it arrives', async () => {
		const at = world({
			get_current_attempt: () => ({ attempt: attempt() }),
			edit_attempt: (input) => attempt({ photographs: input.add_photographs as string[] }),
		});
		await at.call('get_current_attempt', { lineage_id: 'l_1' });
		at.offline();
		const name = await at.outbox.keepPhotograph(new Blob(['a picture']));
		const edited = await at.call('edit_attempt', {
			attempt_id: 'at_server000000001',
			add_photographs: [name],
		});
		expect(edited.photographs).toEqual([name]);
		globalThis.URL.createObjectURL ??= () => 'blob:kept';
		expect(await at.outbox.photographSrc(name, 'card')).toMatch(/^blob:/);

		at.online();
		await at.outbox.flush();
		expect(at.uploaded).toHaveLength(1);
		expect(at.sent.at(-1)).toMatchObject({
			operation: 'edit_attempt',
			input: { add_photographs: ['p_1'] },
		});
		expect(at.pictures.size).toBe(0);
		expect(await at.outbox.photographSrc('p_1', 'page')).toBe('/api/photographs/p_1/page');
	});
});

describe('the Shopping List with no network', () => {
	it('works the rows out on the phone while changes wait, and hands back to the server after', async () => {
		const at = world({
			get_shopping_list: () => ({
				chosen: [
					{
						branch_id: 'b_tart',
						title: 'Tarte',
						gone: false,
						shopping_yield: null,
						written_yield: { amount: '4', noun: 'servings' },
					},
				],
				rows: [],
			}),
			get_reading_preferences: () => ({ reading_language: 'en', reading_measures: 'metric' }),
			shopping_basis: (input) =>
				input.branch_id === 'b_tart' ? basis('b_tart', 'Tarte', 500) : basis('b_cake', 'Cake', 700),
			add_to_shopping_list: () => ({ chosen: [], rows: [] }),
			set_shopping_yield: () => ({ chosen: [], rows: [] }),
			add_loose_item: () => ({ chosen: [], rows: [] }),
		});
		for (const read of ['get_shopping_list', 'get_reading_preferences'] as const)
			await at.call(read);
		await at.call('shopping_basis', { branch_id: 'b_tart' });
		await at.call('shopping_basis', { branch_id: 'b_cake' });
		at.offline();

		await at.call('add_to_shopping_list', { branch_id: 'b_cake' });
		await at.call('set_shopping_yield', {
			branch_id: 'b_tart',
			shopping_yield: { amount: '8', noun: 'servings' },
		});
		const list = await at.call('add_loose_item', { text: 'bin bags' });
		expect(list.chosen.map((entry: { title: string }) => entry.title)).toEqual(['Tarte', 'Cake']);
		expect(list.rows.map((row: { name: string }) => row.name)).toEqual(['bin bags', 'flour']);
		expect(list.rows[1].parts[0].text).toBe('about 1.7 kg');

		const read = await at.call('get_shopping_list');
		expect(read).toEqual(list);
		const text = await at.call('shopping_list_as_text');
		expect(text.text).toContain('- [ ] flour — about 1.7 kg');

		at.online();
		await at.outbox.flush();
		expect(at.sent.slice(-3).map((each) => each.operation)).toEqual([
			'add_to_shopping_list',
			'set_shopping_yield',
			'add_loose_item',
		]);
		expect(at.sent.at(-1)?.input).toMatchObject({
			text: 'bin bags',
			item_id: expect.stringMatching(/^i_/),
		});
		expect(at.kept()?.shopping).toBeNull();
	});
});

describe('two things at once', () => {
	it('goes on sending to the cooking the server answered with, after everything waiting has gone', async () => {
		const at = world({
			get_recipe: () => recipe,
			get_current_attempt: () => ({ attempt: null }),
			start_attempt: () => attempt({ id: 'at_fromtheipad0001' }),
			advance_attempt: (input) => attempt({ id: input.attempt_id as string }),
		});
		await at.call('get_recipe', { branch_id: 'b_1' });
		await at.call('get_current_attempt', { lineage_id: 'l_1' });
		at.offline();
		const started = await at.call('start_attempt', { branch_id: 'b_1' });
		at.online();
		await at.outbox.flush();
		// The cooking screen still holds the phone's name for it.
		await at.call('advance_attempt', { attempt_id: started.id, current_step_index: 2 });
		expect(at.sent.at(-1)).toMatchObject({
			operation: 'advance_attempt',
			input: { attempt_id: 'at_fromtheipad0001' },
		});
	});

	it('loses no write to two taps made at once', async () => {
		const at = world({
			get_shopping_list: () => ({ chosen: [], rows: [] }),
			get_reading_preferences: () => ({ reading_language: 'en', reading_measures: 'metric' }),
			add_loose_item: () => ({ chosen: [], rows: [] }),
		});
		await at.call('get_shopping_list');
		await at.call('get_reading_preferences');
		at.offline();
		await Promise.all([
			at.call('add_loose_item', { text: 'bin bags' }),
			at.call('add_loose_item', { text: 'coffee' }),
			at.call('add_loose_item', { text: 'tea' }),
		]);
		expect(at.kept()?.queue.map((pending) => pending.input.text)).toEqual([
			'bin bags',
			'coffee',
			'tea',
		]);
		const list = await at.call('get_shopping_list');
		expect(list.rows).toHaveLength(3);
	});
});

describe('starting a dish again', () => {
	it('resumes the cooking in progress, not one already finished of the same dish', async () => {
		const at = world({ get_recipe: () => recipe, get_current_attempt: () => ({ attempt: null }) });
		await at.call('get_recipe', { branch_id: 'b_1' });
		await at.call('get_current_attempt', { lineage_id: 'l_1' });
		at.offline();
		const first = await at.call('start_attempt', { branch_id: 'b_1' });
		await at.call('finish_attempt', { attempt_id: first.id });
		const second = await at.call('start_attempt', { branch_id: 'b_1' });
		await at.call('advance_attempt', { attempt_id: second.id, current_step_index: 1 });

		// The cook leaves the screen and comes back to it.
		const again = await at.call('start_attempt', { branch_id: 'b_1' });
		expect(again.id).toBe(second.id);
		expect(again.current_step_index).toBe(1);
		const current = await at.call('get_current_attempt', { lineage_id: 'l_1' });
		expect(current.attempt?.id).toBe(second.id);
	});
});

describe('starting a dish again, from what the phone last read', () => {
	it('never resumes a cooking finished here, whatever the phone last read about it', async () => {
		const at = world({
			get_recipe: () => recipe,
			// Online, the phone read this cooking as In Progress, and kept that.
			get_current_attempt: () => ({ attempt: attempt() }),
		});
		await at.call('get_recipe', { branch_id: 'b_1' });
		await at.call('get_current_attempt', { lineage_id: 'l_1' });
		at.offline();
		const first = await at.call('start_attempt', { branch_id: 'b_1' });
		expect(first.id).toBe('at_server000000001');
		await at.call('finish_attempt', { attempt_id: first.id });

		const again = await at.call('start_attempt', { branch_id: 'b_1' });
		expect(again.id).not.toBe(first.id);
		expect(again.finished_at).toBeNull();
		const diary = at.kept()!.attempts[first.id].attempt;
		expect(diary.finished_at).not.toBeNull();
	});
});

describe('throwing a cooking away while its start is on its way', () => {
	it('tells the server after the start rather than losing a write', async () => {
		let arrive: () => void = () => {};
		const onItsWay = new Promise<void>((resolve) => (arrive = resolve));
		let holding = true;
		const at = world({
			get_recipe: () => recipe,
			get_current_attempt: () => ({ attempt: null }),
			start_attempt: async (input) => {
				if (holding) await onItsWay;
				return attempt({ id: input.attempt_id as string });
			},
			delete_attempt: () => ({ deleted: true }),
			add_loose_item: () => ({ chosen: [], rows: [] }),
			get_shopping_list: () => ({ chosen: [], rows: [] }),
			get_reading_preferences: () => ({ reading_language: 'en', reading_measures: 'metric' }),
		});
		for (const read of ['get_shopping_list', 'get_reading_preferences'] as const)
			await at.call(read);
		await at.call('get_recipe', { branch_id: 'b_1' });
		await at.call('get_current_attempt', { lineage_id: 'l_1' });
		at.offline();
		const started = await at.call('start_attempt', { branch_id: 'b_1' });
		await at.call('add_loose_item', { text: 'bin bags' });
		await at.outbox.flush();
		at.online();

		// The start leaves and is held on its way; the cook throws it away.
		const sending = at.outbox.flush();
		await vi.waitFor(() => expect(at.kept()?.sending).toBeDefined());
		await at.call('delete_attempt', { attempt_id: started.id });
		holding = false;
		arrive();
		await sending;
		await at.outbox.flush();

		const sent = at.sent.map((each) => each.operation);
		expect(sent.slice(sent.indexOf('start_attempt'))).toEqual([
			'start_attempt',
			'add_loose_item',
			'delete_attempt',
		]);
		expect(at.kept()?.queue).toEqual([]);
	});
});

describe('starting a dish again, after the network went mid-cook', () => {
	it('resumes the cooking begun online, from nothing but the answer to starting it', async () => {
		const at = world({
			get_recipe: () => recipe,
			start_attempt: () => attempt({ current_step_index: 2 }),
		});
		await at.call('get_recipe', { branch_id: 'b_1' });
		const online = await at.call('start_attempt', { branch_id: 'b_1' });
		at.offline();
		const reopened = await at.call('start_attempt', { branch_id: 'b_1' });
		expect(reopened.id).toBe(online.id);
		expect(reopened.current_step_index).toBe(2);
		expect(at.kept()?.queue ?? []).toEqual([]);
	});

	it('never resumes a cooking finished online, however old the phone’s copy of it', async () => {
		const at = world({
			get_recipe: () => recipe,
			get_current_attempt: () => ({ attempt: attempt() }),
			finish_attempt: () => attempt({ finished_at: '2026-09-19T11:00:00.000Z', resumable: false }),
		});
		await at.call('get_recipe', { branch_id: 'b_1' });
		await at.call('get_current_attempt', { lineage_id: 'l_1' });
		await at.call('finish_attempt', { attempt_id: 'at_server000000001' });
		at.offline();
		const again = await at.call('start_attempt', { branch_id: 'b_1' });
		expect(again.id).not.toBe('at_server000000001');
	});
});

describe('a cooking finished on another device', () => {
	it('learns from the diary that it finished elsewhere, and does not resume it', async () => {
		const at = world({
			get_recipe: () => recipe,
			start_attempt: () => attempt(),
			list_attempts: () => ({
				attempts: [
					{
						...attempt({ finished_at: '2026-09-19T11:00:00.000Z', resumable: false }),
						recipe: { branch_id: 'b_1', title: 'Katsu Curry' },
					},
				],
			}),
		});
		await at.call('get_recipe', { branch_id: 'b_1' });
		await at.call('start_attempt', { branch_id: 'b_1' });
		// Finished on the iPad; the phone reads its diary.
		await at.call('list_attempts');
		at.offline();
		const again = await at.call('start_attempt', { branch_id: 'b_1' });
		expect(again.id).not.toBe('at_server000000001');
	});
});

describe('a cooking thrown away online', () => {
	it('is never resumed by a phone that has lost the network since', async () => {
		const at = world({
			get_recipe: () => recipe,
			start_attempt: () => attempt(),
			delete_attempt: () => ({ deleted: true }),
		});
		await at.call('get_recipe', { branch_id: 'b_1' });
		const online = await at.call('start_attempt', { branch_id: 'b_1' });
		await at.call('delete_attempt', { attempt_id: online.id });
		at.offline();
		const again = await at.call('start_attempt', { branch_id: 'b_1' });
		expect(again.id).not.toBe(online.id);
	});
});

describe('the diary with no network', () => {
	it('names a cooking begun online by its recipe, from the copy the phone holds', async () => {
		const at = world({
			get_recipe: () => recipe,
			start_attempt: () => attempt(),
			list_attempts: () => ({ attempts: [] }),
			advance_attempt: (input) => attempt({ id: input.attempt_id as string }),
		});
		await at.call('list_attempts');
		await at.call('get_recipe', { branch_id: 'b_1' });
		const started = await at.call('start_attempt', { branch_id: 'b_1' });
		at.offline();
		await at.call('advance_attempt', { attempt_id: started.id, current_step_index: 1 });
		const diary = await at.call('list_attempts');
		// And what the recipe makes, so the diary can say how much was cooked (#109).
		expect(diary.attempts[0].recipe).toEqual({
			branch_id: 'b_1',
			title: 'Katsu Curry',
			written_yield: { amount: '4', noun: 'servings' },
		});
	});
});
