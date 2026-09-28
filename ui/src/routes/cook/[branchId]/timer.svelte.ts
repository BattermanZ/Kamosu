/**
 * The timer a Step offers — one tap, nothing typed and nothing stored
 * (ADR 0011).
 *
 * It is **the cook's, not the step's**: it lives here, beside the screen, so it
 * follows them to the next step rather than being cancelled by moving on. That
 * is why one is set at all — you start the rice timer and then get on with the
 * sauce.
 *
 * When it runs out it **says so and stays** rather than disappearing, because a
 * timer that vanishes at zero is one a cook who was looking at the pan never
 * sees. It is put away by hand, which is also how a second one is started.
 *
 * Nothing here is written to the server: how far through a timer is, is not
 * part of what a cooking is (ADR 0010 holds the Step, the ticks and the Yield,
 * and deliberately nothing else). A timer is lost when the tab is closed, which
 * is the honest consequence of not storing it.
 */

export interface Timer {
	/** Seconds left, or null when nothing is running and nothing has just ended. */
	readonly remaining: number | null;
	/** True once it has reached zero and before the cook has put it away. */
	readonly rung: boolean;
	/** Start (or restart) a timer of this many seconds. */
	start(seconds: number): void;
	/** Put it away — stop a running one, or acknowledge one that has rung. */
	clear(): void;
	/** Stop the interval. The screen calls this as it goes away. */
	dispose(): void;
}

export function timer(now: () => number = Date.now): Timer {
	let endsAt = $state<number | null>(null);
	let remaining = $state<number | null>(null);
	let ticking: ReturnType<typeof setInterval> | undefined;

	function tick() {
		if (endsAt === null) return;
		const left = Math.ceil((endsAt - now()) / 1000);
		remaining = left > 0 ? left : 0;
		if (remaining === 0) stopTicking();
	}

	function stopTicking() {
		clearInterval(ticking);
		ticking = undefined;
	}

	return {
		get remaining() {
			return remaining;
		},
		get rung() {
			return remaining === 0;
		},
		start(seconds: number) {
			stopTicking();
			endsAt = now() + seconds * 1000;
			remaining = seconds;
			ticking = setInterval(tick, 250);
		},
		clear() {
			stopTicking();
			endsAt = null;
			remaining = null;
		},
		dispose() {
			stopTicking();
		},
	};
}

/**
 * A running timer's clock — `4:07`, `1:02:30`. Digits rather than words on
 * purpose: it is read at a glance from across a kitchen, and the colon form is
 * the one every oven and phone already uses, in every Language.
 */
export function clock(seconds: number): string {
	const safe = Math.max(0, Math.floor(seconds));
	const hours = Math.floor(safe / 3600);
	const minutes = Math.floor((safe % 3600) / 60);
	const rest = safe % 60;
	const pad = (n: number) => String(n).padStart(2, '0');
	return hours > 0 ? `${hours}:${pad(minutes)}:${pad(rest)}` : `${minutes}:${pad(rest)}`;
}
