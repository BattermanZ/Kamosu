/**
 * The cards a person has put away on this device (#76). Remembered here and
 * not on the account, because what they describe — this connection, this
 * phone's Home Screen — is about the device.
 */

export type PutAway = 'insecure' | 'install';

const KEY = 'kamosu.put-away';

function read(): Partial<Record<PutAway, true>> {
	try {
		return JSON.parse(localStorage.getItem(KEY) ?? '{}') as Partial<Record<PutAway, true>>;
	} catch {
		return {};
	}
}

export function wasPutAway(card: PutAway): boolean {
	return read()[card] === true;
}

export function putAway(card: PutAway): void {
	try {
		localStorage.setItem(KEY, JSON.stringify({ ...read(), [card]: true }));
	} catch {
		// Storage refused: the card is put away for this visit only.
	}
}
