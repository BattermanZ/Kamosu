<!--
	A COMPONENT'S OWN LINE: which recipe it names and how much of it, or the one
	sentence saying why there is no unfolding. It sits in the same slot every
	Ingredient Line has for its conversion (#49, ADR 0016) — a Component says
	something DIFFERENT there, not something extra beside it — and it is worded
	by the Core, so this screen, the Share Link page and an agent at the MCP
	door all say it alike.

	IT IS ALSO THE WAY IN, where there is something to open. The written line
	above keeps its own tap, which every line on this page has, so the recipe's
	NAME is what opens the recipe — the more obvious of the two anyway. A
	Component with nothing behind it is not a target: a missing recipe and a
	stopped repeat are sentences, not doors.

	Drawn outside the written line's button on purpose: a button inside a
	button is invalid HTML and gives one row two overlapping targets, which on
	a phone is a coin toss.
-->
<script lang="ts">
	import type { Component } from './unfolding.svelte';

	interface Props {
		component: Component;
		open: boolean;
		toggle: () => void;
	}

	let { component, open, toggle }: Props = $props();
</script>

{#if component.content}
	<button
		type="button"
		class="flex w-full items-start gap-1 text-left text-read text-support-2"
		aria-expanded={open}
		onclick={toggle}
	>
		<span class="min-w-0 flex-1">{component.said}</span>
		<svg
			viewBox="0 0 24 24"
			aria-hidden="true"
			class="mt-1 h-3 w-3 shrink-0"
			style={open ? 'transform: rotate(90deg)' : ''}
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			stroke-linecap="round"
			stroke-linejoin="round"
		>
			<path d="m9 5 7 7-7 7" />
		</svg>
	</button>
{:else}
	<span class="block text-read text-support-2">{component.said}</span>
{/if}
