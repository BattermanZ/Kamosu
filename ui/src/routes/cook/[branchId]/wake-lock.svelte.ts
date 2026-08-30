/**
 * The screen staying awake while you cook — the **real** Wake Lock API and
 * nothing else (#61).
 *
 * No fallback hack and no silent looping video. A browser that does not have
 * the API simply does not hold the screen awake, and the screen says so, which
 * is a true statement a cook can act on; a hidden video element that keeps the
 * backlight on is a lie about what the app is doing and drains a battery
 * nobody agreed to spend.
 *
 * A lock is released by the browser itself whenever the page is hidden — a
 * phone locked in a pocket, a tab switched away from — and is never given back
 * on its own. So it is **re-acquired on the way back**, which is the whole
 * reason this is a small state machine rather than one call.
 *
 * `wanted` is the cook's own switch and outlives any interruption: turning it
 * off must stay off across a tab switch, and turning it on must survive one.
 */

/** The slice of the Wake Lock API this uses, typed here because `lib.dom` in
 *  this TypeScript version does not carry it. */
interface Sentinel {
	released: boolean;
	release(): Promise<void>;
	/** A real sentinel is an EventTarget and fires `release`. Optional because a
	 *  browser that has the API but not the event must not crash the screen. */
	addEventListener?(type: 'release', listener: () => void): void;
}
interface WakeLockCapable {
	wakeLock?: { request(type: 'screen'): Promise<Sentinel> };
}

export interface WakeLock {
	/** Whether the browser has the API at all — what the off switch is drawn from. */
	readonly available: boolean;
	/** Whether the cook wants the screen kept awake. Their switch, not the browser's. */
	readonly wanted: boolean;
	/** Whether a lock is actually held right now. */
	readonly held: boolean;
	toggle(): void;
	/** Start holding it, and keep re-acquiring it after interruptions. */
	start(): void;
	/** Give it back and stop listening — the cook has left the stove. */
	stop(): void;
}

export function wakeLock(): WakeLock {
	const api = (navigator as Navigator & WakeLockCapable).wakeLock;
	let wanted = $state(true);
	let held = $state(false);
	let sentinel: Sentinel | null = null;
	let listening = false;

	/** Whether what we are holding is still a live lock. A sentinel the browser
	 *  has released is not one, and must not stand in the way of taking another. */
	const holding = () => Boolean(sentinel) && !sentinel?.released;

	async function acquire() {
		if (!api || !wanted || holding()) return;
		sentinel = null;
		try {
			const taken = await api.request('screen');
			// The cook may have turned it off while the request was in flight.
			// Honour the switch as it stands now, not as it stood when we asked.
			if (!wanted) {
				await taken.release();
				return;
			}
			sentinel = taken;
			held = true;
			// The browser releases the lock by itself whenever the page is
			// hidden, and says so here. Without this the sentinel would look live
			// forever after the first interruption.
			taken.addEventListener?.('release', () => {
				if (sentinel === taken) {
					sentinel = null;
					held = false;
				}
			});
		} catch {
			// Refused — the tab is hidden, or the browser said no. Not an error a
			// cook can do anything about, and the switch beneath the step says
			// the screen may sleep, which is true.
			sentinel = null;
			held = false;
		}
	}

	async function release() {
		const current = sentinel;
		sentinel = null;
		held = false;
		if (current && !current.released) await current.release();
	}

	// The browser drops the lock whenever the page is hidden and hands nothing
	// back on return, so coming back into view is where it is taken again. The
	// sentinel is dropped on the way out rather than kept: holding a released
	// one is what would make `acquire` decide there was nothing to do.
	function onVisibility() {
		if (document.visibilityState === 'visible') {
			void acquire();
		} else {
			sentinel = null;
			held = false;
		}
	}

	return {
		get available() {
			return Boolean(api);
		},
		get wanted() {
			return wanted;
		},
		get held() {
			return held;
		},
		toggle() {
			wanted = !wanted;
			if (wanted) void acquire();
			else void release();
		},
		start() {
			if (!listening) {
				document.addEventListener('visibilitychange', onVisibility);
				listening = true;
			}
			void acquire();
		},
		stop() {
			if (listening) {
				document.removeEventListener('visibilitychange', onVisibility);
				listening = false;
			}
			void release();
		},
	};
}
