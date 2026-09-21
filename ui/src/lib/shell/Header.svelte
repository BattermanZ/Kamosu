<!--
	The shell's top: the mark on the left, and the one way into Settings on the
	right (#102).

	The way in is **you**, not the Kitchen's name. #37 intended a Kitchen's name
	here and never built one, which left a logo that was secretly a button: the
	only thing that ever said so was the word *Settings* spoken to screen readers
	and hidden from everyone else. What Settings holds is your account, your
	phone and your Kitchens, so the door is a person.

	No Kitchen is named here at any number of Kitchens. ADR 0027 ruled out a
	switcher, and filtering by Kitchen already lives where it belongs — as chips
	on the Recipes shelf (#62). A second, worse place to think about Kitchens is
	not worth the width. Built and walked at a phone viewport, a Kitchen's name
	truncated on six Kitchens and still said nothing about opening Settings.

	No name is shown, and none can be: Kamosu has no Operation answering *who is
	signed in* — `rename_person` writes a name and nothing reads one back, and a
	home Kitchen may hold more than one member. *You* needs none.

	The mark is a plain wordmark and taps nothing, so the header holds exactly
	one control. The middle stays empty rather than inventing work for itself.

	This card is also the shell's expanding pair: it and the Settings screen's
	title share `view-transition-name: settings`, so the card grows into the page
	rather than the screen swapping (ADR 0012). Every recipe card that opens into
	its page is the same pair under a different name. On Settings itself the card
	*is* the page, so it drops the name — naming both halves of a pair on one
	screen is not a pair.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';

	interface Props {
		/** Whether Settings is the page being shown, which the card is the door to. */
		onSettings: boolean;
	}

	let { onSettings }: Props = $props();
</script>

<header class="sticky top-0 z-10 border-b border-rule bg-ground pt-safe">
	<div class="mx-auto flex max-w-2xl items-center justify-between gap-3 px-gutter py-3">
		<span class="flex min-w-0 items-center gap-3 font-display text-title font-semibold text-ink">
			<img src="/assets/img/kamosu-mark.svg" alt="" class="h-8 w-8 shrink-0 rounded-sm" />
			<span class="truncate">{m.app_name()}</span>
		</span>

		<a
			href="/settings"
			aria-current={onSettings ? 'page' : undefined}
			class="flex min-h-12 shrink-0 items-center gap-2 rounded-sm border p-2 text-body font-medium
			{onSettings ? 'border-accent bg-card text-accent' : 'border-rule bg-card text-ink'}"
			style={onSettings
				? undefined
				: 'view-transition-name: settings; view-transition-class: expanding'}
		>
			<span
				class="flex h-8 w-8 items-center justify-center rounded-sm border border-rule bg-ground-2"
			>
				<svg
					viewBox="0 0 24 24"
					aria-hidden="true"
					class="h-6 w-6 text-ink-2"
					fill="none"
					stroke="currentColor"
					stroke-width="1.5"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<path d="M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm-7 8a7 7 0 0 1 14 0" />
				</svg>
			</span>
			{m.header_you()}
			<!-- Where the card goes, for a reader who cannot see that it is a card.
			     The visible word is *You*; this names the room it opens. -->
			<span class="sr-only">— {m.open_settings()}</span>
			<svg
				viewBox="0 0 24 24"
				aria-hidden="true"
				class="h-4 w-4 text-ink-2"
				fill="none"
				stroke="currentColor"
				stroke-width="1.75"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<path d="m9 5 7 7-7 7" />
			</svg>
		</a>
	</div>
</header>
