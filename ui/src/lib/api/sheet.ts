/**
 * Fetching a finished Sheet as a file the phone can hand on (#149).
 *
 * In the app installed on an iPhone a Sheet cannot open in a tab: iOS keeps
 * every address inside the manifest's scope in the app's own window, which has
 * no Share, Save or Print. So there the screen fetches the PDF itself and gives
 * it to the share sheet. The PDF is not an Operation's answer, so it comes
 * through here under the same Credential, and a screen takes the fetcher from
 * context exactly as it takes its uploader (`./upload`).
 */

import { getContext, setContext } from 'svelte';
import { OperationError, readEnvelope } from './client';
import { watched } from '../mistake.svelte';

/** The Sheet at `fetch_at`, as a PDF file named the way the server named it. */
export type SheetFetcher = (at: string) => Promise<File>;

const KEY = Symbol('sheet');

export function provideSheets(fetcher: () => SheetFetcher): void {
	setContext(KEY, fetcher);
}

export function useSheets(): SheetFetcher {
	const fetcher = getContext<(() => SheetFetcher) | undefined>(KEY)?.();
	if (!fetcher) throw new Error('no sheet fetcher in context');
	return fetcher;
}

export const realSheets =
	(doFetch: typeof globalThis.fetch = globalThis.fetch.bind(globalThis)): SheetFetcher =>
	(at) =>
		// Not an Operation either, so `createClient` reports none of its mistakes (#98).
		watched(fetchSheet(doFetch, at));

async function fetchSheet(doFetch: typeof globalThis.fetch, at: string): Promise<File> {
	let response: Response;
	try {
		response = await doFetch(at);
	} catch (cause) {
		throw new OperationError('make_sheet', 'internal', 'Kamosu could not be reached.', {
			cause,
			reached: false,
		});
	}
	if (!response.ok) {
		// A refusal comes back in the envelope every Door route answers with.
		await readEnvelope('make_sheet', response);
		throw new OperationError('make_sheet', 'internal', `Kamosu answered ${response.status}.`);
	}
	const name = sheetFileName(response.headers.get('content-disposition')) ?? 'Sheet.pdf';
	return new File([await response.blob()], name, { type: 'application/pdf' });
}

/**
 * The file name a `Content-Disposition` gives: `filename*`, which carries the
 * real name percent-encoded as UTF-8 (RFC 6266), before the ASCII stand-in in
 * `filename`. A title like `Bœuf bourguignon` needs the first.
 */
export function sheetFileName(disposition: string | null): string | undefined {
	if (!disposition) return undefined;
	const encoded = /filename\*\s*=\s*UTF-8''([^;]+)/i.exec(disposition)?.[1];
	if (encoded) {
		try {
			return decodeURIComponent(encoded.trim());
		} catch {
			// Not valid percent-encoding: the plain name below still serves.
		}
	}
	return /filename\s*=\s*"([^"]*)"/i.exec(disposition)?.[1] || undefined;
}
