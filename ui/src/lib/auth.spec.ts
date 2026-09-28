import { afterEach, describe, expect, it, vi } from 'vitest';
import { realAuth } from './auth';
import { OperationError } from './api/client';

const input = { name: 'Camille', password: 'a password', session_name: 'A browser' };

/** Answer every request with `status` and `body`, as the Door would. */
function answering(status: number, body: unknown) {
	vi.stubGlobal(
		'fetch',
		vi.fn(async () => new Response(JSON.stringify(body), { status })),
	);
}

describe('signing in', () => {
	afterEach(() => vi.unstubAllGlobals());

	// It used to say "authentication failed" whatever the Core said, so a spent
	// Invite read exactly like a wrong password (#126).
	it('passes on the words the Core refused in', async () => {
		answering(401, {
			ok: false,
			error: { kind: 'unauthorized', message: 'this Invite has already been spent or revoked' },
		});

		const refused = realAuth().authenticate('invite', { ...input, link: '/invite/8f2c1a94e07b' });

		await expect(refused).rejects.toThrow(OperationError);
		await expect(refused).rejects.toMatchObject({
			kind: 'unauthorized',
			message: 'this Invite has already been spent or revoked',
		});
		expect(fetch).toHaveBeenCalledWith('/auth/invite', expect.objectContaining({ method: 'POST' }));
	});

	it('keeps the kind the Core named', async () => {
		answering(400, {
			ok: false,
			error: { kind: 'bad_request', message: 'Invite link is invalid' },
		});

		await expect(
			realAuth().authenticate('invite', { ...input, link: '/invite/' }),
		).rejects.toMatchObject({ kind: 'bad_request', message: 'Invite link is invalid' });
	});

	it('posts to the route for its kind of link', async () => {
		answering(200, { ok: true, result: {} });

		await realAuth().authenticate('recover', {
			password: 'a password',
			session_name: 'A browser',
			link: '/recover/3b71d0ae5c92',
		});

		expect(fetch).toHaveBeenCalledWith(
			'/auth/recover',
			expect.objectContaining({ method: 'POST' }),
		);
	});
});
