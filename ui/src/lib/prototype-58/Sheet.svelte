<!--
	PROTOTYPE — #58, TREATMENT B. THROWAWAY. Delete on merge.

	The sheet a cook says "I did it differently" into. It rises from the foot and
	is deliberately NOT full height: the Step's own sentence stays visible above
	its top edge, which is what makes this "without leaving the step you are
	standing on" rather than a detour into a form.

	It holds two things, in this order:

	  · THIS STEP — its sentence, and the amounts it uses, each a field;
	  · EARLIER IN THIS COOKING — every other line this cook has already
	    rewritten, wherever they were standing at the time, each reopenable.

	The second half is the whole argument for a sheet over writing in place: the
	cooking's drift is one thing a cook can see and correct from anywhere,
	instead of being scattered across sixteen steps they would have to walk back
	through.
-->
<script lang="ts">
	import { asCooked, type Rewrite } from './deviation.svelte';

	interface Amount {
		at: number;
		was: string;
		text: string;
		changed: boolean;
	}

	interface Props {
		branchId: string;
		title: string;
		stepAt: number;
		stepWas: string;
		amounts: Amount[];
		/** Every Ingredient Line this step does not itself name. */
		otherLines: Amount[];
		rewrites: Rewrite[];
		close: () => void;
	}

	let { branchId, title, stepAt, stepWas, amounts, otherLines, rewrites, close }: Props = $props();

	/** The whole list, open. See the same control in treatment A for why. */
	let wholeList = $state(false);

	const stepNow = $derived(asCooked.rewriteOf(branchId, 'step', stepAt)?.now ?? stepWas);

	/** What was changed anywhere BUT the lines this step already shows above. */
	const elsewhere = $derived(
		rewrites.filter(
			(one) =>
				!(one.list === 'step' && one.at === stepAt) &&
				!(one.list === 'ingredient' && amounts.some((amount) => amount.at === one.at)),
		),
	);

	function write(list: 'ingredient' | 'step', at: number, was: string, now: string) {
		asCooked.write(branchId, title, list, at, was, now);
	}
</script>

<div
	class="fixed inset-x-0 bottom-0 z-40 mx-auto flex max-w-2xl flex-col bg-cook-panel px-gutter pb-safe"
	style="max-height:66vh;border-top:2px solid var(--color-cook-accent)"
	role="dialog"
	aria-modal="true"
	aria-label="I did it differently"
>
	<div class="flex shrink-0 items-baseline justify-between gap-3 py-3">
		<p class="text-label font-semibold text-cook-accent uppercase">I did it differently</p>
		<button type="button" class="min-h-12 text-label text-cook-ink uppercase" onclick={close}>
			Done
		</button>
	</div>

	<div class="min-h-0 flex-1 overflow-y-auto pb-4">
		<p class="pb-2 text-label text-cook-ink-2 uppercase">This step</p>

		<textarea
			value={stepNow}
			rows="3"
			oninput={(event) => write('step', stepAt, stepWas, event.currentTarget.value)}
			class="w-full font-display text-body font-semibold"
			style="background:var(--color-cook-ground);color:var(--color-cook-ink);
				border:1px solid var(--color-cook-rule);border-radius:2px;padding:8px;resize:none"></textarea>
		{#if stepNow !== stepWas}
			<button
				type="button"
				class="min-h-12 text-left text-read text-cook-ink-2"
				style="text-decoration:line-through"
				onclick={() => write('step', stepAt, stepWas, stepWas)}
			>
				{stepWas}
			</button>
		{/if}

		{#each amounts as amount (amount.at)}
			<input
				value={amount.text}
				oninput={(event) => write('ingredient', amount.at, amount.was, event.currentTarget.value)}
				class="mt-2 w-full font-display text-panel-figure font-semibold"
				style="background:var(--color-cook-ground);color:var(--color-cook-ink);
					border:1px solid var(--color-cook-rule);border-radius:2px;padding:8px;min-height:48px"
			/>
		{/each}

		{#if amounts.length === 0}
			<p class="mt-2 text-read text-cook-ink-2">
				This step names no amounts — which is the ordinary case. Across the corpus a step names one
				only 42% of the time.
			</p>
		{/if}

		<!--
			The rest of the list. The sheet is where this costs nothing: it already
			scrolls, and it is already a place rather than the screen itself. A step
			names an amount only 42% of the time, so without this the salt a cook
			actually changed is unreachable on most steps.
		-->
		<button
			type="button"
			class="mt-3 flex min-h-12 w-full items-center justify-between text-label uppercase
				{wholeList ? 'font-semibold text-cook-accent' : 'text-cook-ink-2'}"
			style="border-top:1px solid var(--color-cook-rule);padding-top:8px"
			onclick={() => (wholeList = !wholeList)}
		>
			<span>{wholeList ? 'Just this step' : 'Any other line'}</span>
			<span>{wholeList ? '−' : `+${otherLines.length}`}</span>
		</button>

		{#if wholeList}
			{#each otherLines as line (line.at)}
				<div class="py-1">
					<input
						value={line.text}
						oninput={(event) => write('ingredient', line.at, line.was, event.currentTarget.value)}
						class="w-full font-display text-body"
						style="background:transparent;color:var(--color-cook-ink);
							border-bottom:1px solid var(--color-cook-rule);padding:6px 0;min-height:44px"
					/>
					{#if line.changed}
						<button
							type="button"
							class="min-h-12 text-read text-cook-ink-2"
							style="text-decoration:line-through"
							onclick={() => write('ingredient', line.at, line.was, line.was)}
						>
							{line.was}
						</button>
					{/if}
				</div>
			{/each}
		{/if}

		<!--
			Everything already changed, from wherever the cook was standing. This
			is the half a per-step treatment cannot offer: on step 14 of 29 you can
			see, and fix, the amount you rewrote on step 6.
		-->
		{#if elsewhere.length > 0}
			<p
				class="mt-5 pb-2 text-label text-cook-ink-2 uppercase"
				style="border-top:1px solid var(--color-cook-rule);padding-top:12px"
			>
				Earlier in this cooking
			</p>
			{#each elsewhere as one (one.list + one.at)}
				<div class="py-2" style="border-bottom:1px solid var(--color-cook-rule)">
					<input
						value={one.now}
						oninput={(event) => write(one.list, one.at, one.was, event.currentTarget.value)}
						class="w-full font-display text-body font-semibold"
						style="background:transparent;color:var(--color-cook-ink);border:0;min-height:44px"
					/>
					<p class="text-read text-cook-ink-2" style="text-decoration:line-through">{one.was}</p>
				</div>
			{/each}
		{/if}
	</div>
</div>
