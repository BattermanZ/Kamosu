<!--
	One group of the Thread: a shared trunk of Versions, then — where the
	group's Branches actually diverge — a column per continuation, laid out
	side by side (Aurélien's choice among #53's three directions: "A — Lanes").

	A rail that itself forks again past this point (a Branch forking off
	another Branch, not the trunk) cannot become a third column on a phone —
	so a rail shows only its OWN trunk, and any further fork inside it is
	deferred to a full-width section rendered below the whole grid, via
	`hideTrunk` (its trunk was already shown in the rail; only what comes
	after it still needs drawing). A known, accepted cost for a case that
	stays rare.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { ratingLabel } from '$lib/rating';
	import ThreadGroup from './ThreadGroup.svelte';
	import type { ForkGroup, ThreadBranch, ThreadVersion } from './tree';
	import type { GetThreadOutput } from '$lib/api/catalogue';

	type Attempt = GetThreadOutput['attempts'][number];

	interface Props {
		group: ForkGroup;
		branches: Map<string, ThreadBranch>;
		attemptsByVersion: Map<string, Attempt[]>;
		onOpenVersion: (version: ThreadVersion) => void;
		onOpenAttempt: (attempt: Attempt) => void;
		/** Rendered inside a rail — smaller, with a left rule — rather than as
		 *  the screen's own plain column. */
		nested?: boolean;
		/** This group's own trunk was already drawn by the rail that deferred
		 *  to this instance — skip it and go straight to what forks past it. */
		hideTrunk?: boolean;
	}

	let {
		group,
		branches,
		attemptsByVersion,
		onOpenVersion,
		onOpenAttempt,
		nested = false,
		hideTrunk = false
	}: Props = $props();

	const RUN_THRESHOLD = 4;
	const isQuiet = (version: ThreadVersion) => !version.name && !version.change_note;
	const rowKey = (version: ThreadVersion) => `${version.branch_id}:${version.sequence}`;

	let expanded = $state(new Set<string>());

	interface Run {
		key: string;
		versions: ThreadVersion[];
		collapsible: boolean;
	}

	function runsOf(trunk: ThreadVersion[]): Run[] {
		const out: Run[] = [];
		let quiet: ThreadVersion[] = [];
		const flush = () => {
			if (!quiet.length) return;
			out.push({ key: rowKey(quiet[0]), versions: quiet, collapsible: quiet.length >= RUN_THRESHOLD });
			quiet = [];
		};
		for (const version of trunk) {
			if (isQuiet(version)) {
				quiet.push(version);
			} else {
				flush();
				out.push({ key: rowKey(version), versions: [version], collapsible: false });
			}
		}
		flush();
		return out;
	}

	function toggle(key: string) {
		const next = new Set(expanded);
		next.has(key) ? next.delete(key) : next.add(key);
		expanded = next;
	}

	function railLabel(branchIds: string[]): string {
		return branchIds
			.map((id) => branches.get(id))
			.filter((branch): branch is ThreadBranch => !!branch)
			.map((branch) => (branch.language === 'en' ? branch.hand_id : `${branch.hand_id} (${branch.language})`))
			.join(', ');
	}

	const RAIL_COLORS = ['border-accent', 'border-support', 'border-support-2'];
	function railColor(index: number): string {
		return RAIL_COLORS[index % RAIL_COLORS.length];
	}
</script>

{#snippet trunkList(trunk: ThreadVersion[])}
	{#each runsOf(trunk) as run (run.key)}
		{#if run.collapsible && !expanded.has(run.key)}
			<button
				type="button"
				class="flex min-h-12 w-full items-center gap-3 border-b border-rule py-2 text-left"
				onclick={() => toggle(run.key)}
			>
				<span class="h-2 w-2 shrink-0 rounded-sm bg-rule"></span>
				<span class="min-w-0 flex-1 text-read text-ink-2">
					{m.thread_quiet_run({
						count: run.versions.length,
						from: run.versions[0].created_at.slice(0, 10),
						to: run.versions[run.versions.length - 1].created_at.slice(0, 10)
					})}
				</span>
				<span class="shrink-0 text-label uppercase text-accent">{m.thread_show()}</span>
			</button>
		{:else}
			{#each run.versions as version (rowKey(version))}
				<div class="border-b border-rule py-2">
					<button
						type="button"
						class="flex w-full items-start gap-3 text-left"
						onclick={() => onOpenVersion(version)}
					>
						<span class="mt-1 h-2.5 w-2.5 shrink-0 rounded-sm bg-accent"></span>
						<span class="min-w-0 flex-1">
							<span class="block text-label uppercase text-ink-2">
								{version.created_at.slice(0, 10)}
							</span>
							<span class="block font-display text-body text-ink">
								{version.hand_id}{#if version.name}
									<span class="text-accent"> · “{version.name}”</span>
								{/if}
							</span>
							<span class="block text-read text-ink-2">
								{version.change_note ?? m.thread_quiet_save()}
							</span>
						</span>
					</button>
					{#if attemptsByVersion.get(version.version_id)?.length}
						<div class="mt-1 ml-6 flex flex-wrap gap-1">
							{#each attemptsByVersion.get(version.version_id) ?? [] as attempt (attempt.id)}
								<button
									type="button"
									class="rounded-sm border border-rule px-1 text-label text-ink-2"
									onclick={() => onOpenAttempt(attempt)}
								>
									{attempt.rating ? `🍲 ${ratingLabel(attempt.rating)}` : '🍲'}
								</button>
							{/each}
						</div>
					{/if}
				</div>
			{/each}
			{#if run.collapsible}
				<button
					type="button"
					class="w-full py-1 text-left text-label uppercase text-ink-2"
					onclick={() => toggle(run.key)}
				>
					{m.thread_collapse()}
				</button>
			{/if}
		{/if}
	{/each}
{/snippet}

<div class={nested ? 'border-l border-rule pl-3' : undefined}>
	{#if !hideTrunk}
		{@render trunkList(group.trunk)}
	{/if}

	{#if group.children.length >= 2}
		<div class="border-l-4 border-support-2 bg-ground-2 p-3">
			<p class="font-display text-body text-support-2">
				{m.thread_fork_marker({ count: group.children.length })}
			</p>
			<p class="mt-1 text-read text-ink-2">{m.thread_fork_detail()}</p>
		</div>
		<div
			class="grid gap-0"
			style={`grid-template-columns: repeat(${group.children.length}, minmax(0, 1fr))`}
		>
			{#each group.children as child, index (child.branchIds.join(','))}
				<div class={`min-w-0 border-t-2 ${railColor(index)}`}>
					<p class="border-b border-rule py-2 text-label uppercase text-ink-2">
						{railLabel(child.branchIds)}
					</p>
					{@render trunkList(child.group.trunk)}
				</div>
			{/each}
		</div>
		<!-- Any rail that itself forks further drops that fork out of the grid
		     entirely — rendered here, full width, one section per rail that
		     still has more to show, its own trunk already drawn above. -->
		{#each group.children as child (child.branchIds.join(',') + ':deeper')}
			{#if child.group.children.length >= 1}
				<ThreadGroup
					group={child.group}
					{branches}
					{attemptsByVersion}
					{onOpenVersion}
					{onOpenAttempt}
					hideTrunk
				/>
			{/if}
		{/each}
	{:else if group.children.length === 1}
		<ThreadGroup
			group={group.children[0].group}
			{branches}
			{attemptsByVersion}
			{onOpenVersion}
			{onOpenAttempt}
			{nested}
		/>
	{/if}
</div>
