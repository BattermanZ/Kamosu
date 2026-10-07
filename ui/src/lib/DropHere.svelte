<!--
	What a small place wears while a file is held over it (ADR 0044, #205): a
	Step, a cooking's photographs, the Crouton import. Aurélien chose this on
	7 October 2026 from three drawn on the real screens (option 3), over the
	look Recipes has, whose words do not fit a Step, and over that look in one
	line, which blanked each Step the pointer passed.

	**Nothing is covered.** It is drawn twice in a place, once for each part:

	- `part="frame"`, inside the place, which is `relative`: a dashed line
	  round it. A place that is bare text holds the line `off` itself.
	- `part="words"`, inside the place's own button, which is `relative` too:
	  the button turns solid and says what letting go will do. Never smaller
	  than the button, and as much wider as the words need. A button drawn
	  with a line round it says so with `edged`, and the words lie over the
	  line as well.

	Something the place cannot take turns both the alert colour. Nothing here
	can be pressed, so the drag goes on reaching the place beneath.

	The 2px line and the 18px drawing are the prototype's, as chosen.
-->
<script lang="ts">
	import type { HeldFile } from '$lib/drop.svelte';
	import { drawnFor } from '$lib/drop-drawings';

	type Props = { held: HeldFile } & (
		| { part: 'frame'; off?: boolean }
		| {
				part: 'words';
				/** What the place says of each thing it can be shown. */
				says: Record<HeldFile, string>;
				/** The drawing for what the place takes. */
				icon: string;
				/** The button has a 1px line round it, which the words cover too. */
				edged?: boolean;
		  }
	);

	let props: Props = $props();
</script>

{#if props.part === 'frame'}
	<div
		class="pointer-events-none absolute z-20 rounded-sm border-2 border-dashed {props.off
			? '-inset-2'
			: 'inset-0'} {props.held === 'no' ? 'border-support' : 'border-accent'}"
	></div>
{:else}
	<span
		role="status"
		class="pointer-events-none absolute z-20 flex items-center gap-2 rounded-sm px-3 text-read font-semibold whitespace-nowrap text-on-accent {props.edged
			? 'top-[-1px] left-[-1px] h-[calc(100%+2px)] min-w-[calc(100%+2px)]'
			: 'top-0 left-0 h-full min-w-full'} {props.held === 'no' ? 'bg-support' : 'bg-accent'}"
	>
		<svg
			viewBox="0 0 24 24"
			aria-hidden="true"
			class="h-[18px] w-[18px] shrink-0"
			fill="none"
			stroke="currentColor"
			stroke-width="1.5"
			stroke-linecap="round"
			stroke-linejoin="round"><path d={drawnFor(props.held, props.icon)} /></svg
		>
		{props.says[props.held]}
	</span>
{/if}
