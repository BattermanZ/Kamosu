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
		ListAccessKeysOutput
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
		es: () => m.language_es()
	};

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
			const [sessionsAnswer, keysAnswer] = await Promise.all([
				kamosu.listSessions(),
				kamosu.listAccessKeys()
			]);
			sessions = sessionsAnswer.sessions.filter((session) => !session.revoked);
			accessKeys = keysAnswer.access_keys.filter((key) => !key.revoked);
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
		return lastUsedAt ? m.access_last_used({ when: new Date(lastUsedAt).toLocaleString() }) : m.access_never_used();
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
					<code class="mt-2 block overflow-x-auto rounded-sm bg-ground p-2 text-read">{mintedSecret}</code>
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
										<span class="ml-1 text-label uppercase text-ink-2"
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
	{/if}
</Screen>
