# Finding English abstract: M4 pre-release filesystem hardening

> Full finding: [docs/agent/findings/m4-pre-release-filesystem-hardening.md](../../agent/findings/m4-pre-release-filesystem-hardening.md).

## Observation

CS-02 M1-M3 has functional Git compatibility for the supported subset, but the M4 pre-release audit found release-readiness gaps in the local-filesystem domain:

- loose object, index, and ref writes need stronger symlink/atomic-write/concurrency protection;
- `.mg/index` entry counts can drive large allocation before validation;
- loose object inflation lacks a decoded-size cap;
- `mg add` can stage repository internals under `.mg/**` / `.git/**`;
- human docs overstate `.mg`/`.git` compatibility and list unsupported `ls-files`.

## Handling

Accepted as input for the M4 release-hardening sprint and consolidated in ADR-0005. The goal is to close filesystem-hardening and documentation-honesty gaps before claiming `0.1.0` readiness.

## Status

`accepted` — 2026-05-13.
