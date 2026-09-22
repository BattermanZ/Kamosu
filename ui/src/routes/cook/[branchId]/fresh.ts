/**
 * Whether a cooking is one nothing has happened to yet — where the cooking
 * screen asks how much is being made before it starts (#109).
 *
 * Every trace a cooking can leave counts against it: a step moved, a line
 * ticked, an amount already said, words written, a photograph taken. Opening
 * the same fresh cooking on a second device asks there too, which is right:
 * nobody has answered yet.
 */
import type { StartAttemptOutput } from '$lib/api/catalogue';

export function fresh(cooking: StartAttemptOutput): boolean {
	return (
		cooking.current_step_index === 0 &&
		cooking.ticked_ingredients.length === 0 &&
		cooking.cooking_yield === null &&
		cooking.as_cooked === null &&
		cooking.photographs.length === 0
	);
}
