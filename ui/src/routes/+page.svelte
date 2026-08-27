<script lang="ts">
	import { page } from '$app/state';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { useAuth } from '$lib/auth';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';

	const kamosu = useKamosu();
	const auth = useAuth();
	let setupComplete = $state<boolean | undefined>(undefined);
	let name = $state('');
	let password = $state('');
	let failed = $state<string | undefined>(undefined);
	let busy = $state(false);
	const invite = $derived(page.url.pathname.startsWith('/invite/') ? page.url.pathname : undefined);
	const recovery = $derived(page.url.pathname.startsWith('/recover/') ? page.url.pathname : undefined);
	const mode = $derived(invite ? 'invite' : recovery ? 'recover' : setupComplete ? 'login' : 'first-person');

	$effect(() => {
		let current = true;
		kamosu.instanceStatus().then((status) => {
			if (current) setupComplete = status.setup_complete;
		}).catch((error: unknown) => {
			if (current) failed = error instanceof Error ? error.message : m.account_failed();
		});
		return () => {
			current = false;
		};
	});

	async function submit() {
		busy = true;
		failed = undefined;
		try {
			await auth.authenticate(mode, {
				name: recovery ? undefined : name,
				password,
				session_name: 'this browser',
				link: invite ?? recovery
			});
			if (mode === 'first-person' || mode === 'invite') setupComplete = true;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		} finally {
			busy = false;
			password = '';
		}
	}
</script>

{#if setupComplete === undefined}
	<Screen title="Loading Kamosu…" />
{:else}
	<Screen
		title={invite ? m.account_invite_title() : recovery ? m.account_recover_title() : setupComplete ? m.account_login_title() : m.account_setup_title()}
		blurb={invite ? m.account_invite_blurb() : recovery ? m.account_recover_blurb() : setupComplete ? undefined : m.account_setup_blurb()}
	>
		<form class="grid gap-4" onsubmit={(event) => { event.preventDefault(); submit(); }}>
			{#if !recovery}
				<label class="grid gap-1 text-body text-ink">
					{m.account_name()}
					<input class="min-h-12 rounded-sm border border-rule bg-card px-3" bind:value={name} required autocomplete="username" />
				</label>
			{/if}
			<label class="grid gap-1 text-body text-ink">
				{m.account_password()}
				<input class="min-h-12 rounded-sm border border-rule bg-card px-3" type="password" bind:value={password} required autocomplete={mode === 'login' ? 'current-password' : 'new-password'} />
			</label>
			{#if failed}
				<p class="text-body text-accent" role="alert">{failed}</p>
			{/if}
			<button class="min-h-12 rounded-sm bg-accent px-4 font-semibold text-on-accent disabled:opacity-60" disabled={busy}>
				{invite ? m.account_invite_submit() : recovery ? m.account_recover_submit() : setupComplete ? m.account_login() : m.account_create()}
			</button>
		</form>
	</Screen>
{/if}
