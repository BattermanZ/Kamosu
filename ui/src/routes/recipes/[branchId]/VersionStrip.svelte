<!--
	The switch between the versions of one recipe (#131, Aurélien's screen
	choice 1 of 24 September 2026: a strip of every version).

	ADR 0014 designed a threshold for exactly two Branches: you stood in yours
	and crossed into theirs. Under ADR 0041 three or four versions of one dish
	in a family Kitchen is ordinary — yours, your mother's, your brother's, your
	vegetarian one — so every version is a chip across the top, and tapping one
	opens it. Each still has an address of its own, so the page is always the
	recipe it says it is.

	WHAT A MARK COMPARES AGAINST: YOUR OWN. Whatever version is open, the lines
	marked are the ones it does not share with the recipe in your own Cookbook
	— the question a reader has is *how is theirs different from mine*, never
	*how is Luc's different from Hélène's*. On your own there is nothing to
	mark, and the strip says how to see the others instead. A reader with no
	version of their own sees no marks at all.

	The band keeps the threshold's colours and its split curtain: the page's
	paper still changes with whose recipe you are reading.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { branchLabel, type LabelledBranch } from '$lib/cookbook';

	interface Props {
		/** Every version of this recipe the reader may see, oldest first. */
		versions: (LabelledBranch & { branch_id: string })[];
		/** The version open on this page. */
		current: string;
		/**
		 * Lines the open version does not share with yours, and the name of
		 * whose it is; absent where there is nothing to compare — on your own,
		 * or where you have none.
		 */
		compared?: { unshared: number; with: string };
		/** Whether the page is your own recipe, so the strip says how to see the others. */
		onYours: boolean;
		marks: boolean;
		toggleMarks: () => void;
	}

	let { versions, current, compared, onYours, marks, toggleMarks }: Props = $props();
</script>

<div class="strip bg-[var(--whose)] px-gutter pt-3 text-on-accent">
	<p class="text-label uppercase opacity-60">
		{m.switch_versions({ count: versions.length })}
	</p>
	<!-- Scrolls sideways past four, with the gutter still there at the end. -->
	<ul class="-mx-gutter mt-2 flex gap-2 overflow-x-auto px-gutter pb-1">
		{#each versions as version (version.branch_id)}
			{@const label = branchLabel(version)}
			{@const here = version.branch_id === current}
			<li class="shrink-0">
				<a
					href="/recipes/{version.branch_id}"
					aria-current={here ? 'page' : undefined}
					class="flex min-h-12 flex-col justify-center rounded-sm border px-3 py-1 {here
						? 'border-card bg-card text-ink'
						: 'border-on-accent/35 text-on-accent'}"
				>
					<span class="font-display text-body font-semibold">{label.name}</span>
					<span class="text-label opacity-75">{label.whose}</span>
				</a>
			</li>
		{/each}
	</ul>
</div>

{#if compared && compared.unshared === 0}
	<!-- Nothing to mark, so nothing to hide: a variation started unchanged. -->
	<p class="border-b border-rule bg-ground px-gutter py-2 text-read text-ink-2">
		{m.divergence_unshared_none()}
	</p>
{:else if compared}
	<div class="flex items-center gap-3 border-b border-rule bg-ground px-gutter py-2">
		<p class="flex-1 text-read text-ink-2">
			{#if !marks}
				{m.divergence_put_away({ count: compared.unshared })}
			{:else if compared.unshared === 1}
				{m.divergence_unshared_one({ kitchen: compared.with })}
			{:else}
				{m.divergence_unshared({ count: compared.unshared, kitchen: compared.with })}
			{/if}
		</p>
		<button
			type="button"
			onclick={toggleMarks}
			class="rounded-sm border border-rule bg-card px-3 py-1 text-read text-accent"
		>
			{marks ? m.divergence_hide() : m.divergence_show()}
		</button>
	</div>
{:else if onYours}
	<p class="border-b border-rule bg-ground px-gutter py-2 text-read text-ink-2">
		{m.switch_on_yours()}
	</p>
{/if}

<style>
	/* The noren: a split curtain cut into the band's own bottom edge, so it
	   hangs below the band without ever sitting over the page. */
	.strip {
		padding-bottom: 16px;
		-webkit-mask-image:
			linear-gradient(#000 0 0),
			repeating-linear-gradient(90deg, #000 0 26px, transparent 26px 32px);
		-webkit-mask-size:
			100% calc(100% - 8px),
			100% 8px;
		-webkit-mask-position:
			0 0,
			0 100%;
		-webkit-mask-repeat: no-repeat;
		mask-image:
			linear-gradient(#000 0 0),
			repeating-linear-gradient(90deg, #000 0 26px, transparent 26px 32px);
		mask-size:
			100% calc(100% - 8px),
			100% 8px;
		mask-position:
			0 0,
			0 100%;
		mask-repeat: no-repeat;
	}
</style>
