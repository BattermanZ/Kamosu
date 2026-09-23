<!--
	Renaming, merging and deleting a Tag — Settings, not the recipe page (#104).

	Aurélien put them here on 22 September 2026, having first been shown them
	inside the recipe's own tag sheet: "I don't think a recipe page is the place
	to rename tags, it makes no sense." He is right, and the line is a useful one
	to keep: the sheet on a recipe answers *which of my words describe this
	dish*, and this answers *what my words are*. One is about a dish and the
	other is about a library.

	THE COUNT IS ON EVERY ROW, and it is the same number the shelf shows for the
	same tag, because both count Lineages (`recipes_with_tag`). That is what
	makes deleting an informed act rather than a leap: *18 recipes lose this
	tag* is a fact you can weigh.

	NOTHING HERE CHANGES A RECIPE (ADR 0035), and the section says so once at the
	top rather than on every row. A rename reaches every recipe carrying the Tag
	immediately, which is the whole point of keeping Tags once per Kitchen and is
	why no recipe has to be walked or re-saved.

	A TAG IS NAMED PER LANGUAGE, NOT SPLIT BY IT (ADR 0006). So a row shows the
	name in the reader's own Language where there is one, says which Language it
	fell back to where there is not, and offers to add the missing one — which is
	the only place in Kamosu where a Tag known only as *mijoté* can be given an
	English name.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Confirm from '$lib/Confirm.svelte';
	import { byWord, oneOfThree, tagWord, type ReadingLanguage, type Tag } from '$lib/tags';
	import { languageName } from '$lib/language';
	import type { ListKitchensOutput } from '$lib/api/catalogue';

	interface Props {
		/** The Kitchens this Person cooks in, as Settings already read them. */
		kitchens: ListKitchensOutput['kitchens'];
		/**
		 * The Language this Person reads RECIPES in, which Settings has already
		 * asked for. Not the interface locale: the two are separate settings,
		 * and a Tag's name belongs to the Language its recipes are read in
		 * (ADR 0006).
		 */
		readingLanguage: ReadingLanguage;
	}

	let { kitchens, readingLanguage }: Props = $props();

	const kamosu = useKamosu();

	/** Each Kitchen's Tags, by Kitchen id. A Tag belongs to one (ADR 0007). */
	let byKitchen = $state<Record<string, Tag[]>>({});
	let failed = $state<string | undefined>(undefined);
	/** The act running right now, so a row cannot be fired twice. */
	let busy = $state(false);
	/**
	 * What is being confirmed, and what doing it would take: a delete, or a
	 * merge that has been chosen. It carries the ids and the words rather than
	 * the Tags, so the act needs nothing narrowed or cast back out of it — the
	 * compiler is the reviewer this frontend does not have (ADR 0012), and a
	 * cast is where it stops looking.
	 */
	let confirming = $state<
		| { kind: 'delete'; id: string; name: string; recipes: number; run: () => void }
		| {
				kind: 'merge';
				fromName: string;
				intoName: string;
				recipes: number;
				run: () => void;
		  }
		| null
	>(null);
	/** What a rename just reached, said once and cleared by the next act. */
	let renamed = $state<number | null>(null);

	async function load() {
		const lists = await Promise.all(
			kitchens.map(async (kitchen) => {
				try {
					const answer = await kamosu.listTags({ kitchen_id: kitchen.id });
					return [kitchen.id, answer.tags] as const;
				} catch (error) {
					if (!(error instanceof OperationError)) throw error;
					return [kitchen.id, [] as Tag[]] as const;
				}
			}),
		);
		byKitchen = Object.fromEntries(lists);
	}

	// Read again whenever the Kitchens change, which is how a Kitchen just
	// joined arrives with its own words rather than with none. `load` reads
	// `kitchens` before its first await, so the effect tracks it.
	//
	// And whenever the Reading Language changes: the Core chose each word and
	// its mark for the Language asked for when the list was read, so a list
	// kept across a change shows one Language while renames write another
	// (#112).
	$effect(() => {
		void readingLanguage;
		void load();
	});

	/** The field that adds the missing name says which Language it wants, in words. */
	const addName = $derived(m.settings_tags_add_name({ language: languageName(readingLanguage) }));

	/** In the order a reader can predict: by the word they see. */
	function listed(kitchenId: string): Tag[] {
		return byWord(byKitchen[kitchenId] ?? []);
	}

	async function act(run: () => Promise<void>) {
		failed = undefined;
		busy = true;
		try {
			await run();
			await load();
			confirming = null;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		} finally {
			busy = false;
		}
	}

	/**
	 * Name a Tag in one Language, or change the name it has there. Which
	 * Language is the one the name on screen is written in — renaming what you
	 * can see — except where that is not the reader's own, and then the second
	 * field below adds the missing name instead. One Operation either way.
	 *
	 * It says what it reached, because "renaming reaches every recipe carrying
	 * it, immediately" is the whole reason Tags are kept once per Kitchen and
	 * was until now a promise with nothing on screen behind it.
	 *
	 * The number comes from the RENAME'S OWN ANSWER rather than from the count
	 * the row was showing. They are the same number whenever the row is fresh,
	 * and when it is not — somebody tagged that recipe on their phone a moment
	 * ago — the row's is the stale one, and a sentence about how far a rename
	 * reached is the wrong place to be approximately right.
	 */
	function rename(tag: Tag, language: ReadingLanguage, name: string) {
		const word = name.trim();
		if (word === '' || word === tag.name) return;
		renamed = null;
		return act(async () => {
			const renamedTag = await kamosu.renameTag({ tag_id: tag.id, language, name: word });
			renamed = renamedTag.recipes;
		});
	}

	function merge(fromId: string, intoId: string) {
		renamed = null;
		return act(async () => {
			await kamosu.mergeTags({ keep_tag_id: intoId, merge_tag_id: fromId });
		});
	}

	function remove(tagId: string) {
		renamed = null;
		return act(async () => {
			await kamosu.deleteTag({ tag_id: tagId });
		});
	}

	/** What a Tag can be merged into: this Kitchen's other Tags, never itself. */
	function others(kitchenId: string, tag: Tag): Tag[] {
		return listed(kitchenId).filter((held) => held.id !== tag.id);
	}
</script>

<p class="mb-4 text-read text-ink-2">{m.settings_tags_blurb()}</p>

{#if failed}
	<p class="mb-4 text-body text-support" role="alert">{failed}</p>
{/if}

<!--
	What the rename just reached. The ticket asks that renaming is *seen* to
	reach every recipe carrying the Tag, and until this line the only evidence
	was a sentence promising it would.
-->
{#if renamed !== null}
	<p class="mb-4 text-read text-accent" role="status">
		{renamed === 0
			? m.settings_tags_renamed_none()
			: renamed === 1
				? m.settings_tags_renamed_one()
				: m.settings_tags_renamed({ count: renamed })}
	</p>
{/if}

<ul class="grid gap-4">
	{#each kitchens as kitchen (kitchen.id)}
		{@const tags = listed(kitchen.id)}
		<li class="min-w-0 rounded-sm border border-rule bg-card p-3">
			<h3 class="mb-2 text-label text-ink-2 uppercase">{kitchen.nickname ?? kitchen.name}</h3>

			{#if tags.length === 0}
				<p class="text-read text-ink-2">{m.settings_tags_none()}</p>
			{:else}
				<ul class="grid gap-3">
					{#each tags as tag (tag.id)}
						{@const word = tagWord(tag)}
						<li class="min-w-0 border-t border-rule pt-3 first:border-t-0 first:pt-0">
							<!--
								`min-w-0` on the row and on the box, for the reason
								the Kitchen card above documents: a text box will
								not shrink below its content otherwise, and at a
								320 px viewport the card shoves the whole page
								sideways.
							-->
							<form
								class="flex items-center gap-2"
								onsubmit={(event) => {
									event.preventDefault();
									const input = event.currentTarget.elements.namedItem('name') as HTMLInputElement;
									// Renaming what is on screen names it in the
									// Language it is written in, which is not
									// necessarily the one this Person reads
									// recipes in (ADR 0006).
									// The Language the name on screen is written in, or
									// this reader's own where it is already theirs.
									rename(tag, oneOfThree(word.elsewhere, readingLanguage), input.value);
								}}
							>
								<label class="sr-only" for={`tag-name-${tag.id}`}>
									{m.settings_tags_name_label()}
								</label>
								<input
									id={`tag-name-${tag.id}`}
									name="name"
									value={tag.name ?? ''}
									class="min-h-12 min-w-0 flex-1 rounded-sm border border-rule bg-ground px-2 text-body
									text-ink"
								/>
								{#if word.elsewhere}
									<span class="shrink-0 text-label text-support uppercase">
										<span aria-hidden="true">{word.elsewhere}</span>
										<span class="sr-only">{word.said}</span>
									</span>
								{/if}
								<span class="shrink-0 text-read text-ink-2">
									{tag.recipes === 1
										? m.settings_tags_one_recipe()
										: m.settings_tags_recipes({ count: tag.recipes })}
								</span>
								<button
									class="shrink-0 text-label text-accent underline"
									type="submit"
									disabled={busy}
								>
									{m.settings_tags_rename()}
								</button>
							</form>

							<!--
								The missing name, offered where the Tag has none in
								the reader's own Language. This is the fallback
								being *fixed* rather than merely displayed: ADR
								0006 built the falling back, and nothing until now
								let anybody add the name it fell back from.
							-->
							{#if word.elsewhere}
								<form
									class="mt-2 flex items-center gap-2"
									onsubmit={(event) => {
										event.preventDefault();
										const input = event.currentTarget.elements.namedItem(
											'added',
										) as HTMLInputElement;
										rename(tag, readingLanguage, input.value);
									}}
								>
									<label class="sr-only" for={`tag-add-${tag.id}`}>
										{addName}
									</label>
									<input
										id={`tag-add-${tag.id}`}
										name="added"
										placeholder={addName}
										class="min-h-12 min-w-0 flex-1 rounded-sm border border-rule bg-ground px-2 text-body
										text-ink"
									/>
									<button
										class="shrink-0 text-label text-accent underline"
										type="submit"
										disabled={busy}
									>
										{m.settings_tags_rename()}
									</button>
								</form>
							{/if}

							<div class="mt-2 flex flex-wrap items-center gap-4">
								{#if others(kitchen.id, tag).length > 0}
									<label class="flex min-w-0 items-center gap-2">
										<span class="sr-only">{m.settings_tags_merge_label()}</span>
										<select
											disabled={busy}
											class="min-h-12 min-w-0 rounded-sm border border-rule bg-ground px-2 text-read
											text-accent"
											onchange={(event) => {
												const chosen = event.currentTarget.value;
												event.currentTarget.value = '';
												const into = others(kitchen.id, tag).find((held) => held.id === chosen);
												if (!into) return;
												// Asked before it happens, like every
												// other act that cannot be undone
												// (#103): the sheet leads with how many
												// recipes move.
												confirming = {
													kind: 'merge',
													fromName: word.name,
													intoName: tagWord(into).name,
													recipes: tag.recipes,
													run: () => void merge(tag.id, into.id),
												};
											}}
										>
											<option value="">{m.settings_tags_merge()}</option>
											{#each others(kitchen.id, tag) as into (into.id)}
												<option value={into.id}>{tagWord(into).name}</option>
											{/each}
										</select>
									</label>
								{/if}
								<button
									type="button"
									disabled={busy}
									onclick={() =>
										(confirming = {
											kind: 'delete',
											id: tag.id,
											name: word.name,
											recipes: tag.recipes,
											run: () => void remove(tag.id),
										})}
									class="text-label text-support uppercase underline"
								>
									{m.settings_tags_delete()}
								</button>
							</div>
						</li>
					{/each}
				</ul>
			{/if}
		</li>
	{/each}
</ul>

{#if confirming?.kind === 'delete'}
	{@const asked = confirming}
	<Confirm
		title={m.settings_tags_delete_title({ tag: asked.name })}
		count={asked.recipes}
		counting={asked.recipes === 1
			? m.settings_tags_delete_counting_one()
			: m.settings_tags_delete_counting()}
		consequence={m.settings_tags_delete_consequence()}
		act={m.settings_tags_delete()}
		{busy}
		{failed}
		run={asked.run}
		cancel={() => (confirming = null)}
	/>
{:else if confirming?.kind === 'merge'}
	{@const asked = confirming}
	<Confirm
		title={m.settings_tags_merge_title({ from: asked.fromName, into: asked.intoName })}
		count={asked.recipes}
		counting={asked.recipes === 1
			? m.settings_tags_merge_counting_one()
			: m.settings_tags_merge_counting()}
		consequence={m.settings_tags_merge_consequence({
			from: asked.fromName,
			into: asked.intoName,
		})}
		act={m.settings_tags_merge_act()}
		{busy}
		{failed}
		run={asked.run}
		cancel={() => (confirming = null)}
	/>
{/if}
