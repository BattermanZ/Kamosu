/**
 * Files and links dropped onto Kamosu (ADR 0044, #204).
 *
 * A drop is taken only where it has one meaning: a PDF, a Bundle or a web
 * link onto Recipes, and whatever #205 adds. Two things live here, and every
 * place that takes a drop is built from them:
 *
 * - `DropZone` follows a drag over one place, says what it carries while it
 *   is there, and hands over what was let go.
 * - `keepDropsOut` is the rest of the app. A browser shown a dropped file
 *   opens it in place of the page, and a dropped link is followed, so
 *   anywhere no zone took the drop it is turned away and Kamosu stays.
 *
 * **Before it is let go a browser says only what kind of thing is carried**:
 * that there are files and each one's type, or that there is a link. The
 * file's name and the link's address arrive with the drop. A type can be
 * empty, where the computer knows no type for the file, so a zone decides
 * again once it holds the file.
 *
 * A drag that began inside Kamosu is none of this, and is left alone: a
 * recipe's card pulled across the shelf carries a link too.
 */

import { typedInto } from './page';

/** What a drag says it carries before it is let go. */
export interface Carried {
	/** The type of each file, in the order carried. Empty where the browser knows none. */
	files: string[];
	/** A link, and no file. */
	link: boolean;
}

/** What was let go: files, or the address of a link, or a link that carried no address. */
export type Dropped = { files: File[] } | { link: string | null };

/**
 * A drag that began on this page is under way. Followed from the moment the
 * module loads, since a drag can begin before any zone is drawn.
 *
 * `dragend` goes to what was picked up, and never arrives if that has since
 * been redrawn away. So a press of the pointer ends it too: nothing dragged
 * from outside presses on the page, and a new drag from inside says so again.
 */
let startedHere = false;
if (typeof window !== 'undefined') {
	const over = () => (startedHere = false);
	addEventListener('dragstart', () => (startedHere = true), true);
	addEventListener('dragend', over, true);
	addEventListener('pointerdown', over, true);
}

/** What Firefox calls a file from the computer until it is let go: no type at all. */
const UNTYPED = 'application/x-moz-file';

/**
 * How long a drag may send nothing before it is taken to have gone. A browser
 * says a drag is still over the page several times a second, and says it has
 * left by naming the element it left. One redrawn away under the pointer
 * never says so, and the words would stay up over the page.
 */
const GONE_AFTER = 1000;

/**
 * What this drag carries, or nothing where it is no file and no link: words
 * selected on another page, say. A file wins over a link, since a picture
 * pulled from a web page carries both and is a picture.
 */
export function carriedBy(data: DataTransfer | null): Carried | null {
	if (startedHere || !data) return null;
	const files = [...(data.items ?? [])].filter((item) => item.kind === 'file');
	if (files.length > 0)
		return { files: files.map((item) => (item.type === UNTYPED ? '' : item.type)), link: false };
	const types = [...data.types];
	// A browser that lists no items still says files are carried.
	if (types.includes('Files')) return { files: [''], link: false };
	if (types.includes('text/uri-list')) return { files: [], link: true };
	return null;
}

function droppedFrom(data: DataTransfer): Dropped {
	const files = [...data.files];
	if (files.length > 0) return { files };
	// A list of addresses, one to a line, with `#` starting a comment.
	const link = data
		.getData('text/uri-list')
		.split(/\r?\n/)
		.find((line) => line !== '' && !line.startsWith('#'));
	return { link: link ?? null };
}

/**
 * One place that takes a drop. Made inside a component, which says where it
 * is with `listen`: the window for a whole screen, or one element.
 */
export class DropZone {
	#over = $state.raw<Carried | null>(null);
	/** How many elements inside the place the drag is over: leaving one for another is not leaving. */
	#depth = 0;
	#ondrop: (dropped: Dropped) => void;
	#takes: () => boolean;

	/**
	 * @param ondrop what to do with what was let go here
	 * @param takes whether the place is taking drops just now. While it is
	 * not, a drag over it is turned away like one anywhere else.
	 */
	constructor(ondrop: (dropped: Dropped) => void, takes: () => boolean = () => true) {
		this.#ondrop = ondrop;
		this.#takes = takes;
	}

	/** What is being held over this place, or nothing. */
	get over(): Carried | null {
		return this.#takes() ? this.#over : null;
	}

	/**
	 * Follow drags over `target` until what this returns is called.
	 *
	 * Heard on the way down to what is under the pointer, so a zone has always
	 * answered before `keepDropsOut` asks whether anything did.
	 */
	listen(target: EventTarget): () => void {
		let gone: ReturnType<typeof setTimeout> | undefined;
		const forget = () => {
			clearTimeout(gone);
			this.#depth = 0;
			this.#over = null;
		};
		const stillHere = () => {
			clearTimeout(gone);
			gone = setTimeout(forget, GONE_AFTER);
		};
		const enter = (event: Event) => {
			const carried = carriedBy((event as DragEvent).dataTransfer);
			if (!carried) return;
			this.#depth += 1;
			this.#over = carried;
			stillHere();
		};
		const leave = () => {
			if (this.#depth > 0) this.#depth -= 1;
			if (this.#depth === 0) forget();
		};
		const hold = (event: Event) => {
			if (!this.over) return;
			stillHere();
			// Without this a browser never sends the drop. Which effect the
			// pointer shows is left to the browser, which knows what the place
			// the drag came from allows.
			event.preventDefault();
		};
		const drop = (event: Event) => {
			const data = (event as DragEvent).dataTransfer;
			const taken = this.over !== null;
			forget();
			if (!taken || !data) return;
			event.preventDefault();
			this.#ondrop(droppedFrom(data));
		};
		const heard = { dragenter: enter, dragleave: leave, dragover: hold, drop };
		for (const [type, hear] of Object.entries(heard)) target.addEventListener(type, hear, true);
		return () => {
			for (const [type, hear] of Object.entries(heard))
				target.removeEventListener(type, hear, true);
			forget();
		};
	}
}

/**
 * Turn away every drop no zone took, for as long as the app runs. Returns
 * what stops it.
 *
 * A link held over a field is left to the browser, which types its address
 * there: that is a person filling a field in, and the page goes nowhere.
 */
export function keepDropsOut(): () => void {
	const turnAway = (event: DragEvent) => {
		if (event.defaultPrevented) return;
		const carried = carriedBy(event.dataTransfer);
		if (!carried) return;
		if (carried.link && typedInto(event.target)) return;
		event.preventDefault();
		if (event.type === 'dragover' && event.dataTransfer) event.dataTransfer.dropEffect = 'none';
	};
	addEventListener('dragover', turnAway);
	addEventListener('drop', turnAway);
	return () => {
		removeEventListener('dragover', turnAway);
		removeEventListener('drop', turnAway);
	};
}
