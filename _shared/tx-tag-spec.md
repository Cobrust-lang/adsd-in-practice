# Tx Tag Commit Spec

> Sub-set of conventional commits + ADSD-specific Tx tag。沿用 [Cobrust Studio CLAUDE.md §4](https://github.com/Cobrust-lang/cobrust-studio/blob/main/CLAUDE.md) 风格。

## Format

```
<type>(<scope>): <Tx-tag> <subject> (Wave <X.Y>)

[optional body]

[optional footer with Co-Authored-By, refs, etc.]
```

## Fields

| 字段 | 必填 | 说明 |
|---|---|---|
| `type` | yes | `feat` / `fix` / `chore` / `docs` / `test` / `refactor` / `perf` / `merge` |
| `scope` | yes | 子系统名(`server` / `storage` / `parser` / `cli` / `web` 等) |
| `Tx-tag` | yes | 一个字母 + 1-3 个数字 + 可选 `.M.N`,见下 |
| `subject` | yes | 50 字内,祈使句不句号 |
| `Wave X.Y` | 推荐 | 跟当前 milestone 对齐(可选) |

## Tx tag 字母约定

每个 case **独立编号空间**(cs01-A1.1 跟 cs02-A1.1 不冲突,文件路径不同)。

| 字母 | 含义 | 示例 |
|---|---|---|
| **A** | Architecture / ADR 实施 | `A1.1` 实施 ADR-0001 的第一步 |
| **F** | Feature / 用户可见功能 | `F1.2` |
| **B** | Bug fix(对应 finding) | `B3.1` |
| **D** | Doc(双语 / ADR / finding 文档本身)| `D2.1` |
| **T** | Test / 测试基础设施 | `T4.1` |
| **R** | Refactor / 非功能性结构改进 | `R1.1` |
| **P** | Perf / 性能优化 | `P2.1` |
| **M** | Meta / repo / CI / build script | `M0.1` |

## 例子

```
feat(server): A1.4 wire SSE dispatch route (Wave M1)

实施 ADR-0003 第 4 步:加 /api/dispatch 路由,SSE 协议从 cobrust-llm-router lift 过来。

Refs: ADR-0003, ADR-0005
```

```
fix(parser): B2.1 close finding cobrust-shougate (Wave M4)

Path<String> extractor 在 SPA fallback 路由上 panic,根因见
finding cto-shougate-test-gate-grep-leak.md。改用 Uri extractor。

Closes: finding cto-shougate-test-gate-grep-leak
```

```
docs(adr): D1.1 ADR-0001 stack choice (Wave M0)

固化:Rust + Axum + SvelteKit 5 + SQLite + rust-embed。
3 个候选:轴心 Axum vs Actix vs warp。
```

```
chore(meta): M0.1 workspace + 5-gate skeleton (Wave M0)
```

## 反模式(禁止)

- ❌ 不写 Tx tag:`feat(server): add SSE route` ← 必加 `A1.4` 之类
- ❌ scope 写 `misc` / `various` / `cleanup` ← 必须是具体子系统
- ❌ subject 长于 50 字 ← 用 body
- ❌ 一个 commit 多个 Tx:`A1.1 + B2.3` ← 必须拆 commit
- ❌ wave 写 `WIP` / `tmp` ← 公开 push 前必须 squash 进真实 wave

## 跟 git workflow 的接口

- 一个 Tx 一个 commit
- Wave 合并 = `git merge --no-ff` of Tx branches into wave branch
- Wave branch 合并 main 用 squash/rebase 都行,但 **Tx tag 必须保留在 squash 后的 message 里**

## CI gate

CI 应该验证:
- subject 行格式匹配 `^(feat|fix|chore|docs|test|refactor|perf|merge)\(.+\): [A-Z]\d+(\.\d+(\.\d+)?)?\s+\S+`
- subject ≤ 50 chars

(本 repo 暂不强制 CI 检查 Tx tag 格式,但鼓励作者本地 hook。)
