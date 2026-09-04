/*
 * PROTOTYPE — #58, "As Cooked and Promotion". THROWAWAY. Delete on merge.
 *
 * The As Cooked has no backend yet, and the point of this prototype is what
 * the screens look like, not whether the server can hold one. So the whole
 * deviation lives here in memory, mirrored into sessionStorage only because
 * the two moments the ticket asks to see sit on different routes: you deviate
 * on /cook/<branch>, and you are offered Promotion on /recipes/<branch> or
 * /cooked. Reloading the tab keeps it; closing it throws it away, which is
 * exactly what a prototype's state should do.
 */

/** One line of the recipe a cook rewrote: what it said, and what it says now. */
export interface Rewrite {
	/** 'ingredient' or 'step' — which of the Version's two lists this indexes. */
	list: 'ingredient' | 'step';
	/** The index into that list. The same index the Core stores. */
	at: number;
	/** The written line as the recipe has it. */
	was: string;
	/** The written line as this cook wrote it. */
	now: string;
}

interface Held {
	branchId: string;
	title: string;
	when: string;
	rewrites: Rewrite[];
}

const KEY = 'prototype-58-as-cooked';

function load(): Held | null {
	if (typeof sessionStorage === 'undefined') return null;
	const raw = sessionStorage.getItem(KEY);
	if (!raw) return null;
	try {
		return JSON.parse(raw) as Held;
	} catch {
		return null;
	}
}

let held = $state<Held | null>(load());

/**
 * Pretending the network is gone, so the offline refusal can be looked at.
 * Kept in sessionStorage like the As Cooked, so a full page load between the
 * two moments does not quietly put the network back.
 */
let offline = $state(
	typeof sessionStorage !== 'undefined' && sessionStorage.getItem(KEY + '-offline') === 'yes',
);

export const asCooked = {
	get held() {
		return held;
	},
	get offline() {
		return offline;
	},
	set offline(value: boolean) {
		offline = value;
		if (typeof sessionStorage !== 'undefined') {
			sessionStorage.setItem(KEY + '-offline', value ? 'yes' : 'no');
		}
	},

	/** Every rewrite made in this cooking, in the recipe's own order. */
	rewrites(branchId: string): Rewrite[] {
		return held?.branchId === branchId ? held.rewrites : [];
	},

	/** What this cook wrote for one line, or null where they left it alone. */
	rewriteOf(branchId: string, list: Rewrite['list'], at: number): Rewrite | null {
		return this.rewrites(branchId).find((one) => one.list === list && one.at === at) ?? null;
	},

	/**
	 * Write a line. Writing back exactly what the recipe says takes the rewrite
	 * off again — cooked as written stores nothing, at the level of one line as
	 * much as of a whole cooking.
	 */
	write(
		branchId: string,
		title: string,
		list: Rewrite['list'],
		at: number,
		was: string,
		now: string,
	) {
		const kept = (held?.branchId === branchId ? held.rewrites : []).filter(
			(one) => !(one.list === list && one.at === at),
		);
		if (now.trim() !== was.trim() && now.trim() !== '') {
			kept.push({ list, at, was, now: now.trim() });
		}
		kept.sort((a, b) => (a.list === b.list ? a.at - b.at : a.list === 'ingredient' ? -1 : 1));
		held =
			kept.length === 0
				? null
				: { branchId, title, when: held?.when ?? new Date().toISOString(), rewrites: kept };
		if (typeof sessionStorage !== 'undefined') {
			if (held) sessionStorage.setItem(KEY, JSON.stringify(held));
			else sessionStorage.removeItem(KEY);
		}
	},

	/** Throw the whole thing away — what promoting or dismissing it does here. */
	clear() {
		held = null;
		if (typeof sessionStorage !== 'undefined') sessionStorage.removeItem(KEY);
	},
};
