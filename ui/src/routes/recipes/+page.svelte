<!--
	The route: Shelf.svelte holds everything the screen actually does.

	`?tag=` is read here rather than in the screen, which is this project's
	pattern for every route-shaped input (see `Thread.svelte`): the screen takes
	an ordinary prop, so a test drives it without a router. A chip on a recipe
	page is what puts the tag in the URL (#104).
-->
<script lang="ts">
	import { page } from '$app/state';
	import Shelf from './Shelf.svelte';
</script>

<Shelf tag={page.url.searchParams.get('tag')} />
