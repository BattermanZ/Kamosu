<!--
	Settings: reached through the *You* card in the header (#102), never from the
	tab bar.

	This is also where the signature transition is shown in the shell: that card
	and this screen's title carry the same `view-transition-name`, so the card
	grows into the page rather than the screen swapping (ADR 0012). Every recipe
	card that opens into its page later is this same pair, with a different name.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { m } from '$lib/paraglide/messages';
	import { getLocale, locales, setLocale, type Locale } from '$lib/paraglide/runtime';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';
	import Section from '$lib/shell/Section.svelte';
	import { MeaningSearch } from '$lib/meaning.svelte';
	import { asksWhetherAdministering } from '$lib/operator/administering';
	import ImportCrouton from './ImportCrouton.svelte';
	import Tags from './Tags.svelte';
	import Confirm from '$lib/Confirm.svelte';
	import {
		cookbookCalled,
		cookbookPlainName,
		cookbookWhose,
		joinedNames,
		kitchenName,
	} from '$lib/cookbook';
	import InstallSteps from '$lib/offline/InstallSteps.svelte';
	import { thisDevice } from '$lib/offline/device.svelte';
	import { readableSize, useLibrary } from '$lib/offline/library.svelte';
	import { languageName } from '$lib/language';
	import type { ReadingLanguage } from '$lib/tags';
	import type {
		InstanceStatusOutput,
		ListSessionsOutput,
		ListAccessKeysOutput,
		ListKitchensOutput,
		GetCookbookOutput,
		GetReadingPreferencesOutput,
		GetPersonOutput,
	} from '$lib/api/catalogue';

	const kamosu = useKamosu();

	/** Where the line saying what moved waits out a reload (#112). */
	const MOVED_KEY = 'kamosu.reading-language-moved';

	let status = $state<InstanceStatusOutput | undefined>(undefined);
	let failed = $state(false);

	// The one Operation this shell calls: the instance says what version it is and
	// whether setup has happened. Everything else arrives with later tickets.
	$effect(() => {
		let current = true;
		kamosu
			.instanceStatus()
			.then((answer) => {
				if (current) status = answer;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				if (current) failed = true;
			});
		return () => {
			current = false;
		};
	});

	// --- Meaning Search (#63, ADR 0029) -----------------------------------
	//
	// Deliberately NOT where it is discovered — a settings screen is where
	// features go to be undiscovered, which is why the offer lives inside
	// *nothing found* instead. This section appears only to an Operator, and
	// only once somebody has answered that offer: it is where they manage what
	// they chose — turn it off, or change their mind about having declined.
	// ADR 0029's rule is that the offer is never *made* twice; the nag is what
	// it forbids, not the door.

	const meaning = new MeaningSearch(kamosu);
	$effect(() => meaning.ask());

	// --- The way into the Operator's screen (#103) -------------------------
	//
	// Kamosu has no Operation answering *who is signed in*, so whether this
	// Person administers the instance is learnt by asking something only an
	// Operator may ask. `list_accounts` is the cheapest of those and the one
	// the screen itself opens with.
	let mayAdminister = $state(false);

	$effect(() => {
		let current = true;
		asksWhetherAdministering(kamosu).then((answer) => {
			// Only a plain yes opens the door. A Kamosu that could not answer
			// is not a Kamosu this Person may administer *yet*, and offering a
			// way in that then refuses would be the worse of the two.
			if (current) mayAdminister = answer.may === true;
		});
		return () => {
			current = false;
		};
	});

	// --- This phone (#76) --------------------------------------------------
	//
	// Where the "not right now" facts live once their cards are put away: the
	// card says a thing once, and this is where it can be read again.
	const device = thisDevice();
	const library = useLibrary();

	const names: Record<Locale, () => string> = {
		en: () => m.language_en(),
		fr: () => m.language_fr(),
		es: () => m.language_es(),
	};

	/**
	 * Reading Measures: how this Person measures (#49, ADR 0016).
	 *
	 * It lives on the ACCOUNT and not in this browser, which is the whole point
	 * — a cook who switches to metric on her phone finds the recipe in metric on
	 * the iPad on the worktop, and an agent at the MCP door reads it the same
	 * way. Setting it stores nothing on any recipe and makes no Version.
	 *
	 * The Reading Language rides along because `set_reading_preferences` takes
	 * the two together. What is sent back is the account's OWN language, not the
	 * interface locale above: changing Measures changes nothing else, and
	 * quietly overwriting the Language would be a lie about what the button did.
	 */
	type Measures = GetReadingPreferencesOutput['reading_measures'];

	const measureNames: Record<Measures, () => string> = {
		us: () => m.measures_us(),
		metric: () => m.measures_metric(),
		as_written: () => m.measures_as_written(),
	};

	let preferences = $state<GetReadingPreferencesOutput | undefined>(undefined);

	// --- Reading Language (#112) --------------------------------------------
	//
	// Two settings, shown as one until somebody wants them apart. That is
	// Aurélien's choice of 23 September 2026 (option C). The interface locale is
	// Paraglide's and lives in this browser; the Reading Language is the
	// account's, and it decides which Language a recipe's title, a Tag and a
	// Food are shown in at every Door (ADR 0006). Most people want both the
	// same, so the one control moves both. A French speaker keeping an English
	// library is ordinary too, so the recipes can split off.
	//
	// Whether they are split is read off the facts when the screen opens, and
	// never stored: a browser whose locale differs from the account's Reading
	// Language opens split, because showing it folded would claim the recipes
	// follow a control they do not follow.
	const locale = getLocale();
	let split = $state(false);
	/** The Language the shelf just moved to, said once beneath the picker. */
	let moved = $state<ReadingLanguage | undefined>(movedBeforeReload());
	/** A change of Language running, so a second tap cannot race the first. */
	let changing = $state(false);

	/**
	 * Choosing a language with the two together reloads the page, which would
	 * take the line saying what moved with it. So it is left for the next load
	 * of this screen, in this tab only, and read once.
	 */
	function movedBeforeReload(): ReadingLanguage | undefined {
		try {
			const left = sessionStorage.getItem(MOVED_KEY);
			sessionStorage.removeItem(MOVED_KEY);
			return locales.find((known) => known === left);
		} catch {
			return undefined;
		}
	}

	/** A Language other than the one being read in: the mark a card would carry. */
	const markedExample = $derived(
		locales.find((other) => other !== preferences?.reading_language) ?? 'en',
	);

	/**
	 * Store part of the reading preferences on the account, sending the rest
	 * back as they are. False where the account refused, and the screen is put
	 * back to what the account still holds.
	 */
	async function savePreferences(change: Partial<GetReadingPreferencesOutput>): Promise<boolean> {
		if (!preferences) return false;
		const previous = preferences;
		const next = { ...previous, ...change };
		if (
			next.reading_language === previous.reading_language &&
			next.reading_measures === previous.reading_measures
		) {
			return true;
		}
		preferences = next;
		try {
			await kamosu.setReadingPreferences(next);
			return true;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			preferences = previous;
			return false;
		}
	}

	async function chooseWords(next: Locale) {
		if (next === locale || changing) return;
		changing = true;
		// The account first: `setLocale` reloads the page, and a save still in
		// flight when it does may never land. A refused save still changes the
		// words, which is what was asked; the screen then opens split, saying
		// truthfully that the recipes did not follow.
		if (preferences && !split) {
			const before = preferences.reading_language;
			if ((await savePreferences({ reading_language: next })) && before !== next) {
				try {
					sessionStorage.setItem(MOVED_KEY, next);
				} catch {
					// Only the line saying what moved is lost; the move is made.
				}
			}
		}
		setLocale(next);
	}

	async function chooseReading(next: ReadingLanguage) {
		if (changing || preferences?.reading_language === next) return;
		changing = true;
		if (await savePreferences({ reading_language: next })) moved = next;
		changing = false;
	}

	async function rejoin() {
		if (changing) return;
		changing = true;
		const before = preferences?.reading_language;
		if (await savePreferences({ reading_language: locale })) {
			split = false;
			moved = before === locale ? undefined : locale;
		}
		changing = false;
	}

	function chooseMeasures(measures: Measures) {
		void savePreferences({ reading_measures: measures });
	}

	// --- You (#113) ---------------------------------------------------------
	//
	// Your own name, and where it is changed. Aurélien's choice of 23 September
	// 2026 (option A): a section of its own at the top, because the name is
	// about you rather than your devices, and it reaches your whole history.
	// Nothing is keyed on it (ADR 0015) — a Version carries a Hand, and the
	// server names that Hand live — so renaming rewrites no id and moves no
	// fingerprint. What it does reach is said before the button, since it is
	// also the name you sign in with.
	let me = $state<GetPersonOutput | undefined>(undefined);
	let editingName = $state(false);
	let draftName = $state('');
	let renaming = $state(false);
	let renameError = $state<string | undefined>(undefined);
	/** The name just taken, said once above the section. */
	let renamedTo = $state<string | undefined>(undefined);

	function startRenaming() {
		if (!me) return;
		draftName = me.name;
		renameError = undefined;
		renamedTo = undefined;
		editingName = true;
	}

	async function renameMe() {
		renaming = true;
		renameError = undefined;
		try {
			const answer = await kamosu.renamePerson({ name: draftName });
			if (me) me = { ...me, name: answer.name };
			renamedTo = answer.name;
			editingName = false;
			// Your own row in each Kitchen, and in your Cookbook, reads the
			// name the server holds.
			await Promise.all([loadKitchens(), loadCookbook()]);
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			renameError = error.message;
		} finally {
			renaming = false;
		}
	}

	// Sessions and Access Keys, listed together and each ending individually
	// from any device (ADR 0031). Reached only by a Person: a stranger visiting
	// this screen simply sees nothing here, rather than a refusal.
	type Session = ListSessionsOutput['sessions'][number];
	type AccessKey = ListAccessKeysOutput['access_keys'][number];

	let signedIn = $state(false);
	let sessions = $state<Session[]>([]);
	let accessKeys = $state<AccessKey[]>([]);
	let mintedSecret = $state<string | undefined>(undefined);
	let newKeyName = $state('');
	let newKeyReadOnly = $state(false);
	let minting = $state(false);
	let accessError = $state<string | undefined>(undefined);

	async function loadAccess() {
		try {
			const [sessionsAnswer, keysAnswer, preferencesAnswer, personAnswer] = await Promise.all([
				kamosu.listSessions(),
				kamosu.listAccessKeys(),
				kamosu.getReadingPreferences(),
				kamosu.getPerson(),
			]);
			sessions = sessionsAnswer.sessions.filter((session) => !session.revoked);
			accessKeys = keysAnswer.access_keys.filter((key) => !key.revoked);
			preferences = preferencesAnswer;
			me = personAnswer;
			split = preferencesAnswer.reading_language !== locale;
			signedIn = true;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			signedIn = false;
		}
	}

	$effect(() => {
		loadAccess();
	});

	function whenLastUsed(lastUsedAt: string | null): string {
		return lastUsedAt
			? m.access_last_used({ when: new Date(lastUsedAt).toLocaleString() })
			: m.access_never_used();
	}

	/**
	 * The Sessions as the list shows them: the one in your hand first, the
	 * rest newest first as the Core answers them. Ending the one in your hand
	 * signs you out here, so it is marked and its button says so (#114).
	 */
	const sessionsShown = $derived(
		[...sessions].sort((a, b) => Number(b.current) - Number(a.current)),
	);

	/**
	 * What every Session was called before a Session was named for its device.
	 * Kamosu cannot name such a row after the fact, so it says plainly what it
	 * is until its owner renames it.
	 */
	const UNNAMED_SESSION = 'this browser';

	let editingSession = $state<string | undefined>(undefined);
	let draftSessionName = $state('');
	let renamingSession = $state(false);
	let sessionRenameError = $state<string | undefined>(undefined);

	function startRenamingSession(session: Session) {
		editingSession = session.id;
		draftSessionName = session.name === UNNAMED_SESSION ? '' : session.name;
		sessionRenameError = undefined;
	}

	async function renameSession(sessionId: string) {
		renamingSession = true;
		sessionRenameError = undefined;
		try {
			await kamosu.renameSession({ session_id: sessionId, name: draftSessionName });
			editingSession = undefined;
			await loadAccess();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			sessionRenameError = error.message;
		} finally {
			renamingSession = false;
		}
	}

	async function endSession(session: Session) {
		await kamosu.revokeSession({ session_id: session.id });
		// Ending the Session in your hand is signing out: Home is the sign-in
		// form to somebody who is not signed in.
		if (session.current) {
			await goto('/');
			return;
		}
		await loadAccess();
	}

	async function endAccessKey(accessKeyId: string) {
		await kamosu.revokeAccessKey({ access_key_id: accessKeyId });
		await loadAccess();
	}

	async function mintKey() {
		minting = true;
		accessError = undefined;
		try {
			const created = await kamosu.mintAccessKey({ name: newKeyName, read_only: newKeyReadOnly });
			mintedSecret = created.secret;
			newKeyName = '';
			newKeyReadOnly = false;
			await loadAccess();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			accessError = error.message;
		} finally {
			minting = false;
		}
	}

	// Kitchens: every circle this Person cooks in. Loaded alongside Access —
	// both require a Credential, and a stranger simply sees neither.
	type Kitchen = ListKitchensOutput['kitchens'][number];

	let kitchens = $state<Kitchen[]>([]);
	let kitchensError = $state<string | undefined>(undefined);
	let mintedInvite = $state<{ kitchenId: string; secret: string } | undefined>(undefined);
	let newKitchenName = $state('');
	let joinSecret = $state('');
	let joining = $state(false);

	async function loadKitchens() {
		try {
			const answer = await kamosu.listKitchens();
			kitchens = answer.kitchens;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			kitchens = [];
		}
	}

	$effect(() => {
		if (signedIn) loadKitchens();
	});

	async function withKitchenError(action: () => Promise<void>) {
		kitchensError = undefined;
		try {
			await action();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			kitchensError = error.message;
		}
	}

	function renameKitchen(kitchenId: string, name: string) {
		return withKitchenError(async () => {
			await kamosu.renameKitchen({ kitchen_id: kitchenId, name });
			await loadKitchens();
		});
	}

	function setNickname(kitchenId: string, nickname: string) {
		return withKitchenError(async () => {
			await kamosu.setKitchenNickname({ kitchen_id: kitchenId, nickname: nickname || null });
			await loadKitchens();
		});
	}

	function removeMember(kitchenId: string, personId: string) {
		return withKitchenError(async () => {
			await kamosu.removeKitchenMember({ kitchen_id: kitchenId, person_id: personId });
			await loadKitchens();
		});
	}

	function inviteToKitchen(kitchenId: string) {
		return withKitchenError(async () => {
			const invite = await kamosu.inviteToKitchen({ kitchen_id: kitchenId });
			mintedInvite = { kitchenId, secret: invite.secret };
		});
	}

	async function createKitchen() {
		await withKitchenError(async () => {
			await kamosu.createKitchen({ name: newKitchenName });
			newKitchenName = '';
			await loadKitchens();
		});
	}

	async function joinKitchen() {
		joining = true;
		await withKitchenError(async () => {
			await kamosu.acceptKitchenInvite({ secret: joinSecret });
			joinSecret = '';
			await loadKitchens();
		});
		joining = false;
	}

	/**
	 * Leaving a Kitchen, asked first with what each side keeps (#131, screen
	 * choice 4). The counts come from the Core's own dry run of the leave, so
	 * the sheet says what the act will do rather than a guess at it.
	 */
	let leavingKitchen = $state<{ kitchen: Kitchen; theyKeep: number; youKeep: number } | undefined>(
		undefined,
	);
	let leaveBusy = $state(false);
	let leaveFailed = $state<string | undefined>(undefined);

	function askToLeaveKitchen(kitchen: Kitchen) {
		return withKitchenError(async () => {
			const preview = await kamosu.previewLeavingKitchen({ kitchen_id: kitchen.id });
			leaveFailed = undefined;
			leavingKitchen = { kitchen, theyKeep: preview.they_keep, youKeep: preview.you_keep };
		});
	}

	async function leaveKitchen() {
		const asked = leavingKitchen;
		if (!asked || !me) return;
		leaveBusy = true;
		leaveFailed = undefined;
		try {
			await kamosu.removeKitchenMember({ kitchen_id: asked.kitchen.id, person_id: me.person_id });
			leavingKitchen = undefined;
			await Promise.all([loadKitchens(), loadCookbook()]);
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			leaveFailed = error.message;
		} finally {
			leaveBusy = false;
		}
	}

	// Your Cookbook (#131, ADR 0041): the circle that CHANGES your recipes,
	// shown above the Kitchens, which only see them (screen choice 5).
	let cookbook = $state<GetCookbookOutput | undefined>(undefined);
	let cookbookError = $state<string | undefined>(undefined);
	/**
	 * The link just minted. Held only on this screen: the Core keeps a hash,
	 * so once this page is left the link cannot be shown again, only ended.
	 */
	let cookbookLink = $state<{ inviteId: string; url: string } | undefined>(undefined);
	let copied = $state(false);
	let cookbookAsk = $state<
		{ kind: 'leave' } | { kind: 'remove'; personId: string; name: string } | undefined
	>(undefined);
	let cookbookBusy = $state(false);
	let cookbookFailed = $state<string | undefined>(undefined);

	async function loadCookbook() {
		try {
			cookbook = await kamosu.getCookbook();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			cookbook = undefined;
		}
	}

	$effect(() => {
		if (signedIn) loadCookbook();
	});

	async function withCookbookError(action: () => Promise<void>) {
		cookbookError = undefined;
		try {
			await action();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			cookbookError = error.message;
		}
	}

	function renameCookbook(name: string) {
		return withCookbookError(async () => {
			cookbook = await kamosu.renameCookbook({ name: name.trim() || null });
			// Every Kitchen card names the Cookbooks it sees.
			await loadKitchens();
		});
	}

	function inviteToCookbook() {
		return withCookbookError(async () => {
			const invite = await kamosu.inviteToCookbook();
			copied = false;
			cookbookLink = {
				inviteId: invite.invite_id,
				url: `${location.origin}/cookbook-invite/${invite.secret}`,
			};
			await loadCookbook();
		});
	}

	function cancelCookbookInvite(inviteId: string) {
		return withCookbookError(async () => {
			await kamosu.cancelCookbookInvite({ invite_id: inviteId });
			if (cookbookLink?.inviteId === inviteId) cookbookLink = undefined;
			await loadCookbook();
		});
	}

	async function copyCookbookLink() {
		if (!cookbookLink) return;
		try {
			await navigator.clipboard.writeText(cookbookLink.url);
			copied = true;
		} catch {
			// A browser that will not copy still shows the link to select by hand.
			copied = false;
		}
	}

	async function answerCookbookAsk() {
		const asked = cookbookAsk;
		if (!asked) return;
		cookbookBusy = true;
		cookbookFailed = undefined;
		try {
			cookbook =
				asked.kind === 'leave'
					? await kamosu.leaveCookbook()
					: await kamosu.removeCookbookAuthor({ person_id: asked.personId });
			cookbookAsk = undefined;
			await loadKitchens();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			cookbookFailed = error.message;
		} finally {
			cookbookBusy = false;
		}
	}
</script>

{#snippet languageButton(label: string, pressed: boolean, choose: () => void)}
	<li>
		<button
			type="button"
			aria-pressed={pressed}
			onclick={choose}
			class="min-h-12 rounded-sm border px-4 text-body
				{pressed ? 'border-accent bg-accent text-on-accent' : 'border-rule bg-card text-ink'}"
		>
			{label}
		</button>
	</li>
{/snippet}

<Screen title={m.settings_title()} expandsFrom="settings">
	{#if signedIn && me}
		<Section heading={m.settings_you()}>
			{#if renamedTo}
				<p role="status" class="mb-3 border-l-3 border-accent bg-card px-3 py-2 text-read text-ink">
					{m.you_name_changed({ name: renamedTo })}
				</p>
			{/if}
			{#if editingName}
				<form
					class="grid gap-3"
					onsubmit={(event) => {
						event.preventDefault();
						renameMe();
					}}
				>
					<label class="grid gap-1 text-body text-ink">
						{m.you_name_label()}
						<input
							class="min-h-12 min-w-0 rounded-sm border border-rule bg-card px-3"
							bind:value={draftName}
							required
							{@attach (node) => node.focus()}
						/>
					</label>
					<!-- What renaming reaches, said before it happens. -->
					<p class="text-read text-ink">{m.you_name_reach_versions()}</p>
					<p class="text-read text-ink">{m.you_name_reach_sign_in()}</p>
					{#if renameError}
						<p class="text-body text-accent" role="alert">{renameError}</p>
					{/if}
					<div class="flex flex-wrap items-center gap-4">
						<button
							class="min-h-12 rounded-sm bg-accent px-4 font-semibold text-on-accent disabled:opacity-60"
							disabled={renaming}
						>
							{m.you_name_confirm()}
						</button>
						<button
							type="button"
							class="text-read text-accent underline"
							onclick={() => (editingName = false)}
						>
							{m.you_name_keep()}
						</button>
					</div>
				</form>
			{:else}
				<p class="font-display text-title font-semibold break-words text-ink">{me.name}</p>
				<p class="mt-1 mb-3 text-read text-ink-2">{m.you_name_explained()}</p>
				<button type="button" class="text-read text-accent underline" onclick={startRenaming}>
					{m.you_change_name()}
				</button>
			{/if}
		</Section>
	{/if}

	<Section heading={m.settings_language()}>
		<p class="mb-3 text-read text-ink-2">
			{preferences && !split ? m.settings_language_together() : m.settings_language_words()}
		</p>
		<!-- Paraglide compiles every phrase to a function, so this list can only
		     offer languages that were actually compiled. -->
		<ul class="flex flex-wrap gap-2" aria-label={m.settings_language()}>
			{#each locales as choice (choice)}
				{@render languageButton(names[choice](), locale === choice, () => chooseWords(choice))}
			{/each}
		</ul>

		{#if preferences && !split}
			<p class="mt-3 text-read text-ink-2">
				{m.settings_language_follows()}
				<button type="button" class="text-accent underline" onclick={() => (split = true)}>
					{m.settings_language_split()}
				</button>
			</p>
		{:else if preferences}
			<h3 id="reading-language" class="mt-6 mb-2 text-body font-semibold text-ink">
				{m.settings_reading_language()}
			</h3>
			<p class="mb-3 text-read text-ink-2">
				{m.settings_reading_language_explained()}
				<!-- An example of the mark a card carries (Tile.svelte's colours, at
				     the size of this sentence). The sentence is whole without it, so a
				     screen reader is not read a code. -->
				<span aria-hidden="true" class="rounded-sm bg-support px-1 text-label text-ground uppercase"
					>{markedExample}</span
				>
			</p>
			<ul class="flex flex-wrap gap-2" aria-labelledby="reading-language">
				{#each locales as choice (choice)}
					{@render languageButton(names[choice](), preferences.reading_language === choice, () =>
						chooseReading(choice),
					)}
				{/each}
			</ul>
			<button type="button" class="mt-3 text-read text-accent underline" onclick={rejoin}>
				{m.settings_reading_language_rejoin()}
			</button>
		{/if}
		{#if preferences && moved}
			<p role="status" class="mt-3 border-l-3 border-accent bg-card px-3 py-2 text-read text-ink">
				{m.settings_reading_language_moved({ language: languageName(moved) })}
			</p>
		{/if}
	</Section>

	<Section heading={m.settings_phone()}>
		<div class="space-y-3 text-body">
			<p class={device.secure ? '' : 'text-support'}>
				{device.secure ? m.offline_phone_secure() : m.offline_insecure_body()}
			</p>
			{#if device.secure}
				{#if device.installed}
					<p>{m.offline_phone_installed()}</p>
				{:else}
					<div><InstallSteps apple={device.apple} /></div>
				{/if}
				{#if library.phase === 'filling'}
					<p>{m.offline_library_filling({ done: library.done, total: library.total })}</p>
				{:else if library.phase === 'kept' && library.filledAt}
					<p>
						{m.offline_phone_kept({
							count: library.held.size,
							date: library.filledAt.toLocaleDateString(),
						})}
					</p>
					<button
						type="button"
						onclick={() => void library.fill()}
						class="min-h-12 rounded-sm border border-rule bg-card px-4 text-body text-accent"
					>
						{m.offline_phone_refresh()}
					</button>
				{:else if library.phase === 'absent'}
					<p>
						{m.offline_library_body({
							count: library.count,
							size: readableSize(library.bytes),
						})}
					</p>
					<button
						type="button"
						onclick={() => void library.fill()}
						class="min-h-12 rounded-sm border border-accent bg-accent px-4 text-body text-on-accent"
					>
						{m.offline_library_fetch()}
					</button>
				{/if}
			{/if}
		</div>
	</Section>

	{#if signedIn && preferences}
		<Section heading={m.settings_measures()}>
			<p class="mb-3 text-body text-ink-2">{m.measures_explained()}</p>
			<ul class="flex flex-wrap gap-2">
				{#each Object.entries(measureNames) as [measures, name] (measures)}
					{@const choice = measures as Measures}
					<li>
						<button
							type="button"
							aria-pressed={preferences.reading_measures === choice}
							onclick={() => chooseMeasures(choice)}
							class="min-h-12 rounded-sm border px-4 text-body
								{preferences.reading_measures === choice
								? 'border-accent bg-accent text-on-accent'
								: 'border-rule bg-card text-ink'}"
						>
							{name()}
						</button>
					</li>
				{/each}
			</ul>
		</Section>
	{/if}

	{#if signedIn}
		<ImportCrouton />
	{/if}

	<Section heading={m.settings_instance()}>
		<p class="text-body text-ink-2">
			{#if failed}
				{m.instance_unreachable()}
			{:else if status}
				{m.instance_version({ version: status.version })} ·
				{status.setup_complete ? m.instance_setup_done() : m.instance_setup_pending()}
			{:else}
				{m.loading()}
			{/if}
		</p>

		<!--
			The way into the Operator's screen (#103), shown only to somebody the
			instance has agreed may administer it. `list_accounts` is the asking:
			refused, this row is simply not here, which is the same answer the
			screen itself gives (ADR 0040). A row that appeared and then refused
			would be a worse way of saying the same thing.
		-->
		{#if mayAdminister}
			<a
				href="/operator"
				class="mt-3 flex min-h-12 items-center justify-between gap-3 rounded-sm border
				border-rule bg-card px-3 text-body font-medium text-ink"
			>
				{m.operator_open()}
				<span aria-hidden="true" class="text-ink-2">›</span>
			</a>
		{/if}
	</Section>

	{#if signedIn}
		<Section heading={m.settings_access()}>
			{#if mintedSecret}
				<div class="mb-4 rounded-sm border border-accent bg-card p-3" role="alert">
					<p class="text-body font-semibold text-ink">{m.access_key_secret_once()}</p>
					<code class="mt-2 block overflow-x-auto rounded-sm bg-ground p-2 text-read"
						>{mintedSecret}</code
					>
					<button
						type="button"
						class="mt-2 text-label text-accent underline"
						onclick={() => (mintedSecret = undefined)}
					>
						{m.access_key_secret_dismiss()}
					</button>
				</div>
			{/if}

			<h3 class="mb-2 text-body font-semibold text-ink">{m.settings_sessions()}</h3>
			{#if sessions.length === 0}
				<p class="mb-6 text-body text-ink-2">{m.access_sessions_empty()}</p>
			{:else}
				<ul class="mb-6 grid gap-2">
					{#each sessionsShown as session (session.id)}
						<li
							class={[
								'rounded-sm border bg-card px-3 py-2',
								session.current ? 'border-accent' : 'border-rule',
							]}
						>
							{#if editingSession === session.id}
								<form
									class="grid gap-2"
									onsubmit={(event) => {
										event.preventDefault();
										renameSession(session.id);
									}}
								>
									<label class="grid gap-1 text-read text-ink-2">
										{m.sessions_name_label()}
										<input
											class="min-h-12 min-w-0 rounded-sm border border-rule bg-ground px-3 text-body text-ink"
											bind:value={draftSessionName}
											required
											{@attach (node) => node.focus()}
										/>
									</label>
									{#if sessionRenameError}
										<p class="text-body text-accent" role="alert">{sessionRenameError}</p>
									{/if}
									<div class="flex flex-wrap items-center gap-4">
										<button
											class="min-h-12 rounded-sm bg-accent px-4 font-semibold text-on-accent disabled:opacity-60"
											disabled={renamingSession}
										>
											{m.sessions_name_save()}
										</button>
										<button
											type="button"
											class="text-read text-accent underline"
											onclick={() => (editingSession = undefined)}
										>
											{m.you_name_keep()}
										</button>
									</div>
								</form>
							{:else}
								<div class="flex items-center justify-between gap-3">
									<div class="min-w-0">
										<p class="text-body break-words text-ink">
											{#if session.name === UNNAMED_SESSION}
												<span class="text-ink-2 italic">{m.sessions_unnamed()}</span>
											{:else}
												{session.name}
											{/if}
											{#if session.current}
												<span class="ml-1 text-label font-semibold text-accent uppercase"
													>{m.sessions_this_device()}</span
												>
											{/if}
										</p>
										<p class="text-read text-ink-2">
											{m.sessions_signed_in({
												when: new Date(session.created_at).toLocaleDateString(),
											})} · {whenLastUsed(session.last_used_at)}
										</p>
									</div>
									<div class="flex shrink-0 flex-col items-end gap-2">
										<button
											type="button"
											class="text-label text-accent underline"
											onclick={() => endSession(session)}
										>
											{session.current ? m.sessions_sign_out() : m.access_end()}
										</button>
										<button
											type="button"
											class="text-label text-accent underline"
											onclick={() => startRenamingSession(session)}
										>
											{m.sessions_rename()}
										</button>
									</div>
								</div>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}

			<h3 class="mb-2 text-body font-semibold text-ink">{m.settings_access_keys()}</h3>
			{#if accessKeys.length === 0}
				<p class="mb-6 text-body text-ink-2">{m.access_keys_empty()}</p>
			{:else}
				<ul class="mb-6 grid gap-2">
					{#each accessKeys as key (key.id)}
						<li
							class="flex items-center justify-between gap-3 rounded-sm border border-rule bg-card px-3 py-2"
						>
							<div>
								<p class="text-body text-ink">
									{key.name}
									{#if key.read_only}
										<span class="ml-1 text-label text-ink-2 uppercase"
											>{m.access_key_read_only()}</span
										>
									{/if}
								</p>
								<p class="text-read text-ink-2">{whenLastUsed(key.last_used_at)}</p>
							</div>
							<button
								type="button"
								class="shrink-0 text-label text-accent underline"
								onclick={() => endAccessKey(key.id)}
							>
								{m.access_end()}
							</button>
						</li>
					{/each}
				</ul>
			{/if}

			<form
				class="grid gap-3"
				onsubmit={(event) => {
					event.preventDefault();
					mintKey();
				}}
			>
				<label class="grid gap-1 text-body text-ink">
					{m.access_key_name()}
					<input
						class="min-h-12 rounded-sm border border-rule bg-card px-3"
						bind:value={newKeyName}
						required
					/>
				</label>
				<label class="flex items-center gap-2 text-body text-ink">
					<input type="checkbox" bind:checked={newKeyReadOnly} />
					{m.access_key_read_only_label()}
				</label>
				{#if accessError}
					<p class="text-body text-accent" role="alert">{accessError}</p>
				{/if}
				<button
					class="min-h-12 rounded-sm bg-accent px-4 font-semibold text-on-accent disabled:opacity-60"
					disabled={minting}
				>
					{m.access_key_mint()}
				</button>
			</form>
		</Section>

		<!--
			Your Cookbook (#131), above the Kitchens because it is what your
			recipes ARE and the Kitchens are only who sees them (screen choice 5).
			One card, since a person writes exactly one Cookbook.
		-->
		{#if cookbook}
			{@const book = cookbook}
			<Section heading={m.settings_cookbook()}>
				{#if cookbookError}
					<p class="mb-4 text-body text-accent" role="alert">{cookbookError}</p>
				{/if}
				<div class="min-w-0 rounded-sm border border-t-4 border-rule border-t-accent bg-card p-3">
					<!-- Blank until named: the placeholder is the name it goes by
					     meanwhile, its Co-authors', so clearing the box is how a
					     name of its own is given back. -->
					<form
						class="flex items-center gap-2"
						onsubmit={(event) => {
							event.preventDefault();
							const input = event.currentTarget.elements.namedItem('name') as HTMLInputElement;
							renameCookbook(input.value);
						}}
					>
						<label class="sr-only" for="cookbook-name">{m.cookbook_name_label()}</label>
						<input
							id="cookbook-name"
							name="name"
							class="min-h-12 min-w-0 flex-1 rounded-sm border border-rule bg-ground px-2 font-semibold text-ink"
							placeholder={cookbookWhose({ ...book, name: null })}
							value={book.name ?? ''}
						/>
						<button class="shrink-0 text-label text-accent underline" type="submit">
							{m.kitchen_save()}
						</button>
					</form>
					<p class="mt-2 text-read text-ink-2">
						{book.authors.length > 1 ? m.cookbook_name_note_together() : m.cookbook_name_note()}
					</p>

					<h3 class="mt-3 mb-1 text-label text-ink-2 uppercase">{m.cookbook_written_by()}</h3>
					<ul class="grid gap-1">
						{#each book.authors as author (author.person_id)}
							<li class="flex items-center justify-between gap-3 text-body text-ink">
								<span>
									{author.name}
									{#if author.person_id === me?.person_id}
										<span class="ml-1 text-label text-ink-2 uppercase"
											>{m.kitchen_member_you()}</span
										>
									{/if}
								</span>
								{#if author.person_id !== me?.person_id}
									<button
										type="button"
										class="shrink-0 text-label text-accent underline"
										onclick={() => {
											cookbookFailed = undefined;
											cookbookAsk = {
												kind: 'remove',
												personId: author.person_id,
												name: author.name,
											};
										}}
									>
										{m.kitchen_remove()}
									</button>
								{/if}
							</li>
						{/each}
					</ul>
					<p class="mt-2 text-read text-ink-2">
						{book.recipe_count === 1
							? m.cookbook_count_one()
							: m.cookbook_count({ count: book.recipe_count })}
					</p>

					{#if cookbookLink}
						<div class="mt-3 rounded-sm border border-accent bg-ground p-3" role="status">
							<p class="text-body text-ink">{m.cookbook_invite_said()}</p>
							<code
								class="mt-2 block rounded-sm border border-dashed border-rule bg-card p-2 text-read break-all"
								>{cookbookLink.url}</code
							>
							<div class="mt-2 flex flex-wrap items-center gap-4">
								<button
									type="button"
									class="text-label text-accent underline"
									onclick={copyCookbookLink}
								>
									{copied ? m.cookbook_invite_copied() : m.cookbook_invite_copy()}
								</button>
								<button
									type="button"
									class="text-label text-support underline"
									onclick={() => cookbookLink && cancelCookbookInvite(cookbookLink.inviteId)}
								>
									{m.cookbook_invite_cancel()}
								</button>
							</div>
						</div>
					{:else}
						<!-- A link minted on an earlier visit cannot be shown again,
						     since only its hash is kept, but it can still be ended. -->
						{#each book.invites as invite (invite.invite_id)}
							<div class="mt-3 flex flex-wrap items-center justify-between gap-3">
								<p class="text-read text-ink-2">{m.cookbook_invite_waiting()}</p>
								<button
									type="button"
									class="shrink-0 text-label text-support underline"
									onclick={() => cancelCookbookInvite(invite.invite_id)}
								>
									{m.cookbook_invite_cancel()}
								</button>
							</div>
						{/each}
					{/if}

					<div class="mt-3 flex flex-wrap items-center justify-between gap-3">
						{#if !cookbookLink}
							<button
								type="button"
								class="text-label text-accent underline"
								onclick={inviteToCookbook}
							>
								{m.cookbook_invite()}
							</button>
						{/if}
						{#if book.authors.length > 1}
							<button
								type="button"
								class="text-label text-support underline"
								onclick={() => {
									cookbookFailed = undefined;
									cookbookAsk = { kind: 'leave' };
								}}
							>
								{m.cookbook_leave()}
							</button>
						{/if}
					</div>
				</div>
			</Section>
		{/if}

		<Section heading={m.settings_kitchens()}>
			{#if kitchensError}
				<p class="mb-4 text-body text-accent" role="alert">{kitchensError}</p>
			{/if}

			<ul class="grid gap-4">
				{#each kitchens as kitchen (kitchen.id)}
					<li class="min-w-0 rounded-sm border border-rule bg-card p-3">
						<!-- `min-w-0` in three places is what keeps this card inside a
						     narrow phone (#102). Both a grid item and a flex item take
						     `min-width: auto`, which means neither will shrink below its
						     content — and a text box's content is about twenty characters
						     wide whatever the screen. So the card above refused to narrow
						     past 308px and shoved the whole page sideways, header and all:
						     at a 320px viewport the page wanted 328, and a larger iOS text
						     size widens the same rows further.

						     It has to be on the <li> as well as on the two boxes. Freeing
						     only the boxes leaves the grid track sized to the card's own
						     minimum, which is the measurement that was wrong. The labels
						     keep their words by not stretching at all; only the boxes give. -->
						<form
							class="flex items-center gap-2"
							onsubmit={(event) => {
								event.preventDefault();
								const input = event.currentTarget.elements.namedItem('name') as HTMLInputElement;
								renameKitchen(kitchen.id, input.value);
							}}
						>
							<label class="sr-only" for={`kitchen-name-${kitchen.id}`}>
								{m.kitchen_name_label()}
							</label>
							<input
								id={`kitchen-name-${kitchen.id}`}
								name="name"
								class="min-h-12 min-w-0 flex-1 rounded-sm border border-rule bg-ground px-2 font-semibold text-ink"
								value={kitchen.name}
							/>
							<button class="shrink-0 text-label text-accent underline" type="submit">
								{m.kitchen_save()}
							</button>
						</form>

						<form
							class="mt-2 flex items-center gap-2"
							onsubmit={(event) => {
								event.preventDefault();
								const input = event.currentTarget.elements.namedItem(
									'nickname',
								) as HTMLInputElement;
								setNickname(kitchen.id, input.value);
							}}
						>
							<label class="shrink-0 text-label text-ink-2" for={`nickname-${kitchen.id}`}>
								{m.kitchen_nickname_label()}
							</label>
							<input
								id={`nickname-${kitchen.id}`}
								name="nickname"
								class="min-h-12 min-w-0 flex-1 rounded-sm border border-rule bg-ground px-2 text-body text-ink"
								placeholder={m.kitchen_nickname_placeholder()}
								value={kitchen.nickname ?? ''}
							/>
							<button class="shrink-0 text-label text-accent underline" type="submit">
								{m.kitchen_save()}
							</button>
						</form>

						<h3 class="mt-3 mb-1 text-label text-ink-2 uppercase">{m.kitchen_members()}</h3>
						<!-- Your own row carries no button: leaving is its own act at
						     the foot of the card, asked first with what each side
						     keeps (#131, screen choice 4). Since #131 a Kitchen never
						     refuses a leave, because nobody's recipes live in one. -->
						<ul class="grid gap-1">
							{#each kitchen.members as member (member.person_id)}
								<li class="flex items-center justify-between gap-3 text-body text-ink">
									<span>
										{member.name}
										{#if member.person_id === me?.person_id}
											<span class="ml-1 text-label text-ink-2 uppercase"
												>{m.kitchen_member_you()}</span
											>
										{/if}
									</span>
									{#if member.person_id !== me?.person_id}
										<button
											type="button"
											class="shrink-0 text-label text-accent underline"
											onclick={() => removeMember(kitchen.id, member.person_id)}
										>
											{m.kitchen_remove()}
										</button>
									{/if}
								</li>
							{/each}
						</ul>
						<!-- Whose recipes this Kitchen shows: every member's Cookbook,
						     your own first, as the Core answers them. -->
						{#if kitchen.cookbooks.length > 0}
							<p class="mt-2 text-read text-ink-2">
								{m.kitchen_recipes_here({
									cookbooks: joinedNames(kitchen.cookbooks.map(cookbookCalled)),
								})}
							</p>
						{/if}

						{#if mintedInvite?.kitchenId === kitchen.id}
							<div class="mt-3 rounded-sm border border-accent bg-ground p-3" role="alert">
								<p class="text-body font-semibold text-ink">{m.kitchen_invite_secret_once()}</p>
								<code class="mt-2 block overflow-x-auto rounded-sm bg-card p-2 text-read"
									>{mintedInvite.secret}</code
								>
								<button
									type="button"
									class="mt-2 text-label text-accent underline"
									onclick={() => (mintedInvite = undefined)}
								>
									{m.kitchen_invite_secret_dismiss()}
								</button>
							</div>
						{:else}
							<button
								type="button"
								class="mt-3 text-label text-accent underline"
								onclick={() => inviteToKitchen(kitchen.id)}
							>
								{m.kitchen_invite()}
							</button>
						{/if}
						<div class="mt-3">
							<button
								type="button"
								class="text-label text-support underline"
								onclick={() => askToLeaveKitchen(kitchen)}
							>
								{m.kitchen_leave_this()}
							</button>
						</div>
					</li>
				{/each}
			</ul>

			<h3 class="mt-6 mb-2 text-body font-semibold text-ink">{m.kitchen_create_title()}</h3>
			<form
				class="grid gap-3"
				onsubmit={(event) => {
					event.preventDefault();
					createKitchen();
				}}
			>
				<label class="grid gap-1 text-body text-ink">
					{m.kitchen_create_name_label()}
					<input
						class="min-h-12 rounded-sm border border-rule bg-card px-3"
						bind:value={newKitchenName}
						required
					/>
				</label>
				<button class="min-h-12 rounded-sm bg-accent px-4 font-semibold text-on-accent">
					{m.kitchen_create()}
				</button>
			</form>

			<h3 class="mt-6 mb-2 text-body font-semibold text-ink">{m.kitchen_join_title()}</h3>
			<form
				class="grid gap-3"
				onsubmit={(event) => {
					event.preventDefault();
					joinKitchen();
				}}
			>
				<label class="grid gap-1 text-body text-ink">
					{m.kitchen_join_secret_label()}
					<input
						class="min-h-12 rounded-sm border border-rule bg-card px-3"
						bind:value={joinSecret}
						required
					/>
				</label>
				<button
					class="min-h-12 rounded-sm bg-accent px-4 font-semibold text-on-accent disabled:opacity-60"
					disabled={joining}
				>
					{m.kitchen_join()}
				</button>
			</form>
		</Section>

		<!--
			Tags (#104). One card, your Cookbook's, since #131 put Tags in the
			Cookbook that writes the recipes they file.

			Renaming and merging are HERE and not on the recipe page, which is
			Aurélien's choice of 22 September 2026 — `Tags.svelte` beside this
			file records his reasoning. Putting them in Settings is not the same
			mistake Meaning Search would have made by living here: a tag is
			discovered on a recipe, where the row and the sheet are, and this is
			only where the list is tidied.
		-->
		<!--
			`readingLanguage` is the account's, not the interface's: this screen
			already holds the preferences, and naming a Tag in the locale the
			buttons happen to be in would name it in the wrong Language for
			anybody whose two settings differ (ADR 0006). Until the preferences
			land, English — the same fallback `set_reading_preferences` is
			offered with above.
		-->
		<Section heading={m.settings_tags()}>
			<Tags
				cookbook={cookbook ? cookbookCalled(cookbook) : m.settings_cookbook()}
				readingLanguage={preferences?.reading_language ?? 'en'}
			/>
		</Section>
	{/if}

	<!--
		The way to what has been brought in (#108). A door rather than the list
		itself, for the same reason Foods below is one: a source opens onto its
		own arrivals and onto forgetting, which is a screen and not a section.

		Beneath the Crouton importer on purpose. That control is where an import
		is *started*; this is where every import that ever ran is found again,
		including the forty-seven the importer's own "last report" link could
		never reach.
	-->
	{#if signedIn}
		<Section heading={m.settings_imports()}>
			<p class="text-body text-ink-2">{m.settings_imports_blurb()}</p>
			<a
				href="/imports"
				class="mt-3 flex min-h-12 items-center justify-between gap-3 rounded-sm border
				border-rule bg-card px-3 text-body font-medium text-ink"
			>
				{m.settings_imports_open()}
				<span aria-hidden="true" class="text-ink-2">›</span>
			</a>
		</Section>
	{/if}

	<!--
		The second of the two doors onto a Food (#107). The first is the Reading
		corrector on the recipe itself, which is where a wrong Cup Weight is
		noticed; this one is for the Food you have to go looking for, which is
		every Food whose name in another Language is missing — nothing on a
		recipe naming `salt` tells you a separate `sel` exists elsewhere.

		A door rather than the list itself, which is where Tags above differs: a
		Kitchen has a handful of Tags and this instance has 607 Foods, so the
		list is a screen with a search field on it and not a section anybody
		scrolls past.

		Shown to every Person, because a Food is instance-wide and all five of
		its Operations are `Permission::Person` — correcting one is not an
		Operator's power, and #103 holds the two that are.
	-->
	<Section heading={m.settings_foods()}>
		<p class="text-body text-ink-2">{m.settings_foods_blurb()}</p>
		<a
			href="/foods"
			class="mt-3 flex min-h-12 items-center justify-between gap-3 rounded-sm border
			border-rule bg-card px-3 text-body font-medium text-ink"
		>
			{m.settings_foods_open()}
			<span aria-hidden="true" class="text-ink-2">›</span>
		</a>
	</Section>

	<!--
		Shown only to somebody who could act on it, and only once they have
		answered the offer — this is where Meaning Search is *managed*, never
		where it is discovered. A settings screen is where features go to be
		undiscovered, which is why the offer lives inside nothing-found instead
		(ADR 0029).
	-->
	{#if meaning.status?.may_change && meaning.status.state !== 'unasked'}
		<Section heading={m.settings_meaning()}>
			{#if meaning.status.on}
				<p class="text-body text-ink">
					{m.settings_meaning_on({ model: meaning.status.model })}
				</p>
				{#if meaning.status.recipes_not_yet_indexed > 0}
					<!--
						Said rather than hidden: the index catches up by itself
						within the minute, and pretending an edit made a second
						ago is already findable by meaning would be a small lie
						with no upside.
					-->
					<p class="mt-1 text-read text-ink-2">
						{m.settings_meaning_catching_up({ count: meaning.status.recipes_not_yet_indexed })}
					</p>
				{/if}
				<!--
					No warning and no confirmation, because there is nothing to
					warn about: everything this discards is derived from the
					recipes and rebuilds itself (ADR 0003, ADR 0009).
				-->
				<button
					type="button"
					class="mt-3 text-label text-accent underline"
					onclick={() => meaning.turnOff()}
				>
					{m.settings_meaning_turn_off()}
				</button>
			{:else if meaning.working}
				<p class="text-body text-ink-2" role="status">{meaning.saying}</p>
			{:else}
				<p class="text-body text-ink-2">
					{meaning.status.state === 'declined'
						? m.settings_meaning_declined()
						: m.settings_meaning_off()}
				</p>
				<button
					type="button"
					class="mt-3 text-label text-accent underline"
					onclick={() => meaning.turnOn()}
				>
					{m.recipes_nothing_meaning_turn_on()}
				</button>
			{/if}
			{#if meaning.failed}
				<p class="mt-2 text-read text-support" role="alert">{meaning.failed}</p>
			{/if}
		</Section>
	{/if}

	<!-- The honest list ADR 0034 ships, the same seven items README.md carries
	     and worded shorter, because this one is read standing up. It is part of
	     the product rather than an internal note: the people who need it are
	     the ones deciding whether to run this and whether to mint an agent a
	     Key. An entry that stops being true is worse than one that never
	     existed, so a change to ADR 0031-0034 is a change to both copies, and
	     `tests/defences.rs` fails if either loses an item the other kept. -->
	<Section heading={m.settings_security()}>
		<ul class="space-y-3 text-body text-ink-2">
			<li>{m.settings_operator_boundary()}</li>
			<li>{m.settings_hand_unverified()}</li>
			<li>{m.settings_share_no_unsay()}</li>
			<li>{m.settings_stranger_waits()}</li>
			<li>{m.settings_stolen_phone()}</li>
			<li>{m.settings_name_held_off()}</li>
			<li>{m.settings_agent_reads()}</li>
			<li>{m.settings_unaudited()}</li>
		</ul>
	</Section>
</Screen>

{#if leavingKitchen}
	{@const asked = leavingKitchen}
	{@const called = kitchenName(asked.kitchen)}
	{@const others = asked.kitchen.members.filter((member) => member.person_id !== me?.person_id)}
	<Confirm
		title={m.kitchen_leave_title({ kitchen: called })}
		consequence={m.kitchen_leave_said({ kitchen: called })}
		act={m.kitchen_leave_do({ kitchen: called })}
		cancelLabel={m.cookbook_stay()}
		busy={leaveBusy}
		failed={leaveFailed}
		run={leaveKitchen}
		cancel={() => (leavingKitchen = undefined)}
	>
		<!-- What each side keeps, counted by the Core's own dry run: a copy
		     of what they cooked of the other's, and nothing more. -->
		<ul>
			{#if others.length > 0}
				<li class="flex justify-between gap-3 border-b border-rule py-2 text-body text-ink">
					<span>
						{others.length === 1
							? m.kitchen_leave_they_keep_one({ name: others[0]!.name })
							: m.kitchen_leave_they_keep({ names: joinedNames(others.map((o) => o.name)) })}
					</span>
					<span class="text-right text-read text-ink-2"
						>{asked.theyKeep === 0
							? m.kitchen_leave_they_keep_what_none()
							: asked.theyKeep === 1
								? m.kitchen_leave_they_keep_what_one()
								: m.kitchen_leave_they_keep_what({ count: asked.theyKeep })}</span
					>
				</li>
			{/if}
			<li class="flex justify-between gap-3 border-b border-rule py-2 text-body text-ink">
				<span>{m.kitchen_leave_you_keep()}</span>
				<span class="text-right text-read text-ink-2"
					>{asked.youKeep === 0
						? m.kitchen_leave_you_keep_what_none()
						: asked.youKeep === 1
							? m.kitchen_leave_you_keep_what_one()
							: m.kitchen_leave_you_keep_what({ count: asked.youKeep })}</span
				>
			</li>
		</ul>
		<p class="mt-3 text-read text-ink-2">{m.kitchen_leave_nothing_else()}</p>
	</Confirm>
{/if}

{#if cookbookAsk && cookbook}
	{@const asked = cookbookAsk}
	{@const book = cookbook}
	{#if asked.kind === 'leave'}
		<Confirm
			title={m.cookbook_leave_title({ name: cookbookPlainName(book) })}
			consequence={m.cookbook_leave_said({ count: book.recipe_count })}
			act={m.cookbook_leave_do()}
			cancelLabel={m.cookbook_stay()}
			busy={cookbookBusy}
			failed={cookbookFailed}
			run={answerCookbookAsk}
			cancel={() => (cookbookAsk = undefined)}
		>
			<p class="text-read text-ink-2">{m.cookbook_leave_said_links()}</p>
		</Confirm>
	{:else}
		<Confirm
			title={m.cookbook_remove_title({ name: asked.name })}
			consequence={m.cookbook_remove_said({ name: asked.name, count: book.recipe_count })}
			act={m.cookbook_remove_do({ name: asked.name })}
			cancelLabel={m.cookbook_stay()}
			busy={cookbookBusy}
			failed={cookbookFailed}
			run={answerCookbookAsk}
			cancel={() => (cookbookAsk = undefined)}
		/>
	{/if}
{/if}
