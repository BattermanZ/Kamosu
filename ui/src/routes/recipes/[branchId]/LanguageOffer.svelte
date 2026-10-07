<!--
	The Language offer, put to the cook (#106, ADR 0006).

	THIS IS THE HALF OF ADR 0006 THAT WAS NEVER BUILT. Every save reads the
	recipe's own text and answers `language_offer` — the Language it actually
	reads as, when that disagrees with the one the recipe carries. Until #106
	no production code in `ui/` read that field, so *set when blank, offered
	when it disagrees, never changed silently* held for an agent at the MCP
	door and for nobody using a browser. Every screen test set the field to
	null, which is why nothing went red.

	IT IS AN OFFER AND NEVER A CHANGE. Nothing here applies on its own, on a
	timer, or on the way out. `set_recipe_language` is the only thing that can
	act on it and it runs on a deliberate tap.

	IT ARRIVES WHERE THE SAVE ALREADY SPEAKS, which is the choice of
	22 September 2026 against a band under the photograph at the top of the
	page. The reasoning is his: the offer comes at the moment of saving, which
	is the worst moment to interrupt somebody, so it goes where `Saved.` is
	already said rather than somewhere that has to be noticed.

	IT BELONGS TO ONE SAVE, NOT TO THE RECIPE. `language_offer` rides out on a
	save's answer and `get_recipe` does not carry it, so there is nothing to
	come back to tomorrow — which is what the ticket means by *does not ask
	again for that save*. Declining ends it here and now, and nothing is
	written to remember that it was declined. Saving again re-reads the text
	and asks again, which is correct: the text is what changed.

	ACCEPTING MINTS A VERSION, and the line beneath says so before it is
	tapped. That is what makes this different from tagging (#104) and relating
	(#105), which appear nowhere in the Thread.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { languageName, type BranchLanguage } from '$lib/language';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';

	interface Props {
		branchId: string;
		/** The Language the recipe carries — what it is filed as. */
		filed: string;
		/**
		 * The Language its text reads as. Never equal to `filed`: the Core
		 * would have answered null. Typed as the Catalogue's own enum rather
		 * than as a string, so passing it back to `set_recipe_language` needs
		 * no cast — a cast here would be a third hand-written copy of that
		 * enum, and the whole point of `$lib/language` is that there are none.
		 */
		offered: BranchLanguage;
		/**
		 * It was said. Carries the Branch it landed on: saying this about a
		 * recipe another Kitchen writes is a Copy like any other change
		 * (ADR 0020), and the answer then names a NEW Branch.
		 */
		onSaid: (landed: { branch_id: string; language: string }) => void;
	}

	let { branchId, filed, offered, onSaid }: Props = $props();

	const kamosu = useKamosu();

	/**
	 * Where the question has got to. `left` draws nothing at all — an offer
	 * that has been declined is not a thing to be reminded of.
	 */
	let answered = $state<'asking' | 'saying' | 'said' | 'left'>('asking');
	let failed = $state<string | null>(null);
	/**
	 * What it ended up filed as, once it has been said — the Core's own answer
	 * rather than what was offered, so the line that replaces the question
	 * reports what happened instead of what was asked for. Null until then.
	 */
	let became = $state<string | null>(null);

	async function accept() {
		if (answered === 'saying') return;
		answered = 'saying';
		failed = null;
		try {
			const landed = await kamosu.setRecipeLanguage({
				branch_id: branchId,
				language: offered,
			});
			became = landed.language;
			answered = 'said';
			onSaid({ branch_id: landed.branch_id, language: landed.language });
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
			answered = 'asking';
		}
	}
</script>

{#if answered === 'said'}
	<p class="mx-gutter mt-2 text-read text-accent" role="status">
		{m.recipe_language_offer_done({ language: languageName(became ?? offered) })}
	</p>
{:else if answered !== 'left'}
	<div class="mx-gutter mt-2 border border-rule bg-card p-3">
		<p class="text-read text-ink">
			{m.recipe_language_offer({
				filed: languageName(filed),
				reads: languageName(offered),
			})}
		</p>
		{#if failed}
			<p class="mt-2 text-read text-support" role="alert">{failed}</p>
		{/if}
		<div class="mt-3 flex gap-2">
			<!--
				Accepting is `set_recipe_language`, which is on the server's side
				of the line and is never queued — the same rule as editing and
				the Language control at the foot (ADR 0013, #76). So offline it
				says what it is waiting for rather than failing when pressed.
				Declining beside it needs nothing: it is an answer this page
				gives on its own.
			-->
			<NeedsServer
				label={m.recipe_language_offer_accept({ language: languageName(offered) })}
				waiting={m.offline_waits_edit()}
				disabled={answered === 'saying'}
				onclick={accept}
				shapeClass="min-h-12 flex-1 px-3 py-2 text-center font-display text-read"
				lookClass="bg-accent text-on-accent"
			/>
			<button
				type="button"
				disabled={answered === 'saying'}
				onclick={() => (answered = 'left')}
				class="min-h-12 flex-1 border border-rule px-3 py-2 text-center font-display text-read
				text-accent disabled:opacity-60"
			>
				{m.recipe_language_offer_decline()}
			</button>
		</div>
		<!--
			Said before it is tapped, not after. A cook deciding whether to
			accept is deciding whether to put an entry in the recipe's history.
		-->
		<p class="mt-2 text-read text-ink-2">{m.recipe_language_offer_version()}</p>
	</div>
{/if}
