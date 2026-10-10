/**
 * What the marks say on an older Version's page (#211).
 *
 * The rows are a Divergence's, read from the `theirs` side: `mine` is the
 * recipe as it stands and `theirs` is the Version on the page. So a row with
 * no line of its own is a Ghost of something written since, and a row the
 * recipe no longer has is a real line of this page.
 *
 * Nothing here names a Kitchen. Both recipes are one Branch, at two moments.
 */
import { m } from '$lib/paraglide/messages';
import type { DivergenceOutput } from '$lib/api/catalogue';
import { proseOf, type Taken, type Words } from '../divergence';

/**
 * `writes` is whether the reader may save onto this Branch. Only then is a
 * line offered back to the recipe, the choice recorded on #211: the save it
 * leads to is an ordinary Version on the Branch, never a Copy.
 */
export function sinceThen(writes: boolean): Words {
	return {
		caption(seen, _side, taken) {
			if (taken === 'remove') return m.older_taken_remove();
			if (taken) return m.older_taken();
			if (seen.ghost) return m.older_added();
			return seen.other ? m.older_changed() : m.older_taken_out();
		},
		heading: (seen) =>
			seen.ghost || !seen.other ? m.divergence_what_happened() : m.older_now_line(),
		sentence(seen) {
			if (seen.ghost) return m.older_said_added();
			return seen.other ? seen.other.text : m.older_said_taken_out();
		},
		offer(row, taken) {
			if (!writes) return null;
			if (taken) return 'undo';
			if (row.state === 'same') return null;
			// A line written since can always be taken out again: unlike a line
			// of another Kitchen's, it is the reader's own recipe that has it.
			return row.state === 'only-mine' ? 'remove' : 'write';
		},
		offerLabel(row, what) {
			if (what === 'undo') return m.older_untake();
			if (what === 'remove') return m.older_take_remove();
			return row.state === 'only-theirs' ? m.older_take_back() : m.older_take_write();
		},
		nothingToCarry: null,
		unsaved: (count) => (count === 1 ? m.older_unsaved_one() : m.older_unsaved({ count })),
		saveHint: m.older_save_hint(),
	};
}

/** The *what changed* line a save from this page is prefilled with. */
export function proseSince(
	when: string,
	divergence: DivergenceOutput,
	taken: Map<string, Taken>,
): string {
	return proseOf(divergence, taken, {
		took: (what) => m.older_prose_back({ what, when }),
		tookOut: (what) => m.older_prose_out({ what, when }),
	});
}
