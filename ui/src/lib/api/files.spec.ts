/**
 * A file fetched to be shared (#149, #156): named the way the server named it,
 * of the kind asked for, and a refusal read as one.
 */

import { describe, expect, it, vi } from 'vitest';
import { BUNDLE, SHEET, fileName, realFiles } from './files';

// As `disposition` in `src/web_door.rs` writes it.
const BOEUF =
	'inline; filename="B_uf bourguignon.pdf"; filename*=UTF-8\'\'B%C5%93uf%20bourguignon.pdf';

describe('fileName', () => {
	it('reads the real name over the ASCII stand-in', () => {
		expect(fileName(BOEUF)).toBe('Bœuf bourguignon.pdf');
	});

	it('falls back to the plain name where there is no encoded one', () => {
		expect(fileName('inline; filename="Soup.pdf"')).toBe('Soup.pdf');
	});

	it('names nothing when the server named nothing', () => {
		expect(fileName(null)).toBeUndefined();
		expect(fileName('inline')).toBeUndefined();
	});
});

describe('realFiles', () => {
	it('answers a PDF file under the name the server gave it', async () => {
		const fetch = vi.fn(
			async () =>
				new Response('%PDF-1.7', {
					headers: { 'content-type': 'application/pdf', 'content-disposition': BOEUF },
				}),
		);
		const file = await realFiles(fetch as unknown as typeof globalThis.fetch)(
			'/api/sheets/j_1',
			SHEET,
		);

		expect(fetch).toHaveBeenCalledWith('/api/sheets/j_1');
		expect(file.name).toBe('Bœuf bourguignon.pdf');
		expect(file.type).toBe('application/pdf');
	});

	it('answers a recipe file as a zip under the name the server gave it', async () => {
		const fetch = vi.fn(
			async () =>
				new Response('PK', {
					headers: {
						'content-type': 'application/zip',
						// As `zip_response` in `src/web_door.rs` writes it.
						'content-disposition':
							'attachment; filename="B_uf bourguignon.zip"; filename*=UTF-8\'\'B%C5%93uf%20bourguignon.zip',
					},
				}),
		);
		const file = await realFiles(fetch as unknown as typeof globalThis.fetch)(
			'/api/bundles/b_1',
			BUNDLE,
		);

		expect(fetch).toHaveBeenCalledWith('/api/bundles/b_1');
		expect(file.name).toBe('Bœuf bourguignon.zip');
		expect(file.type).toBe('application/zip');
	});

	it('throws the refusal the server answered', async () => {
		const fetch = vi.fn(
			async () =>
				new Response(
					JSON.stringify({ ok: false, error: { kind: 'not_found', message: 'no such Sheet' } }),
					{ status: 404, headers: { 'content-type': 'application/json' } },
				),
		);

		await expect(
			realFiles(fetch as unknown as typeof globalThis.fetch)('/api/sheets/j_1', SHEET),
		).rejects.toThrow('no such Sheet');
	});
});
