/**
 * "Not right now", said once at the top of the page (#76, option C).
 *
 * What these guard: one card at a time, most pressing first; each put away
 * once and remembered for as long as it should be; the offline card telling a
 * recipe the Kitchen holds from one merely kept; and the first fill waiting
 * for the person, never arriving over mobile data by surprise.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import type { GetRecipeOutput } from '$lib/api/catalogue';
import NoticesTestHarness from './NoticesTestHarness.svelte';
import type { Device } from './device.svelte';
import { sessionBegan } from './library.svelte';
import { reached } from './device.svelte';
import { standing } from './standing.svelte';

const entry = (branch_id: string, main_photo: string | null = null) => ({
	lineage_id: `l_${branch_id}`,
	branch_id,
	title: `Recipe ${branch_id}`,
	language: 'en',
	language_fallback: false,
	main_photo,
	yield: null,
	matched: null,
});

const recipe = (branch_id: string): GetRecipeOutput => ({
	branch_id,
	lineage_id: `l_${branch_id}`,
	kitchen_id: 'k_1',
	hand_id: 'h_1',
	language: 'en',
	origin_address: null,
	head_version_id: `v_${branch_id}`,
	translation: null,
	versions: [
		{
			sequence: 1,
			version_id: `v_${branch_id}`,
			parent_version_id: null,
			hand_id: 'h_1',
			name: null,
			change_note: null,
			created_at: '2026-09-01T00:00:00Z',
			translates_version_id: null,
			language: 'en',
			components: [],
			content: {
				title: `Recipe ${branch_id}`,
				yield: null,
				prep_time_minutes: null,
				cook_time_minutes: null,
				note: null,
				main_photo: `p_${branch_id}`,
				nutrition: null,
				source: null,
				ingredients: [],
				steps: [],
			},
			readings: [],
			measured: { ingredients: [], steps: [] },
			cooking: { steps: [] },
		},
	],
	tags: [],
	related_recipes: [],
	cooked: { count: 0, last_cooked_at: null, ratings: [] },
});

const thread = (branch_id: string, others: string[] = []) => ({
	lineage_id: `l_${branch_id}`,
	branches: [branch_id, ...others].map((id) => ({
		branch_id: id,
		kitchen_id: `k_${id}`,
		hand_id: `h_${id}`,
		language: 'en',
		head_version_id: `v_${id}`,
		translation: null,
	})),
	versions: [],
	attempts: [],
});

const kitchen = (id: string) => ({
	id,
	hand_id: `h_${id}`,
	is_home: true,
	members: [],
	name: 'Maison Batterman',
	nickname: null,
});

/** A Kitchen of two recipes, one with a picture. */
const library = (): Answers => ({
	search_recipes: { query: null, closest: false, recipes: [entry('b_1', 'p_1'), entry('b_2')] },
	get_recipe: () => recipe('b_1'),
	get_thread: () => thread('b_1'),
	// b_2's Thread names b_1 as its one other Branch, so its page would ask.
	divergence: { refuse: 'not_found' },
	list_kitchens: { kitchens: [kitchen('k_b_1'), kitchen('k_b_2')] },
});

const device = (over: Partial<Device> = {}): Device => ({
	secure: true,
	installed: false,
	apple: true,
	wifi: () => undefined,
	...over,
});

const fetched: string[] = [];

function setOnline(value: boolean) {
	Object.defineProperty(navigator, 'onLine', { value, configurable: true });
	dispatchEvent(new Event(value ? 'online' : 'offline'));
}

beforeEach(() => {
	localStorage.clear();
	fetched.length = 0;
	standing.branchId = undefined;
	standing.keptAt = undefined;
	// A page a service worker controls: reading is keeping.
	vi.stubGlobal('isSecureContext', true);
	Object.defineProperty(navigator, 'serviceWorker', {
		value: { controller: {}, addEventListener() {}, removeEventListener() {} },
		configurable: true,
	});
	vi.stubGlobal('fetch', async (url: string) => {
		fetched.push(url);
		return new Response('webp');
	});
	setOnline(true);
});

afterEach(() => {
	cleanup();
	vi.unstubAllGlobals();
	Reflect.deleteProperty(navigator, 'onLine');
	Reflect.deleteProperty(navigator, 'serviceWorker');
});

function show(answers: Answers = library(), on: Device = device()) {
	const kamosu = standIn(answers);
	render(NoticesTestHarness, { props: { client: kamosu.client, device: on } });
	return kamosu;
}

const filled = (held: string[]) =>
	localStorage.setItem('kamosu.library', JSON.stringify({ filledAt: Date.now(), held }));

describe('the first fill', () => {
	it('waits to be asked, saying what it costs', async () => {
		const kamosu = show();
		expect(await screen.findByText("Your library isn't on this phone yet")).toBeInTheDocument();
		expect(screen.getByText(/2 recipes and their pictures, about 116 KB/)).toBeInTheDocument();
		// Nothing was fetched: an iPhone cannot tell wifi from mobile data.
		expect(kamosu.calls.filter((c) => c.operation === 'get_recipe')).toHaveLength(0);
	});

	it('reads every recipe as its page would, with its pictures, when asked', async () => {
		const kamosu = show({
			...library(),
			get_thread: () => thread('b_1', ['b_friend']),
			divergence: { refuse: 'not_found' },
		});
		await fireEvent.click(await screen.findByRole('button', { name: 'Fetch it now' }));

		await waitFor(() =>
			expect(screen.queryByText("Your library isn't on this phone yet")).not.toBeInTheDocument(),
		);
		const asked = kamosu.calls.map((c) => c.operation);
		expect(asked.filter((o) => o === 'get_recipe')).toHaveLength(2);
		expect(asked.filter((o) => o === 'get_thread')).toHaveLength(2);
		// b_1 has exactly one other Branch, so its page would lay it over this
		// one; b_2 then sees two others, and its page would not.
		expect(asked.filter((o) => o === 'divergence')).toHaveLength(1);
		expect(fetched).toContain('/api/photographs/p_1/card');
		expect(fetched).toContain('/api/photographs/p_b_1/page');
		// Remembered, so the next start only tops it up.
		expect(JSON.parse(localStorage.getItem('kamosu.library')!).held).toEqual(['b_1', 'b_2']);
	});

	it('also fills a second Branch of a Lineage when one of the Kitchens holds it', async () => {
		// The shelf shows one card per Lineage, so the friend's Kitchen's Branch
		// is only found through the Thread — and it is fetched only when that
		// Kitchen is one of the Person's.
		const kamosu = show({
			...library(),
			search_recipes: { query: null, closest: false, recipes: [entry('b_1')] },
			get_thread: () => thread('b_1', ['b_other']),
			list_kitchens: { kitchens: [kitchen('k_b_1'), kitchen('k_b_other')] },
		});
		await fireEvent.click(await screen.findByRole('button', { name: 'Fetch it now' }));
		await waitFor(() =>
			expect(JSON.parse(localStorage.getItem('kamosu.library') ?? '{}').filledAt).toBeDefined(),
		);
		const read = kamosu.calls
			.filter((c) => c.operation === 'get_recipe')
			.map((c) => (c.input as { branch_id: string }).branch_id);
		expect(read).toEqual(['b_1', 'b_other']);
	});

	it('leaves a Branch alone when no Kitchen of the Person holds it', async () => {
		const kamosu = show({
			...library(),
			search_recipes: { query: null, closest: false, recipes: [entry('b_1')] },
			get_thread: () => thread('b_1', ['b_stranger']),
			list_kitchens: { kitchens: [kitchen('k_b_1')] },
		});
		await fireEvent.click(await screen.findByRole('button', { name: 'Fetch it now' }));
		await waitFor(() =>
			expect(JSON.parse(localStorage.getItem('kamosu.library') ?? '{}').filledAt).toBeDefined(),
		);
		expect(kamosu.calls.filter((c) => c.operation === 'get_recipe')).toHaveLength(1);
	});

	it("learns which recipes are the Kitchen's before any fill, and keeps that", async () => {
		show();
		await screen.findByText("Your library isn't on this phone yet");
		expect(JSON.parse(localStorage.getItem('kamosu.library')!)).toEqual({ held: ['b_1', 'b_2'] });
	});

	it('starts by itself where the phone says it is on wifi', async () => {
		const kamosu = show(library(), device({ wifi: () => true }));
		await waitFor(() =>
			expect(kamosu.calls.filter((c) => c.operation === 'get_recipe')).toHaveLength(2),
		);
	});

	it('steps aside until next time on "When I\'m on wifi"', async () => {
		show();
		await fireEvent.click(await screen.findByRole('button', { name: "When I'm on wifi" }));
		expect(screen.queryByText("Your library isn't on this phone yet")).not.toBeInTheDocument();
		// The next thing worth saying takes its place.
		expect(screen.getByText('Keep Kamosu on your phone')).toBeInTheDocument();
	});

	it('is forgotten when someone else signs in on this phone', async () => {
		filled(['b_1']);
		sessionBegan();
		expect(localStorage.getItem('kamosu.library')).toBeNull();
	});
});

describe('offline', () => {
	it('says the whole Kitchen is here once the library is', async () => {
		filled(['b_1', 'b_2']);
		setOnline(false);
		show();
		expect(await screen.findByText("You're offline")).toBeInTheDocument();
		expect(screen.getByText(/Every recipe in your Kitchen is on this phone/)).toBeInTheDocument();
	});

	it('is said when the server is out of reach even though the browser thinks it is online', async () => {
		// A wifi with nothing behind it, or the Kamosu at home down while the
		// phone has signal: navigator.onLine says yes, and the server does not answer.
		filled(['b_1']);
		show();
		await screen.findByText('Keep Kamosu on your phone');
		reached(false);
		expect(await screen.findByText("You're offline")).toBeInTheDocument();
		reached(true);
		await waitFor(() => expect(screen.queryByText("You're offline")).not.toBeInTheDocument());
	});

	it('says only what was opened is here before the library has arrived', async () => {
		setOnline(false);
		show();
		expect(await screen.findByText(/only what you've opened is here/)).toBeInTheDocument();
	});

	it('says a recipe the Kitchen does not hold is the copy from the day it was opened', async () => {
		filled(['b_1']);
		standing.branchId = 'b_friend';
		standing.keptAt = new Date(2026, 8, 12);
		setOnline(false);
		show();
		expect(
			await screen.findByText(
				new RegExp(`copy kept when you opened it on ${new Date(2026, 8, 12).toLocaleDateString()}`),
			),
		).toBeInTheDocument();
	});

	it('claims nothing about a recipe before this phone has learned what the Kitchen holds', async () => {
		standing.branchId = 'b_1';
		standing.keptAt = new Date(2026, 8, 12);
		setOnline(false);
		show();
		expect(await screen.findByText(/only what you've opened is here/)).toBeInTheDocument();
		expect(screen.queryByText(/isn't in your Kitchen/)).not.toBeInTheDocument();
	});

	it("does not call a recipe of the Kitchen's a copy, even before the first fill", async () => {
		localStorage.setItem('kamosu.library', JSON.stringify({ held: ['b_1'] }));
		standing.branchId = 'b_1';
		standing.keptAt = new Date(2026, 8, 12);
		setOnline(false);
		show();
		expect(await screen.findByText(/only what you've opened is here/)).toBeInTheDocument();
		expect(screen.queryByText(/isn't in your Kitchen/)).not.toBeInTheDocument();
	});

	it('is put away for this spell offline, and said again the next', async () => {
		filled(['b_1']);
		setOnline(false);
		show();
		await fireEvent.click(await screen.findByRole('button', { name: 'Got it' }));
		expect(screen.queryByText("You're offline")).not.toBeInTheDocument();

		setOnline(true);
		setOnline(false);
		expect(await screen.findByText("You're offline")).toBeInTheDocument();
	});
});

describe('over plain http://', () => {
	it('warns that nothing can be kept, once, and remembers being put away', async () => {
		show(library(), device({ secure: false }));
		expect(await screen.findByText("This connection isn't secure")).toBeInTheDocument();
		expect(screen.getByRole('alert')).toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: 'Got it' }));
		expect(screen.queryByText("This connection isn't secure")).not.toBeInTheDocument();

		cleanup();
		show(library(), device({ secure: false }));
		// Kamosu still works: nothing else stands in the way.
		await Promise.resolve();
		expect(screen.queryByText("This connection isn't secure")).not.toBeInTheDocument();
	});
});

describe('in a browser tab', () => {
	it('explains how to add it to the Home Screen on an iPhone, and never refuses', async () => {
		filled(['b_1']);
		show(library(), device({ apple: true }));
		expect(await screen.findByText('Keep Kamosu on your phone')).toBeInTheDocument();
		expect(screen.getByText(/after 7 days without a visit/)).toBeInTheDocument();
		expect(screen.getByText('Choose Add to Home Screen')).toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: 'Not now' }));
		expect(screen.queryByText('Keep Kamosu on your phone')).not.toBeInTheDocument();
		expect(JSON.parse(localStorage.getItem('kamosu.put-away')!)).toEqual({ install: true });
	});

	it('explains it the way other browsers do it elsewhere', async () => {
		filled(['b_1']);
		show(library(), device({ apple: false }));
		expect(await screen.findByText(/choose Install, or Add to Home screen/)).toBeInTheDocument();
	});

	it('says nothing at all once installed, filled and online', async () => {
		filled(['b_1']);
		show(library(), device({ installed: true }));
		await waitFor(() => expect(screen.queryByRole('status')).not.toBeInTheDocument());
		expect(screen.queryByRole('alert')).not.toBeInTheDocument();
	});
});
