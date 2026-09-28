/**
 * Puts text on the clipboard, answering whether it went. A browser can refuse
 * (no permission, or no clipboard at all on a plain-HTTP address), and every
 * caller keeps the text on screen to select by hand when it does.
 */
export async function copyText(text: string): Promise<boolean> {
	try {
		await navigator.clipboard.writeText(text);
		return true;
	} catch {
		return false;
	}
}
