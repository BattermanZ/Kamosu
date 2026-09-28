<!--
	Test-only, for the same reason the recipe screen's harness exists:
	Cooking.svelte takes `branchId` as a real prop so a test can drive it
	directly, which the shared prop-less `renderScreen` helper cannot do.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import type { Keeping } from '$lib/offline/outbox';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Cooking from './Cooking.svelte';

	interface Props {
		client: KamosuClient;
		branchId: string;
		keeping?: Keeping;
	}

	let { client, branchId, keeping }: Props = $props();
</script>

<Kamosu {client} {keeping}>
	<Cooking {branchId} />
</Kamosu>
