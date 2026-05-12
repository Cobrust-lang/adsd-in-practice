# ADR-0012 中文摘要:M4.2 doc sweep + release artifacts

> 完整 ADR 见 [docs/agent/adr/0012-m4-2-doc-sweep-release-artifacts.md](../../agent/adr/0012-m4-2-doc-sweep-release-artifacts.md)。

## 决策

M4.2 一波 holistic doc sweep + release artifacts,7 buckets ~30 items,**全是 audit-surfaced doc**:

**Bucket A — Release artifacts**(Sarah hard blockers):
- LICENSE-APACHE + LICENSE-MIT 在 repo root(SPDX 已声明无 license text)
- CHANGELOG.md / CONTRIBUTING.md / SECURITY.md
- METHODOLOGY-STATUS.md cs01 节填实

**Bucket B — README sediment**(public-readiness BLOCK + HIGH 全清):
- §Status update 反映 M1-M4.1 进度
- /pubsub stub note 删 → read-only dashboard
- quick-start 去掉 `--aof data/dump.aof`(ENOENT)
- 加 §"Supported commands" / §"Prerequisites" / §"Docs" / §"Why does this exist?" / §"Known behavioral deltas vs real Redis"

**Bucket C — bootstrap.sh** 不再说 "M1.0 scaffold"

**Bucket D — Bilingual finding mirrors**:m1-1 + m3-1 + m3-2 各 zh + en abstract(顶层 CLAUDE.md §1.1 invariant)

**Bucket E — `_shared/doc-coverage.sh` enforce finding bilingual**(防 future F1 sediment 复发,LOW-2 root cause)

**Bucket F — ADR metadata sweep**:
- ADR-0005 last_verified_commit re-bless
- ADR-0001 Done Criteria rust-embed defer 到 M4.3+
- ADR-0009 §Done Criteria 加 "RESET" + §Implementation deltas 补 pubsub.rs 不存在
- cs01 CLAUDE.md §3 wave closure markers

## 数字目标

- backend test ≥ 284(不变)
- oracle 36/36(不变)
- 顶层 + cs01 双 CHANGELOG.md
- 6 个新 bilingual finding abstract
- 5 个 release artifact 文件 at repo root

## 状态

`accepted` — 2026-05-12。下一步 M4.3 v0.1.0 tag。
