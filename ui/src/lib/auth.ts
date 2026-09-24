import { getContext, setContext } from 'svelte';
import { OperationError, readEnvelope } from './api/client';
import { watched } from './mistake.svelte';
import { sessionBegan } from './offline/library.svelte';

export interface AuthClient {
	authenticate(
		mode: 'first-person' | 'login' | 'invite' | 'recover',
		input: { name?: string; password: string; session_name: string; link?: string },
	): Promise<void>;
}

const KEY = Symbol('auth');

export function provideAuth(client: () => AuthClient): void {
	setContext(KEY, client);
}

export function useAuth(): AuthClient {
	const client = getContext<(() => AuthClient) | undefined>(KEY)?.();
	if (!client) throw new Error('no authentication client in context');
	return client;
}

/**
 * Authenticating is the one ask a screen makes that cannot be an Operation —
 * there is no Credential yet — so the Catalogue's client carries none of it,
 * and `watched` below is what puts its mistakes where an Operation's go (#98).
 */
async function reach(
	mode: Parameters<AuthClient['authenticate']>[0],
	input: Parameters<AuthClient['authenticate']>[1],
): Promise<void> {
	const path = {
		'first-person': '/auth/first-person',
		login: '/auth/login',
		invite: '/auth/invite',
		recover: '/auth/recover',
	}[mode];

	let response: Response;
	try {
		response = await fetch(path, {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(input),
		});
	} catch (cause) {
		// Never reached, named exactly as `httpTransport` names it: a phone with
		// no network signing in is not Kamosu having gone wrong, and the bare
		// `TypeError` this used to let through would have said it was.
		throw new OperationError('authentication', 'internal', 'Kamosu could not be reached.', {
			cause,
			reached: false,
		});
	}

	// The Core's own words, in the envelope every Door route answers with. A
	// spent Invite and a wrong password are different things to be told, and
	// this used to say "authentication failed" to both (#126). Called only for
	// the refusal it throws: an answer that is not ok carries no result.
	if (!response.ok) await readEnvelope('authentication', response);
	// Whoever this is, the library on this phone was not filled for them (#76).
	sessionBegan();
}

export const realAuth = (): AuthClient => ({
	authenticate: (mode, input) => watched(reach(mode, input)),
});
