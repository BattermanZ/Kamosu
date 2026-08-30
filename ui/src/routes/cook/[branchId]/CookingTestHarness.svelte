<!--
	Test-only, for the same reason the recipe screen's harness exists:
	Cooking.svelte takes `branchId` as a real prop so a test can drive it
	directly, which the shared prop-less `renderScreen` helper cannot do.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Cooking from './Cooking.svelte';

	interface Props {
		client: KamosuClient;
		branchId: string;
	}

	let { client, branchId }: Props = $props();
</script>

<Kamosu {client}>
	<Cooking {branchId} />
</Kamosu>
