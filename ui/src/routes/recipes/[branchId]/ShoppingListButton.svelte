<!--
	Onto the Shopping List (#73, ADR 0024). A button and not a link: it is one
	act that finishes here, and pressing it again takes the recipe back off.
	What it stores is the choosing — the Branch, at whatever Version it is on
	when the list is next read.

	Whether the recipe is on the list is the recipe screen's to know, since the
	list's amount is what the page opens at (#109); this only changes it.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { Wanted } from '$lib/how-much';

	interface Props {
		branchId: string;
		/** What the page is scaled to, or null where it is as written. */
		scaledTo: Wanted;
		onTheList: boolean;
	}

	let { branchId, scaledTo, onTheList = $bindable() }: Props = $props();

	const kamosu = useKamosu();
	let shopping = $state(false);

	async function toggle() {
		shopping = true;
		try {
			if (onTheList) {
				await kamosu.removeFromShoppingList({ branch_id: branchId });
				onTheList = false;
			} else {
				// At the amount on screen (#109): what the errands scaler is for.
				await kamosu.addToShoppingList(
					scaledTo ? { branch_id: branchId, shopping_yield: scaledTo } : { branch_id: branchId },
				);
				onTheList = true;
			}
		} catch (error: unknown) {
			if (!(error instanceof OperationError)) throw error;
		} finally {
			shopping = false;
		}
	}
</script>

<button
	type="button"
	disabled={shopping}
	class="mx-gutter mt-2 block w-[calc(100%-2*var(--spacing-gutter))] border border-rule p-4 text-center font-display text-body {onTheList
		? 'text-ink-2'
		: 'text-accent'}"
	onclick={toggle}
>
	{onTheList ? m.shopping_on_your_list() : m.shopping_add_this()}
</button>
