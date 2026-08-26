/**
 * The whole app is one shell (ADR 0028).
 *
 * SvelteKit's server half is unused — it is taken for its router and layout
 * (ADR 0012) — so nothing is rendered or prerendered on a server. The Rust
 * binary serves `index.html` for every path it has not otherwise claimed, and
 * the router runs in the browser from there.
 */
export const ssr = false;
export const prerender = false;
