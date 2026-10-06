<!--
	Test-only. What stands where the open Food's page does in the Foods list's
	screen test: a field holding its id, which is all the list needs of it, and
	a button that says the Food was corrected, as its page does after an act.
-->
<script lang="ts">
	import type { Food } from '$lib/foods';
	import { useCorrected } from './corrected.svelte';

	interface Props {
		open?: string;
		/** What the Core answered the correction with. */
		correction?: Food;
	}

	let { open, correction }: Props = $props();

	const corrected = useCorrected();
</script>

<input aria-label="the open Food" value={open} />
<button
	type="button"
	onclick={() => {
		if (corrected) corrected.latest = correction;
	}}
>
	correct it
</button>
