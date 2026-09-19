import { getContext, setContext } from 'svelte';
import { OperationError } from './api/client';
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

export const realAuth = (): AuthClient => ({
	async authenticate(mode, input) {
		const path = {
			'first-person': '/auth/first-person',
			login: '/auth/login',
			invite: '/auth/invite',
			recover: '/auth/recover',
		}[mode];
		const response = await fetch(path, {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(input),
		});
		if (!response.ok)
			throw new OperationError('authentication', 'unauthorized', 'authentication failed');
		// Whoever this is, the library on this phone was not filled for them (#76).
		sessionBegan();
	},
});
