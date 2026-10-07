<!--
	Naming a version of this recipe, renaming it, or clearing its name (#134,
	choice B of 24 September 2026: a line among the recipe's
	actions, opening a small sheet, over tapping the name under the title or
	pressing and holding its chip).

	A "version" here is a Branch: the strip at the top counts them ("This
	recipe, 2 versions"), and the name given here is what its chip and the line
	under the title show. It is NOT the History screen's "Rename" (#115), which
	names one saved Version. The sentence under the field says so in the terms
	a cook can check — nothing in the recipe changes, nothing is added to its
	History — because `rename_branch` writes no Version and moves no id.

	WHAT THE CORE REFUSES IS SAID AS IT CAME. A Cookbook keeps one unnamed
	Branch of a recipe in each Language, so clearing the name of a second one
	is refused in a sentence the Core words itself. The sheet stays open with
	what was typed, so the cook can give it a name instead. A request that
	never reached Kamosu has no sentence of its own, and gets a plain failure
	line.

	WHAT LANDED IS READ BACK, NOT PATCHED IN. The page re-reads the recipe and
	its Thread after a rename, so the chip and the line under the title show
	what the Core now holds. A local patch would have to know how `branchLabel`
	names a version, and could drift from it.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import SheetFrame from '$lib/SheetFrame.svelte';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';

	interface Props {
		branchId: string;
		/** The name this version carries now, or null where it has none. */
		name: string | null;
		/** What the line that opened it says, so the sheet is called what was tapped. */
		title: string;
		/** A name landed or was cleared; the page reads the recipe again. */
		onRenamed: () => void;
		onClose: () => void;
	}

	let { branchId, name, title, onRenamed, onClose }: Props = $props();

	const kamosu = useKamosu();
	const uid = $props.id();

	/**
	 * What is typed, seeded ONCE from the name it had when the sheet opened —
	 * `Writing`'s rule: a re-read landing mid-word must not overwrite it.
	 */
	// svelte-ignore state_referenced_locally
	let draft = $state(name ?? '');
	/** A name is being sent, so neither button can be pressed twice. */
	let working = $state(false);
	/** The Core's own sentence, or the failure line for a request that never arrived. */
	let refused = $state<string | null>(null);

	async function send(next: string | null) {
		if (working) return;
		working = true;
		refused = null;
		try {
			await kamosu.renameBranch({ branch_id: branchId, name: next });
			onRenamed();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			refused = error.reached ? error.message : m.recipe_version_name_failed();
		} finally {
			working = false;
		}
	}
</script>

<!-- The field takes the caret: typing a name is the one thing this sheet is for. -->
<SheetFrame
	labelledby="{uid}-title"
	tall={78}
	class="overflow-y-auto bg-ground px-gutter pt-6"
	onclose={onClose}
>
	<h3 id="{uid}-title" class="font-display text-title font-semibold">{title}</h3>
	<form
		onsubmit={(event) => {
			event.preventDefault();
			// Only spaces is no name, and `required` does not see that. Saving
			// it would clear the name, which is Remove the name's job alone.
			const next = draft.trim();
			if (next === '') {
				draft = '';
				return;
			}
			void send(next);
		}}
	>
		<label class="mt-4 block text-label text-ink-2 uppercase" for="{uid}-name">
			{m.recipe_version_name_label()}
		</label>
		<input
			id="{uid}-name"
			class="mt-1 min-h-12 w-full rounded-sm border border-rule bg-card px-3 text-body text-ink"
			placeholder={m.recipe_version_name_placeholder()}
			bind:value={draft}
			required
			data-sheet-focus
		/>
		<p class="mt-2 text-read text-ink-2">{m.recipe_version_name_what()}</p>
		{#if refused}
			<p class="mt-2 text-read text-support" role="alert">{refused}</p>
		{/if}
		<button
			class="mt-4 block w-full bg-accent p-4 text-center font-display text-body text-on-accent disabled:opacity-60"
			disabled={working}
		>
			{m.recipe_version_name_save()}
		</button>
	</form>
	<button
		type="button"
		class="mt-2 block w-full border border-rule p-4 text-center font-display text-body text-accent"
		onclick={onClose}
	>
		{m.recipe_version_name_cancel()}
	</button>
	{#if name !== null}
		<!--
			Small and set apart, below the way out: clearing is the rarer act,
			and the Core may refuse it where another version already goes
			without a name.
		-->
		<p class="mt-4 text-center">
			<button
				type="button"
				class="px-3 py-2 text-read text-accent underline underline-offset-4 disabled:opacity-60"
				disabled={working}
				onclick={() => send(null)}
			>
				{m.recipe_version_name_remove()}
			</button>
		</p>
	{/if}
</SheetFrame>
