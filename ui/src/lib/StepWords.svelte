<!--
	A Step's own words, with each conversion drawn straight after what it
	converts (#150): "preheat to 425° (about 220 °C)", "1 lb. (about 455 g)
	ground chicken". Option A, Aurélien's choice on 25 September 2026, over all
	of a Step's figures on one line beneath it, which left a cook counting along
	a row of numbers to match them back.

	The words are the Step's truth and are drawn exactly as written; what sits
	between them is the screen's addition and was never written into the Step
	(ADR 0016). The two screens that read a Step set its size and colour. The
	written text and its figure never break across a line: found live, "3" ended one
	line and "oz. (about 85 g)" began the next.
-->
<script lang="ts">
	import { stepPieces, type StepConversions } from '$lib/step-conversions';

	interface Props {
		text: string;
		conversions: StepConversions;
		/** The addition's type and colour, which the two screens set differently. */
		readingClass: string;
	}

	let { text, conversions, readingClass }: Props = $props();

	const pieces = $derived(stepPieces(text, conversions));
</script>

{#each pieces as piece, index (index)}{piece.text}{#if piece.written}<span class="whitespace-nowrap"
			>{piece.written}&nbsp;<span class={readingClass}>({piece.measured})</span></span
		>{/if}{/each}
