<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import Screen from '$lib/shell/Screen.svelte';
	import Empty from '$lib/shell/Empty.svelte';
</script>

<Screen title={m.cooked_title()} blurb={m.cooked_blurb()}>
	<Empty>{m.cooked_empty()}</Empty>
</Screen>
