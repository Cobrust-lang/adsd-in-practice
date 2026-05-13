#!/usr/bin/env bash
# CS-02 oracle: compare mg v0.1 subset behavior with real git, including M4 hardening negatives.
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
MG_BIN="$ROOT/target/debug/mg"
WORK="$ROOT/target/oracle-m4"

export GIT_AUTHOR_NAME='A U Thor'
export GIT_AUTHOR_EMAIL='author@example.com'
export GIT_AUTHOR_DATE='1700000000 +0000'
export GIT_COMMITTER_NAME='C O Mitter'
export GIT_COMMITTER_EMAIL='committer@example.com'
export GIT_COMMITTER_DATE='1700000001 +0000'

cargo build --manifest-path "$ROOT/Cargo.toml" --bin mg --locked >/dev/null
rm -rf "$WORK"
mkdir -p "$WORK"

assert_eq() {
    local expected="$1"
    local actual="$2"
    local context="$3"
    if [ "$expected" != "$actual" ]; then
        printf 'oracle mismatch: %s\nexpected: %s\nactual:   %s\n' "$context" "$expected" "$actual" >&2
        exit 1
    fi
}

assert_contains() {
    local haystack="$1"
    local needle="$2"
    local context="$3"
    if [[ "$haystack" != *"$needle"* ]]; then
        printf 'oracle mismatch: %s\nmissing: %s\nactual:\n%s\n' "$context" "$needle" "$haystack" >&2
        exit 1
    fi
}

assert_fails_with() {
    local expected="$1"
    shift
    set +e
    local output
    output=$("$@" 2>&1)
    local status=$?
    set -e
    if [ "$status" -eq 0 ]; then
        printf 'oracle mismatch: expected failure containing %s but command succeeded: %s\n' "$expected" "$*" >&2
        exit 1
    fi
    if [[ "$output" != *"$expected"* ]]; then
        printf 'oracle mismatch: expected failure containing %s\nactual output:\n%s\n' "$expected" "$output" >&2
        exit 1
    fi
}

fixed_fixture() {
    local name="$1"
    local content="$2"
    local file="$WORK/$name"
    printf '%s' "$content" > "$file"
    local git_sha
    local mg_sha
    git_sha=$(git hash-object "$file")
    mg_sha=$("$MG_BIN" hash-object "$file")
    assert_eq "$git_sha" "$mg_sha" "fixed fixture $name hash-object"
}

fixed_fixture empty ''
fixed_fixture hello 'hello'
fixed_fixture hello-newline 'hello
'
fixed_fixture binary $'\x00\x01blob 5\0hello\xff'

RANDOM_BLOB_DIR="$WORK/random-blobs"
mkdir -p "$RANDOM_BLOB_DIR"
python3 - "$RANDOM_BLOB_DIR" <<'PY'
import pathlib
import random
import sys

out = pathlib.Path(sys.argv[1])
rng = random.Random(0xA2_02_0001)
for i in range(1000):
    size = rng.randrange(0, 4097)
    data = bytes(rng.randrange(0, 256) for _ in range(size))
    (out / f"case-{i:04d}.bin").write_bytes(data)
PY

for file in "$RANDOM_BLOB_DIR"/*.bin; do
    git_sha=$(git hash-object "$file")
    mg_sha=$("$MG_BIN" hash-object "$file")
    assert_eq "$git_sha" "$mg_sha" "random $(basename "$file") hash-object"
done

GIT_READER="$WORK/git-reader"
mkdir -p "$GIT_READER"
git -C "$GIT_READER" init -q

MG_REPO="$WORK/mg-repo"
mkdir -p "$MG_REPO"
(
    cd "$MG_REPO"
    "$MG_BIN" init >/dev/null
    printf 'mg writes git-readable blob\n' > blob.txt
    sha=$("$MG_BIN" hash-object -w blob.txt)
    git_payload=$(GIT_OBJECT_DIRECTORY="$MG_REPO/.mg/objects" git -C "$GIT_READER" cat-file -p "$sha")
    expected=$(printf 'mg writes git-readable blob')
    assert_eq "$expected" "$git_payload" "git cat-file reads mg-written blob"
    mg_payload=$("$MG_BIN" cat-file -p "$sha")
    assert_eq "$expected" "$mg_payload" "mg cat-file reads mg-written blob"
)

GIT_REPO="$WORK/git-repo"
mkdir -p "$GIT_REPO"
(
    cd "$GIT_REPO"
    git init -q
    printf 'git writes mg-readable blob\n' > blob.txt
    sha=$(git hash-object -w blob.txt)
    mv .git .mg
    mg_payload=$("$MG_BIN" cat-file -p "$sha")
    expected=$(printf 'git writes mg-readable blob')
    assert_eq "$expected" "$mg_payload" "mg cat-file reads git-written blob"
)

copy_regular_tree() {
    local src_dir="$1"
    local dst_dir="$2"
    mkdir -p "$dst_dir"
    (cd "$src_dir" && find . -type d -exec mkdir -p "$dst_dir/{}" \;)
    (cd "$src_dir" && find . -type f -exec cp "{}" "$dst_dir/{}" \;)
}

stage_all_regular_files() {
    local repo_dir="$1"
    (
        cd "$repo_dir"
        while IFS= read -r -d '' file; do
            "$MG_BIN" add "${file#./}"
        done < <(find . \( -path './.mg' -o -path './.mg/*' -o -path './.git' -o -path './.git/*' \) -prune -o -type f -print0 | LC_ALL=C sort -z)
    )
}

stage_with_mg_and_compare_tree() {
    local src_dir="$1"
    local context="$2"
    local mg_repo="$WORK/${context}-mg"
    local git_repo="$WORK/${context}-git"
    mkdir -p "$mg_repo" "$git_repo"

    copy_regular_tree "$src_dir" "$mg_repo"
    copy_regular_tree "$src_dir" "$git_repo"

    (
        cd "$mg_repo"
        "$MG_BIN" init >/dev/null
    )
    stage_all_regular_files "$mg_repo"
    mg_tree=$(cd "$mg_repo" && "$MG_BIN" write-tree)

    (
        cd "$git_repo"
        git init -q
        git add .
        git_tree=$(git write-tree)
        assert_eq "$git_tree" "$mg_tree" "$context recursive write-tree sha"
    )

    ls_stage=$(GIT_DIR="$mg_repo/.mg" GIT_WORK_TREE="$mg_repo" git ls-files --stage)
    if [ -z "$ls_stage" ]; then
        printf 'oracle mismatch: %s git ls-files --stage returned empty output\n' "$context" >&2
        exit 1
    fi
    (
        cd "$git_repo"
        while IFS= read -r -d '' file; do
            rel=${file#./}
            blob_sha=$(git hash-object "$rel")
            expected_stage=$'100644 '"$blob_sha"$' 0\t'"$rel"
            if ! grep -Fqx "$expected_stage" <<<"$ls_stage"; then
                printf 'oracle mismatch: %s missing stage line\nexpected: %s\nactual:\n%s\n' "$context" "$expected_stage" "$ls_stage" >&2
                exit 1
            fi
        done < <(find . \( -path './.mg' -o -path './.mg/*' -o -path './.git' -o -path './.git/*' \) -prune -o -type f -print0 | LC_ALL=C sort -z)
    )

    pretty=$(GIT_OBJECT_DIRECTORY="$mg_repo/.mg/objects" git -C "$GIT_READER" cat-file -p "$mg_tree")
    if [ -z "$pretty" ]; then
        printf 'oracle mismatch: %s git cat-file -p tree returned empty output\n' "$context" >&2
        exit 1
    fi
}

FIXED_TREE_SRC="$WORK/fixed-tree-src"
mkdir -p "$FIXED_TREE_SRC/src" "$FIXED_TREE_SRC/docs/guides" "$FIXED_TREE_SRC/spaced dir"
printf 'alpha\n' > "$FIXED_TREE_SRC/a.txt"
printf 'beta\n' > "$FIXED_TREE_SRC/src/lib.rs"
printf 'nested\n' > "$FIXED_TREE_SRC/docs/guides/intro.txt"
printf 'space name\n' > "$FIXED_TREE_SRC/spaced dir/file with space.txt"
stage_with_mg_and_compare_tree "$FIXED_TREE_SRC" "fixed-tree"

RANDOM_TREE_SRC="$WORK/random-tree-src"
mkdir -p "$RANDOM_TREE_SRC"
python3 - "$RANDOM_TREE_SRC" <<'PY'
import pathlib
import random
import string
import sys

out = pathlib.Path(sys.argv[1])
rng = random.Random(0xA4_02_0003)
alphabet = string.ascii_letters + string.digits + "._-"
seen = set()
for i in range(1000):
    depth = rng.randrange(1, 5)
    dirs = []
    for level in range(depth - 1):
        dirs.append("d" + "".join(rng.choice(alphabet) for _ in range(rng.randrange(3, 12))) + f"-{level}")
    while True:
        stem = "f" + "".join(rng.choice(alphabet) for _ in range(rng.randrange(6, 24)))
        rel = pathlib.Path(*dirs) / f"{stem}-{i:04d}.dat"
        if rel.as_posix() not in seen:
            seen.add(rel.as_posix())
            break
    path = out / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    size = rng.randrange(0, 2048)
    data = bytes(rng.randrange(0, 256) for _ in range(size))
    path.write_bytes(data)
PY
stage_with_mg_and_compare_tree "$RANDOM_TREE_SRC" "random-recursive-tree-1000"

COMMIT_REPO="$WORK/commit-repo"
mkdir -p "$COMMIT_REPO"
(
    cd "$COMMIT_REPO"
    "$MG_BIN" init >/dev/null
    mkdir -p src docs
    printf 'hello commit\n' > README.md
    printf 'fn main() {}\n' > src/main.rs
    "$MG_BIN" add README.md
    (cd src && "$MG_BIN" add main.rs)
    first=$("$MG_BIN" commit -m 'first commit')
    head=$(GIT_DIR="$COMMIT_REPO/.mg" GIT_WORK_TREE="$COMMIT_REPO" git rev-parse HEAD)
    assert_eq "$first" "$head" "git rev-parse HEAD reads mg first commit ref"
    commit_payload=$(GIT_DIR="$COMMIT_REPO/.mg" GIT_WORK_TREE="$COMMIT_REPO" git cat-file -p "$first")
    assert_contains "$commit_payload" 'tree ' "git cat-file -p first commit tree line"
    assert_contains "$commit_payload" 'author A U Thor <author@example.com> 1700000000 +0000' "git cat-file -p first author"
    assert_contains "$commit_payload" 'committer C O Mitter <committer@example.com> 1700000001 +0000' "git cat-file -p first committer"
    assert_contains "$commit_payload" 'first commit' "git cat-file -p first message"
    ls_tree=$(GIT_DIR="$COMMIT_REPO/.mg" GIT_WORK_TREE="$COMMIT_REPO" git ls-tree -r --name-only HEAD)
    assert_contains "$ls_tree" 'README.md' "git ls-tree -r first commit README"
    assert_contains "$ls_tree" 'src/main.rs' "git ls-tree -r first commit nested file"

    printf 'second\n' > docs/second.txt
    (cd docs && "$MG_BIN" add second.txt)
    second=$("$MG_BIN" commit -m 'second commit')
    head=$(GIT_DIR="$COMMIT_REPO/.mg" GIT_WORK_TREE="$COMMIT_REPO" git rev-parse HEAD)
    assert_eq "$second" "$head" "git rev-parse HEAD reads mg second commit ref"
    git_log=$(GIT_DIR="$COMMIT_REPO/.mg" GIT_WORK_TREE="$COMMIT_REPO" git log --format='%H %s')
    mg_log=$("$MG_BIN" log)
    assert_eq "$git_log" "$mg_log" "mg log matches git first-parent log"
    second_payload=$(GIT_DIR="$COMMIT_REPO/.mg" GIT_WORK_TREE="$COMMIT_REPO" git cat-file -p "$second")
    assert_contains "$second_payload" "parent $first" "git cat-file -p second parent"
)

COMMIT_TREE_REPO="$WORK/commit-tree-repo"
mkdir -p "$COMMIT_TREE_REPO"
(
    cd "$COMMIT_TREE_REPO"
    "$MG_BIN" init >/dev/null
    printf 'plumbing\n' > p.txt
    "$MG_BIN" add p.txt
    tree=$("$MG_BIN" write-tree)
    commit=$("$MG_BIN" commit-tree "$tree" -m 'plumbing commit')
    payload=$(GIT_OBJECT_DIRECTORY="$COMMIT_TREE_REPO/.mg/objects" git -C "$GIT_READER" cat-file -p "$commit")
    assert_contains "$payload" "tree $tree" "git cat-file -p commit-tree tree"
    assert_contains "$payload" 'plumbing commit' "git cat-file -p commit-tree message"
)

NEGATIVE_REPO="$WORK/negative-repo"
mkdir -p "$NEGATIVE_REPO"
(
    cd "$NEGATIVE_REPO"
    "$MG_BIN" init >/dev/null
    printf 'ok\n' > ok.txt
    "$MG_BIN" add ok.txt

    assert_fails_with 'refusing to stage repository-internal path' "$MG_BIN" add .mg/HEAD

    mkdir -p nested/.git
    printf 'sentinel\n' > nested/.git/secret
    assert_fails_with 'refusing to stage repository-internal path' "$MG_BIN" add nested/.git/secret

    sha=$("$MG_BIN" hash-object ok.txt)
    upper_sha=$(printf '%s' "$sha" | tr '[:lower:]' '[:upper:]')
    assert_fails_with 'lowercase 40-character SHA-1 hex object ID' "$MG_BIN" cat-file -p "$upper_sha"

    rm .mg/index
    ln -s ok.txt .mg/index
    assert_fails_with 'refusing to overwrite symlink target' "$MG_BIN" add ok.txt
    rm .mg/index
    "$MG_BIN" add ok.txt

    ln -s ok.txt symlink.txt
    assert_fails_with 'does not support symlink inputs' "$MG_BIN" add symlink.txt
    rm symlink.txt

    rm -f .mg/index.lock .mg/index
    ln -s ok.txt .mg/index
    assert_fails_with 'refusing to overwrite symlink target' "$MG_BIN" add ok.txt
    [ ! -e .mg/index.lock ] || { printf 'oracle mismatch: stale .mg/index.lock should be cleaned after write failure\n' >&2; exit 1; }
    rm .mg/index
    "$MG_BIN" add ok.txt

    touch .mg/index.lock
    assert_fails_with 'File exists' "$MG_BIN" add ok.txt
    rm .mg/index.lock

    rm -f .mg/refs/heads/main
    ln -s ../../ok.txt .mg/refs/heads/main
    assert_fails_with 'refusing to overwrite symlink target' "$MG_BIN" commit -m 'blocked by ref symlink'
    rm .mg/refs/heads/main

    rm -rf .mg/refs
    mkdir -p .mg/redirect-parent
    ln -s ../redirect-parent .mg/refs
    assert_fails_with 'refusing to traverse symlink ancestor' "$MG_BIN" commit -m 'blocked by ref ancestor symlink'
    rm .mg/refs
    mkdir -p .mg/refs/heads

    printf 'another\n' > another.txt
    another_sha=$("$MG_BIN" hash-object another.txt)
    another_prefix=${another_sha:0:2}
    rm -rf ".mg/objects/$another_prefix" .mg/redirect-objects
    mkdir -p .mg/redirect-objects
    ln -s ../redirect-objects ".mg/objects/$another_prefix"
    assert_fails_with 'refusing to traverse symlink ancestor' "$MG_BIN" hash-object -w another.txt
    rm ".mg/objects/$another_prefix"

    huge_sha=$(python3 - <<'PY'
import hashlib
import pathlib
import zlib

mg = pathlib.Path('.mg')
objects = mg / 'objects'
payload = b'a' * (16 * 1024 * 1024 + 1)
raw = b'blob ' + str(len(payload)).encode() + b'\0' + payload
sha = hashlib.sha1(raw).hexdigest()
path = objects / sha[:2]
path.mkdir(parents=True, exist_ok=True)
(path / sha[2:]).write_bytes(zlib.compress(raw))
print(sha)
PY
)
    assert_fails_with 'decoded size cap' "$MG_BIN" cat-file -p "$huge_sha"

    python3 - <<'PY'
import hashlib
import pathlib
import struct

entry_count = 1_000_000
header = b'DIRC' + struct.pack('>II', 2, entry_count)
checksum = hashlib.sha1(header).digest()
(pathlib.Path('.mg') / 'index').write_bytes(header + checksum)
PY
    assert_fails_with 'entry count exceeds file length lower bound' "$MG_BIN" write-tree
)

printf 'oracle ok: M1 blobs + M2 recursive index/tree + M3 commits/log + M4 hardening negatives + 1000 deterministic randomized regular-file path/content cases\n'
