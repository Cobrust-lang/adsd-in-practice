# CS-02 mini-git-rust — Local Agent Constitution

> Local CLAUDE.md。覆盖顶层 [`/CLAUDE.md`](../CLAUDE.md) 的 case-specific 规则。

---

## 1. 本 case 不可简化的核心约束(F24 防御)

- ❌ **不准用 `serde_json` 序列化 git 对象**——git 对象格式是 binary + zlib,改 JSON 就是 F24 偷懒
- ❌ **不准用 sqlite 当 object store**——must be loose files in `.mg/objects/aa/bb..`,跟真 git 兼容
- ❌ **不准跳过 zlib 压缩**——必须用 `flate2` 或等价 crate,这是格式兼容性的核心
- ✅ 允许只支持 SHA-1(0.1.0),但 hash abstraction 必须留出 SHA-256 升级口

判断标准:**`git cat-file -p $sha` 必须能读我们写的对象**,反过来 `mg cat-file -p $sha` 必须能读真 `git` 写的对象。这是 oracle。

## 2. 本 case 的 oracle(F23-A 防御)

```bash
# 起一个真 git repo
mkdir oracle && cd oracle && git init
echo "hello" > a.txt
git add a.txt
git commit -m "init"

# 用 mg 读真 git 的 object
SHA=$(git rev-parse HEAD)
mg cat-file -p "$SHA"   # 必须输出跟 `git cat-file -p $SHA` 一致

# 反向:用 mg 写,用 git 读
mkdir reverse && cd reverse && mg init
echo "world" > b.txt
mg add b.txt
mg commit -m "init"
# 把 .mg → .git 重命名,然后 git log 必须能读
mv .mg .git
git log  # 必须显示 mg 写的 commit
```

测试脚本:`tests/oracle.sh`,gate 4 跑它。

## 3. 实施顺序(F22 cadence-aware)

**Wave M1**(object 层 + hash-object):
1. SHA-1 hash 抽象 + `mg hash-object` / `cat-file` 两个 plumbing 命令
2. Blob 对象读写

**Wave M2**(index + tree):
3. Index 文件 reader/writer(对照 `git ls-files --stage` 输出)
4. `mg add <path>` / `mg write-tree`(plumbing)

**Wave M3**(commit + repo):
5. `mg init` / `mg commit-tree` / `mg commit -m` / `mg log`
6. Repo discovery

**Wave M4**:release + status

## 4. crate 依赖

- `mg-core`(lib):**纯库,不依赖 clap**。完整 API exposed for embedding。
- `mg-cli`(bin):依赖 `mg-core` + `clap`。

依赖单向。

## 5. 测试策略

- 单测:每个 object 类型的 hash/encode round-trip
- 集成:`tests/oracle.sh` 跟真 git 双向兼容
- 模糊:**至少 1000 个随机文件名/内容做 hash round-trip**(ADSD `≥1,000 fuzz inputs` 约束)

## 6. 双语 doc 边界

同 cs01。优先中文 README,代码注释英文。

---

**End. 其它沿用顶层 CLAUDE.md。**
