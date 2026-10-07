<!--
	A picture taken while cooking, made the recipe's (#110, spec item 118): the
	Main Photo, or a Step's photo. It is an ordinary edit making a Version
	(`promote_attempt_photograph`), so a rapid second one folds into the
	Version already being shaped, and on another Kitchen's recipe it makes a
	Copy — the same two outcomes, in the same words, as saving on the writing
	screen (#83).

	ONE SHEET, REACHED FROM TWO PLACES. Chosen on 23 September 2026 from
	a prototype of three: the diary, where the pictures already are (a tapped
	photograph arrives here already picked), and the recipe page, where a
	recipe wearing its Cover is noticed and every cooking of it is in one row.
	Not the end of a cook. Both targets are one flow: a Step only adds which
	Step.

	A COPY BY A COOK IN SEVERAL KITCHENS ASKS WHICH KEEPS IT (#111), with the
	list the writing screen asks with, and the button waits for the answer.

	IT SAYS WHAT SAYING YES DOES, BESIDE THE BUTTON. Every other photograph act
	in the interface is quiet; this one edits the recipe AND takes a picture
	from the private side to the public one (ADR 0005, ADR 0026). So once a
	place is picked, the sheet says whether it saves or makes a Copy, and that
	the photograph stops being private — next to the button, not at the foot of
	a 29-step list the cook never scrolls to.

	WHOSE PICTURES THESE ARE is the caller's to get right, and both callers read
	them from `list_attempts`, which the Core scopes to the caller: another
	Person's cooking is never on offer. The Core refuses one anyway.
-->
<script lang="ts" module>
	/** One picture on offer, and the cooking that holds it. */
	export interface Offered {
		photograph: string;
		attempt: string;
		/** When that cooking was, as the diary writes a day. */
		taken: string;
	}
</script>

<script lang="ts">
	import { untrack } from 'svelte';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { GetRecipeOutput, PromoteAttemptPhotographOutput } from '$lib/api/catalogue';
	import AttemptPhoto from '$lib/offline/AttemptPhoto.svelte';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';
	import SheetFrame from '$lib/SheetFrame.svelte';
	import { copySaid } from '$lib/cookbook';

	interface Props {
		/** The recipe to put it on: any Branch of the Lineage that was cooked. */
		branchId: string;
		pictures: Offered[];
		/** The one already picked, where a tap on a photograph opened this. */
		picked?: string;
		onPromoted: (landed: PromoteAttemptPhotographOutput) => void;
		onClose: () => void;
	}

	let { branchId, pictures, picked, onPromoted, onClose }: Props = $props();

	const kamosu = useKamosu();
	const uid = $props.id();

	let recipe = $state<GetRecipeOutput | undefined>(undefined);
	let failed = $state(false);
	/** Read once, as it was when the sheet opened: a tap changes it after. */
	let chosen = $state(
		untrack(() => picked ?? (pictures.length === 1 ? pictures[0]?.photograph : undefined)),
	);
	/** The Main Photo, or a Step by its index into the recipe's own steps. */
	let target = $state<'main' | number | undefined>(undefined);
	let working = $state(false);
	let refused = $state<string | undefined>(undefined);

	$effect(() => {
		let current = true;
		kamosu
			.getRecipe({ branch_id: branchId })
			.then((read) => {
				if (current) recipe = read;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				if (current) failed = true;
			});
		return () => {
			current = false;
		};
	});

	const content = $derived(recipe?.versions.at(-1)?.content);

	/**
	 * A Version, or a Copy in the cook's own Cookbook, by the writing screen's
	 * own rule: the Core's `writes` on the recipe (ADR 0041).
	 */
	const forking = $derived(recipe !== undefined && !recipe.writes);

	/** A Step's number counts Steps only; a section heading takes none. */
	const steps = $derived.by(() => {
		let counted = 0;
		return (content?.steps ?? []).map((row, index) => ({
			index,
			row,
			number: row.kind === 'step' ? ++counted : null,
		}));
	});

	async function promote() {
		const from = pictures.find((picture) => picture.photograph === chosen);
		if (!from || target === undefined) return;
		working = true;
		refused = undefined;
		try {
			const landed = await kamosu.promoteAttemptPhotograph({
				attempt_id: from.attempt,
				photograph_id: from.photograph,
				branch_id: branchId,
				step_index: target === 'main' ? null : target,
			});
			onPromoted(landed);
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			refused = error.message;
		} finally {
			working = false;
		}
	}
</script>

<!--
	Escape closes it, as it closes every other sheet, except while the save
	is on its way, when closing would leave its answer nowhere to land.
-->
<SheetFrame
	label={m.promote_title()}
	tall={88}
	class="flex flex-col bg-ground text-ink"
	onclose={() => {
		if (!working) onClose();
	}}
>
	<div class="border-b border-rule px-gutter py-3">
		<p class="text-label text-support-2 uppercase">{m.promote_title()}</p>
		{#if content}
			<p class="mt-1 font-display text-line">{content.title}</p>
		{/if}
	</div>

	<div class="min-h-0 flex-1 overflow-y-auto px-gutter py-3">
		{#if failed}
			<p class="text-body text-support" role="alert">{m.recipe_failed()}</p>
		{:else if !content}
			<p class="text-body text-ink-2">{m.loading()}</p>
		{:else}
			{#if pictures.length > 1}
				<h3 id="{uid}-which" class="mb-2 text-label text-ink-2 uppercase">
					{m.promote_which()}
				</h3>
				<ul class="flex flex-wrap gap-2" aria-labelledby="{uid}-which">
					{#each pictures as picture (picture.attempt + picture.photograph)}
						<li>
							<button
								type="button"
								aria-pressed={chosen === picture.photograph}
								onclick={() => (chosen = picture.photograph)}
								class="rounded-sm border-2 p-1 {chosen === picture.photograph
									? 'border-accent'
									: 'border-transparent'}"
							>
								<AttemptPhoto
									id={picture.photograph}
									alt={m.promote_taken({ date: picture.taken })}
								/>
							</button>
						</li>
					{/each}
				</ul>
			{:else if chosen}
				<!--
					Large, from the server directly: only a picture that has
					reached it can be offered here, so there is no copy on the
					phone to ask the outbox for.
				-->
				<img
					src="/api/photographs/{chosen}/page"
					alt={m.cooked_photo_alt()}
					class="block max-h-[28vh] w-full rounded-sm object-cover"
				/>
			{/if}

			<h3 id="{uid}-where" class="mt-4 mb-2 text-label text-ink-2 uppercase">
				{m.promote_where()}
			</h3>
			<div role="group" aria-labelledby="{uid}-where">
				<button
					type="button"
					aria-pressed={target === 'main'}
					onclick={() => (target = 'main')}
					class="flex min-h-12 w-full items-start gap-3 border-y border-rule py-2 text-left"
				>
					{@render box(target === 'main')}
					<span class="min-w-0 flex-1">
						<span class="block text-line">{m.promote_main()}</span>
						<span class="block text-read text-ink-2">
							{content.main_photo ? m.promote_main_replaces() : m.promote_main_cover()}
						</span>
					</span>
				</button>
				<!--
					The Steps fold away behind one row: the Main Photo is what a
					cooking's picture nearly always becomes, and 29 Steps between
					it and the button would bury the sentence saying what the
					button does.
				-->
				<details class="border-b border-rule" open={typeof target === 'number'}>
					<summary class="flex min-h-12 cursor-pointer items-center py-2 text-line">
						{m.promote_step()}
					</summary>
					<ul>
						{#each steps as { index, row, number } (index)}
							{#if number === null}
								<li class="pt-3 pb-1 text-label text-ink-2 uppercase">{row.text}</li>
							{:else}
								<li>
									<button
										type="button"
										aria-pressed={target === index}
										onclick={() => (target = index)}
										class="flex w-full items-start gap-3 border-t border-rule py-2 text-left"
									>
										{@render box(target === index)}
										<span class="w-6 shrink-0 font-display text-line text-accent">{number}</span>
										<span class="line-clamp-2 min-w-0 flex-1 text-read">{row.text}</span>
										{#if row.photo}
											<span class="shrink-0 text-label text-ink-2 uppercase">
												{m.promote_step_has_one()}
											</span>
										{/if}
									</button>
								</li>
							{/if}
						{/each}
					</ul>
				</details>
			</div>
		{/if}
	</div>

	<!-- What saying yes does, said once there is something to say it about. -->
	{#if content && target !== undefined}
		<div class="border-t border-rule bg-ground-2 px-gutter py-3" role="status">
			<p class="text-label uppercase {forking ? 'text-support' : 'text-accent'}">
				{forking ? m.write_will_fork() : m.write_will_save()}
			</p>
			<p class="mt-1 text-read">
				{forking && recipe
					? copySaid(recipe, content.title)
					: `${m.write_said_save({ title: content.title })} ${m.promote_in_thread()}`}
			</p>
			<p class="mt-2 text-read">{m.promote_public()}</p>
		</div>
	{/if}
	{#if refused}
		<p class="px-gutter py-2 text-read text-support" role="alert">
			{m.promote_failed()}
			{refused}
		</p>
	{/if}

	<div class="flex border-t border-rule">
		<button
			type="button"
			class="flex-1 p-4 text-center font-display text-body text-ink-2 disabled:opacity-40"
			disabled={working}
			onclick={onClose}
		>
			{m.promote_cancel()}
		</button>
		<NeedsServer
			label={working ? m.promote_doing() : forking ? m.write_do_fork() : m.promote_do()}
			waiting={m.offline_waits_save()}
			onclick={promote}
			disabled={!chosen || target === undefined || working || !content}
			shapeClass="flex-1 p-4 text-center font-display text-body disabled:opacity-40"
			lookClass={forking ? 'bg-support text-on-accent' : 'bg-accent text-on-accent'}
		/>
	</div>
</SheetFrame>

<!-- A choice's square, filled when chosen: the house pattern from the Tags sheet. -->
{#snippet box(on: boolean)}
	<span
		class="mt-1 block h-4 w-4 shrink-0 rounded-sm border border-accent {on ? 'bg-accent' : ''}"
		aria-hidden="true"
	></span>
{/snippet}
