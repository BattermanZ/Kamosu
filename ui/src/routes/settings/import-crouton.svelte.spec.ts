/**
 * The screen-seam test: bringing a Crouton library in from Settings (#69).
 *
 * The file itself travels out of band (ADR 0001), so the test stands in for
 * the upload and checks what happens around it: the id it answers is what
 * `import_crouton` is asked with, and the screen then opens that Job's Report.
 */

import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import ImportTestHarness from './ImportTestHarness.svelte';
import { standIn } from '$lib/api/stand-in';
import { OperationError } from '$lib/api/client';
import { went } from '../../testing/navigation';
import { carrying } from '../../testing/drops';

const zip = () => new File(['PK'], 'Crouton Recipes.zip', { type: 'application/zip' });

describe('importing a Crouton library', () => {
	it('sends the file, asks for the import with its id, and opens the Report', async () => {
		const kamosu = standIn({
			list_jobs: { jobs: [] },
			import_crouton: { job_id: 'j_7' },
		});
		const upload = vi.fn(async () => 'u_staged');
		render(ImportTestHarness, { props: { client: kamosu.client, upload } });

		const input = screen.getByLabelText('Choose a Crouton export');
		await fireEvent.change(input, { target: { files: [zip()] } });

		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/imports/j_7'));
		expect(upload).toHaveBeenCalledOnce();
		expect(kamosu.calls.find((call) => call.operation === 'import_crouton')?.input).toEqual({
			upload_id: 'u_staged',
		});
	});

	it('links the last Crouton import so its Report is never lost', async () => {
		const kamosu = standIn({
			list_jobs: {
				jobs: [
					{
						id: 'j_web',
						operation: 'import_web_link',
						status: 'completed',
						created_at: '2026-09-19T10:00:00Z',
					},
					{
						id: 'j_old',
						operation: 'import_crouton',
						status: 'completed',
						created_at: '2026-09-18T10:00:00Z',
					},
				],
			},
		});
		render(ImportTestHarness, { props: { client: kamosu.client, upload: vi.fn() } });
		const link = await screen.findByRole('link', { name: /last Crouton import/ });
		expect(link).toHaveAttribute('href', '/imports/j_old');
	});

	it('says why a file was refused', async () => {
		const kamosu = standIn({ list_jobs: { jobs: [] } });
		const upload = vi.fn(async () => {
			throw new OperationError('upload', 'bad_request', 'the file is larger than 2 GB');
		});
		render(ImportTestHarness, { props: { client: kamosu.client, upload } });
		await fireEvent.change(screen.getByLabelText('Choose a Crouton export'), {
			target: { files: [zip()] },
		});
		expect(await screen.findByRole('alert')).toHaveTextContent('the file is larger than 2 GB');
	});
});

describe('dropping a Crouton export on its import (#205)', () => {
	function drawn() {
		const kamosu = standIn({ list_jobs: { jobs: [] }, import_crouton: { job_id: 'j_7' } });
		const upload = vi.fn(async () => 'u_staged');
		render(ImportTestHarness, { props: { client: kamosu.client, upload } });
		return { kamosu, upload, place: screen.getByRole('heading', { name: 'Bring recipes in' }) };
	}

	it('starts the import its button starts', async () => {
		const { kamosu, upload, place } = drawn();
		const dataTransfer = carrying({ files: [zip()] });
		await fireEvent.dragEnter(place, { dataTransfer });
		expect(screen.getByText('Drop to import this export')).toBeInTheDocument();
		expect(await fireEvent.dragOver(place, { dataTransfer })).toBe(false);
		await fireEvent.drop(place, { dataTransfer });

		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/imports/j_7'));
		expect(upload).toHaveBeenCalledOnce();
		expect(kamosu.calls.find((call) => call.operation === 'import_crouton')?.input).toEqual({
			upload_id: 'u_staged',
		});
	});

	it('takes one .crumb, which no browser knows a type for', async () => {
		const { upload, place } = drawn();
		const dataTransfer = carrying({ files: [new File(['{}'], 'Miso Soup.crumb', { type: '' })] });
		await fireEvent.dragEnter(place, { dataTransfer });
		expect(screen.getByText('Drop a Crouton export here')).toBeInTheDocument();
		await fireEvent.drop(place, { dataTransfer });
		await vi.waitFor(() => expect(upload).toHaveBeenCalledOnce());
	});

	it('says what the button says when the file cannot be sent', async () => {
		const kamosu = standIn({ list_jobs: { jobs: [] } });
		const upload = vi.fn(async () => {
			throw new OperationError('upload', 'internal', 'Kamosu could not be reached');
		});
		render(ImportTestHarness, { props: { client: kamosu.client, upload } });
		const place = screen.getByRole('heading', { name: 'Bring recipes in' });
		const dataTransfer = carrying({ files: [zip()] });
		await fireEvent.dragEnter(place, { dataTransfer });
		await fireEvent.drop(place, { dataTransfer });
		expect(await screen.findByRole('alert')).toHaveTextContent('Kamosu could not be reached');
		expect(kamosu.calls.some((call) => call.operation === 'import_crouton')).toBe(false);
	});

	it('takes no second export while one is being sent, as the button takes none', async () => {
		const kamosu = standIn({ list_jobs: { jobs: [] }, import_crouton: { job_id: 'j_7' } });
		let sent: (id: string) => void = () => {};
		const upload = vi.fn(() => new Promise<string>((resolve) => (sent = resolve)));
		render(ImportTestHarness, { props: { client: kamosu.client, upload } });
		const place = screen.getByRole('heading', { name: 'Bring recipes in' });
		const dataTransfer = carrying({ files: [zip()] });
		await fireEvent.dragEnter(place, { dataTransfer });
		await fireEvent.drop(place, { dataTransfer });
		await vi.waitFor(() => expect(upload).toHaveBeenCalledOnce());

		await fireEvent.dragEnter(place, { dataTransfer });
		expect(screen.queryByText('Drop to import this export')).not.toBeInTheDocument();
		await fireEvent.drop(place, { dataTransfer });
		expect(upload).toHaveBeenCalledOnce();
		sent('u_staged');
		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/imports/j_7'));
	});

	it('refuses anything else in words and starts nothing', async () => {
		const { kamosu, upload, place } = drawn();
		const dataTransfer = carrying({
			files: [new File(['x'], 'plate.jpg', { type: 'image/jpeg' })],
		});
		await fireEvent.dragEnter(place, { dataTransfer });
		expect(screen.getByText('Only a Crouton export goes here')).toBeInTheDocument();
		await fireEvent.drop(place, { dataTransfer });
		expect(screen.getByRole('alert')).toHaveTextContent(
			"That didn't go anywhere. The import takes the zip Crouton exports, or one .crumb.",
		);
		expect(upload).not.toHaveBeenCalled();
		expect(kamosu.calls.some((call) => call.operation === 'import_crouton')).toBe(false);
	});
});
