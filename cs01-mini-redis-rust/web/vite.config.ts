// ADR-0008: vite proxy `/api -> http://localhost:6381`, no CORS headers
// needed (proxy makes browser see same-origin), Tailwind 4 via the
// `@tailwindcss/vite` plugin (Tailwind 4 dropped the separate PostCSS
// plugin — see ADR-0008 §Notes).
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
// Use vitest's `defineConfig` (which re-exports vite's plus a typed
// `test` field) so we can colocate the vitest config without a
// separate `vitest.config.ts`.
import { defineConfig } from 'vitest/config';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	server: {
		port: 5173,
		strictPort: false,
		proxy: {
			'/api': {
				target: 'http://localhost:6381',
				changeOrigin: true,
				// SSE: must not buffer. Vite 8 streams chunks through
				// http-proxy by default; we explicitly disable websocket
				// upgrades because /api/* is plain HTTP/1.1 SSE.
				ws: false
			}
		}
	},
	test: {
		environment: 'jsdom',
		include: ['src/**/*.{test,spec}.{js,ts}'],
		globals: false
	}
});
