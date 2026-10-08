#!/usr/bin/env bash
# check-tegra-pool-registry-build.sh — the `tegra-pool` feature must fail
# loudly against registry cudarc (0.8.28 Slice 30, R30-07 / AC30-07; design
# § 4.2).
#
# WHY THIS EXISTS
#   `tegra-pool` needs FathomDB's vendored cudarc 0.19.7
#   (third_party/cudarc-0.19.7, FATHOMDB-PATCH.md items 5 and 6). The root
#   `[patch.crates-io]` that selects it applies to workspace builds only, so a
#   downstream user of the published crates gets registry cudarc. The vendored
#   crate declares the no-op feature `fathomdb-private-pool`, and
#   fathomdb-embedder's `tegra-pool` enables it, so such a build must stop at
#   resolution with Cargo's "depends on `cudarc` with feature
#   `fathomdb-private-pool` but `cudarc` does not have that feature" instead
#   of an opaque missing-symbol error, while the build without `tegra-pool`
#   must still compile against registry cudarc.
#
# WHAT IT DOES
#   Copies the tracked workspace sources into a scratch directory (mktemp
#   under $TMPDIR) and adds two downstream consumer crates, each its own
#   workspace root, that depend on the copied fathomdb-embedder by path. Each
#   consumer's `[patch.crates-io]` is the workspace's minus the cudarc entry.
#     (a) The consumer with `features = ["embed-cuda"]` must `cargo check`,
#         with cudarc 0.19.7 resolved from the registry.
#     (b) The consumer with `features = ["embed-cuda", "tegra-pool"]` must
#         fail with the feature-marker message.
#   A consumer, not the workspace itself, is what a downstream user is: Cargo
#   resolves a workspace member's lockfile with every feature, so inside the
#   workspace (a) would fail on the marker too. For the same reason each
#   consumer names its features on the dependency, not in a [features]
#   table of its own.
#   The Candle and libsqlite3-sys patch entries stay. The Fathom Candle fork's
#   0.10.3 crates are not on crates.io yet (publishing them is a separate
#   0.8.28 release task), so without them nothing resolves and the check
#   would prove nothing about cudarc. Once they are published, drop their
#   entries too.
#
# WHEN TO RUN
#   A release check, run by hand before a release that changes cudarc,
#   Candle or the `tegra-pool` wiring (dev/design/release.md). It is NOT part
#   of scripts/agent-verify.sh: it needs the network (crates.io, the Candle
#   fork) and a CUDA toolkit (candle-kernels runs nvcc), and it is slow.
#
# ENV
#   CUDA_COMPUTE_CAP  passed to candle-kernels (default 87, the Jetson Orin).
#   CARGO_TARGET_DIR  reuse a target directory (default: inside the scratch
#                     copy, removed on exit).
#   KEEP_SCRATCH=1    keep the scratch copy for inspection.
#
# Exit 0 when both halves hold; 1 otherwise.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck disable=SC2016 # literal backticks in Cargo's message
MARKER_MESSAGE='with feature `fathomdb-private-pool` but `cudarc` does not have that feature'

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/fathomdb-tegra-pool-registry.XXXXXX")"
cleanup() {
  if [ "${KEEP_SCRATCH:-}" = "1" ]; then
    printf 'check-tegra-pool-registry-build: kept %s\n' "$SCRATCH"
  else
    rm -rf "$SCRATCH"
  fi
}
trap cleanup EXIT

fail() {
  printf 'FAIL  check-tegra-pool-registry-build: %s\n' "$1" >&2
  exit 1
}

# Tracked files only, from the working tree: build output and untracked
# scratch never reach the copy.
(
  cd "$REPO_ROOT"
  git ls-files -z -- Cargo.toml Cargo.lock rust-toolchain.toml src/rust \
    third_party/libsqlite3-sys-0.38.1 \
    | tar --null -T - -cf -
) | tar -xf - -C "$SCRATCH"

# make_consumer DIR FEATURES: a downstream crate, its own workspace root,
# depending on the copied fathomdb-embedder with exactly FEATURES (a TOML
# array body) and patching crates.io like the workspace minus cudarc.
make_consumer() {
  local dir="$1" features="$2"
  mkdir -p "$dir/src"
  : >"$dir/src/lib.rs"
  cp "$SCRATCH/rust-toolchain.toml" "$dir/"
  python3 - "$SCRATCH/Cargo.toml" "$dir/Cargo.toml" "$features" <<'PY'
from pathlib import Path
import re
import sys

workspace = Path(sys.argv[1]).read_text(encoding="utf-8")
patch = re.search(r"^\[patch\.crates-io\]\n(?P<body>(?:[^\[\n].*\n|\n)*)", workspace, re.MULTILINE)
if patch is None:
    raise SystemExit("workspace Cargo.toml has no [patch.crates-io] table")
lines = [line for line in patch.group("body").splitlines() if line and not line.startswith("#")]
kept = [line for line in lines if not line.startswith("cudarc ")]
if len(lines) - len(kept) != 1:
    raise SystemExit("[patch.crates-io] must carry exactly one cudarc entry")
kept = [line.replace('path = "third_party/', 'path = "../third_party/') for line in kept]
Path(sys.argv[2]).write_text(
    "\n".join(
        [
            "[package]",
            'name = "tegra-pool-registry-consumer"',
            'version = "0.0.0"',
            'edition = "2021"',
            "publish = false",
            "",
            "[workspace]",
            "",
            "[dependencies]",
            "fathomdb-embedder = { path = \"../src/rust/crates/fathomdb-embedder\", "
            f"features = [{sys.argv[3]}] }}",
            "",
            "[patch.crates-io]",
            *kept,
            "",
        ]
    ),
    encoding="utf-8",
)
PY
  if grep -q '^cudarc ' "$dir/Cargo.toml"; then
    fail 'the consumer manifest still patches cudarc'
  fi
}

export CUDA_COMPUTE_CAP="${CUDA_COMPUTE_CAP:-87}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$SCRATCH/target}"

printf 'check-tegra-pool-registry-build: (a) embed-cuda against registry cudarc\n'
make_consumer "$SCRATCH/consumer-embed-cuda" '"embed-cuda"'
# Unpinned, a downstream lockfile takes the newest registry cudarc 0.19.x for
# Candle (fathomdb-embedder's own `=0.19.7` edge is optional and off here).
# Pin the reviewed 0.19.7 that the workspace lockfile and the vendored copy
# carry, so (a) checks the version the vendored tree patches.
(
  cd "$SCRATCH/consumer-embed-cuda"
  cargo generate-lockfile -q
  RESOLVED="$(sed -n '/^name = "cudarc"$/{n;s/^version = "\(0\.19\.[0-9]*\)"$/\1/p}' Cargo.lock)"
  [ -n "$RESOLVED" ] || fail 'the consumer lockfile has no cudarc 0.19.x'
  if [ "$RESOLVED" != "0.19.7" ]; then
    cargo update -q -p "cudarc@$RESOLVED" --precise 0.19.7
  fi
)
if ! (cd "$SCRATCH/consumer-embed-cuda" && cargo check -q); then
  fail 'fathomdb-embedder with embed-cuda does not build against registry cudarc'
fi
if ! python3 - "$SCRATCH/consumer-embed-cuda/Cargo.lock" <<'PY'
from pathlib import Path
import re
import sys

# Python 3.10 has no tomllib; Cargo.lock's package blocks are regular.
lock = Path(sys.argv[1]).read_text(encoding="utf-8")
blocks = [b for b in lock.split("[[package]]") if 'name = "cudarc"\nversion = "0.19.7"' in b]
sources = [re.search(r'^source = "([^"]*)"', b, re.MULTILINE) for b in blocks]
ok = bool(blocks) and all(m is not None and m.group(1).startswith("registry+") for m in sources)
raise SystemExit(0 if ok else 1)
PY
then
  fail 'the consumer did not resolve cudarc 0.19.7 from the registry'
fi
printf 'PASS  (a) embed-cuda builds against registry cudarc 0.19.7\n'

printf 'check-tegra-pool-registry-build: (b) embed-cuda,tegra-pool against registry cudarc\n'
make_consumer "$SCRATCH/consumer-tegra-pool" '"embed-cuda", "tegra-pool"'
set +e
OUTPUT="$(cd "$SCRATCH/consumer-tegra-pool" && cargo check -q 2>&1)"
STATUS=$?
set -e
if [ "$STATUS" -eq 0 ]; then
  fail 'fathomdb-embedder with embed-cuda,tegra-pool built against registry cudarc'
fi
if ! grep -Fq -- "$MARKER_MESSAGE" <<<"$OUTPUT"; then
  printf '%s\n' "$OUTPUT" >&2
  fail "the tegra-pool build failed without the marker message: $MARKER_MESSAGE"
fi
printf 'PASS  (b) tegra-pool refuses registry cudarc: %s\n' "$MARKER_MESSAGE"
