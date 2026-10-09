<!--
	The shell's top on the phone layout: the mark on the left, and the one way
	into Settings on the right (#102). The wide layout draws the sidebar in its
	place, which lists Settings itself (#194).

	The way in is a gear, 48 × 48, icon and no word (#218, option 2 of the
	audit of 9 October 2026). It was the *You* card from #102 until then: an
	avatar, a word and a chevron, 110px wide, for one control. The rail draws
	Settings as a gear, and the phone now reaches it by the same sign; the
	accessible name says where it goes.

	No Kitchen is named here at any number of Kitchens. ADR 0027 ruled out a
	switcher, and filtering by Kitchen already lives where it belongs — as chips
	on the Recipes shelf (#62).

	The header scrolls away with the page (#218): it held nothing a reader of
	a long recipe needed, and with the bar it took a fifth of a phone screen.
	The way back on such a page is the arrow (#219), which stays.

	The mark is a plain wordmark and taps nothing, so the header holds exactly
	one control. The middle stays empty rather than inventing work for itself.

	The gear is also the shell's expanding pair: it and the Settings screen's
	title share `view-transition-name: settings`, so it grows into the page
	rather than the screen swapping (ADR 0012). On Settings itself the gear
	*is* the page, so it drops the name — naming both halves of a pair on one
	screen is not a pair.

	On a page somebody reads before they are in the app (the account form, an
	Invite, a recovery link), the header is drawn `bare`: the mark and the
	name, and no way into a Settings that cannot load (#218, #214).
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { settings } from './places';

	interface Props {
		/** Whether Settings is the page being shown, which the gear is the door to. */
		onSettings: boolean;
		/** Drawn for somebody not yet in the app: the mark and the name, no gear. */
		bare?: boolean;
	}

	let { onSettings, bare = false }: Props = $props();
</script>

<header class="border-b border-rule bg-ground pt-safe">
	<div class="mx-auto flex max-w-2xl items-center justify-between gap-3 px-gutter py-3">
		<span class="flex min-w-0 items-center gap-3 font-display text-title font-semibold text-ink">
			<img src="/assets/img/kamosu-mark.svg" alt="" class="h-8 w-8 shrink-0 rounded-sm" />
			<span class="truncate">{m.app_name()}</span>
		</span>

		{#if !bare}
			<a
				href={settings.href}
				aria-label={settings.label()}
				aria-current={onSettings ? 'page' : undefined}
				class="flex h-12 w-12 shrink-0 items-center justify-center rounded-sm
				{onSettings ? 'text-accent' : 'text-ink'}"
				style={onSettings
					? undefined
					: 'view-transition-name: settings; view-transition-class: expanding'}
			>
				<svg
					viewBox="0 0 24 24"
					aria-hidden="true"
					class="h-[28px] w-[28px]"
					fill="none"
					stroke="currentColor"
					stroke-width={onSettings ? 2 : 1.5}
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<path d={settings.path} />
				</svg>
			</a>
		{/if}
	</div>
</header>
