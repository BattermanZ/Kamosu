<!--
	The file field a Bundle (a Kamosu zip file) is chosen with, opened by a button
	somewhere else (#93), or a recipe PDF (#176): one field per kind, since each
	offers the phone's picker only the files it can take.

	A file cannot be chosen from a `<button>`, and the button has to be able to
	say offline what it is waiting for (#76) rather than fail when pressed. So
	the field is here, hidden, and whoever draws the button calls `open()`.

	**Kept out of the accessibility tree, not merely off the screen.** A file
	field maps to `role="button"` in Chromium, so an `sr-only` one beside the
	real button announced the same act twice and put a stop in the tab order
	that looked identical to the next one. Found on the dev instance with a
	screen-reader tree, 21 September 2026, not by any test. `display: none`
	costs nothing: a file picker opens from a programmatic `.click()` whether
	the field is drawn or not.

	The label stays because it is what says *which* field this is when anything
	does reach it, and it is how a screen test grips it.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import type { Adding } from './adding.svelte';

	interface Props {
		adding: Adding;
		/** What it picks. A Bundle unless said otherwise. */
		kind?: 'bundle' | 'pdf';
	}

	let { adding, kind = 'bundle' }: Props = $props();

	/** What each kind offers the picker, calls itself, and does once chosen. */
	const KINDS = {
		bundle: {
			accept: '.zip,application/zip',
			label: m.bring_in_file_full,
			take: (file: File) => adding.file(file),
		},
		pdf: {
			accept: '.pdf,application/pdf',
			label: m.plus_pdf_full,
			take: (file: File) => adding.pdf(file),
		},
	};
	const picks = $derived(KINDS[kind]);

	let field = $state<HTMLInputElement | undefined>(undefined);

	/** Open the phone's own file picker. */
	export function open() {
		field?.click();
	}
</script>

<input
	type="file"
	accept={picks.accept}
	class="hidden"
	aria-label={picks.label()}
	aria-hidden="true"
	tabindex="-1"
	bind:this={field}
	disabled={adding.working !== null}
	onchange={(event) => {
		const file = event.currentTarget.files?.[0];
		// Cleared at once, so choosing the same file twice is two choices.
		event.currentTarget.value = '';
		if (file) picks.take(file);
	}}
/>
