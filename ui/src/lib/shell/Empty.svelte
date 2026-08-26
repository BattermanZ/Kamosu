<!--
	What a section says when it holds nothing yet. A sentence, not an icon and an
	apology: it names what will arrive here, so an empty Kamosu still explains
	itself.
-->
<script lang="ts">
	interface Props {
		children: import('svelte').Snippet;
	}

	let { children }: Props = $props();
</script>

<p class="rounded-sm border border-rule border-dashed bg-ground-2 p-6 text-body text-ink-2">
	{@render children()}
</p>
