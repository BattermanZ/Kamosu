/**
 * The stand-in's own guarantee: a test may lie about the values, never about
 * the shape. Everything else in the screen-seam harness rests on this, so it is
 * checked directly rather than only through the screens that use it.
 */

import { describe, expect, it } from 'vitest';
import { standIn } from './stand-in';
import { OperationError } from './client';
import { CATALOGUE, METHOD_NAMES } from './catalogue';

describe('the Catalogue-derived stand-in', () => {
	it('answers what the Catalogue declares', async () => {
		const kamosu = standIn({
			instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 },
		});
		await expect(kamosu.client.instanceStatus()).resolves.toEqual({
			version: '0.1.0',
			setup_complete: true,
			password_minimum: 15,
		});
	});

	it('refuses an answer that is missing a declared field', async () => {
		// `version` is required by instance_status's output schema.
		const kamosu = standIn({
			instance_status: { setup_complete: true, password_minimum: 15 } as never,
		});
		await expect(kamosu.client.instanceStatus()).rejects.toThrow(/missing version/);
	});

	it('refuses an answer carrying a field the Catalogue does not declare', async () => {
		const kamosu = standIn({
			instance_status: {
				version: '0.1.0',
				setup_complete: true,
				password_minimum: 15,
				kitchen: 'home',
			} as never,
		});
		await expect(kamosu.client.instanceStatus()).rejects.toThrow(/does not declare/);
	});

	it('refuses a value of the wrong type', async () => {
		const kamosu = standIn({
			instance_status: { version: '0.1.0', setup_complete: 'yes', password_minimum: 15 } as never,
		});
		await expect(kamosu.client.instanceStatus()).rejects.toThrow(/declares boolean/);
	});

	it('refuses a status outside the ones a Job can be in', async () => {
		const kamosu = standIn({
			get_job: {
				id: 'j1',
				operation: 'probe_job',
				status: 'nearly' as never,
				progress: {},
				result: null,
				error: null,
				errorCode: null,
				created_at: 't',
				updated_at: 't',
			},
		});
		await expect(kamosu.client.getJob({ job_id: 'j1' })).rejects.toThrow(
			/which the Catalogue does not allow/,
		);
	});

	it('carries a refusal through as the Core would', async () => {
		const kamosu = standIn({ list_jobs: { refuse: 'unauthorized' } });
		await expect(kamosu.client.listJobs()).rejects.toBeInstanceOf(OperationError);
		await expect(kamosu.client.listJobs()).rejects.toMatchObject({ kind: 'unauthorized' });
	});

	it('exposes a method for every Operation in the Catalogue, and no others', () => {
		const kamosu = standIn();
		// Named from the generated METHOD_NAMES, not from a rule re-derived here:
		// a test that re-implements the generator can only agree with its bugs.
		expect(Object.keys(kamosu.client).sort()).toEqual(
			CATALOGUE.map((declaration) => METHOD_NAMES[declaration.name]).sort(),
		);
	});
});
