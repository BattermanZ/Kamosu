/**
 * What a refusal is.
 *
 * The one line every screen draws is between a refusal the Core meant to send
 * and a mistake in Kamosu itself (#98), so both sides of that line need to name
 * this class: the client that makes the calls, and the module that says what a
 * mistake means. It lives on its own so those two do not have to import each
 * other in a circle.
 *
 * **Screens keep importing it from `./client`**, which re-exports it. There is
 * still one way a screen reaches Kamosu, and this is not a second one.
 */

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
	/**
	 * How many seconds to wait before asking again, where Kamosu said: a
	 * sign-in tried while its name still waits out wrong passwords (#138).
	 */
	readonly retryAfterSeconds: number | undefined;
	/**
	 * Which refusal this is, where the Core names it for a screen to say in
	 * its own words: `pdf_has_no_text` for a scanned PDF (#176). The message
	 * is the Core's English sentence, so a screen that knows the reason says
	 * it in the reader's Language instead.
	 */
	readonly reason: string | undefined;

	constructor(
		operation: string,
		kind: ErrorKind,
		message: string,
		options?: ErrorOptions & { reached?: boolean; retryAfterSeconds?: number; reason?: string },
	) {
		super(message, options);
		this.name = 'OperationError';
		this.kind = kind;
		this.operation = operation;
		this.reached = options?.reached ?? true;
		this.retryAfterSeconds = options?.retryAfterSeconds;
		this.reason = options?.reason;
	}
}
