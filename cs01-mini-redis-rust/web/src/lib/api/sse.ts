// Tiny EventSource wrapper. We intentionally use the browser-native
// EventSource constructor (ADR-0008 §"EventSource 用法") rather than a
// polyfill — Safari/Chrome/Firefox all ship it. The wrapper centralises
// the JSON-parse safety net so callers can't accidentally `JSON.parse`
// malformed data and crash the whole page.

import type { KeyInfo, PubsubSnapshot, StatsEvent } from './types';

/**
 * Parse a single SSE `data:` line into a typed StatsEvent.
 *
 * Returns `null` on any failure (bad JSON, missing fields, wrong
 * types). The caller is expected to ignore null and wait for the next
 * frame — SSE streams self-resync after a bad frame.
 */
export function parseStatsLine(raw: string): StatsEvent | null {
	if (typeof raw !== 'string' || raw.length === 0) {
		return null;
	}
	let value: unknown;
	try {
		value = JSON.parse(raw);
	} catch {
		return null;
	}
	if (typeof value !== 'object' || value === null) {
		return null;
	}
	const v = value as Record<string, unknown>;
	if (
		typeof v.connections_active !== 'number' ||
		typeof v.commands_total !== 'number' ||
		typeof v.keys_active !== 'number' ||
		typeof v.mem_value_bytes !== 'number' ||
		typeof v.uptime_secs !== 'number'
	) {
		return null;
	}
	return {
		connections_active: v.connections_active,
		commands_total: v.commands_total,
		keys_active: v.keys_active,
		mem_value_bytes: v.mem_value_bytes,
		uptime_secs: v.uptime_secs
	};
}

/**
 * Parse a single SSE `data:` line into a typed `PubsubSnapshot`.
 *
 * Returns `null` on any failure (bad JSON, missing `channels` field,
 * non-array shape, etc.). Individual rows that don't match the schema
 * are dropped (so a single malformed entry doesn't lose the whole
 * snapshot). Empty `channels` is a valid output: `{ channels: [] }`.
 */
export function parsePubsubLine(raw: string): PubsubSnapshot | null {
	if (typeof raw !== 'string' || raw.length === 0) {
		return null;
	}
	let value: unknown;
	try {
		value = JSON.parse(raw);
	} catch {
		return null;
	}
	if (typeof value !== 'object' || value === null) {
		return null;
	}
	const v = value as Record<string, unknown>;
	if (!Array.isArray(v.channels)) {
		return null;
	}
	const channels: PubsubSnapshot['channels'] = [];
	for (const item of v.channels) {
		if (typeof item !== 'object' || item === null) {
			continue;
		}
		const r = item as Record<string, unknown>;
		if (typeof r.name !== 'string' || typeof r.subscribers !== 'number') {
			continue;
		}
		channels.push({ name: r.name, subscribers: r.subscribers });
	}
	return { channels };
}

/**
 * Parse a single SSE `data:` line into a typed `KeyInfo[]`.
 *
 * Returns `null` on any failure. Individual entries that don't match
 * the schema are dropped (so a single bad row doesn't lose the whole
 * snapshot).
 */
export function parseKeysLine(raw: string): KeyInfo[] | null {
	if (typeof raw !== 'string' || raw.length === 0) {
		return null;
	}
	let value: unknown;
	try {
		value = JSON.parse(raw);
	} catch {
		return null;
	}
	if (!Array.isArray(value)) {
		return null;
	}
	const out: KeyInfo[] = [];
	for (const item of value) {
		if (typeof item !== 'object' || item === null) {
			continue;
		}
		const r = item as Record<string, unknown>;
		if (
			typeof r.key !== 'string' ||
			(r.type !== 'string' && r.type !== 'none') ||
			typeof r.ttl_secs !== 'number'
		) {
			continue;
		}
		out.push({ key: r.key, type: r.type, ttl_secs: r.ttl_secs });
	}
	return out;
}
