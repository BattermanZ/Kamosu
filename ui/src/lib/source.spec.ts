import { describe, expect, it } from 'vitest';
import { hostOf, tappableLink } from './source';

describe('a Source link offered to tap (#152)', () => {
	it('offers an http or https address as written', () => {
		expect(tappableLink('https://www.allrecipes.com/recipe/1/')).toBe(
			'https://www.allrecipes.com/recipe/1/',
		);
		expect(tappableLink(' http://example.com ')).toBe('http://example.com');
	});

	it('offers nothing that is not a web address', () => {
		for (const link of ['javascript:alert(1)', 'data:text/html,hi', 'allrecipes', '', null]) {
			expect(tappableLink(link)).toBeNull();
		}
	});
});

describe('naming a Source by its link (#152)', () => {
	// The same answers `host()` in src/crouton.rs gives, so a Source named by
	// the importer and one named on the writing screen read alike.
	it('takes the host without a leading www.', () => {
		expect(hostOf('https://www.bonappetit.com/recipe/x')).toBe('bonappetit.com');
		expect(hostOf('https://japan.recipetineats.com/katsu?x=1')).toBe('japan.recipetineats.com');
		expect(hostOf('youtu.be/abc')).toBe('youtu.be');
	});

	it('names nothing when the link has no host', () => {
		expect(hostOf('https:///path')).toBeNull();
	});
});
