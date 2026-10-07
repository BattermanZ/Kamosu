<!--
	The back arrow the wide layout draws on a page reached from another page
	(ADR 0044, #195).

	An app installed from Safari on an iPad shows no browser buttons, so
	without it a tablet has no reliable way back from a recipe.

	A round button holding an arrow and no word, at the top left of the page
	just right of the rail, staying there while the page scrolls. It was chosen
	on 5 October 2026 over a line naming the parent; ADR 0044 says why.

	It is a link to the page's parent, which is where it goes when nothing of
	Kamosu is behind the page. When something is, it goes back instead, exactly
	as the device's own back would.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { somethingBehind } from './way-back';

	interface Props {
		/** The page's parent: where the arrow goes with nothing behind the page. */
		parent: string;
	}

	let { parent }: Props = $props();

	function back(event: MouseEvent) {
		if (!somethingBehind()) return;
		event.preventDefault();
		history.back();
	}
</script>

<a
	href={parent}
	onclick={back}
	aria-label={m.shell_back()}
	class="fixed back-arrow z-10 flex h-12 w-12 items-center justify-center border border-rule bg-card text-accent"
>
	<svg
		viewBox="0 0 24 24"
		aria-hidden="true"
		class="h-6 w-6"
		fill="none"
		stroke="currentColor"
		stroke-width="1.75"
		stroke-linecap="round"
		stroke-linejoin="round"
	>
		<path d="m15 5-7 7 7 7" />
	</svg>
</a>
