// Pure formatting helpers. Kept dependency-free so vitest can run them
// without spinning up jsdom. All numeric inputs are non-negative
// integers (well-formed StatsEvent guarantees this).

/**
 * Format a byte count using binary units (KiB / MiB / GiB).
 *
 * Returns `0 B` for `0`, `1.0 KiB` for `1024`, `1.5 KiB` for `1536`,
 * `1.0 GiB` for `2 ** 30`, etc. Non-integer inputs are tolerated and
 * rounded; NaN/negative inputs fall back to `'0 B'`.
 */
export function formatBytes(n: number): string {
	if (!Number.isFinite(n) || n < 0) {
		return '0 B';
	}
	if (n < 1024) {
		return `${Math.round(n)} B`;
	}
	const units = ['KiB', 'MiB', 'GiB', 'TiB', 'PiB'] as const;
	let value = n / 1024;
	let unitIdx = 0;
	while (value >= 1024 && unitIdx < units.length - 1) {
		value /= 1024;
		unitIdx += 1;
	}
	// One decimal place for sub-10 values, otherwise round to int — same
	// readability shape as `redis-cli INFO` memory output.
	const formatted = value < 10 ? value.toFixed(1) : Math.round(value).toString();
	return `${formatted} ${units[unitIdx]}`;
}

/**
 * Format an uptime in seconds as a compact `1d 2h 3m 4s`-style string.
 *
 * Only the largest 2–3 non-zero units are shown to keep the dashboard
 * readable. Sub-second precision is dropped (backend emits whole
 * seconds anyway). `0` → `'0s'`.
 */
export function formatUptime(secs: number): string {
	if (!Number.isFinite(secs) || secs < 0) {
		return '0s';
	}
	const total = Math.floor(secs);
	if (total === 0) {
		return '0s';
	}
	const days = Math.floor(total / 86_400);
	const hours = Math.floor((total % 86_400) / 3600);
	const minutes = Math.floor((total % 3600) / 60);
	const seconds = total % 60;

	const parts: string[] = [];
	if (days > 0) parts.push(`${days}d`);
	if (hours > 0) parts.push(`${hours}h`);
	if (minutes > 0) parts.push(`${minutes}m`);
	if (seconds > 0) parts.push(`${seconds}s`);

	// Show at most the three highest-order units so 1d 2h 3m 4s
	// collapses to "1d 2h 3m" — dashboard readability.
	return parts.slice(0, 3).join(' ');
}

/**
 * Format a TTL value (as returned by `KeyInfo.ttl_secs`) for display.
 *
 * - `-1` (Redis "no TTL") → `'永久'`
 * - `0` (just expired / about to expire) → `'即将过期'`
 * - positive seconds → human-friendly (`'30s'` / `'5m 30s'` / `'1h'`)
 *
 * Negative values other than `-1` (e.g. `-2` for "key missing") map to
 * `'—'` so the table renders a dash rather than a raw number.
 */
export function formatTtl(secs: number): string {
	if (secs === -1) {
		return '永久';
	}
	if (!Number.isFinite(secs) || secs < 0) {
		return '—';
	}
	if (secs === 0) {
		return '即将过期';
	}
	// Reuse the uptime formatter — same compact shape; capped at 3
	// units which is plenty for any sensible TTL.
	return formatUptime(secs);
}
