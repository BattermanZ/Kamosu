<!--
	The one place Meaning Search is offered (#63, ADR 0029).

	Not in settings. A settings screen is where features go to be undiscovered,
	and this is the moment a person can see exactly what they are missing: they
	looked for something, and words did not find it. So the offer is carried
	once, here, where it is earned.

	It is only ever shown to somebody who can act on it — the Operation decides
	that, not this component — and never again to an Operator who said no. An
	answered question asked twice is a nag.

	Turning it on is three Operations in a row: accept the terms, fetch the
	model, read the library. The middle two are Jobs, watched through `get_job`
	like every other Job in Kamosu, because a 220 MB download and a whole
	library's worth of inference are exactly what a Job is for (ADR 0032).
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import type { MeaningSearch } from '$lib/meaning.svelte';

	interface Props {
		meaning: MeaningSearch;
		/** What the instance last said. Never absent where this is rendered. */
		status: NonNullable<MeaningSearch['status']>;
	}

	let { meaning, status }: Props = $props();

	/**
	 * Turning it on is minutes, not seconds, and a person watching deserves to
	 * know which minute they are in. Both Jobs report through the same row every
	 * Job reports through, so this only has to print what it says.
	 */
	let done = $state(false);

	async function turnOn() {
		await meaning.turnOn();
		done = meaning.failed === undefined;
	}
</script>

<div class="mt-4 border-t border-rule pt-3">
	{#if done}
		<p class="text-read text-ink-2" role="status">{m.meaning_now_on()}</p>
	{:else}
		<p class="text-read text-ink-2">{m.recipes_nothing_meaning()}</p>
		<!--
			What is actually being agreed to, said before the button rather than
			after it: Kamosu ships no weights, so this downloads somebody else's
			under somebody else's terms, and the person accepting is the person
			the terms are about.
		-->
		<p class="mt-2 text-read text-ink-2">{m.meaning_what_it_costs({ model: status.model })}</p>
		<!--
			Each of those is a whole sentence and each link a whole label:
			splitting a sentence around a link fixes English word order into
			every other language, and Kamosu speaks three from the first day.
		-->
		<p class="mt-1 flex flex-wrap gap-x-4 text-read">
			<a href={status.terms_url} target="_blank" rel="noreferrer noopener" class="text-accent">
				{m.meaning_terms()}
			</a>
			<a
				href={status.prohibited_use_policy_url}
				target="_blank"
				rel="noreferrer noopener"
				class="text-accent"
			>
				{m.meaning_policy()}
			</a>
		</p>

		{#if meaning.working}
			<p class="mt-3 text-read text-ink-2" role="status">{meaning.saying}</p>
		{:else}
			<div class="mt-3 flex flex-wrap gap-2">
				<button
					type="button"
					onclick={turnOn}
					class="min-h-12 rounded-sm bg-accent px-4 py-3 font-display text-body text-on-accent"
				>
					{m.recipes_nothing_meaning_turn_on()}
				</button>
				<button
					type="button"
					onclick={() => meaning.decline()}
					class="min-h-12 rounded-sm border border-rule bg-card px-4 py-3 font-display text-body text-ink-2"
				>
					{m.meaning_no_thanks()}
				</button>
			</div>
		{/if}
	{/if}

	{#if meaning.failed}
		<p class="mt-2 text-read text-support" role="alert">{meaning.failed}</p>
	{/if}
</div>
