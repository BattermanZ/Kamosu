<!--
	The card Kamosu shows when Kamosu is the thing that broke (#98, option C —
	Aurélien, 22 September 2026).

	It wears the same shape as the "not right now" cards (#76) so it reads as one
	voice, and the same beni rule along the top edge as the warning, because that
	is the tone this is. What it must not do is borrow a refusal's wording: a
	screen saying *this recipe could not be read* when the truth is that Kamosu
	went wrong is the kind of small lie ADR 0003 and ADR 0021 refuse elsewhere.

	It sits above the other cards, and it is the one thing the cooking screen
	carries despite wearing no shell (ADR 0011). The header and the tab bar are
	kept off that screen because they cost it permanent height and put Shopping
	one wet thumb away; this costs neither — it is the rarest card in the app,
	and a cook whose cooking screen has quietly stopped working is exactly who
	most needs telling.
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
