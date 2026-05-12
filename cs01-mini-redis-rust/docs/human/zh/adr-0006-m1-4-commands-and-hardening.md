# ADR-0006 中文摘要:M1.4 命令扩展 + 加固

> 完整 ADR 见 [docs/agent/adr/0006-m1-4-commands-and-hardening.md](../../agent/adr/0006-m1-4-commands-and-hardening.md)。

## 决策

M1.4 一波收齐 cs01 Wave M1 剩余 8 件事:

1. **EXPIRE / TTL / PERSIST**:DelayQueue 走 Option A(stale entry 让 expiry task 自然 skip),TTL 严格 -2 / -1 / 剩余秒数对齐 Redis
2. **TYPE**:加 `Reply::SimpleString(String)` variant;v0.1 只 `"string"` / `"none"`
3. **KEYS pattern**:自写 ~50 LOC glob matcher 支持 `*` / `?` / `[a-z]` / `\` 转义(F24 防御:不引 globset)
4. **PING optional message**:扩 `Command::Ping { message: Option<Vec<u8>> }`
5. **max-frame-size guard**:512 MiB 默认 + `--max-frame-size` CLI flag,F5 hardening
6. **docker oracle**:`tests/oracle.sh` 真接 `redis:7-alpine` round-trip,F23-A 兑现;opt-in via `CS01_RUN_ORACLE=1`
7. **目标 test count ≥ 100**(M1.3 是 82)
8. **doc-coverage + 全 5 gate green**

## 拒绝的方案

- DelayQueue Option B 真 cancel(过早优化)
- KEYS 用 globset(过度依赖 + 语义不完全合)
- KEYS 退化只支持 `*` / `?`(F23-A oracle 必挂)
- Reply::Bulk 假装 SimpleString(字节不一致,F24)

## 状态

`accepted` — 2026-05-12
