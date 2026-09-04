<!--
	Cooked: the cooking diary (#60).

	Your Attempts sorted by date rather than by recipe, which is the whole of
	why this screen exists — *what did I cook that week* is a question the shelf
	cannot answer however it is filtered, because the shelf is filed by dish.
	So the headings here are months, and a recipe cooked four times appears
	four times.

	**Cookings nobody ever finished are in it**, marked and never hidden — see
	the mark itself, below, for why there is exactly one mark and not two.

	Everything here is one person's own. `list_attempts` is scoped to its
	caller in the Core, so there is no filter on this screen to get wrong and
	nothing to say about whose diary it is: it is yours, and there is no other.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { getLocale } from '$lib/paraglide/runtime';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { ratingLabel } from '$lib/rating';
	import type { ListAttemptsOutput } from '$lib/api/catalogue';
	import Screen from '$lib/shell/Screen.svelte';
	import Empty from '$lib/shell/Empty.svelte';
	// PROTOTYPE — #58. THROWAWAY. Delete on merge, with $lib/prototype-58.
	import { page } from '$app/state';
	import Prototype58Promote from '$lib/prototype-58/PromoteInDiary.svelte';

	const kamosu = useKamosu();

	/** One line of the diary: an Attempt, and the recipe it was cooked from. */
	type Entry = ListAttemptsOutput['attempts'][number];

	let entries = $state<Entry[] | undefined>(undefined);
	let failed = $state(false);

	$effect(() => {
		let current = true;
		kamosu
			.listAttempts({})
			.then((diary) => {
				if (current) {
					entries = diary.attempts;
					failed = false;
				}
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				if (current) failed = true;
			});
		return () => {
			current = false;
		};
	});

	const day = (when: string) =>
		new Date(when).toLocaleDateString(getLocale(), {
			day: 'numeric',
			month: 'long',
		});

	/**
	 * The months, newest first, each holding its own cookings in the order the
	 * Core sent them. A month is the heading a diary wants: fine enough that
	 * *that week* is a glance rather than a scroll, coarse enough that a run of
	 * evenings does not become a run of headings.
	 */
	const months = $derived.by(() => {
		const grouped: { key: string; heading: string; of: Entry[] }[] = [];
		for (const entry of entries ?? []) {
			const when = new Date(entry.created_at);
			const key = `${when.getFullYear()}-${when.getMonth()}`;
			const last = grouped.at(-1);
			if (last?.key === key) {
				last.of.push(entry);
			} else {
				grouped.push({
					key,
					heading: when.toLocaleDateString(getLocale(), {
						month: 'long',
						year: 'numeric',
					}),
					of: [entry],
				});
			}
		}
		return grouped;
	});

	// --- opening one, correcting it, putting it away ----------------------
	//
	// An Attempt is freely editable and deletable by its cook, deliberately
	// unlike the recipe it was cooked from (ADR 0005): append-only is the price
	// of history that is shared and built upon, and a private diary is neither.

	/** Which entry has its panel open, by id — one at a time. */
	let opened = $state<string | null>(null);
	let draftNote = $state('');
	/**
	 * The verdict being chosen, which includes *none* — a rating is optional and
	 * taking one back off is an edit like any other. Typed off the field it
	 * edits rather than restated, so the day a fourth verdict is declared this
	 * offers it or fails the build.
	 */
	let draftRating = $state<Entry['rating']>(null);
	let saving = $state(false);
	let deleting = $state(false);
	/** The open entry, with delete asked for and not yet confirmed. */
	let confirming = $state(false);
	let writeFailed = $state<string | undefined>(undefined);

	function open(entry: Entry) {
		if (opened === entry.id) {
			opened = null;
			return;
		}
		opened = entry.id;
		draftNote = entry.note ?? '';
		draftRating = entry.rating;
		confirming = false;
		writeFailed = undefined;
	}

	/** Lay a saved change over the list in place. Nothing here needs refetching. */
	function layOver(id: Entry['id'], change: Partial<Entry>) {
		entries = (entries ?? []).map((entry) => (entry.id === id ? { ...entry, ...change } : entry));
	}

	async function save(entry: Entry) {
		saving = true;
		writeFailed = undefined;
		try {
			const note = draftNote.trim();
			const edited = await kamosu.editAttempt({
				attempt_id: entry.id,
				note: note === '' ? null : note,
				rating: draftRating,
			});
			layOver(entry.id, { note: edited.note, rating: edited.rating });
			opened = null;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			writeFailed = error.message;
		} finally {
			saving = false;
		}
	}

	async function remove(entry: Entry) {
		deleting = true;
		writeFailed = undefined;
		try {
			await kamosu.deleteAttempt({ attempt_id: entry.id });
			entries = (entries ?? []).filter((held) => held.id !== entry.id);
			opened = null;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			writeFailed = error.message;
		} finally {
			deleting = false;
		}
	}
</script>

<Screen title={m.cooked_title()} blurb={m.cooked_blurb()}>
	{#if failed}
		<p class="text-body text-support" role="alert">{m.cooked_failed()}</p>
	{:else if !entries}
		<p class="text-body text-ink-2">{m.loading()}</p>
	{:else if entries.length === 0}
		<Empty>{m.cooked_empty()}</Empty>
	{:else}
		<p class="mb-3 text-label text-ink-2 uppercase" role="status">
			{entries.length === 1 ? m.cooked_count_one() : m.cooked_count({ count: entries.length })}
		</p>

		{#each months as month (month.key)}
			<section class="mt-6 first:mt-0">
				<h2 class="mb-2 border-b border-rule pb-2 text-label font-medium text-accent uppercase">
					{month.heading}
				</h2>
				<ul>
					{#each month.of as entry (entry.id)}
						<li class="border-b border-rule py-3">
							<button
								type="button"
								class="block w-full text-left"
								aria-expanded={opened === entry.id}
								onclick={() => open(entry)}
							>
								<span class="flex items-baseline justify-between gap-3">
									<span class="min-w-0 font-display text-line">{entry.recipe.title}</span>
									<span class="shrink-0 text-read text-ink-2">{day(entry.created_at)}</span>
								</span>

								<!--
									The mark, and only where there is something to mark: a
									finished cooking is the ordinary case and wears nothing.

									`finished_at` is the whole of the question. Kamosu also
									knows `resumable` — still unfinished, and touched inside
									three days — and this screen deliberately does not read
									it, because that interval decides whether resuming is
									*offered* and never whether the cooking happened
									(ADR 0010). A pan you walked away from a fortnight ago is
									still the cooking `start_attempt` hands you back, so it
									reads the same here as one from this evening. A mark that
									flipped at seventy-two hours, with nobody having touched
									anything, would be Kamosu describing its own prompt.
								-->
								{#if entry.finished_at === null}
									<span class="mt-1 block text-read text-support">
										{m.cooked_in_progress()}
									</span>
								{/if}
								{#if entry.rating}
									<span class="mt-1 block text-read text-accent uppercase">
										{ratingLabel(entry.rating)}
									</span>
								{/if}
								{#if entry.note}
									<span class="mt-1 block text-read text-ink-2">{entry.note}</span>
								{/if}
							</button>

							{#if opened === entry.id}
								<div class="mt-3 rounded-sm border border-rule bg-ground-2 p-3">
									{#if entry.recipe.branch_id}
										<a
											href={`/recipes/${entry.recipe.branch_id}`}
											class="block min-h-12 py-3 text-body text-accent"
										>
											{m.cooked_open_recipe()}
										</a>
									{:else}
										<!--
											Leaving a Kitchen is not a deletion: the cooking
											stays, under the name it was known by, and there
											is simply nowhere for it to open (#32's story 20).
										-->
										<p class="py-3 text-read text-ink-2">{m.cooked_gone()}</p>
									{/if}

									<!--
										The verdict, as a toggle group — the house pattern
										from Settings, named once by the heading the group
										points at. *Not said* is one of the four on purpose:
										a rating you can set and never take back off is not
										the freely editable Attempt ADR 0005 promised.
									-->
									<div class="mt-2 border-t border-rule pt-3">
										<h3 id={`${entry.id}-verdict`} class="mb-2 text-label text-ink-2 uppercase">
											{m.cooked_rating()}
										</h3>
										<ul class="flex flex-wrap gap-2" aria-labelledby={`${entry.id}-verdict`}>
											{#each [null, 'again', 'tweak', 'no'] as const as verdict (verdict ?? 'none')}
												<li>
													<button
														type="button"
														aria-pressed={draftRating === verdict}
														onclick={() => (draftRating = verdict)}
														class="min-h-8 rounded-sm border px-3 text-read {draftRating === verdict
															? 'border-accent bg-accent text-on-accent'
															: 'border-rule text-ink-2'}"
													>
														{verdict === null ? m.cooked_rating_none() : ratingLabel(verdict)}
													</button>
												</li>
											{/each}
										</ul>
									</div>

									<!-- PROTOTYPE #58, TREATMENT B. THROWAWAY. -->
									{#if page.url.searchParams.get('variant') === 'B'}
										<Prototype58Promote branchId={entry.recipe.branch_id} />
									{/if}

									<label class="mt-3 grid gap-1 text-label text-ink-2 uppercase">
										{m.cooked_note()}
										<textarea
											bind:value={draftNote}
											rows="3"
											placeholder={m.cooked_no_note()}
											class="w-full rounded-sm border border-rule bg-card p-2 text-body text-ink"
										></textarea>
									</label>

									<div class="mt-3 grid gap-2">
										<button
											type="button"
											onclick={() => save(entry)}
											disabled={saving || deleting}
											class="min-h-12 rounded-sm bg-accent px-4 py-3 font-display text-body text-on-accent disabled:opacity-60"
										>
											{saving ? m.cooked_saving() : m.cooked_save()}
										</button>

										{#if confirming}
											<p class="text-read text-support">{m.cooked_delete_confirm()}</p>
											<div class="flex gap-2">
												<button
													type="button"
													onclick={() => remove(entry)}
													disabled={deleting}
													class="min-h-12 flex-1 rounded-sm border border-support px-4 py-3 text-body text-support disabled:opacity-60"
												>
													{m.cooked_delete_yes()}
												</button>
												<button
													type="button"
													onclick={() => (confirming = false)}
													class="min-h-12 flex-1 rounded-sm border border-rule px-4 py-3 text-body text-ink-2"
												>
													{m.cooked_delete_no()}
												</button>
											</div>
										{:else}
											<button
												type="button"
												onclick={() => (confirming = true)}
												class="min-h-12 rounded-sm border border-rule px-4 py-3 text-body text-support"
											>
												{m.cooked_delete()}
											</button>
										{/if}

										{#if writeFailed}
											<p class="text-read text-support" role="alert">
												{m.cooked_write_failed()}
												{writeFailed}
											</p>
										{/if}
									</div>
								</div>
							{/if}
						</li>
					{/each}
				</ul>
			</section>
		{/each}
	{/if}
</Screen>
