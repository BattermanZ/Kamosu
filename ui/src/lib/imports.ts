/**
 * What an Import is called and what one arrival says about itself (#108).
 *
 * Shared by the two screens rather than written twice, because the same row
 * is drawn on the source list's card and on the source's own screen, and a
 * count worded two different ways would read as two different facts.
 *
 * **A source kind is not a closed set.** Three importers name their own —
 * `crouton`, `bundle`, `web` — but the general `import` Operation takes
 * whatever an agent passes, and its ledger is as real as theirs. Paraglide
 * compiles every phrase to a function, so an unknown kind has no phrase to
 * name it: it falls back to a frame that holds the raw word. That is honest
 * where inventing a friendly name would not be.
 */

import { m } from '$lib/paraglide/messages';
import type { ListImportsOutput } from '$lib/api/catalogue';

export type Import = ListImportsOutput['imports'][number];
export type Arrival = Import['arrivals'][number];

/** The source kinds Kamosu's own importers write, each with its own wording. */
const KNOWN = ['crouton', 'bundle', 'web'] as const;
type Known = (typeof KNOWN)[number];

export const isKnown = (kind: string): kind is Known => (KNOWN as readonly string[]).includes(kind);

const NAMES: Record<Known, () => string> = {
	crouton: () => m.imports_source_crouton(),
	bundle: () => m.imports_source_bundle(),
	web: () => m.imports_source_web(),
};

export const sourceName = (kind: string): string =>
	isKnown(kind) ? NAMES[kind]() : m.imports_source_other({ kind });

const ASKS: Record<Known, () => string> = {
	crouton: () => m.imports_forget_ask_crouton(),
	bundle: () => m.imports_forget_ask_bundle(),
	web: () => m.imports_forget_ask_web(),
};

/** The confirmation's question, which names the source so it cannot be misread. */
export const forgetAsk = (kind: string): string =>
	isKnown(kind) ? ASKS[kind]() : m.imports_forget_ask_other();

const COSTS: Record<Known, () => string> = {
	crouton: () => m.imports_forget_cost_crouton(),
	bundle: () => m.imports_forget_cost_bundle(),
	web: () => m.imports_forget_cost_web(),
};

/**
 * What forgetting costs, named as the act that will pay it — bringing the same
 * library, file or page in again. The cost is invisible until that moment,
 * which is the whole reason the sentence has to say when it lands.
 *
 * Built from two phrases, because one source imported one recipe is an
 * ordinary case and "all 1 arrive a second time, as 1 new recipes" is what a
 * single phrase carrying the count produced. The act is named per source; how
 * much comes back is named per number.
 */
export const forgetCost = (kind: string, count: number): string => {
	const act = isKnown(kind) ? COSTS[kind]() : m.imports_forget_cost_other();
	const again =
		count === 1 ? m.imports_forget_cost_again_one() : m.imports_forget_cost_again_many({ count });
	return `${act} ${again}`;
};

/** How many recipes this source's ledger can still match on a re-run. */
const remembered = (count: number): string =>
	count === 1 ? m.imports_remembered_one() : m.imports_remembered_many({ count });

const arrivalCount = (count: number): string =>
	count === 1 ? m.imports_arrivals_one() : m.imports_arrivals_many({ count });

/**
 * The line under a source's name, on its card and again on its own screen —
 * one function so the two cannot word the same fact differently.
 *
 * **A ledger of nothing is left unsaid rather than counted at zero.** Two
 * sources reach that state and neither is an error. A Bundle keeps no
 * foreign-id ledger at all, because a Bundle carries the sender's own
 * travelling Branch id and is matched on that instead (ADR 0020) — so recipe
 * files have arrivals and have never remembered anything. And a ledger that
 * has been forgotten is empty by choice. "Nothing remembered" beside forty
 * arrivals reads as damage in both cases, where the arrivals alone read as
 * what they are.
 */
export function sourceSummary(source: Import): string {
	const parts: string[] = [];
	if (source.remembered > 0) parts.push(remembered(source.remembered));
	parts.push(arrivalCount(source.arrivals.length));
	return parts.join(' · ');
}

/**
 * Whether this source has a ledger worth throwing away. An Import row with an
 * empty ledger — a Bundle's, always — has nothing for `forget_import` to take,
 * so offering it would be a frightening button that changes nothing.
 */
export const canForget = (source: Import): boolean =>
	source.import_id !== null && source.remembered > 0;

/**
 * One arrival in a line, as the parts it is made of.
 *
 * The parts are returned rather than a joined string so the screen can mark
 * the loss differently from the rest — a file that could not be read is the
 * one thing on this row worth a second look.
 */
export function whatHappened(arrival: Arrival): { said: string; loss: boolean } {
	if (arrival.status === 'queued' || arrival.status === 'running') {
		return { said: m.imports_row_running(), loss: false };
	}
	if (arrival.status !== 'completed') {
		return { said: m.imports_row_stopped(), loss: true };
	}

	const parts: string[] = [];
	if (arrival.arrived === 0) parts.push(m.imports_row_nothing());
	else if (arrival.created === arrival.arrived)
		parts.push(
			arrival.arrived === 1
				? m.imports_row_new_one()
				: m.imports_row_new_many({ count: arrival.arrived }),
		);
	else if (arrival.created === 0)
		parts.push(
			arrival.arrived === 1
				? m.imports_row_held_one()
				: m.imports_row_held_many({ count: arrival.arrived }),
		);
	else parts.push(m.imports_row_some_new({ count: arrival.arrived, created: arrival.created }));

	if (arrival.unreadable)
		parts.push(
			arrival.unreadable === 1
				? m.imports_row_unreadable_one()
				: m.imports_row_unreadable_many({ count: arrival.unreadable }),
		);
	if (arrival.offered)
		parts.push(
			arrival.offered === 1
				? m.imports_row_offered_one()
				: m.imports_row_offered_many({ count: arrival.offered }),
		);

	// A run that brought nothing at all is the one an eye should catch.
	return { said: parts.join(' · '), loss: arrival.arrived === 0 };
}
