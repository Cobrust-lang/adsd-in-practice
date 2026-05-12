// Tailwind 4 reads zero-config by default via @tailwindcss/vite. This
// file is intentionally minimal — plugin config (DaisyUI 5) lives in
// `src/app.css` via the Tailwind 4 `@plugin "daisyui"` directive
// (ADR-0008 §Notes acknowledges PostCSS / classic plugins[] array are
// retired in Tailwind 4).
//
// Kept for editor / IntelliSense discoverability; deleting it has no
// runtime impact.
import type { Config } from 'tailwindcss';

export default {
	content: ['./src/**/*.{html,js,svelte,ts}']
} satisfies Config;
