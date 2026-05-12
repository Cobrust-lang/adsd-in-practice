// ADR-0008 §Decision: adapter-static + SPA fallback (no SSR; web/ ships
// as a pure SPA so M4 rust-embed can serve the built `build/` directory
// from the redis-server binary).
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		adapter: adapter({
			// `fallback: 'index.html'` makes SvelteKit emit a single
			// index.html for all routes — required for SPA mode under
			// a static file server (rust-embed in M4, vite dev in M2.2).
			fallback: 'index.html',
			pages: 'build',
			assets: 'build',
			strict: true
		})
	}
};

export default config;
