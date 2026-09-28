<!--
	Asking for a title and writing the recipe it names (#6, #64).

	Submitting the field is the same act as the button, because a person who
	has typed a name and pressed enter has already asked.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import type { Adding } from './adding.svelte';

	interface Props {
		adding: Adding;
	}

	let { adding }: Props = $props();

	let typed = $state('');
</script>

<form
	class="grid gap-2"
	onsubmit={(event) => {
		event.preventDefault();
		adding.write(typed);
	}}
>
	<!--
		`w-full` is load-bearing. Without a width, Safari sizes a text field at
		twenty of the font's average character, and Zen Kaku Gothic New is a
		Japanese face whose average character is a full em. That made the field
		about 340px wide on an iPhone and pushed the whole empty-Home card past
		the edge of the screen (24 September 2026).
	-->
	<label class="grid gap-1 text-read text-ink-2">
		{m.add_title()}
		<input
			type="text"
			bind:value={typed}
			required
			class="min-h-12 w-full rounded-sm border border-rule bg-card px-3 text-body text-ink"
		/>
	</label>
	<button
		class="min-h-12 rounded-sm bg-accent px-4 py-3 font-display text-body text-on-accent disabled:opacity-60"
		disabled={adding.working !== null}
	>
		{m.add_write()}
	</button>
</form>
