#!/usr/bin/env bash
# Compile external SDK/CLI consumers from one immutable source archive.
set -euo pipefail
if [ "$#" -ne 2 ]; then
  printf 'usage: %s SOURCE.tar WORK_DIRECTORY\n' "$0" >&2
  exit 2
fi
archive="$(realpath "$1")"
work="$2"
mkdir "$work"
work="$(cd "$work" && pwd -P)"
mkdir "$work/source" "$work/consumer"
tar -xf "$archive" -C "$work/source"
source_snapshot() {
  python3 - "$work/source" <<'PYFILES'
import hashlib
from pathlib import Path
import sys
root = Path(sys.argv[1])
for path in sorted(root.rglob("*")):
    if path.is_symlink():
        print(path.relative_to(root), "symlink", path.readlink())
    elif path.is_file():
        print(path.relative_to(root), hashlib.sha256(path.read_bytes()).hexdigest())
PYFILES
}
source_snapshot > "$work/source-before.txt"
lock_before="$(sha256sum "$work/source/Cargo.lock")"
config_before="$(sha256sum "$work/source/.cargo/config.toml")"
(
  cd "$work/source"
  cargo build --locked -p fathomdb-sdk -p fathomdb-cli --bin fathomdb --lib --target-dir "$work/target"
)
cp "$work/source/scripts/release/smoke/registry-sdk.rs" "$work/consumer/main.rs"
rustc --edition=2021 "$work/consumer/main.rs" \
  --extern "fathomdb_sdk=$work/target/debug/libfathomdb_sdk.rlib" \
  -L "dependency=$work/target/debug/deps" -o "$work/consumer/sdk"
"$work/consumer/sdk" "$work/consumer/persisted.fdb" write
"$work/consumer/sdk" "$work/consumer/persisted.fdb" reopen
version="$(python3 - "$work/source/Cargo.toml" <<'PY'
import sys
import tomllib
with open(sys.argv[1], "rb") as source:
    print(tomllib.load(source)["workspace"]["package"]["version"])
PY
)"
python3 "$work/source/scripts/release/smoke/candidate-cli.py" "$work/target/debug/fathomdb" "$version"
lock_after="$(sha256sum "$work/source/Cargo.lock")"
test "$lock_before" = "$lock_after"
config_after="$(sha256sum "$work/source/.cargo/config.toml")"
test "$config_before" = "$config_after"
source_snapshot > "$work/source-after.txt"
cmp "$work/source-before.txt" "$work/source-after.txt"
printf 'source archive SDK/CLI consumers: PASS; producer lock/config unchanged; no registry resolution claim\n'
