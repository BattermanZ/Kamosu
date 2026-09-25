<!--
	Test-only, for the same reason the Thread's harness exists: Recipe.svelte
	takes `branchId` as a real prop so a test can drive it directly, which the
	shared prop-less `renderScreen` helper cannot do.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import type { SheetFetcher } from '$lib/api/sheet';
	import type { Device } from '$lib/offline/device.svelte';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Recipe from './Recipe.svelte';

	interface Props {
		client: KamosuClient;
		branchId: string;
		device?: Pick<Device, 'installed' | 'apple'>;
		sheets?: SheetFetcher;
	}

	let { client, branchId, device, sheets }: Props = $props();
</script>

<Kamosu {client} {sheets}>
	<Recipe {branchId} {device} />
</Kamosu>
