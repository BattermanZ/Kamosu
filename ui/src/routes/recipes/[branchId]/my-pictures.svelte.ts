/**
 * Your own pictures of this dish, from every time you cooked it (#110,
 * option B) — newest cooking first, as the diary has them. Read from
 * `list_attempts`, which the Core scopes to the caller, and never from the
 * Thread's `attempts`, which carries the household's: a picture somebody else
 * took is theirs to promote, not yours. One still only on this phone waits
 * until it has been sent.
 *
 * Held beside the recipe screen rather than in the row that shows them, so
 * that writing on the recipe and coming back does not read the diary again.
 */
import type { KamosuClient } from '$lib/api/catalogue';
import { OperationError } from '$lib/api/client';
import { cookingDay } from '$lib/cooking-day';
import { onlyOnThisPhone } from '$lib/offline/outbox';
import type { Offered } from '$lib/PhotoToRecipe.svelte';

export interface MyPictures {
	readonly list: Offered[];
}

/**
 * `dish` is the Lineage, as a string, so the read runs again when the page
 * moves to another dish and not every time this one is read again — at
 * another amount, say, or after a save.
 */
export function myPictures(kamosu: KamosuClient, dish: () => string | undefined): MyPictures {
	let list = $state<Offered[]>([]);
	const lineage = $derived(dish());

	$effect(() => {
		if (!lineage) return;
		let current = true;
		kamosu
			.listAttempts({})
			.then((diary) => {
				if (!current) return;
				list = diary.attempts
					.filter((attempt) => attempt.lineage_id === lineage)
					.flatMap((attempt) =>
						attempt.photographs
							.filter((photograph) => !onlyOnThisPhone(photograph))
							.map((photograph) => ({
								photograph,
								attempt: attempt.id,
								taken: cookingDay(attempt.created_at),
							})),
					);
			})
			.catch((error: unknown) => {
				// The row is an offer, not the recipe: a diary that cannot be
				// read leaves it out rather than failing the page.
				if (!(error instanceof OperationError)) throw error;
			});
		return () => {
			current = false;
		};
	});

	return {
		get list() {
			return list;
		},
	};
}
