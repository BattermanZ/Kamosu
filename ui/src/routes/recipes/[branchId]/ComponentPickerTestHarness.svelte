<!--
	Test-only. The picker is a sheet a writing screen raises over itself, so a
	test drives it directly rather than through a whole writing screen it is not
	testing.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import ComponentPicker from './ComponentPicker.svelte';

	interface Props {
		client: KamosuClient;
	}

	let { client }: Props = $props();
</script>

<Kamosu {client}>
	<ComponentPicker line="500 g pizza dough" onChoose={() => {}} onCancel={() => {}} />
</Kamosu>
