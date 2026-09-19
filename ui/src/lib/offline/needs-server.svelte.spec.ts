/**
 * A button for something only the server can do (#76, ADR 0013): offline it
 * stays where it is, greyed, and says what it is waiting for.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import NeedsServer from './NeedsServer.svelte';

afterEach(() => {
	Reflect.deleteProperty(navigator, 'onLine');
});

const props = (onclick = vi.fn()) => ({
	label: 'Edit this recipe',
	waiting: 'Editing waits for the server',
	onclick,
	shapeClass: 'block',
	lookClass: 'bg-accent',
});

describe('a button that needs the server', () => {
	it('does its job online', async () => {
		const onclick = vi.fn();
		render(NeedsServer, { props: props(onclick) });
		await fireEvent.click(screen.getByRole('button', { name: 'Edit this recipe' }));
		expect(onclick).toHaveBeenCalledOnce();
	});

	it('offline, says what it is waiting for and cannot be pressed', async () => {
		Object.defineProperty(navigator, 'onLine', { value: false, configurable: true });
		const onclick = vi.fn();
		render(NeedsServer, { props: props(onclick) });
		const button = await screen.findByRole('button', { name: 'Editing waits for the server' });
		expect(button).toBeDisabled();
		await fireEvent.click(button);
		expect(onclick).not.toHaveBeenCalled();
	});

	it('comes back the moment the browser is online again', async () => {
		Object.defineProperty(navigator, 'onLine', { value: false, configurable: true });
		render(NeedsServer, { props: props() });
		await screen.findByRole('button', { name: 'Editing waits for the server' });
		Object.defineProperty(navigator, 'onLine', { value: true, configurable: true });
		dispatchEvent(new Event('online'));
		expect(await screen.findByRole('button', { name: 'Edit this recipe' })).toBeEnabled();
	});
});
