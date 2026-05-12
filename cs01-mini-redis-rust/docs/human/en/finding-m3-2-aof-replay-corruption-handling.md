# Finding M3.2: AOF replay corrupt-tail handling

## Abstract

ADR-0010 chose the M3.2 AOF replay policy: when `Frame::parse` fails, log a warning, stop replay, return the count of successfully replayed commands, and continue server startup. The implementation does not automatically truncate the corrupt tail; later appends continue at the end of the file.

This differs from a production Redis operational toolchain: there is no `redis-check-aof`-style repair tool and no refuse-to-start policy. If tail corruption combines with non-idempotent commands such as `INCR`/`DECR`, restart-time counter drift or repeated warnings are possible.

## Why accepted

- M3.2 targets demo / case-study usability, preferring “tell the user” over refusing startup.
- Partial-frame corruption is unlikely on the normal `write_all` path.
- The risk is published as accepted debt; this project does not claim production-grade AOF.

## Follow-up condition

For stronger persistence confidence in M4/v0.2, implement refuse-to-start plus repair/truncate tooling, or handle safe-offset truncation during AOF rewrite work.

## Cross-references

- Agent finding: [`../../agent/findings/m3-2-aof-replay-corruption-handling.md`](../../agent/findings/m3-2-aof-replay-corruption-handling.md)
- ADR: [`adr-0010-m3-2-aof-persistence.md`](adr-0010-m3-2-aof-persistence.md)
