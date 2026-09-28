import { vi } from 'vitest';

/**
 * What Chrome sends when Kamosu could be installed (#173), faked: nobody on
 * the project has an Android phone. `answer` is what the person does with the
 * browser's dialog.
 */
export function offerToInstall(answer: 'accepted' | 'dismissed') {
	const offer = anInstallOffer(answer);
	dispatchEvent(offer.event);
	return offer;
}

/** The same offer, not yet sent: for one caught before the app started. */
export function anInstallOffer(answer: 'accepted' | 'dismissed') {
	const event = new Event('beforeinstallprompt', { cancelable: true });
	const prompt = vi.fn(async () => {});
	Object.assign(event, { prompt, userChoice: Promise.resolve({ outcome: answer }) });
	return { event, prompt };
}
