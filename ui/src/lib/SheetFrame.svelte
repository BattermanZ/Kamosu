<!--
	The one frame every sheet in Kamosu is drawn in (#196, ADR 0044).

	On the phone layout it is a bottom sheet, as each of them was when it drew
	its own. On the wide layout it is a window in the middle of the room beside
	the rail, with the page dimmed and still readable behind it. Aurélien chose
	that window on 5 October 2026 over one as wide as the page's column: 480
	wide, as tall as what it holds, centred both ways. Its measurements live in
	`sheet-window` in `app.css`.

	The frame owns what thirteen sheets each repeated, or forgot:

	- the dimmed page behind;
	- Escape, which closes it;
	- the caret, which moves in when it opens and goes back to what opened it
	  when it closes, so a keyboard user never loses their place on the page;
	- Enter, which confirms where the sheet says it has one clear confirm.

	What a sheet holds stays its own, and so does its padding and its colour,
	which it hands in as `class`.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { Attachment } from 'svelte/attachments';
	import { useRoom } from '$lib/room.svelte';

	/** What a screen reader calls it: said outright, or the id of the heading
	 *  inside that names it. One or the other, always. */
	type Named =
		{ label: string; labelledby?: undefined } | { labelledby: string; label?: undefined };

	type Props = Named & {
		/**
		 * Escape. A sheet that must not close just now (a save on its way)
		 * says so by doing nothing here.
		 */
		onclose: () => void;
		/**
		 * Enter, where the sheet has one clear thing to confirm. Not sent from a
		 * button, a link or a multi-line field, which use Enter themselves, nor
		 * from inside a form, which submits on its own.
		 */
		onconfirm?: () => void;
		/**
		 * `cover` is the whole screen on every layout: a photograph being looked
		 * at. Everything else is a `sheet`.
		 */
		kind?: 'sheet' | 'cover';
		/**
		 * What the page behind is dimmed with. `cook` is the cooking screen's,
		 * which dims nothing on the phone, where its sheets always rose over the
		 * Step undimmed.
		 */
		dim?: 'paper' | 'ink' | 'cook';
		/** How much of a phone's height the bottom sheet may take, in hundredths. */
		tall?: 70 | 78 | 85 | 88;
		/** Whether the bottom sheet keeps clear of the phone's home indicator. */
		safe?: boolean;
		/** The sheet's own padding, colour and inner layout. */
		class?: string;
		children: Snippet;
	};

	let {
		label,
		labelledby,
		onclose,
		onconfirm,
		kind = 'sheet',
		dim = 'paper',
		tall,
		safe = true,
		class: own = '',
		children,
	}: Props = $props();

	const room = useRoom();
	const shape = $derived(kind === 'cover' ? 'cover' : room.wide ? 'window' : 'sheet');

	const TALL = {
		70: 'max-h-[70vh]',
		78: 'max-h-[78vh]',
		85: 'max-h-[85vh]',
		88: 'max-h-[88vh]',
	} as const;
	const DIM = { paper: 'bg-accent/40', ink: 'bg-ink/40', cook: 'bg-cook-ground/60' } as const;

	const frame = $derived(
		shape === 'cover'
			? 'fixed inset-0 z-50'
			: shape === 'window'
				? `fixed inset-x-0 z-50 mx-auto sheet-window ${safe ? 'pb-safe' : ''}`
				: `fixed inset-x-0 bottom-0 z-50 mx-auto max-w-2xl ${safe ? 'pb-safe' : ''} ${tall ? TALL[tall] : ''}`,
	);
	const dimmed = $derived(shape === 'window' || (shape === 'sheet' && dim !== 'cook'));
	// Behind a window the paper is dimmed one way, as lightly as the phone dims
	// it (ADR 0044). `ink` is the darker dimming one screen's bottom sheet has.
	const dimWith = $derived(shape === 'window' && dim === 'ink' ? 'paper' : dim);

	let dialog = $state<HTMLElement | undefined>(undefined);

	/**
	 * The caret, in and back.
	 *
	 * In: to the field the sheet marks with `data-sheet-focus`, where typing is
	 * what the sheet is for. Otherwise to the frame itself, never to the first
	 * control inside, so the caret does not land on a choice that changes the
	 * recipe.
	 *
	 * Back: to what opened it. That control is sometimes gone by then: choosing
	 * a recipe for a line turns *Recipe* into *Names Pizza Dough*, so the button
	 * that was focused is destroyed by the choice it was opened to make. What
	 * held it survives, so the caret goes to the button that now stands there.
	 *
	 * On a frame rather than at once, because at teardown the replacement has
	 * not been drawn yet, and focusing then would land on `<body>` after all.
	 */
	const caret: Attachment<HTMLElement> = (node) => {
		const cameFrom = document.activeElement;
		const held = cameFrom instanceof HTMLElement ? cameFrom.parentElement : null;
		(node.querySelector<HTMLElement>('[data-sheet-focus]') ?? node).focus();
		return () => {
			requestAnimationFrame(() => {
				if (cameFrom instanceof HTMLElement && cameFrom.isConnected) {
					cameFrom.focus();
					return;
				}
				if (held?.isConnected) held.querySelector('button')?.focus();
			});
		};
	};

	/** The fields a person types one line into, where Enter means "that's it". */
	const ONE_LINE = new Set(['text', 'search', 'number', 'url', 'email', 'tel']);

	function onKey(event: KeyboardEvent) {
		// A key pressed while a word is being composed belongs to the keyboard
		// composing it: Escape there cancels the word, not the sheet.
		if (event.isComposing) return;
		if (event.key === 'Escape') {
			onclose();
			return;
		}
		// A held key repeats. The Enter that opened the sheet must not also be
		// the Enter that confirms it.
		if (event.key !== 'Enter' || !onconfirm || event.repeat) return;
		const from = event.target;
		if (!(from instanceof HTMLElement) || !dialog?.contains(from)) return;
		const typing = from instanceof HTMLInputElement && ONE_LINE.has(from.type);
		if ((from !== dialog && !typing) || from.closest('form')) return;
		event.preventDefault();
		onconfirm();
	}
</script>

<svelte:window onkeydown={onKey} />

{#if dimmed}
	<div class="fixed inset-0 z-40 {DIM[dimWith]}" data-sheet-dimming></div>
{/if}
<div
	bind:this={dialog}
	class="{frame} {own}"
	role="dialog"
	aria-modal="true"
	aria-label={label}
	aria-labelledby={labelledby}
	tabindex="-1"
	data-sheet={shape}
	data-dim={dim}
	{@attach caret}
>
	{@render children()}
</div>
