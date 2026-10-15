#!/usr/bin/env bash
# The single-cudarc guard (0.8.28 Slice 31, design § 3.5, AC31-07).
#
# FathomDB reaches cudarc only as the published fork crate `cudarc-fathomdb`,
# through Candle. This guard fails when:
#   A. the lock has an upstream `cudarc` 0.19.x package (the ug-cuda chain's
#      cudarc 0.17.8 is excluded by version);
#   C. for each Linux target and each embedder feature set, the resolved
#      normal tree has a `cudarc v0.19` node, a `cudarc-fathomdb` node that is
#      not exactly the pinned registry version (a git or path source prints a
#      parenthesised source), or no `cudarc-fathomdb` node at all for a set
#      that enables embed-cuda or rerank-cuda;
#   D. a concrete `cuda-NNNNN` feature is enabled on `cudarc-fathomdb` (the
#      binding version comes only from CUDARC_CUDA_VERSION or nvcc, D-8W).
# The number, version and source of cudarc-fathomdb lock entries are checked
# by scripts/check-pinned-override-rot.py, not here.
#
# Usage: check-single-cudarc.sh [--root DIR] [--lock FILE]
#   --root  workspace for conditions C and D (default: repository root)
#   --lock  lock file for condition A (default: ROOT/Cargo.lock)
# `cargo tree` runs --offline --locked: fill the registry cache first with
# `cargo fetch --locked`. A cargo failure is a guard failure, never a skip.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$SCRIPT_DIR/.." && pwd)"
lock=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --root) root="$2"; shift 2 ;;
    --lock) lock="$2"; shift 2 ;;
    *) printf 'check-single-cudarc: unknown argument %s\n' "$1" >&2; exit 2 ;;
  esac
done
[ -n "$lock" ] || lock="$root/Cargo.lock"

exec python3 -I - "$root" "$lock" <<'PY'
import json
import re
import subprocess
import sys
from pathlib import Path

try:
    import tomllib
except ImportError:  # Python 3.10 (Ubuntu 22.04, Jetson); tomli is in [dev].
    import tomli as tomllib

root = Path(sys.argv[1]).resolve()
lock_path = Path(sys.argv[2]).resolve()
TARGETS = ("x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu")
FEATURE_SETS = ("embed-cuda", "rerank-cuda", "tegra-pool", "embed-cuda,tegra-pool", "rerank-cuda,tegra-pool")
ALL_FEATURES = "fathomdb-embedder/embed-cuda,fathomdb-embedder/rerank-cuda,fathomdb-embedder/tegra-pool"
CONCRETE_FEATURE = re.compile(r'^cudarc-fathomdb feature "cuda-\d{5}"$')
failures: list[str] = []


def pinned_version() -> str:
    try:
        record = json.loads((root / "scripts/pinned-override-rot.json").read_text(encoding="utf-8"))
        pins = record["cargo_fork_registry_pins"]["pins"]
    except (OSError, ValueError, KeyError, TypeError) as exc:
        raise SystemExit(
            f"check-single-cudarc: {root}/scripts/pinned-override-rot.json has no cargo_fork_registry_pins: {exc!r}"
        )
    versions = [pin.get("version") for pin in pins if pin.get("package") == "cudarc-fathomdb"]
    if len(versions) != 1:
        raise SystemExit("check-single-cudarc: scripts/pinned-override-rot.json must pin cudarc-fathomdb once")
    return versions[0]


def cargo_tree(*args: str) -> list[str]:
    result = subprocess.run(
        ["cargo", "tree", "--offline", "--locked", *args, "--prefix", "none"],
        cwd=root, capture_output=True, text=True, check=False,
    )
    if result.returncode != 0:
        failures.append(f"cargo tree {' '.join(args)} failed: {result.stderr.strip()}")
        return []
    lines = []
    for line in result.stdout.splitlines():
        line = line.rstrip()
        if line.endswith(" (*)"):
            line = line[: -len(" (*)")]
        lines.append(line)
    return lines


# A. Upstream cudarc 0.19 in the lock.
try:
    lock = tomllib.loads(lock_path.read_text(encoding="utf-8"))
except (OSError, ValueError) as exc:
    raise SystemExit(f"check-single-cudarc: cannot read lock {lock_path}: {exc}")
for package in lock.get("package", []):
    if package.get("name") == "cudarc" and str(package.get("version", "")).startswith("0.19."):
        failures.append(
            f"A: {lock_path.name} has upstream cudarc {package['version']}; "
            "only cudarc-fathomdb may provide cudarc 0.19"
        )

version = pinned_version()
expected_line = f"cudarc-fathomdb v{version}"

# C. One cudarc-fathomdb node, from the registry, per target and feature set.
for target in TARGETS:
    for features in FEATURE_SETS:
        lines = cargo_tree("-e", "normal", "--target", target, "-p", "fathomdb-embedder", "--features", features)
        where = f"{target} --features {features}"
        found = False
        for line in lines:
            if line.startswith("cudarc v0.19"):
                failures.append(f"C: {where}: the tree has {line}")
            elif line.startswith("cudarc-fathomdb "):
                found = True
                if line != expected_line:
                    failures.append(f'C: {where}: "{line}" is not exactly "{expected_line}"')
        if not found and ("embed-cuda" in features or "rerank-cuda" in features) and lines:
            first = features.split(",")[0]
            failures.append(f"C: {where}: no cudarc-fathomdb line for {first}")

# D. No concrete CUDA version feature on cudarc-fathomdb.
for target in TARGETS:
    for line in cargo_tree("-e", "features", "-i", "cudarc-fathomdb", "--target", target, "--features", ALL_FEATURES):
        if CONCRETE_FEATURE.match(line):
            failures.append(f"D: {target}: {line} selects the binding version; use CUDARC_CUDA_VERSION")

if failures:
    for failure in failures:
        print(f"FAIL  single-cudarc: {failure}", file=sys.stderr)
    sys.exit(1)
print(f"ok single-cudarc: one cudarc 0.19 in the graph, {expected_line} from the registry, no concrete CUDA feature")
PY
