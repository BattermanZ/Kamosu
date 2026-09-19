<!--
	The tokens page (issue #80, moved here by #37).

	Every value on it is read out of the stylesheet the browser actually loaded,
	with `getComputedStyle`. That is the whole trick: the page cannot drift from
	what Kamosu looks like, because it is not a copy of the tokens — it is the
	tokens, read back. It was rendered from Rust while there was no app to put it
	in; now that there is, it is an ordinary route reading the same values the
	same way.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import Screen from '$lib/shell/Screen.svelte';
	import Section from '$lib/shell/Section.svelte';
	import { COLOUR_ORDER, RADIUS_ORDER, readToken, readTypeScale, readSpacing } from '$lib/tokens';

	// Read once the document exists: this is what the browser resolved, fonts,
	// cascade and all.
	let colours = $state<{ name: string; value: string }[]>([]);
	let type = $state<ReturnType<typeof readTypeScale>>([]);
	let spacing = $state<{ name: string; value: string }[]>([]);
	let radii = $state<{ name: string; value: string }[]>([]);
	let layout = $state<{ name: string; value: string }[]>([]);

	$effect(() => {
		colours = COLOUR_ORDER.map((name) => ({ name, value: readToken(name) }));
		radii = RADIUS_ORDER.map((name) => ({ name, value: readToken(name) }));
		layout = ['--tile-w', '--hero-h', '--rule-w', '--noren-h', '--photo-thumb'].map((name) => ({
			name,
			value: readToken(name),
		}));
		type = readTypeScale();
		spacing = readSpacing();
	});
</script>

<Screen title={m.tokens_title()} blurb={m.tokens_blurb()}>
	<Section heading={m.tokens_colour()}>
		<ul class="grid grid-cols-2 gap-2 sm:grid-cols-3">
			{#each colours as colour (colour.name)}
				<li class="overflow-hidden rounded-sm border border-rule bg-card">
					<div class="h-12" style="background-color: var({colour.name})"></div>
					<div class="p-2">
						<div class="text-read font-medium">{colour.name}</div>
						<div class="text-read text-ink-2">{colour.value}</div>
					</div>
				</li>
			{/each}
		</ul>
	</Section>

	<Section heading={m.tokens_type()}>
		{#each type as step (step.name)}
			<div class="mb-4">
				<p
					style="font-size: var({step.name}); line-height: {step.lineHeight ||
						'normal'}; letter-spacing: {step.letterSpacing || 'normal'}"
				>
					{m.tokens_specimen()}
				</p>
				<p class="text-read text-ink-2">
					{step.name} · {step.size}{step.lineHeight
						? ` / ${step.lineHeight}`
						: ''}{step.letterSpacing ? ` · ${step.letterSpacing}` : ''}
				</p>
			</div>
		{/each}
		<p class="mb-2 font-display font-semibold">
			Zen Old Mincho sets what a person wrote — steps, titles, covers.
		</p>
		<p>Zen Kaku Gothic New sets the interface.</p>
	</Section>

	<Section heading={m.tokens_spacing()}>
		{#each spacing as step (step.name)}
			<div class="mb-3">
				<div class="text-read text-ink-2">{step.name} · {step.value}</div>
				<div class="mt-1 h-4 bg-accent" style="width: var({step.name})"></div>
			</div>
		{/each}
	</Section>

	<Section heading={m.tokens_shape()}>
		<div class="mb-4 flex items-end gap-4">
			{#each radii as radius (radius.name)}
				<div class="text-center">
					<div
						class="h-12 w-12 border-2 border-accent bg-card"
						style="border-radius: var({radius.name})"
					></div>
					<div class="mt-1 text-read text-ink-2">
						{radius.name.replace('--radius-', '')}<br />{radius.value}
					</div>
				</div>
			{/each}
		</div>
		<ul class="text-read text-ink-2">
			{#each layout as constant (constant.name)}
				<li>{constant.name} · {constant.value}</li>
			{/each}
		</ul>
	</Section>
</Screen>
