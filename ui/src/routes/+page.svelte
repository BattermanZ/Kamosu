<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import Screen from '$lib/shell/Screen.svelte';
	import Empty from '$lib/shell/Empty.svelte';
</script>

<Screen title={m.home_title()} blurb={m.home_blurb()}>
	<Empty>{m.home_empty()}</Empty>
</Screen>
