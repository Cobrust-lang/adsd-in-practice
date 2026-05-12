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
