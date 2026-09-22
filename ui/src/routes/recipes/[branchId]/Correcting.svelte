<!--
	Correcting a Reading, in place, on the line it belongs to (#81; #32 item 193:
	"a bad Reading is a tap to fix on screen").

	Three things about this are decisions rather than styling:

	It opens UNDER THE LINE and nowhere else. The written Ingredient Line is the
	truth of the ingredient (ADR 0002) and the Reading is Kamosu's understanding
	of it, so the place to disagree with the understanding is beside the words it
	was read from — not on a settings screen, not in a queue of things to review.

	It makes NO VERSION. `set_reading` writes to the Branch's current head and
	nothing else, so a corrected Reading never appears in the Thread as a change
	to the recipe. The cook did not edit their recipe; they told Kamosu it had
	misread one. An append-only history that recorded the second as the first
	would be lying about what happened.

	It sends the WHOLE Reading, every time. `set_reading` takes amount, unit and
	target together and replaces what is there — there is no per-field patch, so
	a field left blank here is a field cleared, not a field left alone. All three
	blank clears the Reading entirely, which is what "Kamosu read nothing here"
	sends: a line with no Reading is an ordinary state, not a failure (ADR 0002).

	ON A COMPONENT, THE POINTER IS CARRIED AND THE TARGET FIELD IS NOT OFFERED
	(#50, ADR 0008). A Reading's target is either a Food's written word or the
	Lineage of a Recipe, never both, and the Core refuses a Reading claiming to
	be both. So a line naming a recipe says which one instead of offering a word
	to type, and its amount and Unit are corrected as on any other line — which
	is what changes how much of that recipe is wanted.

	Because the whole Reading is sent every time, leaving the Lineage out would
	not leave it alone: it would clear it, and a Component would quietly become
	an ordinary ingredient the first time somebody fixed its amount. So it is
	carried explicitly. "Kamosu read nothing here" still clears everything, the
	pointer included.

	SINCE #87 THE POINTER IS ALSO MADE AND UNMADE HERE. Until then `set_reading`
	could attach one and nothing in the interface called it, so a Component
	could only be made by an agent at the MCP door or by `curl` — and everything
	built on top of it, the unfolding and the annexe and the Passenger on a
	Share Link, was reachable only for a Component somebody had made that way.

	It belongs here for the reason everything else in this box does: this is
	already where you disagree with Kamosu's reading of a line, and saying "this
	line is a recipe" is exactly that disagreement — Kamosu read a Food and it
	is a Recipe. The two are one slot (ADR 0008), so they are one control, and
	picking a recipe puts the target field away rather than sitting beside it.

	NOTHING HERE GUESSES. The picker opens with nothing selected and suggests
	nothing, because ADR 0008 refuses matching `500 g plain flour` against a
	flour recipe in as many words.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { GetRecipeOutput } from '$lib/api/catalogue';
	import ComponentPicker, { type NamedRecipe } from './ComponentPicker.svelte';

	/** One slot of the `readings` array: what Kamosu understood, or nothing. */
	type Slot = GetRecipeOutput['versions'][number]['readings'][number];

	interface Props {
		branchId: string;
		/** Which Ingredient Line this is, counted over the written list. */
		lineIndex: number;
		/** The written line itself, which the picker says it was opened for. */
		line: string;
		/** The Reading as it stands, or null where Kamosu recorded none. */
		reading: Slot;
		/**
		 * The corrected Reading and the one subordinate line it now produces,
		 * handed back so the page redraws the whole row. The converted line is
		 * the Core's answer rather than anything worked out here (#49).
		 */
		onDone: (reading: Slot, measured: string | null) => void;
		onCancel: () => void;
		/** The Recipe this line names, where it names one and this instance has it. */
		componentTitle?: string | null;
	}

	let { branchId, lineIndex, line, reading, componentTitle, onDone, onCancel }: Props = $props();

	const kamosu = useKamosu();

	/**
	 * The Recipe this line names, as the box currently stands: seeded from the
	 * Reading and then changed by the picker and by *this line is not a
	 * recipe*. It is a draft like the three fields below it — nothing is
	 * written until the Reading is saved — which is why *not a recipe* only
	 * puts the target field back rather than writing anything.
	 */
	// svelte-ignore state_referenced_locally
	let namedRecipe = $state<NamedRecipe | null>(
		reading?.lineage_id ? { lineageId: reading.lineage_id, title: componentTitle ?? null } : null,
	);
	/** Whether this line names a Recipe rather than a Food (ADR 0008). */
	const isComponent = $derived(namedRecipe !== null);
	/** Whether the pointer in the box is not the one that is saved. */
	const changed = $derived((namedRecipe?.lineageId ?? null) !== (reading?.lineage_id ?? null));
	let picking = $state(false);

	/**
	 * The way in to the library, in matcha — the colour of a Reading that
	 * points at a recipe everywhere else on this page (#50). Named once because
	 * it is worn by two controls here and two more on the writing screen.
	 */
	const MATCHA = 'rounded-sm border border-support-2 px-2 py-1 text-read text-support-2';

	/**
	 * The three fields are a draft, seeded ONCE from the Reading as it stands
	 * when the corrector opens — deliberately not bound to the prop, so a redraw
	 * cannot overwrite what somebody is halfway through typing. The component is
	 * created and destroyed with the row it opens on, which is what makes
	 * seeding-once correct rather than merely convenient.
	 */
	// svelte-ignore state_referenced_locally
	let amount = $state(reading?.amount ?? '');
	// svelte-ignore state_referenced_locally
	let unit = $state(reading?.unit ?? '');
	// svelte-ignore state_referenced_locally
	let target = $state(reading?.target ?? '');
	let saving = $state(false);
	let failed = $state(false);
	/** Set when the corrector is closed under a request still in flight. */
	let abandoned = false;

	$effect(() => () => {
		abandoned = true;
	});

	/** A field's value, or null where nothing was written. `set_reading`
	 *  distinguishes the two, and an empty string is not a quantity. */
	const orNothing = (value: string) => (value.trim() === '' ? null : value.trim());

	async function send(next: {
		amount: string | null;
		unit: string | null;
		target: string | null;
		lineage_id: string | null;
	}) {
		saving = true;
		failed = false;
		try {
			const answered = await kamosu.setReading({
				branch_id: branchId,
				line_index: lineIndex,
				...next,
			});
			// Cancelling closes the corrector, but a request already in flight
			// still lands. Reporting it then would write a Reading onto the page
			// that somebody had already backed out of.
			if (!abandoned) onDone(answered.reading, answered.measured);
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			if (abandoned) return;
			failed = true;
			saving = false;
		}
	}

	const save = () =>
		send({
			amount: orNothing(amount),
			unit: orNothing(unit),
			// The two are exclusive, and the Core refuses a Reading claiming to
			// be both. A Component keeps its pointer and offers no word to type.
			target: isComponent ? null : orNothing(target),
			lineage_id: namedRecipe?.lineageId ?? null,
		});
	const clear = () => send({ amount: null, unit: null, target: null, lineage_id: null });
</script>

<div class="mt-3 border border-accent bg-card p-3">
	<p class="text-label font-semibold text-accent uppercase">{m.reading_heading()}</p>

	<div class="mt-2 grid grid-cols-2 gap-2">
		<label class="block">
			<span class="block text-label text-ink-2 uppercase">{m.reading_amount()}</span>
			<input
				bind:value={amount}
				inputmode="decimal"
				class="mt-1 w-full rounded-sm border border-rule bg-ground p-2 text-body"
			/>
		</label>
		<label class="block">
			<span class="block text-label text-ink-2 uppercase">{m.reading_unit()}</span>
			<input
				bind:value={unit}
				class="mt-1 w-full rounded-sm border border-rule bg-ground p-2 text-body"
			/>
		</label>
		<!--
			The one slot, in its two spellings (ADR 0008). A line names a Food or
			it names a Recipe, so what sits here is either the word to type or the
			recipe it points at — never both, and never one beside the other.
		-->
		{#if isComponent}
			<div class="col-span-2">
				<p class="text-read text-support-2">
					{changed
						? m.reading_will_name({ title: namedRecipe?.title ?? '' })
						: m.reading_names_recipe({ title: namedRecipe?.title ?? '' })}
				</p>
				<div class="mt-2 flex flex-wrap gap-2">
					<button type="button" class={MATCHA} onclick={() => (picking = true)}>
						{m.reading_change_recipe()}
					</button>
					<!--
						Un-making it is a DRAFT change like every other field in this
						box: the target field comes back and nothing is written until
						the Reading is saved. Clearing the Reading outright is still
						offered below, and is a different act — it says Kamosu read
						nothing here at all.
					-->
					<button
						type="button"
						class="rounded-sm border border-rule px-2 py-1 text-read text-ink-2"
						onclick={() => (namedRecipe = null)}
					>
						{m.reading_not_recipe()}
					</button>
				</div>
			</div>
		{:else}
			<label class="col-span-2 block">
				<span class="block text-label text-ink-2 uppercase">{m.reading_target()}</span>
				<input
					bind:value={target}
					class="mt-1 w-full rounded-sm border border-rule bg-ground p-2 text-body"
				/>
			</label>
			<!--
				Matcha, because that is the colour of a Reading that points at a
				recipe everywhere else on this page (#50). It is the way IN to the
				library, and it suggests nothing: ADR 0008 refuses guessing which
				recipe a line means.
			-->
			<button type="button" class="col-span-2 {MATCHA}" onclick={() => (picking = true)}>
				{m.reading_is_recipe()}
			</button>
		{/if}
	</div>

	<div class="mt-3 flex gap-2">
		<button
			type="button"
			onclick={save}
			disabled={saving}
			class="flex-1 rounded-sm bg-accent p-2 text-center text-read text-on-accent"
		>
			{m.reading_save()}
		</button>
		<button
			type="button"
			onclick={onCancel}
			class="rounded-sm border border-rule px-3 py-2 text-read text-ink-2"
		>
			{m.reading_cancel()}
		</button>
	</div>

	<!--
		THE WAY TO THE FOOD ITSELF (#107). One line, not a panel: everything a
		Food needs saying — its names in three Languages, what a cup of it
		weighs, how many lines point at it — is a screen's worth, and this box
		already carries five controls.

		It carries the SAVED word rather than what is in the box above, because
		the saved word is the one that resolved to a Food. Linking to whatever
		somebody is halfway through typing would go looking for a Food that does
		not exist yet.

		It hands over the WORD and not an id, because a Reading has no id to
		hand: a Food is instance-local and `reading_schema` keeps an id out of
		the Reading deliberately, so that a Reading means the same thing in a
		Bundle on somebody else's instance. The Foods screen resolves the word,
		and shows both where two Foods answer to it (ADR 0022) rather than
		guessing which was meant.

		Matcha, like every other way out of this page into another (#50).
	-->
	{#if !isComponent && reading?.target}
		<a
			href="/foods?q={encodeURIComponent(reading.target)}"
			class="mt-2 block w-full text-center {MATCHA}"
		>
			{m.reading_about_food({ food: reading.target })}
		</a>
	{/if}

	<button
		type="button"
		onclick={clear}
		disabled={saving}
		class="mt-2 block w-full rounded-sm border border-rule p-2 text-center text-read text-accent"
	>
		{m.reading_clear()}
	</button>

	{#if failed}
		<p class="mt-2 text-read text-support" role="alert">{m.reading_failed()}</p>
	{:else}
		<p class="mt-2 text-read text-ink-2">{m.reading_no_version()}</p>
	{/if}
</div>

{#if picking}
	<ComponentPicker
		{line}
		onChoose={(chosen) => {
			namedRecipe = chosen;
			picking = false;
		}}
		onCancel={() => (picking = false)}
	/>
{/if}
