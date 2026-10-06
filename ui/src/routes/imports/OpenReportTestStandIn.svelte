<!--
	Test-only. What stands where the open Report's page does in the screen test
	of the list of imports: a field holding its Job id, which is all the list
	needs of it, and a button that says the import ended, as the Report does
	when it watches one end.
-->
<script lang="ts">
	import { useEnded } from './ended.svelte';

	interface Props {
		open?: string;
	}

	let { open }: Props = $props();

	const ended = useEnded();
</script>

<input aria-label="the open Report" value={open} />
<button type="button" onclick={() => ended?.say()}> it ended </button>
