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
	'unauthorized' | 'unknown_operation' | 'not_found' | 'busy' | 'bad_request' | 'internal';

/**
 * An Operation that failed: refused by Kamosu, or — where `reached` is false —
 * never answered by it at all.
 */
export class OperationError extends Error {
	readonly kind: ErrorKind;
	readonly operation: string;
	/**
	 * Whether Kamosu itself answered. False where the request never arrived, or
	 * something standing in front of Kamosu answered for it: the proxy's 502
	 * for a server at home that is off, or a hotel wifi's login page (#76).
	 * What a phone with no network may keep and send later is decided on this
	 * (#77): a refusal will refuse again, and a request that never arrived has
	 * not been asked yet.
	 */
	readonly reached: boolean;

	constructor(
		operation: string,
		kind: ErrorKind,
		message: string,
		options?: ErrorOptions & { reached?: boolean },
	) {
		super(message, options);
		this.name = 'OperationError';
		this.kind = kind;
		this.operation = operation;
		this.reached = options?.reached ?? true;
	}
}

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
	error?: { kind?: string; message?: string };
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
		client[METHOD_NAMES[name]] = (input) => transport(name, input ?? {});
	}
	return client as unknown as KamosuClient;
}
