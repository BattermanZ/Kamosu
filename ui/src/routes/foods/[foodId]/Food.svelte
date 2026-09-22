<!--
	One Food: what it is called, what a cup of it weighs, and how much it matters (#107).

	Chosen by Aurélien on 22 September 2026 from three designs drawn at a phone
	viewport — a Food is a page, reached two ways: from the Ingredient Line that
	named it, and from the Foods list in Settings. The alternative designs put
	the whole of this inside the Reading corrector, or only in Settings; the
	first cannot reach a Food no recipe in front of you names, and the second is
	a screen away from the moment somebody is holding the bag.

	**NOTHING HERE IS RECIPE CONTENT (ADR 0002).** A Food is Kamosu's
	understanding of an Ingredient Line, never the line itself. Correcting one
	writes no Version, changes no written line and appears in no Thread — which
	is said on the page rather than merely being true, because a screen that
	quietly edits recipes and a screen that quietly does not look identical.

	**THE READING COUNT IS THE HONEST MEASURE.** `get_food` answers how many
	Readings point at this Food, and that is the only fact on the page that says
	whether correcting it is worth anything. A Food nothing points at is a Food
	nobody will notice you fixed.

	**A NAME PER LANGUAGE IS NOT A TRANSLATION OF ANYTHING** (ADR 0006). Teaching
	the Food called `flour` that it is also `farine` is what puts both on one
	shopping-list line. It says nothing about any recipe's Language, and the page
	says so, because "add French" on a screen full of English otherwise reads as
	an offer to translate a recipe.

	**THE LAST NAME CANNOT BE TAKEN OFF.** A Food is known by its words alone, so
	the Core refuses `remove_food_name` on the only one left. The refusal is
	worded here as a sentence and the control is simply absent on a Food with one
	name — a button that exists to explain why it does not work is worse than no
	button. The Core's refusal is still shown verbatim if it ever arrives, since
	it is written to be read.

	**WHAT IS NOT HERE.** No nutrition editor: #72 made nutrition a manual
	per-recipe field and deferred per-ingredient data, and all 607 Foods answer
	`nutrition: null` correctly. No merge, no delete, no merge preview: those are
	the Operator's and live on the Operator's screen (#103).
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';
	import Section from '$lib/shell/Section.svelte';
	import { languageName } from '$lib/language';
	import {
		cupWeightBox,
		linesPointingAt,
		NAME_LANGUAGES,
		nameIn,
		type Food,
		type NameLanguage,
	} from '$lib/foods';

	interface Props {
		foodId: string;
	}

	let { foodId }: Props = $props();

	const kamosu = useKamosu();

	let food = $state<Food | undefined>(undefined);
	/** Set when the Food itself could not be read: there is no page without it. */
	let unreachable = $state(false);
	/** A refusal from an act, shown in the words it arrived in. */
	let said = $state<string | undefined>(undefined);
	let working = $state(false);

	/** Which Language's name is being typed, and what has been typed into it. */
	let editing = $state<NameLanguage | null>(null);
	let draft = $state('');

	/** The Cup Weight box. Seeded from the Food each time one is loaded. */
	let grams = $state('');

	$effect(() => {
		let current = true;
		kamosu
			.getFood({ food_id: foodId })
			.then((answer) => {
				if (!current) return;
				food = answer;
				grams = cupWeightBox(answer.cup_weight_grams);
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				if (current) unreachable = true;
			});
		return () => {
			current = false;
		};
	});

	/**
	 * Every act on this page answers with the whole Food, so what is drawn after
	 * one is the Core's own answer rather than anything patched together here.
	 */
	async function act(run: () => Promise<Food>) {
		working = true;
		said = undefined;
		try {
			const answered = await run();
			food = answered;
			grams = cupWeightBox(answered.cup_weight_grams);
			editing = null;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			said = error.message;
		} finally {
			working = false;
		}
	}

	const saveName = (language: NameLanguage) =>
		act(() => kamosu.setFoodName({ food_id: foodId, language, name: draft.trim() }));

	const takeNameOff = (language: NameLanguage) =>
		act(() => kamosu.removeFoodName({ food_id: foodId, language }));

	/**
	 * An empty box clears the Cup Weight back to "offers millilitres instead of
	 * grams", which is an ordinary state and the one every Food starts in — not
	 * a failure to fill something in.
	 */
	function saveCupWeight() {
		const typed = grams.trim();
		if (typed === '') {
			return act(() => kamosu.setFoodCupWeight({ food_id: foodId, cup_weight_grams: null }));
		}
		// Comma or point: a French keyboard writes 125,5 and means what an
		// English one writes as 125.5.
		const value = Number(typed.replace(',', '.'));
		if (!Number.isFinite(value) || value <= 0) {
			said = m.food_cup_weight_must_be_a_number();
			return Promise.resolve();
		}
		return act(() => kamosu.setFoodCupWeight({ food_id: foodId, cup_weight_grams: value }));
	}

	const nameCount = $derived(food?.names.length ?? 0);

	function startEditing(language: NameLanguage) {
		draft = (food && nameIn(food, language)) ?? '';
		editing = language;
	}
</script>

{#if unreachable}
	<Screen title={m.food_unknown_title()} blurb={m.food_unknown_blurb()} />
{:else if food === undefined}
	<Screen title={m.loading()} />
{:else}
	<Screen title={food.name ?? m.food_unnamed()} blurb={m.food_blurb()}>
		<p class="text-body text-ink-2">
			{linesPointingAt(food.reading_count)}
		</p>

		<Section heading={m.food_called()}>
			{#each NAME_LANGUAGES as language (language)}
				{@const named = nameIn(food, language)}
				<div class="border-b border-rule py-3">
					{#if editing === language}
						<label class="block">
							<span class="block text-label text-ink-2 uppercase">
								{languageName(language)}
							</span>
							<input
								bind:value={draft}
								class="mt-1 w-full rounded-sm border border-rule bg-ground p-2 text-body"
							/>
						</label>
						<div class="mt-2 flex gap-2">
							<button
								type="button"
								disabled={working || draft.trim() === ''}
								onclick={() => saveName(language)}
								class="flex-1 rounded-sm bg-accent p-2 text-center text-read text-on-accent"
							>
								{m.food_name_save()}
							</button>
							<button
								type="button"
								onclick={() => (editing = null)}
								class="rounded-sm border border-rule px-3 py-2 text-read text-ink-2"
							>
								{m.food_name_cancel()}
							</button>
						</div>
					{:else}
						<div class="flex items-center justify-between gap-3">
							<span class="min-w-0">
								{#if named === null}
									<span class="block text-body text-ink-2">
										{m.food_not_named({ language: languageName(language) })}
									</span>
								{:else}
									<span class="block text-body text-ink">{named}</span>
								{/if}
								<span class="text-read text-ink-2">{languageName(language)}</span>
							</span>
							<!--
								The buttons say `Add` and `Take off`, which is all the room a
								phone has and all a reader needs beside the Language they sit
								next to. Anybody listening to the page instead gets the whole
								sentence, because three identical `Take off`s read aloud in a
								row name nothing at all.
							-->
							<span class="flex shrink-0 gap-2">
								<button
									type="button"
									disabled={working}
									onclick={() => startEditing(language)}
									aria-label={named === null
										? m.food_name_add_in({ language: languageName(language) })
										: m.food_name_change_in({ language: languageName(language) })}
									class="rounded-sm border border-rule px-2 py-1 text-read text-ink-2"
								>
									{named === null ? m.food_name_add() : m.food_name_change()}
								</button>
								<!--
									Absent, not disabled, on the last remaining name. The Core
									refuses it and the sentence below says why; a control that
									exists only to explain its own refusal is worse than none.
								-->
								{#if named !== null && nameCount > 1}
									<button
										type="button"
										disabled={working}
										onclick={() => takeNameOff(language)}
										aria-label={m.food_name_remove_in({ language: languageName(language) })}
										class="rounded-sm border border-rule px-2 py-1 text-read text-ink-2"
									>
										{m.food_name_remove()}
									</button>
								{/if}
							</span>
						</div>
					{/if}
				</div>
			{/each}

			{#if nameCount <= 1}
				<p class="mt-3 text-read text-support">{m.food_keeps_one_name()}</p>
			{/if}
			<p class="mt-3 text-read text-ink-2">{m.food_names_not_a_translation()}</p>
		</Section>

		<Section heading={m.food_cup_weight()}>
			<div class="flex gap-2">
				<input
					bind:value={grams}
					inputmode="decimal"
					aria-label={m.food_cup_weight()}
					placeholder={m.food_cup_weight_none()}
					class="w-full rounded-sm border border-rule bg-ground p-2 text-body"
				/>
				<span class="text-body text-ink-2">{m.food_grams()}</span>
			</div>
			<p class="mt-2 text-read text-ink-2">{m.food_cup_weight_explained()}</p>
			<button
				type="button"
				disabled={working}
				onclick={saveCupWeight}
				class="mt-3 w-full rounded-sm bg-accent p-2 text-center text-read text-on-accent"
			>
				{m.food_cup_weight_save()}
			</button>
		</Section>

		{#if said}
			<p class="mt-4 text-body text-support" role="alert">{said}</p>
		{/if}

		<p class="mt-6 text-read text-ink-2">{m.food_changes_no_recipe()}</p>
	</Screen>
{/if}
