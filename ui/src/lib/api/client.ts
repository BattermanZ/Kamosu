/**
 * The one way a screen talks to Kamosu.
 *
 * The web API is RPC-shaped on purpose — `POST /api/op/<name>` — because that
 * is what keeps both Doors materialising the same Catalogue (ADR 0001). This
 * module carries exactly that: a Transport that performs one call, and a client
 * built by walking the generated declarations so every Operation has a method
 * and no Operation has a hand-written one.
 *
 * A screen never imports `fetch`. It takes a `KamosuClient`, which in the app is
 * the real one and in a test is the Catalogue-derived stand-in — the same
 * methods, the same declared shapes, either way.
 */

import { CATALOGUE, METHOD_NAMES, type KamosuClient, type OperationName } from './catalogue';
import { wentWrong } from '../mistake.svelte';
import { OperationError, type ErrorKind } from './refusal';

// A refusal's own declaration lives next door so that the module saying what a
// *mistake* is can name it without the two importing each other in a circle
// (#98). Re-exported here because this is where a screen looks for it, and one
// way in is worth more than a tidy import graph.
export { OperationError, type ErrorKind };

/** One call to one Operation. Everything above this is generated. */
export type Transport = (operation: OperationName, input: unknown) => Promise<unknown>;

export interface TransportOptions {
	/** The Secret, carried as a bearer token. What it *means* is decided in the Core. */
	secret?: () => string | undefined;
	/** How the request is made. The tests substitute this; the app does not. */
	fetch?: typeof globalThis.fetch;
	/**
	 * Told what each request came to — the response, or what was thrown —
	 * so the app can tell a server out of reach from a refusal (#76).
	 */
	observe?: (outcome: unknown) => void;
}

interface Envelope {
	ok: boolean;
	result?: unknown;
	error?: { kind?: string; message?: string; retry_after_seconds?: number };
}

const KINDS: readonly ErrorKind[] = [
	'unauthorized',
	'unknown_operation',
	'not_found',
	'busy',
	'bad_request',
	'internal',
];

const asKind = (value: unknown): ErrorKind =>
	KINDS.includes(value as ErrorKind) ? (value as ErrorKind) : 'internal';

/** The real Transport: one POST per ask, exactly as an agent at the MCP door makes. */
export function httpTransport(options: TransportOptions = {}): Transport {
	const doFetch = options.fetch ?? globalThis.fetch.bind(globalThis);

	return async (operation, input) => {
		const headers: Record<string, string> = { 'content-type': 'application/json' };
		const secret = options.secret?.();
		if (secret) headers.authorization = `Bearer ${secret}`;

		let response: Response;
		try {
			response = await doFetch(`/api/op/${operation}`, {
				method: 'POST',
				headers,
				body: JSON.stringify(input ?? {}),
			});
		} catch (cause) {
			// The instance was not reached at all — a kitchen on a bad connection,
			// not an Operation that refused. Named apart so a screen can say so.
			options.observe?.(cause);
			throw new OperationError(operation, 'internal', 'Kamosu could not be reached.', {
				cause,
				reached: false,
			});
		}

		options.observe?.(response);
		return readEnvelope(operation, response);
	};
}

/**
 * The answer inside the envelope every Door route answers with — an
 * Operation's, or an out-of-band upload's (ADR 0001) — or the refusal it
 * carries, as the kind the Core named.
 */
export async function readEnvelope(operation: string, response: Response): Promise<unknown> {
	let envelope: Envelope;
	try {
		envelope = (await response.json()) as Envelope;
	} catch {
		throw new OperationError(
			operation,
			'internal',
			`Kamosu answered ${response.status} with something that was not an envelope.`,
			{ reached: false },
		);
	}

	if (!envelope.ok) {
		throw new OperationError(
			operation,
			asKind(envelope.error?.kind),
			envelope.error?.message ?? `${operation} was refused.`,
			{ retryAfterSeconds: envelope.error?.retry_after_seconds },
		);
	}
	return envelope.result;
}

/**
 * Build the client by walking the generated declarations — the same walk the
 * Doors do over the Catalogue in Rust. There is nowhere to hand-write a method
 * for one Operation, which is the property worth having.
 */
export function createClient(transport: Transport): KamosuClient {
	const client: Record<string, (input?: unknown) => Promise<unknown>> = {};
	for (const declaration of CATALOGUE) {
		const name = declaration.name as OperationName;
		client[METHOD_NAMES[name]] = async (input) => {
			try {
				return await transport(name, input ?? {});
			} catch (thrown) {
				// The one place every Operation call passes through, in the app and
				// behind a stand-in alike — so it is where a mistake in Kamosu is
				// caught before the screen re-throws it into nowhere (#98). A
				// refusal is not a mistake and `wentWrong` ignores it. Re-thrown
				// either way: saying so is not handling it, and what the screen
				// does about the call having failed is still the screen's business.
				//
				// Deliberately `await`ed rather than watched from the side with
				// `answer.catch(…)`. That costs no microtask, but attaching any
				// handler marks the promise handled — which would swallow a refusal
				// no screen caught, and so quietly blind the very stray detector
				// `src/testing/setup.ts` adds for this issue. One tick is the
				// cheaper half of that trade.
				wentWrong(thrown);
				throw thrown;
			}
		};
	}
	return client as unknown as KamosuClient;
}
