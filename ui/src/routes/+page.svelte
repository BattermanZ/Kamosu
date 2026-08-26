<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';

	const kamosu = useKamosu();
	let setupComplete = $state<boolean | undefined>(undefined);
	let name = $state('');
	let password = $state('');
	let failed = $state(false);
	let busy = $state(false);

	$effect(() => {
		let current = true;
		kamosu.instanceStatus().then((status) => {
			if (current) setupComplete = status.setup_complete;
		});
		return () => {
			current = false;
		};
	});

	async function submit() {
		busy = true;
		failed = false;
		try {
			const response = await fetch(setupComplete ? '/auth/login' : '/auth/first-person', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ name, password, session_name: 'this browser' })
			});
			if (!response.ok) throw new OperationError('authentication', 'unauthorized', 'authentication failed');
			if (!setupComplete) setupComplete = true;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = true;
		} finally {
			busy = false;
			password = '';
		}
	}
</script>

<Screen
	title={setupComplete ? m.account_login_title() : m.account_setup_title()}
	blurb={setupComplete ? undefined : m.account_setup_blurb()}
>
	<form class="grid gap-4" onsubmit={(event) => { event.preventDefault(); submit(); }}>
		<label class="grid gap-1 text-body text-ink">
			{m.account_name()}
			<input class="min-h-12 rounded-sm border border-rule bg-card px-3" bind:value={name} required autocomplete="username" />
		</label>
		<label class="grid gap-1 text-body text-ink">
			{m.account_password()}
			<input class="min-h-12 rounded-sm border border-rule bg-card px-3" type="password" bind:value={password} required autocomplete={setupComplete ? 'current-password' : 'new-password'} />
		</label>
		{#if failed}
			<p class="text-body text-accent" role="alert">{m.account_failed()}</p>
		{/if}
		<button class="min-h-12 rounded-sm bg-accent px-4 font-semibold text-on-accent disabled:opacity-60" disabled={busy}>
			{setupComplete ? m.account_login() : m.account_create()}
		</button>
	</form>
</Screen>
