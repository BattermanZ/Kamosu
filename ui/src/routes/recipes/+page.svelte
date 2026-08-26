<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import Screen from '$lib/shell/Screen.svelte';
	import Empty from '$lib/shell/Empty.svelte';
</script>

<Screen title={m.recipes_title()} blurb={m.recipes_blurb()}>
	<Empty>{m.recipes_empty()}</Empty>
</Screen>
