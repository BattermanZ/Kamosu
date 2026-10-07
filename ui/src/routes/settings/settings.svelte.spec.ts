/**
 * The screen-seam test: Settings, against a stand-in Kamosu.
 *
 * What makes this worth having is not that it runs — it is that the answers
 * below are checked against the Catalogue before the screen sees them. Rename
 * `setup_complete` in `src/catalogue.rs` and this test fails, loudly, in the
 * same commit; it cannot quietly keep passing against a shape the Core no
 * longer serves.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { screen, fireEvent, within, waitFor } from '@testing-library/svelte';
import Settings from './+page.svelte';
import { renderScreen } from '../../testing/render';
import type { Answers } from '$lib/api/stand-in';
import { went } from '../../testing/navigation';
import type { GetCookbookOutput, MeaningSearchStatusOutput } from '$lib/api/catalogue';
import { cookbookLabel, kitchenAnswer } from '../../testing/recipes';
import { holdTheInstallOffer } from '$lib/offline/device.svelte';
import { signingIn } from '$lib/shell/signing-in.svelte';
import { offerToInstall } from '../../testing/install';

/**
 * Paraglide's `setLocale` reloads the page, which jsdom cannot do. What a test
 * wants to know is whether the interface was asked to change, and to what.
 */
const { setLocale } = vi.hoisted(() => ({ setLocale: vi.fn() }));
vi.mock('$lib/paraglide/runtime', async (original) => ({
	...(await original<typeof import('$lib/paraglide/runtime')>()),
	setLocale,
}));

/** Every screen visit asks for Sessions and Access Keys beside instance_status
 * (ADR 0031). Tests unconcerned with Access answer both as an anonymous
 * visitor would be refused — the same shape a stranger meets at either Door. */
const anonymous: Answers = {
	list_sessions: { refuse: 'unauthorized' },
	// Whether this Person administers the instance is learnt by asking
	// something only an Operator may ask (#103). A stranger is refused, and the
	// way into the Operator's screen is simply not there.
	list_accounts: { refuse: 'unauthorized' },
	list_access_keys: { refuse: 'unauthorized' },
	get_reading_preferences: { refuse: 'unauthorized' },
	get_person: { refuse: 'unauthorized' },
	meaning_search_status: { refuse: 'unauthorized' },
};

/** Meaning Search's status, with everything the Catalogue requires present. */
const meaningStatus = (
	over: Partial<MeaningSearchStatusOutput> = {},
): MeaningSearchStatusOutput => ({
	state: 'unasked',
	on: false,
	offer: false,
	may_change: false,
	model: 'EmbeddingGemma-300M (4-bit)',
	terms_url: 'https://ai.google.dev/gemma/terms',
	prohibited_use_policy_url: 'https://ai.google.dev/gemma/prohibited_use_policy',
	terms_version: '2026-04-01',
	accepted_by: null,
	accepted_via_access_key: null,
	accepted_at: null,
	declined_at: null,
	model_present: false,
	indexed_at: null,
	recipes_not_yet_indexed: 0,
	...over,
});

/** Your Cookbook as `get_cookbook` answers it: yours alone, unless a test says otherwise. */
function cookbookOf(over: Partial<GetCookbookOutput> = {}): GetCookbookOutput {
	return {
		id: 'c_1',
		name: null,
		authors: [{ person_id: 'p_1', name: 'Stéphane' }],
		recipe_count: 87,
		kitchens: [],
		invites: [],
		joins: [],
		...over,
	};
}

/**
 * A signed-in Person who has never touched the setting: reading in English, in
 * American measures. That is the stated default rather than a guess about
 * anybody (#49, ADR 0016), and every signed-in screen visit asks for it.
 */
const readsInAmerican: Answers = {
	get_reading_preferences: { reading_language: 'en', reading_measures: 'us' },
	// Who is signed in, and what they are called (#113).
	get_person: { person_id: 'p_1', name: 'Stéphane' },
	// A signed-in Person who does not administer the instance (#103).
	list_accounts: { refuse: 'unauthorized' },
	// Meaning Search as most instances have it: nobody has been asked, so this
	// screen shows nothing about it at all. It is not where the feature is
	// discovered — that is inside *nothing found* (ADR 0029) — so its absence
	// here is the ordinary case rather than an omission.
	meaning_search_status: meaningStatus(),
	// No Crouton library brought in yet, so Settings links to no Report (#69).
	list_jobs: { jobs: [] },
	// A Cookbook of one, as every account starts with (#131), with no words
	// filed yet: its Tags card reads them whatever Kitchens there are.
	get_cookbook: cookbookOf(),
	list_tags: { tags: [] },
};

describe('the settings screen', () => {
	it('asks the instance what it is, and says so', async () => {
		const { kamosu } = renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
			...anonymous,
		});

		expect(await screen.findByText(/Version 0\.1\.0/)).toBeInTheDocument();
		expect(screen.getByText(/Setup is complete/)).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).toContain('instance_status');
	});

	// The story (#158), for showing somebody what Kamosu is.
	it('leads to the story of what Kamosu is', async () => {
		renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
			...anonymous,
		});

		expect(await screen.findByRole('link', { name: /What is Kamosu\?/ })).toHaveAttribute(
			'href',
			'/about',
		);
	});

	it("offers the Operator's screen only to somebody who administers the instance", async () => {
		renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
			...anonymous,
		});
		expect(await screen.findByText(/Version 0\.1\.0/)).toBeInTheDocument();
		expect(
			screen.queryByRole('link', { name: 'Administer this instance' }),
		).not.toBeInTheDocument();
	});

	it("shows the way into the Operator's screen to an Operator", async () => {
		renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
			...anonymous,
			list_accounts: {
				accounts: [
					{
						name: 'Stéphane',
						is_operator: true,
						disabled: false,
						is_you: true,
						created_at: '2026-01-04T10:00:00Z',
					},
				],
			},
		});
		expect(await screen.findByRole('link', { name: 'Administer this instance' })).toHaveAttribute(
			'href',
			'/operator',
		);
	});

	it('says setup has not happened when it has not', async () => {
		renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: false, password_minimum: 15 },
			...anonymous,
		});

		expect(await screen.findByText(/Setup has not happened yet/)).toBeInTheDocument();
	});

	it('says so plainly when the instance cannot be reached', async () => {
		renderScreen(Settings, {
			instance_status: { refuse: 'internal', message: 'Kamosu could not be reached.' },
			...anonymous,
		});

		expect(await screen.findByText(/could not be reached/)).toBeInTheDocument();
	});

	it('offers every language Paraglide compiled, with the current one pressed', () => {
		renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
			...anonymous,
		});

		for (const name of ['English', 'Français', 'Español']) {
			expect(screen.getByRole('button', { name })).toBeInTheDocument();
		}
		expect(screen.getByRole('button', { name: 'English' })).toHaveAttribute('aria-pressed', 'true');
	});

	it('shows nothing about Access when the visitor is not signed in', async () => {
		renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
			...anonymous,
		});

		await screen.findByText(/Version 0\.1\.0/);
		expect(screen.queryByText('Access')).not.toBeInTheDocument();
	});

	it("lists a signed-in Person's Sessions and Access Keys together, each ending on its own", async () => {
		const { kamosu } = renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
			list_accounts: { refuse: 'unauthorized' },
			list_sessions: {
				sessions: [
					{
						id: 's_1',
						name: 'iPhone',
						created_at: '2026-01-01T00:00:00Z',
						last_used_at: null,
						revoked: false,
						current: false,
					},
				],
			},
			...readsInAmerican,
			list_access_keys: {
				access_keys: [
					{
						id: 'ak_1',
						name: 'my agent',
						read_only: true,
						created_at: '2026-01-01T00:00:00Z',
						last_used_at: null,
						revoked: false,
					},
				],
			},
			revoke_session: { revoked: true },
			revoke_access_key: { revoked: true },
			list_kitchens: { kitchens: [] },
		});

		expect(await screen.findByText('iPhone')).toBeInTheDocument();
		expect(screen.getByText('my agent')).toBeInTheDocument();
		expect(screen.getByText('Read-only')).toBeInTheDocument();

		const sessionRow = screen.getByText('iPhone').closest('li');
		await fireEvent.click(within(sessionRow as HTMLElement).getByRole('button', { name: 'End' }));
		expect(kamosu.calls.map((call) => call.operation)).toContain('revoke_session');

		const keyRow = screen.getByText('my agent').closest('li');
		await fireEvent.click(within(keyRow as HTMLElement).getByRole('button', { name: 'End' }));
		expect(kamosu.calls.map((call) => call.operation)).toContain('revoke_access_key');
	});

	it('offers Reading Measures on the account, defaulting to American', async () => {
		const { kamosu } = renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
			list_sessions: { sessions: [] },
			list_access_keys: { access_keys: [] },
			...readsInAmerican,
			list_kitchens: { kitchens: [] },
			set_reading_preferences: { reading_language: 'en', reading_measures: 'metric' },
		});

		// American is the default, stated plainly rather than guessed at from
		// anything about the person (ADR 0016).
		const american = await screen.findByRole('button', { name: 'American' });
		expect(american).toHaveAttribute('aria-pressed', 'true');
		expect(screen.getByRole('button', { name: 'Metric' })).toHaveAttribute('aria-pressed', 'false');

		await fireEvent.click(screen.getByRole('button', { name: 'Metric' }));

		// It goes to the ACCOUNT, not to this browser — so the same cook reads
		// the same recipe the same way on the iPad on the worktop. The Reading
		// Language rides along unchanged.
		const sent = kamosu.calls.find((call) => call.operation === 'set_reading_preferences');
		expect(sent?.input).toEqual({ reading_language: 'en', reading_measures: 'metric' });
		expect(screen.getByRole('button', { name: 'Metric' })).toHaveAttribute('aria-pressed', 'true');
	});

	it("shows a minted Access Key's secret where the form was, once, then never again (#142)", async () => {
		const { kamosu } = renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
			list_sessions: { sessions: [] },
			list_access_keys: { access_keys: [] },
			...readsInAmerican,
			mint_access_key: {
				id: 'ak_new',
				name: 'a new agent',
				read_only: false,
				secret: 'the-one-time-secret',
			},
			list_kitchens: { kitchens: [] },
		});

		await screen.findByText('No Access Keys yet.');
		await fireEvent.input(screen.getByLabelText('Name'), { target: { value: 'a new agent' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Create Access Key' }));

		const secret = await screen.findByText('the-one-time-secret');
		expect(kamosu.calls.map((call) => call.operation)).toContain('mint_access_key');

		// In the form's place, under the list, not at the top of the section:
		// the key is where the eye already is when Create is pressed.
		const heading = screen.getByRole('heading', { name: 'Access Keys' });
		expect(heading.compareDocumentPosition(secret) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
		expect(screen.queryByLabelText('Name')).not.toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Create Access Key' })).not.toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Copy' })).toHaveFocus();

		await fireEvent.click(screen.getByRole('button', { name: 'Done, I copied it' }));
		expect(screen.queryByText('the-one-time-secret')).not.toBeInTheDocument();
		expect(screen.getByLabelText('Name')).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Create Access Key' })).toBeInTheDocument();
	});

	it('copies a minted Access Key, and still shows it when the browser will not copy (#142)', async () => {
		renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
			list_sessions: { sessions: [] },
			list_access_keys: { access_keys: [] },
			...readsInAmerican,
			mint_access_key: {
				id: 'ak_new',
				name: 'a new agent',
				read_only: false,
				secret: 'the-one-time-secret',
			},
			list_kitchens: { kitchens: [] },
		});
		const writeText = vi.fn(async () => {});
		Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });

		await screen.findByText('No Access Keys yet.');
		await fireEvent.input(screen.getByLabelText('Name'), { target: { value: 'a new agent' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Create Access Key' }));
		await fireEvent.click(await screen.findByRole('button', { name: 'Copy' }));

		expect(writeText).toHaveBeenCalledWith('the-one-time-secret');
		expect(await screen.findByRole('button', { name: 'Copied' })).toBeInTheDocument();

		writeText.mockRejectedValueOnce(new Error('not allowed'));
		await fireEvent.click(screen.getByRole('button', { name: 'Copied' }));
		expect(await screen.findByRole('button', { name: 'Copy' })).toBeInTheDocument();
		expect(screen.getByText('the-one-time-secret')).toBeInTheDocument();
		Reflect.deleteProperty(navigator, 'clipboard');
	});

	it("lists a signed-in Person's Kitchens, whose recipes each shows, and lets a member be removed", async () => {
		const { kamosu } = renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
			list_sessions: { sessions: [] },
			list_access_keys: { access_keys: [] },
			...readsInAmerican,
			list_tags: { tags: [] },
			list_kitchens: {
				kitchens: [
					{
						id: 'k_home',
						name: 'Maison Batterman',
						cookbooks: [cookbookLabel()],
						nickname: null,
						members: [{ person_id: 'p_1', name: 'Stéphane' }],
					},
					{
						id: 'k_shared',
						name: 'Supper Club',
						cookbooks: [cookbookLabel(), cookbookLabel('c_2', ['Marie'])],
						nickname: 'Nos amis',
						members: [
							{ person_id: 'p_1', name: 'Stéphane' },
							{ person_id: 'p_2', name: 'Marie' },
						],
					},
				],
			},
			remove_kitchen_member: { removed: true },
		});

		expect(await screen.findByDisplayValue('Supper Club')).toBeInTheDocument();
		expect(screen.getByDisplayValue('Maison Batterman')).toBeInTheDocument();
		// No Home Kitchen any more: a recipe lives in a Cookbook (#131).
		expect(screen.queryByText('Home')).not.toBeInTheDocument();
		expect(screen.getByDisplayValue('Nos amis')).toBeInTheDocument();
		expect(
			screen.getByText('Recipes here: Stéphane’s Cookbook and Marie’s Cookbook'),
		).toBeInTheDocument();

		const marieRow = screen.getByText('Marie').closest('li');
		await fireEvent.click(within(marieRow as HTMLElement).getByRole('button', { name: 'Remove' }));
		expect(kamosu.calls).toContainEqual({
			operation: 'remove_kitchen_member',
			input: { kitchen_id: 'k_shared', person_id: 'p_2' },
		});
	});

	// #131, screen choice 4: leaving is its own act at the foot of the card,
	// asked first with what each side keeps. Your own row carries no button.
	describe('leaving a Kitchen', () => {
		const kitchen = (id: string, members: [string, string][]) =>
			kitchenAnswer(id, {
				name: id,
				members: members.map(([person_id, name]) => ({ person_id, name })),
			});
		const signedIn = (kitchens: ReturnType<typeof kitchen>[], over: Answers = {}): Answers => ({
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
			list_sessions: { sessions: [] },
			list_access_keys: { access_keys: [] },
			...readsInAmerican,
			list_tags: { tags: [] },
			list_kitchens: { kitchens },
			remove_kitchen_member: { removed: true },
			preview_leaving_kitchen: { they_keep: 4, you_keep: 6 },
			...over,
		});
		const rowOf = (name: string) => screen.getByText(name).closest('li') as HTMLElement;
		const TWO = [
			kitchen('Family', [
				['p_1', 'Stéphane'],
				['p_2', 'Hélène'],
				['p_3', 'Luc'],
			]),
			kitchen('Supper Club', [
				['p_1', 'Stéphane'],
				['p_4', 'Marie'],
			]),
		];
		const cardOf = async (name: string) =>
			(await screen.findByDisplayValue(name)).closest('li') as HTMLElement;

		it("offers Remove on everyone else's row and nothing on your own", async () => {
			renderScreen(Settings, signedIn(TWO));
			const family = await cardOf('Family');
			const mine = within(within(family).getByText('Stéphane').closest('li') as HTMLElement);
			expect(mine.queryByRole('button')).not.toBeInTheDocument();
			expect(within(rowOf('Hélène')).getByRole('button', { name: 'Remove' })).toBeInTheDocument();
			expect(
				within(family).getByRole('button', { name: 'Leave this Kitchen' }),
			).toBeInTheDocument();
		});

		it('asks first, saying what each side keeps, as the Core counted it', async () => {
			const { kamosu } = renderScreen(Settings, signedIn(TWO));
			await fireEvent.click(
				within(await cardOf('Family')).getByRole('button', { name: 'Leave this Kitchen' }),
			);

			const sheet = within(await screen.findByRole('dialog'));
			expect(sheet.getByRole('heading', { name: 'Leave Family?' })).toBeInTheDocument();
			expect(
				sheet.getByText('Family stops seeing your recipes, and you stop seeing theirs.'),
			).toBeInTheDocument();
			expect(sheet.getByText('Hélène and Luc keep')).toBeInTheDocument();
			expect(
				sheet.getByText('their own copy of the 4 recipes of yours they cooked'),
			).toBeInTheDocument();
			expect(sheet.getByText('your own copy of the 6 of theirs you cooked')).toBeInTheDocument();
			expect(kamosu.calls).toContainEqual({
				operation: 'preview_leaving_kitchen',
				input: { kitchen_id: 'Family' },
			});
			// Asking is not leaving.
			expect(kamosu.calls.map((call) => call.operation)).not.toContain('remove_kitchen_member');

			await fireEvent.click(sheet.getByRole('button', { name: 'Leave Family' }));
			await waitFor(() =>
				expect(kamosu.calls).toContainEqual({
					operation: 'remove_kitchen_member',
					input: { kitchen_id: 'Family', person_id: 'p_1' },
				}),
			);
			await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
		});

		it('says one person keeps, not one person keep', async () => {
			renderScreen(Settings, signedIn(TWO));
			await fireEvent.click(
				within(await cardOf('Supper Club')).getByRole('button', { name: 'Leave this Kitchen' }),
			);
			expect(
				within(await screen.findByRole('dialog')).getByText('Marie keeps'),
			).toBeInTheDocument();
		});

		it('says one recipe and nothing in words, never "1 recipes" or "0 of theirs"', async () => {
			renderScreen(
				Settings,
				signedIn(TWO, { preview_leaving_kitchen: { they_keep: 1, you_keep: 0 } }),
			);
			await fireEvent.click(
				within(await cardOf('Family')).getByRole('button', { name: 'Leave this Kitchen' }),
			);
			const sheet = within(await screen.findByRole('dialog'));
			expect(
				sheet.getByText('their own copy of the one recipe of yours they cooked'),
			).toBeInTheDocument();
			expect(sheet.getByText('nothing: you cooked none of theirs')).toBeInTheDocument();
		});

		it('stays without leaving anything', async () => {
			const { kamosu } = renderScreen(Settings, signedIn(TWO));
			await fireEvent.click(
				within(await cardOf('Family')).getByRole('button', { name: 'Leave this Kitchen' }),
			);
			await fireEvent.click(
				within(await screen.findByRole('dialog')).getByRole('button', { name: 'Stay' }),
			);
			expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
			expect(kamosu.calls.map((call) => call.operation)).not.toContain('remove_kitchen_member');
		});
	});

	it('creates a Kitchen from the form', async () => {
		const { kamosu } = renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
			list_sessions: { sessions: [] },
			list_access_keys: { access_keys: [] },
			...readsInAmerican,
			list_kitchens: { kitchens: [] },
			create_kitchen: {
				id: 'k_new',
				name: 'Supper Club',
				cookbooks: [cookbookLabel()],
				nickname: null,
				members: [{ person_id: 'p_1', name: 'Stéphane' }],
			},
		});

		await screen.findByText('Create a Kitchen');
		await fireEvent.input(screen.getByLabelText('Kitchen name'), {
			target: { value: 'Supper Club' },
		});
		await fireEvent.click(screen.getByRole('button', { name: 'Create Kitchen' }));

		expect(kamosu.calls).toContainEqual({
			operation: 'create_kitchen',
			input: { name: 'Supper Club' },
		});
	});

	it('mints a Kitchen Invite and shows its secret once', async () => {
		const { kamosu } = renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
			list_sessions: { sessions: [] },
			list_access_keys: { access_keys: [] },
			...readsInAmerican,
			list_tags: { tags: [] },
			list_kitchens: {
				kitchens: [
					{
						id: 'k_home',
						name: 'Maison Batterman',
						cookbooks: [cookbookLabel()],
						nickname: null,
						members: [{ person_id: 'p_1', name: 'Stéphane' }],
					},
				],
			},
			invite_to_kitchen: { invite_id: 'ki_1', secret: 'the-invite-secret' },
		});

		await screen.findByDisplayValue('Maison Batterman');
		await fireEvent.click(screen.getByRole('button', { name: 'Invite someone' }));

		expect(await screen.findByText('the-invite-secret')).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).toContain('invite_to_kitchen');

		const writeText = vi.fn(async () => {});
		Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
		await fireEvent.click(screen.getByRole('button', { name: 'Copy' }));
		expect(writeText).toHaveBeenCalledWith('the-invite-secret');
		expect(await screen.findByRole('button', { name: 'Copied' })).toBeInTheDocument();
		Reflect.deleteProperty(navigator, 'clipboard');

		await fireEvent.click(screen.getByRole('button', { name: 'Done, I copied it' }));
		expect(screen.queryByText('the-invite-secret')).not.toBeInTheDocument();
	});

	it('joins a Kitchen through a pasted Invite', async () => {
		const { kamosu } = renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
			list_sessions: { sessions: [] },
			list_access_keys: { access_keys: [] },
			...readsInAmerican,
			list_kitchens: { kitchens: [] },
			accept_kitchen_invite: {
				id: 'k_shared',
				name: 'Supper Club',
				cookbooks: [cookbookLabel()],
				nickname: null,
				members: [{ person_id: 'p_1', name: 'Stéphane' }],
			},
		});

		await screen.findByText('Join a Kitchen');
		await fireEvent.input(screen.getByLabelText('Invite'), {
			target: { value: 'someones-invite-secret' },
		});
		await fireEvent.click(screen.getByRole('button', { name: 'Join' }));

		expect(kamosu.calls).toContainEqual({
			operation: 'accept_kitchen_invite',
			input: { secret: 'someones-invite-secret' },
		});
	});

	it('carries the expanding pair, and names no Kitchen in its title, on six Kitchens (#102)', async () => {
		// The Kitchen half of #102's test criterion, asked where a Kitchen count
		// is real. The header itself cannot answer it — it takes no client, which
		// is the design (ADR 0027): a switcher is forbidden, and filtering by
		// Kitchen already lives on the Recipes shelf (#62). So what is provable
		// is the other end: with a Person in six Kitchens, the screen the card
		// opens onto still says *Settings* and still carries the one name that
		// makes the card grow into it.
		renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
			list_sessions: { sessions: [] },
			list_access_keys: { access_keys: [] },
			...readsInAmerican,
			list_tags: { tags: [] },
			list_kitchens: {
				// The dev instance's own six.
				kitchens: [
					'Maison Batterman',
					'Chez Marc',
					'Chez Élodie',
					'Le Chalet',
					'Chez Papi',
					'Chez Marie',
				].map((name, index) => ({
					id: `k_${index}`,
					name,
					cookbooks: [cookbookLabel()],
					nickname: null,
					members: [{ person_id: 'p_1', name: 'Stéphane' }],
				})),
			},
		});

		// All six arrived, so the screen really is in the many-Kitchens state.
		expect(await screen.findByDisplayValue('Le Chalet')).toBeInTheDocument();

		const title = screen.getByRole('heading', { level: 1, name: 'Settings' });
		expect(title.getAttribute('style')).toContain('view-transition-name: settings');
		expect(title.getAttribute('style')).toContain('view-transition-class: expanding');

		// Not "Settings — Chez Marc", and not a Kitchen's name in its place.
		expect(title).toHaveTextContent(/^Settings$/);
	});

	it('keeps the "not right now" facts once their cards are put away (#76)', async () => {
		renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
			...anonymous,
		});
		// The test browser is not a secure page, so this phone can keep nothing.
		expect(await screen.findByText('This phone')).toBeInTheDocument();
		expect(screen.getByText(/Kamosu is on http:\/\//)).toBeInTheDocument();
	});

	describe('on a phone a browser may offer to install Kamosu on (#173)', () => {
		let stop: () => void;
		beforeEach(() => {
			vi.stubGlobal('isSecureContext', true);
			stop = holdTheInstallOffer();
		});
		afterEach(() => {
			stop();
			vi.unstubAllGlobals();
			localStorage.removeItem('kamosu.put-away');
		});
		const showSettings = () =>
			renderScreen(Settings, {
				instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
				...anonymous,
			});

		it("offers the browser's install dialog while one is held, and puts the card away once accepted", async () => {
			const { prompt } = offerToInstall('accepted');
			showSettings();
			await fireEvent.click(await screen.findByRole('button', { name: 'Install Kamosu' }));
			expect(prompt).toHaveBeenCalledOnce();
			// This tab is still not the app: the reason stays, with nothing left to do.
			await waitFor(() =>
				expect(screen.queryByRole('button', { name: 'Install Kamosu' })).not.toBeInTheDocument(),
			);
			expect(screen.queryByText('Open the browser menu')).not.toBeInTheDocument();
			expect(screen.getByText(/Kamosu opens like an app/)).toBeInTheDocument();
			// Accepted here, the Home card stays away too, as it does from the card.
			expect(JSON.parse(localStorage.getItem('kamosu.put-away')!)).toEqual({ install: true });
		});

		it('falls back to the written steps when the dialog is turned down', async () => {
			offerToInstall('dismissed');
			showSettings();
			await fireEvent.click(await screen.findByRole('button', { name: 'Install Kamosu' }));
			expect(await screen.findByText('Open the browser menu')).toBeInTheDocument();
			expect(screen.queryByRole('button', { name: 'Install Kamosu' })).not.toBeInTheDocument();
		});

		it('writes the steps out while no install is on offer', async () => {
			showSettings();
			expect(await screen.findByText('Open the browser menu')).toBeInTheDocument();
			expect(screen.queryByRole('button', { name: 'Install Kamosu' })).not.toBeInTheDocument();
		});
	});

	// --- Tags (#104) --------------------------------------------------------
	//
	// Renaming, merging and deleting are HERE and not on the recipe page, which
	// is the choice of 22 September 2026. These guard the two things that
	// make the section worth having: the count beside each word, so deleting is
	// an informed act, and that a tag known only in one Language can be given a
	// name in another.

	/** One signed-in Person with one Kitchen and a Cookbook of their own, the ordinary case. */
	const withKitchen: Answers = {
		instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
		list_sessions: { sessions: [] },
		list_access_keys: { access_keys: [] },
		...readsInAmerican,
		list_kitchens: {
			kitchens: [
				{
					id: 'k_home',
					name: 'Maison Batterman',
					cookbooks: [cookbookLabel()],
					nickname: null,
					members: [{ person_id: 'p_1', name: 'Stéphane' }],
				},
			],
		},
	};

	const settingsTag = (id: string, name: string, recipes: number, language = 'en') => ({
		id,
		cookbook_id: 'c_1',
		name,
		language,
		names: [{ language, name }],
		recipes,
		// The Core decides this against the Reading Language (#104); this
		// Person reads in English, which `readsInAmerican` above says.
		language_fallback: language !== 'en',
	});

	it('says a Cookbook has no tags yet rather than showing an empty box', async () => {
		renderScreen(Settings, { ...withKitchen, list_tags: { tags: [] } });

		expect(await screen.findByRole('heading', { name: 'Tags' })).toBeInTheDocument();
		expect(
			await screen.findByText('No tags yet. Add one to a recipe and it appears here.'),
		).toBeInTheDocument();
		// And it says what renaming and deleting a tag reach.
		expect(screen.getByText(/A deleted one comes off them all/)).toBeInTheDocument();
	});

	it('renames a tag through the Operation, saying it reaches every recipe at once', async () => {
		const { kamosu } = renderScreen(Settings, {
			...withKitchen,
			list_tags: { tags: [settingsTag('t_dessert', 'dessert', 12)] },
			rename_tag: settingsTag('t_dessert', 'desserts', 9),
		});

		// The count is on the row: 12 recipes carry this word, and that is the
		// fact a person weighs before touching it.
		expect(await screen.findByDisplayValue('dessert')).toBeInTheDocument();
		expect(screen.getByText('12 recipes')).toBeInTheDocument();
		// The promise a rename makes, said once at the top of the section.
		expect(screen.getByText(/shows its new name on every recipe at once/)).toBeInTheDocument();

		const field = screen.getByDisplayValue('dessert');
		await fireEvent.input(field, { target: { value: 'desserts' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Rename' }));

		expect(kamosu.calls).toContainEqual({
			operation: 'rename_tag',
			input: { tag_id: 't_dessert', language: 'en', name: 'desserts' },
		});

		// And it says what it reached. "Renaming reaches every recipe carrying
		// it, immediately" is the whole reason a Tag is kept once per Kitchen,
		// and a promise nothing on screen stands behind is not the criterion
		// being met.
		//
		// The number is the one `rename_tag` ANSWERED with — 9 here, against a
		// row that was showing 12 — because a row read a minute ago is the
		// stale one, and this sentence is the wrong place to be approximately
		// right.
		expect(await screen.findByText('Renamed on 9 recipes, all at once.')).toBeInTheDocument();
	});

	it('names a tag in the Language the Person reads recipes in, not the interface locale', async () => {
		// Two separate settings (ADR 0006, ADR 0016). A French interface over an
		// English Reading Language must not name a new word `fr`.
		const { kamosu } = renderScreen(Settings, {
			...withKitchen,
			get_reading_preferences: { reading_language: 'es', reading_measures: 'metric' },
			list_tags: { tags: [settingsTag('t_mijote', 'mijoté', 7, 'fr')] },
			rename_tag: settingsTag('t_mijote', 'guisado', 7, 'es'),
		});

		const adding = await screen.findByPlaceholderText('Name it in Spanish');
		await fireEvent.input(adding, { target: { value: 'guisado' } });
		await fireEvent.click(within(adding.closest('form') as HTMLElement).getByRole('button'));

		expect(kamosu.calls).toContainEqual({
			operation: 'rename_tag',
			input: { tag_id: 't_mijote', language: 'es', name: 'guisado' },
		});
	});

	it('gives a tag known only in French an English name, without renaming the French one', async () => {
		// The fallback ADR 0006 built, finally fixable: nothing until now let
		// anybody add the name it fell back from.
		const { kamosu } = renderScreen(Settings, {
			...withKitchen,
			list_tags: { tags: [settingsTag('t_mijote', 'mijoté', 7, 'fr')] },
			rename_tag: settingsTag('t_mijote', 'slow-cooked', 7),
		});

		expect(await screen.findByDisplayValue('mijoté')).toBeInTheDocument();
		expect(screen.getByText('fr')).toBeInTheDocument();

		const adding = screen.getByPlaceholderText('Name it in English');
		await fireEvent.input(adding, { target: { value: 'slow-cooked' } });
		await fireEvent.click(within(adding.closest('form') as HTMLElement).getByRole('button'));

		// Named in the reader's Language, leaving the French name where it is: a
		// tag holds a name per Language rather than one tag per Language.
		expect(kamosu.calls).toContainEqual({
			operation: 'rename_tag',
			input: { tag_id: 't_mijote', language: 'en', name: 'slow-cooked' },
		});
	});

	it('asks before deleting a tag, leading with how many recipes lose it', async () => {
		const { kamosu } = renderScreen(Settings, {
			...withKitchen,
			list_tags: { tags: [settingsTag('t_try', 'to try', 18)] },
			delete_tag: { deleted: true },
		});

		await screen.findByDisplayValue('to try');
		await fireEvent.click(screen.getByRole('button', { name: 'Delete' }));

		// The figure leads, because "18 recipes lose this tag" is something a
		// person can weigh and "are you sure?" is not (#103).
		expect(await screen.findByText('Delete the tag to try?')).toBeInTheDocument();
		expect(screen.getByText('18')).toBeInTheDocument();
		expect(screen.getByText('recipes lose this tag')).toBeInTheDocument();
		// And it says outright that the recipes themselves are untouched.
		expect(screen.getByText(/The recipes are untouched/)).toBeInTheDocument();

		// Not yet done: the sheet is the act, not the button behind it.
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('delete_tag');

		const sheet = screen.getByRole('dialog');
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Delete' }));
		expect(kamosu.calls).toContainEqual({
			operation: 'delete_tag',
			input: { tag_id: 't_try' },
		});
	});

	it('says “1 recipe loses this tag”, never “1 recipes”', async () => {
		// The figure and the phrase sit side by side in the sheet, so the phrase
		// has to inflect with it.
		renderScreen(Settings, {
			...withKitchen,
			list_tags: { tags: [settingsTag('t_one', 'brunch', 1)] },
			delete_tag: { deleted: true },
		});

		await screen.findByDisplayValue('brunch');
		await fireEvent.click(screen.getByRole('button', { name: 'Delete' }));

		expect(await screen.findByText('recipe loses this tag')).toBeInTheDocument();
		expect(screen.queryByText('recipes lose this tag')).not.toBeInTheDocument();
	});

	it('asks before merging two tags, saying which way round it goes', async () => {
		const { kamosu } = renderScreen(Settings, {
			...withKitchen,
			list_tags: {
				tags: [settingsTag('t_quick', 'quick', 9), settingsTag('t_weeknight', 'weeknight', 21)],
			},
			merge_tags: settingsTag('t_weeknight', 'weeknight', 30),
		});

		await screen.findByDisplayValue('quick');
		const row = screen.getByDisplayValue('quick').closest('li') as HTMLElement;
		await fireEvent.change(within(row).getByRole('combobox'), { target: { value: 't_weeknight' } });

		// Which way round matters and is easy to get backwards, so it is spelt
		// out in both directions before anything moves.
		expect(await screen.findByText('Merge quick into weeknight?')).toBeInTheDocument();
		expect(screen.getByText('recipes move over')).toBeInTheDocument();

		// The button that acts says what it does, without the picker's ellipsis.
		const sheet = screen.getByRole('dialog');
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Merge' }));
		expect(kamosu.calls).toContainEqual({
			operation: 'merge_tags',
			input: { keep_tag_id: 't_weeknight', merge_tag_id: 't_quick' },
		});
	});

	/**
	 * The second of the two doors onto a Food (#107). The first is the Reading
	 * corrector on the recipe; this is the one for a Food you have to go looking
	 * for, and nothing else in the interface reaches it.
	 */
	it('offers the way into the Foods, to every Person and not only an Operator', async () => {
		renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
			list_sessions: { sessions: [] },
			list_access_keys: { access_keys: [] },
			list_kitchens: { kitchens: [] },
			...readsInAmerican,
		});

		const door = await screen.findByRole('link', { name: 'Look up a Food' });
		expect(door).toHaveAttribute('href', '/foods');
		// A Food is instance-wide and all five of its Operations are
		// Permission::Person, so this door is not the Operator's (#103).
		expect(screen.queryByRole('link', { name: 'Administer this Kamosu' })).not.toBeInTheDocument();
	});
});

describe('the Reading Language (#112)', () => {
	/**
	 * The choice of 23 September 2026 (option C): one Language control
	 * that recipes follow, which can split in two for somebody who reads
	 * Kamosu in one Language and their recipes in another.
	 */
	const signedIn = (reading_language: 'en' | 'fr' | 'es'): Answers => ({
		instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
		list_sessions: { sessions: [] },
		list_access_keys: { access_keys: [] },
		list_kitchens: { kitchens: [] },
		...readsInAmerican,
		get_reading_preferences: { reading_language, reading_measures: 'us' },
	});

	const words = () => screen.getByRole('list', { name: 'Language' });
	const recipes = () => screen.getByRole('list', { name: 'Recipes in' });

	it('says recipes follow the one control, and moves both when it is changed', async () => {
		setLocale.mockClear();
		const { kamosu } = renderScreen(Settings, {
			...signedIn('en'),
			set_reading_preferences: { reading_language: 'fr', reading_measures: 'us' },
		});

		expect(
			await screen.findByText(
				"Kamosu's words, and recipe titles where there's one in this language.",
			),
		).toBeInTheDocument();
		expect(screen.getByText(/Recipes follow it\./)).toBeInTheDocument();
		expect(screen.queryByRole('list', { name: 'Recipes in' })).not.toBeInTheDocument();

		// The account first, then the interface: `setLocale` reloads the page,
		// and a save still in flight when it does is a save that may not land.
		let savedFirst = false;
		setLocale.mockImplementationOnce(() => {
			savedFirst = kamosu.calls.some((call) => call.operation === 'set_reading_preferences');
		});

		await fireEvent.click(within(words()).getByRole('button', { name: 'Français' }));

		await vi.waitFor(() => expect(setLocale).toHaveBeenCalledWith('fr'));
		expect(savedFirst).toBe(true);
		expect(kamosu.calls).toContainEqual({
			operation: 'set_reading_preferences',
			input: { reading_language: 'fr', reading_measures: 'us' },
		});
		// The reload would take the line saying what moved with it, so it is
		// left for the next load of this screen.
		expect(sessionStorage.getItem('kamosu.reading-language-moved')).toBe('fr');
	});

	it('says what moved once the reload that carried both is over', async () => {
		sessionStorage.setItem('kamosu.reading-language-moved', 'fr');
		renderScreen(Settings, signedIn('en'));

		expect(
			await screen.findByText("Titles now show in French where there's one. Others are marked."),
		).toBeInTheDocument();
		// Said once: the next visit is quiet.
		expect(sessionStorage.getItem('kamosu.reading-language-moved')).toBeNull();
	});

	it('splits the recipes off on request, and moves only them', async () => {
		setLocale.mockClear();
		const { kamosu } = renderScreen(Settings, {
			...signedIn('en'),
			set_reading_preferences: { reading_language: 'es', reading_measures: 'us' },
		});

		await fireEvent.click(
			await screen.findByRole('button', { name: 'Read recipes in another language' }),
		);

		// Split, the first control is only the interface's words, and says so.
		expect(
			screen.getByText("Kamosu's buttons and headings. This browser only."),
		).toBeInTheDocument();
		expect(within(recipes()).getByRole('button', { name: 'English' })).toHaveAttribute(
			'aria-pressed',
			'true',
		);

		await fireEvent.click(within(recipes()).getByRole('button', { name: 'Español' }));

		expect(kamosu.calls).toContainEqual({
			operation: 'set_reading_preferences',
			input: { reading_language: 'es', reading_measures: 'us' },
		});
		// It says what just changed on the shelf, in the reader's words.
		expect(
			await screen.findByText("Titles now show in Spanish where there's one. Others are marked."),
		).toBeInTheDocument();
		expect(within(recipes()).getByRole('button', { name: 'Español' })).toHaveAttribute(
			'aria-pressed',
			'true',
		);
		// Kamosu's own words did not move.
		expect(setLocale).not.toHaveBeenCalled();
	});

	it('opens split when the account reads in a Language this browser does not speak', async () => {
		// A new phone, or a split made on another device. The screen says what
		// is true rather than pretending the two are one.
		setLocale.mockClear();
		const { kamosu } = renderScreen(Settings, signedIn('fr'));

		await vi.waitFor(() =>
			expect(within(recipes()).getByRole('button', { name: 'Français' })).toHaveAttribute(
				'aria-pressed',
				'true',
			),
		);
		expect(within(words()).getByRole('button', { name: 'English' })).toHaveAttribute(
			'aria-pressed',
			'true',
		);

		// Changing the words while split leaves the account alone.
		await fireEvent.click(within(words()).getByRole('button', { name: 'Español' }));
		expect(setLocale).toHaveBeenCalledWith('es');
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('set_reading_preferences');
	});

	it('joins them again, reading recipes in the interface Language', async () => {
		const { kamosu } = renderScreen(Settings, {
			...signedIn('fr'),
			set_reading_preferences: { reading_language: 'en', reading_measures: 'us' },
		});

		await fireEvent.click(
			await screen.findByRole('button', { name: 'Recipes in the same language as Kamosu' }),
		);

		expect(kamosu.calls).toContainEqual({
			operation: 'set_reading_preferences',
			input: { reading_language: 'en', reading_measures: 'us' },
		});
		expect(await screen.findByText(/Recipes follow it\./)).toBeInTheDocument();
		expect(screen.queryByRole('list', { name: 'Recipes in' })).not.toBeInTheDocument();
		expect(
			screen.getByText("Titles now show in English where there's one. Others are marked."),
		).toBeInTheDocument();
	});

	it('reads the Tags again in the new Language, so they are named in the one renames write', async () => {
		const { kamosu } = renderScreen(Settings, {
			...signedIn('en'),
			list_kitchens: {
				kitchens: [
					{
						id: 'k_home',
						name: 'Maison Batterman',
						cookbooks: [cookbookLabel()],
						nickname: null,
						members: [{ person_id: 'p_1', name: 'Stéphane' }],
					},
				],
			},
			list_tags: { tags: [] },
			set_reading_preferences: { reading_language: 'fr', reading_measures: 'us' },
		});

		const tagReads = () => kamosu.calls.filter((call) => call.operation === 'list_tags').length;
		await vi.waitFor(() => expect(tagReads()).toBeGreaterThan(0));
		const before = tagReads();

		await fireEvent.click(
			await screen.findByRole('button', { name: 'Read recipes in another language' }),
		);
		await fireEvent.click(within(recipes()).getByRole('button', { name: 'Français' }));

		await vi.waitFor(() => expect(tagReads()).toBeGreaterThan(before));
	});

	it('puts the Reading Language back when the account refuses it', async () => {
		renderScreen(Settings, {
			...signedIn('fr'),
			set_reading_preferences: { refuse: 'busy' },
		});

		const spanish = await vi.waitFor(() =>
			within(recipes()).getByRole('button', { name: 'Español' }),
		);
		await fireEvent.click(spanish);

		await vi.waitFor(() =>
			expect(within(recipes()).getByRole('button', { name: 'Français' })).toHaveAttribute(
				'aria-pressed',
				'true',
			),
		);
		expect(screen.queryByText(/Your shelf now shows/)).not.toBeInTheDocument();
	});

	it('tells a stranger only what the words control does, since there is no account to follow', async () => {
		renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
			...anonymous,
		});

		await screen.findByText(/Version 0\.1\.0/);
		expect(
			screen.getByText("Kamosu's buttons and headings. This browser only."),
		).toBeInTheDocument();
		expect(screen.queryByText(/Recipes follow it/)).not.toBeInTheDocument();
	});
});

describe('your own name (#113)', () => {
	/** A Kitchen of your own and one you share with Marie, as the server names them now. */
	const kitchensNaming = (you: string) => ({
		kitchens: [
			{
				id: 'k_home',
				name: 'Maison Batterman',
				cookbooks: [cookbookLabel()],
				nickname: null,
				members: [{ person_id: 'p_1', name: you }],
			},
			{
				id: 'k_shared',
				name: 'Supper Club',
				cookbooks: [cookbookLabel()],
				nickname: null,
				members: [
					{ person_id: 'p_1', name: you },
					{ person_id: 'p_2', name: 'Marie' },
				],
			},
		],
	});

	const signedIn = (over: Answers = {}): Answers => ({
		instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
		list_sessions: { sessions: [] },
		list_access_keys: { access_keys: [] },
		...readsInAmerican,
		list_tags: { tags: [] },
		list_kitchens: kitchensNaming('Stéphane'),
		...over,
	});

	/** The You section alone, so a name elsewhere on the screen cannot answer for it. */
	const youSection = async () =>
		(await screen.findByRole('heading', { name: 'You' })).closest('section') as HTMLElement;

	it('shows your name first, and marks your own row in every Kitchen and your Cookbook as you', async () => {
		renderScreen(Settings, signedIn());

		const you = await youSection();
		expect(within(you).getByText('Stéphane')).toBeInTheDocument();
		expect(
			within(you).getByText('The name on your Versions, and the one you sign in with.'),
		).toBeInTheDocument();
		// It is the first section on the screen.
		expect(document.querySelector('section h2')?.textContent?.trim()).toBe('You');

		await screen.findByDisplayValue('Supper Club');
		const marked = screen.getAllByText('you');
		// Two Kitchens, and the Written by list of your Cookbook (#131).
		expect(marked).toHaveLength(3);
		for (const mark of marked) expect(mark.closest('li')?.textContent).toContain('Stéphane');
		const marieRow = screen.getByText('Marie').closest('li') as HTMLElement;
		expect(within(marieRow).queryByText('you')).not.toBeInTheDocument();
	});

	it('says what renaming reaches before anything is renamed', async () => {
		const { kamosu } = renderScreen(Settings, signedIn());

		await fireEvent.click(
			within(await youSection()).getByRole('button', { name: 'Change your name' }),
		);

		expect(screen.getByLabelText('Your name')).toHaveValue('Stéphane');
		expect(
			screen.getByText('Every Version you ever saved shows the new name, in every Kitchen.'),
		).toBeInTheDocument();
		expect(
			screen.getByText(
				"You'll sign in with the new name. Recipe files already sent keep the old one.",
			),
		).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('rename_person');

		await fireEvent.click(screen.getByRole('button', { name: 'Keep it' }));
		expect(within(await youSection()).getByText('Stéphane')).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('rename_person');
	});

	it('renames you through the Operation, and every place on the screen reads the new name', async () => {
		let renamed = false;
		const { kamosu } = renderScreen(
			Settings,
			signedIn({
				list_kitchens: () => kitchensNaming(renamed ? 'Stéphane Dupont' : 'Stéphane'),
				get_cookbook: () =>
					cookbookOf({
						authors: [{ person_id: 'p_1', name: renamed ? 'Stéphane Dupont' : 'Stéphane' }],
					}),
				rename_person: () => {
					renamed = true;
					return { name: 'Stéphane Dupont' };
				},
			}),
		);

		await fireEvent.click(
			within(await youSection()).getByRole('button', { name: 'Change your name' }),
		);
		await fireEvent.input(screen.getByLabelText('Your name'), {
			target: { value: 'Stéphane Dupont' },
		});
		await fireEvent.click(screen.getByRole('button', { name: 'Change my name' }));

		expect(
			await screen.findByText(
				'Done. Your Versions now say Stéphane Dupont. Sign in with that name from now on.',
			),
		).toBeInTheDocument();
		expect(kamosu.calls).toContainEqual({
			operation: 'rename_person',
			input: { name: 'Stéphane Dupont' },
		});
		const you = await youSection();
		expect(within(you).getByText('Stéphane Dupont')).toBeInTheDocument();
		expect(within(you).getByRole('button', { name: 'Change your name' })).toBeInTheDocument();
		// Your rows in the Kitchens and your Cookbook were read again, and
		// still say they are you.
		await vi.waitFor(() => expect(screen.getAllByText(/^Stéphane Dupont$/).length).toBe(4));
		expect(screen.getAllByText('you')).toHaveLength(3);
	});

	it('says why a name was refused, and keeps the field open to try another', async () => {
		renderScreen(
			Settings,
			signedIn({
				rename_person: {
					refuse: 'bad_request',
					message: 'somebody here already signs in as Marie; choose another name',
				},
			}),
		);

		await fireEvent.click(
			within(await youSection()).getByRole('button', { name: 'Change your name' }),
		);
		await fireEvent.input(screen.getByLabelText('Your name'), { target: { value: 'Marie' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Change my name' }));

		expect(await screen.findByRole('alert')).toHaveTextContent(
			'somebody here already signs in as Marie; choose another name',
		);
		expect(screen.getByLabelText('Your name')).toHaveValue('Marie');
		expect(screen.queryByText(/^Done\./)).not.toBeInTheDocument();
	});

	it('shows a stranger no name to change', async () => {
		renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
			...anonymous,
		});

		await screen.findByText(/Version 0\.1\.0/);
		expect(screen.queryByRole('heading', { name: 'You' })).not.toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Change your name' })).not.toBeInTheDocument();
	});
});

describe('telling your Sessions apart (#114)', () => {
	/** You are on the iPhone; also signed in on a laptop, and on something
	 * from before Sessions were named for their device. */
	const session = (id: string, name: string, current = false) => ({
		id,
		name,
		created_at: '2026-09-12T09:00:00Z',
		last_used_at: '2026-09-20T18:28:00Z',
		revoked: false,
		current,
	});
	const three = [
		session('s_laptop', 'Firefox · Linux'),
		session('s_phone', 'Safari · iPhone', true),
		session('s_old', 'this browser'),
	];

	const signedIn = (over: Answers = {}): Answers => ({
		instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
		list_sessions: { sessions: three },
		list_access_keys: { access_keys: [] },
		...readsInAmerican,
		list_tags: { tags: [] },
		list_kitchens: { kitchens: [] },
		...over,
	});

	const rows = async () => {
		const heading = await screen.findByRole('heading', { name: 'Sessions' });
		const list = heading.nextElementSibling as HTMLElement;
		return within(list).getAllByRole('listitem');
	};

	it('puts the device in your hand first and marks it, and names every other one', async () => {
		renderScreen(Settings, signedIn());

		const [first, second, third] = await rows();
		expect(within(first).getByText('Safari · iPhone')).toBeInTheDocument();
		expect(within(first).getByText('This device')).toBeInTheDocument();
		expect(within(first).getByRole('button', { name: 'Sign out' })).toBeInTheDocument();
		expect(within(first).queryByRole('button', { name: 'End' })).not.toBeInTheDocument();

		expect(within(second).getByText('Firefox · Linux')).toBeInTheDocument();
		expect(within(second).queryByText('This device')).not.toBeInTheDocument();
		expect(within(second).getByRole('button', { name: 'End' })).toBeInTheDocument();

		// A Session from before names says what it is, rather than "this browser",
		// which would be a lie on every device but one.
		expect(within(third).getByText('An older sign-in, not named')).toBeInTheDocument();
		expect(within(third).queryByText('this browser')).not.toBeInTheDocument();
		expect(within(third).getByText(/^Signed in .* · Last used /)).toBeInTheDocument();
		expect(screen.getAllByText('This device')).toHaveLength(1);
	});

	it('ends an older Session on its own, and stays here', async () => {
		let ended = false;
		const { kamosu } = renderScreen(
			Settings,
			signedIn({
				list_sessions: () => ({ sessions: ended ? three.slice(0, 2) : three }),
				revoke_session: () => {
					ended = true;
					return { revoked: true };
				},
			}),
		);

		const old = (await rows())[2];
		await fireEvent.click(within(old).getByRole('button', { name: 'End' }));

		expect(kamosu.calls).toContainEqual({
			operation: 'revoke_session',
			input: { session_id: 's_old' },
		});
		await vi.waitFor(async () => expect(await rows()).toHaveLength(2));
		expect(went).not.toHaveBeenCalled();
	});

	it('signs you out when the Session ended is the one in your hand', async () => {
		const { kamosu } = renderScreen(Settings, signedIn({ revoke_session: { revoked: true } }));

		await fireEvent.click(within((await rows())[0]).getByRole('button', { name: 'Sign out' }));

		expect(kamosu.calls).toContainEqual({
			operation: 'revoke_session',
			input: { session_id: 's_phone' },
		});
		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/'));
		// So the sidebar is gone before Home is drawn, not a moment after (#194).
		expect(signingIn.signedIn).toBe(false);
	});

	it('renames an older Session in place, starting from an empty name', async () => {
		let renamed = false;
		const { kamosu } = renderScreen(
			Settings,
			signedIn({
				list_sessions: () => ({
					sessions: renamed ? [three[0], three[1], session('s_old', 'Kitchen laptop')] : three,
				}),
				rename_session: () => {
					renamed = true;
					return { id: 's_old', name: 'Kitchen laptop' };
				},
			}),
		);

		await fireEvent.click(within((await rows())[2]).getByRole('button', { name: 'Rename' }));
		const field = screen.getByLabelText('Call it');
		expect(field).toHaveValue('');
		await fireEvent.input(field, { target: { value: 'Kitchen laptop' } });
		await fireEvent.click(within((await rows())[2]).getByRole('button', { name: 'Save' }));

		expect(kamosu.calls).toContainEqual({
			operation: 'rename_session',
			input: { session_id: 's_old', name: 'Kitchen laptop' },
		});
		expect(await screen.findByText('Kitchen laptop')).toBeInTheDocument();
		expect(screen.queryByText('An older sign-in, not named')).not.toBeInTheDocument();
		expect(screen.queryByLabelText('Call it')).not.toBeInTheDocument();
	});

	it('starts renaming a named Session from its name, and Keep it changes nothing', async () => {
		const { kamosu } = renderScreen(Settings, signedIn());

		await fireEvent.click(within((await rows())[0]).getByRole('button', { name: 'Rename' }));
		expect(screen.getByLabelText('Call it')).toHaveValue('Safari · iPhone');
		await fireEvent.click(screen.getByRole('button', { name: 'Keep it' }));

		expect(screen.queryByLabelText('Call it')).not.toBeInTheDocument();
		expect(screen.getByText('Safari · iPhone')).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('rename_session');
	});

	it('says why a rename was refused, and keeps the field open', async () => {
		renderScreen(
			Settings,
			signedIn({
				rename_session: {
					refuse: 'not_found',
					message: 'no live Session with that id belongs to this Person',
				},
			}),
		);

		await fireEvent.click(within((await rows())[1]).getByRole('button', { name: 'Rename' }));
		await fireEvent.input(screen.getByLabelText('Call it'), { target: { value: 'Work' } });
		await fireEvent.click(within((await rows())[1]).getByRole('button', { name: 'Save' }));

		expect(await screen.findByRole('alert')).toHaveTextContent(
			'no live Session with that id belongs to this Person',
		);
		expect(screen.getByLabelText('Call it')).toHaveValue('Work');
	});
});

// #131, screen choices 2 and 5: your Cookbook, above the Kitchens, with the
// one-use link that makes it a Cookbook you write with somebody.
describe('your Cookbook (#131)', () => {
	const signedIn = (over: Answers = {}): Answers => ({
		instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
		list_sessions: { sessions: [] },
		list_access_keys: { access_keys: [] },
		...readsInAmerican,
		list_kitchens: { kitchens: [] },
		...over,
	});
	const card = async () =>
		(await screen.findByRole('heading', { level: 2, name: 'Your Cookbook' })).closest(
			'section',
		) as HTMLElement;
	const TOGETHER = cookbookOf({
		authors: [
			{ person_id: 'p_1', name: 'Stéphane' },
			{ person_id: 'p_2', name: 'Camille' },
		],
		recipe_count: 99,
	});

	it('says who writes it and how many recipes, and is named after them until renamed', async () => {
		renderScreen(Settings, signedIn());
		const book = within(await card());
		const name = book.getByLabelText('Your Cookbook’s name');
		expect(name).toHaveValue('');
		expect(name).toHaveAttribute('placeholder', 'Stéphane’s');
		expect(book.getByText('Named after its writer until you rename it.')).toBeInTheDocument();
		expect(
			book.getByText(/^87 recipes\. Only this Cookbook's writers change them/),
		).toBeInTheDocument();
		// A Cookbook of one has nobody to leave.
		expect(book.queryByRole('button', { name: 'Leave this Cookbook' })).not.toBeInTheDocument();
	});

	it('renames it, and clears the name back to its authors’ with an empty box', async () => {
		const { kamosu } = renderScreen(
			Settings,
			signedIn({ rename_cookbook: cookbookOf({ name: 'Chez nous' }) }),
		);
		const book = within(await card());
		await fireEvent.input(book.getByLabelText('Your Cookbook’s name'), {
			target: { value: '  Chez nous ' },
		});
		await fireEvent.click(book.getByRole('button', { name: 'Save' }));
		await waitFor(() =>
			expect(kamosu.calls).toContainEqual({
				operation: 'rename_cookbook',
				input: { name: 'Chez nous' },
			}),
		);

		await fireEvent.input(book.getByLabelText('Your Cookbook’s name'), { target: { value: '' } });
		await fireEvent.click(book.getByRole('button', { name: 'Save' }));
		await waitFor(() =>
			expect(kamosu.calls).toContainEqual({ operation: 'rename_cookbook', input: { name: null } }),
		);
	});

	it('mints a one-use link on this instance, copies it, and cancels it', async () => {
		let minted = false;
		const { kamosu } = renderScreen(
			Settings,
			signedIn({
				invite_to_cookbook: () => {
					minted = true;
					return { invite_id: 'ci_1', secret: 'the-cookbook-secret' };
				},
				get_cookbook: () =>
					cookbookOf({
						invites: minted ? [{ invite_id: 'ci_1', created_at: '2026-09-24T08:00:00Z' }] : [],
					}),
				cancel_cookbook_invite: () => {
					minted = false;
					return { ended: true };
				},
			}),
		);
		const writeText = vi.fn(async () => {});
		Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });

		await fireEvent.click(
			within(await card()).getByRole('button', { name: 'Write it with someone' }),
		);
		const link = `${location.origin}/cookbook-invite/the-cookbook-secret`;
		expect(await screen.findByText(link)).toBeInTheDocument();
		expect(
			screen.getByText(
				'Send this to the person. Opening it makes your recipes one Cookbook you both change. Works once.',
			),
		).toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: 'Copy' }));
		expect(writeText).toHaveBeenCalledWith(link);
		expect(await screen.findByRole('button', { name: 'Copied' })).toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: 'Cancel it' }));
		await waitFor(() =>
			expect(kamosu.calls).toContainEqual({
				operation: 'cancel_cookbook_invite',
				input: { invite_id: 'ci_1' },
			}),
		);
		await waitFor(() => expect(screen.queryByText(link)).not.toBeInTheDocument());
		Reflect.deleteProperty(navigator, 'clipboard');
	});

	it('offers to end a link minted on an earlier visit, which it cannot show again', async () => {
		const { kamosu } = renderScreen(
			Settings,
			signedIn({
				get_cookbook: cookbookOf({
					invites: [{ invite_id: 'ci_old', created_at: '2026-09-20T08:00:00Z' }],
				}),
				cancel_cookbook_invite: { ended: true },
			}),
		);
		const book = within(await card());
		expect(
			book.getByText('An invite to write it with you is waiting to be opened.'),
		).toBeInTheDocument();
		await fireEvent.click(book.getByRole('button', { name: 'Cancel it' }));
		await waitFor(() =>
			expect(kamosu.calls).toContainEqual({
				operation: 'cancel_cookbook_invite',
				input: { invite_id: 'ci_old' },
			}),
		);
	});

	it('asks before leaving a Cookbook written together, saying what each keeps', async () => {
		const { kamosu } = renderScreen(
			Settings,
			signedIn({ get_cookbook: TOGETHER, leave_cookbook: cookbookOf({ recipe_count: 99 }) }),
		);
		const book = within(await card());
		expect(
			book.getByText('Named after its writers until one of you renames it.'),
		).toBeInTheDocument();
		await fireEvent.click(book.getByRole('button', { name: 'Leave this Cookbook' }));

		const sheet = within(await screen.findByRole('dialog'));
		expect(sheet.getByRole('heading', { name: 'Leave Stéphane and Camille?' })).toBeInTheDocument();
		expect(
			sheet.getByText(
				'You take a copy of all 99 recipes, with their history. The others keep theirs.',
			),
		).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('leave_cookbook');

		await fireEvent.click(sheet.getByRole('button', { name: 'Leave' }));
		await waitFor(() =>
			expect(kamosu.calls.map((call) => call.operation)).toContain('leave_cookbook'),
		);
		await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
	});

	it('asks before removing a Co-author, and removes them by the Operation', async () => {
		const { kamosu } = renderScreen(
			Settings,
			signedIn({ get_cookbook: TOGETHER, remove_cookbook_author: cookbookOf() }),
		);
		const camille = (await card()).querySelectorAll('li');
		const row = [...camille].find((li) => li.textContent?.includes('Camille')) as HTMLElement;
		await fireEvent.click(within(row).getByRole('button', { name: 'Remove' }));

		const sheet = within(await screen.findByRole('dialog'));
		expect(sheet.getByRole('heading', { name: 'Stop writing with Camille?' })).toBeInTheDocument();
		await fireEvent.click(sheet.getByRole('button', { name: 'Remove Camille' }));
		await waitFor(() =>
			expect(kamosu.calls).toContainEqual({
				operation: 'remove_cookbook_author',
				input: { person_id: 'p_2' },
			}),
		);
	});

	describe('a join waiting on answers (#135)', () => {
		type Join = GetCookbookOutput['joins'][number];
		/** Stéphane (you, p_1) said yes to Hélène's Invite; Hélène writes with Bob. */
		const joinOf = (over: Partial<Join> = {}): Join => ({
			join_id: 'cj_1',
			state: 'waiting',
			accepted_by: { person_id: 'p_1', name: 'Stéphane' },
			invited_by: { person_id: 'p_3', name: 'Hélène' },
			joining: { id: 'c_1', name: null, authors: TOGETHER.authors },
			into: {
				id: 'c_2',
				name: null,
				authors: [
					{ person_id: 'p_3', name: 'Hélène' },
					{ person_id: 'p_4', name: 'Bob' },
				],
			},
			together_recipes: 120,
			waiting_on: [
				{ person_id: 'p_2', name: 'Camille' },
				{ person_id: 'p_4', name: 'Bob' },
			],
			you: 'accepted',
			refused_by: null,
			refused_by_co_author: false,
			...over,
		});

		it('says whom it waits for, and lets the one who accepted take the yes back', async () => {
			const { kamosu } = renderScreen(
				Settings,
				signedIn({
					get_cookbook: { ...TOGETHER, joins: [joinOf()] },
					answer_cookbook_join: TOGETHER,
				}),
			);
			const book = within(await card());
			expect(book.getByText('One Cookbook with Hélène and Bob')).toBeInTheDocument();
			expect(book.getByText('Waiting for Camille and Bob to say yes.')).toBeInTheDocument();
			await fireEvent.click(book.getByRole('button', { name: 'Take my yes back' }));
			await waitFor(() =>
				expect(kamosu.calls).toContainEqual({
					operation: 'answer_cookbook_join',
					input: { join_id: 'cj_1', yes: false },
				}),
			);
			await waitFor(() =>
				expect(
					screen.queryByText('Waiting for Camille and Bob to say yes.'),
				).not.toBeInTheDocument(),
			);
		});

		it('offers anyone else in it the same no, as calling it off', async () => {
			renderScreen(
				Settings,
				signedIn({ get_cookbook: { ...TOGETHER, joins: [joinOf({ you: 'invited' })] } }),
			);
			const book = within(await card());
			expect(book.getByRole('button', { name: 'Call it off' })).toBeInTheDocument();
		});

		it('asks the question here too, to someone it waits on', async () => {
			const { kamosu } = renderScreen(
				Settings,
				signedIn({
					get_cookbook: { ...TOGETHER, joins: [joinOf({ you: 'asked' })] },
					answer_cookbook_join: TOGETHER,
				}),
			);
			const book = within(await card());
			expect(
				book.getByRole('heading', { name: 'Write one Cookbook with Hélène and Bob?' }),
			).toBeInTheDocument();
			await fireEvent.click(book.getByRole('button', { name: 'Write together' }));
			await waitFor(() =>
				expect(kamosu.calls).toContainEqual({
					operation: 'answer_cookbook_join',
					input: { join_id: 'cj_1', yes: true },
				}),
			);
		});

		it('tells the one who accepted who said no, and how to join anyway', async () => {
			renderScreen(
				Settings,
				signedIn({
					get_cookbook: {
						...TOGETHER,
						joins: [
							joinOf({
								state: 'refused',
								waiting_on: [],
								refused_by: { person_id: 'p_2', name: 'Camille' },
								refused_by_co_author: true,
							}),
						],
					},
				}),
			);
			const book = within(await card());
			expect(book.getByText('Camille said no, so nothing changed.')).toBeInTheDocument();
			expect(
				book.getByText(
					'To join anyway, leave this Cookbook first. You keep a copy of every recipe. Then open the same link again.',
				),
			).toBeInTheDocument();
		});

		it('gives no leaving advice where the no came from the other side', async () => {
			renderScreen(
				Settings,
				signedIn({
					get_cookbook: {
						...TOGETHER,
						joins: [
							joinOf({
								state: 'refused',
								waiting_on: [],
								refused_by: { person_id: 'p_4', name: 'Bob' },
							}),
						],
					},
				}),
			);
			const book = within(await card());
			expect(book.getByText('Bob said no, so nothing changed.')).toBeInTheDocument();
			expect(book.queryByText(/To join anyway/)).not.toBeInTheDocument();
		});
	});

	it('heads the Tags with your Cookbook, the only one whose words you rename', async () => {
		renderScreen(Settings, signedIn({ get_cookbook: cookbookOf({ name: 'Chez nous' }) }));
		const tags = (await screen.findByRole('heading', { name: 'Tags' })).closest(
			'section',
		) as HTMLElement;
		expect(await within(tags).findByRole('heading', { name: 'Chez nous' })).toBeInTheDocument();
	});
});
