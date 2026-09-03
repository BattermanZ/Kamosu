<!--
	PROTOTYPE — #50 Components. Throwaway, and on a `prototype/` branch rather
	than on dev.

	Two treatments of one addition to the recipe page, on real recipes read
	live out of the dev instance through the real `get_recipe` Operation.

	The one thing invented here is the composition itself: which line's Reading
	points at a Lineage rather than a Food. The Core cannot answer that yet —
	building it is what #50 is. Every other thing on the screen is real.
-->
<script lang="ts">
	import { useKamosu } from '$lib/kamosu';
	import PrototypeRecipe from './PrototypeRecipe.svelte';
	import { held, type Held, type Composition } from './composition';

	const kamosu = useKamosu();

	// ---- the cast, all of it in the dev instance's own library -------------

	const PIZZA = 'b_d8ec5cecf8f0c853';
	const DAN_DAN = 'b_87a20bb4bcaabcef';
	const CHILLI_OIL = 'b_d8b44f86d51e64e6';
	const NEAPOLITAN = 'b_a63b79303707914b';
	const VERSATILE = 'b_d63ff373746226c9';

	const LINEAGE = {
		pizza: 'l_276ddd8ba22faff2',
		danDan: 'l_20f814c8d5df0c04',
		chilliOil: 'l_cd91722a80879ed9',
		neapolitan: 'l_3d2c4c8d7676d0f9',
		versatile: 'l_cc45b04ab03687b4',
		/** A Lineage this instance does not hold — a dough that never arrived. */
		absent: 'l_0000000000000000',
	};

	let library = $state<Record<string, Held>>({});
	let byBranch = $state<Record<string, Held>>({});
	let loaded = $state(false);

	$effect(() => {
		void (async () => {
			const branches = [PIZZA, DAN_DAN, CHILLI_OIL, NEAPOLITAN, VERSATILE];
			const read = await Promise.all(branches.map((branch_id) => kamosu.getRecipe({ branch_id })));
			const all = read.map(held);
			library = Object.fromEntries(all.map((r) => [r.lineageId, r]));
			byBranch = Object.fromEntries(all.map((r) => [r.branchId, r]));
			loaded = true;
		})();
	});

	// ---- the switches -----------------------------------------------------

	let treatment = $state<'nest' | 'annexe'>('nest');
	let recipe = $state<'pizza' | 'danDan'>('pizza');
	let unfolded = $state(true);
	/** What the pizza's dough line points at — the three states of a Component. */
	let dough = $state<'resolves' | 'absent' | 'noYield'>('resolves');

	const root = $derived(byBranch[recipe === 'pizza' ? PIZZA : DAN_DAN]);

	/**
	 * The fabricated half. On the pizza, line 0 — `Dough for 2 pizzas`, whose
	 * Reading a person corrected to `500 g pizza dough` — is the Component,
	 * and the switch decides what it finds. On Dan Dan Noodles, line 7 —
	 * `3 tbsp chilli oil` — is the Component, and inside the Chilli Oil line 7
	 * points back at Dan Dan Noodles, which is the cycle.
	 */
	const composition = $derived<Composition>({
		[PIZZA]: {
			0:
				dough === 'resolves'
					? LINEAGE.neapolitan
					: dough === 'absent'
						? LINEAGE.absent
						: LINEAGE.versatile,
		},
		[DAN_DAN]: { 7: LINEAGE.chilliOil },
		[CHILLI_OIL]: { 7: LINEAGE.danDan },
	});
</script>

<svelte:head><title>#50 · Components — two treatments</title></svelte:head>

<div class="pb-40">
	{#if !loaded || !root}
		<p class="px-gutter py-6 text-body text-ink-2">Reading the library…</p>
	{:else}
		<PrototypeRecipe {root} {library} {composition} {treatment} {unfolded} />
	{/if}
</div>

<!-- The rig's own bar. Not part of either treatment. -->
<div class="fixed inset-x-0 bottom-0 z-20 border-t border-rule bg-card px-3 pt-2 pb-safe shadow-lg">
	<div class="mx-auto flex max-w-2xl flex-wrap gap-x-4 gap-y-2 text-read">
		<div class="flex items-center gap-1">
			<span class="text-label text-ink-2 uppercase">Treatment</span>
			{#each [['nest', 'A · Nest'], ['annexe', 'B · Annexe']] as const as [value, label] (value)}
				<button
					type="button"
					class="rounded-sm border px-2 py-1 {treatment === value
						? 'border-accent bg-accent text-on-accent'
						: 'border-rule text-ink'}"
					onclick={() => (treatment = value)}
				>
					{label}
				</button>
			{/each}
		</div>

		<div class="flex items-center gap-1">
			<span class="text-label text-ink-2 uppercase">Recipe</span>
			{#each [['pizza', 'Pizza'], ['danDan', 'Dan Dan']] as const as [value, label] (value)}
				<button
					type="button"
					class="rounded-sm border px-2 py-1 {recipe === value
						? 'border-accent bg-accent text-on-accent'
						: 'border-rule text-ink'}"
					onclick={() => (recipe = value)}
				>
					{label}
				</button>
			{/each}
		</div>

		<div class="flex items-center gap-1">
			<span class="text-label text-ink-2 uppercase">Component</span>
			<button
				type="button"
				class="rounded-sm border px-2 py-1 {unfolded
					? 'border-accent bg-accent text-on-accent'
					: 'border-rule text-ink'}"
				onclick={() => (unfolded = !unfolded)}
			>
				{unfolded ? 'open' : 'closed'}
			</button>
		</div>

		<div class="flex items-center gap-1">
			<span class="text-label text-ink-2 uppercase">The dough</span>
			{#each [['resolves', 'is here'], ['absent', 'is missing'], ['noYield', 'has no Yield']] as const as [value, label] (value)}
				<button
					type="button"
					disabled={recipe !== 'pizza'}
					class="rounded-sm border px-2 py-1 {dough === value
						? 'border-accent bg-accent text-on-accent'
						: 'border-rule text-ink'} {recipe !== 'pizza' ? 'opacity-40' : ''}"
					onclick={() => (dough = value)}
				>
					{label}
				</button>
			{/each}
		</div>
	</div>
</div>
