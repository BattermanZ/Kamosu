/**
 * The real transport: what a screen gets when Kamosu answers, and what it gets
 * when Kamosu refuses. This is the half of the seam the stand-in stands in for,
 * so it is checked directly — every screen test would otherwise pass while the
 * one path that talks to a real instance was wrong.
 */

import { describe, expect, it, vi } from 'vitest';
import { createClient, httpTransport, OperationError } from './client';

/** A fetch that answers one envelope and records what it was asked. */
function answering(envelope: unknown, status = 200) {
	const calls: { url: string; init: RequestInit }[] = [];
	const fetch = vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
		calls.push({ url: String(url), init: init ?? {} });
		return new Response(JSON.stringify(envelope), {
			status,
			headers: { 'content-type': 'application/json' },
		});
	}) as unknown as typeof globalThis.fetch;
	return { fetch, calls };
}

describe('the http transport', () => {
	it('asks the Operation by name and unwraps what it answered', async () => {
		const { fetch, calls } = answering({
			ok: true,
			result: { version: '0.1.0', setup_complete: false },
		});
		const kamosu = createClient(httpTransport({ fetch }));

		await expect(kamosu.instanceStatus()).resolves.toEqual({
			version: '0.1.0',
			setup_complete: false,
		});
		expect(calls[0].url).toBe('/api/op/instance_status');
		expect(calls[0].init.method).toBe('POST');
		expect(calls[0].init.body).toBe('{}');
	});

	it('carries the Secret as a bearer token, and nothing else about it', async () => {
		const { fetch, calls } = answering({ ok: true, result: { jobs: [] } });
		const kamosu = createClient(httpTransport({ fetch, secret: () => 'a-secret' }));

		await kamosu.listJobs();
		const headers = calls[0].init.headers as Record<string, string>;
		expect(headers.authorization).toBe('Bearer a-secret');
	});

	it('sends no Authorization header when there is no Secret', async () => {
		const { fetch, calls } = answering({ ok: true, result: { jobs: [] } });
		const kamosu = createClient(httpTransport({ fetch }));

		await kamosu.listJobs();
		expect(calls[0].init.headers).not.toHaveProperty('authorization');
	});

	it('turns a refusal into an OperationError carrying the kind the Core chose', async () => {
		const { fetch } = answering(
			{ ok: false, error: { kind: 'unauthorized', message: 'no Person is named' } },
			401,
		);
		const kamosu = createClient(httpTransport({ fetch }));

		await expect(kamosu.listJobs()).rejects.toMatchObject({
			name: 'OperationError',
			kind: 'unauthorized',
			operation: 'list_jobs',
			message: 'no Person is named',
		});
	});

	it('does not invent a kind it does not recognise', async () => {
		const { fetch } = answering({ ok: false, error: { kind: 'sideways', message: 'hm' } }, 500);
		const kamosu = createClient(httpTransport({ fetch }));

		await expect(kamosu.listJobs()).rejects.toMatchObject({ kind: 'internal' });
	});

	it('says the instance could not be reached when the request itself fails', async () => {
		const fetch = vi.fn(async () => {
			throw new TypeError('network down');
		}) as unknown as typeof globalThis.fetch;
		const kamosu = createClient(httpTransport({ fetch }));

		const error = await kamosu.instanceStatus().catch((e: unknown) => e);
		expect(error).toBeInstanceOf(OperationError);
		expect((error as OperationError).message).toMatch(/could not be reached/);
		expect((error as OperationError).cause).toBeInstanceOf(TypeError);
	});
});
