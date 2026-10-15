#!/usr/bin/env bash
# check-tegra-pool-registry-build.sh — a downstream consumer of the published
# crates builds and runs `tegra-pool` with no `[patch]` table (0.8.28 Slice 31,
# AC31-08; design § 3.6). It replaces Slice 30's check that the feature
# failed against registry cudarc: the cudarc and Candle forks are now
# published, so the opposite claim holds.
#
# WHAT IT DOES
#   Copies the tracked workspace sources into a scratch directory (mktemp
#   under $TMPDIR) and adds one consumer crate, its own workspace root, that
#   depends on the copied fathomdb-embedder by path with `tegra-pool` and
#   forwards a consumer feature `embed-cuda`. The consumer has no `[patch]`
#   table at all: every crate comes from crates.io.
#     (a) Resolution (any host): `cargo generate-lockfile`; the lock must hold
#         cudarc-fathomdb 0.19.10, candle-{core,nn,transformers}-fathomdb
#         0.11.0 and candle-kernels 0.10.2, each from the crates.io registry,
#         and no cudarc 0.19.x (ug-cuda's cudarc 0.17.8 is allowed).
#     (b) Build and link (aarch64 Linux Tegra host): `cargo build` and
#         `cargo test --no-run` with `--features embed-cuda`. Registry
#         candle-kernels links libcudart dynamically, so LIBRARY_PATH must
#         name the CUDA library directory.
#     (c) Runtime (aarch64 Linux Tegra host): the consumer carries copies of
#         tests/tegra_fragmented_va_cuda.rs and tests/support/live.rs (the
#         Slice 30 files are copied, not edited) and runs
#         forced_cuda_probe_succeeds_when_the_default_memory_pool_is_unavailable
#         with FATHOMDB_REQUIRE_LIVE=1; exactly one test must pass.
#   A non-member path dependency cannot run its own tests in another
#   workspace, which is why the test is copied into the consumer.
#   crates.io consumers get a dynamic libcudart through upstream
#   candle-kernels 0.10.2; REQ-068's CPU-loadable guarantee applies only to
#   FathomDB-built artifacts, which patch candle-kernels to the Candle fork.
#
# WHEN TO RUN
#   A release check, run by hand before a release that changes cudarc,
#   Candle or the `tegra-pool` wiring (dev/design/release.md), and in the
#   Slice 31 qualification (REG-a on any host; REG-b and REG-c on the Orin).
#   It is NOT part of scripts/agent-verify.sh: it needs crates.io, a CUDA
#   toolkit for (b) and a Jetson GPU for (c).
#
# USAGE
#   check-tegra-pool-registry-build.sh                 (a); plus (b) and (c)
#                                                      on an aarch64 Linux
#                                                      Tegra host
#   check-tegra-pool-registry-build.sh --resolve-only  (a) alone
#   ENABLE=embed-cuda-only (env)  the consumer's dependency names no
#                     `tegra-pool`, only `embed-cuda`, which implies it
#                     (slice-31 design.md § 14.3); (a)-(c) must still hold.
#
# ENV
#   CUDA_COMPUTE_CAP  passed to candle-kernels (default 87, the Jetson Orin).
#   CUDA_PATH, PATH, LIBRARY_PATH  the CUDA toolkit for (b) and (c);
#                     LIBRARY_PATH is required there.
#   FATHOMDB_POOL_MODE  passed through to (c) (the qualification runs `off`).
#   CARGO_TARGET_DIR  reuse a target directory (default: inside the scratch
#                     copy, removed on exit).
#   KEEP_SCRATCH=1    keep the scratch copy for inspection.
#
# Exit 0 when every step that applies to this host holds; 1 otherwise. A
# step that does not apply is named in the output, never skipped silently.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEST_NAME='forced_cuda_probe_succeeds_when_the_default_memory_pool_is_unavailable'

RESOLVE_ONLY=0
ENABLE="${ENABLE:-tegra-pool}"
case "$ENABLE" in
  tegra-pool) DEPENDENCY_FEATURES='features = ["tegra-pool"]' ;;
  embed-cuda-only) DEPENDENCY_FEATURES='features = []' ;;
  *) printf 'ENABLE must be tegra-pool or embed-cuda-only; got %s\n' "$ENABLE" >&2; exit 2 ;;
esac
case "${1:-}" in
  '') ;;
  --resolve-only) RESOLVE_ONLY=1 ;;
  *) printf 'usage: check-tegra-pool-registry-build.sh [--resolve-only]\n' >&2; exit 2 ;;
esac

fail() {
  printf 'FAIL  check-tegra-pool-registry-build: %s\n' "$1" >&2
  exit 1
}

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/fathomdb-tegra-pool-registry.XXXXXX")"
cleanup() {
  if [ "${KEEP_SCRATCH:-}" = "1" ]; then
    printf 'check-tegra-pool-registry-build: kept %s\n' "$SCRATCH"
  else
    rm -rf "$SCRATCH"
  fi
}
trap cleanup EXIT

export CUDA_COMPUTE_CAP="${CUDA_COMPUTE_CAP:-87}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$SCRATCH/target}"
printf 'check-tegra-pool-registry-build: CUDA_COMPUTE_CAP=%s\n' "$CUDA_COMPUTE_CAP"
printf 'check-tegra-pool-registry-build: CUDA_PATH=%s\n' "${CUDA_PATH:-<unset>}"
printf 'check-tegra-pool-registry-build: PATH=%s\n' "$PATH"
printf 'check-tegra-pool-registry-build: LIBRARY_PATH=%s\n' "${LIBRARY_PATH:-<unset>}"

# Tracked files only, from the working tree: build output and untracked
# scratch never reach the copy. The SQLite patch is a workspace patch the
# embedder does not reach, so it is not copied.
(
  cd "$REPO_ROOT"
  git ls-files -z -- Cargo.toml Cargo.lock rust-toolchain.toml src/rust \
    | tar --null -T - -cf -
) | tar -xf - -C "$SCRATCH"

CONSUMER="$SCRATCH/consumer"
EMBEDDER_TESTS="$SCRATCH/src/rust/crates/fathomdb-embedder/tests"
mkdir -p "$CONSUMER/src" "$CONSUMER/tests/support"
: >"$CONSUMER/src/lib.rs"
cp "$SCRATCH/rust-toolchain.toml" "$CONSUMER/"
cp "$EMBEDDER_TESTS/tegra_fragmented_va_cuda.rs" "$CONSUMER/tests/"
cp "$EMBEDDER_TESTS/support/live.rs" "$CONSUMER/tests/support/"
cat >"$CONSUMER/Cargo.toml" <<TOML
[package]
name = "tegra-pool-registry-consumer"
version = "0.0.0"
edition = "2021"
publish = false

[workspace]

[features]
embed-cuda = ["fathomdb-embedder/embed-cuda"]

[dependencies]
fathomdb-embedder = { path = "../src/rust/crates/fathomdb-embedder", $DEPENDENCY_FEATURES }

[dev-dependencies]
candle-core = { package = "candle-core-fathomdb", version = "=0.11.0" }
TOML
if grep -q '^\[patch' "$CONSUMER/Cargo.toml"; then
  fail 'the consumer manifest has a [patch] table'
fi

printf 'check-tegra-pool-registry-build: (a) resolution with no [patch] table\n'
(cd "$CONSUMER" && cargo generate-lockfile -q) || fail 'cargo generate-lockfile failed in the consumer'
if ! python3 - "$CONSUMER/Cargo.lock" <<'PY'
from pathlib import Path
import re
import sys

# Python 3.10 (the Jetson's) has no tomllib; Cargo.lock's package blocks are regular.
REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"
EXPECTED = {
    "cudarc-fathomdb": "0.19.10",
    "candle-core-fathomdb": "0.11.0",
    "candle-nn-fathomdb": "0.11.0",
    "candle-transformers-fathomdb": "0.11.0",
    "candle-kernels": "0.10.2",
}
packages = []
for block in Path(sys.argv[1]).read_text(encoding="utf-8").split("[[package]]")[1:]:
    fields = dict(re.findall(r'^(name|version|source) = "([^"]*)"$', block, re.MULTILINE))
    packages.append((fields.get("name"), fields.get("version"), fields.get("source")))
failures = []
for name, version in EXPECTED.items():
    found = [(v, s) for n, v, s in packages if n == name]
    if found != [(version, REGISTRY)]:
        failures.append(f"{name}: expected exactly {version} from crates.io, found {found}")
    else:
        print(f"  {name} {version} {REGISTRY}")
for name, version, source in packages:
    if name == "cudarc":
        if version.startswith("0.19."):
            failures.append(f"upstream cudarc {version} resolved ({source})")
        else:
            print(f"  cudarc {version} {source} (ug-cuda; allowed)")
for failure in failures:
    print(f"  {failure}", file=sys.stderr)
raise SystemExit(1 if failures else 0)
PY
then
  fail '(a) the consumer did not resolve the published fork crates from crates.io'
fi
printf 'PASS  (a) the published fork crates resolve from crates.io with no [patch] table\n'

if [ "$RESOLVE_ONLY" = "1" ]; then
  printf 'check-tegra-pool-registry-build: --resolve-only: (b) and (c) not run\n'
  exit 0
fi
HOST_OS="$(uname -s)"
HOST_ARCH="$(uname -m)"
if [ "$HOST_OS" != "Linux" ] || [ "$HOST_ARCH" != "aarch64" ] || [ ! -e /etc/nv_tegra_release ]; then
  printf 'check-tegra-pool-registry-build: (b) and (c) need an aarch64 Linux Tegra host; this host is %s %s: not run\n' \
    "$HOST_OS" "$HOST_ARCH"
  exit 0
fi
[ -n "${LIBRARY_PATH:-}" ] || fail '(b) LIBRARY_PATH must name the CUDA library directory (registry candle-kernels links libcudart dynamically)'

printf 'check-tegra-pool-registry-build: (b) build and link on this Tegra host\n'
(cd "$CONSUMER" && cargo build --locked --features embed-cuda) || fail '(b) cargo build --features embed-cuda failed'
(cd "$CONSUMER" && cargo test --locked --no-run --features embed-cuda) || fail '(b) cargo test --no-run --features embed-cuda failed'
printf 'PASS  (b) the consumer builds and links tegra-pool from crates.io\n'

printf 'check-tegra-pool-registry-build: (c) %s\n' "$TEST_NAME"
set +e
OUTPUT="$(cd "$CONSUMER" && FATHOMDB_REQUIRE_LIVE=1 cargo test --locked --features embed-cuda \
  --test tegra_fragmented_va_cuda -- --exact "$TEST_NAME" --nocapture 2>&1)"
STATUS=$?
set -e
printf '%s\n' "$OUTPUT"
[ "$STATUS" -eq 0 ] || fail "(c) $TEST_NAME failed"
grep -Eq '^test result: ok\. 1 passed;' <<<"$OUTPUT" || fail "(c) expected exactly one passed test"
printf 'PASS  (c) %s against the crates.io resolution\n' "$TEST_NAME"
