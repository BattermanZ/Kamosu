/**
 * A secondary control may be drawn smaller than 48, never tapped smaller
 * (ADR 0044, 10 October 2026). The small buttons that replaced underlined
 * words are drawn at 40 and the shelf arrows at 32, and each grows an
 * invisible round itself to 48, as the cooking screen's chips have since #88.
 * Read off the stylesheet's source, since jsdom lays nothing out.
 */

import { readFileSync } from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';

const source = readFileSync(path.resolve(__dirname, '../app.css'), 'utf8').replace(
	/\/\*[\s\S]*?\*\//g,
	'',
);

/** The body of one `@utility`, braces balanced. */
function utility(name: string): string {
	const start = source.indexOf(`@utility ${name} {`);
	expect(start).toBeGreaterThan(-1);
	let depth = 0;
	for (let i = source.indexOf('{', start); i < source.length; i++) {
		if (source[i] === '{') depth++;
		if (source[i] === '}' && --depth === 0) return source.slice(start, i + 1);
	}
	throw new Error(`${name} never closes`);
}

const drawn = (body: string, property: string) =>
	Number(body.match(new RegExp(`${property}:\\s*(\\d+)px`))?.[1]);
const overhang = (body: string) => Number(body.match(/inset:\s*-(\d+)px/)?.[1]);

describe('a control drawn under 48', () => {
	it.each([
		['quiet-button', 'min-height'],
		['shelf-arrow', 'height'],
		['tap-out', 'height'],
	])('%s is tapped at 48 whatever it is drawn at', (name, property) => {
		const body = utility(name);
		const size = name === 'tap-out' ? 32 : drawn(body, property);
		expect(body).toContain('position: relative');
		expect(size + 2 * overhang(body)).toBe(48);
	});
});
