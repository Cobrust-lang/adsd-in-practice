# Finding English abstract: M4.4 cross-session Tauri requirement contamination

> Full finding: [docs/agent/findings/m4-4-cross-session-tauri-contamination.md](../../agent/findings/m4-4-cross-session-tauri-contamination.md).

## Observation

cs01 M4.3/M4.4 incorrectly treated Tauri desktop packaging as this repository's release requirement. The user clarified on 2026-05-13 that the request was sent to the wrong session and does not belong to ADSD in Practice / cs01.

The contamination entered README, CHANGELOG, local CLAUDE, ADR/finding indexes, human docs, `web/` dependencies, a Tauri runtime helper, `web/src-tauri/`, and the gate script.

## Handling

M4.4 withdraws the desktop packaging scope and restores cs01's release surface to: Rust RESP server + Axum HTTP/SSE control plane + SvelteKit browser dashboard.

Tauri-specific code, dependencies, gate scripts, and release claims are removed. The earlier Tauri packaging blocker is no longer live release debt; this finding records it as cross-session scope contamination.

## Status

`closed by scope correction` — 2026-05-13.
