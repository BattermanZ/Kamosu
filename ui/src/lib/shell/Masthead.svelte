<!--
	Kamosu's mark with its name under it, above a page the wide layout draws
	with no sidebar: the account form, an Invite, a recovery link, and a
	Cookbook Invite read by somebody not signed in (#194, #214).

	The sidebar is where a wide window reads the name, and these pages have
	none, so without this the name stood nowhere on them. It was asked for
	on 5 October 2026, on the same objection the first rail met. The phone
	layout draws its own header bare on the same pages (#218), by the same
	answer in `Shell.svelte` (#214).
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
</script>

<header
	class="flex flex-col items-center gap-2 pt-safe font-display text-title font-semibold text-ink"
>
	<img src="/assets/img/kamosu-mark.svg" alt="" class="mt-8 h-12 w-12 rounded-sm" />
	{m.app_name()}
</header>
