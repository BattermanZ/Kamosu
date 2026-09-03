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
	/** What went wrong, said out loud. A rig that hangs on one line tells nobody anything. */
	let failed = $state<string | null>(null);

	$effect(() => {
		let current = true;
		void (async () => {
			const branches = [PIZZA, DAN_DAN, CHILLI_OIL, NEAPOLITAN, VERSATILE];
			// Settled one by one rather than through `Promise.all`, which throws
			// away four good answers because a fifth failed — and then says
			// nothing about which.
			const read = await Promise.allSettled(
				branches.map((branch_id) => kamosu.getRecipe({ branch_id })),
			);
			if (!current) return;
			const broken = read.flatMap((r, index) =>
				r.status === 'rejected'
					? [`${branches[index]}: ${String(r.reason?.message ?? r.reason)}`]
					: [],
			);
			const all = read.flatMap((r) => (r.status === 'fulfilled' ? [held(r.value)] : []));
			library = Object.fromEntries(all.map((r) => [r.lineageId, r]));
			byBranch = Object.fromEntries(all.map((r) => [r.branchId, r]));
			failed = broken.length ? broken.join(' · ') : null;
			loaded = true;
		})();
		return () => {
			current = false;
		};
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

<!--
	The rig's own bar. Not part of either treatment.

	At the TOP, sticky, which is where #81's rig put its four switches. It was
	at the foot to begin with and that was wrong on a phone: the app's tab bar
	is fixed over the bottom 61px and swallowed the last row of switches
	whole — `bottom-tabbar` lifts a thing by 48px, which is not enough.
-->
<div class="sticky top-0 z-20 border-b border-rule bg-card px-3 py-2 shadow-sm">
	<div class="mx-auto grid max-w-2xl gap-y-1 text-read">
		{#snippet row(label: string, children: import('svelte').Snippet)}
			<div class="flex items-center gap-1">
				<span class="w-20 shrink-0 text-label text-ink-2 uppercase">{label}</span>
				{@render children()}
			</div>
		{/snippet}

		{#snippet pill(label: string, on: boolean, press: () => void, off = false)}
			<button
				type="button"
				disabled={off}
				class="rounded-sm border px-2 py-1 {on
					? 'border-accent bg-accent text-on-accent'
					: 'border-rule text-ink'} {off ? 'opacity-40' : ''}"
				onclick={press}
			>
				{label}
			</button>
		{/snippet}

		{#snippet treatmentRow()}
			{@render pill('A · Nest', treatment === 'nest', () => (treatment = 'nest'))}
			{@render pill('B · Annexe', treatment === 'annexe', () => (treatment = 'annexe'))}
		{/snippet}
		{@render row('Treatment', treatmentRow)}

		{#snippet recipeRow()}
			{@render pill('Pizza', recipe === 'pizza', () => (recipe = 'pizza'))}
			{@render pill('Dan Dan', recipe === 'danDan', () => (recipe = 'danDan'))}
			<span class="w-2"></span>
			{@render pill(unfolded ? 'open' : 'closed', unfolded, () => (unfolded = !unfolded))}
		{/snippet}
		{@render row('Recipe', recipeRow)}

		{#snippet doughRow()}
			{@render pill(
				'is here',
				dough === 'resolves',
				() => (dough = 'resolves'),
				recipe !== 'pizza',
			)}
			{@render pill('is missing', dough === 'absent', () => (dough = 'absent'), recipe !== 'pizza')}
			{@render pill('no Yield', dough === 'noYield', () => (dough = 'noYield'), recipe !== 'pizza')}
		{/snippet}
		{@render row('The dough', doughRow)}
	</div>
</div>

{#if !loaded}
	<p class="px-gutter py-6 text-body text-ink-2">Reading the library…</p>
{:else if !root}
	<div class="px-gutter py-6" role="alert">
		<p class="text-body text-support">The library would not open.</p>
		<p class="mt-2 text-read text-ink-2">{failed ?? 'No recipe came back.'}</p>
		<p class="mt-3 text-read text-ink-2">
			If that says <em>unauthorized</em>, this browser is not signed in to the dev instance — open
			<a href="/" class="underline">the app</a>, sign in, then come back.
		</p>
	</div>
{:else}
	{#if failed}
		<p class="border-b border-rule px-gutter py-2 text-read text-support" role="alert">
			Some of the cast would not open: {failed}
		</p>
	{/if}
	<PrototypeRecipe {root} {library} {composition} {treatment} {unfolded} />
{/if}
