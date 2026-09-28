<!--
	`/import?link=<Share Link>`: where a Share Link page's *Import this recipe*
	leads (#170), and where another Kamosu sends a reader who keeps their
	recipes here. The screen is `ImportShared`; this only reads the address.
-->
<script lang="ts">
	import { page } from '$app/state';
	import ImportShared from './ImportShared.svelte';
</script>

<ImportShared
	link={page.url.searchParams.get('link')}
	origin={page.url.origin}
	leave={(address) => window.location.assign(address)}
/>
