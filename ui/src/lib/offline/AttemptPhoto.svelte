<!--
	One of a cooking's photographs (#77). It may not have reached the server
	yet — taken with no network, it waits on the phone — so where it is drawn
	from is asked of the outbox rather than written as a URL here.

	Square, at `--photo-thumb`, which says why it is the size it is.
-->
<script lang="ts">
	import { useKeeping } from '$lib/offline/outbox';

	interface Props {
		id: string;
		alt: string;
	}

	let { id, alt }: Props = $props();
	const keeping = useKeeping();
	const SQUARE = 'width: var(--photo-thumb); height: var(--photo-thumb)';

	let src = $state<string | undefined>(undefined);
	$effect(() => {
		let current = true;
		void keeping.photographSrc(id, 'card').then((found) => {
			if (current) src = found;
		});
		return () => {
			current = false;
		};
	});
</script>

{#if src}
	<img {src} {alt} class="rounded-sm object-cover" style={SQUARE} />
{:else}
	<span class="block rounded-sm bg-ground-2" style={SQUARE} aria-hidden="true"></span>
{/if}
