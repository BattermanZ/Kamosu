<!--
	PROTOTYPE — #58. THROWAWAY. Delete on merge.

	The bar Aurélien drives the prototype from. Deliberately ugly and obviously
	not Kamosu: it must never be mistaken for part of what is being judged.

	It carries three things — which treatment is on screen, a switch that
	pretends the network is gone so the offline refusal can be looked at, and a
	button that throws the cooking's changes away so the common case (deviated
	from nothing) can be seen again without restarting anything.
-->
<script lang="ts">
	import { dev } from '$app/environment';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { asCooked } from './deviation.svelte';

	const names: Record<string, string> = {
		A: 'A — the step becomes writable',
		B: 'B — one sheet, the whole cooking',
	};

	const current = $derived(page.url.searchParams.get('variant') ?? '');

	function show(variant: string) {
		const url = new URL(page.url);
		if (variant === '') url.searchParams.delete('variant');
		else url.searchParams.set('variant', variant);
		void goto(url, { replaceState: true, noScroll: true, keepFocus: true });
	}

	/**
	 * A and B only. "Prototype off" used to sit in this cycle, and pressing an
	 * arrow onto it looked exactly like the arrows doing nothing — which is how
	 * the first hand-over of this prototype was read. Every press now visibly
	 * changes the screen. Drop `?variant=` from the URL to see the screens as
	 * they are today.
	 */
	function cycle(by: number) {
		const order = ['A', 'B'];
		const at = Math.max(0, order.indexOf(current));
		show(order[(at + by + order.length) % order.length] ?? 'A');
	}

	/** Collapsed to a dot, so it can be got out of the way of the cook screen. */
	let shown = $state(true);

	function onKey(event: KeyboardEvent) {
		const on = event.target as HTMLElement | null;
		if (on && (on.isContentEditable || ['INPUT', 'TEXTAREA'].includes(on.tagName))) return;
		if (event.key === 'ArrowLeft') cycle(-1);
		if (event.key === 'ArrowRight') cycle(1);
	}
</script>

<svelte:window onkeydown={onKey} />

{#if dev && !shown}
	<button
		type="button"
		onclick={() => (shown = true)}
		style="position:fixed;right:8px;bottom:8px;z-index:2147483647;all:unset;cursor:pointer;
			width:34px;height:34px;border-radius:999px;background:#111;color:#fff;
			font:700 11px/34px ui-monospace,monospace;text-align:center;
			box-shadow:0 4px 20px rgba(0,0,0,.45)">58</button
	>
{/if}

{#if dev && shown}
	<div
		style="position:fixed;left:50%;bottom:6px;transform:translateX(-50%);z-index:2147483647;
			display:flex;align-items:center;gap:6px;padding:5px 7px;border-radius:999px;
			background:#111;color:#fff;font:600 11px/1.1 ui-monospace,monospace;
			box-shadow:0 4px 20px rgba(0,0,0,.45);white-space:nowrap;max-width:96vw;overflow:auto"
	>
		<button type="button" onclick={() => cycle(-1)} style="all:unset;cursor:pointer;padding:4px 6px"
			>◀</button
		>
		<span style="padding:0 4px">
			{current === '' ? 'prototype off — #58' : names[current]}
		</span>
		<button type="button" onclick={() => cycle(1)} style="all:unset;cursor:pointer;padding:4px 6px"
			>▶</button
		>
		<button
			type="button"
			onclick={() => (asCooked.offline = !asCooked.offline)}
			style="all:unset;cursor:pointer;padding:4px 8px;border-radius:999px;
				background:{asCooked.offline ? '#b8474b' : '#333'};color:#fff"
		>
			{asCooked.offline ? 'offline' : 'online'}
		</button>
		<button
			type="button"
			onclick={() => asCooked.clear()}
			style="all:unset;cursor:pointer;padding:4px 8px;border-radius:999px;background:#333;color:#fff"
		>
			reset
		</button>
		<button
			type="button"
			onclick={() => (shown = false)}
			style="all:unset;cursor:pointer;padding:4px 8px;border-radius:999px;background:#333;color:#fff"
		>
			hide
		</button>
	</div>
{/if}
