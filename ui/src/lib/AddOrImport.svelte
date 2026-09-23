<!--
	The things a person with nothing to look at was about to do anyway: write a
	recipe, or bring one in.

	It lives here rather than inside one screen because two screens reach the
	same dead end from different directions — Recipes, when a search matched
	nothing (#62, ADR 0027), and Home, when there is nothing on the shelf to
	suggest from (#64). Both are the same situation: you do not have it yet, and
	adding it was the next thing you were going to do.

	**Both offers *do* the thing rather than pointing at a screen to do it on.**
	That is the whole point of them: an offer that only navigates has put a
	screen between a person and the one act they came for — and on an empty
	instance it can put them on a second empty screen, which is worse than
	saying nothing.

	A recipe needs only a title (#6), so writing one is a title and a tap. Where
	the caller already knows the title — Recipes knows it, because you typed it
	into the search box — it passes it and the field disappears: asking for a
	word somebody has just finished typing is asking them to type it twice.

	**Bringing one in from outside lives in `BringIn`**, which this draws in its
	`offer` look. It used to be here, and moved when the recipe file joined the
	link beside it (#93): the same pair is now above the shelf every day as well
	as at these two dead ends, and one act written twice is one act that will
	eventually be two.
	**A cook in several Kitchens is asked whose recipe it is** (#111), in a
	sheet the tap opens, with `KitchenChoice` — nothing picked, nothing
	remembered, because a recipe cannot be moved between Kitchens afterwards.
	A cook in one is never asked, and the tap writes the recipe as it always
	did. Bringing one in is not asked: an Import lands in the Home Kitchen
	(#68), which #111 kept.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import BringIn from '$lib/BringIn.svelte';
	import KitchenChoice from '$lib/KitchenChoice.svelte';
	import { focusInAndBack } from '$lib/focus-in-and-back';
	import { kitchenName } from '$lib/where-a-save-lands';
	import type { ListKitchensOutput } from '$lib/api/catalogue';

	interface Props {
		/**
		 * The title to offer to write, where the caller already knows it. Absent
		 * means ask for one.
		 */
		title?: string;
		/**
		 * The Kitchens this Person cooks in, where the caller has already asked
		 * for them; absent means ask. With more than one, the cook is asked which
		 * keeps the new recipe.
		 */
		kitchens?: ListKitchensOutput['kitchens'];
	}

	let { title, kitchens }: Props = $props();

	const kamosu = useKamosu();

	/** Asked for only when the caller did not already have them. */
	let known = $state<ListKitchensOutput['kitchens']>([]);
	let typed = $state('');
	let adding = $state(false);
	/** Whether `BringIn` is working, so writing a recipe quiets while it is. */
	let bringing = $state(false);
	let failed = $state<string | undefined>(undefined);

	/** Whether the cook is asked which Kitchen keeps it: open, and their answer. */
	let asking = $state(false);
	let chosen = $state<string | undefined>(undefined);

	const held = $derived(kitchens ?? known);
	const chosenKitchen = $derived(held.find((kitchen) => kitchen.id === chosen));
	/** What *Write a recipe* would write. The caller's title wins where there is one. */
	const naming = $derived((title ?? typed).trim());

	$effect(() => {
		if (kitchens) return;
		let current = true;
		kamosu
			.listKitchens({})
			.then((all) => {
				if (current) known = all.kitchens;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
			});
		return () => {
			current = false;
		};
	});

	/**
	 * A recipe needs only a title (#6), so the title *is* the recipe. With one
	 * Kitchen there is nowhere else for it to go; with several, it waits for
	 * an answer, asked afresh each time.
	 */
	function add() {
		if (held.length === 0 || naming === '') return;
		const [only] = held;
		if (held.length === 1 && only) {
			write(only.id);
			return;
		}
		chosen = undefined;
		asking = true;
	}

	async function write(kitchenId: string) {
		adding = true;
		failed = undefined;
		try {
			const made = await kamosu.createRecipe({ kitchen_id: kitchenId, title: naming });
			await goto(`/recipes/${made.branch_id}`);
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
			adding = false;
		}
	}
</script>

<div class="grid gap-2">
	{#if title === undefined}
		<!--
			No title was handed down, so this is the one place it can come from.
			Submitting the field is the same act as the button, because a person
			who has typed a name and pressed enter has already asked.
		-->
		<form
			class="grid gap-2"
			onsubmit={(event) => {
				event.preventDefault();
				add();
			}}
		>
			<label class="grid gap-1 text-read text-ink-2">
				{m.add_title()}
				<input
					type="text"
					bind:value={typed}
					required
					class="min-h-12 rounded-sm border border-rule bg-card px-3 text-body text-ink"
				/>
			</label>
			<button
				class="min-h-12 rounded-sm bg-accent px-4 py-3 font-display text-body text-on-accent disabled:opacity-60"
				disabled={adding || bringing}
			>
				{m.add_write()}
			</button>
		</form>
	{:else}
		<button
			type="button"
			onclick={add}
			disabled={adding || bringing}
			class="min-h-12 rounded-sm bg-accent px-4 py-3 text-left font-display text-body text-on-accent disabled:opacity-60"
		>
			{m.recipes_nothing_add({ query: title })}
		</button>
	{/if}

	<BringIn look="offer" disabled={adding} onBusy={(working) => (bringing = working)} />

	{#if failed && !asking}
		<p class="text-read text-support" role="alert">{failed}</p>
	{/if}
</div>

<svelte:window
	onkeydown={(event) => {
		if (asking && event.key === 'Escape' && !adding) asking = false;
	}}
/>

<!--
	Whose recipe is this (#111). The same bottom sheet the writing screen saves
	through, so the two places a recipe is made ask in the same shape.
-->
{#if asking}
	<div class="fixed inset-0 z-40 bg-accent/40" aria-hidden="true"></div>
	<div
		class="fixed inset-x-0 bottom-0 z-50 mx-auto max-h-[88vh] max-w-2xl overflow-y-auto bg-ground px-gutter pt-6 pb-safe"
		role="dialog"
		aria-modal="true"
		aria-label={m.kitchen_ask_title()}
		tabindex="-1"
		{@attach focusInAndBack}
	>
		<p class="text-label text-ink-2 uppercase">{m.kitchen_ask_title()}</p>
		<p class="mt-2 text-body">{m.kitchen_ask_add({ title: naming })}</p>
		<div class="mt-4">
			<KitchenChoice kitchens={held} bind:chosen />
		</div>
		{#if failed}
			<p class="mt-3 text-read text-support" role="alert">{failed}</p>
		{/if}
		<button
			type="button"
			class="mt-4 block w-full bg-accent p-4 text-center font-display text-body text-on-accent disabled:opacity-60"
			disabled={!chosenKitchen || adding}
			onclick={() => chosenKitchen && write(chosenKitchen.id)}
		>
			{chosenKitchen
				? m.kitchen_ask_write_in({ kitchen: kitchenName(chosenKitchen) })
				: m.kitchen_ask_choose_first()}
		</button>
		<button
			type="button"
			class="mt-2 mb-4 block w-full border border-rule p-4 text-center font-display text-body text-ink-2"
			disabled={adding}
			onclick={() => (asking = false)}
		>
			{m.write_back()}
		</button>
	</div>
{/if}
