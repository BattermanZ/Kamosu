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
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { GetRecipeOutput } from '$lib/api/catalogue';

	/** One slot of the `readings` array: what Kamosu understood, or nothing. */
	type Slot = GetRecipeOutput['versions'][number]['readings'][number];

	interface Props {
		branchId: string;
		/** Which Ingredient Line this is, counted over the written list. */
		lineIndex: number;
		/** The Reading as it stands, or null where Kamosu recorded none. */
		reading: Slot;
		/** The corrected Reading, handed back so the page redraws the line. */
		onDone: (reading: Slot) => void;
		onCancel: () => void;
	}

	let { branchId, lineIndex, reading, onDone, onCancel }: Props = $props();

	const kamosu = useKamosu();

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

	async function send(next: { amount: string | null; unit: string | null; target: string | null }) {
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
			if (!abandoned) onDone(answered.reading);
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			if (abandoned) return;
			failed = true;
			saving = false;
		}
	}

	const save = () =>
		send({ amount: orNothing(amount), unit: orNothing(unit), target: orNothing(target) });
	const clear = () => send({ amount: null, unit: null, target: null });
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
		<label class="col-span-2 block">
			<span class="block text-label text-ink-2 uppercase">{m.reading_target()}</span>
			<input
				bind:value={target}
				class="mt-1 w-full rounded-sm border border-rule bg-ground p-2 text-body"
			/>
		</label>
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
