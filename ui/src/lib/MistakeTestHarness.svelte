<!--
	The root layout's arrangement, in the small (#98): the card OUTSIDE the
	boundary and the screen inside it.

	That order is the whole point, so it is what a test renders. A boundary drops
	its content once it has handled something, and if the card were inside it the
	one surface meant to report the mistake would be dropped along with the screen
	that made it.
-->
<script lang="ts">
	import WentWrong from './WentWrong.svelte';
	import { wentWrong } from './mistake.svelte';

	let { broken = false }: { broken?: boolean } = $props();

	/** A screen going wrong while rendering, which is what a boundary is for. */
	const goWrong = (): string => {
		throw new TypeError('this screen goes wrong while rendering');
	};
</script>

<WentWrong />

<svelte:boundary onerror={wentWrong}>
	<p>{broken ? goWrong() : 'the screen, drawn'}</p>
</svelte:boundary>
