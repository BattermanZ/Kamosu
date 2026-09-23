<!--
	The Thread drawn as a graph (#115, Aurélien's choice "G1", replacing #53's
	side-by-side lanes). One row per Version, in date order across every
	Branch; a dot per Version on a line down the left edge, grey while
	several Branches share it and a colour per Branch from where it splits.
	`graph.ts` decides the shape; this only draws it.

	A Version you saved can be named or renamed in place, under its row
	(Aurélien's choice "A"). Its *what changed* line is shown beside the field
	and never offered for editing: it records why an edit was made, at the
	moment that was true, and nothing can write it afterwards (#83).
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { ratingLabel } from '$lib/rating';
	import { isUnknown, languageName } from '$lib/language';
	import { OperationError } from '$lib/api/client';
	import { useKamosu } from '$lib/kamosu';
	import {
		foldedLines,
		graphOf,
		linesBelow,
		rowKey,
		runsOf,
		type GraphRow,
		type Line,
		type Tone,
		type VersionRow,
	} from './graph';
	import type { ThreadBranch, ThreadVersion } from './tree';
	import type { GetThreadOutput } from '$lib/api/catalogue';

	type Attempt = GetThreadOutput['attempts'][number];

	interface Props {
		thread: GetThreadOutput;
		/** The reader's own Hand: only the Versions it saved offer a rename. */
		me: string | undefined;
		attemptsByVersion: Map<string, Attempt[]>;
		/** Which rows said what Language this recipe is in (#106). */
		languageSaid: Map<string, string>;
		onOpenVersion: (version: ThreadVersion) => void;
		onOpenAttempt: (attempt: Attempt) => void;
		/** A rename landed: read the Thread again. */
		onRenamed: () => Promise<void>;
	}

	let {
		thread,
		me,
		attemptsByVersion,
		languageSaid,
		onOpenVersion,
		onOpenAttempt,
		onRenamed,
	}: Props = $props();

	const kamosu = useKamosu();

	const branches = $derived(new Map(thread.branches.map((branch) => [branch.branch_id, branch])));
	const graph = $derived(
		graphOf(
			thread.branches.map((branch) => branch.branch_id),
			thread.versions,
		),
	);

	/**
	 * A save with nothing written down, which runs of get folded away. Saying
	 * a Language is never quiet (#106): nothing was typed, but it is the whole
	 * reason that row exists.
	 */
	const isQuiet = (row: VersionRow) =>
		!row.version.name && !row.version.change_note && !languageSaid.has(rowKey(row.version));

	let expanded = $state(new Set<string>());
	const items = $derived(runsOf(graph.rows, isQuiet));

	function toggle(key: string) {
		const next = new Set(expanded);
		if (next.has(key)) next.delete(key);
		else next.add(key);
		expanded = next;
	}

	// ---- drawing ------------------------------------------------------------

	const LANE_W = 18;
	const PAD = 9;
	/** How far down a row its dot's centre sits, in px. */
	const DOT_Y = 17;
	const FORK_H = 44;
	const gutter = $derived(PAD * 2 + (graph.lanes - 1) * LANE_W);
	const x = (lane: number) => PAD + lane * LANE_W;

	/** The existing rail colours, in the order #53 handed them out. */
	const TONES = ['var(--color-accent)', 'var(--color-support)', 'var(--color-support-2)'];
	const colour = (tone: Tone) =>
		tone === 'shared' ? 'var(--color-ink-2)' : TONES[tone % TONES.length];

	function lineStyle(line: Line): string {
		const spans = {
			full: 'top:0;bottom:0',
			above: `top:0;height:${DOT_Y}px`,
			below: `top:${DOT_Y}px;bottom:0`,
			none: 'display:none',
		};
		return `left:${x(line.lane) - 1}px;width:2px;${spans[line.reach]};background:${colour(line.tone)}`;
	}
	/**
	 * Round, the one exception to the near-square direction in app.css:
	 * Aurélien asked for the Thread as "coloured balls, like git" and chose
	 * the round dots of prototype G1 (#115).
	 */
	const dotStyle = (row: VersionRow, hollow = false) =>
		`left:${x(row.lane) - 6}px;top:${DOT_Y - 6}px;width:12px;height:12px;border-radius:50%;` +
		(hollow
			? `background:var(--color-ground);border:2px solid ${colour(row.tone)}`
			: `background:${colour(row.tone)};box-shadow:0 0 0 2px var(--color-ground)`);

	/** A split drawn: straight lines for what passes, a curve into each continuation. */
	function forkPaths(fork: Extract<GraphRow, { kind: 'fork' }>): { d: string; tone: Tone }[] {
		const from = x(fork.from);
		return [
			...fork.passing.map((line) => ({ d: `M${x(line.lane)} 0 V ${FORK_H}`, tone: line.tone })),
			...fork.children.map((child) => ({
				d: `M${from} 0 C ${from} ${FORK_H * 0.6}, ${x(child.lane)} ${FORK_H * 0.4}, ${x(child.lane)} ${FORK_H}`,
				tone: child.tone,
			})),
		];
	}

	/**
	 * What names a Branch, or a line several still share: whose it is, and —
	 * where it is worth saying — the Language it stands in. Never a Language
	 * code, and never a mark on a Branch whose Language is Unknown (#106,
	 * ADR 0006); English is unmarked, as on the shelf.
	 */
	function branchLabel(branchIds: string[]): string {
		return branchIds
			.map((id) => branches.get(id))
			.filter((branch): branch is ThreadBranch => !!branch)
			.map((branch) =>
				branch.language === 'en' || isUnknown(branch.language)
					? handName(branch)
					: `${handName(branch)} (${languageName(branch.language)})`,
			)
			.join(', ');
	}

	/**
	 * Who a Hand is, in words (#113, ADR 0015), named live by the server. A
	 * Hand nothing here has a name for is said to be one, never printed as an id.
	 */
	function handName(holder: { hand_name: string | null }): string {
		return holder.hand_name ?? m.thread_unnamed_hand();
	}

	// ---- renaming -----------------------------------------------------------

	/** The row being named, by key. */
	let editing = $state<string | undefined>(undefined);
	let draft = $state('');
	let saving = $state(false);
	let renameError = $state<string | undefined>(undefined);
	/** What the last rename did, said under the row it did it to. */
	let lastRename = $state<{ key: string; cleared: boolean } | undefined>(undefined);

	/**
	 * A Version is yours to name when your Hand saved it (#115) — on every
	 * Branch the row stands for, since naming it names each of them. The
	 * server refuses anyone else; the screen does not offer it.
	 */
	const yours = (row: VersionRow) =>
		me !== undefined && row.occurrences.every((occurrence) => occurrence.hand_id === me);

	function startNaming(row: VersionRow) {
		editing = row.key;
		draft = row.version.name ?? '';
		renameError = undefined;
		lastRename = undefined;
	}

	/**
	 * Name every occurrence the row stands for. A Version several Branches
	 * still share is one row on this screen, so it takes one name; each
	 * Branch holds its own occurrence, and `rename_version` names them one by one.
	 * An empty name clears it, which is an answer and not a mistake.
	 */
	async function rename(row: VersionRow, name: string | null) {
		saving = true;
		renameError = undefined;
		try {
			for (const { branch_id, sequence } of row.occurrences) {
				await kamosu.renameVersion({ branch_id, sequence, name });
			}
			editing = undefined;
			lastRename = { key: row.key, cleared: !name?.trim() };
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			renameError = m.thread_rename_failed();
		} finally {
			saving = false;
		}
		// Read back what landed either way: a shared row that failed part of
		// the way has named some Branches already, and the screen should say so.
		await onRenamed().catch((error: unknown) => {
			if (!(error instanceof OperationError)) throw error;
		});
	}

	/** What a row says it was: what was written down, or what the save itself did. */
	function whatItWas(version: ThreadVersion): string {
		const said = languageSaid.get(rowKey(version));
		if (said !== undefined) return m.thread_said_language({ language: languageName(said) });
		return version.change_note ?? m.thread_quiet_save();
	}
</script>

{#snippet gutterOf(row: VersionRow, hollow = false)}
	<span class="relative shrink-0" style={`width:${gutter}px`} aria-hidden="true">
		{#each row.lines as line (line.lane)}
			<span class="absolute" style={lineStyle(line)}></span>
		{/each}
		<span class="absolute" style={dotStyle(row, hollow)}></span>
	</span>
{/snippet}

{#snippet when(row: VersionRow)}
	<!-- After the first split, a row says which Branch it is on, in its colour. -->
	{#if row.labelled}
		<span style={`color:${colour(row.tone)}`}>{branchLabel(row.branchIds)}</span> ·
	{/if}
	{row.version.created_at.slice(0, 10)}
{/snippet}

{#snippet versionRow(row: VersionRow)}
	{@const version = row.version}
	<li class="flex">
		{@render gutterOf(row)}
		<div class="min-w-0 flex-1 border-b border-rule py-2 pl-2">
			{#if editing === row.key}
				<form
					class="grid gap-2"
					onsubmit={(event) => {
						event.preventDefault();
						void rename(row, draft);
					}}
				>
					<p class="text-label text-ink-2 uppercase">{@render when(row)}</p>
					<label class="grid gap-1 text-body text-ink">
						{m.thread_name_label()}
						<input
							class="min-h-12 min-w-0 rounded-sm border border-rule bg-card px-3 text-body text-ink"
							bind:value={draft}
							placeholder={m.thread_name_placeholder()}
							{@attach (node) => node.focus()}
						/>
					</label>
					{#if renameError}
						<p class="text-body text-accent" role="alert">{renameError}</p>
					{/if}
					<div class="flex flex-wrap items-center gap-4">
						<button
							class="min-h-12 rounded-sm bg-accent px-4 font-semibold text-on-accent disabled:opacity-60"
							disabled={saving}
						>
							{m.thread_name_save()}
						</button>
						{#if version.name}
							<button
								type="button"
								class="text-read text-accent underline"
								disabled={saving}
								onclick={() => rename(row, null)}
							>
								{m.thread_name_remove()}
							</button>
						{/if}
						<button
							type="button"
							class="text-read text-accent underline"
							disabled={saving}
							onclick={() => (editing = undefined)}
						>
							{m.thread_name_keep()}
						</button>
					</div>
					<!--
						Shown, never a field: the why of an edit is written when it is
						true, and the caption says the fixing is deliberate (#83).
					-->
					<div>
						<p class="text-label text-ink-2 uppercase">{m.thread_what_changed()}</p>
						<p class="text-read text-ink">{whatItWas(version)}</p>
						<p class="text-label text-ink-2">{m.thread_what_changed_fixed()}</p>
					</div>
				</form>
			{:else}
				<button type="button" class="block w-full text-left" onclick={() => onOpenVersion(version)}>
					<span class="block text-label text-ink-2 uppercase">{@render when(row)}</span>
					<span class="block font-display text-body text-ink">
						{handName(version)}{#if version.name}
							<!-- In an expression: a space leading a tag's text is trimmed away. -->
							<span class="text-accent">{` · “${version.name}”`}</span>
						{/if}
					</span>
					<span class="block text-read text-ink-2">{whatItWas(version)}</span>
				</button>
				{#if attemptsByVersion.get(version.version_id)?.length || yours(row)}
					<div class="mt-1 flex flex-wrap items-center gap-2">
						{#each attemptsByVersion.get(version.version_id) ?? [] as attempt (attempt.id)}
							<button
								type="button"
								class="rounded-sm border border-rule px-1 text-label text-ink-2"
								onclick={() => onOpenAttempt(attempt)}
							>
								{attempt.rating ? `🍲 ${ratingLabel(attempt.rating)}` : '🍲'}
							</button>
						{/each}
						{#if yours(row)}
							<button
								type="button"
								class="text-read text-accent underline"
								onclick={() => startNaming(row)}
							>
								{version.name ? m.thread_rename() : m.thread_name_it()}
							</button>
						{/if}
					</div>
				{/if}
				{#if lastRename?.key === row.key}
					<p
						role="status"
						class="mt-2 border-l-3 border-accent bg-card px-3 py-2 text-read text-ink"
					>
						{lastRename.cleared ? m.thread_name_removed() : m.thread_named()}
					</p>
				{/if}
			{/if}
		</div>
	</li>
{/snippet}

<ul>
	{#each items as item (item.kind === 'run' ? item.key : item.row.key)}
		{#if item.kind === 'run' && !expanded.has(item.key)}
			{@const first = item.rows[0]}
			<li class="flex">
				{@render gutterOf({ ...first, lines: foldedLines(item.rows) }, true)}
				<button
					type="button"
					class="flex min-h-12 min-w-0 flex-1 items-center gap-3 border-b border-rule py-2 pl-2 text-left"
					onclick={() => toggle(item.key)}
				>
					<span class="min-w-0 flex-1 text-read text-ink-2">
						{m.thread_quiet_run({
							count: item.rows.length,
							from: first.version.created_at.slice(0, 10),
							to: item.rows[item.rows.length - 1].version.created_at.slice(0, 10),
						})}
					</span>
					<span class="shrink-0 text-label text-accent uppercase">{m.thread_show()}</span>
				</button>
			</li>
		{:else if item.kind === 'run'}
			{#each item.rows as row (row.key)}
				{@render versionRow(row)}
			{/each}
			<li class="flex">
				<span class="relative shrink-0" style={`width:${gutter}px`} aria-hidden="true">
					{#each linesBelow(item.rows[item.rows.length - 1]) as line (line.lane)}
						<span class="absolute" style={lineStyle(line)}></span>
					{/each}
				</span>
				<button
					type="button"
					class="w-full py-1 pl-2 text-left text-label text-ink-2 uppercase"
					onclick={() => toggle(item.key)}
				>
					{m.thread_collapse()}
				</button>
			</li>
		{:else if item.row.kind === 'version'}
			{@render versionRow(item.row)}
		{:else}
			{@const fork = item.row}
			<li class="flex bg-ground-2">
				<svg
					class="block shrink-0 self-stretch"
					width={gutter}
					viewBox={`0 0 ${gutter} ${FORK_H}`}
					preserveAspectRatio="none"
					style="height:auto;min-height:44px"
					aria-hidden="true"
				>
					{#each forkPaths(fork) as path (path.d)}
						<path
							d={path.d}
							stroke={colour(path.tone)}
							stroke-width="2"
							fill="none"
							vector-effect="non-scaling-stroke"
						/>
					{/each}
				</svg>
				<div class="min-w-0 flex-1 py-2 pl-2">
					<p class="font-display text-body text-support-2">
						{m.thread_fork_marker({ count: fork.children.length })}
					</p>
					<p class="mt-1 flex flex-wrap gap-x-3 gap-y-1 text-label text-ink-2">
						{#each fork.children as child (child.lane)}
							<span>
								<span
									class="mr-1 inline-block h-2 w-2"
									style={`border-radius:50%;background:${colour(child.tone)}`}
								></span>{branchLabel(child.branchIds)}
							</span>
						{/each}
					</p>
				</div>
			</li>
		{/if}
	{/each}
</ul>
