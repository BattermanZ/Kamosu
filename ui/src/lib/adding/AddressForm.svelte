<!--
	Asking for a web page's address and importing it (#62, #93).

	Drawn the same wherever a link is offered, the + beside the search box and
	the dead ends alike: revealing it in place is what makes the offer *do* the
	thing rather than navigate (ADR 0027), and there is no reason two places
	should ask for an address differently.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import type { Adding } from './adding.svelte';

	interface Props {
		adding: Adding;
	}

	let { adding }: Props = $props();

	let link = $state('');
</script>

<form
	class="grid gap-2"
	onsubmit={(event) => {
		event.preventDefault();
		adding.link(link);
	}}
>
	<label class="grid gap-1 text-read text-ink-2">
		{m.recipes_import_link()}
		<input
			type="url"
			bind:value={link}
			required
			placeholder="https://"
			class="min-h-12 w-full rounded-sm border border-rule bg-card px-3 text-body text-ink"
		/>
	</label>
	<button
		class="min-h-12 rounded-sm bg-accent px-4 py-3 font-display text-body text-on-accent disabled:opacity-60"
		disabled={adding.working !== null}
	>
		{adding.working === 'link' ? m.recipes_import_working() : m.recipes_nothing_import()}
	</button>
</form>
