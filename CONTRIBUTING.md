# Contributing to ADSD in Practice

Thanks for considering a contribution. This repository is both software and methodology evidence, so contributions must keep implementation, decisions, and bilingual documentation in sync.

## Ground rules

- Read the top-level `CLAUDE.md` and the case-local `CLAUDE.md` before changing a case.
- Do not make irreversible decisions without maintainer sign-off: license, case name, public API freeze, or breaking changes after `0.1.0`.
- If you add a stub, mock, or intentionally simplified implementation, mark it in the README, the commit message, and an ADR if the decision crosses files.

## ADR and finding discipline

- Cross-file design decisions need an ADR in `docs/agent/adr/NNNN-*.md`; start from `_shared/adr-template.md`.
- Negative results, unexpected divergences, and methodology failures go in `docs/agent/findings/*.md`; start from `_shared/finding-template.md`.
- Every ADR and finding must have human abstracts in both `docs/human/zh/` and `docs/human/en/`.
- Keep `last_verified_commit` honest. If a document still makes current claims after a change, re-verify or add an addendum.

## Required gates

For Rust cases, run the case-specific 5 gates before asking for review:

```bash
cargo fmt --manifest-path cs01-mini-redis-rust/Cargo.toml --all -- --check
cargo clippy --manifest-path cs01-mini-redis-rust/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo build --manifest-path cs01-mini-redis-rust/Cargo.toml --workspace --all-targets --locked
cargo test --manifest-path cs01-mini-redis-rust/Cargo.toml --workspace --locked
bash _shared/doc-coverage.sh
```

If the case has frontend assets, also run its frontend gate when available:

```bash
bash cs01-mini-redis-rust/scripts/frontend-gate.sh
```

For future Python/C++ cases, use the matching `_shared/5-gate-*.sh` script plus the case-local gates documented in that case.

## Documentation coverage

`_shared/doc-coverage.sh` is a merge gate, not a suggestion. It checks:

- README and `CLAUDE.md` exist.
- Agent ADRs have required frontmatter.
- Agent findings have required frontmatter.
- Every ADR has zh/en human coverage.
- Every finding has zh/en human abstract files named `finding-<agent-finding-basename>.md`.

Run it from the repository root or from a case directory. Cases without findings directories are skipped gracefully.

## Commit style

Use conventional commits with case scope and Tx tag:

```text
<type>(<scope>): <Tx tag> <subject> (Wave <X.Y>)
```

Example:

```text
docs(cs01): A12.1 complete M4.2 release artifacts (Wave M4.2)
```

Prefer atomic commits that include code, tests, ADR/finding updates, and zh/en docs together. Do not split a behavior change from its documentation mirror.

## Bilingual rule

ADSD in Practice intentionally keeps three documentation tracks:

- `docs/agent/` for dense agent-facing source of truth.
- `docs/human/zh/` for Chinese human readers.
- `docs/human/en/` for English human readers.

If you touch an ADR or finding, update both human tracks in the same commit.
