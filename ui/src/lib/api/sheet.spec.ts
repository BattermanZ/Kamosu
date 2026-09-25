/**
 * The Sheet fetched to be shared (#149): named the way the server named it,
 * and a refusal read as one.
 */

import { describe, expect, it, vi } from 'vitest';
import { realSheets, sheetFileName } from './sheet';

// As `disposition` in `src/web_door.rs` writes it.
const BOEUF =
	'inline; filename="B_uf bourguignon.pdf"; filename*=UTF-8\'\'B%C5%93uf%20bourguignon.pdf';

describe('sheetFileName', () => {
	it('reads the real name over the ASCII stand-in', () => {
		expect(sheetFileName(BOEUF)).toBe('Bœuf bourguignon.pdf');
	});

	it('falls back to the plain name where there is no encoded one', () => {
		expect(sheetFileName('inline; filename="Soup.pdf"')).toBe('Soup.pdf');
	});

	it('names nothing when the server named nothing', () => {
		expect(sheetFileName(null)).toBeUndefined();
		expect(sheetFileName('inline')).toBeUndefined();
	});
});

describe('realSheets', () => {
	it('answers a PDF file under the name the server gave it', async () => {
		const fetch = vi.fn(
			async () =>
				new Response('%PDF-1.7', {
					headers: { 'content-type': 'application/pdf', 'content-disposition': BOEUF },
				}),
		);
		const file = await realSheets(fetch as unknown as typeof globalThis.fetch)('/api/sheets/j_1');

		expect(fetch).toHaveBeenCalledWith('/api/sheets/j_1');
		expect(file.name).toBe('Bœuf bourguignon.pdf');
		expect(file.type).toBe('application/pdf');
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
			realSheets(fetch as unknown as typeof globalThis.fetch)('/api/sheets/j_1'),
		).rejects.toThrow('no such Sheet');
	});
});
