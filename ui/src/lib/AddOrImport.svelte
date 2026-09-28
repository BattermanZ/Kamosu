<!--
	The things a person with nothing to look at was about to do anyway: write a
	recipe, or bring one in.

	It lives here rather than inside one screen because two screens reach the
	same dead end from different directions — Recipes, when a search matched
	nothing (#62, ADR 0027), and Home, when there is nothing on the shelf to
	suggest from (#64). Both are the same situation: you do not have it yet, and
	adding it was the next thing you were going to do.

	**Every offer *does* the thing rather than pointing at a screen to do it
	on.** That is the whole point of them: an offer that only navigates has put
	a screen between a person and the one act they came for — and on an empty
	instance it can put them on a second empty screen, which is worse than
	saying nothing.

	A recipe needs only a title (#6), so writing one is a title and a tap. Where
	the caller already knows the title — Recipes knows it, because you typed it
	into the search box — it passes it and the field disappears: asking for a
	word somebody has just finished typing is asking them to type it twice.

	**The acts themselves live in `$lib/adding`**, shared with the + beside the
	search box (#174), which offers the same three every day. This is only how
	the dead ends draw them: full-width buttons, since an empty screen should
	say what to do rather than point at a small button.
	**Nobody is asked where it goes.** A recipe you write goes into your own
	Cookbook, always (ADR 0041), so the tap writes it.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';
	import { useAdding } from '$lib/adding/adding.svelte';
	import AddressForm from '$lib/adding/AddressForm.svelte';
	import TitleForm from '$lib/adding/TitleForm.svelte';
	import FilePicker from '$lib/adding/FilePicker.svelte';
	import Said from '$lib/adding/Said.svelte';

	interface Props {
		/**
		 * The title to offer to write, where the caller already knows it. Absent
		 * means ask for one.
		 */
		title?: string;
	}

	let { title }: Props = $props();

	const adding = useAdding();

	/** Whether the link offer has been taken, which swaps its button for the address field. */
	let asking = $state(false);
	let picker = $state<FilePicker | undefined>(undefined);
</script>

<FilePicker {adding} bind:this={picker} />

<div class="grid gap-2">
	{#if title === undefined}
		<!-- No title was handed down, so this is the one place it can come from. -->
		<TitleForm {adding} />
	{:else}
		<button
			type="button"
			onclick={() => adding.write(title)}
			disabled={adding.working !== null}
			class="min-h-12 rounded-sm bg-accent px-4 py-3 text-center font-display text-body text-on-accent disabled:opacity-60"
		>
			{m.recipes_nothing_add({ query: title })}
		</button>
	{/if}

	{#if !asking}
		<NeedsServer
			label={m.recipes_nothing_import()}
			waiting={m.offline_waits_import_link()}
			disabled={adding.working !== null}
			onclick={() => (asking = true)}
			shapeClass="min-h-12 rounded-sm px-4 py-3 text-center font-display text-body"
			lookClass="border border-rule bg-card text-accent disabled:opacity-60"
		/>
	{:else}
		<AddressForm {adding} />
	{/if}

	<NeedsServer
		label={adding.working === 'file' ? m.bring_in_file_working() : m.bring_in_file()}
		waiting={m.offline_waits_bring_in()}
		disabled={adding.working !== null}
		onclick={() => picker?.open()}
		shapeClass="min-h-12 rounded-sm px-4 py-3 text-center font-display text-body"
		lookClass="border border-rule bg-card text-accent disabled:opacity-60"
	/>
</div>

<Said {adding} />
