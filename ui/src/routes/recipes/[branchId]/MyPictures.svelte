<!--
	Your own pictures of the dish, and the way to put one on the recipe (#110,
	option B): among how the dish has actually gone, because that is what they
	are. Nothing at all when you have none, which is nearly always; and only in
	your own Branch, for the reason the Tags row is. `my-pictures.svelte.ts`
	holds which pictures are yours.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';
	import AttemptPhoto from '$lib/offline/AttemptPhoto.svelte';
	import PhotoToRecipe, { type Offered } from '$lib/PhotoToRecipe.svelte';
	import type { ComponentProps } from 'svelte';

	interface Props {
		branchId: string;
		pictures: Offered[];
		onPromoted: ComponentProps<typeof PhotoToRecipe>['onPromoted'];
	}

	let { branchId, pictures, onPromoted }: Props = $props();

	let promoting = $state(false);
</script>

{#if pictures.length > 0}
	<div class="mt-3">
		<h3 class="mb-2 text-label text-ink-2 uppercase">{m.recipe_my_photos()}</h3>
		<ul class="flex flex-wrap gap-2">
			{#each pictures as picture (picture.attempt + picture.photograph)}
				<li>
					<AttemptPhoto id={picture.photograph} alt={m.promote_taken({ date: picture.taken })} />
				</li>
			{/each}
		</ul>
		<NeedsServer
			label={m.recipe_use_photo()}
			waiting={m.offline_waits_edit()}
			onclick={() => (promoting = true)}
			shapeClass="mt-2 min-h-12 w-full px-4 text-body"
			lookClass="border border-rule text-accent"
		/>
	</div>
{/if}

{#if promoting}
	<PhotoToRecipe
		{branchId}
		{pictures}
		onPromoted={(landed) => {
			promoting = false;
			onPromoted(landed);
		}}
		onClose={() => (promoting = false)}
	/>
{/if}
