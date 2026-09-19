/**
 * Sending a file too large for an Operation's envelope (ADR 0001).
 *
 * A Crouton library is 114 MB (#69). Carried as base64 inside a JSON body it
 * would be a third larger again and would sit whole in the Job's stored input,
 * so it travels as its own bytes to `POST /api/uploads` under the same
 * Credential, and the Operation that reads it is asked with the id this
 * answers.
 *
 * A screen takes the uploader from context, exactly as it takes its client
 * (`$lib/kamosu`) and its authentication (`$lib/auth`): the layout provides
 * the real one, a test provides a stand-in, and a screen never reaches the
 * network itself.
 */

import { getContext, setContext } from 'svelte';
import { OperationError, readEnvelope } from './client';

/** Stage one file and answer the id an Operation names it by. */
export type Uploader = (file: Blob) => Promise<string>;

const KEY = Symbol('upload');

export function provideUpload(uploader: () => Uploader): void {
	setContext(KEY, uploader);
}

export function useUpload(): Uploader {
	const uploader = getContext<(() => Uploader) | undefined>(KEY)?.();
	if (!uploader) throw new Error('no uploader in context');
	return uploader;
}

export const realUpload =
	(doFetch: typeof globalThis.fetch = globalThis.fetch.bind(globalThis)): Uploader =>
	async (file) => {
		let response: Response;
		try {
			response = await doFetch('/api/uploads', {
				method: 'POST',
				headers: { 'content-type': file.type || 'application/octet-stream' },
				body: file,
			});
		} catch (cause) {
			throw new OperationError('upload', 'internal', 'Kamosu could not be reached.', { cause });
		}
		const staged = (await readEnvelope('upload', response)) as { upload_id?: string };
		if (!staged?.upload_id) {
			throw new OperationError('upload', 'internal', 'Kamosu staged the file but named no id.');
		}
		return staged.upload_id;
	};
