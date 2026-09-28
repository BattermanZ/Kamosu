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
