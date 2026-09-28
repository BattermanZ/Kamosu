/**
 * How much room the window has (ADR 0044, #193).
 *
 * Kamosu on a tablet or a computer is the same app laid out again when the
 * window has room. Which layout it gets is decided by the window's size alone,
 * never by asking what device it is on:
 *
 * - `phone`: the layout Kamosu has always had. A phone either way up, a small
 *   tablet held upright, a narrow browser window.
 * - `wide`: room for a sidebar in place of the tab bar. Every tablet, either
 *   way up.
 * - `roomy`: wide, and room for two columns side by side. A tablet on its
 *   side, a computer.
 *
 * The thresholds are not written here. They live once, in `ui/src/app.css`,
 * which also sets `--room` on the page from them; the Room is that property
 * read back. So a style rule written with `wide:` and a screen asking
 * `room.wide` cannot disagree about which layout is showing, and moving a
 * figure in the stylesheet moves both.
 */

import { getContext, setContext } from 'svelte';
import { readToken } from './tokens';

export type Room = 'phone' | 'wide' | 'roomy';

/** What the stylesheet said, as a Room. Anything else is the phone's layout. */
export function roomOf(value: string): Room {
	return value === 'wide' || value === 'roomy' ? value : 'phone';
}

/** A Room, with the two questions a screen asks of it. */
export interface RoomFacts {
	readonly current: Room;
	/** The wide layout: a roomy window is wide too. */
	readonly wide: boolean;
	/** Room for two columns. */
	readonly roomy: boolean;
}

function facts(room: () => Room): RoomFacts {
	return {
		get current() {
			return room();
		},
		get wide() {
			return room() !== 'phone';
		},
		get roomy() {
			return room() === 'roomy';
		},
	};
}

/**
 * This window's Room, followed as it is resized or a tablet is turned. Made
 * once, inside the root layout, and handed to the provider from there; a
 * screen takes it with `useRoom` rather than making its own.
 *
 * Read on `resize`, which a browser sends for a turned tablet and for an iPad
 * app dragged to half the screen as well as for a window's edge. Not on the
 * visual viewport: a keyboard sliding up over a field on an iPad moves that
 * and not this, and must not re-lay the page out under the cook's fingers.
 *
 * Read once as it is made as well, so the first frame a wide window draws is
 * already wide rather than the phone's for a moment. Not while the app is
 * being prerendered at build time, where there is no window to read.
 *
 * @param read what the stylesheet says `--room` is. A test hands in its own.
 */
export class WindowRoom {
	#room: Room = $state('phone');

	constructor(read: () => string = () => readToken('--room')) {
		const look = () => {
			this.#room = roomOf(read());
		};
		if (typeof window !== 'undefined') look();
		$effect(() => {
			look();
			addEventListener('resize', look);
			return () => removeEventListener('resize', look);
		});
	}

	get current(): Room {
		return this.#room;
	}
}

const KEY = Symbol('room');

/** Put the Room into context. The provider does this, holding what it was given. */
export function provideRoom(room: () => Room): void {
	setContext(KEY, room);
}

/**
 * The Room this screen is drawn in. Outside a provider, and in a test that
 * hands in none, it is the phone's: the layout that holds wherever nothing
 * says there is more room.
 */
export function useRoom(): RoomFacts {
	const room = getContext<(() => Room) | undefined>(KEY);
	return facts(room ?? (() => 'phone'));
}
