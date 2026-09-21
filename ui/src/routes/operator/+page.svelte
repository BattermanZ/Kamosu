<!--
	The Operator's screen (#103).

	Kamosu had eighteen Operator-only Operations and a screen for one of them.
	Everything else — inviting the person you live with, taking a Backup, joining
	two Foods that are one thing — was reachable only through `curl`, an agent at
	the MCP door, or the terminal. This is the door.

	**Two rooms**, chosen 21 September 2026 from three designs drawn at a phone
	viewport. The Worklist holds what accumulates and wants working through; the
	Instance holds what merely sits there. The ticket raised the doubt itself —
	Merge Suggestions is evidence that piles up, which is a different rhythm from
	a settings toggle — and this takes it at its word rather than wedging a
	worklist between Backups and an address.

	**ADR 0007 draws the line this screen must not cross.** The Operator
	administers and does not read: no view-as-user, no support mode, no window
	onto another Person's recipes or Attempts. That is kept by `list_accounts`
	answering no question about a recipe, an Attempt or a Kitchen — the boundary
	lives in the Core, not in this screen's restraint — and it is said out loud
	here rather than merely observed, because a courtesy nobody mentions reads as
	a wall that is not there.

	**Nothing here is visible to a Person who is not an Operator.** The screen
	asks `list_accounts` and, refused, shows what a room that is not here shows
	(ADR 0040). It does not render an explanation of what it would have held.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { waitForJob } from '$lib/api/job';
	import Screen from '$lib/shell/Screen.svelte';
	import Section from '$lib/shell/Section.svelte';
	import Confirm from '$lib/operator/Confirm.svelte';
	import { asksWhetherAdministering } from '$lib/operator/administering';
	import type {
		ListAccountsOutput,
		ListBackupsOutput,
		ListMergeSuggestionsOutput,
		ListFoodsOutput,
		PreviewFoodMergeOutput,
		SweepPhotographsOutput,
	} from '$lib/api/catalogue';

	const kamosu = useKamosu();

	/**
	 * A Backup copies the database and every Photograph, and reading a library's
	 * Ingredient Lines is inference over every recipe. Both are minutes on a
	 * small machine, so the wait is bounded by what suits the longest Job here
	 * rather than by the default that suits the shortest one in Kamosu.
	 */
	const LONG_ENOUGH = 30 * 60_000;

	type Account = ListAccountsOutput['accounts'][number];
	type Backup = ListBackupsOutput['backups'][number];
	type Suggestion = ListMergeSuggestionsOutput['suggestions'][number];
	type Food = ListFoodsOutput['foods'][number];

	/**
	 * Whether this Person administers the instance. `undefined` while nobody has
	 * asked yet — which is not the same as "no", and rendering the two alike
	 * would flash the empty room at an Operator on every visit.
	 */
	let mayAdminister = $state<boolean | undefined>(undefined);
	let room = $state<'worklist' | 'instance'>('worklist');

	let accounts = $state<Account[]>([]);
	let backups = $state<Backup[]>([]);
	let suggestions = $state<Suggestion[]>([]);
	let foods = $state<Food[]>([]);
	let address = $state<string | null>(null);

	/**
	 * How many unused Foods are listed at once. A library can carry hundreds,
	 * and a phone showing all of them is a scroll nobody works through. The
	 * line under the list says the whole figure rather than letting the cap
	 * pass for the total.
	 */
	const SHOWN = 20;

	/** A Food nothing points at is the only kind `delete_food` will take. */
	const unused = $derived(foods.filter((food) => food.reading_count === 0));

	async function loadAccounts() {
		const answer = await asksWhetherAdministering(kamosu);
		// Only a refusal means this room is not yours. Anything else is Kamosu
		// failing, and showing an Operator an empty room for it would be a lie
		// about why they cannot see their own instance.
		mayAdminister = answer.may !== false;
		if (answer.may === true) accounts = answer.accounts;
		if (answer.may === 'unknown') said = answer.failed;
	}

	/**
	 * Everything else this screen shows, asked only once the instance has agreed
	 * this Person may administer it. Each is caught on its own: a Backup
	 * directory that cannot be read is no reason to show no accounts.
	 */
	async function loadRest() {
		const quietly = async (run: () => Promise<void>) => {
			try {
				await run();
			} catch (error) {
				if (!(error instanceof OperationError)) throw error;
			}
		};
		await Promise.all([
			quietly(async () => {
				backups = (await kamosu.listBackups({})).backups;
			}),
			quietly(async () => {
				suggestions = (await kamosu.listMergeSuggestions({})).suggestions;
			}),
			quietly(async () => {
				foods = (await kamosu.listFoods({})).foods;
			}),
			quietly(async () => {
				address = (await kamosu.getPublicAddress({})).public_address;
			}),
		]);
	}

	$effect(() => {
		loadAccounts();
	});

	$effect(() => {
		if (mayAdminister) loadRest();
	});

	// ── Saying what went wrong ────────────────────────────────────────────────
	//
	// A refusal is shown in the words it arrived in. The Core's refusals are
	// written to be read by a person — "'Aurélien' is the only Operator this
	// instance has…" — so paraphrasing one here would lose the reason and keep
	// only the fact.
	let said = $state<string | undefined>(undefined);
	let working = $state(false);

	async function act(run: () => Promise<void>, after?: () => Promise<void>) {
		working = true;
		said = undefined;
		try {
			await run();
			await after?.();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			said = error.message;
		} finally {
			working = false;
		}
	}

	// ── What is being confirmed ───────────────────────────────────────────────
	//
	// One slot, because only one sheet is ever open. Its shape is whatever the
	// act needs; `Confirm` reads the parts it was given.
	type Asking =
		| { kind: 'delete-account'; who: string }
		| { kind: 'disable-account'; who: string }
		| { kind: 'delete-food'; food: Food }
		| { kind: 'sweep' }
		| { kind: 'merge'; preview: PreviewFoodMergeOutput };

	let asking = $state<Asking | undefined>(undefined);

	function close() {
		asking = undefined;
		said = undefined;
	}

	// ── People ────────────────────────────────────────────────────────────────

	let invite = $state<string | undefined>(undefined);
	let recovery = $state<{ who: string; link: string } | undefined>(undefined);
	let inviteAsOperator = $state(false);

	const mintInvite = () =>
		act(async () => {
			invite = (await kamosu.mintInvite({ is_operator: inviteAsOperator })).link;
			inviteAsOperator = false;
		});

	const mintRecovery = (who: string) =>
		act(async () => {
			recovery = { who, link: (await kamosu.mintRecoveryLink({ name: who })).link };
		});

	const setOperator = (who: string, is_operator: boolean) =>
		act(
			async () => {
				await kamosu.setOperator({ name: who, is_operator });
			},
			() => loadAccounts(),
		);

	const endAccount = (who: string, deleted: boolean) =>
		act(
			async () => {
				if (deleted) await kamosu.deleteAccount({ name: who });
				else await kamosu.disableAccount({ name: who });
			},
			async () => {
				await loadAccounts();
				close();
			},
		);

	// ── Backups ───────────────────────────────────────────────────────────────

	let takingBackup = $state(false);
	let backupSaying = $state('');

	async function takeBackup() {
		takingBackup = true;
		backupSaying = '';
		said = undefined;
		try {
			await waitForJob(kamosu, (await kamosu.takeBackup({})).job_id, {
				giveUpAfter: LONG_ENOUGH,
				whileWaiting: (job) => {
					if (job.progress.message) backupSaying = job.progress.message;
				},
			});
			backups = (await kamosu.listBackups({})).backups;
		} catch (error) {
			// A Job that failed arrives as a plain Error from waitForJob; a
			// refused Operation as an OperationError. Both are shown as what
			// they say.
			if (!(error instanceof Error)) throw error;
			said = error.message;
		} finally {
			takingBackup = false;
			backupSaying = '';
		}
	}

	/** Megabytes, to one place. An archive is never small enough for bytes. */
	const weight = (bytes: number) => `${(bytes / 1_000_000).toFixed(1)} MB`;

	const when = (at: string) => new Date(at).toLocaleDateString();

	// ── Photographs, and reading the library's lines ──────────────────────────

	let swept = $state<SweepPhotographsOutput | undefined>(undefined);
	let linesRead = $state<number | undefined>(undefined);
	let readingLines = $state(false);

	const sweep = () =>
		act(
			async () => {
				swept = await kamosu.sweepPhotographs({});
			},
			async () => close(),
		);

	async function readLines() {
		readingLines = true;
		said = undefined;
		try {
			const done = await waitForJob(kamosu, (await kamosu.readIngredientLines({})).job_id, {
				giveUpAfter: LONG_ENOUGH,
			});
			linesRead = (done.result as { read?: number } | null)?.read ?? 0;
		} catch (error) {
			if (!(error instanceof Error)) throw error;
			said = error.message;
		} finally {
			readingLines = false;
		}
	}

	// ── Foods ─────────────────────────────────────────────────────────────────

	/** A Food's name in whatever language it has one, never its id. */
	const naming = (food: { name: string | null; id: string }) => food.name ?? food.id;

	/**
	 * What to call one Food of a pair when the pair is being told apart.
	 *
	 * Two different Foods can carry the same display name — ADR 0022 makes
	 * doubt produce a new Food rather than a merge, and a Merge gives the
	 * survivor every name both held, so a later suggestion naming that survivor
	 * reads as the same word twice. Found live on the dev corpus, where the
	 * worklist offered two identical *Keep liveflour* buttons and neither said
	 * which Food it kept. The language tells them apart where it can, the id
	 * where it cannot; the common case where the names already differ is
	 * untouched.
	 */
	function telling(food: Food, other: Food): string {
		const name = naming(food);
		if (name !== naming(other)) return name;
		// The language only tells them apart when the two differ in it. On the
		// dev corpus both were English, so language alone still printed the
		// same label twice — the id always differs, and is the fallback.
		if (food.language && food.language !== other.language) return `${name} (${food.language})`;
		return `${name} (${food.id.slice(2, 8)})`;
	}

	const askMerge = (survivor: Food, absorbed: Food) =>
		act(async () => {
			asking = {
				kind: 'merge',
				preview: await kamosu.previewFoodMerge({
					survivor_food_id: survivor.id,
					absorbed_food_id: absorbed.id,
				}),
			};
		});

	const doMerge = (preview: PreviewFoodMergeOutput) =>
		act(
			async () => {
				await kamosu.mergeFood({
					survivor_food_id: preview.survivor.id,
					absorbed_food_id: preview.absorbed.id,
					// The figure the preview announced, said back. A Merge that does
					// not match it is refused — that is the safety net, not a
					// formality, so it is sent from the preview and never recomputed.
					ingredient_lines: preview.ingredient_lines,
				});
			},
			async () => {
				await loadRest();
				close();
			},
		);

	const doDeleteFood = (food: Food) =>
		act(
			async () => {
				await kamosu.deleteFood({ food_id: food.id });
			},
			async () => {
				await loadRest();
				close();
			},
		);

	/** Why Kamosu thinks two Foods are one thing, in words rather than a code. */
	const because = (reason: Suggestion['reason']) =>
		reason === 'arrived_as_one' ? m.operator_merge_arrived() : m.operator_merge_typed();

	// ── The public address ────────────────────────────────────────────────────
	//
	// Changing one already stored, never setting the first. The first is asked
	// at the first Share Link and stays there (ADR 0029); moving it here would
	// put a question about domains in front of somebody who only wanted to send
	// a recipe to their sister.

	let typedAddress = $state('');
	let addressSaved = $state(false);

	const saveAddress = () =>
		act(async () => {
			address = (await kamosu.setPublicAddress({ public_address: typedAddress })).public_address;
			typedAddress = '';
			addressSaved = true;
		});
</script>

<Screen title={m.operator_title()} blurb={m.operator_blurb()}>
	{#if mayAdminister === undefined}
		<p class="text-body text-ink-2">{m.loading()}</p>
	{:else if !mayAdminister}
		<!-- ADR 0040: a room that is not yours answers as a room that is not here. -->
		<p class="text-body text-ink-2">{m.operator_not_here()}</p>
	{:else}
		<div class="flex border-b border-rule" role="tablist">
			{#each [['worklist', m.operator_worklist()] as const, ['instance', m.operator_instance()] as const] as [which, label] (which)}
				<button
					type="button"
					role="tab"
					aria-selected={room === which}
					aria-controls="operator-room"
					id="operator-tab-{which}"
					onclick={() => (room = which)}
					class="-mb-px min-h-12 flex-1 border-b-2 text-body font-medium {room === which
						? 'border-accent text-accent'
						: 'border-transparent text-ink-2'}"
				>
					{label}
				</button>
			{/each}
		</div>

		{#if said && !asking}
			<p class="mt-4 text-body text-support" role="alert">{said}</p>
		{/if}

		<div role="tabpanel" id="operator-room" aria-labelledby="operator-tab-{room}">
			{#if room === 'worklist'}
				<Section heading={m.operator_merge_heading()}>
					{#if suggestions.length === 0}
						<div class="rounded-sm bg-ground-2 p-6 text-center">
							<p class="font-display text-list-title font-semibold text-ink">
								{m.operator_merge_empty_title()}
							</p>
							<p class="mt-1 text-body text-ink-2">{m.operator_merge_empty()}</p>
						</div>
						<p class="mt-2 text-read text-ink-2">{m.operator_merge_never_itself()}</p>
					{:else}
						<ul class="overflow-hidden rounded-sm border border-rule bg-card">
							{#each suggestions as suggestion (suggestion.foods[0].id + suggestion.foods[1].id)}
								<li class="border-b border-rule p-3 last:border-b-0">
									<p class="text-body font-medium text-ink">
										{telling(suggestion.foods[0], suggestion.foods[1])} ·
										{telling(suggestion.foods[1], suggestion.foods[0])}
									</p>
									<p class="mt-1 text-read text-ink-2">{because(suggestion.reason)}</p>
									<div class="mt-2 flex flex-wrap gap-2">
										{#each suggestion.foods as keep, index (keep.id)}
											<button
												type="button"
												disabled={working}
												onclick={() => askMerge(keep, suggestion.foods[index === 0 ? 1 : 0])}
												class="min-h-12 rounded-sm border border-accent px-3 text-read font-medium
											text-accent disabled:opacity-60"
											>
												{m.operator_merge_keep({
													food: telling(keep, suggestion.foods[index === 0 ? 1 : 0]),
												})}
											</button>
										{/each}
									</div>
								</li>
							{/each}
						</ul>
					{/if}
				</Section>

				<Section heading={m.operator_unused_heading()}>
					{#if unused.length === 0}
						<p class="text-body text-ink-2">{m.operator_unused_empty()}</p>
					{:else}
						<ul class="overflow-hidden rounded-sm border border-rule bg-card">
							{#each unused.slice(0, SHOWN) as food (food.id)}
								<li class="flex items-center gap-3 border-b border-rule p-3 last:border-b-0">
									<span class="min-w-0 flex-1">
										<span class="block truncate text-body text-ink">{naming(food)}</span>
										<span class="block text-read text-ink-2">{m.operator_unused_none()}</span>
									</span>
									<button
										type="button"
										disabled={working}
										onclick={() => (asking = { kind: 'delete-food', food })}
										class="min-h-12 shrink-0 rounded-sm border border-support px-3 text-read
									font-medium text-support disabled:opacity-60"
									>
										{m.operator_delete()}
									</button>
								</li>
							{/each}
						</ul>
						<p class="mt-2 text-read text-ink-2">
							{unused.length > SHOWN
								? m.operator_unused_capped({ shown: SHOWN, count: unused.length })
								: m.operator_unused_kept({ count: unused.length })}
						</p>
					{/if}
				</Section>

				<Section heading={m.operator_photographs_heading()}>
					<div class="rounded-sm border border-rule bg-card p-3">
						<p class="text-body text-ink">
							{#if swept}
								{m.operator_photographs_swept({ swept: swept.swept, kept: swept.referenced })}
							{:else}
								{m.operator_photographs_blurb()}
							{/if}
						</p>
						<button
							type="button"
							disabled={working}
							onclick={() => (asking = { kind: 'sweep' })}
							class="mt-3 min-h-12 w-full rounded-sm border border-accent px-4 text-body font-medium
						text-accent disabled:opacity-60"
						>
							{m.operator_photographs_sweep()}
						</button>
					</div>
				</Section>

				<Section heading={m.operator_lines_heading()}>
					<div class="rounded-sm border border-rule bg-card p-3">
						<p class="text-body text-ink-2">
							{#if linesRead !== undefined}
								{m.operator_lines_read({ count: linesRead })}
							{:else}
								{m.operator_lines_blurb()}
							{/if}
						</p>
						<button
							type="button"
							disabled={readingLines}
							onclick={readLines}
							class="mt-3 min-h-12 w-full rounded-sm border border-accent px-4 text-body font-medium
						text-accent disabled:opacity-60"
						>
							{readingLines ? m.operator_working() : m.operator_lines_run()}
						</button>
					</div>
				</Section>
			{:else}
				<Section heading={m.operator_people_heading()}>
					{#if invite}
						<div class="mb-4 rounded-sm border border-accent bg-card p-3" role="alert">
							<p class="text-body font-semibold text-ink">{m.operator_invite_once()}</p>
							<code class="mt-2 block overflow-x-auto rounded-sm bg-ground p-2 text-read"
								>{invite}</code
							>
							<button
								type="button"
								onclick={() => (invite = undefined)}
								class="mt-3 min-h-12 w-full rounded-sm border border-accent px-4 text-body font-medium
							text-accent"
							>
								{m.operator_invite_done()}
							</button>
						</div>
					{/if}

					{#if recovery}
						<div class="mb-4 rounded-sm border border-accent bg-card p-3" role="alert">
							<p class="text-body font-semibold text-ink">
								{m.operator_recovery_once({ who: recovery.who })}
							</p>
							<code class="mt-2 block overflow-x-auto rounded-sm bg-ground p-2 text-read"
								>{recovery.link}</code
							>
							<button
								type="button"
								onclick={() => (recovery = undefined)}
								class="mt-3 min-h-12 w-full rounded-sm border border-accent px-4 text-body font-medium
							text-accent"
							>
								{m.operator_invite_done()}
							</button>
						</div>
					{/if}

					<ul class="overflow-hidden rounded-sm border border-rule bg-card">
						{#each accounts as account (account.name)}
							<li class="border-b border-rule p-3 last:border-b-0">
								<div class="flex items-center gap-3">
									<span class="min-w-0 flex-1">
										<span class="block truncate text-body font-medium text-ink">{account.name}</span
										>
										<span class="block text-read text-ink-2">
											{account.is_operator ? m.operator_role_operator() : m.operator_role_person()}
											{#if account.disabled}· {m.operator_role_disabled()}{/if}
										</span>
									</span>
									{#if account.is_you}
										<span
											class="shrink-0 rounded-sm bg-accent px-2 py-1 text-label text-on-accent uppercase"
											>{m.operator_you()}</span
										>
									{/if}
								</div>
								<div class="mt-2 flex flex-wrap gap-2">
									<button
										type="button"
										disabled={working}
										onclick={() => setOperator(account.name, !account.is_operator)}
										class="min-h-12 rounded-sm border border-accent px-3 text-read font-medium
									text-accent disabled:opacity-60"
									>
										{account.is_operator ? m.operator_stand_down() : m.operator_make_operator()}
									</button>
									<button
										type="button"
										disabled={working}
										onclick={() => mintRecovery(account.name)}
										class="min-h-12 rounded-sm border border-rule px-3 text-read font-medium text-ink
									disabled:opacity-60"
									>
										{m.operator_recovery()}
									</button>
									{#if !account.disabled}
										<button
											type="button"
											disabled={working}
											onclick={() => (asking = { kind: 'disable-account', who: account.name })}
											class="min-h-12 rounded-sm border border-rule px-3 text-read font-medium text-ink
										disabled:opacity-60"
										>
											{m.operator_disable()}
										</button>
									{/if}
									<button
										type="button"
										disabled={working}
										onclick={() => (asking = { kind: 'delete-account', who: account.name })}
										class="min-h-12 rounded-sm border border-support px-3 text-read font-medium
									text-support disabled:opacity-60"
									>
										{m.operator_delete()}
									</button>
								</div>
							</li>
						{/each}
					</ul>

					<label class="mt-4 flex min-h-12 items-center gap-3">
						<input type="checkbox" bind:checked={inviteAsOperator} class="h-6 w-6" />
						<span class="text-body text-ink">{m.operator_invite_as_operator()}</span>
					</label>
					<button
						type="button"
						disabled={working}
						onclick={mintInvite}
						class="mt-2 min-h-12 w-full rounded-sm border border-accent bg-accent px-4 text-body
					font-medium text-on-accent disabled:opacity-60"
					>
						{m.operator_invite()}
					</button>
					<p class="mt-2 text-read text-ink-2">{m.settings_operator_boundary()}</p>
				</Section>

				<Section heading={m.operator_backups_heading()}>
					<ul class="overflow-hidden rounded-sm border border-rule bg-card">
						{#each backups as backup (backup.name)}
							<li class="flex items-center gap-3 border-b border-rule p-3 last:border-b-0">
								<span class="min-w-0 flex-1">
									<span class="block text-body text-ink">{when(backup.taken_at)}</span>
									<span class="block text-read text-ink-2"
										>{backup.slot} · {weight(backup.size_bytes)}</span
									>
								</span>
								<a
									href="/api/backups/{backup.name}"
									class="min-h-12 shrink-0 content-center rounded-sm border border-accent px-3 text-read
								font-medium text-accent"
								>
									{m.operator_backup_get()}
								</a>
							</li>
						{:else}
							<li class="p-3 text-body text-ink-2">{m.operator_backups_empty()}</li>
						{/each}
					</ul>
					<button
						type="button"
						disabled={takingBackup}
						onclick={takeBackup}
						class="mt-3 min-h-12 w-full rounded-sm border border-accent px-4 text-body font-medium
					text-accent disabled:opacity-60"
					>
						{takingBackup ? backupSaying || m.operator_working() : m.operator_backup_take()}
					</button>
					<p class="mt-2 text-read text-ink-2">{m.operator_backups_blurb()}</p>
				</Section>

				<Section heading={m.operator_address_heading()}>
					<div class="rounded-sm border border-rule bg-card p-3">
						<p class="text-read text-ink-2">{m.operator_address_current()}</p>
						<p class="text-body text-ink">{address ?? m.operator_address_none()}</p>
						<label class="mt-3 block">
							<span class="sr-only">{m.operator_address_heading()}</span>
							<input
								type="url"
								bind:value={typedAddress}
								placeholder="https://kamosu.example"
								class="min-h-12 w-full rounded-sm border border-rule bg-ground px-3 text-body"
							/>
						</label>
						<button
							type="button"
							disabled={working || typedAddress.trim() === ''}
							onclick={saveAddress}
							class="mt-2 min-h-12 w-full rounded-sm border border-accent px-4 text-body font-medium
						text-accent disabled:opacity-60"
						>
							{m.operator_address_save()}
						</button>
						{#if addressSaved}
							<p class="mt-2 text-body text-ink" role="status">{m.operator_address_saved()}</p>
						{/if}
					</div>
					<!--
					The honest sentence, corrected on this ticket. Kamosu keeps only a
					Share Link's *hash* and looks a visitor up by that alone, so a link
					already sent is a string in somebody else's phone that nothing here
					can reach, and Kamosu cannot reissue it either — it discarded the
					secret at minting. Changing this fixes the future, not the past.
				-->
					<p class="mt-2 text-read text-ink-2">{m.operator_address_warning()}</p>
				</Section>
			{/if}
		</div>
	{/if}
</Screen>

{#if asking}
	{#if asking.kind === 'merge'}
		{@const preview = asking.preview}
		<Confirm
			title={m.operator_merge_title({
				absorbed: naming(preview.absorbed),
				survivor: naming(preview.survivor),
			})}
			count={preview.ingredient_lines}
			counting={m.operator_merge_counting({
				absorbed: naming(preview.absorbed),
				survivor: naming(preview.survivor),
			})}
			consequence={m.operator_merge_consequence({
				absorbed: naming(preview.absorbed),
				survivor: naming(preview.survivor),
			})}
			act={m.operator_merge_do()}
			busy={working}
			failed={said}
			run={() => doMerge(preview)}
			cancel={close}
		>
			{#if preview.cup_weight_conflict}
				<p class="text-read text-ink-2">{m.operator_merge_cup_weight()}</p>
			{/if}
		</Confirm>
	{:else if asking.kind === 'sweep'}
		<Confirm
			title={m.operator_sweep_title()}
			consequence={m.operator_sweep_consequence()}
			act={m.operator_photographs_sweep()}
			busy={working}
			failed={said}
			run={sweep}
			cancel={close}
		/>
	{:else if asking.kind === 'delete-food'}
		{@const food = asking.food}
		<Confirm
			title={m.operator_food_title({ food: naming(food) })}
			consequence={m.operator_food_consequence()}
			act={m.operator_delete()}
			busy={working}
			failed={said}
			run={() => doDeleteFood(food)}
			cancel={close}
		/>
	{:else if asking.kind === 'delete-account'}
		{@const who = asking.who}
		<Confirm
			title={m.operator_account_delete_title({ who })}
			consequence={m.operator_account_delete_consequence({ who })}
			act={m.operator_account_delete_do()}
			instead={{ label: m.operator_disable(), run: () => endAccount(who, false) }}
			busy={working}
			failed={said}
			run={() => endAccount(who, true)}
			cancel={close}
		/>
	{:else}
		{@const who = asking.who}
		<Confirm
			title={m.operator_account_disable_title({ who })}
			consequence={m.operator_account_disable_consequence({ who })}
			act={m.operator_disable()}
			busy={working}
			failed={said}
			run={() => endAccount(who, false)}
			cancel={close}
		/>
	{/if}
{/if}
