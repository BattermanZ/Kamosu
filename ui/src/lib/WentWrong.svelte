<!--
	The card Kamosu shows when Kamosu is the thing that broke (#98, option C —
	Aurélien, 22 September 2026).

	It wears the same shape as the "not right now" cards (#76) so it reads as one
	voice, and the same beni rule along the top edge as the warning, because that
	is the tone this is. What it must not do is borrow a refusal's wording: a
	screen saying *this recipe could not be read* when the truth is that Kamosu
	went wrong is the kind of small lie ADR 0003 and ADR 0021 refuse elsewhere.

	It sits above the other cards, and it is not drawn on the cooking screen
	(ADR 0011), which is drawn over the page and would hide it. That screen says
	*Went wrong* in its own top row instead and raises these same words as a sheet
	when the cook taps it (#119). The one time the root layout does draw this
	there is when the cooking screen itself went wrong while being drawn and is
	gone, because then there is no Step left to cover.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import Notice from '$lib/offline/Notice.svelte';
	import { mistake, putMistakeAway } from '$lib/mistake.svelte';
</script>

{#if mistake.made !== undefined}
	<Notice
		title={m.wrong_title()}
		tone="warning"
		actions={[{ label: m.notice_got_it(), act: putMistakeAway }]}
	>
		<p>{m.wrong_body()}</p>
	</Notice>
{/if}
