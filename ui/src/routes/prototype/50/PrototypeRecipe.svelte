<!--
	PROTOTYPE — #50 Components. Throwaway.

	The recipe page as #81 left it — the 320px hero with the title standing on
	it over the indigo wash, the full-bleed ruled meta strip, the hairline-ruled
	Ingredient rows led by a small square, the Method's indigo number in a
	narrow column, the quiet uppercase Sections — with ONE addition drawn on it
	two different ways.

	What is being chosen is only that addition. Everything else here is copied
	from `ui/src/routes/recipes/[branchId]/Recipe.svelte` so that neither
	treatment is flattered by a page that differs from the real one.

	The Divergence, the Threshold, the inline Reading corrector and the Thread
	link are left out: they are settled, they are not what is being chosen, and
	none of the recipes in this rig has a second Branch to show them against.
-->
<script lang="ts">
	import Cover from '$lib/cover/Cover.svelte';
	import { factor, scaled, share, type Held, type Slot, type Composition } from './composition';

	interface Props {
		root: Held;
		library: Record<string, Held>;
		composition: Composition;
		treatment: 'nest' | 'annexe';
		unfolded: boolean;
	}

	let { root, library, composition, treatment, unfolded }: Props = $props();

	/** One Component, worked out: what it points at, how much, and what went wrong. */
	type Component = {
		key: string;
		lineIndex: number;
		text: string;
		reading: Slot;
		inner: Held | undefined;
		how: number | null;
		/** Unfolding stopped here because this recipe is already open above. */
		cycle: boolean;
		/** The Components inside this one. */
		within: Component[];
	};

	/**
	 * Unfolding, with the repeat guard ADR 0008 asks for: a loop is never
	 * refused, it stops at the first repeat and says so. `seen` is the chain of
	 * Lineages open above this point, not everything ever visited — two
	 * different lines may name the same dough without that being a cycle.
	 */
	function unfold(recipe: Held, seen: string[], path: string): Component[] {
		const lines = composition[recipe.branchId] ?? {};
		return Object.entries(lines).map(([index, lineage]) => {
			const lineIndex = Number(index);
			const reading = recipe.readings[lineIndex] ?? null;
			const inner = library[lineage];
			const cycle = Boolean(inner) && seen.includes(lineage);
			return {
				key: `${path}.${lineIndex}`,
				lineIndex,
				text: recipe.ingredients[lineIndex]?.text ?? '',
				reading,
				inner,
				how: inner ? factor(reading, inner.yield) : null,
				cycle,
				within: inner && !cycle ? unfold(inner, [...seen, lineage], `${path}.${lineIndex}`) : [],
			};
		});
	}

	const components = $derived(unfold(root, [root.lineageId], 'r'));
	const byLine = $derived(new Map(components.map((c) => [c.lineIndex, c])));

	/** Which Components are open. Closed by default, which is ADR 0008's word. */
	let open = $state(new Set<string>());
	$effect(() => {
		// The rig's own switch, so a whole treatment can be seen at once.
		open = unfolded ? new Set(everyKey(components)) : new Set();
	});
	function everyKey(list: Component[]): string[] {
		return list.flatMap((c) => [c.key, ...everyKey(c.within)]);
	}
	function toggle(key: string) {
		const next = new Set(open);
		if (next.has(key)) next.delete(key);
		else next.add(key);
		open = next;
	}

	/**
	 * The one line beneath a Component's written line: which recipe it names,
	 * and how much of it. The same quiet slot every other Ingredient Line uses
	 * for its conversion (#49, ADR 0016) — a Component says something different
	 * in it, not something extra beside it.
	 */
	function beneath(c: Component): string {
		if (!c.inner) return 'Kamosu does not have this recipe.';
		if (c.cycle) return `${c.inner.title} is already open above — Kamosu stops here.`;
		if (c.how === null) {
			return `${c.inner.title} — Kamosu could not work out how much, so this is the recipe as written.`;
		}
		return `${c.inner.title} · ${share(c.how)}`;
	}

	/** Openable only where there is something to open. */
	const openable = (c: Component) => Boolean(c.inner) && !c.cycle;

	/**
	 * B only: every Component's method, in the order the page meets them, so
	 * the foot of the page reads down in the same order the ingredient list does.
	 */
	function methods(list: Component[]): Component[] {
		return list.flatMap((c) =>
			openable(c) && open.has(c.key) && c.inner!.steps.length ? [c, ...methods(c.within)] : [],
		);
	}
	const annexes = $derived(treatment === 'annexe' ? methods(components) : []);

	/**
	 * A Step's number: counted over the rows so a Section takes none, and
	 * worked out from the list itself rather than by a running counter — a
	 * counter held in a `{@const}` is re-entered on every reactive redraw and
	 * silently skips numbers.
	 */
	function numberOf(steps: Held['steps'], index: number): number {
		return steps.slice(0, index + 1).filter((item) => item.kind !== 'section').length;
	}
</script>

<div class="mx-auto max-w-2xl pb-tabbar">
	<!-- The hero: the Main Photo, or the Cover for a recipe that has none (#46, #81). -->
	<div class="relative overflow-hidden">
		{#if root.photo}
			<img
				src="/api/photographs/{root.photo}/page"
				alt=""
				class="block w-full object-cover"
				style="height: var(--hero-h)"
			/>
			<div class="pointer-events-none absolute inset-x-0 bottom-0 wash"></div>
		{:else}
			<Cover lineageId={root.lineageId} title={root.title} band={false} />
		{/if}
		<div class="absolute inset-x-0 bottom-0 px-gutter pt-8 pb-4">
			<h1 class="mt-1 font-display text-title font-semibold text-on-accent">{root.title}</h1>
		</div>
	</div>

	<!-- The meta: one full-bleed strip, three cells, hairlines between. -->
	{#if root.prep !== null || root.cook !== null || root.yield}
		<div class="mt-4 flex border-y border-rule">
			{#if root.prep !== null}
				<div class="flex-1 px-2 py-3 text-center">
					<b class="block font-display text-panel-figure font-semibold">{root.prep}</b>
					<span class="mt-1 block text-label text-ink-2 uppercase">min prep</span>
				</div>
			{/if}
			{#if root.cook !== null}
				<div class="flex-1 border-l border-rule px-2 py-3 text-center first:border-l-0">
					<b class="block font-display text-panel-figure font-semibold">{root.cook}</b>
					<span class="mt-1 block text-label text-ink-2 uppercase">min cook</span>
				</div>
			{/if}
			{#if root.yield}
				<div class="flex-1 border-l border-rule px-2 py-3 text-center first:border-l-0">
					<b class="block font-display text-panel-figure font-semibold">{root.yield.amount}</b>
					<span class="mt-1 block text-label text-ink-2 uppercase">{root.yield.noun}</span>
				</div>
			{/if}
		</div>
	{/if}

	<!--
		ONE INGREDIENT LINE. Identical for a Component and an ordinary line
		above the subordinate slot: the written line is the truth (ADR 0002),
		so a Component is not a different kind of row. What differs is the
		colour of the square in front of it — matcha rather than indigo, the
		token `--color-support-2` was reserved for exactly this — and that
		there is something to open.
	-->
	{#snippet line(text: string, at: number)}
		{@const c = byLine.get(at)}
		<li class="border-b border-rule py-3">
			{#if c}
				{@render componentRow(c)}
			{:else}
				<div class="flex gap-3">
					<span class="ingredient-marker shrink-0 bg-accent" aria-hidden="true"></span>
					<div class="min-w-0 flex-1">
						<span class="block text-line">{text}</span>
					</div>
				</div>
			{/if}
		</li>
	{/snippet}

	<!-- A Component's row, and — where it is open — its unfolding. -->
	{#snippet componentRow(c: Component)}
		<div class="flex gap-3">
			<span class="ingredient-marker shrink-0 bg-support-2" aria-hidden="true"></span>
			<div class="min-w-0 flex-1">
				{#if openable(c)}
					<button
						type="button"
						class="flex w-full items-start gap-2 text-left"
						aria-expanded={open.has(c.key)}
						onclick={() => toggle(c.key)}
					>
						<span class="min-w-0 flex-1">
							<span class="block text-line">{c.text}</span>
							<span class="block text-read text-support-2">{beneath(c)}</span>
						</span>
						<svg
							viewBox="0 0 24 24"
							aria-hidden="true"
							class="mt-1 h-4 w-4 shrink-0 text-support-2 transition-transform"
							style={open.has(c.key) ? 'transform: rotate(90deg)' : ''}
							fill="none"
							stroke="currentColor"
							stroke-width="1.75"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<path d="m9 5 7 7-7 7" />
						</svg>
					</button>
				{:else}
					<span class="block text-line">{c.text}</span>
					<span class="block text-read text-ink-2">{beneath(c)}</span>
				{/if}

				{#if openable(c) && open.has(c.key)}
					{@render inside(c)}
				{/if}
			</div>
		</div>
	{/snippet}

	<!--
		THE UNFOLDING, drawn two ways.

		A · THE NEST — the whole of the inner recipe lives inside the row that
		names it: its Ingredient Lines, then a quiet rule, then its own Steps.
		The outer Method never mentions it, so reading the method top to bottom
		you meet only this recipe's own steps.

		B · THE ANNEXE — only the inner Ingredient Lines unfold here, so the
		ingredient list stays a list. Its Steps live at the foot of the page,
		under a heading of their own, in the same grammar as the page's other
		Sections.

		Both hold the unfolded block on the recessed ground behind a matcha
		rule, so it is visibly another recipe's inside without being a card.
	-->
	{#snippet inside(c: Component)}
		{@const inner = c.inner!}
		<div class="mt-3 border-l-2 border-support-2 bg-ground-2 py-1 pl-3">
			<ul>
				{#each inner.ingredients as item, index (index)}
					{#if item.kind === 'section'}
						<li class="border-b border-rule py-3 pb-1">
							<h4 class="font-display text-label text-ink-2 uppercase">{item.text}</h4>
						</li>
					{:else}
						{@const nested = (composition[inner.branchId] ?? {})[index]}
						{#if nested !== undefined}
							{@const child = c.within.find((w) => w.lineIndex === index)!}
							<li class="border-b border-rule py-3 last:border-b-0">
								{@render componentRow(child)}
							</li>
						{:else}
							<li class="flex gap-3 border-b border-rule py-3 last:border-b-0">
								<span class="ingredient-marker shrink-0 bg-accent" aria-hidden="true"></span>
								<div class="min-w-0 flex-1">
									<span class="block text-line">{item.text}</span>
									{#if scaled(inner.readings[index] ?? null, c.how)}
										<span class="block text-read text-ink-2">
											{scaled(inner.readings[index] ?? null, c.how)}
										</span>
									{/if}
								</div>
							</li>
						{/if}
					{/if}
				{/each}
			</ul>

			{#if inner.steps.length}
				{#if treatment === 'nest'}
					<!-- A: the inner method, here, kept plainly separate from the outer one. -->
					<h4
						class="mt-4 mb-1 border-t border-rule pt-3 font-display text-label font-semibold text-support-2 uppercase"
					>
						Its own method
					</h4>
					{@render method(inner)}
				{:else}
					<!-- B: the inner method is at the foot, and the row says where. -->
					<p class="py-3 text-read text-support-2">
						<a href="#annexe-{c.key}" class="underline underline-offset-2">
							Its method is at the foot of the page ↓
						</a>
					</p>
				{/if}
			{/if}
		</div>
	{/snippet}

	<!-- The Method's grammar: an indigo number in a narrow column, a hairline under each (#81). -->
	{#snippet method(recipe: Held)}
		<ol>
			{#each recipe.steps as item, index (index)}
				{#if item.kind === 'section'}
					<li class="border-b border-rule py-4 pb-1">
						<h4 class="font-display text-label text-ink-2 uppercase">{item.text}</h4>
					</li>
				{:else}
					<li class="flex gap-3 border-b border-rule py-3 last:border-b-0">
						<span class="w-6 shrink-0 font-display text-line font-semibold text-accent">
							{numberOf(recipe.steps, index)}
						</span>
						<p class="min-w-0 flex-1 text-body">{item.text}</p>
					</li>
				{/if}
			{/each}
		</ol>
	{/snippet}

	<!-- Ingredients ------------------------------------------------------ -->
	<h2 class="mx-gutter mt-8 mb-2 font-display text-label font-semibold text-accent uppercase">
		Ingredients
	</h2>
	<ul class="px-gutter">
		{#each root.ingredients as item, index (index)}
			{#if item.kind === 'section'}
				<li class="border-b border-rule py-4 pb-1">
					<h3 class="font-display text-label text-ink-2 uppercase">{item.text}</h3>
				</li>
			{:else}
				{@render line(item.text, index)}
			{/if}
		{/each}
	</ul>

	<!-- Method ----------------------------------------------------------- -->
	<h2 class="mx-gutter mt-8 mb-2 font-display text-label font-semibold text-accent uppercase">
		Method
	</h2>
	<div class="px-gutter">{@render method(root)}</div>

	<!--
		B ONLY — the annexe. A Component's Steps at the foot of the page, under
		a heading in the page's own Section grammar but in matcha, so it reads
		as belonging to the Component rather than to this recipe's method.
	-->
	{#each annexes as c (c.key)}
		<div id="annexe-{c.key}" class="mt-8">
			<h2 class="mx-gutter mb-1 font-display text-label font-semibold text-support-2 uppercase">
				{c.inner!.title} · its own method
			</h2>
			<p class="mx-gutter mb-2 text-read text-ink-2">{beneath(c)}</p>
			<div class="border-l-2 border-support-2 bg-ground-2 px-gutter py-1">
				{@render method(c.inner!)}
			</div>
		</div>
	{/each}

	{#if root.note}
		<div class="mx-gutter mt-6 border-l-2 border-accent py-1 pl-4 text-body whitespace-pre-wrap">
			{root.note}
		</div>
	{/if}
</div>
