/**
 * What a place draws while something is held over it (#204, #205), each the
 * path of a 24 by 24 line drawing. A source of recipes has its own under the
 * + (`$lib/adding/sources`).
 */
export const DRAWN = {
	/** A photograph. */
	picture:
		'M4 5h16a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1zM3 16l5-5 4 4 3-3 6 6M15.5 9.5h.01',
	/** A file the browser cannot name until it is let go. */
	unsure: 'M12 5v14M5 12h14',
	/** Something that does not go here. */
	no: 'M6 6l12 12M18 6L6 18',
};

/** What a place for one file draws: its own drawing for what it takes, and the shared ones otherwise. */
export const drawnFor = (held: 'ok' | 'unsure' | 'no', own: string): string =>
	held === 'ok' ? own : DRAWN[held];
