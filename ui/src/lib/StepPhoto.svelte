<!--
	A Step's photograph, where a Step is read (#110): a small square beside the
	words, which opens the picture across the whole screen. The
	square was chosen on both screens on 23 September 2026 — R2 on the recipe page, K3 at
	the stove — over a full-width picture that pushed the Method apart and, at
	the stove, cut a long Step's words off to make room.

	Until #110 a Step's photograph was drawn nowhere but the writing screen, so
	a picture put on a Step was a Version nobody could see.

	The picture is the Step's, never the cooking's: it is part of the recipe,
	which is why it is drawn from `/api/photographs` directly, as the hero is.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import SheetFrame from '$lib/SheetFrame.svelte';

	interface Props {
		photograph: string;
		/** The Step's number as the list shows it, for what the picture is called. */
		number: number;
		/** Size and place of the square, which the two screens set differently. */
		shapeClass: string;
	}

	let { photograph, number, shapeClass }: Props = $props();

	let open = $state(false);
</script>

<button
	type="button"
	class="brightens shrink-0 {shapeClass}"
	aria-label={m.step_photo_open({ number })}
	onclick={() => (open = true)}
>
	<img
		src="/api/photographs/{photograph}/card"
		alt=""
		class="block h-full w-full rounded-sm object-cover"
	/>
</button>

{#if open}
	<!--
		The whole screen, and a tap anywhere puts it away: there is nothing here
		to do but look, and a cook's hands are rarely clean enough to aim for a
		small close button.

		On the cooking screen's own dark ground on BOTH screens, the recipe page
		too: a photograph is looked at on dark, and one picture shown two ways
		would be two things to learn.

		The whole screen on the wide layout as well (#196), where every other
		sheet became a small window: a photograph is opened to be seen large,
		and it was already in the middle of a dimmed page.
	-->
	<SheetFrame
		kind="cover"
		label={m.step_photo_alt({ number })}
		class="flex items-center justify-center bg-cook-ground/95"
		onclose={() => (open = false)}
	>
		<img
			src="/api/photographs/{photograph}/page"
			alt={m.step_photo_alt({ number })}
			class="max-h-full max-w-full object-contain"
		/>
		<!-- `bg-transparent` is said outright because this button lies over the
		     whole photograph: with a fill of its own, even none, the shade
		     everything else takes under the pointer leaves it alone (#203). -->
		<button
			type="button"
			class="absolute inset-0 flex items-start justify-end bg-transparent p-4 text-body text-cook-ink"
			onclick={() => (open = false)}
		>
			{m.photo_close()}
		</button>
	</SheetFrame>
{/if}
