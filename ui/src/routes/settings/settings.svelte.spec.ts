/**
 * The screen-seam test: Settings, against a stand-in Kamosu.
 *
 * What makes this worth having is not that it runs — it is that the answers
 * below are checked against the Catalogue before the screen sees them. Rename
 * `setup_complete` in `src/catalogue.rs` and this test fails, loudly, in the
 * same commit; it cannot quietly keep passing against a shape the Core no
 * longer serves.
 */

import { describe, expect, it } from 'vitest';
import { screen, fireEvent, within } from '@testing-library/svelte';
import Settings from './+page.svelte';
import { renderScreen } from '../../testing/render';
import type { Answers } from '$lib/api/stand-in';
import type { MeaningSearchStatusOutput } from '$lib/api/catalogue';

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

/**
 * A signed-in Person who has never touched the setting: reading in English, in
 * American measures. That is the stated default rather than a guess about
 * anybody (#49, ADR 0016), and every signed-in screen visit asks for it.
 */
const readsInAmerican: Answers = {
	get_reading_preferences: { reading_language: 'en', reading_measures: 'us' },
	// A signed-in Person who does not administer the instance (#103).
	list_accounts: { refuse: 'unauthorized' },
	// Meaning Search as most instances have it: nobody has been asked, so this
	// screen shows nothing about it at all. It is not where the feature is
	// discovered — that is inside *nothing found* (ADR 0029) — so its absence
	// here is the ordinary case rather than an omission.
	meaning_search_status: meaningStatus(),
	// No Crouton library brought in yet, so Settings links to no Report (#69).
	list_jobs: { jobs: [] },
};

describe('the settings screen', () => {
	it('asks the instance what it is, and says so', async () => {
		const { kamosu } = renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true },
			...anonymous,
		});

		expect(await screen.findByText(/Version 0\.1\.0/)).toBeInTheDocument();
		expect(screen.getByText(/Setup is complete/)).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).toContain('instance_status');
	});

	it("offers the Operator's screen only to somebody who administers the instance", async () => {
		renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true },
			...anonymous,
		});
		expect(await screen.findByText(/Version 0\.1\.0/)).toBeInTheDocument();
		expect(
			screen.queryByRole('link', { name: 'Administer this instance' }),
		).not.toBeInTheDocument();
	});

	it("shows the way into the Operator's screen to an Operator", async () => {
		renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true },
			...anonymous,
			list_accounts: {
				accounts: [
					{
						name: 'Aurélien',
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
			instance_status: { version: '0.1.0', setup_complete: false },
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
			instance_status: { version: '0.1.0', setup_complete: true },
			...anonymous,
		});

		for (const name of ['English', 'Français', 'Español']) {
			expect(screen.getByRole('button', { name })).toBeInTheDocument();
		}
		expect(screen.getByRole('button', { name: 'English' })).toHaveAttribute('aria-pressed', 'true');
	});

	it('shows nothing about Access when the visitor is not signed in', async () => {
		renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true },
			...anonymous,
		});

		await screen.findByText(/Version 0\.1\.0/);
		expect(screen.queryByText('Access')).not.toBeInTheDocument();
	});

	it("lists a signed-in Person's Sessions and Access Keys together, each ending on its own", async () => {
		const { kamosu } = renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true },
			list_accounts: { refuse: 'unauthorized' },
			list_sessions: {
				sessions: [
					{
						id: 's_1',
						name: 'iPhone',
						created_at: '2026-01-01T00:00:00Z',
						last_used_at: null,
						revoked: false,
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
			instance_status: { version: '0.1.0', setup_complete: true },
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

	it("shows a minted Access Key's secret once, then never again", async () => {
		const { kamosu } = renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true },
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

		expect(await screen.findByText('the-one-time-secret')).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).toContain('mint_access_key');

		await fireEvent.click(screen.getByRole('button', { name: 'Done, I copied it' }));
		expect(screen.queryByText('the-one-time-secret')).not.toBeInTheDocument();
	});

	it("lists a signed-in Person's Kitchens, marking the Home one and letting a member be removed", async () => {
		const { kamosu } = renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true },
			list_sessions: { sessions: [] },
			list_access_keys: { access_keys: [] },
			...readsInAmerican,
			list_kitchens: {
				kitchens: [
					{
						id: 'k_home',
						name: "Aurélien's Home Kitchen",
						hand_id: 'k_home',
						is_home: true,
						nickname: null,
						members: [{ person_id: 'p_1', name: 'Aurélien' }],
					},
					{
						id: 'k_shared',
						name: 'Supper Club',
						hand_id: 'k_shared',
						is_home: false,
						nickname: 'Nos amis',
						members: [
							{ person_id: 'p_1', name: 'Aurélien' },
							{ person_id: 'p_2', name: 'Marie' },
						],
					},
				],
			},
			remove_kitchen_member: { removed: true },
		});

		expect(await screen.findByDisplayValue('Supper Club')).toBeInTheDocument();
		expect(screen.getByDisplayValue("Aurélien's Home Kitchen")).toBeInTheDocument();
		expect(screen.getByText('Home')).toBeInTheDocument();
		expect(screen.getByDisplayValue('Nos amis')).toBeInTheDocument();

		const marieRow = screen.getByText('Marie').closest('li');
		await fireEvent.click(within(marieRow as HTMLElement).getByRole('button', { name: 'Remove' }));
		expect(kamosu.calls).toContainEqual({
			operation: 'remove_kitchen_member',
			input: { kitchen_id: 'k_shared', person_id: 'p_2' },
		});
	});

	it('creates a Kitchen from the form', async () => {
		const { kamosu } = renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true },
			list_sessions: { sessions: [] },
			list_access_keys: { access_keys: [] },
			...readsInAmerican,
			list_kitchens: { kitchens: [] },
			create_kitchen: {
				id: 'k_new',
				name: 'Supper Club',
				hand_id: 'k_new',
				is_home: false,
				nickname: null,
				members: [{ person_id: 'p_1', name: 'Aurélien' }],
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
			instance_status: { version: '0.1.0', setup_complete: true },
			list_sessions: { sessions: [] },
			list_access_keys: { access_keys: [] },
			...readsInAmerican,
			list_kitchens: {
				kitchens: [
					{
						id: 'k_home',
						name: "Aurélien's Home Kitchen",
						hand_id: 'k_home',
						is_home: true,
						nickname: null,
						members: [{ person_id: 'p_1', name: 'Aurélien' }],
					},
				],
			},
			invite_to_kitchen: { invite_id: 'ki_1', secret: 'the-invite-secret' },
		});

		await screen.findByDisplayValue("Aurélien's Home Kitchen");
		await fireEvent.click(screen.getByRole('button', { name: 'Invite someone' }));

		expect(await screen.findByText('the-invite-secret')).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).toContain('invite_to_kitchen');

		await fireEvent.click(screen.getByRole('button', { name: 'Done, I copied it' }));
		expect(screen.queryByText('the-invite-secret')).not.toBeInTheDocument();
	});

	it('joins a Kitchen through a pasted Invite', async () => {
		const { kamosu } = renderScreen(Settings, {
			instance_status: { version: '0.1.0', setup_complete: true },
			list_sessions: { sessions: [] },
			list_access_keys: { access_keys: [] },
			...readsInAmerican,
			list_kitchens: { kitchens: [] },
			accept_kitchen_invite: {
				id: 'k_shared',
				name: 'Supper Club',
				hand_id: 'k_shared',
				is_home: false,
				nickname: null,
				members: [{ person_id: 'p_1', name: 'Aurélien' }],
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
			instance_status: { version: '0.1.0', setup_complete: true },
			list_sessions: { sessions: [] },
			list_access_keys: { access_keys: [] },
			...readsInAmerican,
			list_kitchens: {
				// The dev instance's own six, home Kitchen included.
				kitchens: [
					"Aurélien's Home Kitchen",
					'Chez Marc',
					'Chez Élodie',
					'Le Chalet',
					'Chez Papi',
					'Chez Marie',
				].map((name, index) => ({
					id: `k_${index}`,
					name,
					hand_id: `k_${index}`,
					is_home: index === 0,
					nickname: null,
					members: [{ person_id: 'p_1', name: 'Aurélien' }],
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
			instance_status: { version: '0.1.0', setup_complete: true },
			...anonymous,
		});
		// The test browser is not a secure page, so this phone can keep nothing.
		expect(await screen.findByText('This phone')).toBeInTheDocument();
		expect(screen.getByText(/reached over http:\/\//)).toBeInTheDocument();
	});
});
