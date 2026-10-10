<!--
	An older Version, opened as a recipe page (#211).

	ADR 0014 again: the reader stands in a whole recipe, and what differs is
	marked on the line it belongs to. Here the two recipes are one Branch at
	two moments. The page is the Version in the URL, and the lines it does not
	share with the newest Version of its Branch are marked the way two
	Branches are, by the same rows and the same row component. The Core pairs
	them (`changed_since`); nothing here does.

	THREE CHOICES were made against a prototype on 10 October 2026, and are
	recorded on #211:

	  · a dark bar stays at the top of the window for as long as the page is
	    read, so the amounts five screens down are never read as the
	    recipe's. It is ink, where the strip of versions is indigo or beni
	    and holds chips: an older Version is not another Branch;
	  · the page keeps what is read and cooked from, and leaves the rest to
	    the recipe as it stands. No Tags, Related Recipes or Cooked, which are
	    the dish's. No editing, Language, renaming, sharing or deleting,
	    which change a recipe, and a Version never changes. No amount picker,
	    Sheet or Shopping List, which read a Branch. A Reading is not
	    corrected here either;
	  · a marked line can be written back into the recipe, by whoever may
	    write the recipe. It is carried and saved exactly as a line from
	    another Branch is: unsaved until an ordinary Version is saved.

	One column at reading width on every layout, as every page with marks is
	(#192, story 64).

	The newest Version is the recipe, so its address goes there.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { m } from '$lib/paraglide/messages';
	import { aDate } from '$lib/dates';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { ChangedSinceOutput } from '$lib/api/catalogue';
	import StepWords from '$lib/StepWords.svelte';
	import StepPhoto from '$lib/StepPhoto.svelte';
	import { tappableLink } from '$lib/source';
	import Hero from '../Hero.svelte';
	import MetaStrip from '../MetaStrip.svelte';
	import MarkedRow from '../MarkedRow.svelte';
	import ComponentLine from '../ComponentLine.svelte';
	import Unfolded from '../Unfolded.svelte';
	import Annexe from '../Annexe.svelte';
	import Carrying from '../Carrying.svelte';
	import { nutritionText, numbering, rowKey } from '../divergence';
	import { marking } from '../marking.svelte';
	import { unfolding } from '../unfolding.svelte';
	import { corrections } from '../corrections.svelte';
	import { proseSince, sinceThen } from './older';

	interface Props {
		branchId: string;
		versionId: string;
		/** Going to another page. A test hands in its own. */
		navigate?: (to: string, how?: { replaceState: boolean }) => Promise<void>;
	}

	let { branchId, versionId, navigate = goto }: Props = $props();

	const kamosu = useKamosu();

	/**
	 * What each single value is called where the recipe now holds another.
	 * The mark stands apart from the value it is about, and two of them on
	 * one page both read "nothing", so each says which it is.
	 */
	const WHAT = {
		title: m.older_what_title,
		source: m.older_what_source,
		yield: m.older_what_yield,
		prep_time_minutes: m.older_what_prep,
		cook_time_minutes: m.older_what_cook,
		nutrition: m.older_what_nutrition,
	};

	let compared = $state<ChangedSinceOutput | undefined>(undefined);
	let failed = $state(false);
	/** Bumped after a save, to read the two again: the recipe has moved. */
	let reread = $state(0);
	let cooking = $state<'idle' | 'starting' | 'failed' | 'another'>('idle');

	$effect(() => {
		void reread;
		const asked = { branch_id: branchId, version_id: versionId };
		let current = true;
		kamosu
			.changedSince(asked)
			.then((answer) => {
				if (!current) return;
				// The newest Version is the recipe. Its page is the recipe's.
				if (answer.version.newest) {
					void navigate(`/recipes/${asked.branch_id}`, { replaceState: true });
					return;
				}
				compared = answer;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				if (current) failed = true;
			});
		return () => {
			current = false;
		};
	});

	/** When this Version was saved, as every sentence here says it. */
	const when = $derived(compared ? aDate(compared.version.created_at) : '');
	const words = $derived(sinceThen(compared?.writes ?? false));

	/** The marks, and what has been carried back to the recipe and not saved. */
	const marks = marking({
		kamosu,
		divergence: () => compared,
		// The list is rebuilt from a different set of rows.
		onToggled: () => unfold.closeAll(),
		onSaved: () => (reread += 1),
		older: {
			field(name, value) {
				if (name === 'note') return m.older_field_note_now({ value });
				// The Main Photo is never marked in words: the page shows it.
				return name === 'main_photo' ? '' : m.older_field_now({ what: WHAT[name](), value });
			},
			prose: (divergence, taken) => proseSince(when, divergence, taken),
		},
	});
	// The page is the older Version, which the rows call `theirs`.
	marks.side = 'theirs';

	const then = $derived(compared?.theirs);
	const content = $derived(then?.content);
	const measured = $derived(then?.measured ?? { ingredients: [], steps: [] });
	const unfold = unfolding(() => then?.components ?? []);
	/** Only read from: the one line beneath an Ingredient Line, as the recipe words it. */
	const beneath = corrections({
		readings: () => then?.readings ?? [],
		measured: () => measured,
		written: (index) => content?.ingredients[index]?.text ?? '',
	});

	const saved = $derived.by(() => {
		if (!compared) return '';
		const who = compared.version.hand_name ?? m.thread_unnamed_hand();
		const name = compared.version.name;
		return name ? m.older_saved_named({ when, who, name }) : m.older_saved({ when, who });
	});

	/**
	 * Cook this Version. The Attempt is pinned to it, and the cooking screen
	 * is handed the cooking that is already under way (ADR 0005, ADR 0011).
	 *
	 * A dish has one cooking at a time, so where one is under way from
	 * another Version the Core hands that one back. Going to it would open a
	 * recipe other than the one the button named, so the page says so
	 * and stays.
	 */
	async function cook() {
		cooking = 'starting';
		try {
			const started = await kamosu.startAttempt({ branch_id: branchId, version_id: versionId });
			if (started.version_id !== versionId) {
				cooking = 'another';
				return;
			}
			await navigate(`/cook/${branchId}`);
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			cooking = 'failed';
		}
	}
</script>

<svelte:head>
	<title>{content ? `${content.title} · ` : ''}{m.older_bar_title()} · {m.app_name()}</title>
</svelte:head>

{#snippet sourceLine(source: NonNullable<NonNullable<typeof content>['source']>)}
	{@const href = tappableLink(source.link)}
	{#if href}
		<a {href} target="_blank" rel="noopener noreferrer" class="underline underline-offset-2">
			{m.recipe_from_source({ source: source.text })}<span aria-hidden="true">&nbsp;↗</span><span
				class="sr-only">, {m.recipe_source_opens()}</span
			>
		</a>
	{:else}
		{m.recipe_from_source({ source: source.text })}
	{/if}
{/snippet}

{#snippet titleOnHero()}
	{#if content?.main_photo && content.source}
		<p class="text-label text-on-accent uppercase">{@render sourceLine(content.source)}</p>
	{/if}
	<h1 class="mt-1 font-display text-title font-semibold text-on-accent">{content?.title}</h1>
{/snippet}

{#snippet fieldMarks(names: Parameters<typeof marks.markOf>[0][])}
	{#each names as name (name)}
		{#if marks.markOf(name)}
			<p class="mt-1 px-gutter text-read text-accent">{marks.markOf(name)}</p>
		{/if}
	{/each}
{/snippet}

{#snippet ingredientLine(text: string, at: number)}
	{@const component = unfold.at([], at)}
	<li class="flex gap-3 border-b border-rule py-3">
		<span
			class="ingredient-marker shrink-0 {component ? 'bg-support-2' : 'bg-accent'}"
			aria-hidden="true"
		></span>
		<div class="min-w-0 flex-1">
			<span class="block text-line">{text}</span>
			{#if component}
				<ComponentLine
					{component}
					open={unfold.isOpen(component.path)}
					toggle={() => unfold.toggle(component.path)}
				/>
				{#if unfold.isOpen(component.path)}
					<Unfolded {component} at={unfold.at} />
				{/if}
			{:else if beneath.beneathLine(at)}
				<span class="block text-read text-ink-2">{beneath.beneathLine(at)}</span>
			{/if}
		</div>
	</li>
{/snippet}

{#snippet stepLine(text: string, at: number, n: number | null, photo: string | null)}
	<li class="flex gap-3 border-b border-rule py-3">
		<span class="w-6 shrink-0 font-display text-line font-semibold text-accent">{n}</span>
		<div class="min-w-0 flex-1">
			<p class="text-body">
				<StepWords
					{text}
					conversions={measured.steps[at] ?? []}
					readingClass="text-read text-ink-2"
				/>
			</p>
		</div>
		{#if photo && n !== null}
			<StepPhoto
				photograph={photo}
				number={n}
				shapeClass="h-[var(--photo-thumb)] w-[var(--photo-thumb)]"
			/>
		{/if}
	</li>
{/snippet}

{#snippet section(text: string)}
	<li class="border-b border-rule py-4 pb-1">
		<h3 class="font-display text-label text-ink-2 uppercase">{text}</h3>
	</li>
{/snippet}

<div class="older mx-auto max-w-2xl pb-tabbar">
	{#if failed}
		<p class="px-gutter py-6 text-body text-support" role="alert">{m.older_failed()}</p>
	{:else if !compared || !content}
		<p class="px-gutter py-6 text-body text-ink-2">{m.loading()}</p>
	{:else}
		<!--
			The bar that follows (choice B). Ink, with the paper's colour for
			its words: `text-on-accent` is also what tells the stylesheet what
			shows under the pointer on a dark ground (#203).
		-->
		<div class="older-bar flex min-h-12 items-center gap-3 bg-ink px-gutter py-2 text-on-accent">
			<p class="min-w-0 flex-1 text-read">
				<b class="block font-display text-body font-semibold">{m.older_bar_title()}</b>
				{when}
			</p>
			<a
				href="/recipes/{branchId}"
				class="tap-out shrink-0 rounded-sm border border-on-accent px-[10px] py-[6px] text-read"
			>
				{m.older_bar_back()}
			</a>
		</div>

		<Hero
			lineageId={compared.lineage_id}
			title={content.title}
			photo={content.main_photo}
			over={titleOnHero}
		/>

		<p class="px-gutter pt-3 text-read text-ink-2">{saved}</p>
		{#if content.source && !content.main_photo}
			<p class="px-gutter pt-3 text-label text-ink-2 uppercase">
				{@render sourceLine(content.source)}
			</p>
		{/if}

		{#if marks.unshared === 0}
			<p class="border-b border-rule px-gutter py-2 text-read text-ink-2">
				{m.older_unshared_none()}
			</p>
		{:else}
			<div class="flex items-center gap-3 border-b border-rule px-gutter py-2">
				<p class="flex-1 text-read text-ink-2">
					{#if !marks.marks}
						{m.divergence_put_away({ count: marks.unshared })}
					{:else if marks.unshared === 1}
						{m.older_unshared_one()}
					{:else}
						{m.older_unshared({ count: marks.unshared })}
					{/if}
				</p>
				<button
					type="button"
					onclick={marks.toggleMarks}
					class="rounded-sm border border-rule bg-card px-3 py-1 text-read text-accent"
				>
					{marks.marks ? m.divergence_hide() : m.divergence_show()}
				</button>
			</div>
		{/if}
		{@render fieldMarks(['title', 'source'])}

		<MetaStrip {content} />
		{@render fieldMarks(['prep_time_minutes', 'cook_time_minutes', 'yield'])}

		<button
			type="button"
			disabled={cooking === 'starting' || cooking === 'another'}
			onclick={cook}
			class="mx-gutter mt-4 flex min-h-12 w-[calc(100%-2*var(--spacing-gutter))] items-center justify-center rounded-sm bg-accent px-4 text-center font-display text-body font-semibold text-on-accent disabled:opacity-60"
		>
			{m.older_cook()}
		</button>
		{#if cooking === 'another'}
			<p class="mx-gutter mt-2 text-read text-support" role="alert">
				{m.older_cooking_another()}
				<a href="/cook/{branchId}" class="text-accent underline underline-offset-2">
					{m.older_cooking_open()}
				</a>
			</p>
		{:else if cooking === 'failed'}
			<p class="mx-gutter mt-2 text-read text-support" role="alert">
				{m.thread_cooking_failed()}
			</p>
		{/if}

		<h2 class="mx-gutter mt-8 mb-2 font-display text-label font-semibold text-accent uppercase">
			{m.recipe_ingredients()}
		</h2>
		<ul class="px-gutter">
			{#if marks.showing}
				{#each compared.ingredients as row, index (rowKey('ingredients', index))}
					{@const key = rowKey('ingredients', index)}
					{@const own = row.theirs}
					{#if row.kind === 'section'}
						{@render section((own ?? row.mine)?.text ?? '')}
					{:else if row.state === 'same'}
						{@render ingredientLine(own?.text ?? '', own?.index ?? -1)}
					{:else}
						<!-- A marked line that names a recipe says which, and does not
						     unfold, as on the recipe page (#50). -->
						<MarkedRow
							{row}
							side="theirs"
							{words}
							beneath={own
								? (unfold.at([], own.index)?.said ?? beneath.beneathLine(own.index))
								: ''}
							open={marks.isOpen(key)}
							taken={marks.taken.get(key)}
							onToggle={() => marks.toggle(key)}
							onCarry={(what) => marks.carry(key, what)}
						/>
					{/if}
				{/each}
			{:else}
				{#each content.ingredients as item, index (index)}
					{#if item.kind === 'section'}
						{@render section(item.text)}
					{:else}
						{@render ingredientLine(item.text, index)}
					{/if}
				{/each}
			{/if}
		</ul>
		{#if nutritionText(content.nutrition ?? null)}
			<p class="mt-3 px-gutter text-read text-ink-2">{nutritionText(content.nutrition ?? null)}</p>
		{/if}
		{@render fieldMarks(['nutrition'])}

		<h2 class="mx-gutter mt-8 mb-2 font-display text-label font-semibold text-accent uppercase">
			{m.recipe_method()}
		</h2>
		<ol class="px-gutter">
			{#if marks.showing}
				{@const number = numbering()}
				{#each compared.steps as row, index (rowKey('steps', index))}
					{@const key = rowKey('steps', index)}
					{@const own = row.theirs}
					{@const n = row.kind === 'section' ? null : number(!own)}
					{#if row.kind === 'section'}
						{@render section((own ?? row.mine)?.text ?? '')}
					{:else if row.state === 'same' && own}
						{@render stepLine(own.text, own.index, n, content.steps[own.index]?.photo ?? null)}
					{:else}
						<MarkedRow
							{row}
							side="theirs"
							{words}
							number={n}
							conversions={own ? (measured.steps[own.index] ?? []) : []}
							photo={own ? (content.steps[own.index]?.photo ?? null) : null}
							open={marks.isOpen(key)}
							taken={marks.taken.get(key)}
							onToggle={() => marks.toggle(key)}
							onCarry={(what) => marks.carry(key, what)}
						/>
					{/if}
				{/each}
			{:else}
				{@const number = numbering()}
				{#each content.steps as item, index (index)}
					{#if item.kind === 'section'}
						{@render section(item.text)}
					{:else}
						{@render stepLine(item.text, index, number(false), item.photo)}
					{/if}
				{/each}
			{/if}
		</ol>

		{#each unfold.annexes as component (component.path.join('.'))}
			<Annexe {component} />
		{/each}

		{#if content.note}
			<div class="mx-gutter mt-6 border-l-2 border-accent py-1 pl-4 text-body whitespace-pre-wrap">
				{content.note}
			</div>
		{/if}
		{@render fieldMarks(['note'])}

		{#if marks.saved === 'yes'}
			<p class="mx-gutter mt-4 text-read text-accent" role="status">
				{m.older_saved_as_version()}
			</p>
		{:else if marks.saved === 'failed'}
			<p class="mx-gutter mt-4 text-read text-support" role="alert">
				{m.divergence_save_failed()}
			</p>
		{/if}

		<div class="mx-gutter mt-6 grid grid-cols-2 gap-2">
			<a href="/recipes/{branchId}" class="act-tile text-accent">{m.older_back()}</a>
			<a href="/recipes/{branchId}/thread" class="act-tile text-accent">{m.recipe_the_thread()}</a>
		</div>

		<Carrying marking={marks} {words} />
	{/if}
</div>

<style>
	/* A marked line takes indigo and a Ghost beni, as on a reader's own
	   recipe: both are one Branch, so neither is anybody else's. `MarkedRow`
	   reads `--whose`, and turns the two round under `data-side='theirs'`,
	   which this page therefore does not say. */
	.older {
		--whose: var(--color-accent);
	}

	/* Stays at the top of the window while the page is read, below the
	   phone's own status bar where the app is installed. */
	.older-bar {
		position: sticky;
		top: env(safe-area-inset-top);
		z-index: 15;
	}
</style>
