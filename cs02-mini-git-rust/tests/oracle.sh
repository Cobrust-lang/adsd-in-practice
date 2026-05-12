#!/usr/bin/env bash
# CS-02 M1 oracle: compare mg blob identity and loose-object IO with real git.
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
MG_BIN="$ROOT/target/debug/mg"
WORK="$ROOT/target/oracle-m1"

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

RANDOM_DIR="$WORK/random"
mkdir -p "$RANDOM_DIR"
python3 - "$RANDOM_DIR" <<'PY'
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

for file in "$RANDOM_DIR"/*.bin; do
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

printf 'oracle ok: fixed fixtures + 1000 randomized blobs + bidirectional loose-object read/write\n'
