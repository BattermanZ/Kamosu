/**
 * The Divergence's marks on the recipe screen (ADR 0014, ADR 0019): which side
 * you are standing in, whether the marks are shown, which marked rows are
 * open, and what has been carried across from the other version and not yet
 * saved.
 *
 * Carried across and not yet saved is all it is. It becomes real only when an
 * ordinary Version is saved — there is no other kind of save here.
 *
 * Nothing here pairs anything: which line is which is read against the Branch
 * Point by the `divergence` Operation, which the recipe screen reads and
 * hands in.
 */
import { m } from '$lib/paraglide/messages';
import { OperationError } from '$lib/api/client';
import type { DivergenceOutput, KamosuClient } from '$lib/api/catalogue';
import { timeText } from '$lib/duration';
import { draftVersion, fieldText, prose, type Side, type Taken } from './divergence';

type Field = keyof DivergenceOutput['fields'];

export interface Marking {
	/**
	 * Which side of the Divergence the page draws. `mine` is always the
	 * reader's own recipe (#131), so on anybody else's version the page is
	 * `theirs`: the rows are the same rows, read from the other side, and the
	 * page is still the recipe in the URL.
	 */
	side: Side;
	/** Whether the divergence is marked at all. Off is simply the recipe. */
	readonly marks: boolean;
	/** Marked rows are only ever drawn when there IS a divergence and it is shown. */
	readonly showing: boolean;
	/** How many rows the two versions do not share. */
	readonly unshared: number;
	readonly taken: ReadonlyMap<string, Taken>;
	/** The sheet saving what was carried is open. */
	saving: boolean;
	/** What that save says changed, prefilled from what was carried. */
	changeNote: string;
	readonly saved: 'no' | 'yes' | 'failed';
	/** What the other side has for a single value, when the two do not agree. */
	markOf(name: Field): string | null;
	isOpen(key: string): boolean;
	toggle(key: string): void;
	toggleMarks(): void;
	carry(key: string, what: Taken | 'undo'): void;
	/** Put back everything carried. */
	undoAll(): void;
	startSaving(): void;
	save(): Promise<void>;
	/** The page moved to another recipe: nothing marked on the last one follows. */
	reset(): void;
}

export function marking(page: {
	kamosu: KamosuClient;
	divergence: () => DivergenceOutput | undefined;
	/**
	 * Whose the other version is, named plainly for the marks' own sentences.
	 * Left out where the other recipe is nobody else's (`older`).
	 */
	otherKitchen?: () => string;
	/** The marks were put away or brought back. */
	onToggled: () => void;
	/** What was carried was saved as a Version. */
	onSaved: () => void;
	/**
	 * Given where the other recipe is an older Version of this same Branch
	 * (#211), and left out for two Branches. It words the two things said
	 * here: a single value the recipe now holds differently, and the *what
	 * changed* line of the save. And since both recipes are then one Branch,
	 * a Step reworded from the older one keeps its Photograph (`draftList`).
	 */
	older?: {
		field(name: Field, value: string): string;
		prose(divergence: DivergenceOutput, taken: Map<string, Taken>): string;
	};
}): Marking {
	let side = $state<Side>('mine');
	let marks = $state(true);
	let open = $state(new Set<string>());
	let taken = $state(new Map<string, Taken>());
	let saving = $state(false);
	let changeNote = $state('');
	let saved = $state<'no' | 'yes' | 'failed'>('no');

	const shown = $derived(Boolean(page.divergence()) && marks);
	const unshared = $derived.by(() => {
		const divergence = page.divergence();
		return divergence
			? [...divergence.ingredients, ...divergence.steps].filter((row) => row.state !== 'same')
					.length
			: 0;
	});

	/**
	 * A marked value in words. The two times are minutes, so they read with
	 * their unit, `1 h 30`, as they do in the strip (#175); everything else is
	 * `fieldText`'s.
	 */
	function markText(name: Field, value: unknown): string {
		const time = name === 'prep_time_minutes' || name === 'cook_time_minutes';
		return time && typeof value === 'number' ? timeText(value) : fieldText(value);
	}

	return {
		get side() {
			return side;
		},
		set side(value) {
			side = value;
		},
		get marks() {
			return marks;
		},
		get showing() {
			return shown;
		},
		get unshared() {
			return unshared;
		},
		get taken() {
			return taken;
		},
		get saving() {
			return saving;
		},
		set saving(value) {
			saving = value;
		},
		get changeNote() {
			return changeNote;
		},
		set changeNote(value) {
			changeNote = value;
		},
		get saved() {
			return saved;
		},
		/**
		 * The marking covers the whole recipe, not only the two lists (ADR
		 * 0019): a Title renamed or a Yield halved is a difference a cook needs
		 * to see.
		 */
		markOf(name) {
			const divergence = page.divergence();
			if (!shown || !divergence) return null;
			const field = divergence.fields[name];
			if (field.same) return null;
			// A Note is a block of prose; "{kitchen} has {value}" reads as nonsense
			// against one, so it is introduced as the note it is.
			if (page.older) return page.older.field(name, markText(name, field.mine));
			if (side === 'theirs') {
				// Their value is the page, so the mark says what YOURS has (#131):
				// naming them beside your value would say something false about
				// their recipe.
				const value = markText(name, field.mine);
				return name === 'note'
					? m.divergence_field_note_yours({ value })
					: m.divergence_field_yours({ value });
			}
			const value = markText(name, field.theirs);
			const kitchen = page.otherKitchen?.() ?? '';
			return name === 'note'
				? m.divergence_field_note({ kitchen, value })
				: m.divergence_field_differs({ kitchen, value });
		},
		isOpen(key) {
			return open.has(key);
		},
		toggle(key) {
			const next = new Set(open);
			if (next.has(key)) next.delete(key);
			else next.add(key);
			open = next;
		},
		// Putting the marking away rebuilds the list from a different set of
		// rows — so an open panel would reopen on whatever line happens to land
		// at that index. It closes everything first.
		toggleMarks() {
			marks = !marks;
			open = new Set();
			page.onToggled();
		},
		carry(key, what) {
			const next = new Map(taken);
			if (what === 'undo') next.delete(key);
			else next.set(key, what);
			taken = next;
			saved = 'no';
		},
		undoAll() {
			taken = new Map();
		},
		startSaving() {
			const divergence = page.divergence();
			if (!divergence) return;
			changeNote = (page.older?.prose ?? prose)(divergence, taken);
			saving = true;
		},
		async save() {
			const divergence = page.divergence();
			if (!divergence) return;
			try {
				await page.kamosu.saveRecipeVersion(
					draftVersion(divergence, taken, changeNote, page.older !== undefined),
				);
				taken = new Map();
				saving = false;
				saved = 'yes';
				page.onSaved();
			} catch (error) {
				if (!(error instanceof OperationError)) throw error;
				saved = 'failed';
			}
		},
		reset() {
			side = 'mine';
			marks = true;
			open = new Set();
			taken = new Map();
			changeNote = '';
			saved = 'no';
		},
	};
}
