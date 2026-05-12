// ADR-0008: SPA mode. Disable SSR and prerender so adapter-static
// produces only a fallback index.html and the app renders entirely in
// the browser. M4 will rust-embed the built `build/` directory.
export const prerender = false;
export const ssr = false;
export const trailingSlash = 'never';
