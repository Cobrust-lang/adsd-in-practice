#!/usr/bin/env bash
# CS-02 oracle: compare mg object/index/tree behavior with real git.
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
MG_BIN="$ROOT/target/debug/mg"
WORK="$ROOT/target/oracle-m2"

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

stage_with_mg_and_compare_tree() {
    local src_dir="$1"
    local context="$2"
    local mg_repo="$WORK/${context}-mg"
    local git_repo="$WORK/${context}-git"
    mkdir -p "$mg_repo" "$git_repo"

    cp "$src_dir"/* "$mg_repo"/
    cp "$src_dir"/* "$git_repo"/

    (
        cd "$mg_repo"
        "$MG_BIN" init >/dev/null
        for file in *; do
            [ -f "$file" ] || continue
            "$MG_BIN" add "$file"
        done
        mg_tree=$("$MG_BIN" write-tree)
        git -C "$git_repo" init -q
        (
            cd "$git_repo"
            git add .
            git_tree=$(git write-tree)
            assert_eq "$git_tree" "$mg_tree" "$context write-tree sha"
        )

        ls_stage=$(GIT_DIR="$mg_repo/.mg" GIT_WORK_TREE="$mg_repo" git ls-files --stage)
        if [ -z "$ls_stage" ]; then
            printf 'oracle mismatch: %s git ls-files --stage returned empty output\n' "$context" >&2
            exit 1
        fi
        for file in *; do
            [ -f "$file" ] || continue
            blob_sha=$(git hash-object "$file")
            expected_stage="100644 $blob_sha 0	$file"
            if ! grep -Fqx "$expected_stage" <<<"$ls_stage"; then
                printf 'oracle mismatch: %s missing stage line\nexpected: %s\nactual:\n%s\n' "$context" "$expected_stage" "$ls_stage" >&2
                exit 1
            fi
        done

        pretty=$(GIT_OBJECT_DIRECTORY="$mg_repo/.mg/objects" git -C "$GIT_READER" cat-file -p "$mg_tree")
        if [ -z "$pretty" ]; then
            printf 'oracle mismatch: %s git cat-file -p tree returned empty output\n' "$context" >&2
            exit 1
        fi
    )
}

FIXED_TREE_SRC="$WORK/fixed-tree-src"
mkdir -p "$FIXED_TREE_SRC"
printf 'alpha\n' > "$FIXED_TREE_SRC/a.txt"
printf 'beta\n' > "$FIXED_TREE_SRC/b.txt"
printf 'space name\n' > "$FIXED_TREE_SRC/file with space.txt"
stage_with_mg_and_compare_tree "$FIXED_TREE_SRC" "fixed-tree"

RANDOM_TREE_SRC="$WORK/random-tree-src"
mkdir -p "$RANDOM_TREE_SRC"
python3 - "$RANDOM_TREE_SRC" <<'PY'
import pathlib
import random
import string
import sys

out = pathlib.Path(sys.argv[1])
rng = random.Random(0xA3_02_0002)
alphabet = string.ascii_letters + string.digits + "._-"
seen = set()
for i in range(1000):
    while True:
        stem = "f" + "".join(rng.choice(alphabet) for _ in range(rng.randrange(6, 24)))
        name = f"{stem}-{i:04d}.dat"
        if name not in seen:
            seen.add(name)
            break
    size = rng.randrange(0, 2048)
    data = bytes(rng.randrange(0, 256) for _ in range(size))
    (out / name).write_bytes(data)
PY
stage_with_mg_and_compare_tree "$RANDOM_TREE_SRC" "random-tree-1000"

UNSUPPORTED_REPO="$WORK/unsupported"
mkdir -p "$UNSUPPORTED_REPO/subdir"
(
    cd "$UNSUPPORTED_REPO"
    "$MG_BIN" init >/dev/null
    printf 'nested\n' > subdir/nested.txt
    if "$MG_BIN" add subdir/nested.txt 2>/dev/null; then
        printf 'oracle mismatch: nested path should fail clearly in M2\n' >&2
        exit 1
    fi
)

printf 'oracle ok: M1 blobs + M2 git-readable index/tree + 1000 randomized flat add/write-tree cases\n'
