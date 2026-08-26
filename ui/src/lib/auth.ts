import { getContext, setContext } from 'svelte';
import { OperationError } from './api/client';

export interface AuthClient {
	authenticate(mode: 'first-person' | 'login', input: { name: string; password: string; session_name: string }): Promise<void>;
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
		const response = await fetch(mode === 'login' ? '/auth/login' : '/auth/first-person', {
			method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(input)
		});
		if (!response.ok) throw new OperationError('authentication', 'unauthorized', 'authentication failed');
	}
});
