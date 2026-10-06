<!--
	One recipe on the shelf: a **noren** — a dyed or photographed ground with a
	cloth band at the hem carrying the recipe's name.

	Chosen by Aurélien on 2026-08-28 from three shelves of the real 86 recipes;
	the reasoning is on issue #62. What the choice settles is that a recipe with
	a Photograph and one without are the *same object* rather than two
	treatments: a Cover already carries its title on a band at its hem (#46), so
	a photograph is given the same band rather than a caption underneath. Card
	height is identical either way, which is why a third of the shelf carrying
	no Photograph does not read as ragged (ADR 0027).

	No Kitchen name appears here, and none could: the shelf Operation sends
	neither the name nor the id.
-->
<script lang="ts">
	import Cover from '$lib/cover/Cover.svelte';
	import { matchedLabel, fallbackLanguage, type Entry } from './matched';

	interface Props {
		entry: Entry;
	}

	let { entry }: Props = $props();

	const label = $derived(matchedLabel(entry.matched));
	const fallback = $derived(fallbackLanguage(entry));
</script>

<li>
	<a href="/recipes/{entry.branch_id}" class="brightens block">
		<!--
			3:4 rather than a fixed height, so two tiles fit any phone's width and
			the shelf stays two columns of the same thing.
		-->
		<div class="relative overflow-hidden rounded-sm" style="aspect-ratio: 3 / 4">
			{#if entry.main_photo}
				<img
					src="/api/photographs/{entry.main_photo}/card"
					alt=""
					class="absolute inset-0 h-full w-full object-cover"
					loading="lazy"
				/>
				<!--
					The photograph's own band. Sumi at 84% rather than flat: the
					picture continues behind the name instead of being cut off by
					it, which is what makes this read as cloth over a photograph
					rather than a caption bar.

					Mixed from the ink token rather than written out, so the one
					generated stylesheet stays the only place Kamosu's colours
					are decided (AGENTS.md, "Design tokens").
				-->
				<div
					class="absolute inset-x-0 bottom-0 px-gutter py-3"
					style="background: color-mix(in srgb, var(--color-ink) 84%, transparent)"
				>
					<div class="line-clamp-3 font-display text-tile-title font-semibold text-on-accent">
						{entry.title}
					</div>
				</div>
			{:else}
				<Cover
					lineageId={entry.lineage_id}
					title={entry.title}
					height="100%"
					titleClass="text-tile-title line-clamp-3"
				/>
			{/if}

			{#if fallback}
				<!--
					Beni, which the visual identity names as the Language badge's
					colour. It marks that this is not the Language you read in —
					and it is the whole of the mark, because the recipe is shown
					either way (ADR 0006).

					The chip carries the Language's code, which is what fits on
					168px of cloth; the whole sentence is there for anyone
					listening to the screen rather than looking at it, since
					"FR" read aloud is not a fallback anyone would understand.
				-->
				<span
					class="absolute top-2 right-2 rounded-sm bg-support px-1 py-1 text-label text-ground uppercase"
				>
					<span aria-hidden="true">{entry.language}</span>
					<span class="sr-only">{fallback}</span>
				</span>
			{/if}
		</div>
	</a>

	{#if label && entry.matched}
		<p class="mt-1 text-read text-ink-2">
			<span class="block text-label uppercase">{label}</span>
			{entry.matched.line}
		</p>
	{/if}
</li>
