# Finding M1.1: P9 missed shared doc-coverage

## Abstract

During M1.1, the P9 implementation agent reported that no doc-coverage script existed. The script did exist at the repository root as `_shared/doc-coverage.sh`; the agent only searched the case-local tree and missed shared tooling.

## Impact

This is an ADSD F17 sub-agent KPI/self-report fidelity risk: an agent saying “checked” is not the same as verified fact. P10/P9 gatekeeping must rerun the 5 gates instead of trusting completion reports alone.

## Mitigation / lesson

- Persistent memory now records the correct invocation from a case directory: `bash ../_shared/doc-coverage.sh`.
- P9/P10 gatekeeping must run doc-coverage directly.
- M4.2 extends `_shared/doc-coverage.sh` so finding zh/en mirrors are enforced as well.

## Cross-references

- Agent finding: [`../../agent/findings/m1-1-p9-missed-shared-doc-coverage.md`](../../agent/findings/m1-1-p9-missed-shared-doc-coverage.md)
- Shared gate: [`../../../../_shared/doc-coverage.sh`](../../../../_shared/doc-coverage.sh)
