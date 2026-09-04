<!--
	PROTOTYPE — #58, TREATMENT B, the second moment. THROWAWAY. Delete on merge.

	Promotion offered IN THE DIARY, inside the entry you already open to rate the
	cooking and write your note.

	The argument for putting it here: what you changed is part of what you would
	say about that afternoon, and this panel is already the one place an Attempt
	is edited. You are not asked about it while looking at the recipe — you are
	asked about it while looking at the cooking, next to "would you cook it
	again", which is the question that actually decides the answer.

	One tap mints the Version. There is no editing step: the As Cooked is a whole
	recipe already, written by the cook, at the stove, in their own words — and
	an intermediate form to tidy it in is a second place for it to be wrong.
-->
<script lang="ts">
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { asCooked } from './deviation.svelte';

	interface Props {
		branchId: string | null;
	}
	let { branchId }: Props = $props();

	const kamosu = useKamosu();
	const held = $derived(branchId && asCooked.held?.branchId === branchId ? asCooked.held : null);
	let saved = $state(false);
	let seeding = $state(false);

	/**
	 * Fill in a plausible deviation off the real recipe, so this moment can be
	 * looked at without cooking one first. Prototype scaffolding only: the first
	 * hand-over gave this screen to Aurélien cold, it drew nothing, and that read
	 * as a broken prototype rather than as an empty state.
	 */
	async function seed() {
		if (!branchId) return;
		seeding = true;
		try {
			const recipe = await kamosu.getRecipe({ branch_id: branchId });
			const content = recipe.versions.at(-1)?.content;
			if (!content) return;
			const ingredientAt = content.ingredients.findIndex((row) => row.kind === 'ingredient');
			const stepAt = content.steps.findIndex((row) => row.kind === 'step');
			const ingredient = ingredientAt >= 0 ? content.ingredients[ingredientAt] : undefined;
			const step = stepAt >= 0 ? content.steps[stepAt] : undefined;
			if (ingredient) {
				asCooked.write(
					branchId,
					content.title,
					'ingredient',
					ingredientAt,
					ingredient.text,
					ingredient.text + ' — used half',
				);
			}
			if (step) {
				asCooked.write(
					branchId,
					content.title,
					'step',
					stepAt,
					step.text,
					step.text + ' Gave it ten minutes longer than that.',
				);
			}
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
		} finally {
			seeding = false;
		}
	}
</script>

{#if !held && branchId}
	<!--
		The empty state, prototype scaffolding only: this block is not drawn at all
		for a cooking that deviated from nothing, which is the common case and the
		point. Said out loud so landing here cold does not look like nothing
		happening.
	-->
	<div class="mt-3 border-t border-rule pt-3">
		<p class="text-label text-ink-2 uppercase">Prototype — treatment B</p>
		<p class="mt-1 text-body text-ink-2">
			This cooking deviated from nothing, so no block is drawn here.
		</p>
		<a
			href="/cook/{branchId}?variant=B"
			class="mt-3 block border border-accent p-3 text-center font-display text-body text-accent"
		>
			Go and cook it, and change something
		</a>
		<button
			type="button"
			disabled={seeding}
			class="mt-2 block w-full border border-rule p-3 text-center font-display text-body text-ink-2"
			onclick={seed}
		>
			{seeding ? 'Filling it in…' : 'Or fill in a change for me'}
		</button>
	</div>
{/if}

{#if held}
	<div class="mt-3 border-t border-rule pt-3">
		<h3 class="mb-2 text-label text-ink-2 uppercase">What you changed</h3>

		{#if saved}
			<p class="text-read text-accent" role="status">
				Kept. It is a Version on the recipe now — this cooking still says it cooked the old one.
			</p>
		{:else}
			<ul>
				{#each held.rewrites as one (one.list + one.at)}
					<li class="border-b border-rule py-2">
						<span class="block font-display text-line text-ink">{one.now}</span>
						<span class="block text-read text-ink-2" style="text-decoration:line-through">
							{one.was}
						</span>
					</li>
				{/each}
			</ul>

			{#if asCooked.offline}
				<!--
					The refusal (ADR 0013). Said before the button rather than after
					it, because a disabled control a cook has already reached for has
					already cost them the reach.
				-->
				<p class="mt-3 text-read text-support">
					Promoting needs the server, and you are offline. The cooking is saved; the recipe is the
					half Kamosu will not write without one.
				</p>
				<button
					type="button"
					disabled
					class="mt-2 min-h-12 w-full rounded-sm border border-rule px-4 py-3 text-body text-ink-2"
					style="opacity:.55"
				>
					Keep this as a new Version
				</button>
			{:else}
				<button
					type="button"
					class="mt-3 min-h-12 w-full rounded-sm bg-accent px-4 py-3 font-display text-body text-on-accent"
					onclick={() => {
						saved = true;
						asCooked.clear();
					}}
				>
					Keep this as a new Version
				</button>
			{/if}
		{/if}
	</div>
{/if}
