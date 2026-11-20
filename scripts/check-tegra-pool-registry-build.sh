#!/usr/bin/env bash
# check-tegra-pool-registry-build.sh — current .30 candidate registry consumer.
# Source/staged-package qualification does not make unpublished versions visible.
# Run this only as the separately authorized public delivery qualification;
# existing historical published-floor receipts keep their original identities.
#
# WHAT IT DOES
#   Copies tracked workspace sources to owned scratch and adds an independent
#   consumer workspace with no consumer-local patches or source config. Resolution
#   requires exactly cudarc-fathomdb0.21.0 and the complete Candle0.13.0 cohort,
#   including candle-kernels-fathomdb, with logical crates.io identities, and no upstream cudarc or
#   upstream candle-kernels. The kernel source retains static CUDART ownership.
#   Full mode builds/links on actual Tegra and runs the existing real synchronous fallback
#   test with the default pool unavailable with FATHOMDB_REQUIRE_LIVE=1; exactly one test must pass.
#   A path dependency cannot run its own tests in another workspace, so the
#   existing test and live support are copied without modifying their oracle.
#
# WHEN TO RUN
#   A release check, run by hand before a release that changes cudarc,
#   Candle or the `tegra-pool` wiring (dev/design/release.md), and in the
#   Slice 31 qualification (REG-a on any host; REG-b and REG-c on the Orin).
#   It is NOT part of scripts/agent-verify.sh: it needs crates.io, a CUDA
#   toolkit for (b) and a Jetson GPU for (c).
#
# USAGE
#   check-tegra-pool-registry-build.sh                 (a), (b) and (c); only
#                                                      on an aarch64 Linux
#                                                      Tegra host, elsewhere
#                                                      it exits 2 before (a)
#   check-tegra-pool-registry-build.sh --resolve-only  (a) alone
#   ENABLE=embed-cuda-only (env)  the consumer's dependency names no
#                     `tegra-pool`, only `embed-cuda`, which implies it
#                     (slice-31 design.md § 14.3); (a)-(c) must still hold.
#
# ENV
#   CUDA_COMPUTE_CAP  passed to candle-kernels (default 87, the Jetson Orin).
#   CUDA_PATH, PATH, LIBRARY_PATH  the CUDA12.6 toolkit for (b) and (c);
#                     LIBRARY_PATH must expose its static CUDART archive.
#   FATHOMDB_POOL_MODE  passed through to (c) (the qualification runs `off`).
#   CARGO_TARGET_DIR  reuse a target directory (default: inside the scratch
#                     copy, removed on exit).
#   KEEP_SCRATCH=1    keep the scratch copy for inspection.
#   FATHOMDB_TEGRA_RELEASE_FILE  the Tegra marker file the host check reads
#                     (default /etc/nv_tegra_release; tests point it away).
#
# Exit 0 when every requested step holds; 1 when a step fails; 2 on a usage
# error, including full mode on a host that cannot run (b) and (c), so an
# exit status of 0 always means the whole requested check ran.
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

HOST_OS="$(uname -s)"
HOST_ARCH="$(uname -m)"
TEGRA_RELEASE_FILE="${FATHOMDB_TEGRA_RELEASE_FILE:-/etc/nv_tegra_release}"
if [ "$RESOLVE_ONLY" = "0" ] \
  && { [ "$HOST_OS" != "Linux" ] || [ "$HOST_ARCH" != "aarch64" ] || [ ! -e "$TEGRA_RELEASE_FILE" ]; }; then
  printf 'check-tegra-pool-registry-build: (b) and (c) need an aarch64 Linux Tegra host; this host is %s %s.\n' \
    "$HOST_OS" "$HOST_ARCH" >&2
  printf 'check-tegra-pool-registry-build: run with --resolve-only for (a) alone.\n' >&2
  exit 2
fi

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

# Tracked working-tree sources only; no root Cargo config reaches the consumer.
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
candle-core = { package = "candle-core-fathomdb", version = "=0.13.0" }
TOML
if grep -q '^\[patch' "$CONSUMER/Cargo.toml"; then
  fail 'the consumer manifest has a [patch] table'
fi

printf 'check-tegra-pool-registry-build: (a) logical resolution with no [patch] table; ambient mirrors are not public-delivery proof\n'
(cd "$CONSUMER" && cargo generate-lockfile -q) || fail 'cargo generate-lockfile failed in the consumer'
if ! python3 - "$CONSUMER/Cargo.lock" <<'PY'
from pathlib import Path
import re
import sys

# Python 3.10 (the Jetson's) has no tomllib; Cargo.lock's package blocks are regular.
REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"
EXPECTED = {
    "cudarc-fathomdb": "0.21.0",
    "candle-core-fathomdb": "0.13.0",
    "candle-nn-fathomdb": "0.13.0",
    "candle-transformers-fathomdb": "0.13.0",
    "candle-kernels-fathomdb": "0.13.0",
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
    if name in {"cudarc", "candle-kernels"}:
        failures.append(f"upstream {name} {version} resolved ({source})")
for failure in failures:
    print(f"  {failure}", file=sys.stderr)
raise SystemExit(1 if failures else 0)
PY
then
  fail '(a) the consumer did not resolve the exact fork crates with logical crates.io identities'
fi
printf 'PASS  (a) exact logical crates.io identities resolve with no [patch] table; independently verify public archive checksums/downloads before public delivery sign-off\n'

if [ "$RESOLVE_ONLY" = "1" ]; then
  printf 'check-tegra-pool-registry-build: --resolve-only: (b) and (c) not run\n'
  exit 0
fi
[ -n "${LIBRARY_PATH:-}" ] || fail '(b) LIBRARY_PATH must name the CUDA toolkit library directory containing libcudart_static.a'

printf 'check-tegra-pool-registry-build: (b) build and link on this Tegra host\n'
(cd "$CONSUMER" && cargo build --locked --features embed-cuda) || fail '(b) cargo build --features embed-cuda failed'
(cd "$CONSUMER" && cargo test --locked --no-run --features embed-cuda) || fail '(b) cargo test --no-run --features embed-cuda failed'
printf 'PASS  (b) the consumer builds and links the locked registry cohort\n'

printf 'check-tegra-pool-registry-build: (c) %s\n' "$TEST_NAME"
set +e
OUTPUT="$(cd "$CONSUMER" && FATHOMDB_REQUIRE_LIVE=1 cargo test --locked --features embed-cuda \
  --test tegra_fragmented_va_cuda -- --exact "$TEST_NAME" --nocapture 2>&1)"
STATUS=$?
set -e
printf '%s\n' "$OUTPUT"
[ "$STATUS" -eq 0 ] || fail "(c) $TEST_NAME failed"
grep -Eq '^test result: ok\. 1 passed;' <<<"$OUTPUT" || fail "(c) expected exactly one passed test"
printf 'PASS  (c) %s against the locked logical registry resolution\n' "$TEST_NAME"
