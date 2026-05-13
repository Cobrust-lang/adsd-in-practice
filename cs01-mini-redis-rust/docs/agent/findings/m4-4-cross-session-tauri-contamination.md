---
finding: m4-4-cross-session-tauri-contamination
date: 2026-05-13
case: cs01-mini-redis-rust
severity: P1
status: closed by scope correction
related_adr: 0013-withdrawn
related_f: F1 (sediment), F17 (context fidelity), cross-session contamination
---

# Finding: Cross-session Tauri requirement contaminated cs01 release scope

## Observation

During cs01 M4.3/M4.4, a Tauri desktop packaging requirement was treated as if it belonged to ADSD in Practice. The user clarified on 2026-05-13 that this was sent to the wrong session and is not a requirement for this repository.

The contamination affected:

- README and changelog release-surface claims;
- cs01 local constitution wording;
- ADR/finding indexes and bilingual human docs;
- `web/` package dependencies and Tauri runtime helper code;
- a Tauri source tree and gate script.

## Root cause

The agent accepted a plausible adjacent requirement without revalidating that it belonged to the current repository's strategic scope. Because ADSD execution was running autonomously, the wrong requirement propagated into code, docs, and release-readiness evidence before the user corrected it.

## Handling

M4.4 withdraws the desktop packaging scope and restores cs01 to the valid release surface: Rust RESP server + Axum HTTP/SSE control plane + SvelteKit browser dashboard.

Tauri-specific code, dependencies, gate scripts, and release claims are removed. The previous desktop packaging blocker finding is superseded by this scope-correction finding rather than kept as live release debt.

## Lesson

Autonomous ADSD loops still need a scope-contamination check when a new requirement is orthogonal to the existing case wedge. A plausible requirement can be wrong if it came from another session.
