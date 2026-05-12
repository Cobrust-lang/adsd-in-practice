// ADR-0008 §TypeScript: hand-written types whose field names MUST
// match the wire schema locked in ADR-0007 §Q5 exactly. The backend
// emits these via `serde_json::to_string(&StatsSnapshot)` (snake_case),
// and KeyJson uses `#[serde(rename = "type")]` to remap `kind` -> "type"
// on the wire.  Any drift here = SSE payload mismatch = silent UI bug.

/** One SSE `event: stats` frame's JSON payload. */
export interface StatsEvent {
	connections_active: number;
	commands_total: number;
	keys_active: number;
	mem_value_bytes: number;
	uptime_secs: number;
}

/** One element of the SSE `event: keys` JSON array. */
export interface KeyInfo {
	key: string;
	type: 'string' | 'none';
	/** `-1` = no TTL set; `>= 0` = seconds remaining (round-half-up,
	 * see backend commit `0800d86`). */
	ttl_secs: number;
}

/** One channel row in the `/api/pubsub` SSE payload (ADR-0009 §Q7). */
export interface PubsubChannel {
	name: string;
	/** Current subscriber count. May be 0 because M3.1 deliberately
	 * does NOT GC empty channels (the entry stays in the dashboard
	 * snapshot until M4 release-readiness adds eviction). */
	subscribers: number;
}

/** One SSE `event: pubsub` frame's JSON payload. */
export interface PubsubSnapshot {
	/** Sorted by `name` ascending — the backend `sort_by(|a, b| ...)`
	 * happens in `Store::pubsub_snapshot`. Empty array, never null. */
	channels: PubsubChannel[];
}
