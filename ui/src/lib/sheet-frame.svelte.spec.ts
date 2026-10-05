/**
 * The one frame every sheet is drawn in (#196, ADR 0044).
 *
 * What these guard: the frame is a bottom sheet in a phone Room and a window
 * in a wide one; a photograph covers the screen in both; the page behind is
 * dimmed, except under the cooking screen's sheets on the phone; the caret
 * goes in when it opens and back when it closes, to the control that now
 * stands where the opener was if the opener is gone; Escape closes; Enter
 * confirms only where the sheet says it has one thing to confirm, and never
 * from a control that uses Enter itself.
 *
 * And that no screen draws a dialog of its own any more, which is the whole
 * point of having one frame.
 *
 * What they cannot guard is where the window sits or how big it is: jsdom lays
 * nothing out. That is checked in a real browser at the sizes #192 names.
 */

import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import type { ComponentProps } from 'svelte';
import type { Room } from '$lib/room.svelte';
import Confirm from '$lib/Confirm.svelte';
import SheetFrameTestHarness from '$lib/SheetFrameTestHarness.svelte';

type Props = ComponentProps<typeof SheetFrameTestHarness>;

async function opened(props: Props = {}) {
	render(SheetFrameTestHarness, { props });
	const opener = screen.getByRole('button', { name: 'Open' });
	opener.focus();
	await fireEvent.click(opener);
	return { opener, sheet: screen.getByRole('dialog', { name: 'A sheet' }) };
}

const dimming = () => document.querySelector('[data-sheet-dimming]');

describe('the shape of a sheet', () => {
	it('is a bottom sheet in a phone Room', async () => {
		const { sheet } = await opened({ room: 'phone' });
		expect(sheet.dataset.sheet).toBe('sheet');
	});

	it.each<Room>(['wide', 'roomy'])('is a window in a %s Room', async (room) => {
		const { sheet } = await opened({ room });
		expect(sheet.dataset.sheet).toBe('window');
	});

	it.each<Room>(['phone', 'wide'])(
		'a photograph covers the whole screen in a %s Room',
		async (room) => {
			const { sheet } = await opened({ room, kind: 'cover' });
			expect(sheet.dataset.sheet).toBe('cover');
		},
	);

	it.each<Room>(['phone', 'wide'])('dims the page behind it in a %s Room', async (room) => {
		await opened({ room });
		expect(dimming()).not.toBeNull();
	});

	it('dims nothing under the cooking screen on the phone', async () => {
		await opened({ room: 'phone', dim: 'cook' });
		expect(dimming()).toBeNull();
	});

	it('dims the cooking screen behind a window', async () => {
		await opened({ room: 'wide', dim: 'cook' });
		expect(dimming()).not.toBeNull();
	});
});

describe('the caret', () => {
	it.each<Room>(['phone', 'wide'])('moves into the sheet when it opens (%s)', async (room) => {
		const { sheet } = await opened({ room });
		expect(document.activeElement).toBe(sheet);
	});

	it('lands on the field that asks for it', async () => {
		await opened({ field: true });
		expect(document.activeElement).toBe(screen.getByRole('textbox', { name: 'A name' }));
	});

	it.each<Room>(['phone', 'wide'])(
		'goes back to what opened the sheet when it closes (%s)',
		async (room) => {
			const { opener } = await opened({ room });
			await fireEvent.click(screen.getByRole('button', { name: 'Done' }));
			expect(screen.queryByRole('dialog')).toBeNull();
			await waitFor(() => expect(document.activeElement).toBe(opener));
		},
	);

	it('goes to the control standing in its place when the opener is gone', async () => {
		await opened({ replaces: true });
		await fireEvent.click(screen.getByRole('button', { name: 'Done' }));
		await waitFor(() =>
			expect(document.activeElement).toBe(
				screen.getByRole('button', { name: 'Names Pizza Dough' }),
			),
		);
	});
});

describe('the keyboard', () => {
	it.each<Room>(['phone', 'wide'])('Escape closes it (%s)', async (room) => {
		const { opener } = await opened({ room });
		await fireEvent.keyDown(window, { key: 'Escape' });
		expect(screen.queryByRole('dialog')).toBeNull();
		await waitFor(() => expect(document.activeElement).toBe(opener));
	});

	it('Enter confirms from the sheet itself and from a one-line field', async () => {
		const onconfirm = vi.fn();
		const { sheet } = await opened({ confirms: true, field: true, onconfirm });
		await fireEvent.keyDown(sheet, { key: 'Enter' });
		await fireEvent.keyDown(screen.getByRole('textbox', { name: 'A name' }), { key: 'Enter' });
		expect(onconfirm).toHaveBeenCalledTimes(2);
	});

	it('Enter is left alone on a button and in a multi-line field', async () => {
		const onconfirm = vi.fn();
		await opened({ confirms: true, onconfirm });
		await fireEvent.keyDown(screen.getByRole('button', { name: 'Done' }), { key: 'Enter' });
		await fireEvent.keyDown(screen.getByRole('textbox', { name: 'A note' }), { key: 'Enter' });
		expect(onconfirm).not.toHaveBeenCalled();
	});

	it('a held Enter, still repeating from the press that opened it, confirms nothing', async () => {
		const onconfirm = vi.fn();
		const { sheet } = await opened({ confirms: true, onconfirm });
		await fireEvent.keyDown(sheet, { key: 'Enter', repeat: true });
		expect(onconfirm).not.toHaveBeenCalled();
	});

	it('Enter in a field on the page behind confirms nothing', async () => {
		const onconfirm = vi.fn();
		await opened({ confirms: true, onconfirm });
		await fireEvent.keyDown(screen.getByRole('textbox', { name: 'A search on the page' }), {
			key: 'Enter',
		});
		expect(onconfirm).not.toHaveBeenCalled();
	});
});

describe('confirming something that cannot be undone', () => {
	const asked = (more: Partial<ComponentProps<typeof Confirm>> = {}) => {
		const run = vi.fn();
		const cancel = vi.fn();
		render(Confirm, {
			props: {
				title: 'Delete Soba?',
				consequence: 'It goes.',
				act: 'Delete',
				run,
				cancel,
				...more,
			},
		});
		return { run, cancel, sheet: screen.getByRole('dialog', { name: 'Delete Soba?' }) };
	};

	it('Enter does it, and Escape backs out', async () => {
		const { run, cancel, sheet } = asked();
		expect(document.activeElement).toBe(sheet);
		await fireEvent.keyDown(sheet, { key: 'Enter' });
		expect(run).toHaveBeenCalledTimes(1);
		await fireEvent.keyDown(window, { key: 'Escape' });
		expect(cancel).toHaveBeenCalledTimes(1);
	});

	it('Enter chooses nothing where a gentler act is offered beside it', async () => {
		const { run, sheet } = asked({ instead: { label: 'Stand them down', run: vi.fn() } });
		await fireEvent.keyDown(sheet, { key: 'Enter' });
		expect(run).not.toHaveBeenCalled();
	});

	it('neither key does anything while the act is running', async () => {
		const { run, cancel, sheet } = asked({ busy: true });
		await fireEvent.keyDown(sheet, { key: 'Enter' });
		await fireEvent.keyDown(window, { key: 'Escape' });
		expect(run).not.toHaveBeenCalled();
		expect(cancel).not.toHaveBeenCalled();
	});
});

describe('every sheet in Kamosu', () => {
	it('is drawn by the frame, none by itself', () => {
		const sources = import.meta.glob<string>('/src/**/*.svelte', {
			query: '?raw',
			import: 'default',
			eager: true,
		});
		const own = Object.entries(sources)
			.filter(([path]) => path !== '/src/lib/SheetFrame.svelte')
			// A dialog by name, or a bottom sheet drawn by hand without one, as
			// History's two were.
			.filter(([, source]) =>
				/role="dialog"|aria-modal|fixed inset-x-0 bottom-0 z-[345]0|fixed inset-0 z-\d+ flex items-end/.test(
					source,
				),
			)
			.map(([path]) => path);
		expect(own).toEqual([]);

		const framed = Object.entries(sources)
			.filter(([, source]) => /<SheetFrame[\s>]/.test(source))
			.map(([path]) => path.replace(/^.*\//, ''))
			.filter((name) => !name.includes('TestHarness'))
			.sort();
		expect(framed).toEqual([
			'Carrying.svelte',
			'ComponentPicker.svelte',
			'Confirm.svelte',
			'Cooking.svelte',
			'LanguageSheet.svelte',
			'PasteSheet.svelte',
			'PhotoToRecipe.svelte',
			'RelatedSheet.svelte',
			'RenameSheet.svelte',
			'StepPhoto.svelte',
			'TagSheet.svelte',
			'Thread.svelte',
			'Writing.svelte',
		]);
	});
});
