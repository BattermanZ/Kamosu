<!--
	The story (#158): what Kamosu is, told in eleven stages, before an Invite's
	form and at `/about`.

	Stories-style and tap only. One stage fills the screen; a tap on the right
	goes on, a tap on the left edge goes back, and each stage plays its drawing
	once and then waits. Nothing moves on a timer: a reader who has not tapped
	for a few seconds is shown a pulsing "tap to continue", and that is all.

	Its words switch between English, French and Spanish in place, on the stage
	the reader is on, and the choice is kept the way the rest of Kamosu keeps it
	(Paraglide's `localStorage` strategy), so the form and the app that follow
	open in it. Every word is real text, the stamps included; the drawings
	around them are decoration and are hidden from a screen reader. With reduced
	motion asked for, each stage is drawn finished: every animation here starts
	from somewhere else and ends on the element's own style, so taking the
	animation away leaves exactly the last frame.

	The direction (Paper), the storyboard and every sentence were chosen on
	prototypes, 25 September 2026; the issue holds the record.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { getLocale, locales, setLocale, type Locale } from '$lib/paraglide/runtime';
	import { story } from './showing.svelte';

	interface Props {
		/**
		 * How the last stage ends. At an Invite it ends on the button that opens
		 * the form; at `/about` there is no Invite, so it asks the reader to get
		 * one, and offers a way out instead.
		 */
		ending: 'invite' | 'about';
		/** The Invite's button was pressed. */
		onCreate?: () => void;
		/** The reader closed the story at `/about`. */
		onClose?: () => void;
	}

	let { ending, onCreate, onClose }: Props = $props();

	/** How long the reader may sit still on a finished stage before the hint. */
	const HINT_AFTER = 3500;
	/** The two people the reader shares a Kitchen with: names, not words. */
	const SAM = 'Sam';
	const INES = 'Inès';

	/**
	 * The stages, in order: the big sentence, the paragraph under it, and how
	 * long the drawing takes, so the hint waits for it to finish. The first six
	 * are part one (your recipes), the rest part two (cooking together).
	 */
	const STAGE_WORDS = [
		{ say: m.story_1_say, said: m.story_1_said, hold: 1500 },
		{ say: m.story_2_say, said: m.story_2_said, hold: 1500 },
		{ say: m.story_3_say, said: m.story_3_said, hold: 3600 },
		{ say: m.story_4_say, said: m.story_4_said, hold: 2400 },
		{ say: m.story_5_say, said: m.story_5_said, hold: 2800 },
		{ say: m.story_6_say, said: m.story_6_said, hold: 3200 },
		{ say: m.story_7_say, said: m.story_7_said, hold: 2600 },
		{ say: m.story_8_say, said: m.story_8_said, hold: 2600 },
		{ say: m.story_9_say, said: m.story_9_said, hold: 2800 },
		{ say: m.story_10_say, said: m.story_10_said, hold: 2400 },
		{ say: m.story_11_say, said: m.story_11_said, hold: 1800 },
	];
	const STAGES = STAGE_WORDS.length;
	const PART_ONE = 6;
	/** The diary's three cookings of the curry: when, and what was written. */
	const COOKINGS = {
		1: { date: m.story_date_1, note: m.story_note_1 },
		2: { date: m.story_date_2, note: m.story_note_2 },
		3: { date: m.story_date_3, note: m.story_note_3 },
	};

	let at = $state(0);
	let lang = $state<Locale>(getLocale());
	let hint = $state(false);
	const still =
		typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;
	const last = $derived(at === STAGES - 1);
	/** Every message in the story is asked for in the language on screen. */
	const L = $derived({ locale: lang });

	$effect(() => {
		story.showing = true;
		return () => {
			story.showing = false;
		};
	});

	$effect(() => {
		hint = false;
		if (last) return;
		const wait = setTimeout(() => (hint = true), (still ? 0 : STAGE_WORDS[at].hold) + HINT_AFTER);
		return () => clearTimeout(wait);
	});

	function go(to: number) {
		if (to < 0 || to >= STAGES || to === at) return;
		at = to;
	}

	/**
	 * Words in another language, on the same stage. `reload: false` because the
	 * story re-renders itself: Paraglide's default reload would throw the reader
	 * back to stage one. What it does still do is keep the choice, so everything
	 * drawn after the story opens in it.
	 */
	function speak(next: Locale) {
		lang = next;
		setLocale(next, { reload: false });
		document.documentElement.lang = next;
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'ArrowRight') go(at + 1);
		else if (event.key === 'ArrowLeft') go(at - 1);
		else if (event.key === ' ' && !(event.target instanceof HTMLButtonElement)) {
			event.preventDefault();
			go(at + 1);
		}
	}
</script>

<svelte:window {onkeydown} />

<div class="backdrop">
	<div class="story" class:still class:last>
		<div class="progress" aria-hidden="true">
			{#each { length: STAGES }, n (n)}
				<span class:done={n < at} class:now={n === at}><i></i></span>
			{/each}
		</div>

		<div class="chrome">
			<div class="langs" role="group" aria-label={m.story_languages({}, L)}>
				{#each locales as each, n (each)}
					{#if n > 0}<span aria-hidden="true">·</span>{/if}
					<button
						type="button"
						aria-pressed={each === lang}
						lang={each}
						onclick={() => speak(each)}
					>
						{each.toUpperCase()}
					</button>
				{/each}
			</div>
			{#if !last}
				<button type="button" class="corner" onclick={() => go(STAGES - 1)}>
					{m.story_skip({}, L)}
				</button>
			{:else if ending === 'about'}
				<button type="button" class="corner" onclick={() => onClose?.()}>
					{m.story_close({}, L)}
				</button>
			{/if}
		</div>

		<button
			type="button"
			class="tap tap-back"
			aria-label={m.story_back({}, L)}
			onclick={() => go(at - 1)}
		></button>
		<button
			type="button"
			class="tap tap-next"
			aria-label={m.story_next({}, L)}
			onclick={() => go(at + 1)}
		></button>

		<div class="stages" aria-live="polite">
			{#key at}
				<section class="stage st-{at + 1}">
					<div class="slot">
						<div class="scene">
							<div class="pic" aria-hidden="true">
								{#if at === 0}
									{@render phone('cookbook', false)}
									{@render diary([2, 1])}
								{:else if at === 1}
									{@render things(false)}
								{:else if at === 2}
									{@render phone('cookbook', true)}
									{@render things(true)}
								{:else if at === 3}
									{@render recipe('r-still', 'chillies')}
									{@render diary([2, 1])}
								{:else if at === 4}
									{@render recipe('r-still', 'chillies')}
									<div class="cooking">
										<small>{m.story_cooking({}, L)}</small>
										<b>{m.story_curry({}, L)}</b>
										<p class="cook-line">
											<span class="was">{m.story_chillies({}, L)}</span>
											<span class="now">{m.story_chilli({}, L)}</span>
										</p>
										<p class="cook-said">{m.cook_changed_it({}, L)}</p>
									</div>
									{@render diary([3, 2, 1])}
								{:else if at === 5}
									{@render recipe('r-old', 'chillies', m.story_version_1({}, L))}
									{@render recipe('r-new', 'chilli', m.story_version_2({}, L))}
									<div class="choice">
										<ol class="from">{@render entry(3)}</ol>
										<span class="keep"><span>{m.recipe_keep_as_version({}, L)}</span></span>
										<span class="leave">{m.recipe_leave_in_diary({}, L)}</span>
									</div>
								{:else if at === 6}
									<div class="kitchen">
										<span class="kitchen-name">{m.story_kitchen_name({}, L)}</span>
									</div>
									{@render mini('m-you', m.settings_your_cookbook({}, L), [
										['curry', m.story_curry({}, L)],
										['mark', m.story_mark({}, L)],
										['dal', m.story_dal({}, L)],
									])}
									{@render mini('m-sam', m.cookbook_called({ names: SAM }, L), [
										['lasagne', m.story_lasagne({}, L)],
										['bread', m.story_bread({}, L)],
										['pancake', m.story_pancakes({}, L)],
									])}
									{@render mini('m-ines', m.cookbook_called({ names: INES }, L), [
										['ramen', m.story_ramen({}, L)],
										['card', m.story_lemon({}, L)],
										['tiramisu', m.story_tiramisu({}, L)],
									])}
								{:else if at === 7}
									{@render phone('mixed', false)}
								{:else if at === 8}
									{@render recipe('r-yours', 'chilli', m.settings_your_cookbook({}, L))}
									<svg class="thread" viewBox="0 0 100 100" preserveAspectRatio="none">
										<path d="M22 50 C 20 70, 30 78, 44 80" pathLength="1" />
									</svg>
									{@render recipe('r-ines', 'chillies_3', m.cookbook_called({ names: INES }, L))}
								{:else if at === 9}
									<div class="ev ev-shop">
										<small>{m.shopping_title({}, L)}</small>
										<p>
											<i></i><span
												><b>{m.story_shop_1({}, L)}</b><em>{m.story_shop_1_from({}, L)}</em></span
											>
										</p>
										<p><i></i><span><b>{m.story_shop_2({}, L)}</b></span></p>
										<p><i></i><span><b>{m.story_shop_3({}, L)}</b></span></p>
									</div>
									<div class="ev ev-cook">
										<small>{m.story_step_of({}, L)}</small>
										<span class="sun"></span>
										<b>{m.story_awake({}, L)}</b>
									</div>
									<div class="ev ev-link">
										<span class="chain"></span>
										<code>kamosu…/s/Xk2p</code>
										<span class="copy">{m.share_copy({}, L)}</span>
									</div>
									<div class="ev ev-off">
										<span class="nowifi"></span>
										<b>{m.story_offline({}, L)}</b>
									</div>
								{:else}
									{@render phone('waiting', false)}
								{/if}
							</div>

							<!-- The stamps are the one part of a drawing that is read out: each is
						     an idea's own word, as the app says it (tested against the app's
						     messages, so a rename cannot leave the story behind). -->
							{#if at === 2}
								<p class="stamp s-cookbook"><span>{m.story_stamp_cookbook({}, L)}</span></p>
							{:else if at === 3}
								<p class="stamp s-diary"><span>{m.story_stamp_diary({}, L)}</span></p>
							{:else if at === 5}
								<p class="stamp s-version"><span>{m.story_stamp_version({}, L)}</span></p>
							{:else if at === 6}
								<p class="stamp s-kitchen"><span>{m.story_stamp_kitchen({}, L)}</span></p>
							{/if}
						</div>
					</div>

					<div class="words">
						<p class="kicker">
							{at < PART_ONE ? m.story_part_one({}, L) : m.story_part_two({}, L)}
							<span class="sr-only">· {m.story_where({ at: at + 1, count: STAGES }, L)}</span>
						</p>
						<h1 class="say">{STAGE_WORDS[at].say({}, L)}</h1>
						{#if last && ending === 'about'}
							<p class="said">{m.story_11_said_about({}, L)}</p>
						{:else}
							<p class="said">{STAGE_WORDS[at].said({}, L)}</p>
						{/if}
						{#if last && ending === 'invite'}
							<p class="cta-wrap">
								<button type="button" class="cta" onclick={() => onCreate?.()}>
									{m.account_invite_submit({}, L)}
								</button>
							</p>
						{/if}
					</div>
				</section>
			{/key}
		</div>

		<p class="hint" class:show={hint} aria-hidden="true">{m.story_hint({}, L)}</p>
	</div>
</div>

<!-- The screenshot, the bookmark, Mum's text and Inès's card: scattered on
     stage 2, and on stage 3 each tagged with how it gets in and gathered. -->
{#snippet things(gather: boolean)}
	<div class="obj shot" class:into={gather}>
		<div class="shot-bar"></div>
		<div class="shot-img"></div>
		<b>{m.story_shot({}, L)}</b><i></i><i></i>
		{#if gather}<span class="how">{m.story_via_ai({}, L)}</span>{/if}
	</div>
	<div class="obj mark" class:into={gather}>
		<div class="ribbon"></div>
		<div class="mark-bar"><u></u><u></u><u></u><span>cuisine-et-mots.fr</span></div>
		<div class="mark-body">
			<div class="mark-img"></div>
			<div class="mark-text">
				<b>{m.story_mark({}, L)}</b><em>★★★★★</em><i></i><i></i>
			</div>
		</div>
		{#if gather}<span class="how">{m.story_via_link({}, L)}</span>{/if}
	</div>
	<div class="obj msg" class:into={gather}>
		<small><span class="avatar"></span><span>{m.story_mum({}, L)}</span></small>
		<p class="bubble">{m.story_mum_1({}, L)}</p>
		<p class="bubble">{m.story_mum_2({}, L)}</p>
		{#if gather}<span class="how">{m.story_via_paste({}, L)}</span>{/if}
	</div>
	<div class="obj card" class:into={gather}>
		<b>{m.story_card({}, L)}</b><i></i><i></i><i></i>
		{#if gather}<span class="how">{m.story_via_typed({}, L)}</span>{/if}
	</div>
{/snippet}

<!-- Kamosu on a phone: your Cookbook, the Kitchen's mixed list, or an empty
     Cookbook waiting for an Invite's new account. -->
{#snippet phone(showing: 'cookbook' | 'mixed' | 'waiting', how: boolean)}
	<div class="phone" class:waiting={showing === 'waiting'}>
		<div class="phone-top">
			<img src="/assets/img/kamosu-mark.svg" alt="" /><span>Kamosu</span>
		</div>
		<p class="phone-title">
			{showing === 'mixed' ? m.recipes_title({}, L) : m.settings_your_cookbook({}, L)}
		</p>
		<ul class="rows">
			{#if showing === 'cookbook'}
				{@render row('shot', m.story_shot({}, L), how ? m.story_via_ai({}, L) : '')}
				{@render row('mark', m.story_mark({}, L), how ? m.story_via_link({}, L) : '')}
				{@render row('dal', m.story_dal({}, L), how ? m.story_via_paste({}, L) : '')}
				{@render row('card', m.story_card({}, L), how ? m.story_via_typed({}, L) : '')}
				{@render row('curry', m.story_curry({}, L), how ? m.story_via_own({}, L) : '')}
			{:else if showing === 'mixed'}
				{@render row('curry', m.story_curry({}, L), '')}
				{@render row('lasagne', m.story_lasagne({}, L), m.cookbook_whose({ names: SAM }, L), true)}
				{@render row('ramen', m.story_ramen({}, L), m.cookbook_whose({ names: INES }, L), true)}
				{@render row('mark', m.story_mark({}, L), '')}
				{@render row('bread', m.story_bread({}, L), m.cookbook_whose({ names: SAM }, L), true)}
			{:else}
				{#each { length: 5 }, n (n)}<li class="row empty"></li>{/each}
			{/if}
		</ul>
	</div>
{/snippet}

{#snippet row(picture: string, title: string, under: string, theirs = false)}
	<li class="row row-{picture}" class:theirs>
		<span class="thumb t-{picture}"></span>
		<span class="row-text"
			><b>{title}</b>{#if under}<small>{under}</small>{/if}</span
		>
	</li>
{/snippet}

<!-- The curry's recipe card. `label` is the small line on top: a Version's
     date on stage 6, whose Cookbook it is in on stage 9. -->
{#snippet recipe(kind: string, first: 'chillies' | 'chilli' | 'chillies_3', label?: string)}
	<div class="recipe {kind}">
		{#if label}<small>{label}</small>{/if}
		<b>{m.story_curry({}, L)}</b>
		<ul>
			<li class="changed">
				<span
					>{first === 'chilli'
						? m.story_chilli({}, L)
						: first === 'chillies'
							? m.story_chillies({}, L)
							: m.story_chillies_3({}, L)}</span
				>
			</li>
			<li>{m.story_coconut({}, L)}</li>
			<li>{m.story_onion({}, L)}</li>
			<li>{m.story_paste({}, L)}</li>
		</ul>
	</div>
{/snippet}

<!-- The diary: newest first, the way the Cooked screen lists it. -->
{#snippet diary(entries: (1 | 2 | 3)[])}
	<div class="diary">
		<ol>
			{#each entries as n (n)}{@render entry(n)}{/each}
		</ol>
	</div>
{/snippet}

{#snippet entry(n: 1 | 2 | 3)}
	<li class="entry e{n}">
		<span class="bowl"></span>
		<span class="entry-text">
			<small>{COOKINGS[n].date({}, L)}</small>
			<span class="hand">{COOKINGS[n].note({}, L)}</span>
			{#if n === 3}<em class="differently">{m.story_differently({}, L)}</em>{/if}
		</span>
	</li>
{/snippet}

{#snippet mini(kind: string, title: string, items: [string, string][])}
	<div class="mini {kind}">
		<p>{title}</p>
		<ul>
			{#each items as [picture, name] (picture)}
				<li><span class="thumb t-{picture}"></span><b>{name}</b></li>
			{/each}
		</ul>
	</div>
{/snippet}

<style>
	/* The story's own drawing. It is not Tailwind: the drawings are sized in
	   units of the square they are drawn in (cqw), which no utility scale has.
	   The faces, the paper, the ink and every colour Kamosu itself wears are
	   the app's tokens; only the food in the pictures (egg yolk, curry, crust,
	   shadow warmth) has colours of its own. */
	.backdrop {
		position: fixed;
		inset: 0;
		z-index: 40;
		background: var(--color-ground-2);
		overflow: hidden;
		/* Readers tap fast through a story: two taps are two stages, never a
		   zoom. Pinching still zooms, for whoever needs the words bigger. */
		touch-action: manipulation;
	}
	.backdrop :global(*) {
		touch-action: manipulation;
	}
	.story {
		position: relative;
		width: min(100vw, 430px);
		height: 100dvh;
		margin: 0 auto;
		overflow: hidden;
		isolation: isolate;
		user-select: none;
		-webkit-user-select: none;
		-webkit-tap-highlight-color: transparent;
		color: var(--color-ink);
		font-family: var(--font-sans);
	}
	@media (min-width: 600px) and (min-height: 700px) {
		/* On a computer: a phone-sized card centred on the page. */
		.backdrop {
			display: grid;
			place-items: center;
		}
		.story {
			height: min(100dvh - 48px, 880px);
			border-radius: var(--radius-md);
			box-shadow:
				0 1px 0 color-mix(in srgb, var(--color-ink) 5%, transparent),
				0 30px 60px -30px color-mix(in srgb, var(--color-ink) 45%, transparent);
		}
	}
	.cta {
		width: 100%;
	}

	/* Still: every animation here ends on the element's own style, so taking
	   the animations away draws each stage finished. */
	@media (prefers-reduced-motion: reduce) {
		.story *,
		.story *::before,
		.story *::after {
			animation: none !important;
			transition: none !important;
		}
	}
	.story.still *,
	.story.still *::before,
	.story.still *::after {
		animation: none !important;
		transition: none !important;
	}

	/* ---- progress ---- */
	.progress {
		position: absolute;
		inset: 10px 12px auto;
		display: flex;
		gap: 4px;
		z-index: 5;
	}
	.progress span {
		flex: 1;
		height: 2px;
		background: color-mix(in srgb, var(--color-ink) 14%, transparent);
		overflow: hidden;
	}
	.progress i {
		display: block;
		height: 100%;
		width: 0;
		background: var(--color-accent);
	}
	.progress .done i {
		width: 100%;
	}
	.progress .now i {
		width: 100%;
		animation: fill 900ms ease-out both;
	}
	@keyframes fill {
		from {
			width: 0;
		}
	}

	/* ---- corner controls ---- */
	.chrome {
		position: absolute;
		inset: 20px 8px auto;
		display: flex;
		justify-content: space-between;
		align-items: center;
		z-index: 5;
		pointer-events: none;
	}
	.chrome > * {
		pointer-events: auto;
	}
	.langs {
		display: flex;
		align-items: center;
		font-size: 12px;
		letter-spacing: 0.08em;
		color: var(--color-ink-2);
	}
	.langs span {
		opacity: 0.5;
	}
	.chrome button {
		font: inherit;
		font-size: 12px;
		color: var(--color-ink-2);
		background: none;
		border: 0;
		padding: 10px 7px;
		min-height: 40px;
		cursor: pointer;
		letter-spacing: 0.08em;
	}
	.langs button[aria-pressed='true'] {
		color: var(--color-ink);
		font-weight: 700;
		text-decoration: underline;
		text-underline-offset: 4px;
		text-decoration-thickness: 1px;
	}
	.corner {
		letter-spacing: 0.04em !important;
	}
	/* Under the pointer (#203). The story draws itself and `app.css` does not
	   reach its buttons, so the same shades are said again here: the darker
	   paper under the small words, the lighter indigo on the one button. The
	   two tap zones cover the page and stay as they are. */
	@media (hover: hover) {
		.chrome button:hover {
			background: var(--color-ground-2);
		}
		.cta:hover {
			background: var(--color-accent-2);
		}
	}
	button:focus-visible {
		outline: 2px solid var(--color-accent);
		outline-offset: -4px;
	}

	/* ---- tap zones: left edge goes back, the rest goes forward ---- */
	.tap {
		position: absolute;
		top: 64px;
		bottom: 0;
		z-index: 4;
		background: none;
		border: 0;
		padding: 0;
		cursor: pointer;
	}
	.tap-back {
		left: 0;
		width: 28%;
	}
	.tap-next {
		right: 0;
		width: 72%;
	}
	.story.last .tap-next {
		cursor: default;
	}

	/* ---- a stage ---- */
	.stages {
		position: absolute;
		inset: 0;
	}
	.stage {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		padding: 68px 0 64px;
	}
	.slot {
		flex: 1;
		min-height: 0;
		container-type: size;
		display: grid;
		place-items: center;
		position: relative;
	}
	.scene {
		position: relative;
		width: min(100cqw - 2 * var(--spacing-gutter), 100cqh, 380px);
		aspect-ratio: 1;
		container-type: inline-size;
	}
	.words {
		/* the width of the window, less the screen gutter */
		padding: 12px var(--spacing-gutter) 0;
		flex: none;
	}
	.kicker {
		margin: 0 0 10px;
		font-size: var(--text-label);
		letter-spacing: var(--text-label--letter-spacing);
		text-transform: uppercase;
		color: var(--color-ink-2);
	}
	.say {
		margin: 0;
		font-family: var(--font-display);
		font-weight: 600;
		font-size: var(--text-step);
		line-height: var(--text-step--line-height);
		letter-spacing: var(--text-step--letter-spacing);
		text-wrap: pretty;
	}
	.said {
		margin: 10px 0 0;
		font-size: var(--text-body);
		line-height: var(--text-body--line-height);
		color: var(--color-ink-2);
		text-wrap: pretty;
	}
	.words > * {
		animation: words 700ms ease-out both;
		animation-delay: var(--words-at, 300ms);
	}
	.words .said {
		animation-delay: calc(var(--words-at, 300ms) + 250ms);
	}
	@keyframes words {
		from {
			opacity: 0;
			transform: translateY(6px);
		}
	}

	/* A short screen: a little less padding round the stage. */
	@media (max-height: 720px) {
		.stage {
			padding: 60px 0 44px;
		}
	}

	/* ---- the hint ---- */
	.hint {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 18px;
		margin: 0;
		text-align: center;
		font-size: 12px;
		letter-spacing: 0.06em;
		color: var(--color-ink-2);
		opacity: 0;
		transition: opacity 400ms;
		pointer-events: none;
		z-index: 3;
	}
	.hint.show {
		opacity: 1;
		animation: pulse 1.8s ease-in-out infinite;
	}
	@keyframes pulse {
		50% {
			opacity: 0.35;
		}
	}

	/* What the drawn things are and where they sit; how they move is further
	   down. Sizes are in cqw of the square scene, so the whole drawing scales
	   as one. */
	.pic {
		position: absolute;
		inset: 0;
	}
	.obj,
	.recipe,
	.phone,
	.diary,
	.cooking,
	.choice {
		position: absolute;
		transform: rotate(var(--r, 0deg));
		color: var(--color-ink);
	}
	.obj,
	.recipe,
	.diary,
	.choice {
		background: var(--color-card);
		box-shadow: var(--lift);
	}

	/* ------------------------------------------ stage 2: the mess, also flown on 3 */

	/* the screenshot */
	.shot {
		--r: -7deg;
		left: 5%;
		top: 7%;
		width: 29%;
		height: 50%;
		border-radius: 3cqw;
		padding: 2cqw;
		display: flex;
		flex-direction: column;
		gap: 1.4cqw;
		--dx: 30;
		--dy: -4;
	}
	.shot-bar {
		height: 1cqw;
		width: 35%;
		margin: 0 auto 1cqw;
		border-radius: 1cqw;
		background: var(--color-ink);
		opacity: 0.8;
	}
	.shot-img,
	.t-shot {
		aspect-ratio: 1;
		border-radius: 50%;
		background:
			radial-gradient(
				circle at 36% 38%,
				#f7f1df 0 6%,
				#e7b64a 6.5% 10%,
				#f7f1df 10.5% 17%,
				transparent 17.5%
			),
			radial-gradient(
				circle at 64% 45%,
				#f7f1df 0 6%,
				#e7b64a 6.5% 10%,
				#f7f1df 10.5% 17%,
				transparent 17.5%
			),
			radial-gradient(
				circle at 46% 68%,
				#f7f1df 0 6%,
				#e7b64a 6.5% 10%,
				#f7f1df 10.5% 17%,
				transparent 17.5%
			),
			radial-gradient(circle at 70% 70%, var(--color-support-2) 0 3%, transparent 3.5%),
			radial-gradient(
				circle,
				var(--color-support) 0 62%,
				#8e3336 63% 67%,
				var(--color-ink) 68% 72%,
				transparent 73%
			);
	}
	.shot b {
		font-size: 3.4cqw;
		font-weight: 700;
		margin-top: 0.6cqw;
	}
	.shot i,
	.mark-text i,
	.card i {
		display: block;
		height: 0.9cqw;
		border-radius: 1cqw;
		background: var(--color-rule);
	}
	.shot i + i {
		width: 70%;
	}

	/* the bookmark: a printed web page with a ribbon over its edge */
	.mark {
		--r: 4deg;
		right: 3%;
		top: 5%;
		width: 52%;
		height: 26%;
		border-radius: 0.6cqw;
		--dx: -21;
		--dy: 23;
	}
	.mark-bar {
		display: flex;
		align-items: center;
		gap: 0.8cqw;
		padding: 1.3cqw 1.6cqw;
		border-bottom: 1px solid var(--color-rule);
		font-size: 2.3cqw;
		color: var(--color-ink-2);
	}
	.mark-bar u {
		width: 1.3cqw;
		height: 1.3cqw;
		border-radius: 50%;
		background: var(--color-rule);
	}
	.mark-bar span {
		margin-left: 1cqw;
		background: var(--color-ground);
		padding: 0.3cqw 1.2cqw;
		border-radius: 0.6cqw;
		white-space: nowrap;
	}
	.mark-body {
		display: flex;
		gap: 2cqw;
		padding: 1.8cqw;
	}
	.mark-img,
	.t-mark {
		background:
			radial-gradient(ellipse at 50% 58%, #c77b3a 0 34%, #9b5623 35% 40%, transparent 41%),
			radial-gradient(ellipse at 50% 64%, #e9dfcb 0 44%, transparent 45%), var(--color-support-2);
	}
	.mark-img {
		width: 36%;
		aspect-ratio: 1.1;
		border-radius: 0.4cqw;
	}
	.mark-text {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 1cqw;
		min-width: 0;
	}
	.mark-text b {
		font-family: var(--font-display);
		font-weight: 600;
		font-size: 3.6cqw;
		line-height: 1.1;
	}
	.mark-text em {
		font-style: normal;
		font-size: 2.4cqw;
		letter-spacing: 0.1em;
		color: #d49a2c;
	}
	.mark-text i + i {
		width: 60%;
	}
	.ribbon {
		position: absolute;
		right: 9%;
		top: -2.6cqw;
		width: 4cqw;
		height: 9cqw;
		background: var(--color-support);
		clip-path: polygon(0 0, 100% 0, 100% 100%, 50% 80%, 0 100%);
	}

	/* the text from her mum */
	.msg {
		--r: 3deg;
		left: 7%;
		bottom: 4%;
		width: 44%;
		height: 38%;
		border-radius: 2.4cqw;
		padding: 2.4cqw 2.6cqw;
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 1.4cqw;
		--dx: 21;
		--dy: -23;
	}
	.msg small {
		display: flex;
		align-items: center;
		gap: 1.2cqw;
		align-self: stretch;
		font-size: 2.8cqw;
		font-weight: 700;
		padding-bottom: 1.4cqw;
		border-bottom: 1px solid var(--color-rule);
	}
	.avatar {
		width: 4.4cqw;
		height: 4.4cqw;
		border-radius: 50%;
		background:
			radial-gradient(circle at 50% 38%, var(--color-card) 0 22%, transparent 23%),
			radial-gradient(ellipse at 50% 100%, var(--color-card) 0 42%, transparent 43%),
			var(--color-support-2);
	}
	.bubble {
		margin: 0;
		max-width: 92%;
		font-size: 2.7cqw;
		line-height: 1.3;
		background: var(--color-ground-2);
		padding: 1cqw 1.8cqw;
		border-radius: 2.2cqw 2.2cqw 2.2cqw 0.6cqw;
	}
	.t-curry,
	.entry .bowl {
		border-radius: 50%;
		background:
			radial-gradient(circle at 38% 40%, #f3ecd8 0 8%, transparent 8.5%),
			radial-gradient(circle at 62% 60%, #f3ecd8 0 7%, transparent 7.5%),
			radial-gradient(circle at 58% 32%, var(--color-support) 0 5%, transparent 5.5%),
			radial-gradient(circle, #a4ad6a 0 56%, #fffdf8 57% 70%, #c9c0ad 71% 74%, transparent 75%);
	}
	.t-curry {
		background-color: var(--color-ground-2);
		border-radius: var(--radius-sm);
	}
	.t-dal {
		background:
			radial-gradient(circle at 60% 40%, #f3e2b5 0 6%, transparent 6.5%),
			radial-gradient(circle, #d9953c 0 60%, #fffdf8 61% 72%, transparent 73%),
			var(--color-ground-2);
	}

	/* Inès's card, written by hand */
	.card {
		--r: -3deg;
		right: 4%;
		bottom: 11%;
		width: 49%;
		height: 30%;
		border-radius: 0.4cqw;
		padding: 3.2cqw 3cqw 0;
		background-image:
			linear-gradient(var(--color-support), var(--color-support)),
			repeating-linear-gradient(to bottom, transparent 0 5.4cqw, #c9d3e0 5.4cqw calc(5.4cqw + 1px));
		background-size:
			100% 1.5px,
			100% 100%;
		background-position:
			0 3cqw,
			0 3.6cqw;
		background-repeat: no-repeat;
		--dx: -21;
		--dy: -7;
	}
	.card b {
		display: block;
		margin-top: 1.2cqw;
		font-family: var(--font-display);
		font-weight: 400;
		font-size: 3.8cqw;
		line-height: 1.4;
		color: var(--color-accent);
		transform: skewX(-9deg);
		transform-origin: left;
	}
	.card i {
		height: 3cqw;
		margin-top: 2.4cqw;
		background: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 90 12'%3E%3Cpath d='M2 8c3-5 6-5 8 0s4 4 6-1 5-4 7 1 3 3 6-2 4-3 7 1 2 3 5-1 4-4 7 0 3 3 5-1 3-3 6 0 4 2 6-2 3-3 5 1 3 2 6-1' fill='none' stroke='%231d2b4c' stroke-width='1.3' stroke-linecap='round'/%3E%3C/svg%3E")
			left / auto 100% repeat-x;
		border-radius: 0;
		opacity: 0.8;
	}
	.card i:nth-of-type(2) {
		width: 72%;
	}
	.card i:nth-of-type(3) {
		width: 48%;
	}
	.t-card {
		background:
			radial-gradient(ellipse at 50% 60%, #f1d97a 0 36%, #d8b64a 37% 41%, transparent 42%),
			var(--color-card);
		box-shadow: inset 0 0 0 1px var(--color-rule);
	}

	/* ----------------------------- stages 1, 3, 8 and 11: Kamosu on a phone, and 3's tags */

	/* how a thing got in: a paper tag tied to it */
	.how {
		position: absolute;
		left: 50%;
		bottom: -2.4cqw;
		transform: translateX(-50%) rotate(-3deg);
		white-space: nowrap;
		font-size: 2.9cqw;
		font-weight: 700;
		line-height: 1;
		color: var(--color-accent);
		background: var(--color-card);
		padding: 1.1cqw 1.8cqw 1.1cqw 3.4cqw;
		border-radius: var(--radius-sm);
		box-shadow:
			inset 0 0 0 1px var(--color-accent),
			0 0.8cqw 1.6cqw -0.8cqw rgb(60 45 20 / 0.5);
	}
	.how::before {
		content: '';
		position: absolute;
		left: 1.2cqw;
		top: 50%;
		width: 1.1cqw;
		height: 1.1cqw;
		margin-top: -0.55cqw;
		border-radius: 50%;
		box-shadow: inset 0 0 0 1px var(--color-accent);
	}

	.shot .how {
		bottom: auto;
		top: -2.4cqw;
	}
	.phone {
		left: 23%;
		top: 0;
		width: 54%;
		height: 100%;
		border-radius: 5.5cqw;
		padding: 3cqw 3.2cqw;
		overflow: hidden;
		background: var(--color-ground);
		box-shadow:
			inset 0 0 0 0.9cqw var(--color-ink),
			inset 0 0 0 1.3cqw #3a3a3e,
			var(--book-lift);
	}
	.phone-top {
		display: flex;
		align-items: center;
		gap: 1.4cqw;
		padding: 2cqw 1.4cqw 2.2cqw;
		border-bottom: 1px solid var(--color-rule);
	}
	.phone-top img {
		width: 4.6cqw;
		height: 4.6cqw;
	}
	.phone-top span {
		font-family: var(--font-display);
		font-weight: 600;
		font-size: 3.6cqw;
	}
	.phone-title {
		margin: 0;
		padding: 3cqw 1.4cqw 2cqw;
		font-family: var(--font-display);
		font-weight: 600;
		font-size: 4.6cqw;
		line-height: 1.1;
	}
	.rows {
		list-style: none;
		margin: 0;
		padding: 0 1cqw;
		display: flex;
		flex-direction: column;
		gap: 1.4cqw;
	}
	.row {
		display: flex;
		align-items: center;
		gap: 2cqw;
		background: var(--color-card);
		padding: 1.3cqw;
		border-radius: var(--radius-sm);
		box-shadow: 0 0 0 1px var(--color-rule);
	}
	.thumb {
		flex: none;
		width: 9cqw;
		height: 9cqw;
		border-radius: var(--radius-sm);
	}
	.t-shot {
		border-radius: var(--radius-sm);
		background-color: var(--color-ground-2);
		background-size: 130% 130%;
		background-position: center;
	}
	.row-text {
		display: flex;
		flex-direction: column;
		gap: 0.5cqw;
		min-width: 0;
	}
	.row-text b {
		font-size: 3cqw;
		font-weight: 700;
		line-height: 1.15;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.row-text small {
		font-size: 2.3cqw;
		color: var(--color-ink-2);
		line-height: 1.1;
	}
	.into {
		opacity: 0;
		transform: translate(calc(var(--dx) * 1cqw), calc(var(--dy) * 1cqw)) scale(0.12);
	}

	/* ------------------------------------------------ stages 4 to 6: the curry */

	.recipe {
		left: 3%;
		top: 1%;
		width: 58%;
		height: 50%;
		padding: 3.6cqw 4cqw;
		border-radius: 0.5cqw;
	}
	.recipe small {
		position: absolute;
		right: 3.4cqw;
		top: 2.6cqw;
		font-size: 2.4cqw;
		letter-spacing: 0.12em;
		text-transform: uppercase;
		color: var(--color-ink-2);
	}
	.recipe b {
		display: block;
		font-family: var(--font-display);
		font-weight: 600;
		font-size: 5cqw;
		line-height: 1.1;
		padding: 1.6cqw 0 2.2cqw;
		border-bottom: 1px solid var(--color-rule);
		margin-bottom: 1.6cqw;
	}
	.recipe ul {
		list-style: none;
		margin: 0;
		padding: 0;
		font-size: 3.4cqw;
		line-height: 1.75;
	}
	.recipe li span {
		padding: 0 0.6cqw;
		margin: 0 -0.6cqw;
	}

	/* the diary: newest first, the way the Cooked screen lists it */
	.diary {
		left: 0;
		right: 0;
		top: 55%;
		border-radius: 0.5cqw;
		padding: 3cqw 3.4cqw 1cqw;
		background-image: radial-gradient(circle, var(--color-ground-2) 0 0.9cqw, transparent 1cqw);
		background-size: 5cqw 3cqw;
		background-repeat: repeat-x;
		background-position: 1.5cqw 0.4cqw;
	}
	.diary ol {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.entry {
		display: flex;
		align-items: center;
		gap: 2.6cqw;
		max-height: 20cqw;
		padding: 1.8cqw 0;
		border-top: 1px solid var(--color-rule);
		overflow: hidden;
	}
	.entry:first-child {
		border-top: 0;
	}
	.entry .bowl {
		flex: none;
		width: 8cqw;
		height: 8cqw;
	}
	.entry-text {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.6cqw;
		min-width: 0;
	}
	.entry small {
		font-size: 2.3cqw;
		letter-spacing: 0.12em;
		text-transform: uppercase;
		color: var(--color-ink-2);
	}
	.hand {
		font-family: var(--font-display);
		font-size: 3.7cqw;
		line-height: 1.2;
		color: var(--color-accent);
		transform: skewX(-9deg);
		transform-origin: left;
	}
	.differently {
		font-style: normal;
		font-size: 2.3cqw;
		font-weight: 700;
		color: var(--color-support);
		box-shadow: inset 0 0 0 1px var(--color-support);
		padding: 0.4cqw 1.2cqw;
		border-radius: var(--radius-sm);
		margin-top: 0.4cqw;
	}

	/* cooking mode: the one indigo room */
	.cooking {
		--r: 2.5deg;
		right: 1%;
		top: 5%;
		width: 48%;
		padding: 3cqw 3.4cqw;
		border-radius: 0.8cqw;
		background: var(--color-cook-ground);
		color: var(--color-cook-ink);
		box-shadow: var(--book-lift);
	}
	.cooking small {
		display: block;
		font-size: 2.3cqw;
		letter-spacing: 0.15em;
		text-transform: uppercase;
		color: var(--color-cook-ink-2);
	}
	.cooking b {
		display: block;
		font-family: var(--font-display);
		font-weight: 600;
		font-size: 4.4cqw;
		margin: 1cqw 0 2.4cqw;
	}
	.cook-line {
		margin: 0;
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 0.4cqw 2cqw;
		font-size: 3.8cqw;
	}
	.was {
		color: var(--color-cook-ink-2);
		background: linear-gradient(currentColor, currentColor) no-repeat 0 55% / 100% 1.5px;
	}
	.now {
		font-family: var(--font-display);
		font-weight: 600;
		font-size: 4.6cqw;
	}
	.cook-said {
		margin: 2.4cqw 0 0;
		font-size: 2.6cqw;
		color: var(--color-cook-ink-2);
	}
	.cook-said::before {
		content: '✓ ';
	}

	/* two Versions, and the choice that made the second */
	.r-old {
		transform: translate(-3cqw, -1cqw) rotate(-5deg);
		filter: brightness(0.95) saturate(0.85);
	}
	.r-new {
		left: 32%;
		top: 9%;
		--r: 1.5deg;
	}
	.r-new .changed span {
		background: linear-gradient(
				transparent 55%,
				color-mix(in srgb, var(--color-support) 28%, transparent) 55% 92%,
				transparent 92%
			)
			no-repeat 0 0 / 100% 100%;
		font-weight: 700;
	}
	.choice {
		left: 0;
		right: 0;
		bottom: 1%;
		padding: 3cqw;
		border-radius: 0.6cqw;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 2cqw;
		padding-top: 1.6cqw;
	}
	.from {
		align-self: stretch;
		list-style: none;
		margin: 0;
		padding: 0;
		border-bottom: 1px solid var(--color-rule);
	}
	.keep {
		align-self: stretch;
		text-align: center;
		font-size: 3.4cqw;
		font-weight: 700;
		color: var(--color-on-accent);
		background: var(--color-accent);
		padding: 2.4cqw;
		border-radius: var(--radius-sm);
		box-shadow: 0 0 0 0.9cqw color-mix(in srgb, var(--color-accent) 22%, transparent);
	}
	.keep > span::before {
		content: '✓ ';
	}
	.leave {
		font-size: 3cqw;
		color: var(--color-ink-2);
		text-decoration: underline;
		text-underline-offset: 0.6cqw;
		opacity: 0.6;
	}

	/* stage 1: the phone to one side, a page of the diary beside it */
	.st-1 .phone {
		left: 3%;
		width: 52%;
		--r: -2deg;
	}
	.st-1 .row-text small {
		display: none;
	}
	.st-1 .diary {
		left: 44%;
		right: 1%;
		top: 34%;
		--r: 3deg;
	}

	/* ------------------------------------------------ where each stamp lands */
	.stamp {
		position: absolute;
		margin: 0;
		z-index: 2;
	}
	.s-cookbook {
		right: 1%;
		bottom: 9%;
	}
	.s-diary {
		right: 4%;
		top: 42%;
	}
	.s-version {
		right: 3%;
		top: 53%;
	}

	/* ======================================================= part two */

	/* more of other people's food */
	.t-lasagne {
		background:
			repeating-linear-gradient(
				to bottom,
				#f1e3c4 0 12%,
				var(--color-support) 12% 22%,
				#e8c98a 22% 30%
			),
			var(--color-card);
		box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--color-ink) 15%, transparent);
	}
	.t-bread {
		background:
			linear-gradient(to bottom, transparent 22%, #8a5a2b 22% 30%, #b98549 30% 90%, transparent 90%)
				50% 0 / 76% 100% no-repeat,
			var(--color-ground-2);
	}
	.t-pancake {
		background:
			radial-gradient(ellipse at 50% 40%, #e2b86a 0 34%, transparent 35%),
			radial-gradient(ellipse at 50% 55%, #d9a24f 0 38%, transparent 39%),
			radial-gradient(ellipse at 50% 70%, #c98d3f 0 40%, transparent 41%), var(--color-ground-2);
	}
	.t-ramen {
		background:
			radial-gradient(circle at 60% 42%, #f5c542 0 7%, #fffdf8 7.5% 14%, transparent 14.5%),
			radial-gradient(circle at 36% 58%, var(--color-support-2) 0 5%, transparent 5.5%),
			radial-gradient(circle, #c9894a 0 56%, var(--color-ink) 57% 68%, transparent 69%),
			var(--color-ground-2);
	}
	.t-tiramisu {
		background:
			linear-gradient(
					to bottom,
					#6b4a33 0 22%,
					#f3e7cf 22% 48%,
					#a8784f 48% 60%,
					#f3e7cf 60% 84%,
					#6b4a33 84%
				)
				50% 50% / 80% 70% no-repeat,
			var(--color-ground-2);
	}

	/* stage 7: three Cookbooks side by side, in one Kitchen */
	.kitchen {
		position: absolute;
		left: 1%;
		right: 1%;
		top: 8%;
		bottom: 10%;
		border-radius: var(--radius-md);
		background: color-mix(in srgb, var(--color-accent) 5%, transparent);
		box-shadow: inset 0 0 0 1.5px var(--color-accent);
	}
	.kitchen-name {
		position: absolute;
		left: 4cqw;
		top: -3.2cqw;
		font-size: 3.2cqw;
		font-weight: 700;
		color: var(--color-on-accent);
		background: var(--color-accent);
		padding: 1.2cqw 2.6cqw;
		border-radius: var(--radius-sm);
	}
	.mini {
		position: absolute;
		top: 16%;
		width: 30%;
		height: 64%;
		padding: 2.6cqw 2cqw;
		border-radius: 3.6cqw;
		background: var(--color-ground);
		box-shadow:
			inset 0 0 0 0.7cqw var(--color-ink),
			var(--book-lift);
		transform: rotate(var(--r, 0deg));
	}
	.m-you {
		left: 3%;
		--r: -2deg;
	}
	.m-sam {
		left: 35%;
	}
	.m-ines {
		left: 67%;
		--r: 2deg;
	}
	.mini p {
		margin: 0.6cqw 0.6cqw 2cqw;
		font-family: var(--font-display);
		font-weight: 600;
		font-size: 3cqw;
		line-height: 1.15;
		min-height: 7cqw;
	}
	.mini ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 1.4cqw;
	}
	.mini li {
		display: flex;
		align-items: center;
		gap: 1.2cqw;
		background: var(--color-card);
		padding: 0.9cqw;
		border-radius: var(--radius-sm);
		box-shadow: 0 0 0 1px var(--color-rule);
	}
	.mini .thumb {
		width: 6cqw;
		height: 6cqw;
	}
	.mini b {
		font-size: 2.2cqw;
		font-weight: 700;
		line-height: 1.15;
		min-width: 0;
	}
	.s-kitchen {
		right: 4%;
		bottom: 2%;
	}

	/* stage 8: one list, everyone's recipes, each marked with whose it is */
	.theirs small {
		color: var(--color-accent);
		font-weight: 700;
	}
	.theirs {
		box-shadow:
			0 0 0 1px var(--color-rule),
			inset 0.8cqw 0 0 var(--color-support-2);
	}

	/* stage 9: Inès's own curry, started from yours */
	.recipe.r-yours,
	.recipe.r-ines {
		width: 55%;
		height: 52%;
	}
	.r-yours {
		left: 3%;
		top: 1%;
	}
	.r-ines {
		left: 42%;
		top: 44%;
		--r: 2deg;
	}
	.r-yours small,
	.r-ines small {
		position: static;
		display: block;
		color: var(--color-accent);
		font-weight: 700;
	}
	.r-yours .changed span,
	.r-ines .changed span {
		font-weight: 700;
	}
	.r-ines .changed span {
		background: linear-gradient(
				transparent 55%,
				color-mix(in srgb, var(--color-support) 28%, transparent) 55% 92%,
				transparent 92%
			)
			no-repeat 0 0 / 100% 100%;
	}
	.thread {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		overflow: visible;
	}
	.thread path {
		fill: none;
		stroke: var(--color-accent);
		stroke-width: 1.5;
		stroke-dasharray: 0.03 0.025;
		stroke-linecap: round;
		vector-effect: non-scaling-stroke;
	}

	/* stage 10: the everyday things */
	.ev {
		position: absolute;
		width: 48.5%;
		padding: 3cqw;
		border-radius: 0.6cqw;
		background: var(--color-card);
		box-shadow: var(--lift);
		transform: rotate(var(--r, 0deg));
		color: var(--color-ink);
	}
	.ev small {
		display: block;
		font-size: 2.3cqw;
		letter-spacing: 0.15em;
		text-transform: uppercase;
		color: var(--color-ink-2);
		margin-bottom: 1.6cqw;
	}
	.ev-shop {
		left: 0;
		top: 2%;
		--r: -2deg;
	}
	.ev-shop p {
		margin: 0 0 1.4cqw;
		display: flex;
		gap: 1.6cqw;
		align-items: flex-start;
		font-size: 2.9cqw;
		line-height: 1.2;
	}
	.ev-shop i {
		flex: none;
		width: 2.8cqw;
		height: 2.8cqw;
		border-radius: var(--radius-sm);
		box-shadow: inset 0 0 0 1px var(--color-ink-2);
	}
	.ev-shop span {
		display: flex;
		flex-direction: column;
	}
	.ev-shop em {
		font-style: normal;
		font-size: 2.3cqw;
		color: var(--color-support-2);
		font-weight: 700;
	}
	.ev-cook {
		right: 0;
		top: 5%;
		--r: 2deg;
		background: var(--color-cook-ground);
		color: var(--color-cook-ink);
		box-shadow: var(--book-lift);
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 2cqw;
	}
	.ev-cook small {
		color: var(--color-cook-ink-2);
		margin: 0;
	}
	.sun {
		width: 11cqw;
		height: 11cqw;
		border-radius: 50%;
		background:
			radial-gradient(circle, #ffd98a 0 34%, transparent 35%),
			repeating-conic-gradient(#ffd98a 0 6deg, transparent 6deg 30deg);
		-webkit-mask: radial-gradient(
			circle,
			#000 0 34%,
			transparent 35% 46%,
			#000 47% 70%,
			transparent 71%
		);
		mask: radial-gradient(circle, #000 0 34%, transparent 35% 46%, #000 47% 70%, transparent 71%);
	}
	.ev-cook b {
		font-family: var(--font-display);
		font-weight: 600;
		font-size: 3.6cqw;
		line-height: 1.15;
	}
	.ev-link {
		left: 0;
		top: 54%;
		--r: 1.5deg;
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 2cqw;
	}
	.chain {
		position: relative;
		width: 10cqw;
		height: 6cqw;
	}
	.chain::before,
	.chain::after {
		content: '';
		position: absolute;
		top: 1cqw;
		width: 6cqw;
		height: 3.4cqw;
		border-radius: 1.8cqw;
		box-shadow: inset 0 0 0 0.7cqw var(--color-accent);
		transform: rotate(-30deg);
	}
	.chain::before {
		left: 0;
	}
	.chain::after {
		left: 3.8cqw;
	}
	.ev-link code {
		font-size: 2.6cqw;
		color: var(--color-ink-2);
		background: var(--color-ground-2);
		padding: 0.8cqw 1.6cqw;
		border-radius: var(--radius-sm);
	}
	.copy {
		font-size: 2.7cqw;
		font-weight: 700;
		color: var(--color-on-accent);
		background: var(--color-accent);
		padding: 1.2cqw 2.2cqw;
		border-radius: var(--radius-sm);
	}
	.ev-off {
		right: 0;
		top: 52%;
		--r: -2deg;
		display: flex;
		flex-direction: column;
		gap: 2cqw;
	}
	.nowifi {
		position: relative;
		width: 11cqw;
		height: 8cqw;
		background: radial-gradient(
			circle at 50% 100%,
			var(--color-ink-2) 0 12%,
			transparent 13% 30%,
			var(--color-ink-2) 31% 42%,
			transparent 43% 58%,
			var(--color-ink-2) 59% 70%,
			transparent 71%
		);
		opacity: 0.8;
		clip-path: polygon(0 0, 100% 0, 100% 100%, 0 100%);
	}
	.nowifi::after {
		content: '';
		position: absolute;
		left: 10%;
		right: 10%;
		top: 45%;
		height: 0.8cqw;
		background: var(--color-support);
		transform: rotate(-35deg);
		border-radius: 1cqw;
	}
	.ev-off b {
		font-size: 2.9cqw;
		line-height: 1.3;
		font-weight: 700;
	}

	/* stage 11: an empty Cookbook, waiting */
	.waiting .empty {
		height: 11cqw;
		background: none;
		box-shadow: none;
		border: 1px dashed var(--color-rule);
	}
	.cta-wrap {
		margin: 18px 0 0;
		position: relative;
		z-index: 6;
	}
	.cta {
		display: block;
		text-align: center;
		font-size: 16px;
		font-weight: 700;
		color: var(--color-on-accent);
		background: var(--color-accent);
		padding: 14px;
		border-radius: var(--radius-sm);
		text-decoration: none;
	}

	/* A · Paper. The kinari ground; recipes are things you could pick up.
   They drop and settle with a little weight, and each idea's word is a
   printed label pressed down onto the scene. */
	.story {
		--outside: var(--color-ground-2);
		--lift:
			0 0.3cqw 0 color-mix(in srgb, var(--color-ink) 5%, transparent),
			0 1.6cqw 3.2cqw -1.4cqw rgb(60 45 20 / 0.45);
		--book-lift:
			0 0.4cqw 0 color-mix(in srgb, var(--color-ink) 20%, transparent),
			0 3cqw 5cqw -2cqw color-mix(in srgb, var(--color-ink) 55%, transparent);
	}
	.story {
		background:
			radial-gradient(120% 70% at 50% 35%, rgb(255 253 248 / 0.7), transparent 70%),
			var(--color-ground);
	}

	/* the label a stamp is, in paper: letterpress on a card, pressed down */
	.stamp span {
		display: inline-block;
		font-family: var(--font-display);
		font-weight: 600;
		font-size: 6.2cqw;
		line-height: 1;
		color: var(--color-accent);
		background: var(--color-card);
		padding: 2cqw 3.4cqw 2.3cqw;
		border-radius: var(--radius-sm);
		box-shadow:
			inset 0 0 0 0.6cqw var(--color-card),
			inset 0 0 0 0.9cqw var(--color-accent),
			0 1cqw 2cqw -1cqw rgb(60 45 20 / 0.6);
		transform: rotate(-3deg);
	}

	/* ---- motion: things with weight ---- */
	@keyframes drop {
		from {
			opacity: 0;
			transform: translateY(-16cqw) rotate(calc(var(--r, 0deg) - 10deg));
		}
		55% {
			opacity: 1;
		}
	}
	@keyframes rise {
		from {
			opacity: 0;
			transform: translateY(10cqw) rotate(var(--r, 0deg));
		}
	}
	@keyframes gather {
		from {
			opacity: 1;
			transform: rotate(var(--r, 0deg));
		}
		80% {
			opacity: 1;
		}
	}
	@keyframes tie {
		from {
			opacity: 0;
			transform: translateX(-50%) translateY(-2cqw) rotate(-14deg) scale(1.2);
		}
	}
	@keyframes appear {
		from {
			opacity: 0;
			transform: scale(0.9);
		}
	}
	@keyframes press {
		from {
			opacity: 0;
			transform: translateY(-3cqw) scale(1.25) rotate(-8deg);
			box-shadow: 0 5cqw 6cqw -2cqw rgb(60 45 20 / 0.35);
		}
		60% {
			opacity: 1;
		}
	}
	@keyframes grow {
		from {
			max-height: 0;
			padding-block: 0;
			opacity: 0;
			border-color: transparent;
		}
		60% {
			opacity: 0;
		}
	}
	@keyframes slide-in {
		from {
			opacity: 0;
			transform: translate(50cqw, 4cqw) rotate(10deg);
		}
		30% {
			opacity: 1;
		}
	}
	@keyframes strike {
		from {
			background-size: 0 1.5px;
			color: var(--color-cook-ink);
		}
	}
	@keyframes lift-from-diary {
		from {
			opacity: 0;
			transform: translateY(40cqw) scale(0.45) rotate(0deg);
		}
		35% {
			opacity: 1;
		}
	}
	@keyframes step-back {
		from {
			transform: none;
			filter: none;
		}
	}
	@keyframes mark {
		from {
			background-size: 0 100%;
		}
	}
	@keyframes tap {
		0%,
		55% {
			box-shadow: 0 0 0 0 color-mix(in srgb, var(--color-accent) 0%, transparent);
			transform: none;
		}
		70% {
			transform: scale(0.95);
		}
	}
	@keyframes tick {
		0%,
		70% {
			opacity: 0;
		}
	}

	/* 1 · what it is: your Cookbook in Kamosu, and a page of your diary */
	.st-1 .phone {
		animation: rise 700ms 150ms cubic-bezier(0.2, 0.7, 0.3, 1) both;
	}
	.st-1 .diary {
		animation: slide-in 800ms 700ms cubic-bezier(0.25, 0.8, 0.3, 1.08) both;
	}
	.st-1 {
		--words-at: 500ms;
	}

	/* 2 · the mess */
	.st-2 .obj {
		animation: drop 750ms cubic-bezier(0.3, 0.7, 0.35, 1.15) both;
	}
	.st-2 .shot {
		animation-delay: 150ms;
	}
	.st-2 .mark {
		animation-delay: 380ms;
	}
	.st-2 .msg {
		animation-delay: 610ms;
	}
	.st-2 .card {
		animation-delay: 840ms;
	}
	.st-2 {
		--words-at: 900ms;
	}

	/* 3 · a tag on each thing says how it got in; each flies to its row */
	.st-3 .phone {
		animation: rise 700ms cubic-bezier(0.2, 0.7, 0.3, 1) both;
	}
	.st-3 .how {
		animation: tie 420ms cubic-bezier(0.2, 0.8, 0.3, 1.2) both;
	}
	.st-3 .into {
		animation: gather 750ms cubic-bezier(0.55, 0, 0.7, 0.4) both;
	}
	.st-3 .row:not(.row-curry) {
		animation: appear 400ms cubic-bezier(0.2, 0.8, 0.3, 1.2) both;
	}
	.st-3 .shot.into .how {
		animation-delay: 600ms;
	}
	.st-3 .mark.into .how {
		animation-delay: 760ms;
	}
	.st-3 .msg.into .how {
		animation-delay: 920ms;
	}
	.st-3 .card.into .how {
		animation-delay: 1080ms;
	}
	.st-3 .shot.into {
		animation-delay: 1500ms;
	}
	.st-3 .mark.into {
		animation-delay: 1850ms;
	}
	.st-3 .msg.into {
		animation-delay: 2200ms;
	}
	.st-3 .card.into {
		animation-delay: 2550ms;
	}
	.st-3 .row-shot {
		animation-delay: 2100ms;
	}
	.st-3 .row-mark {
		animation-delay: 2450ms;
	}
	.st-3 .row-dal {
		animation-delay: 2800ms;
	}
	.st-3 .row-card {
		animation-delay: 3150ms;
	}
	.st-3 {
		--words-at: 1000ms;
	}
	.s-cookbook span {
		animation: press 420ms 3400ms cubic-bezier(0.2, 0.8, 0.3, 1.1) both;
	}

	/* 4 · the recipe never moves; the diary fills, newest on top */
	.st-4 .diary {
		animation: rise 600ms 200ms cubic-bezier(0.2, 0.7, 0.3, 1) both;
	}
	.st-4 .e1 {
		animation: grow 700ms 700ms ease-out both;
	}
	.st-4 .e2 {
		animation: grow 700ms 1500ms ease-out both;
	}
	.st-4 {
		--words-at: 600ms;
	}
	.s-diary span {
		animation: press 420ms 2200ms cubic-bezier(0.2, 0.8, 0.3, 1.1) both;
	}

	/* 5 · cooking it differently: the change lives in the diary, not the recipe */
	.st-5 .cooking {
		animation: slide-in 750ms 250ms cubic-bezier(0.25, 0.8, 0.3, 1.08) both;
	}
	.st-5 .was {
		animation: strike 400ms 1150ms ease-out both;
	}
	.st-5 .now {
		animation: drop 500ms 1450ms cubic-bezier(0.3, 0.7, 0.35, 1.15) both;
	}
	.st-5 .cook-said {
		animation: appear 400ms 1800ms ease-out both;
	}
	.st-5 .e3 {
		animation: grow 800ms 2200ms ease-out both;
	}
	.st-5 {
		--words-at: 900ms;
	}

	/* 6 · you keep it: the cooking becomes the recipe's new page */
	.st-6 .choice {
		animation: rise 600ms 150ms cubic-bezier(0.2, 0.7, 0.3, 1) both;
	}
	.st-6 .keep {
		animation: tap 900ms 400ms ease-out both;
	}
	.st-6 .keep > span::before {
		animation: tick 900ms 400ms both;
	}
	.st-6 .r-old {
		animation: step-back 800ms 1400ms cubic-bezier(0.4, 0, 0.2, 1) both;
	}
	.st-6 .r-new {
		animation: lift-from-diary 900ms 1400ms cubic-bezier(0.25, 0.8, 0.3, 1.05) both;
	}
	.st-6 .r-new .changed span {
		animation: mark 500ms 2400ms ease-out both;
	}
	.st-6 {
		--words-at: 1600ms;
	}
	.s-version span {
		animation: press 420ms 2800ms cubic-bezier(0.2, 0.8, 0.3, 1.1) both;
	}

	/* 7 · three Cookbooks, then the Kitchen around them */
	.st-7 .mini {
		animation: rise 650ms cubic-bezier(0.2, 0.7, 0.3, 1) both;
	}
	.st-7 .m-you {
		animation-delay: 150ms;
	}
	.st-7 .m-sam {
		animation-delay: 450ms;
	}
	.st-7 .m-ines {
		animation-delay: 750ms;
	}
	.st-7 .kitchen {
		animation: frame 600ms 1300ms ease-out both;
	}
	.st-7 .kitchen-name {
		animation: drop 550ms 1600ms cubic-bezier(0.3, 0.7, 0.35, 1.15) both;
	}
	@keyframes frame {
		from {
			opacity: 0;
			transform: scale(1.06);
		}
	}
	.st-7 {
		--words-at: 600ms;
	}
	.s-kitchen span {
		animation: press 420ms 2100ms cubic-bezier(0.2, 0.8, 0.3, 1.1) both;
	}

	/* 8 · their recipes slot in between yours */
	.st-8 .phone {
		animation: rise 600ms 100ms cubic-bezier(0.2, 0.7, 0.3, 1) both;
	}
	.st-8 .theirs {
		animation: grow-row 600ms ease-out both;
	}
	.st-8 .theirs:nth-child(2) {
		animation-delay: 900ms;
	}
	.st-8 .theirs:nth-child(3) {
		animation-delay: 1250ms;
	}
	.st-8 .theirs:nth-child(5) {
		animation-delay: 1600ms;
	}
	@keyframes grow-row {
		from {
			max-height: 0;
			padding-block: 0;
			opacity: 0;
			margin-top: -1.4cqw;
		}
		60% {
			opacity: 0;
		}
	}
	.st-8 .row {
		max-height: 14cqw;
		overflow: hidden;
	}
	.st-8 {
		--words-at: 500ms;
	}

	/* 9 · Inès's curry comes out of yours; yours never moves */
	.st-9 .r-ines {
		animation: copy-out 900ms 700ms cubic-bezier(0.25, 0.8, 0.3, 1.05) both;
	}
	@keyframes copy-out {
		from {
			opacity: 0;
			transform: translate(-39cqw, -43cqw) rotate(0deg);
		}
		25% {
			opacity: 0.9;
		}
	}
	.st-9 .r-ines .changed span {
		animation: mark 500ms 1900ms ease-out both;
	}
	.st-9 .thread {
		animation: appear 600ms 1500ms ease-out both;
	}
	.st-9 {
		--words-at: 500ms;
	}

	/* 10 · the everyday things drop into place */
	.st-10 .ev {
		animation: drop 700ms cubic-bezier(0.3, 0.7, 0.35, 1.15) both;
	}
	.st-10 .ev-shop {
		animation-delay: 150ms;
	}
	.st-10 .ev-cook {
		animation-delay: 400ms;
	}
	.st-10 .ev-link {
		animation-delay: 650ms;
	}
	.st-10 .ev-off {
		animation-delay: 900ms;
	}
	.st-10 {
		--words-at: 800ms;
	}

	/* 11 · an empty Cookbook, and the button */
	.st-11 .phone {
		animation: rise 700ms 100ms cubic-bezier(0.2, 0.7, 0.3, 1) both;
	}
	.st-11 .empty {
		animation: appear 500ms ease-out both;
	}
	.st-11 .empty:nth-child(1) {
		animation-delay: 700ms;
	}
	.st-11 .empty:nth-child(2) {
		animation-delay: 820ms;
	}
	.st-11 .empty:nth-child(3) {
		animation-delay: 940ms;
	}
	.st-11 .empty:nth-child(4) {
		animation-delay: 1060ms;
	}
	.st-11 .empty:nth-child(5) {
		animation-delay: 1180ms;
	}
	.st-11 {
		--words-at: 400ms;
	}
	.st-11 .cta-wrap {
		animation: appear 500ms 1300ms ease-out both;
	}
</style>
