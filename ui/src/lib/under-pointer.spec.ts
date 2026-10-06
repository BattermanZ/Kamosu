/**
 * What a control looks like under the pointer is written once, at the foot of
 * `app.css` (#203, ADR 0044). jsdom has no pointer and applies no media
 * query, so what these guard is the one-place rule itself: no screen writes a
 * hover state of its own, and no clickable thing carries a fill that the
 * block has no line for, which would do nothing under the pointer.
 */

import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';

const sources = import.meta.glob<string>('/src/**/*.svelte', {
	query: '?raw',
	import: 'default',
	eager: true,
});

/**
 * The rules for the pointer, from their heading to the end of the file. Read
 * off the disk: asked for as a module, a stylesheet arrives here empty.
 */
const stylesheet = readFileSync('src/app.css', 'utf8');
const underPointer = stylesheet.slice(stylesheet.indexOf('Under the pointer (ADR 0044, #203)'));

/** An opening tag of something clickable, with every attribute it carries. */
const CLICKABLE =
	/<(?:a|button|label|select|summary)\b(?:[^>"'{]|"[^"]*"|'[^']*'|\{(?:[^{}]|\{[^{}]*\})*\})*>/g;

describe('under the pointer', () => {
	it('is written in the stylesheet, where the block is found', () => {
		expect(underPointer).toContain('@media (hover: hover)');
	});

	it('is written on no screen: none carries a hover: class', () => {
		const own = Object.entries(sources)
			.filter(([, source]) => /(?<![\w-])hover:[\w[]/.test(source))
			.map(([path]) => path);
		expect(own).toEqual([]);
	});

	it('has a line for every fill a clickable thing is written with', () => {
		const fills = new Set<string>();
		for (const source of Object.values(sources)) {
			for (const [tag] of source.matchAll(CLICKABLE)) {
				for (const [, fill] of tag.matchAll(/(?<![\w:-])(bg-[a-z0-9-]+)(?![\w/-])/g))
					fills.add(fill);
			}
		}
		// Seen at all, or the pattern above has stopped finding anything.
		expect(fills).toContain('bg-accent');

		const unnamed = [...fills].filter((fill) => !underPointer.includes(`.${fill}`)).sort();
		// The sidebar's current place, and a control lying over a photograph:
		// both stay as they are on purpose.
		expect(unnamed).toEqual(['bg-cook-ground', 'bg-transparent']);
	});
});
