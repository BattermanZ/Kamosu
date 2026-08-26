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

/** The kinds of refusal the Core distinguishes, as the web door names them. */
export type ErrorKind =
	| 'unauthorized'
	| 'unknown_operation'
	| 'not_found'
	| 'busy'
	| 'bad_request'
	| 'internal';

/** An Operation that was reached and refused. Not a network failure. */
export class OperationError extends Error {
	readonly kind: ErrorKind;
	readonly operation: string;

	constructor(operation: string, kind: ErrorKind, message: string, options?: ErrorOptions) {
		super(message, options);
		this.name = 'OperationError';
		this.kind = kind;
		this.operation = operation;
	}
}

/** One call to one Operation. Everything above this is generated. */
export type Transport = (operation: OperationName, input: unknown) => Promise<unknown>;

export interface TransportOptions {
	/** The Secret, carried as a bearer token. What it *means* is decided in the Core. */
	secret?: () => string | undefined;
	/** How the request is made. The tests substitute this; the app does not. */
	fetch?: typeof globalThis.fetch;
}

interface Envelope {
	ok: boolean;
	result?: unknown;
	error?: { kind?: string; message?: string };
}

const KINDS: readonly ErrorKind[] = [
	'unauthorized',
	'unknown_operation',
	'not_found',
	'busy',
	'bad_request',
	'internal'
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
				body: JSON.stringify(input ?? {})
			});
		} catch (cause) {
			// The instance was not reached at all — a kitchen on a bad connection,
			// not an Operation that refused. Named apart so a screen can say so.
			throw new OperationError(operation, 'internal', 'Kamosu could not be reached.', {
				cause
			});
		}

		let envelope: Envelope;
		try {
			envelope = (await response.json()) as Envelope;
		} catch {
			throw new OperationError(
				operation,
				'internal',
				`Kamosu answered ${response.status} with something that was not an envelope.`
			);
		}

		if (!envelope.ok) {
			throw new OperationError(
				operation,
				asKind(envelope.error?.kind),
				envelope.error?.message ?? `${operation} was refused.`
			);
		}
		return envelope.result;
	};
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
		client[METHOD_NAMES[name]] = (input) => transport(name, input ?? {});
	}
	return client as unknown as KamosuClient;
}
