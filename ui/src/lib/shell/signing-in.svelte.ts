/**
 * What the shell knows about whether somebody is in the app, which is what
 * decides whether the shell may draw its navigation: the sidebar on the wide
 * layout (ADR 0044, #194), the tab bar and the gear on the phone (#214). The
 * pages a stranger reads (setup, login, an Invite, a recovery link) get none,
 * so that nothing suggests somebody is in the app before they are.
 *
 * Two facts, because neither is enough alone:
 *
 * - `showing`: the account form is on screen. The form says so itself, since
 *   `/` is Home to a Person and this form to anyone else, and only the answer
 *   to an Operation tells the two apart.
 * - `signedIn`: what that answer was the last time this device heard one. It
 *   covers the moment before the form is drawn, while the question is still
 *   out: without it a stranger opening `/`, a Share Link's `/import` or a
 *   Cookbook Invite would see the navigation come and go. Every screen that
 *   draws the form on a refusal says what it heard, and so does signing out.
 *   Kept between visits, because the Session cookie that would say so cannot
 *   be read from here. It is a hint for drawing and nothing more: who is
 *   signed in is only ever decided by the Core.
 */

const KEY = 'kamosu.signed-in';

function remembered(): boolean | undefined {
	try {
		const kept = localStorage.getItem(KEY);
		return kept === 'yes' ? true : kept === 'no' ? false : undefined;
	} catch {
		// No storage, or none while the app is prerendered: nothing is known.
		return undefined;
	}
}

export const signingIn = $state<{ showing: boolean; signedIn: boolean | undefined }>({
	showing: false,
	signedIn: remembered(),
});

/** An Operation answered a Person, or refused for want of one. */
export function heardWhetherSignedIn(signedIn: boolean): void {
	signingIn.signedIn = signedIn;
	try {
		localStorage.setItem(KEY, signedIn ? 'yes' : 'no');
	} catch {
		// Private browsing, or storage refused: known for this visit only.
	}
}
