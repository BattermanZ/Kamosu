/**
 * A recipe's Source: the words it is credited to, and the link it came from
 * (#152).
 *
 * `parse_source` stores any string as the link, and a recipe can arrive from
 * anyone, so a link is only offered to tap when it is an ordinary web address.
 * Anything else (`javascript:`, `data:`, a bare word) is drawn as the plain
 * line it was before #152. The Content-Security-Policy would already stop a
 * `javascript:` link from running; this is so nothing depends on it alone.
 */
export function tappableLink(link: string | null | undefined): string | null {
	const trimmed = link?.trim() ?? '';
	// The same rule as `web_link` in `src/share_page.rs`, so the recipe page and
	// the Share Link page offer exactly the same links: a scheme of http or
	// https, `://`, and something after it.
	return /^https?:\/\/./i.test(trimmed) ? trimmed : null;
}

/**
 * What a Source is called when the cook gave only its link: the link's host,
 * without a leading `www.`. The same rule the Crouton importer applies to an
 * export with a link and no name (`host()` in `src/crouton.rs`), so a Source
 * named either way reads the same.
 */
export function hostOf(link: string): string | null {
	const rest = link.includes('://') ? link.slice(link.indexOf('://') + 3) : link;
	const host = rest.split(/[/?#]/)[0].replace(/^www\./, '');
	return host === '' ? null : host;
}
