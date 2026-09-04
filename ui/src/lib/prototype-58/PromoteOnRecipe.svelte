<!--
	PROTOTYPE — #58, TREATMENT A, the second moment. THROWAWAY. Delete on merge.

	Promotion offered ON THE RECIPE, in a band above "Cook this".

	The argument for putting it here: what you are being asked is a question
	about the recipe — should these words become part of it — and the recipe is
	where its Versions already live. You are standing in the thing you would be
	changing, and the Thread is one control away, so "what would this look like
	afterwards" is answerable without leaving.

	Pressing it does NOT mint a Version. It opens the recipe's ordinary editing
	screen with the As Cooked already in the fields, so promoting is a save you
	can still back out of, and the lines can be tidied before they join a
	history that is append-only.
-->
<script lang="ts">
	import { asCooked } from './deviation.svelte';

	interface Props {
		branchId: string;
		/**
		 * A line and a step off the recipe on screen, so the prototype can seed a
		 * deviation for someone who has landed here without cooking first. The
		 * first hand-over gave this page to Aurélien cold and it rendered nothing
		 * at all, which read as a broken prototype rather than as an empty state.
		 */
		sample: {
			ingredient: { at: number; text: string } | null;
			step: { at: number; text: string } | null;
		};
	}
	let { branchId, sample }: Props = $props();

	/** Fill in a plausible deviation, so both moments can be looked at cold. */
	function seed() {
		if (sample.ingredient) {
			asCooked.write(
				branchId,
				'',
				'ingredient',
				sample.ingredient.at,
				sample.ingredient.text,
				sample.ingredient.text + ' — used half',
			);
		}
		if (sample.step) {
			asCooked.write(
				branchId,
				'',
				'step',
				sample.step.at,
				sample.step.text,
				sample.step.text + ' Gave it ten minutes longer than that.',
			);
		}
	}

	const held = $derived(asCooked.held?.branchId === branchId ? asCooked.held : null);
	const lines = $derived(held?.rewrites.filter((one) => one.list === 'ingredient').length ?? 0);
	const steps = $derived(held?.rewrites.filter((one) => one.list === 'step').length ?? 0);

	let editing = $state(false);
	let saved = $state(false);

	const when = $derived(
		held
			? new Date(held.when).toLocaleDateString(undefined, { day: 'numeric', month: 'long' })
			: '',
	);

	const counted = $derived(
		[
			lines > 0 ? `${lines} ${lines === 1 ? 'ingredient' : 'ingredients'}` : null,
			steps > 0 ? `${steps} ${steps === 1 ? 'step' : 'steps'}` : null,
		]
			.filter(Boolean)
			.join(' and '),
	);
</script>

{#if held && !saved}
	<div class="mx-gutter mt-6 border border-accent" style="border-radius:2px">
		<div class="p-4">
			<p class="text-label text-accent uppercase">You cooked this differently</p>
			<p class="mt-1 font-display text-line">{when} — you changed {counted}.</p>

			{#if editing}
				<!--
					Standing in for the recipe's own editing screen (#83), pre-filled
					from the As Cooked. The point being judged is that Promotion drops
					you INTO an edit rather than committing one behind a single tap.
				-->
				<p class="mt-3 text-label text-ink-2 uppercase">The lines as you cooked them</p>
				{#each held.rewrites as one (one.list + one.at)}
					<div class="border-b border-rule py-2">
						<input
							value={one.now}
							readonly
							class="w-full font-display text-line text-ink"
							style="background:var(--color-card);border:1px solid var(--color-rule);
								border-radius:2px;padding:8px;min-height:44px"
						/>
						<p class="mt-1 text-read text-ink-2" style="text-decoration:line-through">{one.was}</p>
					</div>
				{/each}
				<button
					type="button"
					class="mt-3 block w-full bg-accent p-4 text-center font-display text-body text-on-accent"
					onclick={() => {
						saved = true;
						asCooked.clear();
					}}
				>
					Save as a new Version
				</button>
				<button
					type="button"
					class="mt-2 block w-full border border-rule p-4 text-center font-display text-body text-ink-2"
					onclick={() => (editing = false)}
				>
					Not now
				</button>
			{:else if asCooked.offline}
				<!--
					The refusal, and the whole of why it is worded like this: nothing
					has been lost, and the cook is not being asked to do anything
					about it (ADR 0013).
				-->
				<p class="mt-3 text-body text-ink-2">
					Keeping it in the recipe needs the server, and you are offline. Your cooking is saved —
					this will still be here.
				</p>
				<button
					type="button"
					disabled
					class="mt-3 block w-full border border-rule p-4 text-center font-display text-body text-ink-2"
					style="opacity:.55"
				>
					Keep it as a new Version
				</button>
			{:else}
				<button
					type="button"
					class="mt-3 block w-full bg-accent p-4 text-center font-display text-body text-on-accent"
					onclick={() => (editing = true)}
				>
					Keep it as a new Version
				</button>
				<button
					type="button"
					class="mt-2 block w-full border border-rule p-4 text-center font-display text-body text-ink-2"
					onclick={() => asCooked.clear()}
				>
					Leave it in the diary
				</button>
			{/if}
		</div>
	</div>
{:else if saved}
	<p class="mx-gutter mt-6 text-read text-accent" role="status">
		Kept. It is a Version on this Branch now, and the Thread says where it came from.
	</p>
{:else}
	<!--
		The empty state, which exists only for the prototype: this band is not
		drawn at all when a cooking deviated from nothing, and that is the point.
		Said out loud here so landing on this page cold reads as "nothing to show
		yet" rather than as nothing happening.
	-->
	<div class="mx-gutter mt-6 border border-rule p-4" style="border-style:dashed">
		<p class="text-label text-ink-2 uppercase">Prototype — treatment A</p>
		<p class="mt-1 text-body text-ink-2">
			Nothing was cooked differently, so this band is not drawn. That is the common case, and it
			costs the recipe page nothing.
		</p>
		<a
			href="/cook/{branchId}?variant=A"
			class="mt-3 block border border-accent p-3 text-center font-display text-body text-accent"
		>
			Go and cook it, and change something
		</a>
		<button
			type="button"
			class="mt-2 block w-full border border-rule p-3 text-center font-display text-body text-ink-2"
			onclick={seed}
		>
			Or fill in a change for me
		</button>
	</div>
{/if}
