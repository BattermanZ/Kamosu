<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import Screen from '$lib/shell/Screen.svelte';
	import Empty from '$lib/shell/Empty.svelte';
</script>

<Screen title={m.shopping_title()} blurb={m.shopping_blurb()}>
	<Empty>{m.shopping_empty()}</Empty>
</Screen>
