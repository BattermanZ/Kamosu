<!--
	The route: reads the word the Reading corrector arrived with, where it sent
	one, and hands it to Foods.svelte, which holds everything the screen does.

	`{#key q}` because the search box is seeded from that word and then belongs
	to whoever is typing in it. SvelteKit reuses one component across two URLs
	that differ only in their query, so without this a second arrival from a
	second ingredient line would find the first line's word still in the box.
-->
<script lang="ts">
	import { page } from '$app/state';
	import Foods from './Foods.svelte';

	const q = $derived(page.url.searchParams.get('q') ?? '');
</script>

{#key q}
	<Foods {q} />
{/key}
