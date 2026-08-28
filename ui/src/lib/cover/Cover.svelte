<!--
	A Cover: what leads a recipe with no Main Photo (#46; CONTEXT.md, "Cover").

	No ADR governs this — the Cover is a vocabulary entry, not a decision with
	alternatives that had to be weighed. What it looks like was Aurélien's
	choice, and the reasoning is recorded on issue #46 rather than here, since
	it is taste rather than architecture.

	Not a placeholder for a missing photograph — a recipe needs only a title,
	and 27 of the 86 recipes in the real corpus have no photograph at all. So
	this is drawn as something a recipe *has*, not as a gap where something
	should be.

	It carries the title itself, on the band at the hem. That is deliberate: a
	Cover with no words on it is a coloured rectangle, and the whole point is
	that a shelf of these is readable.

	What it looks like comes entirely from `coverFor`, which reads the Lineage
	id and nothing else. This component owns only how that is drawn. How the
	recipe *page* frames its hero is #81's decision, not this component's.

	Everything sizes in CSS rather than in pixels worked out here, so the same
	component serves a 282px hero and a 168px shelf tile without being told
	which it is.
-->
<script lang="ts">
	import { coverFor } from './cover';

	interface Props {
		/** The recipe's Lineage id — the only thing the drawing is derived from. */
		lineageId: string;
		/** The recipe's title, set on the hem band. */
		title: string;
		/** Any CSS length. Defaults to the identity's recipe-hero height. */
		height?: string;
		/** The type utility for the title. A shelf card passes something smaller. */
		titleClass?: string;
	}

	let {
		lineageId,
		title,
		height = 'var(--hero-h)',
		titleClass = 'text-title'
	}: Props = $props();

	const cover = $derived(coverFor(lineageId));
</script>

<!--
	The Cover is decoration carrying a title that every screen showing it also
	sets as real text, so it is hidden from assistive technology rather than
	announced twice.
-->
<div
	class="relative flex flex-col overflow-hidden rounded-sm"
	style="height: {height}; background: {cover.ground}"
	aria-hidden="true"
>
	<!--
		The shape is a square whose side is a fraction of the Cover's height,
		centred on a point given as fractions of the Cover's own box. Placing it
		by its centre and pulling it back half its own size is what lets a Cover
		bleed off an edge without any of the arithmetic knowing the pixel size.
	-->
	<div
		class="absolute aspect-square -translate-x-1/2 -translate-y-1/2"
		style="left: {(cover.centreX * 100).toFixed(2)}%;
		       top: {(cover.centreY * 100).toFixed(2)}%;
		       height: {(cover.scale * 100).toFixed(2)}%"
	>
		<svg width="100%" height="100%" viewBox="0 0 100 100" class="block">
			<g transform="rotate({cover.rotation.toFixed(1)} 50 50)">
				{#each cover.shape.body as d (d)}
					<path {d} fill={cover.tone} />
				{/each}
				{#each cover.shape.detail as d (d)}
					<path {d} fill={cover.ground} fill-opacity="0.5" />
				{/each}
			</g>
		</svg>
	</div>

	<div class="relative mt-auto px-gutter py-3" style="background: {cover.band}">
		<div class="font-display font-semibold text-on-accent {titleClass}">
			{title}
		</div>
	</div>
</div>
