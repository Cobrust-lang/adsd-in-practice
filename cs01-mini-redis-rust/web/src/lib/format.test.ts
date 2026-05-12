import { describe, expect, it } from 'vitest';
import { parseKeysLine, parsePubsubLine, parseStatsLine } from './api/sse';
import { formatBytes, formatTtl, formatUptime } from './format';

describe('formatBytes', () => {
	it('renders zero as "0 B"', () => {
		expect(formatBytes(0)).toBe('0 B');
	});
	it('keeps sub-KiB values as raw bytes', () => {
		expect(formatBytes(512)).toBe('512 B');
	});
	it('renders 1024 as "1.0 KiB"', () => {
		expect(formatBytes(1024)).toBe('1.0 KiB');
	});
	it('renders 1536 as "1.5 KiB"', () => {
		expect(formatBytes(1536)).toBe('1.5 KiB');
	});
	it('renders 1 MiB as "1.0 MiB"', () => {
		expect(formatBytes(1024 * 1024)).toBe('1.0 MiB');
	});
	it('renders 1 GiB as "1.0 GiB"', () => {
		expect(formatBytes(2 ** 30)).toBe('1.0 GiB');
	});
	it('falls back to "0 B" on negatives/NaN', () => {
		expect(formatBytes(-1)).toBe('0 B');
		expect(formatBytes(Number.NaN)).toBe('0 B');
	});
});

describe('formatUptime', () => {
	it('renders 0 as "0s"', () => {
		expect(formatUptime(0)).toBe('0s');
	});
	it('renders 1h 1m 1s for 3661s', () => {
		expect(formatUptime(3661)).toBe('1h 1m 1s');
	});
	it('renders 1d for exactly 86400s', () => {
		expect(formatUptime(86_400)).toBe('1d');
	});
	it('renders 1d 1h 1m for 86400 + 3661 = 90061s (truncates to 3 units)', () => {
		// 90061s = 1d 1h 1m 1s -> truncated to "1d 1h 1m"
		expect(formatUptime(90_061)).toBe('1d 1h 1m');
	});
	it('falls back to "0s" on negatives', () => {
		expect(formatUptime(-5)).toBe('0s');
	});
});

describe('formatTtl', () => {
	it('renders -1 (no TTL) as "永久"', () => {
		expect(formatTtl(-1)).toBe('永久');
	});
	it('renders 0 as "即将过期"', () => {
		expect(formatTtl(0)).toBe('即将过期');
	});
	it('renders 60s as "1m"', () => {
		expect(formatTtl(60)).toBe('1m');
	});
	it('renders 3661s as "1h 1m 1s"', () => {
		expect(formatTtl(3661)).toBe('1h 1m 1s');
	});
	it('renders bad negatives (other than -1) as "—"', () => {
		expect(formatTtl(-2)).toBe('—');
	});
});

describe('parseStatsLine', () => {
	it('parses a valid stats payload', () => {
		const json = JSON.stringify({
			connections_active: 3,
			commands_total: 1024,
			keys_active: 42,
			mem_value_bytes: 2048,
			uptime_secs: 300
		});
		const parsed = parseStatsLine(json);
		expect(parsed).toEqual({
			connections_active: 3,
			commands_total: 1024,
			keys_active: 42,
			mem_value_bytes: 2048,
			uptime_secs: 300
		});
	});
	it('rejects malformed JSON', () => {
		expect(parseStatsLine('not-json')).toBeNull();
	});
	it('rejects missing fields', () => {
		expect(parseStatsLine(JSON.stringify({ connections_active: 1 }))).toBeNull();
	});
	it('rejects empty input', () => {
		expect(parseStatsLine('')).toBeNull();
	});
});

describe('parseKeysLine', () => {
	it('parses a valid keys array', () => {
		const json = JSON.stringify([
			{ key: 'foo', type: 'string', ttl_secs: -1 },
			{ key: 'bar', type: 'string', ttl_secs: 60 }
		]);
		const parsed = parseKeysLine(json);
		expect(parsed).toEqual([
			{ key: 'foo', type: 'string', ttl_secs: -1 },
			{ key: 'bar', type: 'string', ttl_secs: 60 }
		]);
	});
	it('rejects non-array payloads', () => {
		expect(parseKeysLine(JSON.stringify({ key: 'foo' }))).toBeNull();
	});
	it('drops individual malformed entries but keeps valid ones', () => {
		const json = JSON.stringify([
			{ key: 'foo', type: 'string', ttl_secs: -1 },
			{ key: 'bar', type: 'WRONG_TYPE', ttl_secs: 60 },
			{ key: 42, type: 'string', ttl_secs: 10 }
		]);
		const parsed = parseKeysLine(json);
		expect(parsed).toEqual([{ key: 'foo', type: 'string', ttl_secs: -1 }]);
	});
	it('rejects malformed JSON', () => {
		expect(parseKeysLine('not-json')).toBeNull();
	});
});

describe('parsePubsubLine', () => {
	it('parses a valid pubsub snapshot', () => {
		const json = JSON.stringify({
			channels: [
				{ name: 'alpha', subscribers: 0 },
				{ name: 'news', subscribers: 3 }
			]
		});
		const parsed = parsePubsubLine(json);
		expect(parsed).toEqual({
			channels: [
				{ name: 'alpha', subscribers: 0 },
				{ name: 'news', subscribers: 3 }
			]
		});
	});
	it('accepts the empty-channels shape (never null)', () => {
		expect(parsePubsubLine(JSON.stringify({ channels: [] }))).toEqual({ channels: [] });
	});
	it('rejects malformed JSON', () => {
		expect(parsePubsubLine('not-json')).toBeNull();
	});
	it('rejects payloads missing the channels field', () => {
		expect(parsePubsubLine(JSON.stringify({ other: 1 }))).toBeNull();
	});
	it('drops malformed entries but keeps valid ones', () => {
		const json = JSON.stringify({
			channels: [
				{ name: 'ok', subscribers: 1 },
				{ name: 123, subscribers: 2 }, // wrong type for name
				{ name: 'noNumber', subscribers: 'three' }, // wrong type for count
				{ name: 'good', subscribers: 7 }
			]
		});
		const parsed = parsePubsubLine(json);
		expect(parsed).toEqual({
			channels: [
				{ name: 'ok', subscribers: 1 },
				{ name: 'good', subscribers: 7 }
			]
		});
	});
	it('rejects null payload', () => {
		expect(parsePubsubLine('null')).toBeNull();
	});
	it('rejects empty input', () => {
		expect(parsePubsubLine('')).toBeNull();
	});
});
