/**
 * Dropping a file or a link where nothing takes it (#204). jsdom has no
 * drag of its own, so each test says what a browser would say is carried.
 */

import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { fireEvent } from '@testing-library/svelte';
import { carriedBy, keepDropsOut } from './drop.svelte';
import { carrying } from '../testing/drops';

const pdf = new File(['%PDF'], 'tarte.pdf', { type: 'application/pdf' });

describe('a drop nothing takes', () => {
	let stop: () => void;
	beforeEach(() => {
		stop = keepDropsOut();
	});
	afterEach(() => {
		stop();
		document.body.replaceChildren();
	});

	it('does nothing, and the browser is kept from opening the file in place of Kamosu', async () => {
		const dataTransfer = carrying({ files: [pdf] });
		// `fireEvent` answers false when the default was prevented.
		expect(await fireEvent.dragOver(document.body, { dataTransfer })).toBe(false);
		// The pointer says so before it is let go.
		expect(dataTransfer.dropEffect).toBe('none');
		expect(await fireEvent.drop(document.body, { dataTransfer })).toBe(false);
	});

	it('keeps a dropped link from being followed', async () => {
		const dataTransfer = carrying({ link: 'https://example.test/tarte' });
		expect(await fireEvent.drop(document.body, { dataTransfer })).toBe(false);
	});

	it('leaves a link dropped into a field to be typed there, and a file not', async () => {
		const field = document.body.appendChild(document.createElement('input'));
		const link = carrying({ link: 'https://example.test/tarte' });
		expect(await fireEvent.drop(field, { dataTransfer: link })).toBe(true);
		expect(await fireEvent.drop(field, { dataTransfer: carrying({ files: [pdf] }) })).toBe(false);
	});

	it('leaves words dragged from another page alone', async () => {
		const dataTransfer = carrying({ text: 'two eggs' });
		expect(await fireEvent.drop(document.body, { dataTransfer })).toBe(true);
	});

	it('leaves a drag that began inside Kamosu alone', async () => {
		const card = document.body.appendChild(document.createElement('a'));
		const dataTransfer = carrying({ link: 'https://kamosu.example/recipes/b_1' });
		await fireEvent.dragStart(card, { dataTransfer });
		expect(await fireEvent.drop(document.body, { dataTransfer })).toBe(true);
		await fireEvent.dragEnd(card, { dataTransfer });
		expect(await fireEvent.drop(document.body, { dataTransfer })).toBe(false);
	});

	it('is not fooled by a card redrawn away mid-drag, whose drag never says it ended', async () => {
		const card = document.body.appendChild(document.createElement('a'));
		await fireEvent.dragStart(card, {
			dataTransfer: carrying({ link: 'https://kamosu.example/' }),
		});
		card.remove();
		// The next thing the pointer does on the page is press on it.
		await fireEvent.pointerDown(document.body);
		expect(await fireEvent.drop(document.body, { dataTransfer: carrying({ files: [pdf] }) })).toBe(
			false,
		);
	});
});

describe('what a drag says it carries', () => {
	it('is a file of no known type where Firefox gives its own name for one', () => {
		const dataTransfer = {
			types: ['Files'],
			items: [{ kind: 'file', type: 'application/x-moz-file' }],
		} as unknown as DataTransfer;
		expect(carriedBy(dataTransfer)).toEqual({ files: [''], link: false });
	});

	it('is a file where a picture pulled from a web page carries its address too', () => {
		const photo = new File(['x'], 'tarte.jpg', { type: 'image/jpeg' });
		const dataTransfer = carrying({ files: [photo], link: 'https://example.test/tarte.jpg' });
		expect(carriedBy(dataTransfer)).toEqual({ files: ['image/jpeg'], link: false });
	});

	it('is a file of no known type where the browser lists none', () => {
		const dataTransfer = { types: ['Files'], files: [] } as unknown as DataTransfer;
		expect(carriedBy(dataTransfer)).toEqual({ files: [''], link: false });
	});
});
