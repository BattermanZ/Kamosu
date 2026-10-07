/**
 * What a new Session is called: the browser and the kind of device, read off
 * what the browser says about itself (#114). The choice of 23 September
 * 2026 (option B) — named without asking, renamable afterwards from Settings.
 *
 * Deliberately coarse. No version and no model, so the name tells your iPhone
 * from your laptop without cataloguing either. The separator is not a word, so
 * the name reads the same in every Reading Language it is later shown in.
 *
 * The order of each list matters: a browser names the ones it descends from
 * as well as itself (Edge says Chrome and Safari too), so the most specific
 * name is looked for first.
 */
const BROWSERS: [RegExp, string][] = [
	[/\bEdg(?:e|iOS|A)?\//, 'Edge'],
	[/\bOPR\//, 'Opera'],
	[/\bSamsungBrowser\//, 'Samsung Internet'],
	[/\b(?:Firefox|FxiOS)\//, 'Firefox'],
	[/\b(?:Chrome|CriOS)\//, 'Chrome'],
	[/\bVersion\/[\d.]+.*\bSafari\//, 'Safari'],
];

const DEVICES: [RegExp, string][] = [
	[/\biPhone\b/, 'iPhone'],
	[/\biPad\b/, 'iPad'],
	[/\bAndroid\b/, 'Android'],
	[/\bCrOS\b/, 'ChromeOS'],
	[/\bWindows\b/, 'Windows'],
	[/\bMacintosh\b/, 'Mac'],
	[/\bLinux\b/, 'Linux'],
];

/**
 * @param touchPoints `navigator.maxTouchPoints`. An iPad has described itself
 * as a Mac since iPadOS 13, and a touch screen is what gives it away.
 */
export function deviceName(userAgent: string, touchPoints: number): string | undefined {
	const browser = BROWSERS.find(([pattern]) => pattern.test(userAgent))?.[1];
	let device = DEVICES.find(([pattern]) => pattern.test(userAgent))?.[1];
	if (device === 'Mac' && touchPoints > 1) device = 'iPad';
	return [browser, device].filter(Boolean).join(' · ') || undefined;
}
