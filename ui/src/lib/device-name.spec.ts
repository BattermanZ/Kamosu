import { describe, expect, it } from 'vitest';
import { deviceName } from './device-name';

/**
 * Real user agents, as the browsers send them in September 2026. What comes
 * back is all a Session is called until somebody renames it (#114), so each
 * one has to be told apart from the others on the same Person's list.
 */
describe('naming a Session for its device', () => {
	const cases: [string, string, number, string | undefined][] = [
		[
			'Safari on an iPhone',
			'Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.5 Mobile/15E148 Safari/604.1',
			5,
			'Safari · iPhone',
		],
		[
			'Chrome on an iPhone, which is Safari underneath and says so',
			'Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/139.0.7258.76 Mobile/15E148 Safari/604.1',
			5,
			'Chrome · iPhone',
		],
		[
			'Firefox on an iPhone',
			'Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) FxiOS/142.0 Mobile/15E148 Safari/605.1.15',
			5,
			'Firefox · iPhone',
		],
		[
			'Safari on an iPad, which claims to be a Mac',
			'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.5 Safari/605.1.15',
			5,
			'Safari · iPad',
		],
		[
			'Safari on a Mac, which has no touch screen',
			'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.5 Safari/605.1.15',
			0,
			'Safari · Mac',
		],
		[
			'Chrome on Android',
			'Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Mobile Safari/537.36',
			5,
			'Chrome · Android',
		],
		[
			'Samsung Internet, which also names Chrome',
			'Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) SamsungBrowser/28.0 Chrome/130.0.0.0 Mobile Safari/537.36',
			5,
			'Samsung Internet · Android',
		],
		[
			'Edge on Windows, which also names Chrome and Safari',
			'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36 Edg/139.0.0.0',
			0,
			'Edge · Windows',
		],
		[
			'Opera on Windows',
			'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36 OPR/122.0.0.0',
			0,
			'Opera · Windows',
		],
		[
			'Firefox on Linux',
			'Mozilla/5.0 (X11; Linux x86_64; rv:142.0) Gecko/20100101 Firefox/142.0',
			0,
			'Firefox · Linux',
		],
		[
			'Chrome on a Chromebook',
			'Mozilla/5.0 (X11; CrOS x86_64 14541.0.0) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36',
			0,
			'Chrome · ChromeOS',
		],
		['only the browser known', 'Mozilla/5.0 (compatible) Firefox/142.0', 0, 'Firefox'],
		['nothing known at all', 'curl/8.5.0', 0, undefined],
	];

	it.each(cases)('%s', (_, userAgent, touchPoints, expected) => {
		expect(deviceName(userAgent, touchPoints)).toBe(expected);
	});

	it('names no version and no model, only the browser and the kind of device', () => {
		const named = deviceName(
			'Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Mobile Safari/537.36',
			5,
		);
		expect(named).toBe('Chrome · Android');
	});
});
