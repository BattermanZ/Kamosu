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
import { watched } from '../mistake.svelte';

/** Stage one file and answer the id an Operation names it by. */
export type Uploader = (file: Blob) => Promise<string>;

const KEY = Symbol('upload');
const PICTURE_KEY = Symbol('photograph');

export function provideUpload(uploader: () => Uploader): void {
	setContext(KEY, uploader);
}

export function useUpload(): Uploader {
	const uploader = getContext<(() => Uploader) | undefined>(KEY)?.();
	if (!uploader) throw new Error('no uploader in context');
	return uploader;
}

/**
 * Sending a picture that has to reach the server NOW, and answer the name the
 * recipe will carry (ADR 0017).
 *
 * This is not the same act as taking a photograph while cooking. That one goes
 * to the outbox, is named `local:…` on the phone, and is given its real name
 * when the cooking it belongs to is finally sent — which only works because
 * the outbox rewrites those names on the way out. Nothing rewrites them inside
 * a `save_recipe_version`, so a recipe that took that path would store a name
 * no server has ever heard of (#83).
 */
export function providePhotograph(uploader: () => Uploader): void {
	setContext(PICTURE_KEY, uploader);
}

export function usePhotograph(): Uploader {
	const uploader = getContext<(() => Uploader) | undefined>(PICTURE_KEY)?.();
	if (!uploader) throw new Error('no photograph uploader in context');
	return uploader;
}

export const realUpload =
	(doFetch: typeof globalThis.fetch = globalThis.fetch.bind(globalThis)): Uploader =>
	(file) =>
		// Not an Operation, so `createClient` reports none of its mistakes (#98).
		watched(stage(doFetch, file));

async function stage(doFetch: typeof globalThis.fetch, file: Blob): Promise<string> {
	let response: Response;
	try {
		response = await doFetch('/api/uploads', {
			method: 'POST',
			headers: { 'content-type': file.type || 'application/octet-stream' },
			body: file,
		});
	} catch (cause) {
		throw new OperationError('upload', 'internal', 'Kamosu could not be reached.', {
			cause,
			reached: false,
		});
	}
	const staged = (await readEnvelope('upload', response)) as { upload_id?: string };
	if (!staged?.upload_id) {
		throw new OperationError('upload', 'internal', 'Kamosu staged the file but named no id.');
	}
	return staged.upload_id;
}

/**
 * Send one picture to `POST /api/photographs`, where it is remade at the door
 * and named by its own bytes (ADR 0017). Answers that name.
 */
export const realPhotographUpload =
	(doFetch: typeof globalThis.fetch = globalThis.fetch.bind(globalThis)) =>
	(picture: Blob): Promise<string> =>
		// Nor is this one (#98).
		watched(keep(doFetch, picture));

async function keep(doFetch: typeof globalThis.fetch, picture: Blob): Promise<string> {
	let response: Response;
	try {
		response = await doFetch('/api/photographs', {
			method: 'POST',
			headers: { 'content-type': picture.type || 'application/octet-stream' },
			body: picture,
		});
	} catch (cause) {
		throw new OperationError('upload_photograph', 'internal', 'Kamosu could not be reached.', {
			cause,
			reached: false,
		});
	}
	const made = (await readEnvelope('upload_photograph', response)) as {
		photograph_id?: string;
	};
	if (!made?.photograph_id) {
		throw new OperationError(
			'upload_photograph',
			'internal',
			'Kamosu kept the picture but named no Photograph.',
		);
	}
	return made.photograph_id;
}
