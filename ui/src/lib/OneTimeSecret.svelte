<!--
	A secret the Core hands over once and never again: an Access Key, a Kitchen
	Invite. It shows the whole secret, wrapped rather than scrolled sideways so
	its end is not lost off a phone's edge, with a Copy button that is the
	biggest thing in the box (#142).

	Whether it went onto the clipboard lives in here, so a new secret starts at
	"Copy" by being a new box rather than by somebody remembering to reset a
	flag. A browser that will not copy leaves the button at "Copy" and the
	secret on screen, where one tap selects all of it by hand.

	`onCard` is for a box drawn inside a card, which swaps the two surfaces so
	the box still stands apart from what holds it.
-->
<script lang="ts">
	import type { Attachment } from 'svelte/attachments';
	import { copyText } from '$lib/clipboard';

	let {
		secret,
		once,
		copy,
		copied,
		dismiss,
		ondismiss,
		focus = false,
		onCard = false,
	}: {
		secret: string;
		once: string;
		copy: string;
		copied: string;
		dismiss: string;
		ondismiss: () => void;
		/** Move focus to Copy, for a box that replaces the control just pressed. */
		focus?: boolean;
		onCard?: boolean;
	} = $props();

	let wentOnClipboard = $state(false);

	const focuses: Attachment<HTMLElement> = (node) => {
		if (focus) node.focus();
	};
</script>

<div class={['rounded-sm border border-accent p-3', onCard ? 'bg-ground' : 'bg-card']} role="alert">
	<p class="text-body font-semibold text-ink">{once}</p>
	<code
		class={[
			'mt-2 block rounded-sm p-2 text-read break-all select-all',
			onCard ? 'bg-card' : 'bg-ground',
		]}>{secret}</code
	>
	<button
		type="button"
		class="mt-3 min-h-12 w-full rounded-sm bg-accent px-4 font-semibold text-on-accent"
		onclick={async () => (wentOnClipboard = await copyText(secret))}
		{@attach focuses}
	>
		{wentOnClipboard ? copied : copy}
	</button>
	<button type="button" class="mt-3 block text-label text-accent underline" onclick={ondismiss}>
		{dismiss}
	</button>
</div>
