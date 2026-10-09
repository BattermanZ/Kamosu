<!--
	The back arrow on a page reached from another page (ADR 0044, #195, #219).

	An app installed from Safari on an iPad shows no browser buttons, so
	without it a tablet has no reliable way back from a recipe.

	A round button holding an arrow and no word, at the top left of the page,
	staying there while the page scrolls. It was chosen on 5 October 2026 over
	a line naming the parent; ADR 0044 says why. The wide layout fixes it just
	right of the rail. The phone had no consistent way back until #219 gave it
	the same arrow (option 1 of the audit of 9 October 2026): there it stands
	in the page's own flow at the top left, and sticks to the top of the
	window as the page scrolls past it, since the header scrolls away (#218)
	and the arrow is what must not.

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
		/** Fixed beside the rail on the wide layout; sticky in the page's flow on the phone. */
		layout: 'wide' | 'phone';
	}

	let { parent, layout }: Props = $props();

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
	class="z-10 flex h-12 w-12 items-center justify-center border border-rule bg-card text-accent
	{layout === 'wide' ? 'fixed back-arrow' : 'back-arrow-phone'}"
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
