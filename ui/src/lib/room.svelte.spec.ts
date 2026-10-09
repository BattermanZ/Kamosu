/**
 * How much room the window has (#193, ADR 0044).
 *
 * jsdom lays nothing out and answers no media query, so the thresholds are
 * checked on the stylesheet itself: `ui/src/app.css` compiled by Tailwind as
 * `just css` compiles it, and the media conditions it wraps `--room` in
 * judged against each device's size. What a real browser makes of them is
 * live acceptance's to show.
 */

import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { createRequire } from 'node:module';
import { flushSync } from 'svelte';
import { compile } from 'tailwindcss';
import { render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { roomOf, WindowRoom, type Room } from './room.svelte';
import Harness from '../testing/Harness.svelte';
import RoomProbe from '../testing/RoomProbe.svelte';
import { standIn } from './api/stand-in';

const SOURCE = resolve(process.cwd(), 'src/app.css');
const require = createRequire(import.meta.url);

/** The stylesheet compiled from `source`, with `candidates` used as class names. */
async function compiled(source: string, candidates: string[] = []): Promise<string> {
	const compiler = await compile(source, {
		base: dirname(SOURCE),
		loadStylesheet: async (id, base) => {
			const path =
				id === 'tailwindcss' ? require.resolve('tailwindcss/index.css') : resolve(base, id);
			return { path, base: dirname(path), content: readFileSync(path, 'utf8') };
		},
	});
	return compiler.build(candidates);
}

/**
 * Every place `wanted` appears in the stylesheet, with the `@media`
 * conditions it sits inside, outermost first. Enough of a CSS reader for a
 * stylesheet that nests, as Tailwind's unminified output does.
 */
function whereFound(css: string, wanted: RegExp): { text: string; media: string[] }[] {
	const found: { text: string; media: string[] }[] = [];
	const stack: string[] = [];
	let from = 0;
	const clean = css.replace(/\/\*[\s\S]*?\*\//g, '');
	for (let at = 0; at < clean.length; at++) {
		const c = clean[at];
		if (c === '{') {
			stack.push(clean.slice(from, at).trim());
			from = at + 1;
		} else if (c === '}' || c === ';') {
			const text = clean.slice(from, at).trim();
			if (wanted.test(text)) {
				const media = stack.filter((p) => p.startsWith('@media')).map((p) => p.slice(6).trim());
				found.push({ text, media });
			}
			if (c === '}') stack.pop();
			from = at + 1;
		}
	}
	return found;
}

/**
 * Whether a window of this size matches a media condition. Only the two
 * features the Room is written in; anything else fails the test rather than
 * being guessed at, so a threshold rewritten in another form is noticed here.
 */
function matches(condition: string, width: number, height: number): boolean {
	return condition.split(/\s+and\s+/).every((part) => {
		const feature = /^\(\s*min-(width|height)\s*:\s*(\d+(?:\.\d+)?)px\s*\)$/.exec(part.trim());
		if (!feature) throw new Error(`the Room cannot judge the media condition ${part}`);
		const [, dimension, figure] = feature;
		return (dimension === 'width' ? width : height) >= Number(figure);
	});
}

/** The Room a window of this size gets from the stylesheet: the last `--room` that applies. */
function roomAt(css: string, width: number, height: number): Room {
	const declared = whereFound(css, /^--room\s*:/).filter(({ media }) =>
		media.every((condition) => matches(condition, width, height)),
	);
	const last = declared.at(-1);
	if (!last) throw new Error('the stylesheet declares no --room');
	return roomOf(last.text.split(':')[1].trim());
}

describe('the thresholds, written once in the stylesheet', () => {
	it.each<[string, number, number, Room]>([
		['a phone turned sideways', 850, 390, 'phone'],
		['a small tablet held upright', 600, 960, 'phone'],
		['an iPad mini held upright', 744, 1133, 'wide'],
		['an iPad Pro 11" held upright', 834, 1194, 'wide'],
		['an iPad Pro 11" on its side', 1194, 834, 'roomy'],
		['a MacBook 14"', 1512, 982, 'roomy'],
		['a phone held upright', 390, 844, 'phone'],
	])('%s (%i × %i) is %s', async (_, width, height, room) => {
		const css = await compiled(readFileSync(SOURCE, 'utf8'));
		expect(roomAt(css, width, height)).toBe(room);
	});

	it('moves a style rule and the Room together when a figure changes', async () => {
		// The same window, 750 by 900, before and after the wide layout's width is
		// moved from 700 to 800 in the one place it is written.
		const source = readFileSync(SOURCE, 'utf8');
		const moved = source.replace(/min-width: 700px/g, 'min-width: 800px');
		expect(moved).not.toBe(source);

		const before = await compiled(source, ['wide:flex']);
		const after = await compiled(moved, ['wide:flex']);

		expect(roomAt(before, 750, 900)).toBe('wide');
		expect(roomAt(after, 750, 900)).toBe('phone');

		// And a class written `wide:` sits under exactly the condition that
		// makes the Room wide, before the change and after it.
		for (const css of [before, after]) {
			const [rule] = whereFound(css, /^display\s*:\s*flex$/);
			const [wide] = whereFound(css, /^--room\s*:\s*wide$/);
			expect(rule.media).toEqual(wide.media);
		}
	});

	it('writes no threshold anywhere else in the stylesheet', () => {
		const source = readFileSync(SOURCE, 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
		// A media feature, in its parentheses: a control's own min-height is no threshold.
		const widths = source.match(/\(min-(width|height)\s*:/g) ?? [];
		// 700 wide, 560 tall and 1000 wide, each once.
		expect(widths).toHaveLength(3);
	});

	it('turns a phone sideways at the same floor wide stands on (#222)', () => {
		const source = readFileSync(SOURCE, 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
		const floor = source.match(/\(min-height:\s*(\d+)px\)/)?.[1];
		const sideways = source.match(/\(height\s*<\s*(\d+)px\)/)?.[1];
		expect(floor).toBeDefined();
		expect(sideways).toBe(floor);
	});
});

describe('the Room', () => {
	let stop: (() => void) | undefined;
	afterEach(() => {
		stop?.();
		stop = undefined;
	});

	it('is right from the first frame, before anything has been drawn', () => {
		let room: WindowRoom | undefined;
		stop = $effect.root(() => {
			room = new WindowRoom(() => 'roomy');
		});
		// No flush: the effects that follow the window have not run yet.
		expect(room?.current).toBe('roomy');
	});

	it('follows the window as it is resized, without a reload', () => {
		let said = 'phone';
		let room: WindowRoom | undefined;
		stop = $effect.root(() => {
			room = new WindowRoom(() => said);
		});
		flushSync();
		expect(room?.current).toBe('phone');

		said = 'roomy';
		dispatchEvent(new Event('resize'));
		flushSync();
		expect(room?.current).toBe('roomy');

		said = 'wide';
		dispatchEvent(new Event('resize'));
		flushSync();
		expect(room?.current).toBe('wide');
	});

	it('is the phone when the stylesheet says nothing', () => {
		expect(roomOf('')).toBe('phone');
		expect(roomOf(' wide')).toBe('phone');
	});

	it.each<[Room | undefined, string, boolean, boolean]>([
		[undefined, 'phone', false, false],
		['phone', 'phone', false, false],
		['wide', 'wide', true, false],
		['roomy', 'roomy', true, true],
	])('is handed to a screen by its test: %s', (room, shown, wide, roomy) => {
		render(Harness, {
			props: { component: RoomProbe, client: standIn().client, room },
		});
		const said = screen.getByTestId('room');
		expect(said).toHaveTextContent(shown);
		expect(said.dataset.wide).toBe(String(wide));
		expect(said.dataset.roomy).toBe(String(roomy));
	});
});
