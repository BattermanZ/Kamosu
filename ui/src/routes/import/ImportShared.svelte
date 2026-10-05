<!--
	What a Share Link page's *Import this recipe* opens (#170), on the instance
	that shared it or, sent on from there, on the reader's own.

	Aurélien's three choices of 26 September 2026, recorded on #170:

	1. **Signed in, a short confirm screen before anything happens.** It shows
	   the recipe, says it goes into your Cookbook with every Version and where
	   it came from, and that its writer stays named. *Import it* or *Not now*.
	   A Person has one Cookbook, so it names it and never asks which.
	2. **Signed out, one screen asking where you keep your recipes**: log in
	   here, or give your own Kamosu's address and be sent there with the link,
	   where this same page opens. Under both, the recipe file, for a reader
	   with no Kamosu at all.
	3. **Already held, the confirm screen says so.** Nothing new: *Open your
	   copy* instead. Newer Versions since: how many, and importing brings only
	   those. The Share Link page itself never asks who is looking.

	**It asks before it knows.** `preview_shared_recipe` is a Job that needs a
	Person, so a refusal for want of a Credential *is* the answer that nobody
	is signed in, as on `/` (#64). What it answers is the confirm screen, and
	the file it read is staged, so *Import it* is `import_bundle` with that
	upload: the recipe is fetched once, and what lands is what was shown.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { StillRunning, waitForJob } from '$lib/api/job';
	import { receiveStaged } from '$lib/receive';
	import { getLocale } from '$lib/paraglide/runtime';
	import Screen from '$lib/shell/Screen.svelte';
	import { heardWhetherSignedIn } from '$lib/shell/signing-in.svelte';
	import type { PreviewSharedRecipeOutput } from '$lib/api/catalogue';
	import Account from '../Account.svelte';

	interface Props {
		/** The `link` the Share Link page put in this page's address. */
		link: string | null;
		/**
		 * Where the reader is, which a link written without a host is read
		 * against. Handed down, like `leave`, so a test can say it.
		 */
		origin: string;
		/** Go to another Kamosu: a whole page load, never this app's router. */
		leave: (address: string) => void;
	}

	let { link, origin, leave }: Props = $props();

	const kamosu = useKamosu();

	/**
	 * The Share Link as a whole address, and where its recipe file is. `null`
	 * for a page opened without one, or with something that is no Share Link's
	 * shape: there is then nothing to ask about.
	 */
	const shared = $derived.by(() => {
		if (!link) return null;
		let address: URL;
		try {
			address = new URL(link, origin);
		} catch {
			return null;
		}
		const token = /^\/s\/([^/]+)/.exec(address.pathname)?.[1];
		if (!token || !['http:', 'https:'].includes(address.protocol)) return null;
		return {
			url: address.href,
			token,
			local: address.origin === origin,
			file: `${address.origin}/s/${token}/bundle`,
		};
	});

	type Step =
		| { is: 'reading' }
		| { is: 'where'; title?: string; sharer?: string }
		| { is: 'ready'; preview: PreviewSharedRecipeOutput }
		| { is: 'importing'; preview: PreviewSharedRecipeOutput }
		/** The import outlasted the wait (#117): not failed, still going. */
		| { is: 'still-importing' }
		| { is: 'failed'; reason: string };

	let step = $state<Step>({ is: 'reading' });
	/** Bumped by signing in, and by *Try again*, to ask once more. */
	let asked = $state(0);

	$effect(() => {
		void asked;
		const here = shared;
		if (!here) {
			step = { is: 'failed', reason: m.import_shared_no_link() };
			return;
		}
		let current = true;
		step = { is: 'reading' };
		(async () => {
			try {
				const job = await kamosu.previewSharedRecipe({ url: here.url });
				// The shell draws its sidebar on this answer (#194).
				heardWhetherSignedIn(true);
				const finished = await waitForJob(kamosu, job.job_id);
				if (current) {
					step = { is: 'ready', preview: finished.result as PreviewSharedRecipeOutput };
				}
			} catch (error) {
				if (!(error instanceof Error)) throw error;
				if (!current) return;
				if (error instanceof OperationError && error.kind === 'unauthorized') {
					heardWhetherSignedIn(false);
					step = { is: 'where' };
					// Named where it can be, which is a link this instance shared:
					// its page is Public, so a stranger may read its title. A far
					// Kamosu's is somebody else's address, and not asked here.
					if (!here.local) return;
					const read = await kamosu.readSharedRecipe({ token: here.token }).catch(() => null);
					const title = read?.recipe?.content.title;
					if (current && title) {
						step = { is: 'where', title, sharer: read?.shared_by ?? undefined };
					}
					return;
				}
				step = { is: 'failed', reason: said(error) };
			}
		})();
		return () => {
			current = false;
		};
	});

	/** What a preview that did not answer says, in place of an answer. */
	function said(error: Error): string {
		return error instanceof StillRunning ? m.import_shared_still_reading() : error.message;
	}

	async function importIt(preview: PreviewSharedRecipeOutput) {
		step = { is: 'importing', preview };
		try {
			const reason = await receiveStaged(kamosu, preview.upload_id, {
				from: 'link',
				writer: preview.written_by ?? undefined,
				added: preview.held?.newer || undefined,
			});
			if (reason !== undefined) step = { is: 'failed', reason };
		} catch (error) {
			if (!(error instanceof Error)) throw error;
			// An import that outlasts the wait is still going, and lands by
			// itself: asking again would only offer a second one (#117).
			step =
				error instanceof StillRunning
					? { is: 'still-importing' }
					: { is: 'failed', reason: error.message };
		}
	}

	/** Back to the shared page this came from, or Home when there is none. */
	function notNow() {
		if (history.length > 1) history.back();
		else void goto('/');
	}

	let address = $state('');
	let addressProblem = $state<string | undefined>(undefined);

	/**
	 * Send the reader to their own Kamosu with the link in hand, where this
	 * page opens again. Only the address's origin is taken: a Kamosu lives at
	 * the root of its host (ADR 0028).
	 */
	function takeItThere(url: string) {
		addressProblem = undefined;
		// Written as a person writes an address, with or without its scheme. A
		// Kamosu at home on plain http says so by writing it.
		const typed = address.trim();
		let theirs: URL;
		try {
			theirs = new URL(/^https?:\/\//i.test(typed) ? typed : `https://${typed}`);
		} catch {
			addressProblem = m.import_shared_address_bad();
			return;
		}
		if (theirs.origin === origin) {
			addressProblem = m.import_shared_address_here();
			return;
		}
		leave(`${theirs.origin}/import?link=${encodeURIComponent(url)}`);
	}

	/**
	 * Which confirm screen a preview makes (Aurélien's choices 1 and 3): not
	 * held; held and written here, so nothing can be imported; held but gone a
	 * different way from the file, so importing would change nothing; held
	 * with nothing new; held with newer Versions to bring in.
	 */
	function answerOf(
		held: PreviewSharedRecipeOutput['held'],
	): 'new' | 'own' | 'diverged' | 'held' | 'newer' {
		if (!held) return 'new';
		if (!held.arrived) return 'own';
		if (held.diverged) return 'diverged';
		return held.newer > 0 ? 'newer' : 'held';
	}

	/** A date as the rest of Kamosu writes one, in the app's own Language. */
	const since = (when: string) =>
		new Date(when).toLocaleDateString(getLocale(), {
			day: 'numeric',
			month: 'long',
			year: 'numeric',
		});
</script>

{#snippet card(preview: PreviewSharedRecipeOutput)}
	{@const about = [
		preview.shared_by ? m.import_shared_by({ name: preview.shared_by }) : undefined,
		preview.source?.text ? m.import_shared_from({ source: preview.source.text }) : undefined,
	].filter(Boolean)}
	<div class="flex items-center gap-3 rounded-sm border border-rule bg-card p-3">
		{#if preview.photo}
			<img
				src={preview.photo}
				alt=""
				class="h-[var(--photo-thumb)] w-[var(--photo-thumb)] shrink-0 rounded-sm object-cover"
			/>
		{/if}
		<div class="min-w-0 flex-1">
			<p class="font-display text-line font-semibold">{preview.title}</p>
			{#if about.length > 0}
				<p class="mt-1 text-read text-ink-2">{about.join(' · ')}</p>
			{/if}
		</div>
	</div>
{/snippet}

{#snippet answer(preview: PreviewSharedRecipeOutput, busy: boolean)}
	{@const held = preview.held}
	{@const is = answerOf(held)}
	{@const name = preview.written_by ?? m.import_shared_their_writer()}
	<Screen
		title={is === 'new'
			? m.import_shared_title()
			: is === 'newer'
				? m.import_shared_newer()
				: m.import_shared_held()}
	>
		{@render card(preview)}
		{#if !held}
			<p class="mt-6 text-body">{m.import_shared_goes({ count: preview.versions })}</p>
			<p class="mt-3 text-read text-ink-2">{m.import_shared_writer({ name })}</p>
		{:else if is === 'own'}
			<p class="mt-6 text-body">{m.import_shared_held_own()}</p>
		{:else if is === 'diverged'}
			<p class="mt-6 text-body">
				{m.import_shared_diverged({ date: since(held.since), name })}
			</p>
		{:else if is === 'held'}
			<p class="mt-6 text-body">{m.import_shared_held_since({ date: since(held.since) })}</p>
		{:else if held.newer === 1}
			<p class="mt-6 text-body">
				{m.import_shared_newer_since_one({ date: since(held.since), name })}
			</p>
			<p class="mt-3 text-read text-ink-2">{m.import_shared_newer_adds_one()}</p>
		{:else}
			<p class="mt-6 text-body">
				{m.import_shared_newer_since({ date: since(held.since), name, count: held.newer })}
			</p>
			<p class="mt-3 text-read text-ink-2">{m.import_shared_newer_adds()}</p>
		{/if}
		{#if held && is !== 'newer'}
			<a
				href={`/recipes/${held.branch_id}`}
				class="mt-6 block min-h-12 rounded-sm bg-accent p-3 text-center font-semibold text-on-accent"
			>
				{m.import_shared_open()}
			</a>
		{:else}
			<button
				type="button"
				class="mt-6 block min-h-12 w-full rounded-sm bg-accent p-3 text-center font-semibold text-on-accent disabled:opacity-60"
				disabled={busy}
				onclick={() => importIt(preview)}
			>
				{!held
					? m.import_shared_import()
					: held.newer === 1
						? m.import_shared_import_newer_one()
						: m.import_shared_import_newer({ count: held.newer })}
			</button>
		{/if}
		<button
			type="button"
			class="mt-2 block min-h-12 w-full rounded-sm border border-rule p-3 text-center text-accent disabled:opacity-60"
			disabled={busy}
			onclick={notNow}
		>
			{m.import_shared_not_now()}
		</button>
	</Screen>
{/snippet}

{#snippet here()}
	<h2 class="mb-3 font-display text-label font-semibold text-accent uppercase">
		{m.import_shared_here()}
	</h2>
{/snippet}

{#snippet elsewhere()}
	{#if shared}
		{@const url = shared.url}
		<h2 class="mt-8 font-display text-label font-semibold text-accent uppercase">
			{m.import_shared_elsewhere()}
		</h2>
		<form
			class="mt-3 grid gap-3"
			onsubmit={(event) => {
				event.preventDefault();
				takeItThere(url);
			}}
		>
			<label class="grid gap-1 text-body text-ink">
				{m.import_shared_address()}
				<input
					class="min-h-12 w-full rounded-sm border border-rule bg-card px-3"
					bind:value={address}
					required
					inputmode="url"
					autocapitalize="off"
					autocomplete="url"
					spellcheck="false"
					placeholder={m.import_shared_address_example()}
				/>
			</label>
			{#if addressProblem}
				<p class="text-body text-accent" role="alert">{addressProblem}</p>
			{/if}
			<button class="min-h-12 rounded-sm border border-accent px-4 font-semibold text-accent">
				{m.import_shared_there()}
			</button>
		</form>
		<p class="mt-8 text-read text-ink-2">
			{m.import_shared_none()}
			<a href={shared.file} class="text-accent underline underline-offset-4">
				{m.import_shared_file()}
			</a>
		</p>
	{/if}
{/snippet}

{#if step.is === 'reading'}
	<Screen title={m.import_shared_reading()} />
{:else if step.is === 'where'}
	<Account
		onSignedIn={() => (asked += 1)}
		framing={{
			title: step.title
				? m.import_shared_where_named({ title: step.title })
				: m.import_shared_where(),
			blurb: step.sharer
				? m.import_shared_where_blurb_by({ name: step.sharer })
				: m.import_shared_where_blurb(),
			submit: m.import_shared_login(),
			before: here,
			after: elsewhere,
		}}
	/>
{:else if step.is === 'ready'}
	{@render answer(step.preview, false)}
{:else if step.is === 'importing'}
	{@render answer(step.preview, true)}
{:else if step.is === 'still-importing'}
	<Screen title={m.import_shared_title()}>
		<p class="text-body" role="status">
			{m.bring_in_still_going({ where: `${m.settings_title()} › ${m.settings_imports()}` })}
		</p>
	</Screen>
{:else}
	<Screen title={m.import_shared_failed()}>
		<p class="text-body" role="alert">{step.reason}</p>
		{#if shared}
			<button
				type="button"
				class="mt-6 min-h-12 w-full rounded-sm border border-rule px-4 text-accent"
				onclick={() => (asked += 1)}
			>
				{m.import_shared_try_again()}
			</button>
		{/if}
	</Screen>
{/if}
