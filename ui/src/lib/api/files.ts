/**
 * Fetching a file the phone can hand on (#149, #156), and handing it on.
 *
 * In the app installed on an iPhone a Sheet or a recipe file cannot open in a
 * tab: iOS keeps every address inside the manifest's scope in the app's own
 * window, which has no Share, Save or Print. So there the screen fetches the
 * file itself and gives it to the share sheet. The file is not an Operation's
 * answer, so it comes through here under the same Credential, and a screen
 * takes the fetcher from context exactly as it takes its uploader (`./upload`).
 */

import { getContext, setContext } from 'svelte';
import { OperationError, readEnvelope } from './client';
import { watched } from '../mistake.svelte';
import type { OperationName } from './catalogue';
import type { Device } from '../offline/device.svelte';

/** What kind of file is being fetched: the Operation that made it, and how it is named. */
export interface FileKind {
	/** The Operation a failure is reported as. */
	operation: OperationName;
	/** The MIME type the file is handed on as. */
	type: string;
	/** The name used where the server gave none. */
	fallbackName: string;
	/** A few bytes of the kind, so `canShare` answers about it and not about nothing. */
	probe: string;
}

/** A Sheet (#75, #149): a PDF made by `make_sheet`. */
export const SHEET: FileKind = {
	operation: 'make_sheet',
	type: 'application/pdf',
	fallbackName: 'Sheet.pdf',
	probe: '%PDF-',
};

/** A recipe file (#65, #156): a Bundle's zip, described by `export_bundle`. */
export const BUNDLE: FileKind = {
	operation: 'export_bundle',
	type: 'application/zip',
	fallbackName: 'Recipe.zip',
	probe: 'PK\u0003\u0004',
};

/** The file at `at`, of the kind given, named the way the server named it. */
export type FileFetcher = (at: string, kind: FileKind) => Promise<File>;

const KEY = Symbol('files');

export function provideFiles(fetcher: () => FileFetcher): void {
	setContext(KEY, fetcher);
}

export function useFiles(): FileFetcher {
	const fetcher = getContext<(() => FileFetcher) | undefined>(KEY)?.();
	if (!fetcher) throw new Error('no file fetcher in context');
	return fetcher;
}

export const realFiles =
	(doFetch: typeof globalThis.fetch = globalThis.fetch.bind(globalThis)): FileFetcher =>
	(at, kind) =>
		// Not an Operation either, so `createClient` reports none of its mistakes (#98).
		watched(fetchFile(doFetch, at, kind));

async function fetchFile(
	doFetch: typeof globalThis.fetch,
	at: string,
	kind: FileKind,
): Promise<File> {
	let response: Response;
	try {
		response = await doFetch(at);
	} catch (cause) {
		throw new OperationError(kind.operation, 'internal', 'Kamosu could not be reached.', {
			cause,
			reached: false,
		});
	}
	if (!response.ok) {
		// A refusal comes back in the envelope every Door route answers with.
		await readEnvelope(kind.operation, response);
		throw new OperationError(kind.operation, 'internal', `Kamosu answered ${response.status}.`);
	}
	const name = fileName(response.headers.get('content-disposition')) ?? kind.fallbackName;
	return new File([await response.blob()], name, { type: kind.type });
}

/**
 * The file name a `Content-Disposition` gives: `filename*`, which carries the
 * real name percent-encoded as UTF-8 (RFC 6266), before the ASCII stand-in in
 * `filename`. A title like `Bœuf bourguignon` needs the first.
 */
export function fileName(disposition: string | null): string | undefined {
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

/**
 * Whether a file of this kind goes to the share sheet rather than to an
 * address: only in the app installed on an iPhone or iPad (#149), and only
 * where the phone says it will share one. Asked with a stand-in before the
 * real file exists, because the choice of path is made at the first tap.
 * Web Share limits which file types it takes, and the list differs by
 * browser, so a no here keeps the screen on the plain navigation.
 */
export function sharesFiles(device: Pick<Device, 'installed' | 'apple'>, kind: FileKind): boolean {
	if (!device.installed || !device.apple || typeof navigator.share !== 'function') return false;
	const probe = new File([kind.probe], kind.fallbackName, { type: kind.type });
	return navigator.canShare?.({ files: [probe] }) === true;
}

/**
 * Hand a file already on the phone to the share sheet: Save to Files, Print,
 * AirDrop. Call it straight from the tap with nothing awaited first, since iOS
 * only opens the share sheet for a tap it can still see (#149).
 *
 * Answers `shared` once it has gone somewhere, `kept` when the share sheet was
 * closed (not a failure: the file stays for another try) and `failed` for any
 * other refusal.
 */
export function shareFile(file: File): Promise<'shared' | 'kept' | 'failed'> {
	return navigator.share({ files: [file] }).then(
		() => 'shared' as const,
		(error: unknown) =>
			error instanceof DOMException && error.name === 'AbortError'
				? ('kept' as const)
				: ('failed' as const),
	);
}
