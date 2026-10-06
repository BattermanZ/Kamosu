<!--
	What a section says when it holds nothing yet. A sentence, not an icon and an
	apology: it names what will arrive here, so an empty Kamosu still explains
	itself.

	It stops at the phone's column, so on a screen that fills a wide window
	(#201) the sentence does not run the width of a monitor.
-->
<script lang="ts">
	interface Props {
		children: import('svelte').Snippet;
	}

	let { children }: Props = $props();
</script>

<p
	class="max-w-2xl rounded-sm border border-dashed border-rule bg-ground-2 p-6 text-body text-ink-2"
>
	{@render children()}
</p>
