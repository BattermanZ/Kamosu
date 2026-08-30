<!--
	Home: the computed shelves that answer *show me something* (#64, ADR 0011,
	ADR 0027).

	The first screen is a suggestion rather than a search box. Everything on it
	is worked out from recipes and cookings that already exist — *Recipes* is
	where you go when you know what you are looking for, and this is where you
	go when you do not.

	**The arrangement is Aurélien's, chosen on 2026-08-30 from four full-screen
	options drawn over the real 86-recipe library; the reasoning is on #64.** It
	takes the labelled section from one option and the sideways rail from
	another. What the choice settles:

	- **A shelf is a section**, headed the way every other section in the app is
	  headed, with the count beside it linking into the library. Home is the
	  library's own furniture rather than a second visual system next to it.
	- **A section carries a rail, not a grid.** The card at the right edge is
	  cut in half on purpose: a peeking card is the only thing that says *there
	  is more this way* without a control saying it, and it is what puts two
	  whole shelves above the fold and all four within one flick.

	The cards themselves are #62's, unchanged — `Tile`, the same component the
	library's shelf draws, taking the same shape the same Operation family
	answers in. A recipe is one object across both screens, not two treatments
	of one.

	**Which shelves exist is the Core's answer, never this screen's.** An empty
	shelf never arrives, so there is no case here where a heading has to decide
	whether to draw itself — and an instance with nothing on its shelf sends no
	shelves at all, which is what lets the empty state be said once instead of
	four times.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import type { HomeShelvesOutput } from '$lib/api/catalogue';
	import Tile from './recipes/Tile.svelte';
	import AddOrImport from '$lib/AddOrImport.svelte';

	interface Props {
		/** The answer `home_shelves` gave. Asked for by the route, so signing in and reading Home are one round trip. */
		home: HomeShelvesOutput;
	}

	let { home }: Props = $props();

	type ShelfName = HomeShelvesOutput['shelves'][number]['name'];

	/**
	 * What each shelf is called, and the sentence under it. The sentence does
	 * the explaining so the heading can stay two words, with the longer phrase
	 * carrying the detail underneath.
	 *
	 * One entry per shelf rather than a map of headings beside a switch of
	 * sentences: the two always change together, and two lists keyed by the same
	 * four names are two chances to add a shelf to one of them.
	 *
	 * The 30 in *quick tonight* is the Core's, carried in the answer. Writing it
	 * into the phrase file instead would let the sentence a reader believes
	 * drift away from the line that actually chose the recipes underneath it.
	 */
	const shelf: Record<ShelfName, { heading: () => string; why: () => string }> = {
		cooked_most: {
			heading: m.home_shelf_cooked_most,
			why: m.home_shelf_cooked_most_why,
		},
		quick_tonight: {
			heading: m.home_shelf_quick_tonight,
			why: () => m.home_shelf_quick_tonight_why({ minutes: home.quick_tonight_minutes }),
		},
		never_cooked: {
			heading: m.home_shelf_never_cooked,
			why: m.home_shelf_never_cooked_why,
		},
		recently_opened: {
			heading: m.home_shelf_recently_opened,
			why: m.home_shelf_recently_opened_why,
		},
	};
</script>

<div class="pt-6 pb-tabbar">
	<div class="mx-auto max-w-2xl px-gutter">
		<h1 class="font-display text-title font-semibold">{m.home_title()}</h1>
		<p class="mt-1 text-read text-ink-2">{m.home_blurb()}</p>
	</div>

	{#if home.shelves.length === 0}
		<!--
			A brand-new instance, and the first thing anybody ever sees. One
			sentence saying why there is nothing here, and the two things that
			were going to happen next anyway — the same answer *nothing found*
			gives on the Recipes screen, because it is the same situation: you do
			not have it yet, and adding it was what you came to do (ADR 0027).
		-->
		<div class="mx-auto mt-8 max-w-2xl px-gutter">
			<div class="rounded-sm border border-dashed border-rule bg-ground-2 p-6">
				<h2 class="font-display text-shelf-heading font-semibold">{m.home_empty_title()}</h2>
				<p class="mt-2 text-read text-ink-2">{m.home_empty_why()}</p>
				<!--
					The two acts themselves, not two links to a screen that would
					be just as empty. Nothing is known about a title here — unlike
					Recipes, nobody has typed one — so the offer asks for one.
				-->
				<div class="mt-4">
					<AddOrImport />
				</div>
			</div>
		</div>
	{:else}
		{#each home.shelves as on (on.name)}
			<section class="mt-8">
				<div class="mx-auto max-w-2xl px-gutter">
					<div class="mb-1 flex items-baseline justify-between gap-3 border-b border-rule pb-2">
						<h2 class="text-label font-medium text-accent uppercase">
							{shelf[on.name].heading()}
						</h2>
						<!--
							How many cards are on this rail, so a thumb knows how far it
							has to flick. It counts what is actually here and says
							nothing about how many recipes *could* have qualified: the
							Core sends a shelf's worth, not a shelf's total, and a
							number claiming to be the total would be wrong the moment a
							shelf was longer than one.

							It is deliberately not a link. The obvious destination would
							be Recipes — but Recipes filters by Kitchen and by *mine*,
							not by *cooked most*, so a link there would land you on the
							whole library having promised this shelf.
						-->
						<span class="text-read text-ink-2">{on.recipes.length}</span>
					</div>
					<p class="text-read text-ink-2">{shelf[on.name].why()}</p>
				</div>
				<!--
					The gutter is padding on the rail rather than on the page, so the
					first card lines up with its heading and the last one can still be
					pushed clear of the right edge. Snap points mean a flick lands on
					a card rather than between two.

					`scroll-pl-gutter` is what keeps that alignment true: a snap point
					is measured from the scroller's edge, not from its padding, so
					without it the rail silently scrolls the gutter away at rest and
					the first card sits flush against the side of the phone while its
					own heading is still indented.
				-->
				<ul
					class="mx-auto mt-3 flex max-w-2xl snap-x snap-mandatory scroll-pl-gutter gap-3 overflow-x-auto px-gutter [&>li]:w-[var(--tile-w)] [&>li]:shrink-0 [&>li]:snap-start"
				>
					{#each on.recipes as entry (entry.lineage_id)}
						<Tile {entry} />
					{/each}
				</ul>
			</section>
		{/each}
	{/if}
</div>
