import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { defineConfig } from 'vitest/config';
import tailwindcss from '@tailwindcss/vite';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';

/// The backend, in development: two processes side by side (ADR 0028). Vite
/// serves the interface and proxies everything the binary owns to it, so a
/// browser only ever talks to one origin and no CORS question exists.
const BACKEND = 'http://127.0.0.1:5266';

export default defineConfig({
	plugins: [
		tailwindcss(),
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},

			// adapter-static with a fallback: SvelteKit's server half is unused
			// (ADR 0012). The Rust binary serves `build/` — one artefact, one version.
			adapter: adapter({ fallback: 'index.html', strict: false })
		}),

		// Paraglide compiles every phrase into a function, so a misspelt key is a
		// build error rather than a blank button in a kitchen (ADR 0012). The
		// locale is a setting, not a URL segment: no `/fr/…` routes exist, which
		// is what lets the whole app be one prerendered shell.
		paraglideVitePlugin({
			project: './project.inlang',
			outdir: './src/lib/paraglide',
			strategy: ['localStorage', 'preferredLanguage', 'baseLocale'],
			emitTsDeclarations: true
		})
	],

	server: {
		// Fixed here, not picked per session: vite's own default 5173 is taken on
		// the dev host. `strictPort` makes a silent fallback to another port
		// impossible — a wrong port must fail loudly, not quietly work elsewhere.
		port: 5174,
		strictPort: true,
		host: '0.0.0.0',
		// The TLS test proxy serves this exact hostname to a browser. Keep the
		// allowlist narrow: accepting arbitrary Host headers enables DNS rebinding.
		allowedHosts: ['kamosu-dev.batterlan.cc'],
		// Everything the binary owns, proxied so dev and production serve the same
		// URLs. `/favicon.svg` is named by app.html; the two root PNGs are asked
		// for by Safari's own convention. All three would 404 in dev otherwise.
		proxy: Object.fromEntries(
			['/api', '/auth', '/assets', '/favicon.svg', '/favicon-32.png', '/apple-touch-icon.png'].map(
				(path) => [path, { target: BACKEND, changeOrigin: false }]
			)
		)
	},

	test: {
		expect: { requireAssertions: true },
		environment: 'jsdom',
		setupFiles: ['./src/testing/setup.ts'],
		include: ['src/**/*.{test,spec}.{js,ts}']
	},

	// Screens are exercised as a browser would run them, so the browser build of
	// Svelte is what the tests resolve — the server build cannot mount anything.
	resolve: process.env.VITEST ? { conditions: ['browser'] } : undefined
});
