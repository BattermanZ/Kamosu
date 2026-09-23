<!--
	Which of your Kitchens keeps a new recipe (#111): asked of a cook in
	several, and never of a cook in one — the caller draws this only when
	`whereASaveLands` says `asking`, or, adding from empty, when there is more
	than one Kitchen to name.

	ONE LIST, FOUR PLACES. Adding a recipe from empty, forking on the writing
	screen, a cooking's picture made the recipe's, and what was cooked kept
	onto it all ask with this, because the ticket's rule is that they must not
	answer the question two different ways. Aurélien chose *asked* over *told
	and changeable* on 23 September 2026.

	NOTHING IS PICKED FOR YOU. Not the Home Kitchen either: a recipe cannot be
	moved between Kitchens afterwards, so the one thing this must not do is let
	a default pass for an answer. The Home Kitchen is listed first and says so,
	and has no other standing.

	NOTHING IS REMEMBERED. A choice that greets you next time is a mode, and
	ADR 0027 rejected modes on the same grounds it rejected a Kitchen switcher.

	Each row says who cooks there, because the question is *whose recipe is
	this* and a Kitchen's name alone does not always answer it.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { getLocale } from '$lib/paraglide/runtime';
	import { kitchenName, type Kitchen } from '$lib/where-a-save-lands';

	interface Props {
		kitchens: Kitchen[];
		/** The chosen Kitchen's id, or nothing until one is chosen. */
		chosen?: string;
	}

	let { kitchens, chosen = $bindable() }: Props = $props();

	const uid = $props.id();

	/** The Home Kitchen first; the others in the order the Core gives them. */
	const ordered = $derived([
		...kitchens.filter((kitchen) => kitchen.is_home),
		...kitchens.filter((kitchen) => !kitchen.is_home),
	]);

	const listed = $derived(new Intl.ListFormat(getLocale(), { type: 'conjunction' }));
</script>

<fieldset>
	<legend class="text-label text-ink-2 uppercase">{m.kitchen_ask_which()}</legend>
	<div class="mt-2 grid gap-2">
		{#each ordered as kitchen (kitchen.id)}
			<label
				class="flex min-h-12 cursor-pointer items-center gap-3 rounded-sm border bg-card px-3 py-2 {chosen ===
				kitchen.id
					? 'border-accent'
					: 'border-rule'}"
			>
				<input
					type="radio"
					name="{uid}-kitchen"
					value={kitchen.id}
					bind:group={chosen}
					class="h-4 w-4 shrink-0 accent-accent"
				/>
				<span class="min-w-0 flex-1">
					<span class="block font-display text-body text-ink">{kitchenName(kitchen)}</span>
					<span class="block text-read text-ink-2">
						{[
							listed.format(kitchen.members.map((member) => member.name)),
							...(kitchen.is_home ? [m.kitchen_ask_home()] : []),
						].join(' · ')}
					</span>
				</span>
			</label>
		{/each}
	</div>
</fieldset>
