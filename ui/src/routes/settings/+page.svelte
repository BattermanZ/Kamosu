<!--
	Settings: reached through the Kitchen's name, never from the tab bar.

	This is also where the signature transition is shown in the shell: the card
	bearing the Kitchen's name in the header and this screen's title carry the
	same `view-transition-name`, so the card grows into the page rather than the
	screen swapping (ADR 0012). Every recipe card that opens into its page later
	is this same pair, with a different name.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { getLocale, locales, setLocale, type Locale } from '$lib/paraglide/runtime';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';
	import Section from '$lib/shell/Section.svelte';
	import type {
		InstanceStatusOutput,
		ListSessionsOutput,
		ListAccessKeysOutput,
		ListKitchensOutput,
		GetReadingPreferencesOutput,
	} from '$lib/api/catalogue';

	const kamosu = useKamosu();

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
	 * interface locale above: the two are separate settings today and quietly
	 * overwriting one while changing the other would be a lie about what the
	 * button did.
	 */
	type Measures = GetReadingPreferencesOutput['reading_measures'];

	const measureNames: Record<Measures, () => string> = {
		us: () => m.measures_us(),
		metric: () => m.measures_metric(),
		as_written: () => m.measures_as_written(),
	};

	let preferences = $state<GetReadingPreferencesOutput | undefined>(undefined);

	async function chooseMeasures(measures: Measures) {
		if (!preferences || preferences.reading_measures === measures) return;
		const previous = preferences;
		preferences = { ...previous, reading_measures: measures };
		try {
			await kamosu.setReadingPreferences({
				reading_language: previous.reading_language,
				reading_measures: measures,
			});
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			preferences = previous;
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
			const [sessionsAnswer, keysAnswer, preferencesAnswer] = await Promise.all([
				kamosu.listSessions(),
				kamosu.listAccessKeys(),
				kamosu.getReadingPreferences(),
			]);
			sessions = sessionsAnswer.sessions.filter((session) => !session.revoked);
			accessKeys = keysAnswer.access_keys.filter((key) => !key.revoked);
			preferences = preferencesAnswer;
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

	async function endSession(sessionId: string) {
		await kamosu.revokeSession({ session_id: sessionId });
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
</script>

<Screen title={m.settings_title()} expandsFrom="kitchen">
	<Section heading={m.settings_language()}>
		<!-- Paraglide compiles every phrase to a function, so this list can only
		     offer languages that were actually compiled. -->
		<ul class="flex flex-wrap gap-2">
			{#each locales as locale (locale)}
				<li>
					<button
						type="button"
						aria-pressed={getLocale() === locale}
						onclick={() => setLocale(locale)}
						class="min-h-12 rounded-sm border px-4 text-body
							{getLocale() === locale
							? 'border-accent bg-accent text-on-accent'
							: 'border-rule bg-card text-ink'}"
					>
						{names[locale]()}
					</button>
				</li>
			{/each}
		</ul>
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
					{#each sessions as session (session.id)}
						<li
							class="flex items-center justify-between gap-3 rounded-sm border border-rule bg-card px-3 py-2"
						>
							<div>
								<p class="text-body text-ink">{session.name}</p>
								<p class="text-read text-ink-2">{whenLastUsed(session.last_used_at)}</p>
							</div>
							<button
								type="button"
								class="shrink-0 text-label text-accent underline"
								onclick={() => endSession(session.id)}
							>
								{m.access_end()}
							</button>
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

		<Section heading={m.settings_kitchens()}>
			{#if kitchensError}
				<p class="mb-4 text-body text-accent" role="alert">{kitchensError}</p>
			{/if}

			<ul class="grid gap-4">
				{#each kitchens as kitchen (kitchen.id)}
					<li class="rounded-sm border border-rule bg-card p-3">
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
								class="min-h-10 flex-1 rounded-sm border border-rule bg-ground px-2 font-semibold text-ink"
								value={kitchen.name}
							/>
							{#if kitchen.is_home}
								<span class="shrink-0 text-label text-ink-2 uppercase"
									>{m.kitchen_home_badge()}</span
								>
							{/if}
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
							<label class="flex-1 text-label text-ink-2" for={`nickname-${kitchen.id}`}>
								{m.kitchen_nickname_label()}
							</label>
							<input
								id={`nickname-${kitchen.id}`}
								name="nickname"
								class="min-h-10 flex-1 rounded-sm border border-rule bg-ground px-2 text-body text-ink"
								placeholder={m.kitchen_nickname_placeholder()}
								value={kitchen.nickname ?? ''}
							/>
							<button class="shrink-0 text-label text-accent underline" type="submit">
								{m.kitchen_save()}
							</button>
						</form>

						<h3 class="mt-3 mb-1 text-label text-ink-2 uppercase">{m.kitchen_members()}</h3>
						<ul class="grid gap-1">
							{#each kitchen.members as member (member.person_id)}
								<li class="flex items-center justify-between gap-3 text-body text-ink">
									<span>{member.name}</span>
									<button
										type="button"
										class="shrink-0 text-label text-accent underline"
										onclick={() => removeMember(kitchen.id, member.person_id)}
									>
										{m.kitchen_remove()}
									</button>
								</li>
							{/each}
						</ul>

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
	{/if}

	<Section heading={m.settings_security()}>
		<p class="text-body text-ink-2">{m.settings_operator_boundary()}</p>
	</Section>
</Screen>
