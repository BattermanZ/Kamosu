/**
 * When Kamosu itself goes wrong (#98, option C — Aurélien, 22 September 2026).
 *
 * Every screen draws one line: a **refusal** the Core meant to send is an
 * `OperationError`, caught and said on screen (ADR 0040); anything else is a
 * **mistake** in Kamosu, re-thrown rather than dressed up as a polite refusal.
 * The line was always right. What was missing is this — something underneath to
 * catch what is thrown across it, so a mistake reaches the cook instead of a
 * console nobody has open.
 *
 * Three layers reach here, because no one of them sees what the others do:
 *
 * 1. `createClient` catches a mistake escaping any Operation call. This is the
 *    screen seam, so a test can raise a real `Error` from behind a stand-in and
 *    assert what the cook is shown.
 * 2. The root layout's `<svelte:boundary>` catches a mistake made while
 *    rendering or in an effect, which no promise ever sees.
 * 3. `watchForMistakes` nets the rest: a mistake in a screen's own code after
 *    the await, which rejects into nowhere because Svelte awaits neither an
 *    event handler's promise nor a fire-and-forget `$effect`.
 *
 * **Layer 3 cannot be tested here, because jsdom never fires
 * `unhandledrejection`** — probed directly rather than assumed. It is verified
 * in a real browser instead, and that one fact is why the other two layers
 * exist rather than one net over everything.
 *
 * A mistake more than one layer sees is said once.
 */

// From `./api/refusal` and not `./api/client`, which imports this module: the
// class lives on its own so these two do not form a circle.
import { OperationError } from './api/refusal';

/** The mistake Kamosu has made and the cook has not yet put away. */
export const mistake = $state<{ made?: unknown }>({});

/**
 * Said already. Held weakly, so a mistake keeps nothing alive: the point is only
 * that the layers above do not raise the same card twice.
 */
const said = new WeakSet<object>();

/**
 * Kamosu went wrong. A refusal is not a mistake and never reaches the card —
 * the rule lives here rather than in each layer, so all three draw the same
 * line the screens do.
 *
 * Never throws. A layer may be reporting from a place where a second throw
 * would have no owner at all.
 */
export function wentWrong(thrown: unknown): void {
	if (thrown instanceof OperationError) return;
	if (typeof thrown === 'object' && thrown !== null) {
		if (said.has(thrown)) return;
		said.add(thrown);
	}

	// Still written where a developer looks. The browser prints its own report
	// with the stack alongside this, which is the one worth reading.
	console.error('Kamosu went wrong:', thrown);
	mistake.made = thrown;
}

/**
 * One ask, watched for a mistake and otherwise untouched.
 *
 * `createClient` covers every Operation, which is nearly everything a screen
 * asks Kamosu. The exceptions are the asks that cannot be Operations: the four
 * authentication routes, the two uploads that carry a file too large for an
 * envelope (ADR 0001), and the Sheet and the recipe file fetched to be shared
 * (#149, #156). They come
 * through here so that *anywhere* a screen asks Kamosu something means
 * anywhere, and not anywhere-but-those.
 */
export async function watched<T>(ask: Promise<T>): Promise<T> {
	try {
		return await ask;
	} catch (thrown) {
		wentWrong(thrown);
		throw thrown;
	}
}

/**
 * The cook has read it. Put away rather than remembered on the device, because
 * a mistake is a thing that happened and not a standing condition like a plain
 * `http://` connection: the next one says so again.
 */
export function putMistakeAway(): void {
	mistake.made = undefined;
}

/**
 * Layer 3: the net under what neither the client seam nor a boundary sees.
 * Installed once by the root layout; returns its own teardown, as an `$effect`
 * wants.
 */
export function watchForMistakes(): () => void {
	const rejected = (event: PromiseRejectionEvent) => wentWrong(event.reason);
	// Only a script's own throw, never a picture that failed to load: a resource
	// error does not bubble, so it never reaches a listener registered here.
	const threw = (event: ErrorEvent) => {
		if (event.error !== undefined && event.error !== null) wentWrong(event.error);
	};

	addEventListener('unhandledrejection', rejected);
	addEventListener('error', threw);
	return () => {
		removeEventListener('unhandledrejection', rejected);
		removeEventListener('error', threw);
	};
}
