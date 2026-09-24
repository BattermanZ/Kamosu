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
	**Nobody is asked where it goes.** A recipe you write goes into your own
	Cookbook, always (ADR 0041), so the tap writes it.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import BringIn from '$lib/BringIn.svelte';

	interface Props {
		/**
		 * The title to offer to write, where the caller already knows it. Absent
		 * means ask for one.
		 */
		title?: string;
	}

	let { title }: Props = $props();

	const kamosu = useKamosu();

	let typed = $state('');
	let adding = $state(false);
	/** Whether `BringIn` is working, so writing a recipe quiets while it is. */
	let bringing = $state(false);
	let failed = $state<string | undefined>(undefined);

	/** What *Write a recipe* would write. The caller's title wins where there is one. */
	const naming = $derived((title ?? typed).trim());

	/** A recipe needs only a title (#6), so the title *is* the recipe. */
	async function add() {
		if (naming === '') return;
		adding = true;
		failed = undefined;
		try {
			const made = await kamosu.createRecipe({ title: naming });
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
			<!--
				`w-full` is load-bearing. Without a width, Safari sizes a text field
				at twenty of the font's average character, and Zen Kaku Gothic New
				is a Japanese face whose average character is a full em. That made
				the field about 340px wide on an iPhone and pushed this whole card
				past the edge of the screen (24 September 2026).
			-->
			<label class="grid gap-1 text-read text-ink-2">
				{m.add_title()}
				<input
					type="text"
					bind:value={typed}
					required
					class="min-h-12 w-full rounded-sm border border-rule bg-card px-3 text-body text-ink"
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
			class="min-h-12 rounded-sm bg-accent px-4 py-3 text-center font-display text-body text-on-accent disabled:opacity-60"
		>
			{m.recipes_nothing_add({ query: title })}
		</button>
	{/if}

	<BringIn look="offer" disabled={adding} onBusy={(working) => (bringing = working)} />

	{#if failed}
		<p class="text-read text-support" role="alert">{failed}</p>
	{/if}
</div>
