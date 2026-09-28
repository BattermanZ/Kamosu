<!--
	What an act that landed nothing says, where it was asked (#93, #117).
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import type { Adding } from './adding.svelte';

	interface Props {
		adding: Adding;
	}

	let { adding }: Props = $props();
</script>

{#if adding.failed}
	<p class="mt-2 text-read text-support" role="alert">{adding.failed}</p>
{:else if adding.stillGoing}
	<p class="mt-2 text-read text-ink-2" role="status">
		{m.bring_in_still_going({ where: `${m.settings_title()} › ${m.settings_imports()}` })}
	</p>
{/if}
